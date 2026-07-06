use serde::{Serialize, Deserialize};
use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Categorization {
    pub category: String,
    pub secondary_category: Option<String>,
    pub confidence: f64,
    pub note: Option<String>,
}

pub struct LlmClient {
    ollama_url: Option<String>,
    api_key: Option<String>,
    use_local_first: bool,
}

impl LlmClient {
    pub fn new(ollama_url: Option<String>, api_key: Option<String>) -> Self {
        let use_local_first = ollama_url.is_some();
        LlmClient {
            ollama_url,
            api_key,
            use_local_first,
        }
    }

    pub async fn categorize(&self, merchant: &str, description: &str) -> Result<Categorization> {
        // TODO: implement categorization via Ollama or API
        Ok(Categorization {
            category: "Uncategorized".to_string(),
            secondary_category: None,
            confidence: 0.0,
            note: None,
        })
    }

    pub async fn health_check(&self) -> Result<bool> {
        // TODO: check if Ollama or API is available
        Ok(true)
    }
}
