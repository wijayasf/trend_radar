use std::path::Path;

use duckdb::{params, Connection, OptionalExt, Transaction};

use crate::models::multi_source::{
    normalize_entity_name, AppendObservationResult, CanonicalEntity, CanonicalEntityMetadataUpdate,
    CollectionMode, CollectionRunStatus, EntityStatus, ExternalSource, LinkReviewState,
    NewCanonicalEntity, NewCollectionRun, NewSourceObservation, NewSourceRecordEntityLink,
    PrimaryEntityType, RelationshipType, ResolutionState, SourceCollectionRun, SourceObservation,
    SourceRecord, SourceRecordEntityLink, SourceRecordUpsert,
};
use crate::services::duckdb_service;

pub struct MultiSourceRepository {
    connection: Connection,
}

impl MultiSourceRepository {
    pub fn open() -> Result<Self, String> {
        let database_path = duckdb_service::initialize_database()?;
        Self::open_initialized(&database_path)
    }

    pub(crate) fn open_at(database_path: &Path) -> Result<Self, String> {
        duckdb_service::initialize_database_at(database_path)?;
        Self::open_initialized(database_path)
    }

    fn open_initialized(database_path: &Path) -> Result<Self, String> {
        let connection = Connection::open(database_path).map_err(|error| {
            format!(
                "DuckDB multi-source connection failed at {}: {error}",
                database_path.display()
            )
        })?;
        Ok(Self { connection })
    }

    pub fn create_canonical_entity(
        &self,
        input: &NewCanonicalEntity,
    ) -> Result<CanonicalEntity, String> {
        validate_non_empty("canonical name", &input.canonical_name)?;
        let normalized_name = normalize_entity_name(&input.canonical_name);
        validate_non_empty("normalized canonical name", &normalized_name)?;
        let entity_id = generate_uuid(&self.connection)?;

        self.connection
            .execute(
                r#"
                INSERT INTO canonical_entities (
                    entity_id,
                    canonical_name,
                    normalized_name,
                    primary_type,
                    status,
                    description,
                    primary_website,
                    primary_repository
                ) VALUES (
                    CAST(?1 AS UUID), ?2, ?3, ?4, 'active', ?5, ?6, ?7
                )
                "#,
                params![
                    entity_id,
                    input.canonical_name.trim(),
                    normalized_name,
                    input.primary_type.as_str(),
                    normalize_optional_text(input.description.as_deref()),
                    normalize_optional_text(input.primary_website.as_deref()),
                    normalize_optional_text(input.primary_repository.as_deref())
                ],
            )
            .map_err(|error| format!("DuckDB canonical entity insert failed: {error}"))?;

        self.get_canonical_entity(&entity_id)?.ok_or_else(|| {
            "Canonical entity was inserted but could not be loaded afterwards.".to_string()
        })
    }

    pub fn get_canonical_entity(&self, entity_id: &str) -> Result<Option<CanonicalEntity>, String> {
        validate_non_empty("entity ID", entity_id)?;
        let raw = self
            .connection
            .query_row(
                &format!(
                    "{} WHERE entity_id = CAST(?1 AS UUID)",
                    canonical_entity_select_sql()
                ),
                params![entity_id],
                read_raw_canonical_entity,
            )
            .optional()
            .map_err(|error| format!("DuckDB canonical entity query failed: {error}"))?;
        raw.map(TryInto::try_into).transpose()
    }

