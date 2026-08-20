mod commands;
mod models;
mod services;
mod utils;

fn main() {
    utils::config::load_env_files_once();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::health::app_health,
            commands::config::env_config_status,
            commands::database::check_database_health,
            commands::database::count_threads_raw_posts,
            commands::database::reset_local_pipeline_data,
            commands::apify::run_apify_discovery_crawl,
            commands::apify::replay_last_apify_crawl,
            commands::apify::import_apify_dataset_cache,
            commands::candidates::approve_candidate_entity,
            commands::candidates::ignore_candidate_entity,
            commands::candidates::list_candidate_entities,
            commands::candidates::list_entity_review_decisions,
            commands::candidates::reset_candidate_review,
            commands::discovery::run_discovery_crawl,
            commands::discovery::test_discovery_seed,
            commands::threads::collect_threads_by_keyword,
            commands::threads::import_sample_threads_posts,
            commands::entities::detect_agent_mentions,
            commands::entities::link_agent_mentions_to_entities,
            commands::regions::classify_regions,
            commands::sentiments::classify_sentiments,
            commands::costs::classify_cost_signals,
            commands::weekly::aggregate_weekly_metrics,
            commands::reports::export_weekly_report_markdown,
            commands::reports::export_weekly_metrics_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use duckdb::Connection;
    use serde_json::Value;

    use crate::models::entities::DetectedAgentMention;
    use crate::models::threads::ThreadPostRaw;
    use crate::services::{
        candidate_review, cost_classifier, discovery_crawler, duckdb_service, entity_detector,
        identity_linker, region_classifier, report_exporter, sentiment_classifier,
        weekly_aggregator,
    };

    #[test]
    fn validates_raw_post_insert_after_schema_init() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-raw-insert-schema-regression.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("schema initialization should succeed");
        let saved_count = duckdb_service::save_threads_raw_posts(&[ThreadPostRaw {
            post_id: "schema-regression-raw-001".to_string(),
            text: "NovaForge appears in AI agent workflow notes.".to_string(),
            text_missing: false,
            author_id: None,
            author_username: Some("schema_tester".to_string()),
            author_display_name: None,
            media_type: Some("TEXT".to_string()),
            permalink: Some("mock://threads/schema-regression-raw-001".to_string()),
            posted_at: Some("2026-07-06T09:00:00Z".to_string()),
            source_type: Some("schema_regression_test".to_string()),
            source_seed_keyword: None,
            keyword_match: None,
            like_count: 0,
            reply_count: 0,
            repost_count: 0,
            quote_count: 0,
            share_count: 0,
            view_count: 0,
            raw_json: "{}".to_string(),
        }])
        .expect("raw post insert should not depend on mention compatibility migration");

        assert_eq!(saved_count, 1);
        assert_eq!(
            duckdb_service::count_threads_raw_posts().expect("raw post count should be readable"),
            1
        );

        let entity_result =
            entity_detector::detect_agent_mentions().expect("entity detection should still work");
        assert!(entity_result
            .preview
            .iter()
            .any(|mention| mention.agent_name == "NovaForge"));

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_mention_identity_linkage_preserves_mvp_fields_and_weekly_metrics() {
        let database_path = std::env::temp_dir()
            .join("ai-agent-trend-radar-mention-identity-linkage-integration.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("schema initialization should succeed");
        duckdb_service::save_threads_raw_posts(&[test_raw_post(
            "identity-link-1",
            "claude-code helps with coding workflows.",
            "2026-07-13T09:00:00Z",
        )])
        .expect("raw post should save");
        let mention = test_mention("identity-link-1", "claude-code", "coding_agent");
        duckdb_service::save_agent_mentions(std::slice::from_ref(&mention))
            .expect("legacy mention should save");

        let before_fields = load_non_identity_mention_fields(&database_path, &mention.mention_id);
        let before_weekly = weekly_aggregator::aggregate_weekly_metrics()
            .expect("baseline weekly metrics should aggregate");

        let linkage = identity_linker::link_agent_mentions_to_entities()
            .expect("mention identity linkage should succeed");
        assert_eq!(linkage.resolved_count, 1);
        assert_eq!(linkage.missing_alias_count, 0);
        assert_eq!(linkage.ambiguous_count, 0);

        let connection = Connection::open(&database_path).expect("identity database should open");
        let identity: (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f64>,
            bool,
        ) = connection
            .query_row(
                r#"
                    SELECT
                        CAST(entity_id AS VARCHAR),
                        identity_resolution_status,
                        identity_resolution_reason,
                        identity_resolution_confidence,
                        identity_resolved_at IS NOT NULL
                    FROM agent_mentions
                    WHERE mention_id = ?1
                    "#,
                [&mention.mention_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("identity fields should be readable");
        drop(connection);
        assert!(identity.0.is_some());
        assert_eq!(identity.1.as_deref(), Some("resolved"));
        assert!(identity
            .2
            .as_deref()
            .is_some_and(|reason| reason.contains("claude-code")));
        assert_eq!(identity.3, Some(1.0));
        assert!(identity.4);
        assert_eq!(
            load_non_identity_mention_fields(&database_path, &mention.mention_id),
            before_fields
        );

        let after_weekly = weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly metrics should remain compatible after linkage");
        assert_eq!(after_weekly.metrics_count, before_weekly.metrics_count);
        assert_eq!(after_weekly.global_count, before_weekly.global_count);
        assert_eq!(after_weekly.top_global[0].agent_name, "claude-code");
        assert_eq!(
            after_weekly.top_global[0].trend_score,
            before_weekly.top_global[0].trend_score
        );

        duckdb_service::save_agent_mentions(&[mention.clone()])
            .expect("mention upsert should preserve linked identity");
        let connection = Connection::open(&database_path).expect("identity database should reopen");
        let preserved_status: Option<String> = connection
            .query_row(
                "SELECT identity_resolution_status FROM agent_mentions WHERE mention_id = ?1",
                [&mention.mention_id],
                |row| row.get(0),
            )
            .expect("preserved identity status should be readable");
        assert_eq!(preserved_status.as_deref(), Some("resolved"));
        drop(connection);

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_existing_mentions_survive_identity_column_migration() {
        let database_path = std::env::temp_dir()
            .join("ai-agent-trend-radar-mention-identity-migration-compatibility.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("baseline schema should initialize");
        duckdb_service::save_threads_raw_posts(&[test_raw_post(
            "identity-migration-1",
            "Claude Code remains available after an additive migration.",
            "2026-07-13T09:00:00Z",
        )])
        .expect("legacy raw post should save");
        let mention = test_mention("identity-migration-1", "Claude Code", "coding_agent");
        duckdb_service::save_agent_mentions(std::slice::from_ref(&mention))
            .expect("legacy mention should save");
        let before_fields = load_non_identity_mention_fields(&database_path, &mention.mention_id);

        let connection = Connection::open(&database_path).expect("legacy database should open");
        connection
            .execute_batch(
                r#"
                DROP INDEX IF EXISTS idx_agent_mentions_agent_region;
                ALTER TABLE agent_mentions DROP COLUMN entity_id;
                ALTER TABLE agent_mentions DROP COLUMN identity_resolution_status;
                ALTER TABLE agent_mentions DROP COLUMN identity_resolution_reason;
                ALTER TABLE agent_mentions DROP COLUMN identity_resolution_confidence;
                ALTER TABLE agent_mentions DROP COLUMN identity_resolved_at;
                "#,
            )
            .expect("test should recreate the pre-IMP-03 mention shape");
        drop(connection);

        duckdb_service::initialize_database()
            .expect("additive identity migration should initialize legacy database");
        assert_eq!(
            load_non_identity_mention_fields(&database_path, &mention.mention_id),
            before_fields
        );
        let connection = Connection::open(&database_path).expect("migrated database should open");
        let identity_columns: i64 = connection
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM information_schema.columns
                WHERE table_name = 'agent_mentions'
                    AND column_name IN (
                        'entity_id',
                        'identity_resolution_status',
                        'identity_resolution_reason',
                        'identity_resolution_confidence',
                        'identity_resolved_at'
                    )
                "#,
                [],
                |row| row.get(0),
            )
            .expect("identity migration columns should be readable");
        assert_eq!(identity_columns, 5);
        drop(connection);

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_weekly_metrics_group_canonical_entities_and_exclude_generic_mcp() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-weekly-canonical-test.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("schema initialization should succeed");
        duckdb_service::save_threads_raw_posts(&[
            test_raw_post(
                "weekly-canonical-1",
                "Claude Code and MCP server are useful.",
                "2026-07-06T09:00:00Z",
            ),
            test_raw_post(
                "weekly-canonical-2",
                "claude code and mcp server again.",
                "2026-07-06T10:00:00Z",
            ),
        ])
        .expect("raw posts should save");

        duckdb_service::save_agent_mentions(&[
            test_mention("weekly-canonical-1", "Claude Code", "coding_agent"),
            test_mention("weekly-canonical-2", "claude code", "coding_agent"),
            test_mention("weekly-canonical-1", "MCP", "mcp_or_connector"),
            test_mention("weekly-canonical-2", "mcp", "mcp_or_connector"),
        ])
        .expect("mentions should save");

        let weekly_result = weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly aggregation should group canonical entities");
        let global_metrics = weekly_result.top_global;
        let claude_rows = global_metrics
            .iter()
            .filter(|metric| metric.agent_name.eq_ignore_ascii_case("Claude Code"))
            .collect::<Vec<_>>();
        let mcp_rows = global_metrics
            .iter()
            .filter(|metric| metric.agent_name.eq_ignore_ascii_case("MCP"))
            .collect::<Vec<_>>();

        assert_eq!(claude_rows.len(), 1);
        assert_eq!(claude_rows[0].mentions, 2);
        assert!(mcp_rows.is_empty());

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_weekly_rankings_only_load_the_latest_week() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-latest-week-ranking-test.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("schema initialization should succeed");
        let posts = [
            ("weekly-old-1", "2026-07-06T09:00:00Z"),
            ("weekly-latest-1", "2026-07-13T09:00:00Z"),
            ("weekly-latest-2", "2026-07-13T10:00:00Z"),
            ("weekly-latest-3", "2026-07-14T09:00:00Z"),
            ("weekly-latest-4", "2026-07-15T09:00:00Z"),
        ]
        .map(|(post_id, posted_at)| test_raw_post(post_id, "Claude Code is useful.", posted_at));
        duckdb_service::save_threads_raw_posts(&posts).expect("raw posts should save");

        let mentions = posts
            .iter()
            .map(|post| test_mention(&post.post_id, "Claude Code", "coding_agent"))
            .collect::<Vec<_>>();
        duckdb_service::save_agent_mentions(&mentions).expect("mentions should save");

        weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly aggregation should build both weeks");
        let global_metrics = duckdb_service::load_weekly_agent_metrics_by_region("global", 20)
            .expect("latest global ranking should load");
        let export_metrics = duckdb_service::load_weekly_agent_metrics(100)
            .expect("latest export metrics should load");

        assert_eq!(global_metrics.len(), 1);
        assert_eq!(global_metrics[0].agent_name, "Claude Code");
        assert_eq!(global_metrics[0].mentions, 4);
        assert_eq!(global_metrics[0].week_start, "2026-07-13");
        assert_eq!(export_metrics.len(), 1);
        assert_eq!(export_metrics[0].week_start, "2026-07-13");

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_reset_local_pipeline_data_preserves_candidate_decisions() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-reset-demo-data-test.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        duckdb_service::initialize_database().expect("schema initialization should succeed");
        duckdb_service::save_threads_raw_posts(&[test_raw_post(
            "reset-demo-1",
            "ResetDemoAI appears in agent workflow notes.",
            "2026-07-06T09:00:00Z",
        )])
        .expect("raw post should save");
        duckdb_service::save_agent_mentions(&[test_mention(
            "reset-demo-1",
            "ResetDemoAI",
            "unknown_candidate",
        )])
        .expect("mention should save");
        candidate_review::approve_candidate_entity(
            "ResetDemoAI".to_string(),
            "ResetDemoAI".to_string(),
            "coding_agent".to_string(),
            Some("preserve decision during demo reset".to_string()),
        )
        .expect("candidate decision should save");
        let _ = weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly metrics should build before reset");

        let reset_message =
            duckdb_service::reset_local_pipeline_data().expect("demo data reset should succeed");
        assert!(reset_message.contains("Candidate decisions were preserved"));
        assert_eq!(
            duckdb_service::count_threads_raw_posts().expect("raw post count should be readable"),
            0
        );
        assert!(duckdb_service::load_raw_posts_for_detection()
            .expect("raw posts should be queryable")
            .is_empty());
        assert!(
            duckdb_service::load_weekly_agent_metrics_by_region("global", 20)
                .expect("weekly metrics should be queryable")
                .is_empty()
        );
        let decisions = candidate_review::list_entity_review_decisions()
            .expect("candidate decisions should remain after reset");
        assert!(decisions
            .decisions
            .iter()
            .any(|decision| decision.candidate_name == "ResetDemoAI"
                && decision.status == "approved"));

        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_legacy_mention_compatibility_objects_are_removed() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-legacy-compat-cleanup-test.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());

        let connection = Connection::open(&database_path)
            .expect("legacy compatibility test database should open");
        connection
            .execute_batch("CREATE TABLE agent_mentions_compatible (id TEXT);")
            .expect("legacy compatibility table should be created");
        drop(connection);

        duckdb_service::initialize_database()
            .expect("schema initialization should remove the legacy table");
        assert!(!database_object_exists(
            &database_path,
            "agent_mentions_compatible"
        ));

        let connection = Connection::open(&database_path)
            .expect("legacy compatibility test database should reopen");
        connection
            .execute_batch(
                "CREATE VIEW agent_mentions_compatible AS SELECT post_id FROM threads_posts_raw;",
            )
            .expect("legacy compatibility view should be created");
        drop(connection);

        duckdb_service::initialize_database()
            .expect("schema initialization should remove the legacy view");
        assert!(!database_object_exists(
            &database_path,
            "agent_mentions_compatible"
        ));

        duckdb_service::reset_local_pipeline_data()
            .expect("pipeline reset should not depend on the legacy object");
        cleanup_database_files(&database_path);
    }

    #[test]
    fn validates_sample_full_mvp_flow() {
        let database_path = temp_database_path();
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());
        std::env::set_var("THREADS_ACCESS_TOKEN", "");
        std::env::set_var("THREADS_MOCK_ID_ONLY_DETAIL", "1");

        let discovery_result =
            discovery_crawler::run_discovery_crawl(Some("all".to_string()), Some(10), Some(false))
                .expect(
                    "discovery crawl should resolve mock ID-only keyword search via detail fetch",
                );
        assert_eq!(discovery_result.mode, "mock_id_only_detail");
        assert!(discovery_result.id_only_results_count > 0);
        assert!(discovery_result.detail_fetched_total > 0);
        assert_eq!(discovery_result.detail_failed_total, 0);
        assert_eq!(discovery_result.saved_total, 5);
        assert!(discovery_result.duplicates_skipped > 0);
        assert!(!discovery_result.run_id.is_empty());
        assert!(!discovery_result.started_at.is_empty());
        assert!(!discovery_result.finished_at.is_empty());
        assert_eq!(discovery_result.max_per_seed, 10);
        assert!(!discovery_result.seed_results.is_empty());
        assert!(discovery_result
            .seed_results
            .iter()
            .any(|seed| seed.search_status == "success"));
        assert_eq!(
            duckdb_service::count_threads_raw_posts().expect("raw post count should be readable"),
            5
        );

        let seed_test = discovery_crawler::test_discovery_seed("Ponytail".to_string())
            .expect("single seed test should resolve mock detail");
        assert_eq!(seed_test.status, "success");
        assert!(seed_test.fetched_count > 0);
        assert!(seed_test.text_available_count > 0);
        assert!(seed_test.sample_text_snippet.contains("Ponytail"));

        let entity_result =
            entity_detector::detect_agent_mentions().expect("entity detection should succeed");
        assert!(entity_result.mentions_found > 0);
        assert!(entity_result.saved_count > 0);
        assert!(entity_result
            .preview
            .iter()
            .any(|mention| mention.agent_name == "Ponytail"));
        assert!(entity_result
            .preview
            .iter()
            .any(|mention| mention.agent_name == "Caveman"));
        assert!(entity_result
            .preview
            .iter()
            .any(|mention| mention.agent_name == "Astryx"));

        let region_result =
            region_classifier::classify_regions().expect("region classification should succeed");
        assert!(region_result.indonesia_count > 0);
        assert!(region_result.global_count > 0);
        assert!(region_result.updated_mentions_count > 0);

        let sentiment_result = sentiment_classifier::classify_sentiments()
            .expect("sentiment classification should succeed");
        assert!(
            sentiment_result.positive_count
                + sentiment_result.neutral_count
                + sentiment_result.negative_count
                + sentiment_result.mixed_count
                > 0
        );
        assert!(sentiment_result.updated_mentions_count > 0);

        let cost_result =
            cost_classifier::classify_cost_signals().expect("cost classification should succeed");
        assert!(
            cost_result.not_mentioned_count
                + cost_result.cost_positive_count
                + cost_result.cost_negative_boros_count
                + cost_result.cost_mixed_count
                > 0
        );
        assert!(cost_result.updated_mentions_count > 0);

        let candidates =
            candidate_review::list_candidate_entities().expect("candidate list should load");
        assert!(candidates
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_name == "NovaForge"
                && candidate.current_status == "pending"));
        assert!(candidates
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_name == "FlowPilot"
                && candidate.current_status == "pending"));

        let approved_candidate = candidate_review::approve_candidate_entity(
            "NovaForge".to_string(),
            "NovaForge".to_string(),
            "coding_agent".to_string(),
            Some("sample candidate approval".to_string()),
        )
        .expect("candidate approval should succeed");
        assert_eq!(approved_candidate.updated_mentions_count, 1);

        let ignored_candidate = candidate_review::ignore_candidate_entity(
            "FlowPilot".to_string(),
            Some("sample false positive ignore".to_string()),
        )
        .expect("candidate ignore should succeed");
        assert_eq!(ignored_candidate.updated_mentions_count, 1);

        let decisions = candidate_review::list_entity_review_decisions()
            .expect("candidate decision registry should load");
        assert!(decisions.decisions.iter().any(|decision| {
            decision.candidate_name == "NovaForge"
                && decision.normalized_name == "NovaForge"
                && decision.category == "coding_agent"
                && decision.status == "approved"
        }));
        assert!(
            decisions
                .decisions
                .iter()
                .any(|decision| decision.candidate_name == "FlowPilot"
                    && decision.status == "ignored")
        );

        duckdb_service::save_threads_raw_posts(&[
            ThreadPostRaw {
                post_id: "mock-detail-novaforge-followup".to_string(),
                text: "NovaForge keeps appearing in AI agent coding workflow notes.".to_string(),
                text_missing: false,
                author_id: None,
                author_username: Some("mock_candidate_reviewer".to_string()),
                author_display_name: None,
                media_type: Some("TEXT".to_string()),
                permalink: Some("mock://threads/mock-detail-novaforge-followup".to_string()),
                posted_at: Some("2026-07-05T12:00:00Z".to_string()),
                source_type: Some("candidate_registry_test".to_string()),
                source_seed_keyword: None,
                keyword_match: None,
                like_count: 0,
                reply_count: 0,
                repost_count: 0,
                quote_count: 0,
                share_count: 0,
                view_count: 0,
                raw_json: "{}".to_string(),
            },
            ThreadPostRaw {
                post_id: "mock-detail-flowpilot-followup".to_string(),
                text: "FlowPilot appears again in AI agent workflow chatter.".to_string(),
                text_missing: false,
                author_id: None,
                author_username: Some("mock_candidate_reviewer".to_string()),
                author_display_name: None,
                media_type: Some("TEXT".to_string()),
                permalink: Some("mock://threads/mock-detail-flowpilot-followup".to_string()),
                posted_at: Some("2026-07-05T13:00:00Z".to_string()),
                source_type: Some("candidate_registry_test".to_string()),
                source_seed_keyword: None,
                keyword_match: None,
                like_count: 0,
                reply_count: 0,
                repost_count: 0,
                quote_count: 0,
                share_count: 0,
                view_count: 0,
                raw_json: "{}".to_string(),
            },
        ])
        .expect("follow-up candidate posts should save");

        let redetection_result =
            entity_detector::detect_agent_mentions().expect("redetection should apply decisions");
        assert!(redetection_result.preview.iter().any(|mention| {
            mention.agent_name == "NovaForge"
                && mention.category == "coding_agent"
                && mention.detection_source == "reviewed_candidate"
                && !mention.needs_review
        }));
        assert!(redetection_result.preview.iter().any(|mention| {
            mention.agent_name == "FlowPilot"
                && mention.category == "unknown_candidate"
                && !mention.needs_review
        }));

        let reviewed_candidates = candidate_review::list_candidate_entities()
            .expect("candidate list should reflect durable decisions");
        assert!(reviewed_candidates.candidates.iter().any(|candidate| {
            candidate.candidate_name == "NovaForge" && candidate.current_status == "approved"
        }));
        assert!(reviewed_candidates.candidates.iter().any(|candidate| {
            candidate.candidate_name == "FlowPilot" && candidate.current_status == "ignored"
        }));

        let _ = region_classifier::classify_regions()
            .expect("region reclassification should succeed after candidate redetection");
        let _ = sentiment_classifier::classify_sentiments()
            .expect("sentiment reclassification should succeed after candidate redetection");
        let _ = cost_classifier::classify_cost_signals()
            .expect("cost reclassification should succeed after candidate redetection");

        let weekly_result = weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly aggregation should succeed");
        assert!(weekly_result.metrics_count > 0);
        assert!(weekly_result.indonesia_count > 0);
        assert!(weekly_result.global_count > 0);
        assert!(weekly_result
            .top_indonesia
            .iter()
            .chain(weekly_result.top_global.iter())
            .chain(weekly_result.top_unknown.iter())
            .any(|metric| metric.agent_name == "Claude Code"));
        assert!(weekly_result
            .top_indonesia
            .iter()
            .chain(weekly_result.top_global.iter())
            .chain(weekly_result.top_unknown.iter())
            .any(|metric| metric.agent_name == "Ponytail"));
        assert!(weekly_result
            .top_indonesia
            .iter()
            .chain(weekly_result.top_global.iter())
            .chain(weekly_result.top_unknown.iter())
            .any(|metric| metric.agent_name == "Astryx"));
        assert!(weekly_result
            .top_indonesia
            .iter()
            .chain(weekly_result.top_global.iter())
            .chain(weekly_result.top_unknown.iter())
            .any(|metric| metric.agent_name == "NovaForge"));
        assert!(!weekly_result
            .top_indonesia
            .iter()
            .chain(weekly_result.top_global.iter())
            .chain(weekly_result.top_unknown.iter())
            .any(|metric| metric.agent_name == "FlowPilot"));

        let markdown_export = report_exporter::export_weekly_report_markdown()
            .expect("Markdown weekly report export should succeed");
        let markdown_content = fs::read_to_string(&markdown_export.file_path)
            .expect("Markdown weekly report should be readable");
        assert!(markdown_content.contains("# AI Agent Trend Radar Weekly Report"));
        assert!(markdown_content.contains("## Top AI Agents - Indonesia"));

        let csv_export = report_exporter::export_weekly_metrics_csv()
            .expect("CSV metrics export should succeed");
        let csv_content =
            fs::read_to_string(&csv_export.file_path).expect("CSV metrics should be readable");
        assert!(csv_content.contains("agent_name"));
        assert!(csv_content.contains("trend_score"));

        if should_cleanup_report_exports() {
            let _ = fs::remove_file(&markdown_export.file_path);
            let _ = fs::remove_file(&csv_export.file_path);
        }
        cleanup_database_files(&database_path);
        std::env::remove_var("THREADS_MOCK_ID_ONLY_DETAIL");
    }

    #[test]
    #[ignore = "Requires a real Threads access token and network access."]
    fn validates_real_threads_discovery_smoke() {
        let database_path =
            std::env::temp_dir().join("ai-agent-trend-radar-real-discovery-smoke.duckdb");
        cleanup_database_files(&database_path);
        let _database_path_guard =
            crate::utils::config::set_test_database_path(database_path.clone());
        std::env::remove_var("THREADS_MOCK_ID_ONLY_DETAIL");

        let discovery_result = discovery_crawler::run_discovery_crawl(
            Some("global".to_string()),
            Some(3),
            Some(false),
        )
        .expect("real discovery crawl should complete or return a safe API error");
        println!(
            "real_discovery_summary seeds_processed={} fetched_total={} id_only_results_count={} detail_fetched_total={} detail_failed_total={} text_missing_total={} saved_total={} duplicates_skipped={} failed_seeds={} mode={}",
            discovery_result.seeds_processed,
            discovery_result.fetched_total,
            discovery_result.id_only_results_count,
            discovery_result.detail_fetched_total,
            discovery_result.detail_failed_total,
            discovery_result.text_missing_total,
            discovery_result.saved_total,
            discovery_result.duplicates_skipped,
            discovery_result.failed_seeds,
            discovery_result.mode
        );
        if !discovery_result.errors.is_empty() {
            println!(
                "real_discovery_safe_errors {}",
                discovery_result.errors.join(" | ")
            );
        }

        if discovery_result.fetched_total == 0 || discovery_result.saved_total == 0 {
            println!("real_discovery_zero_results true");
            cleanup_database_files(&database_path);
            return;
        }

        let entity_result =
            entity_detector::detect_agent_mentions().expect("entity detection should succeed");
        let entity_names = entity_result
            .preview
            .iter()
            .map(|mention| mention.agent_name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        println!(
            "real_entity_summary analyzed_posts={} mentions_found={} saved_count={} preview_entities={}",
            entity_result.analyzed_posts,
            entity_result.mentions_found,
            entity_result.saved_count,
            entity_names
        );
        if entity_result.saved_count == 0 {
            let diagnostics = raw_post_storage_diagnostics(&database_path)
                .expect("raw post diagnostics should be readable");
            println!("real_raw_post_diagnostics {diagnostics}");
        }

        if entity_result.saved_count > 0 {
            let _ = region_classifier::classify_regions()
                .expect("region classification should succeed");
            let _ = sentiment_classifier::classify_sentiments()
                .expect("sentiment classification should succeed");
            let _ = cost_classifier::classify_cost_signals()
                .expect("cost classification should succeed");
            let weekly_result = weekly_aggregator::aggregate_weekly_metrics()
                .expect("weekly aggregation should succeed");
            println!(
                "real_weekly_summary metrics_count={} indonesia_count={} global_count={} unknown_count={}",
                weekly_result.metrics_count,
                weekly_result.indonesia_count,
                weekly_result.global_count,
                weekly_result.unknown_count
            );
        }

        cleanup_database_files(&database_path);
    }

    fn temp_database_path() -> PathBuf {
        std::env::temp_dir().join("ai-agent-trend-radar-full-flow-test.duckdb")
    }

    fn test_raw_post(post_id: &str, text: &str, posted_at: &str) -> ThreadPostRaw {
        ThreadPostRaw {
            post_id: post_id.to_string(),
            text: text.to_string(),
            text_missing: text.trim().is_empty(),
            author_id: None,
            author_username: Some("test_author".to_string()),
            author_display_name: None,
            media_type: Some("TEXT".to_string()),
            permalink: Some(format!("mock://threads/{post_id}")),
            posted_at: Some(posted_at.to_string()),
            source_type: Some("test".to_string()),
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

    fn test_mention(post_id: &str, agent_name: &str, category: &str) -> DetectedAgentMention {
        DetectedAgentMention {
            mention_id: format!("{post_id}::{}", agent_name.to_lowercase().replace(' ', "_")),
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
            region: "global".to_string(),
            confidence: 0.9,
            match_confidence: 0.9,
            relevance_score: 0.9,
            sentiment: "positive".to_string(),
            cost_signal: "not_mentioned".to_string(),
            source_snippet: format!("{agent_name} appears in AI agent workflow notes."),
        }
    }

    fn load_non_identity_mention_fields(
        database_path: &PathBuf,
        mention_id: &str,
    ) -> (String, String, String, String, String, f64, String) {
        let connection =
            Connection::open(database_path).expect("mention database should open for inspection");
        connection
            .query_row(
                r#"
                SELECT
                    post_id,
                    agent_name,
                    agent_alias,
                    category,
                    region,
                    match_confidence,
                    source_snippet
                FROM agent_mentions
                WHERE mention_id = ?1
                "#,
                [mention_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("non-identity mention fields should be readable")
    }

    fn cleanup_database_files(database_path: &PathBuf) {
        let _ = fs::remove_file(database_path);
        let _ = fs::remove_file(database_path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(database_path.with_extension("duckdb.tmp"));
    }

    fn database_object_exists(database_path: &PathBuf, object_name: &str) -> bool {
        let connection = Connection::open(database_path)
            .expect("database should open for compatibility object inspection");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM information_schema.tables WHERE lower(table_name) = lower(?1)",
                [object_name],
                |row| row.get(0),
            )
            .expect("compatibility object count should be readable");
        count > 0
    }

    fn should_cleanup_report_exports() -> bool {
        std::env::var("KEEP_REPORT_EXPORTS")
            .ok()
            .map(|value| value.trim() != "1")
            .unwrap_or(true)
    }

    fn raw_post_storage_diagnostics(database_path: &PathBuf) -> Result<String, String> {
        let connection = Connection::open(database_path)
            .map_err(|error| format!("diagnostic DuckDB open failed: {error}"))?;
        let raw_post_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM threads_posts_raw", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("diagnostic raw post count failed: {error}"))?;
        let text_missing_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM threads_posts_raw WHERE COALESCE(text_missing, FALSE)",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("diagnostic text_missing count failed: {error}"))?;
        let sample_raw_json: Option<String> = connection
            .query_row(
                "SELECT raw_json FROM threads_posts_raw WHERE raw_json IS NOT NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        let raw_json_keys = sample_raw_json
            .and_then(|json| serde_json::from_str::<Value>(&json).ok())
            .and_then(|value| {
                value.as_object().map(|object| {
                    let mut keys = object.keys().cloned().collect::<Vec<String>>();
                    keys.sort();
                    keys.join(",")
                })
            })
            .unwrap_or_else(|| "none".to_string());

        Ok(format!(
            "raw_post_count={raw_post_count} text_missing_count={text_missing_count} sample_raw_json_keys={raw_json_keys}"
        ))
    }
}
