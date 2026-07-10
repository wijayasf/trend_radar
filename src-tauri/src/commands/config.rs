use crate::utils::config::{self, EnvConfigStatus};

#[tauri::command]
pub fn env_config_status() -> EnvConfigStatus {
    config::env_config_status()
}
