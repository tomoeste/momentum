pub mod db;
pub mod errors;
pub mod simplefin;
pub mod llm;
pub mod commands;
pub mod models;
pub mod calculator;
pub mod keychain;

pub use errors::{AppError, Result};
