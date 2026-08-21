use std::collections::{HashMap, HashSet};

use duckdb::{params, Connection, Transaction};
use serde::Serialize;

use crate::models::cross_source::{
    CrossSourceFactorBreakdown, CrossSourceScoreAggregationResult, CrossSourceScoreDiagnostic,
    CrossSourceScorePreview, CROSS_SOURCE_SCORE_VERSION, EXCLUDED_FROM_SCORE_LABEL,
    NEEDS_REVIEW_LABEL, TRUSTED_RANKING_LABEL, WATCHLIST_LABEL,
};
use crate::services::multi_source_repository::MultiSourceRepository;

const REGION_LIMIT: usize = 20;
const TRUSTED_SOURCE_COUNT: f64 = 3.0;

#[derive(Debug, Clone)]
struct WeeklyScoreInput {
    week_start: String,
    week_end: String,
    entity_id: String,
    canonical_name: String,
    entity_type: String,
    region: String,
    mention_count: usize,
    positive_count: usize,
    negative_count: usize,
    mixed_count: usize,
    cost_positive_count: usize,
    cost_negative_boros_count: usize,
    cost_mixed_count: usize,
    conversation_source_count: usize,
    last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RegistryEvidence {
    canonical_name: String,
    entity_type: String,
    record_keys: Vec<String>,
    last_seen_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct SourceEvidenceSnapshot<'a> {
    conversation_source_count: usize,
    approved_registry_record_count: usize,
    approved_registry_record_keys: &'a [String],
    conversation_last_seen_at: &'a Option<String>,
    registry_last_seen_at: &'a Option<String>,
}

pub fn aggregate_cross_source_entity_scores() -> Result<CrossSourceScoreAggregationResult, String> {
    let repository = MultiSourceRepository::open()?;
    let connection = repository.connection();
    let latest_week = load_latest_week(connection)?;
    let weekly_inputs = match latest_week.as_ref() {
        Some((week_start, _)) => load_weekly_score_inputs(connection, week_start)?,
        None => Vec::new(),
    };
    let registry_by_entity = load_approved_registry_evidence(connection)?;
    let score_rows = compute_score_rows(&weekly_inputs, &registry_by_entity)?;

    replace_score_rows(
        connection,
        CROSS_SOURCE_SCORE_VERSION,
        latest_week.as_ref().map(|week| week.0.as_str()),
        &score_rows,
    )?;

    let scored_entity_ids: HashSet<String> =
        score_rows.iter().map(|row| row.entity_id.clone()).collect();
    let scored_names: HashSet<String> = score_rows
        .iter()
        .map(|row| row.canonical_name.trim().to_lowercase())
        .collect();
    let mut watchlist = build_watchlist(&registry_by_entity, &scored_entity_ids);
    let (mut needs_review, mut excluded_from_score) =
        load_external_diagnostics(connection, &scored_entity_ids)?;
    load_mention_diagnostics(
        connection,
        &scored_names,
        &mut needs_review,
        &mut excluded_from_score,
    )?;
    if let Some((week_start, _)) = latest_week.as_ref() {
        load_unknown_region_diagnostics(connection, week_start, &mut excluded_from_score)?;
    }

    deduplicate_and_sort_diagnostics(&mut watchlist);
    deduplicate_and_sort_diagnostics(&mut needs_review);
    deduplicate_and_sort_diagnostics(&mut excluded_from_score);

    let top_global = rank_region(&score_rows, "global", REGION_LIMIT);
    let top_indonesia = rank_region(&score_rows, "indonesia", REGION_LIMIT);
    let mut factor_breakdown_preview = score_rows.clone();
    factor_breakdown_preview.sort_by(compare_score_rows);
    factor_breakdown_preview.truncate(6);

    let scored_rows = score_rows.len();
    let week_start = latest_week.as_ref().map(|week| week.0.clone());
    let week_end = latest_week.as_ref().map(|week| week.1.clone());
    let message = match week_start.as_deref() {
        Some(week) => format!(
            "Built {scored_rows} trusted cross-source score rows for week {week}. Non-ranked evidence remains diagnostic only."
        ),
        None => "No canonical weekly metrics are available. No trusted score rows were created."
            .to_string(),
    };

    Ok(CrossSourceScoreAggregationResult {
        score_version: CROSS_SOURCE_SCORE_VERSION.to_string(),
        week_start,
        week_end,
        scored_rows,
        trusted_ranking_rows: scored_rows,
        watchlist_rows: watchlist.len(),
        needs_review_rows: needs_review.len(),
        excluded_rows: excluded_from_score.len(),
        top_global,
        top_indonesia,
        factor_breakdown_preview,
        watchlist,
        needs_review,
        excluded_from_score,
        fixture_validation: None,
        message,
    })
}

fn load_latest_week(connection: &Connection) -> Result<Option<(String, String)>, String> {
    connection
        .query_row(
            r#"
            SELECT
                CAST(MAX(week_start) AS VARCHAR),
                CAST(MAX(week_end) FILTER (
                    WHERE week_start = (SELECT MAX(week_start) FROM weekly_entity_metrics)
                ) AS VARCHAR)
            FROM weekly_entity_metrics
            "#,
            [],
            |row| {
                let week_start: Option<String> = row.get(0)?;
                let week_end: Option<String> = row.get(1)?;
                Ok(week_start.zip(week_end))
            },
        )
        .map_err(|error| format!("DuckDB cross-source latest week query failed: {error}"))
}

fn load_weekly_score_inputs(
    connection: &Connection,
    week_start: &str,
) -> Result<Vec<WeeklyScoreInput>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                CAST(metrics.week_start AS VARCHAR),
                CAST(metrics.week_end AS VARCHAR),
                CAST(metrics.entity_id AS VARCHAR),
                entities.canonical_name,
                entities.primary_type,
                metrics.region,
                metrics.mention_count,
                metrics.positive_count,
                metrics.negative_count,
                metrics.mixed_count,
                metrics.cost_positive_count,
                metrics.cost_negative_boros_count,
                metrics.cost_mixed_count,
                metrics.source_count,
                CAST(metrics.last_seen_at AS VARCHAR)
            FROM weekly_entity_metrics metrics
            JOIN canonical_entities entities
                ON entities.entity_id = metrics.entity_id
            WHERE metrics.week_start = CAST(?1 AS DATE)
                AND metrics.region IN ('indonesia', 'global')
                AND metrics.mention_count > 0
                AND entities.status = 'active'
            ORDER BY metrics.region, entities.canonical_name
            "#,
        )
        .map_err(|error| format!("DuckDB cross-source input preparation failed: {error}"))?;
    let rows = statement
        .query_map(params![week_start], |row| {
            Ok(WeeklyScoreInput {
                week_start: row.get(0)?,
                week_end: row.get(1)?,
                entity_id: row.get(2)?,
                canonical_name: row.get(3)?,
                entity_type: row.get(4)?,
                region: row.get(5)?,
                mention_count: i64_to_usize(row.get(6)?)?,
                positive_count: i64_to_usize(row.get(7)?)?,
                negative_count: i64_to_usize(row.get(8)?)?,
                mixed_count: i64_to_usize(row.get(9)?)?,
                cost_positive_count: i64_to_usize(row.get(10)?)?,
                cost_negative_boros_count: i64_to_usize(row.get(11)?)?,
                cost_mixed_count: i64_to_usize(row.get(12)?)?,
                conversation_source_count: i64_to_usize(row.get(13)?)?,
                last_seen_at: row.get(14)?,
            })
        })
        .map_err(|error| format!("DuckDB cross-source input query failed: {error}"))?;

    let mut inputs = Vec::new();
    for row in rows {
        inputs
            .push(row.map_err(|error| format!("DuckDB cross-source input read failed: {error}"))?);
    }
    Ok(inputs)
}

