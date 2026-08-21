use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use duckdb::{params, Connection, OptionalExt};
use serde_json::Value;

use crate::models::explainx::{ExplainXImportPreview, ExplainXImportResult, ExplainXRecordInput};
use crate::models::multi_source::{
    normalize_entity_name, AliasEntityMatch, AliasSourceScope, CollectionMode, ExternalSource,
    LinkReviewState, NewCollectionRun, NewSourceObservation, NewSourceRecordEntityLink,
    RelationshipType, ResolutionState, SourceRecordUpsert,
};
use crate::services::duckdb_service;
use crate::services::identity_bootstrap;
use crate::services::multi_source_repository::MultiSourceRepository;
use crate::utils::config;

const PREVIEW_LIMIT: usize = 12;

pub fn import_explainx_records(file_path: String) -> Result<ExplainXImportResult, String> {
    let source_path = resolve_import_path(&file_path)?;
    let database_path = duckdb_service::initialize_database()?;
    import_explainx_records_at(&source_path, &database_path)
}

fn import_explainx_records_at(
    source_path: &Path,
    database_path: &Path,
) -> Result<ExplainXImportResult, String> {
    let contents = fs::read(source_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "ExplainX import file was not found: {}",
                source_path.display()
            )
        } else {
            format!("ExplainX import file could not be read: {error}")
        }
    })?;
    let payload: Value = serde_json::from_slice(&contents)
        .map_err(|error| format!("Invalid ExplainX JSON: {error}"))?;
    let items = payload.as_array().ok_or_else(|| {
        "Unsupported ExplainX JSON shape: expected an array of records.".to_string()
    })?;
    if items.is_empty() {
        return Err("ExplainX dataset is empty.".to_string());
    }

    let mut records = Vec::new();
    let mut invalid = 0;
    for item in items {
        match normalize_record(item) {
            Ok(record) => records.push(record),
            Err(_) => invalid += 1,
        }
    }
    if records.is_empty() {
        return Err(
            "ExplainX dataset has no valid records. Each record must be an object with a non-empty name."
                .to_string(),
        );
    }

    let repository = MultiSourceRepository::open_at(database_path)?;
    identity_bootstrap::bootstrap_curated_aliases(&repository)?;
    let store = ExplainXRecordStore::new(repository.connection());
    let seen_at = store.current_timestamp()?;
    let run = repository.start_collection_run(&NewCollectionRun {
        source: ExternalSource::ExplainX,
        collection_mode: CollectionMode::Import,
        scope_json: Some(format!(
            r#"{{"file_name":{}}}"#,
            serde_json::to_string(
                &source_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("local-import.json")
            )
            .map_err(|error| format!("ExplainX import scope serialization failed: {error}"))?
        )),
    })?;

    let result = process_records(
        &repository,
        &store,
        &records,
        items.len(),
        invalid,
        &run.collection_run_id,
        &seen_at,
    );

    match result {
        Ok(result) => {
            if invalid == 0 {
                repository.complete_collection_run(&run.collection_run_id)?;
            } else {
                repository.mark_collection_run_partial(
                    &run.collection_run_id,
                    Some("Some ExplainX records were skipped because they had no usable name."),
                )?;
            }
            Ok(result)
        }
        Err(error) => {
            let _ = repository.mark_collection_run_failed(
                &run.collection_run_id,
                Some("ExplainX local import failed while persisting validated records."),
            );
            Err(error)
        }
    }
}

