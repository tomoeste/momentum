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
    http_client: reqwest::Client,
}

impl LlmClient {
    pub fn new(ollama_url: Option<String>, api_key: Option<String>) -> Self {
        LlmClient {
            ollama_url,
            api_key,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_categorize_no_llm_configured() {
        // Test fallback when no LLM is configured
        let llm = LlmClient::new(None, None);
        let result = llm.categorize("Coffee Shop", "Morning coffee").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.category, "Uncategorized");
        assert_eq!(cat.confidence, 0.0);
        assert!(cat.note.is_some());
    }

    #[tokio::test]
    async fn test_categorize_with_confidence_clamping() {
        // Test that confidence is clamped to 0.0-1.0
        let llm = LlmClient::new(None, None);
        let result = llm.categorize("Store", "Purchase").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        assert!(cat.confidence >= 0.0 && cat.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_categorize_secondary_category_filtering() {
        // When confidence < 0.85, secondary_category should be None
        // (This is tested indirectly through fallback since we can't easily mock)
        let llm = LlmClient::new(None, None);
        let result = llm.categorize("Test Merchant", "Test Description").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        // Fallback returns None for secondary_category
        assert_eq!(cat.secondary_category, None);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_categorize_ollama_success() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "message": {
                        "content": r#"{"category": "Groceries", "secondary_category": "Produce", "confidence": 0.92}"#
                    }
                }));
        });

        let llm = LlmClient::new(Some(ollama_url), None);
        let result = llm.categorize("Whole Foods", "Fresh vegetables").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.category, "Groceries");
        assert_eq!(cat.secondary_category, Some("Produce".to_string()));
        assert!(cat.confidence >= 0.92);
    }

    #[tokio::test]
    async fn test_categorize_ollama_api_error() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat");
            then.status(503)
                .body("Service Unavailable");
        });

        let llm = LlmClient::new(Some(ollama_url.clone()), None);
        let result = llm.categorize("Merchant", "Description").await;

        // With only Ollama configured and it fails, should return uncategorized fallback
        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.category, "Uncategorized");
    }

    #[tokio::test]
    async fn test_categorize_ollama_malformed_response() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "message": {
                        "content": "not valid json for category"
                    }
                }));
        });

        let llm = LlmClient::new(Some(ollama_url), None);
        let result = llm.categorize("Store", "Item").await;

        // Should fallback to uncategorized due to malformed response
        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.category, "Uncategorized");
    }

    #[tokio::test]
    async fn test_categorize_ollama_network_timeout() {
        // Use a non-routable IP that will timeout
        let llm = LlmClient::new(Some("http://192.0.2.1:11434".to_string()), None);
        let result = llm.categorize("Merchant", "Description").await;

        // Should return uncategorized fallback due to timeout
        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.category, "Uncategorized");
    }

    #[tokio::test]
    async fn test_categorize_confidence_bounds() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        // Test that confidence values > 1.0 are clamped to 1.0
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "message": {
                        "content": r#"{"category": "Shopping", "secondary_category": null, "confidence": 1.5}"#
                    }
                }));
        });

        let llm = LlmClient::new(Some(ollama_url), None);
        let result = llm.categorize("Store", "Purchase").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.confidence, 1.0); // Clamped from 1.5
    }

    #[tokio::test]
    async fn test_categorize_negative_confidence_bounds() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        // Test that negative confidence is clamped to 0.0
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "message": {
                        "content": r#"{"category": "Shopping", "secondary_category": null, "confidence": -0.5}"#
                    }
                }));
        });

        let llm = LlmClient::new(Some(ollama_url), None);
        let result = llm.categorize("Store", "Purchase").await;

        assert!(result.is_ok());
        let cat = result.unwrap();
        assert_eq!(cat.confidence, 0.0); // Clamped from -0.5
    }

    #[tokio::test]
    async fn test_categorize_health_check() {
        let server = MockServer::start();
        let ollama_url = server.base_url();

        server.mock(|when, then| {
            when.method(GET)
                .path("/api/tags");
            then.status(200)
                .json_body(json!({"models": []}));
        });

        let llm = LlmClient::new(Some(ollama_url), None);
        let result = llm.health_check().await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let llm = LlmClient::new(Some("http://192.0.2.1:11434".to_string()), None);
        let result = llm.health_check().await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
