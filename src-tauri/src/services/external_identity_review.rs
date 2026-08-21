use crate::models::external_reviews::{
    ExternalIdentityReviewHistoryEntry, ExternalIdentityReviewHistoryResult,
    ExternalIdentityReviewItem, ExternalIdentityReviewListResult,
    ExternalIdentityReviewSubmissionResult,
};
use crate::models::multi_source::{
    ExternalIdentityDecision, ExternalIdentityReviewRequest, RelationshipType,
};
use crate::services::multi_source_repository::MultiSourceRepository;

pub fn list_external_identity_review_items() -> Result<ExternalIdentityReviewListResult, String> {
    let repository = MultiSourceRepository::open()?;
    list_items(&repository)
}

pub fn submit_external_identity_review(
    link_id: String,
    relationship_type: String,
    decision: String,
    reviewer: String,
    evidence_note: Option<String>,
) -> Result<ExternalIdentityReviewSubmissionResult, String> {
    let repository = MultiSourceRepository::open()?;
    submit_review(
        &repository,
        &link_id,
        &relationship_type,
        &decision,
        &reviewer,
        evidence_note,
    )
}

pub fn get_external_identity_review_history(
    link_id: String,
) -> Result<ExternalIdentityReviewHistoryResult, String> {
    let repository = MultiSourceRepository::open()?;
    load_history(&repository, &link_id)
}

fn list_items(
    repository: &MultiSourceRepository,
) -> Result<ExternalIdentityReviewListResult, String> {
    let mut statement = repository
        .connection()
        .prepare(
            r#"
            WITH latest_reviews AS (
                SELECT
                    link_id,
                    decision,
                    reviewer,
                    review_note,
                    CAST(reviewed_at AS VARCHAR) AS reviewed_at,
                    ROW_NUMBER() OVER (
                        PARTITION BY link_id
                        ORDER BY reviewed_at DESC, created_at DESC, CAST(review_id AS VARCHAR) DESC
                    ) AS review_rank
                FROM external_identity_reviews
            )
            SELECT
                CAST(links.link_id AS VARCHAR),
                records.source,
                CAST(records.source_record_id AS VARCHAR),
                records.source_record_key,
                COALESCE(explainx.name, records.title, records.source_record_key),
                COALESCE(explainx.url, explainx.source_url, records.external_url),
                COALESCE(explainx.description, records.description),
                CAST(entities.entity_id AS VARCHAR),
                entities.canonical_name,
                entities.primary_type,
                links.relationship_type,
                links.review_state,
                links.match_method,
                links.match_confidence,
                links.evidence_json,
                CAST(links.created_at AS VARCHAR),
                CAST(links.updated_at AS VARCHAR),
                latest.decision,
                latest.reviewer,
                latest.review_note,
                latest.reviewed_at
            FROM source_record_entity_links links
            JOIN source_records records
                ON records.source_record_id = links.source_record_id
            JOIN canonical_entities entities
                ON entities.entity_id = links.entity_id
            LEFT JOIN explainx_records explainx
                ON explainx.source_record_id = records.source_record_id
            LEFT JOIN latest_reviews latest
                ON latest.link_id = links.link_id AND latest.review_rank = 1
            WHERE records.source = 'explainx'
            ORDER BY
                CASE links.review_state
                    WHEN 'pending' THEN 0
                    WHEN 'ambiguous' THEN 1
                    WHEN 'approved' THEN 2
                    ELSE 3
                END,
                COALESCE(explainx.name, records.title, records.source_record_key),
                entities.canonical_name
            "#,
        )
        .map_err(|error| {
            format!("DuckDB external identity review list preparation failed: {error}")
        })?;
    let rows = statement
        .query_map([], |row| {
            let match_method: String = row.get(12)?;
            Ok(ExternalIdentityReviewItem {
                link_id: row.get(0)?,
                source: row.get(1)?,
                source_record_id: row.get(2)?,
                source_record_key: row.get(3)?,
                source_record_name: row.get(4)?,
                source_record_url: row.get(5)?,
                source_record_description: row.get(6)?,
                canonical_entity_id: row.get(7)?,
                canonical_entity_name: row.get(8)?,
                canonical_entity_type: row.get(9)?,
                relationship_type: row.get(10)?,
                current_status: row.get(11)?,
                match_reason: describe_match_method(&match_method),
                match_method,
                match_confidence: row.get(13)?,
                evidence: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
                latest_decision: row.get(17)?,
                latest_reviewer: row.get(18)?,
                latest_review_note: row.get(19)?,
                latest_reviewed_at: row.get(20)?,
            })
        })
        .map_err(|error| format!("DuckDB external identity review list query failed: {error}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| {
            format!("DuckDB external identity review list row read failed: {error}")
        })?);
    }
    let pending_count = count_status(&items, "pending");
    let approved_count = count_status(&items, "approved");
    let rejected_count = count_status(&items, "rejected");
    let ambiguous_count = count_status(&items, "ambiguous");

    Ok(ExternalIdentityReviewListResult {
        total: items.len(),
        pending_count,
        approved_count,
        rejected_count,
        ambiguous_count,
        message: if items.is_empty() {
            "No ExplainX identity links are available for review.".to_string()
        } else {
            format!(
                "Loaded {} ExplainX identity links; {} require a decision.",
                items.len(),
                pending_count + ambiguous_count
            )
        },
        items,
    })
}

