use crate::models::threads::{SampleThreadsImportResult, ThreadsCollectionResult};
use crate::services::threads_client;

#[tauri::command]
pub fn collect_threads_by_keyword(keyword: String) -> Result<ThreadsCollectionResult, String> {
    threads_client::collect_threads_by_keyword(keyword)
}

#[tauri::command]
pub fn import_sample_threads_posts() -> Result<SampleThreadsImportResult, String> {
    threads_client::import_sample_threads_posts()
}
