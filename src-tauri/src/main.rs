// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tracing_subscriber;

mod db;
mod errors;
mod simplefin;
mod llm;
mod commands;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_metrics,
            commands::get_transactions,
            commands::sync_simplefin,
            commands::get_accounts,
            commands::set_debt_terms,
            commands::recategorize_transaction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