    pub fn lookup_canonical_entities_by_normalized_name(
        &self,
        name: &str,
    ) -> Result<Vec<CanonicalEntity>, String> {
        let normalized_name = normalize_entity_name(name);
        validate_non_empty("normalized canonical name", &normalized_name)?;
        let mut statement = self
            .connection
            .prepare(&format!(
                "{} WHERE normalized_name = ?1 ORDER BY canonical_name, CAST(entity_id AS VARCHAR)",
                canonical_entity_select_sql()
            ))
            .map_err(|error| {
                format!("DuckDB canonical entity lookup preparation failed: {error}")
            })?;
        let rows = statement
            .query_map(params![normalized_name], read_raw_canonical_entity)
            .map_err(|error| format!("DuckDB canonical entity lookup failed: {error}"))?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(RawCanonicalEntity::try_into(row.map_err(|error| {
                format!("DuckDB canonical entity row read failed: {error}")
            })?)?);
        }
        Ok(entities)
    }

    pub fn update_canonical_entity_metadata(
        &self,
        entity_id: &str,
        update: &CanonicalEntityMetadataUpdate,
    ) -> Result<CanonicalEntity, String> {
        validate_non_empty("entity ID", entity_id)?;
        validate_non_empty("canonical name", &update.canonical_name)?;
        let normalized_name = normalize_entity_name(&update.canonical_name);
        let updated_count = self
            .connection
            .execute(
                r#"
                UPDATE canonical_entities
                SET
                    canonical_name = ?2,
                    normalized_name = ?3,
                    primary_type = ?4,
                    status = ?5,
                    description = ?6,
                    primary_website = ?7,
                    primary_repository = ?8,
                    updated_at = CURRENT_TIMESTAMP
                WHERE entity_id = CAST(?1 AS UUID)
                "#,
                params![
                    entity_id,
                    update.canonical_name.trim(),
                    normalized_name,
                    update.primary_type.as_str(),
                    update.status.as_str(),
                    normalize_optional_text(update.description.as_deref()),
                    normalize_optional_text(update.primary_website.as_deref()),
                    normalize_optional_text(update.primary_repository.as_deref())
                ],
            )
            .map_err(|error| format!("DuckDB canonical entity update failed: {error}"))?;

        if updated_count == 0 {
            return Err(format!("Canonical entity not found: {entity_id}"));
        }
        self.get_canonical_entity(entity_id)?.ok_or_else(|| {
            "Canonical entity was updated but could not be loaded afterwards.".to_string()
        })
    }

    pub fn start_collection_run(
        &self,
        input: &NewCollectionRun,
    ) -> Result<SourceCollectionRun, String> {
        validate_optional_json("collection scope", input.scope_json.as_deref())?;
        let collection_run_id = generate_uuid(&self.connection)?;
        self.connection
            .execute(
                r#"
                INSERT INTO source_collection_runs (
                    collection_run_id,
                    source,
                    collection_mode,
                    scope_json,
                    started_at,
                    status
                ) VALUES (
                    CAST(?1 AS UUID), ?2, ?3, ?4, CURRENT_TIMESTAMP, 'running'
                )
                "#,
                params![
                    collection_run_id,
                    input.source.as_str(),
                    input.collection_mode.as_str(),
                    input.scope_json
                ],
            )
            .map_err(|error| format!("DuckDB source collection run insert failed: {error}"))?;

        self.get_collection_run(&collection_run_id)?.ok_or_else(|| {
            "Collection run was inserted but could not be loaded afterwards.".to_string()
        })
    }

    pub fn get_collection_run(
        &self,
        collection_run_id: &str,
    ) -> Result<Option<SourceCollectionRun>, String> {
        validate_non_empty("collection run ID", collection_run_id)?;
        let raw = self
            .connection
            .query_row(
                &format!(
                    "{} WHERE collection_run_id = CAST(?1 AS UUID)",
                    collection_run_select_sql()
                ),
                params![collection_run_id],
                read_raw_collection_run,
            )
            .optional()
            .map_err(|error| format!("DuckDB source collection run query failed: {error}"))?;
        raw.map(TryInto::try_into).transpose()
    }

    pub fn complete_collection_run(
        &self,
        collection_run_id: &str,
    ) -> Result<SourceCollectionRun, String> {
        self.finish_collection_run(collection_run_id, CollectionRunStatus::Completed, None)
    }

    pub fn mark_collection_run_partial(
        &self,
        collection_run_id: &str,
        error_summary: Option<&str>,
    ) -> Result<SourceCollectionRun, String> {
        self.finish_collection_run(
            collection_run_id,
            CollectionRunStatus::Partial,
            error_summary,
        )
    }

    pub fn mark_collection_run_failed(
        &self,
        collection_run_id: &str,
        error_summary: Option<&str>,
    ) -> Result<SourceCollectionRun, String> {
        self.finish_collection_run(
            collection_run_id,
            CollectionRunStatus::Failed,
            error_summary,
        )
    }

    fn finish_collection_run(
        &self,
        collection_run_id: &str,
        status: CollectionRunStatus,
        error_summary: Option<&str>,
    ) -> Result<SourceCollectionRun, String> {
        if !status.is_terminal() {
            return Err("Collection run can only transition to a terminal status.".to_string());
        }
        let current = self
            .get_collection_run(collection_run_id)?
            .ok_or_else(|| format!("Collection run not found: {collection_run_id}"))?;
        if current.status != CollectionRunStatus::Running {
            return Err(format!(
                "Collection run {} is already {} and cannot transition to {}.",
                collection_run_id,
                current.status.as_str(),
                status.as_str()
            ));
        }

        self.connection
            .execute(
                r#"
                UPDATE source_collection_runs
                SET
                    status = ?2,
                    finished_at = CURRENT_TIMESTAMP,
                    error_summary = ?3
                WHERE collection_run_id = CAST(?1 AS UUID)
                    AND status = 'running'
                "#,
                params![
                    collection_run_id,
                    status.as_str(),
                    normalize_optional_text(error_summary)
                ],
            )
            .map_err(|error| format!("DuckDB source collection run update failed: {error}"))?;

        self.get_collection_run(collection_run_id)?.ok_or_else(|| {
            "Collection run was updated but could not be loaded afterwards.".to_string()
        })
    }

    pub fn upsert_source_record(&self, input: &SourceRecordUpsert) -> Result<SourceRecord, String> {
        validate_source_record_input(input)?;
        upsert_source_record_on(&self.connection, input)
    }

    pub fn get_source_record_by_key(
        &self,
        source: ExternalSource,
        source_record_key: &str,
    ) -> Result<Option<SourceRecord>, String> {
        validate_non_empty("source record key", source_record_key)?;
        let raw = self
            .connection
            .query_row(
                &format!(
                    "{} WHERE source = ?1 AND source_record_key = ?2",
                    source_record_select_sql()
                ),
                params![source.as_str(), source_record_key.trim()],
                read_raw_source_record,
            )
            .optional()
            .map_err(|error| format!("DuckDB source record key query failed: {error}"))?;
        raw.map(TryInto::try_into).transpose()
    }

    pub fn get_source_record(
        &self,
        source_record_id: &str,
    ) -> Result<Option<SourceRecord>, String> {
        validate_non_empty("source record ID", source_record_id)?;
        let raw = self
            .connection
            .query_row(
                &format!(
                    "{} WHERE source_record_id = CAST(?1 AS UUID)",
                    source_record_select_sql()
                ),
                params![source_record_id],
                read_raw_source_record,
            )
            .optional()
            .map_err(|error| format!("DuckDB source record query failed: {error}"))?;
        raw.map(TryInto::try_into).transpose()
    }

    pub fn set_source_record_resolution_state(
        &self,
        source_record_id: &str,
        resolution_state: ResolutionState,
    ) -> Result<SourceRecord, String> {
        validate_non_empty("source record ID", source_record_id)?;
        let approved_links = approved_link_count(&self.connection, source_record_id)?;
        validate_resolution_state(resolution_state, approved_links)?;

        let updated_count = self
            .connection
            .execute(
                r#"
                UPDATE source_records
                SET resolution_state = ?2, updated_at = CURRENT_TIMESTAMP
                WHERE source_record_id = CAST(?1 AS UUID)
                "#,
                params![source_record_id, resolution_state.as_str()],
            )
            .map_err(|error| format!("DuckDB source record resolution update failed: {error}"))?;
        if updated_count == 0 {
            return Err(format!("Source record not found: {source_record_id}"));
        }
        self.get_source_record(source_record_id)?.ok_or_else(|| {
            "Source record resolution was updated but could not be loaded afterwards.".to_string()
        })
    }

    pub fn append_observation(
        &self,
        collection_run_id: &str,
        source_record_id: &str,
        input: &NewSourceObservation,
    ) -> Result<AppendObservationResult, String> {
        validate_observation_input(input)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("DuckDB observation transaction failed: {error}"))?;
        let result =
            append_observation_on(&transaction, collection_run_id, source_record_id, input)?;
        transaction
            .commit()
            .map_err(|error| format!("DuckDB observation transaction commit failed: {error}"))?;
        Ok(result)
    }

    pub fn upsert_record_and_append_observation(
        &self,
        collection_run_id: &str,
        record_input: &SourceRecordUpsert,
        observation_input: &NewSourceObservation,
    ) -> Result<(SourceRecord, AppendObservationResult), String> {
        validate_source_record_input(record_input)?;
        validate_observation_input(observation_input)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("DuckDB source observation transaction failed: {error}"))?;
        let record = upsert_source_record_on(&transaction, record_input)?;
        let observation = append_observation_on(
            &transaction,
            collection_run_id,
            &record.source_record_id,
            observation_input,
        )?;
        transaction.commit().map_err(|error| {
            format!("DuckDB source observation transaction commit failed: {error}")
        })?;
        Ok((record, observation))
    }

    pub fn list_observations_for_source_record(
        &self,
        source_record_id: &str,
    ) -> Result<Vec<SourceObservation>, String> {
        validate_non_empty("source record ID", source_record_id)?;
        let mut statement = self
            .connection
            .prepare(&format!(
                "{} WHERE source_record_id = CAST(?1 AS UUID) ORDER BY observed_at, created_at",
                source_observation_select_sql()
            ))
            .map_err(|error| format!("DuckDB source observation preparation failed: {error}"))?;
        let rows = statement
            .query_map(params![source_record_id], read_source_observation)
            .map_err(|error| format!("DuckDB source observation query failed: {error}"))?;
        let mut observations = Vec::new();
        for row in rows {
            observations.push(
                row.map_err(|error| format!("DuckDB source observation row read failed: {error}"))?,
            );
        }
        Ok(observations)
    }

    pub fn create_source_record_entity_link(
        &self,
        input: &NewSourceRecordEntityLink,
    ) -> Result<SourceRecordEntityLink, String> {
        validate_link_input(input)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("DuckDB source/entity link transaction failed: {error}"))?;
        if input.review_state == LinkReviewState::Approved {
            ensure_record_accepts_approved_link(&transaction, &input.source_record_id)?;
        }
        let link_id = generate_uuid(&transaction)?;
        transaction
            .execute(
                r#"
                INSERT INTO source_record_entity_links (
                    link_id,
                    source_record_id,
                    entity_id,
                    relationship_type,
                    match_method,
                    match_confidence,
                    review_state,
                    evidence_json,
                    reviewed_at
                ) VALUES (
                    CAST(?1 AS UUID),
                    CAST(?2 AS UUID),
                    CAST(?3 AS UUID),
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8,
                    CASE WHEN ?7 = 'pending' THEN NULL ELSE CURRENT_TIMESTAMP END
                )
                "#,
                params![
                    link_id,
                    input.source_record_id,
                    input.entity_id,
                    input.relationship_type.as_str(),
                    input.match_method.trim(),
                    input.match_confidence,
                    input.review_state.as_str(),
                    input.evidence_json
                ],
            )
            .map_err(|error| format!("DuckDB source/entity link insert failed: {error}"))?;
        reconcile_record_resolution(&transaction, &input.source_record_id)?;
        transaction.commit().map_err(|error| {
            format!("DuckDB source/entity link transaction commit failed: {error}")
        })?;

        self.get_source_record_entity_link(&link_id)?
            .ok_or_else(|| {
                "Source/entity link was inserted but could not be loaded afterwards.".to_string()
            })
    }

    pub fn get_links_for_source_record(
        &self,
        source_record_id: &str,
    ) -> Result<Vec<SourceRecordEntityLink>, String> {
        self.query_links(
            "WHERE source_record_id = CAST(?1 AS UUID) ORDER BY created_at, CAST(link_id AS VARCHAR)",
            source_record_id,
        )
    }

    pub fn get_records_for_entity(&self, entity_id: &str) -> Result<Vec<SourceRecord>, String> {
        validate_non_empty("entity ID", entity_id)?;
        let mut statement = self
            .connection
            .prepare(&format!(
                r#"
                {}
                JOIN source_record_entity_links links
                    ON links.source_record_id = source_records.source_record_id
                WHERE links.entity_id = CAST(?1 AS UUID)
                ORDER BY source_records.source, source_records.source_record_key
                "#,
                source_record_select_sql()
            ))
            .map_err(|error| format!("DuckDB entity source record preparation failed: {error}"))?;
        let rows = statement
            .query_map(params![entity_id], read_raw_source_record)
            .map_err(|error| format!("DuckDB entity source record query failed: {error}"))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(RawSourceRecord::try_into(row.map_err(|error| {
                format!("DuckDB entity source record row read failed: {error}")
            })?)?);
        }
        Ok(records)
    }

    pub fn update_link_review_state(
        &self,
        link_id: &str,
        review_state: LinkReviewState,
    ) -> Result<SourceRecordEntityLink, String> {
        let current = self
            .get_source_record_entity_link(link_id)?
            .ok_or_else(|| format!("Source/entity link not found: {link_id}"))?;
        if review_state == LinkReviewState::Approved {
            ensure_record_accepts_approved_link(&self.connection, &current.source_record_id)?;
        }

        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| format!("DuckDB source/entity review transaction failed: {error}"))?;
        transaction
            .execute(
                r#"
                UPDATE source_record_entity_links
                SET
                    review_state = ?2,
                    reviewed_at = CASE WHEN ?2 = 'pending' THEN NULL ELSE CURRENT_TIMESTAMP END,
                    updated_at = CURRENT_TIMESTAMP
                WHERE link_id = CAST(?1 AS UUID)
                "#,
                params![link_id, review_state.as_str()],
            )
            .map_err(|error| format!("DuckDB source/entity review update failed: {error}"))?;
        reconcile_record_resolution(&transaction, &current.source_record_id)?;
        transaction.commit().map_err(|error| {
            format!("DuckDB source/entity review transaction commit failed: {error}")
        })?;

        self.get_source_record_entity_link(link_id)?.ok_or_else(|| {
            "Source/entity link was updated but could not be loaded afterwards.".to_string()
        })
    }

    fn get_source_record_entity_link(
        &self,
        link_id: &str,
    ) -> Result<Option<SourceRecordEntityLink>, String> {
        validate_non_empty("source/entity link ID", link_id)?;
        self.connection
            .query_row(
                &format!(
                    "{} WHERE link_id = CAST(?1 AS UUID)",
                    source_record_entity_link_select_sql()
                ),
                params![link_id],
                read_source_record_entity_link,
            )
            .optional()
            .map_err(|error| format!("DuckDB source/entity link query failed: {error}"))
    }

    fn query_links(&self, clause: &str, id: &str) -> Result<Vec<SourceRecordEntityLink>, String> {
        validate_non_empty("source/entity lookup ID", id)?;
        let mut statement = self
            .connection
            .prepare(&format!(
                "{} {clause}",
                source_record_entity_link_select_sql()
            ))
            .map_err(|error| format!("DuckDB source/entity link preparation failed: {error}"))?;
        let rows = statement
            .query_map(params![id], read_source_record_entity_link)
            .map_err(|error| format!("DuckDB source/entity link query failed: {error}"))?;
        let mut links = Vec::new();
        for row in rows {
            links.push(
                row.map_err(|error| format!("DuckDB source/entity link row read failed: {error}"))?,
            );
        }
        Ok(links)
    }
}