fn process_records(
    repository: &MultiSourceRepository,
    store: &ExplainXRecordStore,
    records: &[ExplainXRecordInput],
    imported: usize,
    invalid: usize,
    ingestion_batch_id: &str,
    seen_at: &str,
) -> Result<ExplainXImportResult, String> {
    let mut inserted = 0;
    let mut updated = 0;
    let mut skipped = 0;
    let mut linked_exact_alias = 0;
    let mut review_needed = 0;
    let mut unlinked = 0;
    let mut sample_records = Vec::new();

    for record in records {
        // DuckDB rejects updates to a parent row after links or observations reference it.
        // Preserve that durable identity and keep mutable source payload in explainx_records.
        let source_record = match repository
            .get_source_record_by_key(ExternalSource::ExplainX, &record.source_record_key)?
        {
            Some(existing) => existing,
            None => repository.upsert_source_record(&SourceRecordUpsert {
                source: ExternalSource::ExplainX,
                source_record_key: record.source_record_key.clone(),
                record_type: record.record_type.clone(),
                resolution_state: ResolutionState::Unresolved,
                title: Some(record.name.clone()),
                external_url: record.url.clone().or_else(|| record.source_url.clone()),
                publisher: None,
                description: record.description.clone(),
                source_category: record.category.clone(),
                repository_url: None,
                published_at: None,
                listed_at: None,
                metadata_json: Some(record.raw_json.clone()),
                seen_at: seen_at.to_string(),
            })?,
        };
        repository.append_observation(
            ingestion_batch_id,
            &source_record.source_record_id,
            &NewSourceObservation {
                observed_at: seen_at.to_string(),
                surface: record.record_type.clone(),
                observation_kind: "registry_import".to_string(),
                time_window: "none".to_string(),
                rank: None,
                source_score: None,
                views: None,
                installs_total: None,
                installs_period: None,
                github_stars: None,
                upvotes: None,
                payload_hash: None,
                source_payload_json: record.raw_json.clone(),
            },
        )?;

        match store.upsert(record, &source_record.source_record_id, ingestion_batch_id)? {
            UpsertOutcome::Inserted => inserted += 1,
            UpsertOutcome::Updated => updated += 1,
            UpsertOutcome::Skipped => skipped += 1,
        }

        let identity = resolve_identity(repository, record, &source_record.source_record_id)?;
        match identity.status.as_str() {
            "linked_exact_alias" => linked_exact_alias += 1,
            "review_needed" => review_needed += 1,
            _ => unlinked += 1,
        }
        if sample_records.len() < PREVIEW_LIMIT {
            sample_records.push(ExplainXImportPreview {
                source_record_key: record.source_record_key.clone(),
                name: record.name.clone(),
                category: record
                    .category
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                tags: record.tags.clone(),
                identity_status: identity.status,
                matched_canonical_entity: identity
                    .matched_canonical_entity
                    .unwrap_or_else(|| "none".to_string()),
                reason: identity.reason,
            });
        }
    }

    Ok(ExplainXImportResult {
        imported,
        inserted,
        updated,
        skipped,
        invalid,
        linked_exact_alias,
        review_needed,
        unlinked,
        ingestion_batch_id: ingestion_batch_id.to_string(),
        message: format!(
            "ExplainX import processed {imported} records: {inserted} inserted, {updated} updated, {skipped} unchanged, {invalid} invalid."
        ),
        sample_records,
    })
}

struct IdentityResolution {
    status: String,
    matched_canonical_entity: Option<String>,
    reason: String,
}

