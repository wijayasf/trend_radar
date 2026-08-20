pub mod apify_connector;
pub mod candidate_review;
pub mod cost_classifier;
pub mod discovery_crawler;
pub mod duckdb_service;
pub mod entity_detector;
#[allow(dead_code)]
pub mod identity_bootstrap;
pub mod identity_linker;
// Phase A persistence is intentionally not wired to commands or UI yet.
#[allow(dead_code)]
pub mod multi_source_repository;
pub mod region_classifier;
pub mod report_exporter;
pub mod sentiment_classifier;
pub mod threads;
pub mod threads_client;
pub mod weekly_aggregator;
