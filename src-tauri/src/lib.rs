pub mod db;
pub mod errors;
pub mod simplefin;
pub mod llm;
pub mod commands;
pub mod models;
pub mod calculator;
pub mod keychain;
pub mod sync_orchestrator;
pub mod sync_state;
#[cfg(test)]
mod db_integration_tests;
#[cfg(test)]
mod performance_tests;

pub use errors::{AppError, Result};