fn resolve_identity(
    repository: &MultiSourceRepository,
    record: &ExplainXRecordInput,
    source_record_id: &str,
) -> Result<IdentityResolution, String> {
    let existing_links = repository.get_links_for_source_record(source_record_id)?;
    if let Some(link) = existing_links
        .iter()
        .find(|link| link.review_state == LinkReviewState::Approved)
    {
        let entity = repository.get_canonical_entity(&link.entity_id)?;
        return Ok(IdentityResolution {
            status: "linked_exact_alias".to_string(),
            matched_canonical_entity: entity.map(|entity| entity.canonical_name),
            reason:
                "An explicit external identity review already approved this source record link."
                    .to_string(),
        });
    }

    let matches = preferred_scope_matches(
        repository.lookup_entities_by_normalized_alias(&record.name, AliasSourceScope::ExplainX)?,
    );
    if matches.is_empty() {
        return Ok(IdentityResolution {
            status: "unlinked".to_string(),
            matched_canonical_entity: None,
            reason: "No active exact alias exists in ExplainX or global scope.".to_string(),
        });
    }

    let entity_ids = matches
        .iter()
        .map(|candidate| candidate.entity.entity_id.as_str())
        .collect::<HashSet<_>>();
    if entity_ids.len() != 1 || matches.iter().any(|candidate| candidate.alias.is_ambiguous) {
        return Ok(IdentityResolution {
            status: "review_needed".to_string(),
            matched_canonical_entity: None,
            reason: if entity_ids.len() != 1 {
                format!(
                    "Exact name matches {} canonical entities; no automatic link was created.",
                    entity_ids.len()
                )
            } else {
                "The exact alias is marked ambiguous; no automatic link was created.".to_string()
            },
        });
    }

    let candidate = &matches[0];
    let child_resource = looks_like_child_resource(record);
    if let Some(existing) = existing_links
        .iter()
        .find(|link| link.entity_id == candidate.entity.entity_id)
    {
        let status = if !child_resource && existing.review_state == LinkReviewState::Pending {
            "linked_exact_alias"
        } else {
            "review_needed"
        };
        return Ok(IdentityResolution {
            status: status.to_string(),
            matched_canonical_entity: Some(candidate.entity.canonical_name.clone()),
            reason: format!(
                "An existing {} link preserves review state {}.",
                existing.relationship_type.as_str(),
                existing.review_state.as_str()
            ),
        });
    }
    if existing_links.iter().any(|link| {
        matches!(
            link.review_state,
            LinkReviewState::Pending | LinkReviewState::Ambiguous
        )
    }) {
        return Ok(IdentityResolution {
            status: "review_needed".to_string(),
            matched_canonical_entity: Some(candidate.entity.canonical_name.clone()),
            reason: "Another unresolved canonical candidate already exists for this source record."
                .to_string(),
        });
    }

    let relationship_type = if child_resource {
        RelationshipType::ChildResource
    } else {
        RelationshipType::SameEntity
    };
    repository.create_source_record_entity_link(&NewSourceRecordEntityLink {
        source_record_id: source_record_id.to_string(),
        entity_id: candidate.entity.entity_id.clone(),
        relationship_type,
        match_method: "exact_active_alias".to_string(),
        match_confidence: Some(1.0),
        review_state: LinkReviewState::Pending,
        evidence_json: Some(
            serde_json::json!({
                "source": "explainx",
                "source_record_key": record.source_record_key,
                "matched_alias": candidate.alias.alias,
                "child_resource_signal": child_resource,
            })
            .to_string(),
        ),
    })?;

    Ok(IdentityResolution {
        status: if child_resource {
            "review_needed".to_string()
        } else {
            "linked_exact_alias".to_string()
        },
        matched_canonical_entity: Some(candidate.entity.canonical_name.clone()),
        reason: if child_resource {
            "Exact alias found, but the record looks like a child resource; a pending child-resource link was created for explicit review."
                .to_string()
        } else {
            "One unambiguous active exact alias was found; a pending same-entity link was created without automatic approval."
                .to_string()
        },
    })
}

fn preferred_scope_matches(matches: Vec<AliasEntityMatch>) -> Vec<AliasEntityMatch> {
    let scoped = matches
        .iter()
        .filter(|candidate| candidate.alias.source_scope == AliasSourceScope::ExplainX)
        .cloned()
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        matches
            .into_iter()
            .filter(|candidate| candidate.alias.source_scope == AliasSourceScope::Global)
            .collect()
    } else {
        scoped
    }
}

