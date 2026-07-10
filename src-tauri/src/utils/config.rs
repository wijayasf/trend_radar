use std::collections::HashSet;
use std::env;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

pub const DEFAULT_DATABASE_PATH: &str = "./data/app.duckdb";
pub const DEFAULT_APP_ENV: &str = "local";
pub const THREADS_ACCESS_TOKEN_ENV: &str = "THREADS_ACCESS_TOKEN";
pub const THREADS_USER_ID_ENV: &str = "THREADS_USER_ID";
pub const APP_ENV_ENV: &str = "APP_ENV";
pub const DATABASE_PATH_ENV: &str = "DATABASE_PATH";

static ENV_FILE_LOADED: OnceLock<bool> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct EnvConfigStatus {
    pub threads_access_token_configured: bool,
    pub threads_user_id_configured: bool,
    pub app_env: String,
    pub app_env_configured: bool,
    pub database_path: String,
    pub database_path_configured: bool,
    pub env_file_loaded: bool,
}

pub fn load_env_files_once() -> bool {
    *ENV_FILE_LOADED.get_or_init(load_first_env_file)
}

pub fn env_config_status() -> EnvConfigStatus {
    let env_file_loaded = load_env_files_once();
    let app_env = env::var(APP_ENV_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_APP_ENV.to_string());
    let database_path = resolved_database_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| error);

    EnvConfigStatus {
        threads_access_token_configured: env_value_configured(THREADS_ACCESS_TOKEN_ENV),
        threads_user_id_configured: env_value_configured(THREADS_USER_ID_ENV),
        app_env_configured: env_value_configured(APP_ENV_ENV),
        app_env,
        database_path_configured: env_value_configured(DATABASE_PATH_ENV),
        database_path,
        env_file_loaded,
    }
}

pub fn env_value_configured(key: &str) -> bool {
    env::var(key)
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn resolved_database_path() -> Result<PathBuf, String> {
    load_env_files_once();

    let configured_path = env::var(DATABASE_PATH_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_DATABASE_PATH.to_string());

    let raw_path = PathBuf::from(configured_path);
    let resolved_path = if raw_path.is_absolute() {
        normalize_path(&raw_path)
    } else {
        normalize_path(&project_root().join(raw_path))
    };

    validate_runtime_database_path(&resolved_path)?;
    Ok(resolved_path)
}

pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".."))
}

fn validate_runtime_database_path(path: &Path) -> Result<(), String> {
    let forbidden_runtime_dir = normalize_path(&project_root().join("src-tauri").join("data"));
    if path.starts_with(&forbidden_runtime_dir) {
        Err(
            "Invalid database path: runtime database must not be stored inside src-tauri"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

fn load_first_env_file() -> bool {
    for candidate in env_file_candidates() {
        if candidate.exists() && dotenvy::from_path(&candidate).is_ok() {
            return true;
        }
    }

    false
}

fn env_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join(".env"));
        candidates.push(current_dir.join("..").join(".env"));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join(".env"));
    candidates.push(manifest_dir.join("..").join(".env"));

    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            for ancestor in exe_dir.ancestors().take(8) {
                candidates.push(ancestor.join(".env"));
                candidates.push(ancestor.join("..").join(".env"));
            }
        }
    }

    deduplicate_paths(candidates)
}

fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduplicated = Vec::new();

    for path in paths {
        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            deduplicated.push(path);
        }
    }

    deduplicated
}
