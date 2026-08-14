use crate::models::threads::ApifyDiscoveryResult;
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