fn looks_like_child_resource(record: &ExplainXRecordInput) -> bool {
    let context = format!(
        "{} {} {}",
        record.source_record_key,
        record.record_type,
        record.category.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    ["mcp-server", "mcp_server", "skill", "plugin", "command"]
        .iter()
        .any(|term| context.contains(term))
}

fn normalize_record(value: &Value) -> Result<ExplainXRecordInput, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "ExplainX record must be a JSON object.".to_string())?;
    let name = first_string(value, &["name", "title", "display_name", "displayName"])
        .ok_or_else(|| "ExplainX record is missing a required name.".to_string())?;
    let normalized_name = normalize_entity_name(&name);
    if normalized_name.is_empty() {
        return Err("ExplainX record name cannot be empty.".to_string());
    }

    let description = first_string(value, &["description", "summary", "tagline"]);
    let category = first_string(value, &["category", "category_name", "categoryName"]);
    let tags = extract_tags(value);
    let url = first_string(
        value,
        &["url", "permalink", "canonical_url", "canonicalUrl"],
    );
    let source_url = first_string(value, &["source_url", "sourceUrl"]);
    let pricing_text = first_string(value, &["pricing_text", "pricingText", "pricing"]);
    let platform_text = first_string(value, &["platform_text", "platformText", "platform"]);
    let record_type = first_string(
        value,
        &["record_type", "recordType", "type", "kind", "surface"],
    )
    .unwrap_or_else(|| "registry_record".to_string());
    let source_record_key = first_scalar_string(
        value,
        &[
            "source_record_key",
            "sourceRecordKey",
            "external_id",
            "externalId",
            "id",
            "canonical_key",
            "canonicalKey",
            "slug",
            "path",
        ],
    )
    .or_else(|| url.clone())
    .or_else(|| source_url.clone())
    .unwrap_or_else(|| {
        format!(
            "generated:{}:{}:{}",
            normalize_entity_name(&record_type),
            category
                .as_deref()
                .map(normalize_entity_name)
                .unwrap_or_else(|| "uncategorized".to_string()),
            normalized_name
        )
    });
    let raw_json = serde_json::to_string(object)
        .map_err(|error| format!("ExplainX record serialization failed: {error}"))?;

    Ok(ExplainXRecordInput {
        source_record_key,
        name,
        normalized_name,
        description,
        category,
        tags,
        url,
        source_url,
        pricing_text,
        platform_text,
        record_type,
        raw_json,
    })
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn first_scalar_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match value.get(*key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Some(Value::Number(value)) => Some(value.to_string()),
        _ => None,
    })
}

fn extract_tags(value: &Value) -> Vec<String> {
    let Some(tags) = value.get("tags").or_else(|| value.get("tag_list")) else {
        return Vec::new();
    };
    match tags {
        Value::Array(values) => values
            .iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| first_string(value, &["name", "label", "slug"]))
            })
            .collect(),
        Value::String(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn resolve_import_path(file_path: &str) -> Result<PathBuf, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("ExplainX JSON file path is required.".to_string());
    }
    let path = PathBuf::from(trimmed);
    Ok(if path.is_absolute() {
        path
    } else {
        config::project_root().join(path)
    })
}

struct ExplainXRecordStore<'a> {
    connection: &'a Connection,
}

impl<'a> ExplainXRecordStore<'a> {
    fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn current_timestamp(&self) -> Result<String, String> {
        self.connection
            .query_row("SELECT CAST(CURRENT_TIMESTAMP AS VARCHAR)", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("DuckDB ExplainX timestamp query failed: {error}"))
    }

