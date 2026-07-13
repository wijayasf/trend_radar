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
            commands::threads::collect_threads_by_keyword,
            commands::threads::import_sample_threads_posts,
            commands::entities::detect_agent_mentions,
            commands::regions::classify_regions,
            commands::sentiments::classify_sentiments,
            commands::costs::classify_cost_signals,
            commands::weekly::aggregate_weekly_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::services::{
        cost_classifier, duckdb_service, entity_detector, region_classifier, sentiment_classifier,
        threads_client, weekly_aggregator,
    };

    #[test]
    fn validates_sample_full_mvp_flow() {
        let database_path = temp_database_path();
        cleanup_database_files(&database_path);
        std::env::set_var("DATABASE_PATH", database_path.to_string_lossy().as_ref());

        let import_result =
            threads_client::import_sample_threads_posts().expect("sample import should succeed");
        assert_eq!(import_result.loaded_count, 10);
        assert_eq!(import_result.saved_count, 10);
        assert_eq!(
            duckdb_service::count_threads_raw_posts().expect("raw post count should be readable"),
            10
        );

        let entity_result =
            entity_detector::detect_agent_mentions().expect("entity detection should succeed");
        assert_eq!(entity_result.mentions_found, 12);
        assert_eq!(entity_result.saved_count, 12);

        let region_result =
            region_classifier::classify_regions().expect("region classification should succeed");
        assert_eq!(region_result.indonesia_count, 4);
        assert_eq!(region_result.global_count, 4);
        assert_eq!(region_result.unknown_count, 2);
        assert_eq!(region_result.updated_mentions_count, 12);

        let sentiment_result = sentiment_classifier::classify_sentiments()
            .expect("sentiment classification should succeed");
        assert_eq!(sentiment_result.positive_count, 4);
        assert_eq!(sentiment_result.neutral_count, 5);
        assert_eq!(sentiment_result.negative_count, 1);
        assert_eq!(sentiment_result.mixed_count, 2);
        assert_eq!(sentiment_result.updated_mentions_count, 12);

        let cost_result =
            cost_classifier::classify_cost_signals().expect("cost classification should succeed");
        assert_eq!(cost_result.not_mentioned_count, 9);
        assert_eq!(cost_result.cost_positive_count, 1);
        assert_eq!(cost_result.cost_negative_boros_count, 1);
        assert_eq!(cost_result.cost_mixed_count, 1);
        assert_eq!(cost_result.updated_mentions_count, 12);

        let weekly_result = weekly_aggregator::aggregate_weekly_metrics()
            .expect("weekly aggregation should succeed");
        assert!(weekly_result.metrics_count > 0);
        assert!(weekly_result.indonesia_count > 0);
        assert!(weekly_result.global_count > 0);
        assert!(weekly_result
            .top_indonesia
            .iter()
            .any(|metric| metric.agent_name == "Claude Code"));
        assert!(weekly_result
            .top_indonesia
            .iter()
            .any(|metric| metric.agent_name == "MCP"));
        assert!(weekly_result
            .top_global
            .iter()
            .any(|metric| metric.agent_name == "Cursor"));
        assert!(weekly_result
            .top_unknown
            .iter()
            .any(|metric| metric.agent_name == "Ponytail"));

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
}
