use crate::models::threads::{ApifyCacheImportResult, ApifyDiscoveryResult};
use crate::services::apify_connector;

#[tauri::command]
pub fn run_apify_discovery_crawl(
    seeds: Option<Vec<String>>,
    max_per_seed: Option<usize>,
) -> Result<ApifyDiscoveryResult, String> {
    apify_connector::run_apify_discovery_crawl(seeds, max_per_seed)
}

#[tauri::command]
pub fn replay_last_apify_crawl() -> Result<ApifyDiscoveryResult, String> {
    apify_connector::replay_last_apify_crawl()
}

#[tauri::command]
pub fn import_apify_dataset_cache(file_path: String) -> Result<ApifyCacheImportResult, String> {
    apify_connector::import_apify_dataset_cache(file_path)
}