fn upsert_source_record_on(
    connection: &Connection,
    input: &SourceRecordUpsert,
) -> Result<SourceRecord, String> {
    let proposed_id = generate_uuid(connection)?;
    connection
        .execute(
            r#"
            INSERT INTO source_records (
                source_record_id,
                source,
                source_record_key,
                record_type,
                resolution_state,
                title,
                external_url,
                publisher,
                description,
                source_category,
                repository_url,
                published_at,
                listed_at,
                metadata_json,
                first_seen_at,
                last_seen_at
            ) VALUES (
                CAST(?1 AS UUID), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                CAST(?12 AS TIMESTAMP), CAST(?13 AS TIMESTAMP), ?14,
                CAST(?15 AS TIMESTAMP), CAST(?15 AS TIMESTAMP)
            )
            ON CONFLICT (source, source_record_key) DO UPDATE SET
                record_type = excluded.record_type,
                title = COALESCE(excluded.title, source_records.title),
                external_url = COALESCE(excluded.external_url, source_records.external_url),
                publisher = COALESCE(excluded.publisher, source_records.publisher),
                description = COALESCE(excluded.description, source_records.description),
                source_category = COALESCE(excluded.source_category, source_records.source_category),
                repository_url = COALESCE(excluded.repository_url, source_records.repository_url),
                published_at = COALESCE(excluded.published_at, source_records.published_at),
                listed_at = COALESCE(excluded.listed_at, source_records.listed_at),
                metadata_json = COALESCE(excluded.metadata_json, source_records.metadata_json),
                last_seen_at = GREATEST(source_records.last_seen_at, excluded.last_seen_at),
                updated_at = excluded.last_seen_at
            "#,
            params![
                proposed_id,
                input.source.as_str(),
                input.source_record_key.trim(),
                input.record_type.trim(),
                input.resolution_state.as_str(),
                normalize_optional_text(input.title.as_deref()),
                normalize_optional_text(input.external_url.as_deref()),
                normalize_optional_text(input.publisher.as_deref()),
                normalize_optional_text(input.description.as_deref()),
                normalize_optional_text(input.source_category.as_deref()),
                normalize_optional_text(input.repository_url.as_deref()),
                normalize_optional_text(input.published_at.as_deref()),
                normalize_optional_text(input.listed_at.as_deref()),
                input.metadata_json,
                input.seen_at
            ],
        )
        .map_err(|error| format!("DuckDB source record upsert failed: {error}"))?;

    let raw = connection
        .query_row(
            &format!(
                "{} WHERE source = ?1 AND source_record_key = ?2",
                source_record_select_sql()
            ),
            params![input.source.as_str(), input.source_record_key.trim()],
            read_raw_source_record,
        )
        .map_err(|error| format!("DuckDB source record reload failed: {error}"))?;
    raw.try_into()
}

