mod commands;
mod models;
mod services;
mod utils;

fn main() {
    utils::config::load_env_files_once();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::health::app_health,
            commands::config::env_config_status,
            commands::database::check_database_health,
            commands::database::count_threads_raw_posts,
            commands::threads::collect_threads_by_keyword,
            commands::threads::import_sample_threads_posts,
            commands::entities::detect_agent_mentions,
            commands::regions::classify_regions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
