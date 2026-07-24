use crate::services::duckdb_service;

#[tauri::command]
pub fn check_database_health() -> Result<String, String> {
    duckdb_service::check_database_health()
}

#[tauri::command]
pub fn count_threads_raw_posts() -> Result<usize, String> {
    duckdb_service::count_threads_raw_posts()
}

#[tauri::command]
pub fn reset_local_pipeline_data() -> Result<String, String> {
    duckdb_service::reset_local_pipeline_data()
}
