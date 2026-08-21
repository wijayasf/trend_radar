pub mod apify_connector;
pub mod candidate_review;
pub mod canonical_weekly_aggregator;
pub mod cost_classifier;
pub mod cross_source_score_aggregator;
pub mod discovery_crawler;
pub mod duckdb_service;
pub mod entity_detector;
pub mod explainx_importer;
pub mod external_identity_review;
#[allow(dead_code)]
pub mod identity_bootstrap;
pub mod identity_linker;
// Some foundation repository methods remain internal until later source-review milestones.
#[allow(dead_code)]
pub mod multi_source_repository;
pub mod region_classifier;
pub mod report_exporter;
pub mod sentiment_classifier;
pub mod threads;
pub mod threads_client;
pub mod weekly_aggregator;