fn append_observation_on(
    transaction: &Transaction<'_>,
    collection_run_id: &str,
    source_record_id: &str,
    input: &NewSourceObservation,
) -> Result<AppendObservationResult, String> {
    validate_non_empty("collection run ID", collection_run_id)?;
    validate_non_empty("source record ID", source_record_id)?;

    if let Some(existing) =
        find_observation_contract(transaction, collection_run_id, source_record_id, input)?
    {
        return Ok(AppendObservationResult {
            observation: existing,
            inserted: false,
        });
    }

    let (run_source, run_status): (String, String) = transaction
        .query_row(
            r#"
            SELECT source, status
            FROM source_collection_runs
            WHERE collection_run_id = CAST(?1 AS UUID)
            "#,
            params![collection_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("DuckDB collection run validation failed: {error}"))?;
    if run_status != CollectionRunStatus::Running.as_str() {
        return Err(format!(
            "Collection run {collection_run_id} is {run_status}; observations require a running collection run."
        ));
    }

    let record_source: String = transaction
        .query_row(
            r#"
            SELECT source
            FROM source_records
            WHERE source_record_id = CAST(?1 AS UUID)
            "#,
            params![source_record_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB observation source record validation failed: {error}"))?;
    if run_source != record_source {
        return Err(format!(
            "Collection run source {run_source} does not match source record source {record_source}."
        ));
    }

    let record_seen_in_run: i64 = transaction
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM source_observations
            WHERE collection_run_id = CAST(?1 AS UUID)
                AND source_record_id = CAST(?2 AS UUID)
            "#,
            params![collection_run_id, source_record_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB observation run membership query failed: {error}"))?;

    let observation_id = generate_uuid(transaction)?;
    transaction
        .execute(
            r#"
            INSERT INTO source_observations (
                observation_id,
                collection_run_id,
                source_record_id,
                observed_at,
                surface,
                observation_kind,
                time_window,
                rank,
                source_score,
                views,
                installs_total,
                installs_period,
                github_stars,
                upvotes,
                payload_hash,
                source_payload_json
            ) VALUES (
                CAST(?1 AS UUID), CAST(?2 AS UUID), CAST(?3 AS UUID),
                CAST(?4 AS TIMESTAMP), ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16
            )
            "#,
            params![
                observation_id,
                collection_run_id,
                source_record_id,
                input.observed_at,
                input.surface.trim(),
                input.observation_kind.trim(),
                input.time_window.trim(),
                input.rank,
                input.source_score,
                input.views,
                input.installs_total,
                input.installs_period,
                input.github_stars,
                input.upvotes,
                normalize_optional_text(input.payload_hash.as_deref()),
                input.source_payload_json
            ],
        )
        .map_err(|error| format!("DuckDB source observation insert failed: {error}"))?;

    let updated_count = transaction
        .execute(
            r#"
            UPDATE source_collection_runs
            SET
                observations_saved = observations_saved + 1,
                records_seen = records_seen + ?2
            WHERE collection_run_id = CAST(?1 AS UUID)
                AND status = 'running'
            "#,
            params![
                collection_run_id,
                if record_seen_in_run == 0 { 1 } else { 0 }
            ],
        )
        .map_err(|error| format!("DuckDB collection run counter update failed: {error}"))?;
    if updated_count != 1 {
        return Err(
            "Collection run counters were not updated; observation was rolled back.".to_string(),
        );
    }

    let observation = transaction
        .query_row(
            &format!(
                "{} WHERE observation_id = CAST(?1 AS UUID)",
                source_observation_select_sql()
            ),
            params![observation_id],
            read_source_observation,
        )
        .map_err(|error| format!("DuckDB source observation reload failed: {error}"))?;
    Ok(AppendObservationResult {
        observation,
        inserted: true,
    })
}

fn find_observation_contract(
    connection: &Connection,
    collection_run_id: &str,
    source_record_id: &str,
    input: &NewSourceObservation,
) -> Result<Option<SourceObservation>, String> {
    connection
        .query_row(
            &format!(
                r#"
                {}
                WHERE collection_run_id = CAST(?1 AS UUID)
                    AND source_record_id = CAST(?2 AS UUID)
                    AND surface = ?3
                    AND observation_kind = ?4
                    AND time_window = ?5
                "#,
                source_observation_select_sql()
            ),
            params![
                collection_run_id,
                source_record_id,
                input.surface.trim(),
                input.observation_kind.trim(),
                input.time_window.trim()
            ],
            read_source_observation,
        )
        .optional()
        .map_err(|error| format!("DuckDB observation idempotency query failed: {error}"))
}

fn approved_link_count(connection: &Connection, source_record_id: &str) -> Result<i64, String> {
    connection
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM source_record_entity_links
            WHERE source_record_id = CAST(?1 AS UUID)
                AND review_state = 'approved'
            "#,
            params![source_record_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB approved identity link count failed: {error}"))
}

fn validate_resolution_state(state: ResolutionState, approved_links: i64) -> Result<(), String> {
    match state {
        ResolutionState::SingleEntity if approved_links != 1 => Err(format!(
            "single_entity requires exactly one approved link; found {approved_links}."
        )),
        ResolutionState::MultipleEntities if approved_links < 2 => Err(format!(
            "multiple_entities requires at least two approved links; found {approved_links}."
        )),
        ResolutionState::NoProductEntity if approved_links != 0 => Err(format!(
            "no_product_entity requires zero approved links; found {approved_links}."
        )),
        _ => Ok(()),
    }
}

fn ensure_record_accepts_approved_link(
    connection: &Connection,
    source_record_id: &str,
) -> Result<(), String> {
    let resolution_state: String = connection
        .query_row(
            r#"
            SELECT resolution_state
            FROM source_records
            WHERE source_record_id = CAST(?1 AS UUID)
            "#,
            params![source_record_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB source record link validation failed: {error}"))?;
    if resolution_state == ResolutionState::NoProductEntity.as_str() {
        Err("A no_product_entity record cannot receive an approved canonical link. Set it to unresolved before review.".to_string())
    } else {
        Ok(())
    }
}

fn reconcile_record_resolution(
    transaction: &Transaction<'_>,
    source_record_id: &str,
) -> Result<(), String> {
    let current_state: String = transaction
        .query_row(
            r#"
            SELECT resolution_state
            FROM source_records
            WHERE source_record_id = CAST(?1 AS UUID)
            "#,
            params![source_record_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB source record resolution query failed: {error}"))?;
    let approved_links = approved_link_count(transaction, source_record_id)?;
    let valid = match ResolutionState::parse(&current_state)? {
        ResolutionState::SingleEntity => approved_links == 1,
        ResolutionState::MultipleEntities => approved_links >= 2,
        ResolutionState::NoProductEntity => approved_links == 0,
        ResolutionState::Unresolved => true,
    };
    if !valid {
        transaction
            .execute(
                r#"
                UPDATE source_records
                SET resolution_state = 'unresolved', updated_at = CURRENT_TIMESTAMP
                WHERE source_record_id = CAST(?1 AS UUID)
                "#,
                params![source_record_id],
            )
            .map_err(|error| format!("DuckDB source record resolution reset failed: {error}"))?;
    }
    Ok(())
}

fn validate_source_record_input(input: &SourceRecordUpsert) -> Result<(), String> {
    validate_non_empty("source record key", &input.source_record_key)?;
    validate_non_empty("source record type", &input.record_type)?;
    validate_non_empty("source record seen_at", &input.seen_at)?;
    validate_optional_json("source record metadata", input.metadata_json.as_deref())
}

fn validate_observation_input(input: &NewSourceObservation) -> Result<(), String> {
    validate_non_empty("observation timestamp", &input.observed_at)?;
    validate_non_empty("observation surface", &input.surface)?;
    validate_non_empty("observation kind", &input.observation_kind)?;
    validate_non_empty("observation time window", &input.time_window)?;
    validate_required_json("source observation payload", &input.source_payload_json)?;
    if input.rank.is_some_and(|value| value <= 0) {
        return Err("Observation rank must be greater than zero.".to_string());
    }
    for (label, value) in [
        ("views", input.views),
        ("installs_total", input.installs_total),
        ("installs_period", input.installs_period),
        ("github_stars", input.github_stars),
        ("upvotes", input.upvotes),
    ] {
        if value.is_some_and(|metric| metric < 0) {
            return Err(format!("Observation {label} cannot be negative."));
        }
    }
    Ok(())
}

fn validate_link_input(input: &NewSourceRecordEntityLink) -> Result<(), String> {
    validate_non_empty("source record ID", &input.source_record_id)?;
    validate_non_empty("entity ID", &input.entity_id)?;
    validate_non_empty("link match method", &input.match_method)?;
    validate_optional_json("link evidence", input.evidence_json.as_deref())?;
    if input
        .match_confidence
        .is_some_and(|confidence| !(0.0..=1.0).contains(&confidence))
    {
        return Err("Link match confidence must be between 0 and 1.".to_string());
    }
    Ok(())
}

fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} is required."))
    } else {
        Ok(())
    }
}

fn validate_optional_json(label: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = normalize_optional_text(value) {
        validate_required_json(label, value)?;
    }
    Ok(())
}

fn validate_required_json(label: &str, value: &str) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(value)
        .map(|_| ())
        .map_err(|error| format!("Invalid {label} JSON: {error}"))
}

fn normalize_optional_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn generate_uuid(connection: &Connection) -> Result<String, String> {
    connection
        .query_row("SELECT CAST(uuid() AS VARCHAR)", [], |row| row.get(0))
        .map_err(|error| format!("DuckDB UUID generation failed: {error}"))
}

fn canonical_entity_select_sql() -> &'static str {
    r#"
    SELECT
        CAST(entity_id AS VARCHAR),
        canonical_name,
        normalized_name,
        primary_type,
        status,
        description,
        primary_website,
        primary_repository,
        CAST(created_at AS VARCHAR),
        CAST(updated_at AS VARCHAR)
    FROM canonical_entities
    "#
}

fn collection_run_select_sql() -> &'static str {
    r#"
    SELECT
        CAST(collection_run_id AS VARCHAR),
        source,
        collection_mode,
        scope_json,
        CAST(started_at AS VARCHAR),
        CAST(finished_at AS VARCHAR),
        status,
        records_seen,
        observations_saved,
        error_summary,
        CAST(created_at AS VARCHAR)
    FROM source_collection_runs
    "#
}

fn source_record_select_sql() -> &'static str {
    r#"
    SELECT
        CAST(source_records.source_record_id AS VARCHAR),
        source_records.source,
        source_records.source_record_key,
        source_records.record_type,
        source_records.resolution_state,
        source_records.title,
        source_records.external_url,
        source_records.publisher,
        source_records.description,
        source_records.source_category,
        source_records.repository_url,
        CAST(source_records.published_at AS VARCHAR),
        CAST(source_records.listed_at AS VARCHAR),
        source_records.metadata_json,
        CAST(source_records.first_seen_at AS VARCHAR),
        CAST(source_records.last_seen_at AS VARCHAR),
        CAST(source_records.created_at AS VARCHAR),
        CAST(source_records.updated_at AS VARCHAR)
    FROM source_records
    "#
}

fn source_observation_select_sql() -> &'static str {
    r#"
    SELECT
        CAST(observation_id AS VARCHAR),
        CAST(collection_run_id AS VARCHAR),
        CAST(source_record_id AS VARCHAR),
        CAST(observed_at AS VARCHAR),
        surface,
        observation_kind,
        time_window,
        rank,
        source_score,
        views,
        installs_total,
        installs_period,
        github_stars,
        upvotes,
        payload_hash,
        source_payload_json,
        CAST(created_at AS VARCHAR)
    FROM source_observations
    "#
}

fn source_record_entity_link_select_sql() -> &'static str {
    r#"
    SELECT
        CAST(link_id AS VARCHAR),
        CAST(source_record_id AS VARCHAR),
        CAST(entity_id AS VARCHAR),
        relationship_type,
        match_method,
        match_confidence,
        review_state,
        evidence_json,
        CAST(reviewed_at AS VARCHAR),
        CAST(created_at AS VARCHAR),
        CAST(updated_at AS VARCHAR)
    FROM source_record_entity_links
    "#
}

