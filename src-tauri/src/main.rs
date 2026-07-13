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
            commands::discovery::run_discovery_crawl,
            commands::threads::collect_threads_by_keyword,
            commands::threads::import_sample_threads_posts,
            commands::entities::detect_agent_mentions,
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

    use crate::services::{
        cost_classifier, discovery_crawler, duckdb_service, entity_detector, region_classifier,
        report_exporter, sentiment_classifier, weekly_aggregator,
    };

    #[test]
    fn validates_sample_full_mvp_flow() {
        let database_path = temp_database_path();
        cleanup_database_files(&database_path);
        std::env::set_var("DATABASE_PATH", database_path.to_string_lossy().as_ref());
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
        assert_eq!(discovery_result.saved_total, 3);
        assert!(discovery_result.duplicates_skipped > 0);
        assert_eq!(
            duckdb_service::count_threads_raw_posts().expect("raw post count should be readable"),
            3
        );

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
        std::env::set_var("DATABASE_PATH", database_path.to_string_lossy().as_ref());
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

    fn cleanup_database_files(database_path: &PathBuf) {
        let _ = fs::remove_file(database_path);
        let _ = fs::remove_file(database_path.with_extension("duckdb.wal"));
        let _ = fs::remove_file(database_path.with_extension("duckdb.tmp"));
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