    fn upsert(
        &self,
        record: &ExplainXRecordInput,
        source_record_id: &str,
        ingestion_batch_id: &str,
    ) -> Result<UpsertOutcome, String> {
        let existing_raw_json: Option<String> = self
            .connection
            .query_row(
                "SELECT raw_json FROM explainx_records WHERE source_record_key = ?1",
                params![record.source_record_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("DuckDB ExplainX record lookup failed: {error}"))?;
        let outcome = match existing_raw_json.as_deref() {
            None => UpsertOutcome::Inserted,
            Some(existing) if existing == record.raw_json => UpsertOutcome::Skipped,
            Some(_) => UpsertOutcome::Updated,
        };
        let tags_json = serde_json::to_string(&record.tags)
            .map_err(|error| format!("ExplainX tags serialization failed: {error}"))?;

        let changed = self
            .connection
            .execute(
                r#"
                INSERT INTO explainx_records (
                    source_record_id,
                    source_record_key,
                    name,
                    normalized_name,
                    description,
                    category,
                    tags_json,
                    url,
                    source_url,
                    pricing_text,
                    platform_text,
                    raw_json,
                    last_seen_at,
                    ingestion_batch_id,
                    status
                ) VALUES (
                    CAST(?1 AS UUID), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    CURRENT_TIMESTAMP, CAST(?13 AS UUID), 'active'
                )
                ON CONFLICT (source_record_key) DO UPDATE SET
                    source_record_id = excluded.source_record_id,
                    name = excluded.name,
                    normalized_name = excluded.normalized_name,
                    description = excluded.description,
                    category = excluded.category,
                    tags_json = excluded.tags_json,
                    url = excluded.url,
                    source_url = excluded.source_url,
                    pricing_text = excluded.pricing_text,
                    platform_text = excluded.platform_text,
                    raw_json = excluded.raw_json,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.last_seen_at,
                    ingestion_batch_id = excluded.ingestion_batch_id,
                    status = 'active'
                "#,
                params![
                    source_record_id,
                    record.source_record_key,
                    record.name,
                    record.normalized_name,
                    record.description,
                    record.category,
                    tags_json,
                    record.url,
                    record.source_url,
                    record.pricing_text,
                    record.platform_text,
                    record.raw_json,
                    ingestion_batch_id
                ],
            )
            .map_err(|error| format!("DuckDB ExplainX record upsert failed: {error}"))?;
        if changed == 0 {
            return Err(format!(
                "DuckDB ExplainX record upsert did not persist source key {}.",
                record.source_record_key
            ));
        }
        Ok(outcome)
    }
}

enum UpsertOutcome {
    Inserted,
    Updated,
    Skipped,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn valid_import_is_idempotent_and_links_only_safe_exact_aliases() {
        with_test_files("valid-idempotent", |database_path, fixture_path| {
            write_fixture(
                fixture_path,
                r#"[
                    {
                        "id": "tools/claude-code",
                        "name": "Claude Code",
                        "description": "Agentic coding tool",
                        "category": "AI coding agent",
                        "tags": ["agent", "coding"],
                        "url": "https://example.test/tools/claude-code"
                    },
                    {
                        "id": "tools/codex",
                        "name": "Codex",
                        "category": "AI coding agent"
                    },
                    {
                        "id": "tools/new-tool",
                        "name": "NewToolFromExplainX",
                        "category": "AI agent"
                    },
                    {
                        "id": "mcp-servers/claude-code",
                        "name": "Claude Code",
                        "type": "mcp-server",
                        "category": "MCP server"
                    }
                ]"#,
            );

            let before_weekly = table_count(database_path, "weekly_entity_metrics");
            let before_candidates = table_count(database_path, "entity_review_decisions");
            let first = import_explainx_records_at(fixture_path, database_path)
                .expect("valid ExplainX import should succeed");
            assert_eq!(first.imported, 4);
            assert_eq!(first.inserted, 4);
            assert_eq!(first.invalid, 0);
            assert_eq!(first.linked_exact_alias, 1);
            assert_eq!(first.review_needed, 2);
            assert_eq!(first.unlinked, 1);
            assert!(first.sample_records.iter().any(|record| {
                record.name == "Claude Code"
                    && record.identity_status == "linked_exact_alias"
                    && record.matched_canonical_entity == "Claude Code"
            }));
            assert!(first.sample_records.iter().any(|record| {
                record.name == "Codex" && record.identity_status == "review_needed"
            }));
            assert!(first.sample_records.iter().any(|record| {
                record.name == "NewToolFromExplainX" && record.identity_status == "unlinked"
            }));
            assert!(first.sample_records.iter().any(|record| {
                record.source_record_key == "mcp-servers/claude-code"
                    && record.identity_status == "review_needed"
            }));

            let connection = Connection::open(database_path).expect("test database should open");
            let raw_json: String = connection
                .query_row(
                    "SELECT raw_json FROM explainx_records WHERE source_record_key = 'tools/claude-code'",
                    [],
                    |row| row.get(0),
                )
                .expect("raw JSON should be retained");
            assert!(raw_json.contains("Agentic coding tool"));
            let record_count: i64 = connection
                .query_row("SELECT COUNT(*) FROM explainx_records", [], |row| {
                    row.get(0)
                })
                .expect("ExplainX count should load");
            assert_eq!(record_count, 4);
            let codex_link_count: i64 = connection
                .query_row(
                    r#"
                    SELECT COUNT(*)
                    FROM source_record_entity_links links
                    JOIN source_records records ON records.source_record_id = links.source_record_id
                    WHERE records.source = 'explainx' AND records.source_record_key = 'tools/codex'
                    "#,
                    [],
                    |row| row.get(0),
                )
                .expect("Codex links should be queryable");
            assert_eq!(codex_link_count, 0);
            drop(connection);

            let second = import_explainx_records_at(fixture_path, database_path)
                .expect("second ExplainX import should succeed");
            assert_eq!(second.inserted, 0);
            assert_eq!(second.updated, 0);
            assert_eq!(second.skipped, 4);
            assert_eq!(table_count(database_path, "explainx_records"), 4);
            assert_eq!(
                table_count(database_path, "weekly_entity_metrics"),
                before_weekly
            );
            assert_eq!(
                table_count(database_path, "entity_review_decisions"),
                before_candidates
            );

            write_fixture(
                fixture_path,
                r#"[
                    {
                        "id": "tools/claude-code",
                        "name": "Claude Code",
                        "description": "Updated description",
                        "category": "AI coding agent",
                        "tags": ["agent", "coding"],
                        "url": "https://example.test/tools/claude-code"
                    },
                    {"id": "tools/codex", "name": "Codex", "category": "AI coding agent"},
                    {"id": "tools/new-tool", "name": "NewToolFromExplainX", "category": "AI agent"},
                    {"id": "mcp-servers/claude-code", "name": "Claude Code", "type": "mcp-server", "category": "MCP server"}
                ]"#,
            );
            let updated = import_explainx_records_at(fixture_path, database_path)
                .expect("changed ExplainX record should update safely");
            assert_eq!(updated.inserted, 0);
            assert_eq!(updated.updated, 1);
            assert_eq!(updated.skipped, 3);
        });
    }

    #[test]
    fn invalid_json_fails_before_any_database_write() {
        with_test_files("invalid-json", |database_path, fixture_path| {
            write_fixture(fixture_path, "{not valid json]");
            let error = import_explainx_records_at(fixture_path, database_path)
                .expect_err("invalid JSON must fail safely");
            assert!(error.contains("Invalid ExplainX JSON"));
            assert_eq!(table_count(database_path, "explainx_records"), 0);
            assert_eq!(table_count(database_path, "source_records"), 0);
        });
    }

    #[test]
    fn missing_name_is_counted_without_blocking_valid_records() {
        with_test_files("missing-name", |database_path, fixture_path| {
            write_fixture(
                fixture_path,
                r#"[
                    {"id": "invalid/missing-name", "description": "No name"},
                    {"id": "tools/cline", "name": "Cline", "tags": "agent, coding"}
                ]"#,
            );
            let result = import_explainx_records_at(fixture_path, database_path)
                .expect("valid rows should import while missing names are counted");
            assert_eq!(result.imported, 2);
            assert_eq!(result.invalid, 1);
            assert_eq!(result.inserted, 1);
            assert_eq!(result.linked_exact_alias, 1);
            assert_eq!(table_count(database_path, "explainx_records"), 1);
        });
    }

    #[test]
    fn unsupported_or_empty_shapes_fail_safely() {
        with_test_files("unsupported-shape", |database_path, fixture_path| {
            write_fixture(fixture_path, r#"{"data": []}"#);
            let unsupported = import_explainx_records_at(fixture_path, database_path)
                .expect_err("object wrapper should be rejected clearly");
            assert!(unsupported.contains("expected an array"));
            write_fixture(fixture_path, "[]");
            let empty = import_explainx_records_at(fixture_path, database_path)
                .expect_err("empty arrays should be rejected clearly");
            assert!(empty.contains("dataset is empty"));
        });
    }

    fn with_test_files<F>(name: &str, test: F)
    where
        F: FnOnce(&Path, &Path),
    {
        let database_path = test_database_path(name);
        let fixture_path =
            std::env::temp_dir().join(format!("ai-agent-trend-radar-explainx-{name}-fixture.json"));
        cleanup_database_files(&database_path);
        let _ = fs::remove_file(&fixture_path);
        duckdb_service::initialize_database_at(&database_path)
            .expect("test schema should initialize");
        test(&database_path, &fixture_path);
        cleanup_database_files(&database_path);
        let _ = fs::remove_file(fixture_path);
    }

    fn write_fixture(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test fixture should write");
    }

    fn table_count(database_path: &Path, table: &str) -> i64 {
        let connection = Connection::open(database_path).expect("test database should open");
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table count should load")
    }

    fn test_database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai-agent-trend-radar-explainx-{name}.duckdb"))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
