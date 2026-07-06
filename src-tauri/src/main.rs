// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber;
use std::path::PathBuf;

mod db;
mod errors;
mod simplefin;
mod llm;
mod commands;
mod models;
mod calculator;
mod keychain;

use db::Database;
use llm::LlmClient;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize database
    let data_dir = if let Some(home) = dirs::home_dir() {
        home.join(".config/momentum")
    } else {
        PathBuf::from(".momentum")
    };

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    }

    let db_path = data_dir.join("momentum.db");
    let database = Database::new(db_path.to_str().unwrap())
        .expect("Failed to initialize database");

    // Initialize LLM client
    // Try to read from environment variables or keychain
    let ollama_url = std::env::var("OLLAMA_URL").ok()
        .or_else(|| Some("http://localhost:11434".to_string()));

    let api_key = std::env::var("LLM_API_KEY").ok()
        .or_else(|| keychain::Keychain::retrieve_llm_api_key().ok());

    let llm_client = LlmClient::new(ollama_url, api_key);

    tauri::Builder::default()
        .manage(database)
        .manage(llm_client)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_metrics,
            commands::get_transactions,
            commands::claim_setup_token,
            commands::sync_simplefin,
            commands::get_accounts,
            commands::set_debt_terms,
            commands::recategorize_transaction,
            commands::get_opportunity_scenarios,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
