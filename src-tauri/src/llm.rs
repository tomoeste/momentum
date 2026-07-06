use serde::{Serialize, Deserialize};
use crate::errors::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Categorization {
    pub category: String,
    pub secondary_category: Option<String>,
    pub confidence: f64,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    format: String,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LlmCategoryResponse {
    category: String,
    secondary_category: Option<String>,
    confidence: f64,
}

pub struct LlmClient {
    ollama_url: Option<String>,
    api_key: Option<String>,
    use_local_first: bool,
    http_client: reqwest::Client,
}

impl LlmClient {
    pub fn new(ollama_url: Option<String>, api_key: Option<String>) -> Self {
        let use_local_first = ollama_url.is_some();
        LlmClient {
            ollama_url,
            api_key,
            use_local_first,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn categorize(&self, merchant: &str, description: &str) -> Result<Categorization> {
        // Try Ollama first if configured
        if let Some(ref ollama_url) = self.ollama_url {
            if let Ok(result) = self.categorize_ollama(ollama_url, merchant, description).await {
                return Ok(result);
            }
            // Fall through to API if Ollama fails
        }

        // Try Claude API if configured
        if let Some(ref api_key) = self.api_key {
            if let Ok(result) = self.categorize_claude(api_key, merchant, description).await {
                return Ok(result);
            }
        }

        // If both fail or neither is configured, return uncategorized with low confidence
        Ok(Categorization {
            category: "Uncategorized".to_string(),
            secondary_category: None,
            confidence: 0.0,
            note: Some("No LLM available for categorization".to_string()),
        })
    }

    async fn categorize_ollama(&self, ollama_url: &str, merchant: &str, description: &str) -> Result<Categorization> {
        let prompt = format!(
            "Categorize this financial transaction into one of these categories: Income, Groceries, Dining Out, Transportation, Utilities, Home & Property, Subscriptions, Shopping, Healthcare, Personal Care, Entertainment, Transfers, Interest, Debt Payments, Uncategorized.

Merchant: {}
Description: {}

Respond with a JSON object containing:
- category (string): the primary category
- secondary_category (string or null): detailed subcategory if confidence >= 0.85
- confidence (number): 0.0-1.0 confidence score

Confidence guidelines:
- 0.9+: High confidence (very likely correct)
- 0.7-0.89: Medium confidence (reasonable guess)
- <0.7: Low confidence (uncertain)

Output only valid JSON, no other text.",
            merchant, description
        );

        let request = OllamaRequest {
            model: "mistral".to_string(), // Default model, can be configurable
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            format: "json".to_string(),
            stream: false,
        };

        let response = self
            .http_client
            .post(format!("{}/api/chat", ollama_url))
            .json(&request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Llm(format!(
                "Ollama returned status: {}",
                response.status()
            )));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Failed to parse Ollama response: {}", e)))?;

        // Parse the JSON from the model's response
        let parsed: LlmCategoryResponse = serde_json::from_str(&ollama_response.message.content)
            .map_err(|e| AppError::Llm(format!("Failed to parse category JSON: {}", e)))?;

        // Only include secondary category if confidence >= 0.85
        let secondary = if parsed.confidence >= 0.85 {
            parsed.secondary_category
        } else {
            None
        };

        Ok(Categorization {
            category: parsed.category,
            secondary_category: secondary,
            confidence: parsed.confidence.clamp(0.0, 1.0),
            note: None,
        })
    }

    async fn categorize_claude(&self, _api_key: &str, _merchant: &str, _description: &str) -> Result<Categorization> {
        // For now, return a placeholder since we don't have Claude SDK integrated
        // This would require adding the Anthropic SDK or making raw HTTP requests
        // For MVP, focus on Ollama since it's local-first
        tracing::warn!("Claude API categorization not yet implemented, falling back to Uncategorized");
        Ok(Categorization {
            category: "Uncategorized".to_string(),
            secondary_category: None,
            confidence: 0.0,
            note: Some("Claude API not yet implemented".to_string()),
        })
    }

    pub async fn health_check(&self) -> Result<bool> {
        // Check Ollama health if configured
        if let Some(ref ollama_url) = self.ollama_url {
            let response = self
                .http_client
                .get(format!("{}/api/tags", ollama_url))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;

            if response.is_ok() {
                tracing::info!("Ollama health check passed");
                return Ok(true);
            }
        }

        // Could add Claude health check here in the future
        tracing::warn!("No LLM health check succeeded");
        Ok(false)
    }
}
