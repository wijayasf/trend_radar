#[tauri::command]
pub fn app_health() -> &'static str {
    "ok"
}