fn submit_review(
    repository: &MultiSourceRepository,
    link_id: &str,
    relationship_type: &str,
    decision: &str,
    reviewer: &str,
    evidence_note: Option<String>,
) -> Result<ExternalIdentityReviewSubmissionResult, String> {
    let current = find_item(repository, link_id)?;
    let relationship_type = RelationshipType::parse(relationship_type.trim())?;
    let decision = ExternalIdentityDecision::parse(decision.trim())?;
    let reviewer = reviewer.trim();
    if reviewer.is_empty() {
        return Err("External identity reviewer is required.".to_string());
    }

    repository.review_external_identity_link(&ExternalIdentityReviewRequest {
        link_id: current.link_id.clone(),
        proposed_relationship_type: relationship_type,
        decision,
        match_method: current.match_method.clone(),
        match_confidence: current.match_confidence,
        evidence_json: current.evidence.clone(),
        review_note: normalize_optional_text(evidence_note),
        reviewer: reviewer.to_string(),
    })?;

    let item = find_item(repository, link_id)?;
    let history_count = repository.list_external_identity_reviews(link_id)?.len();
    Ok(ExternalIdentityReviewSubmissionResult {
        message: format!(
            "{} was marked {} for {}.",
            item.source_record_name,
            decision.as_str(),
            item.canonical_entity_name
        ),
        item,
        history_count,
    })
}

fn load_history(
    repository: &MultiSourceRepository,
    link_id: &str,
) -> Result<ExternalIdentityReviewHistoryResult, String> {
    find_item(repository, link_id)?;
    let reviews = repository.list_external_identity_reviews(link_id)?;
    let mut previous_state = "initial_state".to_string();
    let mut history = Vec::with_capacity(reviews.len());
    for review in reviews {
        let decision = review.decision.as_str().to_string();
        history.push(ExternalIdentityReviewHistoryEntry {
            review_id: review.review_id,
            link_id: review.link_id,
            source_record_id: review.source_record_id,
            canonical_entity_id: review.entity_id,
            previous_state: previous_state.clone(),
            decision: decision.clone(),
            proposed_relationship_type: review.proposed_relationship_type.as_str().to_string(),
            match_method: review.match_method,
            match_confidence: review.match_confidence,
            evidence: review.evidence_json,
            review_note: review.review_note,
            reviewer: review.reviewer,
            reviewed_at: review.reviewed_at,
        });
        previous_state = decision;
    }
    Ok(ExternalIdentityReviewHistoryResult {
        link_id: link_id.to_string(),
        total: history.len(),
        message: if history.is_empty() {
            "No review history has been recorded for this link.".to_string()
        } else {
            format!("Loaded {} append-only review decisions.", history.len())
        },
        history,
    })
}

fn find_item(
    repository: &MultiSourceRepository,
    link_id: &str,
) -> Result<ExternalIdentityReviewItem, String> {
    if link_id.trim().is_empty() {
        return Err("Source/entity link ID is required.".to_string());
    }
    list_items(repository)?
        .items
        .into_iter()
        .find(|item| item.link_id == link_id)
        .ok_or_else(|| format!("ExplainX identity link not found: {link_id}"))
}

fn count_status(items: &[ExternalIdentityReviewItem], status: &str) -> usize {
    items
        .iter()
        .filter(|item| item.current_status == status)
        .count()
}

