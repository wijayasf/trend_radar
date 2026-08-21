use crate::models::trend::WeeklyEntityAggregationResult;
use crate::services::duckdb_service;

const REGION_LIMIT: usize = 20;

pub fn aggregate_weekly_entity_metrics() -> Result<WeeklyEntityAggregationResult, String> {
    let canonical_rows_generated = duckdb_service::rebuild_weekly_entity_metrics()?;
    let resolved_entities_included = duckdb_service::count_weekly_entity_metric_entities()?;
    let skipped = duckdb_service::count_identity_resolution_skips()?;
    let top_indonesia =
        duckdb_service::load_weekly_entity_metrics_by_region("indonesia", REGION_LIMIT)?;
    let top_global = duckdb_service::load_weekly_entity_metrics_by_region("global", REGION_LIMIT)?;
    let errors = if skipped.invalid_resolved > 0 {
        vec![format!(
            "{} resolved mentions reference a missing or archived canonical entity and were excluded.",
            skipped.invalid_resolved
        )]
    } else {
        Vec::new()
    };

    Ok(WeeklyEntityAggregationResult {
        canonical_rows_generated,
        resolved_entities_included,
        unresolved_mentions_skipped: skipped.unresolved,
        ambiguous_mentions_skipped: skipped.ambiguous,
        missing_alias_mentions_skipped: skipped.missing_alias,
        skipped_mentions_skipped: skipped.skipped,
        message: format!(
            "Aggregated {canonical_rows_generated} canonical weekly rows across {resolved_entities_included} resolved entities."
        ),
        top_indonesia,
        top_global,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use duckdb::Connection;

    use crate::models::entities::DetectedAgentMention;
    use crate::models::multi_source::{AliasProvenance, AliasSourceScope, NewEntityAlias};
    use crate::models::threads::ThreadPostRaw;
    use crate::services::{
        identity_bootstrap, identity_linker, multi_source_repository::MultiSourceRepository,
        weekly_aggregator,
    };

    use super::*;

    #[test]
    fn rolls_alias_variants_into_one_row_and_excludes_unresolved_mentions_idempotently() {
        with_test_database("alias-rollup", |database_path| {
            add_claude_code_compact_alias();
            let posts = [
                test_post("canonical-rollup-1", "Claude Code", "2026-08-17T09:00:00Z"),
                test_post("canonical-rollup-2", "claude-code", "2026-08-17T10:00:00Z"),
                test_post("canonical-rollup-3", "ClaudeCode", "2026-08-17T11:00:00Z"),
                test_post("canonical-rollup-4", "Codex", "2026-08-17T12:00:00Z"),
                test_post(
                    "canonical-rollup-5",
                    "UnknownNewTool",
                    "2026-08-17T13:00:00Z",
                ),
            ];
            duckdb_service::save_threads_raw_posts(&posts).expect("test posts should save");
            duckdb_service::save_agent_mentions(&[
                test_mention(
                    "canonical-rollup-1",
                    "Claude Code",
                    "coding_agent",
                    "global",
                    "Claude Code is useful.",
                ),
                test_mention(
                    "canonical-rollup-2",
                    "claude-code",
                    "coding_agent",
                    "global",
                    "claude-code is useful.",
                ),
                test_mention(
                    "canonical-rollup-3",
                    "ClaudeCode",
                    "coding_agent",
                    "global",
                    "ClaudeCode is useful.",
                ),
                test_mention(
                    "canonical-rollup-4",
                    "Codex",
                    "coding_agent",
                    "global",
                    "Codex",
                ),
                test_mention(
                    "canonical-rollup-5",
                    "UnknownNewTool",
                    "unknown_candidate",
                    "global",
                    "UnknownNewTool launched.",
                ),
            ])
            .expect("test mentions should save");

            let legacy_before = weekly_aggregator::aggregate_weekly_metrics()
                .expect("legacy weekly aggregation should run before canonical aggregation");
            let linkage = identity_linker::link_agent_mentions_to_entities()
                .expect("identity linkage should resolve known variants");
            assert_eq!(linkage.resolved_count, 3);
            assert_eq!(linkage.ambiguous_count, 1);
            assert_eq!(linkage.missing_alias_count, 1);

            let first = aggregate_weekly_entity_metrics()
                .expect("first canonical aggregation should succeed");
            let second = aggregate_weekly_entity_metrics()
                .expect("second canonical aggregation should remain idempotent");
            assert_eq!(first.canonical_rows_generated, 1);
            assert_eq!(second.canonical_rows_generated, 1);
            assert_eq!(first.resolved_entities_included, 1);
            assert_eq!(first.ambiguous_mentions_skipped, 1);
            assert_eq!(first.missing_alias_mentions_skipped, 1);
            assert_eq!(first.top_global.len(), 1);
            assert_eq!(first.top_global[0].canonical_name, "Claude Code");
            assert_eq!(first.top_global[0].mention_count, 3);
            assert_eq!(first.top_global[0].source_count, 1);
            assert!(first.errors.is_empty());

            let legacy_after = weekly_aggregator::aggregate_weekly_metrics()
                .expect("legacy weekly aggregation should remain available");
            assert_eq!(legacy_after.metrics_count, legacy_before.metrics_count);

            let connection = Connection::open(database_path)
                .expect("canonical aggregation database should open");
            let canonical_row_count: i64 = connection
                .query_row("SELECT COUNT(*) FROM weekly_entity_metrics", [], |row| {
                    row.get(0)
                })
                .expect("canonical metric count should be readable");
            assert_eq!(canonical_row_count, 1);
        });
    }

    #[test]
    fn preserves_region_split_for_one_canonical_entity() {
        with_test_database("region-split", |_| {
            duckdb_service::save_threads_raw_posts(&[
                test_post(
                    "canonical-region-id",
                    "Claude Code membantu developer Indonesia.",
                    "2026-08-17T09:00:00Z",
                ),
                test_post(
                    "canonical-region-global",
                    "Claude Code helps global developers.",
                    "2026-08-17T10:00:00Z",
                ),
            ])
            .expect("region posts should save");
            duckdb_service::save_agent_mentions(&[
                test_mention(
                    "canonical-region-id",
                    "Claude Code",
                    "coding_agent",
                    "indonesia",
                    "Claude Code membantu developer Indonesia.",
                ),
                test_mention(
                    "canonical-region-global",
                    "Claude Code",
                    "coding_agent",
                    "global",
                    "Claude Code helps global developers.",
                ),
            ])
            .expect("region mentions should save");

            identity_linker::link_agent_mentions_to_entities()
                .expect("region mentions should link");
            let result = aggregate_weekly_entity_metrics()
                .expect("regional canonical aggregation should succeed");
            assert_eq!(result.canonical_rows_generated, 2);
            assert_eq!(result.resolved_entities_included, 1);
            assert_eq!(result.top_indonesia.len(), 1);
            assert_eq!(result.top_global.len(), 1);
            assert_eq!(result.top_indonesia[0].canonical_name, "Claude Code");
            assert_eq!(result.top_global[0].canonical_name, "Claude Code");
            assert_eq!(result.top_indonesia[0].mention_count, 1);
            assert_eq!(result.top_global[0].mention_count, 1);
        });
    }

    #[test]
    fn adds_canonical_metrics_table_without_changing_legacy_data() {
        with_test_database("legacy-compatibility", |database_path| {
            duckdb_service::save_threads_raw_posts(&[test_post(
                "canonical-legacy-1",
                "Claude Code legacy mention.",
                "2026-08-17T09:00:00Z",
            )])
            .expect("legacy post should save");
            duckdb_service::save_agent_mentions(&[test_mention(
                "canonical-legacy-1",
                "Claude Code",
                "coding_agent",
                "global",
                "Claude Code legacy mention.",
            )])
            .expect("legacy mention should save");

            let connection = Connection::open(database_path).expect("legacy database should open");
            connection
                .execute_batch("DROP TABLE weekly_entity_metrics;")
                .expect("test should recreate a pre-IMP-04 database shape");
            drop(connection);

            duckdb_service::initialize_database()
                .expect("additive schema should restore canonical metrics table");
            let result = aggregate_weekly_entity_metrics()
                .expect("canonical aggregation should accept unresolved legacy data");
            assert_eq!(result.canonical_rows_generated, 0);
            assert_eq!(result.unresolved_mentions_skipped, 1);
            assert!(result.top_indonesia.is_empty());
            assert!(result.top_global.is_empty());

            let connection = Connection::open(database_path)
                .expect("upgraded legacy database should remain readable");
            let mention_count: i64 = connection
                .query_row("SELECT COUNT(*) FROM agent_mentions", [], |row| row.get(0))
                .expect("legacy mention should survive");
            assert_eq!(mention_count, 1);
        });
    }

    fn add_claude_code_compact_alias() {
        let repository = MultiSourceRepository::open().expect("test repository should open");
        identity_bootstrap::bootstrap_curated_aliases(&repository)
            .expect("curated aliases should bootstrap");
        let entities = repository
            .lookup_canonical_entities_by_normalized_name("Claude Code")
            .expect("Claude Code entity should load");
        assert_eq!(entities.len(), 1);
        repository
            .create_entity_alias(&NewEntityAlias {
                entity_id: entities[0].entity_id.clone(),
                alias: "ClaudeCode".to_string(),
                source_scope: AliasSourceScope::Global,
                provenance: AliasProvenance::Manual,
                is_ambiguous: false,
                context_terms_json: None,
            })
            .expect("compact alias should insert");
    }

    fn test_post(post_id: &str, text: &str, posted_at: &str) -> ThreadPostRaw {
        ThreadPostRaw {
            post_id: post_id.to_string(),
            text: text.to_string(),
            text_missing: false,
            author_id: None,
            author_username: Some("canonical_test".to_string()),
            author_display_name: None,
            media_type: Some("TEXT".to_string()),
            permalink: Some(format!("mock://threads/{post_id}")),
            posted_at: Some(posted_at.to_string()),
            source_type: Some("test_threads".to_string()),
            source_seed_keyword: None,
            keyword_match: None,
            like_count: 0,
            reply_count: 0,
            repost_count: 0,
            quote_count: 0,
            share_count: 0,
            view_count: 0,
            raw_json: "{}".to_string(),
        }
    }

    fn test_mention(
        post_id: &str,
        agent_name: &str,
        category: &str,
        region: &str,
        source_snippet: &str,
    ) -> DetectedAgentMention {
        DetectedAgentMention {
            mention_id: format!("{post_id}::{}", agent_name.to_lowercase()),
            post_id: post_id.to_string(),
            agent_name: agent_name.to_string(),
            agent_alias: agent_name.to_string(),
            category: category.to_string(),
            detection_source: if category == "unknown_candidate" {
                "candidate_pattern".to_string()
            } else {
                "known_alias".to_string()
            },
            needs_review: category == "unknown_candidate",
            review_status: if category == "unknown_candidate" {
                "pending".to_string()
            } else {
                "approved".to_string()
            },
            reviewed_as: None,
            reviewed_category: None,
            region: region.to_string(),
            confidence: 0.9,
            match_confidence: 0.9,
            relevance_score: 0.9,
            sentiment: "positive".to_string(),
            cost_signal: "not_mentioned".to_string(),
            source_snippet: source_snippet.to_string(),
        }
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
        std::env::temp_dir().join(format!(
            "ai-agent-trend-radar-canonical-weekly-{name}.duckdb"
        ))
    }

    fn cleanup_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(path.with_extension("duckdb.tmp"));
    }
}