struct RawCanonicalEntity {
    entity_id: String,
    canonical_name: String,
    normalized_name: String,
    primary_type: String,
    status: String,
    description: Option<String>,
    primary_website: Option<String>,
    primary_repository: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<RawCanonicalEntity> for CanonicalEntity {
    type Error = String;

    fn try_from(raw: RawCanonicalEntity) -> Result<Self, Self::Error> {
        Ok(Self {
            entity_id: raw.entity_id,
            canonical_name: raw.canonical_name,
            normalized_name: raw.normalized_name,
            primary_type: PrimaryEntityType::parse(&raw.primary_type)?,
            status: EntityStatus::parse(&raw.status)?,
            description: raw.description,
            primary_website: raw.primary_website,
            primary_repository: raw.primary_repository,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

fn read_raw_canonical_entity(row: &duckdb::Row<'_>) -> duckdb::Result<RawCanonicalEntity> {
    Ok(RawCanonicalEntity {
        entity_id: row.get(0)?,
        canonical_name: row.get(1)?,
        normalized_name: row.get(2)?,
        primary_type: row.get(3)?,
        status: row.get(4)?,
        description: row.get(5)?,
        primary_website: row.get(6)?,
        primary_repository: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

struct RawCollectionRun {
    collection_run_id: String,
    source: String,
    collection_mode: String,
    scope_json: Option<String>,
    started_at: String,
    finished_at: Option<String>,
    status: String,
    records_seen: i64,
    observations_saved: i64,
    error_summary: Option<String>,
    created_at: String,
}

impl TryFrom<RawCollectionRun> for SourceCollectionRun {
    type Error = String;

    fn try_from(raw: RawCollectionRun) -> Result<Self, Self::Error> {
        Ok(Self {
            collection_run_id: raw.collection_run_id,
            source: ExternalSource::parse(&raw.source)?,
            collection_mode: CollectionMode::parse(&raw.collection_mode)?,
            scope_json: raw.scope_json,
            started_at: raw.started_at,
            finished_at: raw.finished_at,
            status: CollectionRunStatus::parse(&raw.status)?,
            records_seen: raw.records_seen,
            observations_saved: raw.observations_saved,
            error_summary: raw.error_summary,
            created_at: raw.created_at,
        })
    }
}

fn read_raw_collection_run(row: &duckdb::Row<'_>) -> duckdb::Result<RawCollectionRun> {
    Ok(RawCollectionRun {
        collection_run_id: row.get(0)?,
        source: row.get(1)?,
        collection_mode: row.get(2)?,
        scope_json: row.get(3)?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
        status: row.get(6)?,
        records_seen: row.get(7)?,
        observations_saved: row.get(8)?,
        error_summary: row.get(9)?,
        created_at: row.get(10)?,
    })
}

struct RawSourceRecord {
    source_record_id: String,
    source: String,
    source_record_key: String,
    record_type: String,
    resolution_state: String,
    title: Option<String>,
    external_url: Option<String>,
    publisher: Option<String>,
    description: Option<String>,
    source_category: Option<String>,
    repository_url: Option<String>,
    published_at: Option<String>,
    listed_at: Option<String>,
    metadata_json: Option<String>,
    first_seen_at: String,
    last_seen_at: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<RawSourceRecord> for SourceRecord {
    type Error = String;

    fn try_from(raw: RawSourceRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            source_record_id: raw.source_record_id,
            source: ExternalSource::parse(&raw.source)?,
            source_record_key: raw.source_record_key,
            record_type: raw.record_type,
            resolution_state: ResolutionState::parse(&raw.resolution_state)?,
            title: raw.title,
            external_url: raw.external_url,
            publisher: raw.publisher,
            description: raw.description,
            source_category: raw.source_category,
            repository_url: raw.repository_url,
            published_at: raw.published_at,
            listed_at: raw.listed_at,
            metadata_json: raw.metadata_json,
            first_seen_at: raw.first_seen_at,
            last_seen_at: raw.last_seen_at,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

fn read_raw_source_record(row: &duckdb::Row<'_>) -> duckdb::Result<RawSourceRecord> {
    Ok(RawSourceRecord {
        source_record_id: row.get(0)?,
        source: row.get(1)?,
        source_record_key: row.get(2)?,
        record_type: row.get(3)?,
        resolution_state: row.get(4)?,
        title: row.get(5)?,
        external_url: row.get(6)?,
        publisher: row.get(7)?,
        description: row.get(8)?,
        source_category: row.get(9)?,
        repository_url: row.get(10)?,
        published_at: row.get(11)?,
        listed_at: row.get(12)?,
        metadata_json: row.get(13)?,
        first_seen_at: row.get(14)?,
        last_seen_at: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn read_source_observation(row: &duckdb::Row<'_>) -> duckdb::Result<SourceObservation> {
    Ok(SourceObservation {
        observation_id: row.get(0)?,
        collection_run_id: row.get(1)?,
        source_record_id: row.get(2)?,
        observed_at: row.get(3)?,
        surface: row.get(4)?,
        observation_kind: row.get(5)?,
        time_window: row.get(6)?,
        rank: row.get(7)?,
        source_score: row.get(8)?,
        views: row.get(9)?,
        installs_total: row.get(10)?,
        installs_period: row.get(11)?,
        github_stars: row.get(12)?,
        upvotes: row.get(13)?,
        payload_hash: row.get(14)?,
        source_payload_json: row.get(15)?,
        created_at: row.get(16)?,
    })
}

fn read_source_record_entity_link(row: &duckdb::Row<'_>) -> duckdb::Result<SourceRecordEntityLink> {
    let relationship_type: String = row.get(3)?;
    let review_state: String = row.get(6)?;
    Ok(SourceRecordEntityLink {
        link_id: row.get(0)?,
        source_record_id: row.get(1)?,
        entity_id: row.get(2)?,
        relationship_type: RelationshipType::parse(&relationship_type)
            .expect("database relationship_type is protected by a CHECK constraint"),
        match_method: row.get(4)?,
        match_confidence: row.get(5)?,
        review_state: LinkReviewState::parse(&review_state)
            .expect("database review_state is protected by a CHECK constraint"),
        evidence_json: row.get(7)?,
        reviewed_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use duckdb::Connection;

    use super::*;

    const FIRST_SEEN: &str = "2026-08-18T08:00:00Z";
    const SECOND_SEEN: &str = "2026-08-19T08:00:00Z";

    #[test]
    fn initializes_new_and_legacy_databases_additively() {
        let new_path = test_database_path("new-schema");
        cleanup_database_files(&new_path);
        let _repository =
            MultiSourceRepository::open_at(&new_path).expect("new schema should open");
        assert_all_tables_exist(&new_path);
        cleanup_database_files(&new_path);

        let legacy_path = test_database_path("legacy-schema");
        cleanup_database_files(&legacy_path);
        duckdb_service::initialize_legacy_schema_at(&legacy_path)
            .expect("legacy schema should initialize");
        let legacy = Connection::open(&legacy_path).expect("legacy database should open");
        legacy
            .execute(
                "INSERT INTO threads_posts_raw (post_id, text) VALUES ('legacy-post-1', 'legacy row')",
                [],
            )
            .expect("legacy row should insert");
        drop(legacy);

        let _repository =
            MultiSourceRepository::open_at(&legacy_path).expect("legacy schema should upgrade");
        assert_all_tables_exist(&legacy_path);
        let upgraded = Connection::open(&legacy_path).expect("upgraded database should open");
        let legacy_count: i64 = upgraded
            .query_row(
                "SELECT COUNT(*) FROM threads_posts_raw WHERE post_id = 'legacy-post-1'",
                [],
                |row| row.get(0),
            )
            .expect("legacy row should remain readable");
        assert_eq!(legacy_count, 1);
        cleanup_database_files(&legacy_path);
    }

    #[test]
    fn canonical_entities_use_opaque_ids_and_allow_shared_normalized_names() {
        with_repository("canonical-identity", |repository| {
            let claude = repository
                .create_canonical_entity(&test_entity("Claude Code", PrimaryEntityType::AgentTool))
                .expect("Claude Code should be created");
            assert_eq!(claude.entity_id.len(), 36);
            assert_eq!(claude.normalized_name, "claude code");
            let updated_claude = repository
                .update_canonical_entity_metadata(
                    &claude.entity_id,
                    &CanonicalEntityMetadataUpdate {
                        canonical_name: "Claude Code".to_string(),
                        primary_type: PrimaryEntityType::AgentTool,
                        status: EntityStatus::Active,
                        description: Some("Agentic coding tool".to_string()),
                        primary_website: Some("https://example.test/claude-code".to_string()),
                        primary_repository: None,
                    },
                )
                .expect("canonical metadata should update");
            assert_eq!(
                updated_claude.description.as_deref(),
                Some("Agentic coding tool")
            );

            repository
                .create_canonical_entity(&test_entity("Shared Name", PrimaryEntityType::AgentTool))
                .expect("first shared name should be created");
            repository
                .create_canonical_entity(&test_entity(
                    "  Shared   Name  ",
                    PrimaryEntityType::ConnectorPlugin,
                ))
                .expect("second shared name should be created");
            let matches = repository
                .lookup_canonical_entities_by_normalized_name("shared name")
                .expect("shared names should load");
            assert_eq!(matches.len(), 2);
            assert_ne!(matches[0].entity_id, matches[1].entity_id);
        });
    }

    #[test]
    fn controlled_values_reject_invalid_variants() {
        assert!(PrimaryEntityType::parse("unsupported_tool_type").is_err());
        assert!(ExternalSource::parse("unknown_source").is_err());
        assert!(RelationshipType::parse("looks_similar").is_err());
    }

    #[test]
    fn source_record_upsert_preserves_identity_and_first_seen() {
        with_repository("source-record-upsert", |repository| {
            let original = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "tools/claude-code",
                    FIRST_SEEN,
                ))
                .expect("source record should insert");
            let mut updated_input =
                test_record(ExternalSource::ExplainX, "tools/claude-code", SECOND_SEEN);
            updated_input.title = Some("Claude Code Updated".to_string());
            updated_input.description = Some("Updated description".to_string());
            let updated = repository
                .upsert_source_record(&updated_input)
                .expect("source record should update");
            let github = repository
                .upsert_source_record(&test_record(
                    ExternalSource::GitHub,
                    "tools/claude-code",
                    SECOND_SEEN,
                ))
                .expect("same key from another source should insert");

            assert_eq!(original.source_record_id, updated.source_record_id);
            assert_eq!(original.first_seen_at, updated.first_seen_at);
            assert!(updated.last_seen_at > original.last_seen_at);
            assert_eq!(updated.title.as_deref(), Some("Claude Code Updated"));
            assert_eq!(updated.description.as_deref(), Some("Updated description"));
            assert_ne!(updated.source_record_id, github.source_record_id);
            let by_key = repository
                .get_source_record_by_key(ExternalSource::ExplainX, "tools/claude-code")
                .expect("source record key lookup should succeed")
                .expect("source record should exist");
            assert_eq!(by_key.source_record_id, updated.source_record_id);
        });
    }

    #[test]
    fn observations_are_idempotent_per_run_and_historical_across_runs() {
        with_repository("observation-history", |repository| {
            let record = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "tools/claude-code",
                    FIRST_SEEN,
                ))
                .expect("source record should insert");
            let run_one = repository
                .start_collection_run(&test_run(CollectionMode::Manual))
                .expect("first run should start");
            let first = repository
                .append_observation(
                    &run_one.collection_run_id,
                    &record.source_record_id,
                    &test_observation(FIRST_SEEN, 4, 842.0),
                )
                .expect("first observation should insert");
            let duplicate = repository
                .append_observation(
                    &run_one.collection_run_id,
                    &record.source_record_id,
                    &test_observation(FIRST_SEEN, 4, 842.0),
                )
                .expect("same-run duplicate should be safe");
            assert!(first.inserted);
            assert!(!duplicate.inserted);
            assert_eq!(
                first.observation.observation_id,
                duplicate.observation.observation_id
            );
            let completed = repository
                .complete_collection_run(&run_one.collection_run_id)
                .expect("first run should complete");
            assert_eq!(completed.records_seen, 1);
            assert_eq!(completed.observations_saved, 1);

            let run_two = repository
                .start_collection_run(&test_run(CollectionMode::Scheduled))
                .expect("second run should start");
            repository
                .append_observation(
                    &run_two.collection_run_id,
                    &record.source_record_id,
                    &test_observation(SECOND_SEEN, 2, 1150.0),
                )
                .expect("second historical observation should insert");
            repository
                .complete_collection_run(&run_two.collection_run_id)
                .expect("second run should complete");

            let observations = repository
                .list_observations_for_source_record(&record.source_record_id)
                .expect("observations should load");
            assert_eq!(observations.len(), 2);
            assert_eq!(observations[0].rank, Some(4));
            assert_eq!(observations[1].rank, Some(2));
        });
    }

    #[test]
    fn record_observation_and_run_counters_commit_together() {
        with_repository("observation-transaction", |repository| {
            let run = repository
                .start_collection_run(&test_run(CollectionMode::Import))
                .expect("import run should start");
            let (record, observation) = repository
                .upsert_record_and_append_observation(
                    &run.collection_run_id,
                    &test_record(ExternalSource::ExplainX, "agents/ponytail", FIRST_SEEN),
                    &test_observation(FIRST_SEEN, 7, 420.0),
                )
                .expect("record and observation should commit together");
            assert!(observation.inserted);
            assert_eq!(
                observation.observation.source_record_id,
                record.source_record_id
            );
            let run = repository
                .get_collection_run(&run.collection_run_id)
                .expect("run should load")
                .expect("run should exist");
            assert_eq!(run.records_seen, 1);
            assert_eq!(run.observations_saved, 1);
        });
    }

    #[test]
    fn failed_record_observation_transaction_rolls_back_record_and_counters() {
        with_repository("observation-rollback", |repository| {
            let run = repository
                .start_collection_run(&test_run(CollectionMode::Import))
                .expect("ExplainX import run should start");
            let github_record =
                test_record(ExternalSource::GitHub, "anthropics/claude-code", FIRST_SEEN);
            let result = repository.upsert_record_and_append_observation(
                &run.collection_run_id,
                &github_record,
                &test_observation(FIRST_SEEN, 4, 842.0),
            );
            assert!(result.is_err());
            assert!(repository
                .get_source_record_by_key(ExternalSource::GitHub, "anthropics/claude-code")
                .expect("rolled-back record lookup should succeed")
                .is_none());
            let run = repository
                .get_collection_run(&run.collection_run_id)
                .expect("run should load")
                .expect("run should exist");
            assert_eq!(run.records_seen, 0);
            assert_eq!(run.observations_saved, 0);
        });
    }

    #[test]
    fn approved_same_entity_link_is_retrievable_both_directions() {
        with_repository("same-entity-link", |repository| {
            let entity = repository
                .create_canonical_entity(&test_entity("Claude Code", PrimaryEntityType::AgentTool))
                .expect("entity should insert");
            let record = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "tools/claude-code",
                    FIRST_SEEN,
                ))
                .expect("record should insert");
            let link = repository
                .create_source_record_entity_link(&test_link(
                    &record.source_record_id,
                    &entity.entity_id,
                    RelationshipType::SameEntity,
                    LinkReviewState::Approved,
                ))
                .expect("approved link should insert");
            assert_eq!(link.review_state, LinkReviewState::Approved);
            repository
                .set_source_record_resolution_state(
                    &record.source_record_id,
                    ResolutionState::SingleEntity,
                )
                .expect("single entity state should validate");
            assert_eq!(
                repository
                    .get_links_for_source_record(&record.source_record_id)
                    .expect("record links should load")
                    .len(),
                1
            );
            let entity_records = repository
                .get_records_for_entity(&entity.entity_id)
                .expect("entity records should load");
            assert_eq!(entity_records.len(), 1);
            assert_eq!(entity_records[0].source_record_id, record.source_record_id);
        });
    }

    #[test]
    fn child_multi_entity_no_product_and_ambiguous_states_remain_distinct() {
        with_repository("identity-relations", |repository| {
            let cursor = repository
                .create_canonical_entity(&test_entity("Cursor", PrimaryEntityType::AgentTool))
                .expect("Cursor should insert");
            let child = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "slash-commands/cursor-model",
                    FIRST_SEEN,
                ))
                .expect("child record should insert");
            let child_link = repository
                .create_source_record_entity_link(&test_link(
                    &child.source_record_id,
                    &cursor.entity_id,
                    RelationshipType::ChildResource,
                    LinkReviewState::Approved,
                ))
                .expect("child link should insert");
            assert_eq!(
                child_link.relationship_type,
                RelationshipType::ChildResource
            );
            assert_ne!(child.source_record_id, cursor.entity_id);

            let article = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "blog/top-10-agent-harnesses",
                    FIRST_SEEN,
                ))
                .expect("article should insert");
            for name in ["Claude Code", "OpenCode", "Cline"] {
                let entity = repository
                    .create_canonical_entity(&test_entity(name, PrimaryEntityType::AgentTool))
                    .expect("article entity should insert");
                repository
                    .create_source_record_entity_link(&test_link(
                        &article.source_record_id,
                        &entity.entity_id,
                        RelationshipType::MentionedEntity,
                        LinkReviewState::Approved,
                    ))
                    .expect("mentioned link should insert");
            }
            repository
                .set_source_record_resolution_state(
                    &article.source_record_id,
                    ResolutionState::MultipleEntities,
                )
                .expect("multi-entity state should validate");
            assert_eq!(
                repository
                    .get_links_for_source_record(&article.source_record_id)
                    .expect("article links should load")
                    .len(),
                3
            );

            let mut graph_article = test_record(
                ExternalSource::ExplainX,
                "blog/graph-engineering",
                FIRST_SEEN,
            );
            graph_article.resolution_state = ResolutionState::NoProductEntity;
            let graph_article = repository
                .upsert_source_record(&graph_article)
                .expect("no-product article should insert");
            assert!(repository
                .get_links_for_source_record(&graph_article.source_record_id)
                .expect("no-product links should load")
                .is_empty());

            let codex = repository
                .create_canonical_entity(&test_entity("Codex CLI", PrimaryEntityType::AgentTool))
                .expect("Codex CLI should insert");
            let codex_record = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "mcp-servers/codex-cli",
                    FIRST_SEEN,
                ))
                .expect("Codex-like record should insert");
            let ambiguous = repository
                .create_source_record_entity_link(&test_link(
                    &codex_record.source_record_id,
                    &codex.entity_id,
                    RelationshipType::RelatedEntity,
                    LinkReviewState::Ambiguous,
                ))
                .expect("ambiguous related link should insert");
            assert_eq!(ambiguous.review_state, LinkReviewState::Ambiguous);
            assert_eq!(ambiguous.relationship_type, RelationshipType::RelatedEntity);
            assert_eq!(
                repository
                    .get_source_record(&codex_record.source_record_id)
                    .expect("Codex-like record should load")
                    .expect("Codex-like record should exist")
                    .resolution_state,
                ResolutionState::Unresolved
            );
        });
    }

    #[test]
    fn collection_runs_preserve_completed_partial_and_failed_states() {
        with_repository("collection-statuses", |repository| {
            let completed = repository
                .start_collection_run(&test_run(CollectionMode::Manual))
                .expect("completed run should start");
            let completed = repository
                .complete_collection_run(&completed.collection_run_id)
                .expect("run should complete");
            assert_eq!(completed.status, CollectionRunStatus::Completed);

            let partial = repository
                .start_collection_run(&test_run(CollectionMode::Scheduled))
                .expect("partial run should start");
            let partial = repository
                .mark_collection_run_partial(&partial.collection_run_id, Some("one safe failure"))
                .expect("run should become partial");
            assert_eq!(partial.status, CollectionRunStatus::Partial);

            let failed = repository
                .start_collection_run(&test_run(CollectionMode::Replay))
                .expect("failed run should start");
            let failed = repository
                .mark_collection_run_failed(&failed.collection_run_id, Some("safe failure"))
                .expect("run should fail");
            assert_eq!(failed.status, CollectionRunStatus::Failed);
            assert!(repository
                .complete_collection_run(&failed.collection_run_id)
                .is_err());
        });
    }

    #[test]
    fn high_confidence_does_not_auto_approve_identity() {
        with_repository("confidence-safety", |repository| {
            let entity = repository
                .create_canonical_entity(&test_entity("Cursor", PrimaryEntityType::AgentTool))
                .expect("Cursor should insert");
            let record = repository
                .upsert_source_record(&test_record(
                    ExternalSource::ExplainX,
                    "tools/cursor",
                    FIRST_SEEN,
                ))
                .expect("Cursor record should insert");
            let mut link = test_link(
                &record.source_record_id,
                &entity.entity_id,
                RelationshipType::SameEntity,
                LinkReviewState::Pending,
            );
            link.match_confidence = Some(1.0);
            let link = repository
                .create_source_record_entity_link(&link)
                .expect("pending link should insert");
            assert_eq!(link.review_state, LinkReviewState::Pending);
            assert!(link.reviewed_at.is_none());
            let approved = repository
                .update_link_review_state(&link.link_id, LinkReviewState::Approved)
                .expect("explicit review should approve the link");
            assert_eq!(approved.review_state, LinkReviewState::Approved);
            assert!(approved.reviewed_at.is_some());
        });
    }

    fn test_entity(name: &str, primary_type: PrimaryEntityType) -> NewCanonicalEntity {
        NewCanonicalEntity {
            canonical_name: name.to_string(),
            primary_type,
            description: None,
            primary_website: None,
            primary_repository: None,
        }
    }

    fn test_run(collection_mode: CollectionMode) -> NewCollectionRun {
        NewCollectionRun {
            source: ExternalSource::ExplainX,
            collection_mode,
            scope_json: Some(r#"{"surface":"trending","window":"7d"}"#.to_string()),
        }
    }

    fn test_record(
        source: ExternalSource,
        source_record_key: &str,
        seen_at: &str,
    ) -> SourceRecordUpsert {
        SourceRecordUpsert {
            source,
            source_record_key: source_record_key.to_string(),
            record_type: "tool_profile".to_string(),
            resolution_state: ResolutionState::Unresolved,
            title: Some("Claude Code".to_string()),
            external_url: Some(format!("https://example.test/{source_record_key}")),
            publisher: None,
            description: Some("Initial description".to_string()),
            source_category: Some("developer-tools".to_string()),
            repository_url: None,
            published_at: None,
            listed_at: None,
            metadata_json: Some(r#"{"contract":"test"}"#.to_string()),
            seen_at: seen_at.to_string(),
        }
    }

    fn test_observation(observed_at: &str, rank: i64, source_score: f64) -> NewSourceObservation {
        NewSourceObservation {
            observed_at: observed_at.to_string(),
            surface: "trending".to_string(),
            observation_kind: "ranking_snapshot".to_string(),
            time_window: "7d".to_string(),
            rank: Some(rank),
            source_score: Some(source_score),
            views: Some(100),
            installs_total: None,
            installs_period: None,
            github_stars: None,
            upvotes: None,
            payload_hash: Some(format!("rank-{rank}")),
            source_payload_json: format!(r#"{{"rank":{rank},"score":{source_score}}}"#),
        }
    }

    fn test_link(
        source_record_id: &str,
        entity_id: &str,
        relationship_type: RelationshipType,
        review_state: LinkReviewState,
    ) -> NewSourceRecordEntityLink {
        NewSourceRecordEntityLink {
            source_record_id: source_record_id.to_string(),
            entity_id: entity_id.to_string(),
            relationship_type,
            match_method: "manual_test".to_string(),
            match_confidence: Some(0.99),
            review_state,
            evidence_json: Some(r#"{"evidence":"test"}"#.to_string()),
        }
    }

    fn with_repository<F>(name: &str, test: F)
    where
        F: FnOnce(&MultiSourceRepository),
    {
        let path = test_database_path(name);
        cleanup_database_files(&path);
        let repository =
            MultiSourceRepository::open_at(&path).expect("test repository should open");
        test(&repository);
        drop(repository);
        cleanup_database_files(&path);
    }

    fn test_database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai-agent-trend-radar-{name}.duckdb"))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }

    fn assert_all_tables_exist(path: &Path) {
        let connection = Connection::open(path).expect("database should open for table inspection");
        for table in [
            "threads_posts_raw",
            "crawl_runs",
            "agent_mentions",
            "entity_review_decisions",
            "weekly_agent_metrics",
            "canonical_entities",
            "source_collection_runs",
            "source_records",
            "source_observations",
            "source_record_entity_links",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .expect("table count should be readable");
            assert_eq!(count, 1, "missing table: {table}");
        }
    }
}