fn describe_match_method(match_method: &str) -> String {
    match match_method {
        "exact_active_alias" => {
            "Matched one active, non-ambiguous ExplainX/global alias.".to_string()
        }
        "child_resource_alias" => {
            "Child resource name matched a canonical product alias.".to_string()
        }
        other => format!("Identity candidate created by {other}."),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use duckdb::Connection;

    use super::*;
    use crate::services::{duckdb_service, explainx_importer};

    #[test]
    fn reviews_imported_explainx_links_with_append_only_history() {
        let database_path = test_database_path("review-actions");
        let fixture_path = test_fixture_path("review-actions");
        cleanup_database_files(&database_path);
        let _ = fs::remove_file(&fixture_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());
        duckdb_service::initialize_database().expect("review database should initialize");
        fs::write(
            &fixture_path,
            r#"[
                {"id":"tools/claude-code","name":"Claude Code","description":"Coding agent","url":"https://example.test/claude-code"},
                {"id":"tools/cline","name":"Cline","description":"Coding agent"},
                {"id":"skills/ponytail","name":"Ponytail","description":"Agent skill"}
            ]"#,
        )
        .expect("ExplainX review fixture should write");
        explainx_importer::import_explainx_records(fixture_path.display().to_string())
            .expect("ExplainX fixture should import");

        let before_candidates = table_count(&database_path, "entity_review_decisions");
        let before_weekly = table_count(&database_path, "weekly_entity_metrics");
        let pending = list_external_identity_review_items().expect("pending links should list");
        assert_eq!(pending.total, 3);
        assert_eq!(pending.pending_count, 3);

        let claude = item_named(&pending, "Claude Code");
        let cline = item_named(&pending, "Cline");
        let ponytail = item_named(&pending, "Ponytail");
        let invalid_decision = submit_external_identity_review(
            claude.link_id.clone(),
            "same_entity".to_string(),
            "auto_approved".to_string(),
            "test-reviewer".to_string(),
            None,
        )
        .expect_err("unsupported decisions should fail before mutation");
        assert!(invalid_decision.contains("Unsupported external identity decision"));
        assert_eq!(table_count(&database_path, "external_identity_reviews"), 0);
        assert_eq!(
            item_named(
                &list_external_identity_review_items()
                    .expect("pending item should remain queryable"),
                "Claude Code"
            )
            .current_status,
            "pending"
        );
        submit_external_identity_review(
            claude.link_id.clone(),
            "same_entity".to_string(),
            "approved".to_string(),
            "test-reviewer".to_string(),
            Some("Canonical product match confirmed.".to_string()),
        )
        .expect("approval should succeed");
        submit_external_identity_review(
            cline.link_id.clone(),
            "related_entity".to_string(),
            "rejected".to_string(),
            "test-reviewer".to_string(),
            Some("Fixture rejection coverage.".to_string()),
        )
        .expect("rejection should succeed");
        submit_external_identity_review(
            ponytail.link_id.clone(),
            "same_entity".to_string(),
            "ambiguous".to_string(),
            "test-reviewer".to_string(),
            Some("Needs more source evidence.".to_string()),
        )
        .expect("ambiguous decision should succeed");
        submit_external_identity_review(
            ponytail.link_id.clone(),
            "same_entity".to_string(),
            "approved".to_string(),
            "second-reviewer".to_string(),
            Some("Additional evidence resolves the link.".to_string()),
        )
        .expect("ambiguous link should later allow approval");

        let reviewed = list_external_identity_review_items().expect("reviewed links should list");
        assert_eq!(reviewed.pending_count, 0);
        assert_eq!(reviewed.approved_count, 2);
        assert_eq!(reviewed.rejected_count, 1);
        assert_eq!(reviewed.ambiguous_count, 0);
        let history = get_external_identity_review_history(ponytail.link_id.clone())
            .expect("history should load");
        assert_eq!(history.total, 2);
        assert_eq!(history.history[0].previous_state, "initial_state");
        assert_eq!(history.history[0].decision, "ambiguous");
        assert_eq!(history.history[1].previous_state, "ambiguous");
        assert_eq!(history.history[1].decision, "approved");
        assert_eq!(
            table_count(&database_path, "entity_review_decisions"),
            before_candidates
        );
        assert_eq!(
            table_count(&database_path, "weekly_entity_metrics"),
            before_weekly
        );

        let _ = fs::remove_file(fixture_path);
        cleanup_database_files(&database_path);
    }

    #[test]
    fn invalid_review_input_does_not_append_history() {
        let database_path = test_database_path("invalid-review");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());
        duckdb_service::initialize_database().expect("review database should initialize");

        let error = submit_external_identity_review(
            "00000000-0000-0000-0000-000000000000".to_string(),
            "same_entity".to_string(),
            "approved".to_string(),
            "test-reviewer".to_string(),
            None,
        )
        .expect_err("unknown links should fail safely");
        assert!(error.contains("identity link not found"));
        assert_eq!(table_count(&database_path, "external_identity_reviews"), 0);

        cleanup_database_files(&database_path);
    }

    fn item_named(
        result: &ExternalIdentityReviewListResult,
        name: &str,
    ) -> ExternalIdentityReviewItem {
        result
            .items
            .iter()
            .find(|item| item.source_record_name == name)
            .cloned()
            .unwrap_or_else(|| panic!("expected review item for {name}"))
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
        std::env::temp_dir().join(format!(
            "ai-agent-trend-radar-external-review-{name}.duckdb"
        ))
    }

    fn test_fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai-agent-trend-radar-external-review-{name}.json"))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
