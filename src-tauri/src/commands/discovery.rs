use crate::models::threads::{DiscoveryCrawlResult, DiscoverySeedTestResult};
use crate::services::discovery_crawler;

#[tauri::command]
pub fn run_discovery_crawl(
    region_seed_group: Option<String>,
    max_per_seed: Option<usize>,
    dry_run: Option<bool>,
) -> Result<DiscoveryCrawlResult, String> {
    discovery_crawler::run_discovery_crawl(region_seed_group, max_per_seed, dry_run)
}

#[tauri::command]
pub fn test_discovery_seed(keyword: String) -> Result<DiscoverySeedTestResult, String> {
    discovery_crawler::test_discovery_seed(keyword)
}
