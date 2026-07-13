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
}