fn load_approved_registry_evidence(
    connection: &Connection,
) -> Result<HashMap<String, RegistryEvidence>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                CAST(links.entity_id AS VARCHAR),
                entities.canonical_name,
                entities.primary_type,
                records.source_record_key,
                CAST(explainx.last_seen_at AS VARCHAR)
            FROM source_record_entity_links links
            JOIN source_records records
                ON records.source_record_id = links.source_record_id
            JOIN explainx_records explainx
                ON explainx.source_record_id = records.source_record_id
            JOIN canonical_entities entities
                ON entities.entity_id = links.entity_id
            WHERE records.source = 'explainx'
                AND explainx.status = 'active'
                AND entities.status = 'active'
                AND links.relationship_type = 'same_entity'
                AND links.review_state = 'approved'
            ORDER BY links.entity_id, records.source_record_key
            "#,
        )
        .map_err(|error| format!("DuckDB registry evidence preparation failed: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|error| format!("DuckDB registry evidence query failed: {error}"))?;

    let mut evidence_by_entity = HashMap::new();
    for row in rows {
        let (entity_id, canonical_name, entity_type, record_key, last_seen_at) =
            row.map_err(|error| format!("DuckDB registry evidence read failed: {error}"))?;
        let evidence = evidence_by_entity
            .entry(entity_id)
            .or_insert_with(|| RegistryEvidence {
                canonical_name,
                entity_type,
                record_keys: Vec::new(),
                last_seen_at: None,
            });
        if !evidence.record_keys.contains(&record_key) {
            evidence.record_keys.push(record_key);
        }
        if last_seen_at > evidence.last_seen_at {
            evidence.last_seen_at = last_seen_at;
        }
    }
    Ok(evidence_by_entity)
}

fn compute_score_rows(
    inputs: &[WeeklyScoreInput],
    registry_by_entity: &HashMap<String, RegistryEvidence>,
) -> Result<Vec<CrossSourceScorePreview>, String> {
    let mut cohort_maximums: HashMap<(String, String), usize> = HashMap::new();
    for input in inputs {
        let key = (input.week_start.clone(), input.region.clone());
        cohort_maximums
            .entry(key)
            .and_modify(|maximum| *maximum = (*maximum).max(input.mention_count))
            .or_insert(input.mention_count);
    }

    let mut rows = Vec::with_capacity(inputs.len());
    for input in inputs {
        let maximum = cohort_maximums
            .get(&(input.week_start.clone(), input.region.clone()))
            .copied()
            .unwrap_or(input.mention_count);
        let registry = registry_by_entity.get(&input.entity_id);
        rows.push(build_score_row(input, maximum, registry)?);
    }
    Ok(rows)
}

fn build_score_row(
    input: &WeeklyScoreInput,
    cohort_maximum: usize,
    registry: Option<&RegistryEvidence>,
) -> Result<CrossSourceScorePreview, String> {
    let mention_count_score = normalized_mention_score(input.mention_count, cohort_maximum);
    let sentiment_score = sentiment_score(input);
    let cost_signal_score = cost_signal_score(input);
    let approved_registry_record_count = registry.map_or(0, |value| value.record_keys.len());
    let registry_presence_score = if approved_registry_record_count > 0 {
        100.0
    } else {
        0.0
    };
    let review_confidence_score = registry_presence_score;
    let trusted_conversation_sources = input.conversation_source_count.min(2);
    let trusted_surface_count =
        trusted_conversation_sources + usize::from(approved_registry_record_count > 0);
    let source_diversity_score =
        clamp_score(100.0 * trusted_surface_count as f64 / TRUSTED_SOURCE_COUNT);
    let recency_score = 100.0;
    let conversation_score = clamp_score(
        mention_count_score * 0.55
            + sentiment_score * 0.25
            + cost_signal_score * 0.15
            + recency_score * 0.05,
    );
    let cross_source_score = clamp_score(
        conversation_score * 0.55
            + registry_presence_score * 0.20
            + source_diversity_score * 0.10
            + review_confidence_score * 0.10
            + recency_score * 0.05,
    );
    let factors = CrossSourceFactorBreakdown {
        mention_count_score,
        sentiment_score,
        cost_signal_score,
        region_signal_score: mention_count_score,
        registry_presence_score,
        source_diversity_score,
        review_confidence_score,
        recency_score,
        sentiment_adjustment: sentiment_score - 50.0,
        cost_adjustment: cost_signal_score - 50.0,
        conversation_score,
        cross_source_score,
    };
    let empty_record_keys = Vec::new();
    let empty_last_seen = None;
    let record_keys = registry.map_or(empty_record_keys.as_slice(), |value| {
        value.record_keys.as_slice()
    });
    let registry_last_seen = registry.map_or(&empty_last_seen, |value| &value.last_seen_at);
    let source_evidence = SourceEvidenceSnapshot {
        conversation_source_count: input.conversation_source_count,
        approved_registry_record_count,
        approved_registry_record_keys: record_keys,
        conversation_last_seen_at: &input.last_seen_at,
        registry_last_seen_at: registry_last_seen,
    };
    let factor_breakdown_json = serde_json::to_string(&factors)
        .map_err(|error| format!("Cross-source factor serialization failed: {error}"))?;
    let source_evidence_json = serde_json::to_string(&source_evidence)
        .map_err(|error| format!("Cross-source evidence serialization failed: {error}"))?;
    let explanation = format!(
        "{} mention(s), {} conversation source(s), and {} approved ExplainX same-entity record(s).",
        input.mention_count, input.conversation_source_count, approved_registry_record_count
    );

    Ok(CrossSourceScorePreview {
        rank: 0,
        score_version: CROSS_SOURCE_SCORE_VERSION.to_string(),
        week_start: input.week_start.clone(),
        week_end: input.week_end.clone(),
        entity_id: input.entity_id.clone(),
        canonical_name: input.canonical_name.clone(),
        entity_type: input.entity_type.clone(),
        region: input.region.clone(),
        mention_count: input.mention_count,
        approved_registry_record_count,
        conversation_source_count: input.conversation_source_count,
        conversation_score,
        registry_score: registry_presence_score,
        source_diversity_score,
        review_confidence_score,
        recency_score,
        cost_adjustment: factors.cost_adjustment,
        sentiment_adjustment: factors.sentiment_adjustment,
        cross_source_score,
        ranking_label: TRUSTED_RANKING_LABEL.to_string(),
        factor_breakdown_json,
        source_evidence_json,
        explanation,
    })
}

fn normalized_mention_score(mention_count: usize, cohort_maximum: usize) -> f64 {
    if mention_count == 0 || cohort_maximum == 0 {
        return 0.0;
    }
    clamp_score((1.0 + mention_count as f64).ln() / (1.0 + cohort_maximum as f64).ln() * 100.0)
}

fn sentiment_score(input: &WeeklyScoreInput) -> f64 {
    if input.mention_count == 0 {
        return 50.0;
    }
    let denominator = input.mention_count as f64;
    clamp_score(
        50.0 + 50.0 * (input.positive_count as f64 - input.negative_count as f64) / denominator
            - 15.0 * input.mixed_count as f64 / denominator,
    )
}

fn cost_signal_score(input: &WeeklyScoreInput) -> f64 {
    if input.mention_count == 0 {
        return 50.0;
    }
    let denominator = input.mention_count as f64;
    clamp_score(
        50.0 + 50.0 * (input.cost_positive_count as f64 - input.cost_negative_boros_count as f64)
            / denominator
            - 20.0 * input.cost_mixed_count as f64 / denominator,
    )
}

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn replace_score_rows(
    connection: &Connection,
    score_version: &str,
    week_start: Option<&str>,
    rows: &[CrossSourceScorePreview],
) -> Result<(), String> {
    if score_version.trim().is_empty() {
        return Err("Cross-source score version is required.".to_string());
    }
    if !rows.is_empty() && week_start.is_none() {
        return Err("Cross-source score week is required when rows are present.".to_string());
    }
    if week_start.is_some_and(|week| rows.iter().any(|row| row.week_start != week)) {
        return Err("Cross-source score rows must match the rebuild week scope.".to_string());
    }

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB cross-source score transaction failed: {error}"))?;

    if let Err(error) =
        replace_score_rows_in_transaction(&transaction, score_version, week_start, rows)
    {
        let rollback_result = transaction.rollback();
        return match rollback_result {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(format!(
                "{error} DuckDB cross-source rollback also failed: {rollback_error}"
            )),
        };
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB cross-source score commit failed: {error}"))
}

fn replace_score_rows_in_transaction(
    transaction: &Transaction<'_>,
    score_version: &str,
    week_start: Option<&str>,
    rows: &[CrossSourceScorePreview],
) -> Result<(), String> {
    if let Some(week_start) = week_start {
        transaction
            .execute(
                "DELETE FROM cross_source_entity_scores WHERE score_version = ?1 AND week_start = CAST(?2 AS DATE)",
                params![score_version, week_start],
            )
            .map_err(|error| format!("DuckDB cross-source score reset failed: {error}"))?;
    }

    let mut statement = transaction
        .prepare(
            r#"
            INSERT INTO cross_source_entity_scores (
                score_version,
                week_start,
                week_end,
                entity_id,
                canonical_name,
                entity_type,
                region,
                mention_count,
                approved_registry_record_count,
                conversation_source_count,
                mention_count_score,
                sentiment_score,
                cost_signal_score,
                region_signal_score,
                conversation_score,
                registry_score,
                source_diversity_score,
                review_confidence_score,
                recency_score,
                cost_adjustment,
                sentiment_adjustment,
                cross_source_score,
                ranking_label,
                factor_breakdown_json,
                source_evidence_json
            ) VALUES (
                ?1, CAST(?2 AS DATE), CAST(?3 AS DATE), CAST(?4 AS UUID), ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
            )
            "#,
        )
        .map_err(|error| format!("DuckDB cross-source score insert preparation failed: {error}"))?;

    for row in rows {
        let factors: CrossSourceFactorBreakdown = serde_json::from_str(&row.factor_breakdown_json)
            .map_err(|error| format!("Cross-source factor payload is invalid: {error}"))?;
        statement
            .execute(params![
                score_version,
                &row.week_start,
                &row.week_end,
                &row.entity_id,
                &row.canonical_name,
                &row.entity_type,
                &row.region,
                row.mention_count as i64,
                row.approved_registry_record_count as i64,
                row.conversation_source_count as i64,
                factors.mention_count_score,
                factors.sentiment_score,
                factors.cost_signal_score,
                factors.region_signal_score,
                row.conversation_score,
                row.registry_score,
                row.source_diversity_score,
                row.review_confidence_score,
                row.recency_score,
                row.cost_adjustment,
                row.sentiment_adjustment,
                row.cross_source_score,
                &row.ranking_label,
                &row.factor_breakdown_json,
                &row.source_evidence_json
            ])
            .map_err(|error| format!("DuckDB cross-source score insert failed: {error}"))?;
    }
    Ok(())
}

fn build_watchlist(
    registry_by_entity: &HashMap<String, RegistryEvidence>,
    scored_entity_ids: &HashSet<String>,
) -> Vec<CrossSourceScoreDiagnostic> {
    registry_by_entity
        .iter()
        .filter(|(entity_id, _)| !scored_entity_ids.contains(*entity_id))
        .map(|(_, evidence)| CrossSourceScoreDiagnostic {
            entity_name: evidence.canonical_name.clone(),
            entity_type: Some(evidence.entity_type.clone()),
            region: None,
            ranking_label: WATCHLIST_LABEL.to_string(),
            reason: "Approved registry identity exists, but current conversation evidence is required for trusted ranking.".to_string(),
            source_record_key: evidence.record_keys.first().cloned(),
        })
        .collect()
}

fn load_external_diagnostics(
    connection: &Connection,
    scored_entity_ids: &HashSet<String>,
) -> Result<
    (
        Vec<CrossSourceScoreDiagnostic>,
        Vec<CrossSourceScoreDiagnostic>,
    ),
    String,
> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                explainx.name,
                explainx.category,
                records.source_record_key,
                records.resolution_state,
                CAST(links.entity_id AS VARCHAR),
                links.relationship_type,
                links.review_state,
                entities.primary_type,
                entities.status
            FROM explainx_records explainx
            JOIN source_records records
                ON records.source_record_id = explainx.source_record_id
            LEFT JOIN source_record_entity_links links
                ON links.source_record_id = records.source_record_id
            LEFT JOIN canonical_entities entities
                ON entities.entity_id = links.entity_id
            WHERE explainx.status = 'active'
                AND records.source = 'explainx'
            ORDER BY explainx.name, records.source_record_key
            "#,
        )
        .map_err(|error| format!("DuckDB cross-source diagnostic preparation failed: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(|error| format!("DuckDB cross-source diagnostic query failed: {error}"))?;

    let mut needs_review = Vec::new();
    let mut excluded = Vec::new();
    for row in rows {
        let (
            name,
            category,
            source_record_key,
            resolution_state,
            entity_id,
            relationship_type,
            review_state,
            entity_type,
            entity_status,
        ) = row.map_err(|error| format!("DuckDB cross-source diagnostic read failed: {error}"))?;
        if entity_id
            .as_ref()
            .is_some_and(|value| scored_entity_ids.contains(value))
        {
            continue;
        }

        let diagnostic = |label: &str, reason: String| CrossSourceScoreDiagnostic {
            entity_name: name.clone(),
            entity_type: entity_type.clone().or_else(|| category.clone()),
            region: None,
            ranking_label: label.to_string(),
            reason,
            source_record_key: Some(source_record_key.clone()),
        };

        if resolution_state == "no_product_entity" {
            excluded.push(diagnostic(
                EXCLUDED_FROM_SCORE_LABEL,
                "Source record is classified as no product entity.".to_string(),
            ));
            continue;
        }

        match review_state.as_deref() {
            None | Some("pending") => needs_review.push(diagnostic(
                NEEDS_REVIEW_LABEL,
                "External identity has no approved reviewer decision.".to_string(),
            )),
            Some("ambiguous") => needs_review.push(diagnostic(
                NEEDS_REVIEW_LABEL,
                "External identity remains ambiguous and cannot be trusted-scored.".to_string(),
            )),
            Some("rejected") => excluded.push(diagnostic(
                EXCLUDED_FROM_SCORE_LABEL,
                "External identity relationship was rejected.".to_string(),
            )),
            Some("approved")
                if relationship_type.as_deref() == Some("same_entity")
                    && entity_status.as_deref() == Some("active") => {}
            Some("approved") => excluded.push(diagnostic(
                EXCLUDED_FROM_SCORE_LABEL,
                format!(
                    "Approved relationship '{}' is not eligible for IMP-07 registry scoring.",
                    relationship_type.as_deref().unwrap_or("unknown")
                ),
            )),
            Some(value) => excluded.push(diagnostic(
                EXCLUDED_FROM_SCORE_LABEL,
                format!("Unsupported external review state '{value}'."),
            )),
        }
    }
    Ok((needs_review, excluded))
}

fn load_mention_diagnostics(
    connection: &Connection,
    scored_names: &HashSet<String>,
    needs_review: &mut Vec<CrossSourceScoreDiagnostic>,
    excluded: &mut Vec<CrossSourceScoreDiagnostic>,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                agent_name,
                category,
                region,
                COALESCE(identity_resolution_status, 'unresolved'),
                COALESCE(review_status, 'pending')
            FROM agent_mentions
            WHERE entity_id IS NULL
                OR COALESCE(identity_resolution_status, 'unresolved') <> 'resolved'
            GROUP BY agent_name, category, region, identity_resolution_status, review_status
            ORDER BY agent_name
            "#,
        )
        .map_err(|error| format!("DuckDB mention diagnostic preparation failed: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| format!("DuckDB mention diagnostic query failed: {error}"))?;

    for row in rows {
        let (name, category, region, resolution_status, review_status) =
            row.map_err(|error| format!("DuckDB mention diagnostic read failed: {error}"))?;
        if scored_names.contains(&name.trim().to_lowercase()) {
            continue;
        }
        let diagnostic = CrossSourceScoreDiagnostic {
            entity_name: name,
            entity_type: Some(category),
            region: Some(region),
            ranking_label: if review_status == "ignored" {
                EXCLUDED_FROM_SCORE_LABEL.to_string()
            } else {
                NEEDS_REVIEW_LABEL.to_string()
            },
            reason: if review_status == "ignored" {
                "Candidate Review marked this candidate as ignored.".to_string()
            } else {
                format!(
                    "Conversation identity status is '{resolution_status}' and requires review or canonical linkage."
                )
            },
            source_record_key: None,
        };
        if review_status == "ignored" {
            excluded.push(diagnostic);
        } else {
            needs_review.push(diagnostic);
        }
    }
    Ok(())
}

fn load_unknown_region_diagnostics(
    connection: &Connection,
    week_start: &str,
    excluded: &mut Vec<CrossSourceScoreDiagnostic>,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT canonical_name, entity_type
            FROM weekly_entity_metrics
            WHERE week_start = CAST(?1 AS DATE)
                AND region = 'unknown'
            ORDER BY canonical_name
            "#,
        )
        .map_err(|error| format!("DuckDB unknown-region diagnostic preparation failed: {error}"))?;
    let rows = statement
        .query_map(params![week_start], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("DuckDB unknown-region diagnostic query failed: {error}"))?;
    for row in rows {
        let (name, entity_type) =
            row.map_err(|error| format!("DuckDB unknown-region diagnostic read failed: {error}"))?;
        excluded.push(CrossSourceScoreDiagnostic {
            entity_name: name,
            entity_type: Some(entity_type),
            region: Some("unknown".to_string()),
            ranking_label: EXCLUDED_FROM_SCORE_LABEL.to_string(),
            reason: "Unknown-region evidence is not folded into Indonesia or Global ranking."
                .to_string(),
            source_record_key: None,
        });
    }
    Ok(())
}

fn rank_region(
    rows: &[CrossSourceScorePreview],
    region: &str,
    limit: usize,
) -> Vec<CrossSourceScorePreview> {
    let mut ranked: Vec<_> = rows
        .iter()
        .filter(|row| row.region == region)
        .cloned()
        .collect();
    ranked.sort_by(compare_score_rows);
    ranked.truncate(limit);
    for (index, row) in ranked.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    ranked
}

fn compare_score_rows(
    left: &CrossSourceScorePreview,
    right: &CrossSourceScorePreview,
) -> std::cmp::Ordering {
    right
        .cross_source_score
        .total_cmp(&left.cross_source_score)
        .then_with(|| right.mention_count.cmp(&left.mention_count))
        .then_with(|| left.canonical_name.cmp(&right.canonical_name))
}

fn deduplicate_and_sort_diagnostics(diagnostics: &mut Vec<CrossSourceScoreDiagnostic>) {
    let mut seen = HashSet::new();
    diagnostics.retain(|diagnostic| {
        seen.insert((
            diagnostic.ranking_label.clone(),
            diagnostic.entity_name.trim().to_lowercase(),
            diagnostic.region.clone(),
            diagnostic.source_record_key.clone(),
        ))
    });
    diagnostics.sort_by(|left, right| {
        left.entity_name
            .to_lowercase()
            .cmp(&right.entity_name.to_lowercase())
            .then_with(|| left.source_record_key.cmp(&right.source_record_key))
    });
}

fn i64_to_usize(value: i64) -> Result<usize, duckdb::Error> {
    usize::try_from(value).map_err(|error| {
        duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::BigInt, Box::new(error))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use duckdb::Connection;
    use serde::Deserialize;

    use crate::models::cross_source::CrossSourceFixtureValidationResult;
    use crate::services::duckdb_service;

    use super::*;

    const FIXTURE_JSON: &str =
        include_str!("../../../docs/design/fixtures/cross-source-calibration-fixture.json");
    static TEST_DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    #[derive(Debug, Deserialize)]
    struct CalibrationFixture {
        fixture_version: String,
        score_version: String,
        numeric_tolerance: f64,
        week: FixtureWeek,
        entities: Vec<FixtureEntity>,
        expected_rankings: FixtureExpectedRankings,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureWeek {
        week_start: String,
        week_end: String,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureEntity {
        fixture_key: String,
        display_name: String,
        primary_type: String,
        canonical_identity: FixtureCanonicalIdentity,
        conversation: Vec<FixtureConversation>,
        registry: FixtureRegistry,
        expected: FixtureExpected,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureCanonicalIdentity {
        state: String,
        entity_id: Option<String>,
        status: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureConversation {
        region: String,
        mention_count: usize,
        positive_count: usize,
        neutral_count: usize,
        negative_count: usize,
        mixed_count: usize,
        cost_positive_count: usize,
        cost_negative_boros_count: usize,
        cost_mixed_count: usize,
        cost_not_mentioned_count: usize,
        source_types: Vec<String>,
        last_seen_at: String,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureRegistry {
        source: String,
        record_exists: bool,
        record_status: Option<String>,
        relationship_type: Option<String>,
        review_state: Option<String>,
        last_seen_at: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpected {
        label: String,
        score_row_created: Option<bool>,
        global_factors: Option<FixtureExpectedFactors>,
        indonesia_factors: Option<FixtureExpectedFactors>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpectedFactors {
        mention_count_score: f64,
        sentiment_score: f64,
        cost_signal_score: f64,
        region_signal_score: f64,
        registry_presence_score: f64,
        source_diversity_score: f64,
        review_confidence_score: f64,
        recency_score: f64,
        conversation_score: f64,
        cross_source_score: f64,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpectedRankings {
        global: Vec<String>,
        indonesia: Vec<String>,
        watchlist: Vec<String>,
        needs_review: Vec<String>,
        excluded_from_score: Vec<String>,
    }

    #[test]
    fn calibration_fixture_matches_approved_oracle() {
        let result = validate_calibration_fixture();
        assert!(result.passed, "fixture errors: {:?}", result.errors);
        assert!(result.assertions_checked >= 50);
        assert_eq!(result.score_version, CROSS_SOURCE_SCORE_VERSION);
    }

    #[test]
    fn fixture_backed_rebuild_is_idempotent_and_preserves_weekly_tables() {
        with_test_database("fixture-rebuild", |database_path| {
            let fixture = parse_fixture();
            seed_fixture_database(database_path, &fixture);
            let weekly_entity_before = table_snapshot(database_path, "weekly_entity_metrics");
            let weekly_agent_before = table_snapshot(database_path, "weekly_agent_metrics");

            let first = aggregate_cross_source_entity_scores()
                .expect("first cross-source aggregation should succeed");
            let second = aggregate_cross_source_entity_scores()
                .expect("second cross-source aggregation should remain idempotent");

            assert_eq!(first.scored_rows, 4);
            assert_eq!(second.scored_rows, 4);
            assert_eq!(count_score_rows(database_path), 4);
            assert_eq!(
                first
                    .top_global
                    .iter()
                    .map(|row| row.canonical_name.as_str())
                    .collect::<Vec<_>>(),
                vec!["Claude Code", "Ponytail", "Caveman"]
            );
            assert_eq!(first.top_indonesia.len(), 1);
            assert_eq!(first.top_indonesia[0].canonical_name, "Claude Code");
            assert_eq!(first.top_global[0].registry_score, 100.0);
            assert_eq!(first.top_global[1].registry_score, 0.0);
            assert_eq!(first.top_global[1].review_confidence_score, 0.0);
            assert!(first
                .watchlist
                .iter()
                .any(|item| item.entity_name == "FlowPilot"));
            for name in ["NovaForge", "Codex", "UnknownNewTool"] {
                assert!(first
                    .needs_review
                    .iter()
                    .any(|item| item.entity_name == name));
            }
            assert!(first
                .excluded_from_score
                .iter()
                .any(|item| item.entity_name == "MCP Weekly Roundup"));
            assert!(!first
                .top_global
                .iter()
                .any(|item| item.canonical_name == "FlowPilot"));
            assert_eq!(
                table_snapshot(database_path, "weekly_entity_metrics"),
                weekly_entity_before
            );
            assert_eq!(
                table_snapshot(database_path, "weekly_agent_metrics"),
                weekly_agent_before
            );
        });
    }

    #[test]
    fn failed_score_rebuild_rolls_back_previous_rows() {
        with_test_database("rollback", |database_path| {
            let fixture = parse_fixture();
            seed_fixture_database(database_path, &fixture);
            let result = aggregate_cross_source_entity_scores()
                .expect("baseline cross-source aggregation should succeed");
            assert_eq!(result.scored_rows, 4);
            let baseline_count = count_score_rows(database_path);

            let mut invalid_rows = result.top_global.clone();
            invalid_rows[0].region = "invalid-region".to_string();
            let connection = Connection::open(database_path).expect("test database should open");
            let error = replace_score_rows(
                &connection,
                CROSS_SOURCE_SCORE_VERSION,
                Some(&invalid_rows[0].week_start),
                &invalid_rows,
            )
            .expect_err("invalid region should fail the transactional rebuild");
            assert!(error.contains("cross-source score insert failed"));
            drop(connection);

            assert_eq!(count_score_rows(database_path), baseline_count);
        });
    }

    #[test]
    fn latest_week_rebuild_preserves_historical_score_rows() {
        with_test_database("historical-rebuild", |database_path| {
            let fixture = parse_fixture();
            seed_fixture_database(database_path, &fixture);

            let current = aggregate_cross_source_entity_scores()
                .expect("current-week cross-source aggregation should succeed");
            let mut historical_row = current.top_global[0].clone();
            let historical_week_start = "2026-08-10".to_string();
            historical_row.week_start = historical_week_start.clone();
            historical_row.week_end = "2026-08-16".to_string();

            let connection = Connection::open(database_path).expect("test database should open");
            replace_score_rows(
                &connection,
                CROSS_SOURCE_SCORE_VERSION,
                Some(&historical_week_start),
                &[historical_row],
            )
            .expect("historical score row should insert");
            drop(connection);

            let rebuilt = aggregate_cross_source_entity_scores()
                .expect("latest-week cross-source rebuild should succeed");
            assert_eq!(rebuilt.scored_rows, 4);
            assert_eq!(count_score_rows(database_path), 5);
            assert_eq!(count_score_rows_for_week(database_path, "2026-08-10"), 1);
            assert_eq!(count_score_rows_for_week(database_path, "2026-08-17"), 4);
        });
    }

    #[test]
    fn score_schema_initializes_additively_for_existing_weekly_data() {
        with_test_database("additive-schema", |database_path| {
            let connection = Connection::open(database_path).expect("test database should open");
            connection
                .execute(
                    "INSERT INTO weekly_agent_metrics (week_start, week_end, region, agent_name) VALUES (DATE '2026-08-17', DATE '2026-08-23', 'global', 'Legacy Agent')",
                    [],
                )
                .expect("legacy weekly row should insert");
            connection
                .execute_batch("DROP TABLE cross_source_entity_scores;")
                .expect("test should recreate a pre-IMP-07 database");
            drop(connection);

            duckdb_service::initialize_database()
                .expect("additive initialization should restore the score table");
            let connection = Connection::open(database_path).expect("database should reopen");
            let legacy_count: i64 = connection
                .query_row("SELECT COUNT(*) FROM weekly_agent_metrics", [], |row| {
                    row.get(0)
                })
                .expect("legacy weekly data should remain readable");
            let score_table_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'cross_source_entity_scores'",
                    [],
                    |row| row.get(0),
                )
                .expect("score table should exist");
            assert_eq!(legacy_count, 1);
            assert_eq!(score_table_count, 1);
        });
    }

    fn validate_calibration_fixture() -> CrossSourceFixtureValidationResult {
        let fixture = parse_fixture();
        let mut assertions_checked = 0;
        let mut errors = Vec::new();
        record_check(
            fixture.score_version == CROSS_SOURCE_SCORE_VERSION,
            "Fixture score version does not match the implementation version.",
            &mut assertions_checked,
            &mut errors,
        );

        let (inputs, registry_by_entity) = fixture_score_inputs(&fixture);
        let score_rows = match compute_score_rows(&inputs, &registry_by_entity) {
            Ok(rows) => rows,
            Err(error) => {
                errors.push(error);
                Vec::new()
            }
        };
        let global = rank_region(&score_rows, "global", REGION_LIMIT);
        let indonesia = rank_region(&score_rows, "indonesia", REGION_LIMIT);
        record_check(
            global
                .iter()
                .map(|row| row.canonical_name.clone())
                .collect::<Vec<_>>()
                == fixture.expected_rankings.global,
            "Global fixture ranking does not match the oracle.",
            &mut assertions_checked,
            &mut errors,
        );
        record_check(
            indonesia
                .iter()
                .map(|row| row.canonical_name.clone())
                .collect::<Vec<_>>()
                == fixture.expected_rankings.indonesia,
            "Indonesia fixture ranking does not match the oracle.",
            &mut assertions_checked,
            &mut errors,
        );

        let mut actual_watchlist = Vec::new();
        let mut actual_needs_review = Vec::new();
        let mut actual_excluded = Vec::new();
        for entity in &fixture.entities {
            let actual_label = classify_fixture_entity(entity);
            record_check(
                actual_label == entity.expected.label,
                &format!(
                    "{} label was {}, expected {}.",
                    entity.display_name, actual_label, entity.expected.label
                ),
                &mut assertions_checked,
                &mut errors,
            );
            let score_row_created = score_rows
                .iter()
                .any(|row| row.canonical_name == entity.display_name);
            if let Some(expected) = entity.expected.score_row_created {
                record_check(
                    score_row_created == expected,
                    &format!("{} score-row behavior differs.", entity.display_name),
                    &mut assertions_checked,
                    &mut errors,
                );
            }
            match actual_label.as_str() {
                WATCHLIST_LABEL => actual_watchlist.push(entity.display_name.clone()),
                NEEDS_REVIEW_LABEL => actual_needs_review.push(entity.display_name.clone()),
                EXCLUDED_FROM_SCORE_LABEL => actual_excluded.push(entity.display_name.clone()),
                _ => {}
            }

            if let Some(expected) = entity.expected.global_factors.as_ref() {
                compare_expected_factors(
                    &score_rows,
                    entity,
                    "global",
                    expected,
                    fixture.numeric_tolerance,
                    &mut assertions_checked,
                    &mut errors,
                );
            }
            if let Some(expected) = entity.expected.indonesia_factors.as_ref() {
                compare_expected_factors(
                    &score_rows,
                    entity,
                    "indonesia",
                    expected,
                    fixture.numeric_tolerance,
                    &mut assertions_checked,
                    &mut errors,
                );
            }
        }
        record_check(
            actual_watchlist == fixture.expected_rankings.watchlist,
            "Watchlist fixture output does not match the oracle.",
            &mut assertions_checked,
            &mut errors,
        );
        record_check(
            actual_needs_review == fixture.expected_rankings.needs_review,
            "Needs-review fixture output does not match the oracle.",
            &mut assertions_checked,
            &mut errors,
        );
        record_check(
            actual_excluded == fixture.expected_rankings.excluded_from_score,
            "Excluded fixture output does not match the oracle.",
            &mut assertions_checked,
            &mut errors,
        );

        CrossSourceFixtureValidationResult {
            fixture_version: fixture.fixture_version,
            score_version: fixture.score_version,
            passed: errors.is_empty(),
            assertions_checked,
            errors,
        }
    }

    fn parse_fixture() -> CalibrationFixture {
        serde_json::from_str(FIXTURE_JSON).expect("calibration fixture should parse")
    }

    fn fixture_score_inputs(
        fixture: &CalibrationFixture,
    ) -> (Vec<WeeklyScoreInput>, HashMap<String, RegistryEvidence>) {
        let mut inputs = Vec::new();
        let mut registry_by_entity = HashMap::new();
        for entity in &fixture.entities {
            let Some(entity_id) = entity.canonical_identity.entity_id.as_ref() else {
                continue;
            };
            if entity.canonical_identity.state != "resolved"
                || entity.canonical_identity.status.as_deref() != Some("active")
            {
                continue;
            }
            if fixture_registry_is_approved(&entity.registry) {
                registry_by_entity.insert(
                    entity_id.clone(),
                    RegistryEvidence {
                        canonical_name: entity.display_name.clone(),
                        entity_type: entity.primary_type.clone(),
                        record_keys: vec![format!("fixture/{}", entity.fixture_key)],
                        last_seen_at: entity.registry.last_seen_at.clone(),
                    },
                );
            }
            for conversation in &entity.conversation {
                if !matches!(conversation.region.as_str(), "indonesia" | "global") {
                    continue;
                }
                inputs.push(WeeklyScoreInput {
                    week_start: fixture.week.week_start.clone(),
                    week_end: fixture.week.week_end.clone(),
                    entity_id: entity_id.clone(),
                    canonical_name: entity.display_name.clone(),
                    entity_type: entity.primary_type.clone(),
                    region: conversation.region.clone(),
                    mention_count: conversation.mention_count,
                    positive_count: conversation.positive_count,
                    negative_count: conversation.negative_count,
                    mixed_count: conversation.mixed_count,
                    cost_positive_count: conversation.cost_positive_count,
                    cost_negative_boros_count: conversation.cost_negative_boros_count,
                    cost_mixed_count: conversation.cost_mixed_count,
                    conversation_source_count: conversation.source_types.len(),
                    last_seen_at: Some(conversation.last_seen_at.clone()),
                });
            }
        }
        (inputs, registry_by_entity)
    }

    fn classify_fixture_entity(entity: &FixtureEntity) -> String {
        let resolved_active = entity.canonical_identity.state == "resolved"
            && entity.canonical_identity.entity_id.is_some()
            && entity.canonical_identity.status.as_deref() == Some("active");
        if resolved_active && !entity.conversation.is_empty() {
            TRUSTED_RANKING_LABEL.to_string()
        } else if resolved_active && fixture_registry_is_approved(&entity.registry) {
            WATCHLIST_LABEL.to_string()
        } else if matches!(
            entity.canonical_identity.state.as_str(),
            "unresolved" | "ambiguous" | "missing_alias"
        ) {
            NEEDS_REVIEW_LABEL.to_string()
        } else {
            EXCLUDED_FROM_SCORE_LABEL.to_string()
        }
    }

    fn fixture_registry_is_approved(registry: &FixtureRegistry) -> bool {
        registry.record_exists
            && registry.source == "explainx"
            && registry.record_status.as_deref() == Some("active")
            && registry.relationship_type.as_deref() == Some("same_entity")
            && registry.review_state.as_deref() == Some("approved")
    }

    fn compare_expected_factors(
        rows: &[CrossSourceScorePreview],
        entity: &FixtureEntity,
        region: &str,
        expected: &FixtureExpectedFactors,
        tolerance: f64,
        assertions_checked: &mut usize,
        errors: &mut Vec<String>,
    ) {
        let Some(row) = rows
            .iter()
            .find(|row| row.canonical_name == entity.display_name && row.region == region)
        else {
            errors.push(format!(
                "Missing score row for {} in {region}.",
                entity.display_name
            ));
            return;
        };
        let factors: CrossSourceFactorBreakdown = serde_json::from_str(&row.factor_breakdown_json)
            .expect("factor breakdown should deserialize");
        let comparisons = [
            (
                "mention_count_score",
                factors.mention_count_score,
                expected.mention_count_score,
            ),
            (
                "sentiment_score",
                factors.sentiment_score,
                expected.sentiment_score,
            ),
            (
                "cost_signal_score",
                factors.cost_signal_score,
                expected.cost_signal_score,
            ),
            (
                "region_signal_score",
                factors.region_signal_score,
                expected.region_signal_score,
            ),
            (
                "registry_presence_score",
                factors.registry_presence_score,
                expected.registry_presence_score,
            ),
            (
                "source_diversity_score",
                factors.source_diversity_score,
                expected.source_diversity_score,
            ),
            (
                "review_confidence_score",
                factors.review_confidence_score,
                expected.review_confidence_score,
            ),
            (
                "recency_score",
                factors.recency_score,
                expected.recency_score,
            ),
            (
                "conversation_score",
                factors.conversation_score,
                expected.conversation_score,
            ),
            (
                "cross_source_score",
                factors.cross_source_score,
                expected.cross_source_score,
            ),
        ];
        for (factor_name, actual, expected) in comparisons {
            record_check(
                (actual - expected).abs() <= tolerance,
                &format!(
                    "{} {region} {factor_name} was {actual:.4}, expected {expected:.4}.",
                    entity.display_name
                ),
                assertions_checked,
                errors,
            );
        }
    }

    fn record_check(
        condition: bool,
        message: &str,
        assertions_checked: &mut usize,
        errors: &mut Vec<String>,
    ) {
        *assertions_checked += 1;
        if !condition {
            errors.push(message.to_string());
        }
    }

    fn seed_fixture_database(database_path: &Path, fixture: &CalibrationFixture) {
        let connection = Connection::open(database_path).expect("fixture database should open");
        connection
            .execute(
                "INSERT INTO weekly_agent_metrics (week_start, week_end, region, agent_name, trend_score) VALUES (CAST(?1 AS DATE), CAST(?2 AS DATE), 'global', 'Legacy Agent', 10.0)",
                params![&fixture.week.week_start, &fixture.week.week_end],
            )
            .expect("legacy weekly fixture should insert");

        for (index, entity) in fixture.entities.iter().enumerate() {
            if let Some(entity_id) = entity.canonical_identity.entity_id.as_ref() {
                connection
                    .execute(
                        r#"
                        INSERT INTO canonical_entities (
                            entity_id, canonical_name, normalized_name, primary_type, status
                        ) VALUES (CAST(?1 AS UUID), ?2, lower(?2), ?3, ?4)
                        "#,
                        params![
                            entity_id,
                            &entity.display_name,
                            &entity.primary_type,
                            entity
                                .canonical_identity
                                .status
                                .as_deref()
                                .unwrap_or("active")
                        ],
                    )
                    .expect("fixture canonical entity should insert");

                for conversation in &entity.conversation {
                    connection
                        .execute(
                            r#"
                            INSERT INTO weekly_entity_metrics (
                                week_start, week_end, entity_id, canonical_name, entity_type, region,
                                mention_count, positive_count, neutral_count, negative_count,
                                mixed_count, cost_positive_count, cost_negative_boros_count,
                                cost_mixed_count, cost_not_mentioned_count, source_count,
                                last_seen_at, trend_score
                            ) VALUES (
                                CAST(?1 AS DATE), CAST(?2 AS DATE), CAST(?3 AS UUID), ?4, ?5, ?6,
                                ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                                CAST(?17 AS TIMESTAMP), 0.0
                            )
                            "#,
                            params![
                                &fixture.week.week_start,
                                &fixture.week.week_end,
                                entity_id,
                                &entity.display_name,
                                &entity.primary_type,
                                &conversation.region,
                                conversation.mention_count as i64,
                                conversation.positive_count as i64,
                                conversation.neutral_count as i64,
                                conversation.negative_count as i64,
                                conversation.mixed_count as i64,
                                conversation.cost_positive_count as i64,
                                conversation.cost_negative_boros_count as i64,
                                conversation.cost_mixed_count as i64,
                                conversation.cost_not_mentioned_count as i64,
                                conversation.source_types.len() as i64,
                                &conversation.last_seen_at
                            ],
                        )
                        .expect("fixture canonical metric should insert");
                }
            }

            if entity.registry.record_exists {
                insert_fixture_registry_record(&connection, entity, index);
            }
        }

        connection
            .execute(
                r#"
                INSERT INTO threads_posts_raw (post_id, text, source_type, posted_at)
                VALUES (
                    'fixture-unknown-post',
                    'UnknownNewTool appears in a synthetic calibration post.',
                    'fixture',
                    TIMESTAMP '2026-08-22 11:00:00'
                )
                "#,
                [],
            )
            .expect("missing-alias fixture post should insert");
        connection
            .execute(
                r#"
                INSERT INTO agent_mentions (
                    mention_id, post_id, agent_name, category, detection_source,
                    needs_review, review_status, entity_id, identity_resolution_status, region
                ) VALUES (
                    'fixture-unknown-new-tool', 'fixture-unknown-post', 'UnknownNewTool',
                    'unknown_candidate', 'candidate_pattern', TRUE, 'pending', NULL,
                    'missing_alias', 'global'
                )
                "#,
                [],
            )
            .expect("missing-alias fixture mention should insert");
    }

    fn insert_fixture_registry_record(
        connection: &Connection,
        entity: &FixtureEntity,
        index: usize,
    ) {
        let source_record_id = format!("20000000-0000-4000-8000-{index:012}");
        let explainx_id = format!("30000000-0000-4000-8000-{index:012}");
        let source_record_key = format!("fixture/{}", entity.fixture_key);
        let resolution_state = match entity.canonical_identity.state.as_str() {
            "resolved" => "single_entity",
            "no_product_entity" => "no_product_entity",
            _ => "unresolved",
        };
        connection
            .execute(
                r#"
                INSERT INTO source_records (
                    source_record_id, source, source_record_key, record_type, resolution_state,
                    title, first_seen_at, last_seen_at
                ) VALUES (
                    CAST(?1 AS UUID), 'explainx', ?2, 'registry_entry', ?3, ?4,
                    TIMESTAMP '2026-08-20 00:00:00', TIMESTAMP '2026-08-22 00:00:00'
                )
                "#,
                params![
                    &source_record_id,
                    &source_record_key,
                    resolution_state,
                    &entity.display_name
                ],
            )
            .expect("fixture source record should insert");
        connection
            .execute(
                r#"
                INSERT INTO explainx_records (
                    id, source_record_id, source_record_key, name, normalized_name,
                    category, raw_json, ingestion_batch_id, status, last_seen_at
                ) VALUES (
                    CAST(?1 AS UUID), CAST(?2 AS UUID), ?3, ?4, lower(?4), ?5, '{}',
                    CAST('40000000-0000-4000-8000-000000000001' AS UUID), ?6,
                    CAST(?7 AS TIMESTAMP)
                )
                "#,
                params![
                    &explainx_id,
                    &source_record_id,
                    &source_record_key,
                    &entity.display_name,
                    &entity.primary_type,
                    entity.registry.record_status.as_deref().unwrap_or("active"),
                    entity
                        .registry
                        .last_seen_at
                        .as_deref()
                        .unwrap_or("2026-08-22T00:00:00Z")
                ],
            )
            .expect("fixture ExplainX record should insert");

        if let (Some(entity_id), Some(relationship), Some(review_state)) = (
            entity.canonical_identity.entity_id.as_ref(),
            entity.registry.relationship_type.as_ref(),
            entity.registry.review_state.as_ref(),
        ) {
            connection
                .execute(
                    r#"
                    INSERT INTO source_record_entity_links (
                        source_record_id, entity_id, relationship_type, match_method,
                        match_confidence, review_state
                    ) VALUES (
                        CAST(?1 AS UUID), CAST(?2 AS UUID), ?3, 'fixture', 1.0, ?4
                    )
                    "#,
                    params![&source_record_id, entity_id, relationship, review_state],
                )
                .expect("fixture external identity link should insert");
        }
    }

    fn table_snapshot(database_path: &Path, table_name: &str) -> (i64, f64) {
        let connection = Connection::open(database_path).expect("snapshot database should open");
        let query = match table_name {
            "weekly_entity_metrics" => {
                "SELECT COUNT(*), COALESCE(SUM(trend_score), 0.0) FROM weekly_entity_metrics"
            }
            "weekly_agent_metrics" => {
                "SELECT COUNT(*), COALESCE(SUM(trend_score), 0.0) FROM weekly_agent_metrics"
            }
            _ => panic!("unsupported snapshot table"),
        };
        connection
            .query_row(query, [], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("weekly table snapshot should load")
    }

    fn count_score_rows(database_path: &Path) -> i64 {
        let connection = Connection::open(database_path).expect("score database should open");
        connection
            .query_row(
                "SELECT COUNT(*) FROM cross_source_entity_scores",
                [],
                |row| row.get(0),
            )
            .expect("score row count should load")
    }

    fn count_score_rows_for_week(database_path: &Path, week_start: &str) -> i64 {
        let connection = Connection::open(database_path).expect("score database should open");
        connection
            .query_row(
                "SELECT COUNT(*) FROM cross_source_entity_scores WHERE week_start = CAST(?1 AS DATE)",
                params![week_start],
                |row| row.get(0),
            )
            .expect("weekly score row count should load")
    }

    fn with_test_database<F>(name: &str, test: F)
    where
        F: FnOnce(&Path),
    {
        let database_path = test_database_path(name);
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());
        duckdb_service::initialize_database().expect("test database should initialize");
        test(&database_path);
        cleanup_database_files(&database_path);
    }

    fn test_database_path(name: &str) -> PathBuf {
        let sequence = TEST_DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "ai-agent-trend-radar-cross-source-{name}-{}-{sequence}.duckdb",
            std::process::id()
        ))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
