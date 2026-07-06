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

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
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

    async fn categorize_claude(&self, api_key: &str, merchant: &str, description: &str) -> Result<Categorization> {
        let system_prompt = "You are a financial transaction categorization assistant. Categorize transactions into one of these categories: Income, Groceries, Dining Out, Transportation, Utilities, Home & Property, Subscriptions, Shopping, Healthcare, Personal Care, Entertainment, Transfers, Interest, Debt Payments, Uncategorized.

Return a JSON object with:
- category: primary category (string)
- secondary_category: detailed subcategory if confidence >= 0.85 (string or null)
- confidence: confidence score 0.0-1.0 (number)

Confidence guidelines:
- 0.9+: High confidence (very likely correct)
- 0.7-0.89: Medium confidence (reasonable guess)
- <0.7: Low confidence (uncertain)

Only return valid JSON, no other text.".to_string();

        let user_message = format!(
            "Categorize this transaction:\nMerchant: {}\nDescription: {}",
            merchant, description
        );

        let request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 256,
            system: system_prompt,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: user_message,
            }],
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Claude API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "Claude API returned status {}: {}",
                status, error_text
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Failed to parse Claude response: {}", e)))?;

        // Extract text from response content
        let response_text = claude_response
            .content
            .iter()
            .find_map(|block| {
                if block.content_type == "text" {
                    block.text.as_ref()
                } else {
                    None
                }
            })
            .ok_or_else(|| AppError::Llm("No text content in Claude response".to_string()))?;

        // Parse the JSON response
        let parsed: LlmCategoryResponse = serde_json::from_str(response_text)
            .map_err(|e| AppError::Llm(format!("Failed to parse Claude JSON response: {}", e)))?;

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
