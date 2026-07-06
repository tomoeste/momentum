use crate::errors::{AppError, Result};
use crate::models::{Account, AccountType, RawTransaction};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use base64::{engine::general_purpose, Engine as _};

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleFINBalance {
    pub amount: f64,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleFINAccount {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub balance: SimpleFINBalance,
    #[serde(default)]
    pub account_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleFINAccountsResponse {
    pub accounts: Vec<SimpleFINAccount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleFINTransaction {
    pub id: String,
    pub posted_date: String,
    pub amount: f64,
    #[serde(default)]
    pub merchant: Option<String>,
    pub description: String,
    #[serde(default)]
    pub transaction_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleFINTransactionsResponse {
    pub transactions: Vec<SimpleFINTransaction>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaimTokenRequest {
    pub setup_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaimTokenResponse {
    pub access_url: String,
}

pub struct SimpleFin {
    access_url: String,
    http_client: reqwest::Client,
}

impl SimpleFin {
    pub fn new(access_url: String) -> Self {
        SimpleFin {
            access_url,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn claim_token(setup_token: &str) -> Result<String> {
        // Decode base64-encoded setup token to get the claim URL
        let claim_url = general_purpose::STANDARD
            .decode(setup_token)
            .map_err(|e| AppError::SimpleFin(format!("Failed to decode setup token: {}", e)))
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|e| AppError::SimpleFin(format!("Setup token is not valid UTF-8: {}", e)))
            })?;

        // Validate that the decoded URL is valid
        if !claim_url.starts_with("https://") {
            return Err(AppError::SimpleFin(
                "Decoded claim URL must use HTTPS".to_string(),
            ));
        }

        let client = reqwest::Client::new();

        let response = client
            .post(&claim_url)
            .header("User-Agent", "Momentum/1.0 (compatible with SimpleFIN API)")
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to claim token: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::SimpleFin(format!("Claim failed: {}", error_text)));
        }

        let claim_response: ClaimTokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to parse claim response: {}", e)))?;

        Ok(claim_response.access_url)
    }

    pub async fn fetch_accounts(&self) -> Result<Vec<Account>> {
        let response = self
            .http_client
            .get(&self.access_url)
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to fetch accounts: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::SimpleFin(format!(
                "Accounts request failed: {}",
                response.status()
            )));
        }

        let accounts_response: SimpleFINAccountsResponse = response
            .json()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to parse accounts: {}", e)))?;

        let accounts = accounts_response
            .accounts
            .into_iter()
            .map(|sf_account| Account {
                id: sf_account.id.clone(),
                simplefin_account_id: Some(sf_account.id),
                name: sf_account.name,
                account_type: match sf_account.account_type.as_deref() {
                    Some("credit") => AccountType::CreditCard,
                    Some("loan") => AccountType::Loan,
                    Some("savings") => AccountType::Savings,
                    _ => AccountType::Checking,
                },
                organization: None,
                balance: sf_account.balance.amount,
                last_updated: DateTime::<Utc>::from_timestamp(sf_account.balance.timestamp as i64, 0)
                    .unwrap_or_else(|| Utc::now()),
            })
            .collect();

        Ok(accounts)
    }

    pub async fn fetch_transactions(&self, days_back: u32) -> Result<Vec<RawTransaction>> {
        let start_date = (Utc::now() - Duration::days(days_back as i64))
            .format("%Y-%m-%d")
            .to_string();

        let url = format!("{}?start={}", self.access_url.replace("/accounts", "/transactions"), start_date);

        let response = self
            .http_client
            .get(&url)
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to fetch transactions: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::SimpleFin(format!(
                "Transactions request failed: {}",
                response.status()
            )));
        }

        let txns_response: SimpleFINTransactionsResponse = response
            .json()
            .await
            .map_err(|e| AppError::SimpleFin(format!("Failed to parse transactions: {}", e)))?;

        let transactions = txns_response
            .transactions
            .into_iter()
            .map(|sf_txn| {
                let posted_date = DateTime::parse_from_rfc3339(&sf_txn.posted_date)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                RawTransaction {
                    id: sf_txn.id,
                    account_id: String::new(), // Will be populated by sync command
                    posted_date,
                    amount: sf_txn.amount,
                    merchant: sf_txn.merchant,
                    description: sf_txn.description,
                    transaction_type: sf_txn.transaction_type.unwrap_or_else(|| "unknown".to_string()),
                    imported_at: Utc::now(),
                }
            })
            .collect();

        Ok(transactions)
    }

    pub async fn test_connection(&self) -> Result<()> {
        // Test by fetching accounts to validate credentials
        self.fetch_accounts().await?;
        Ok(())
    }

    pub fn validate_access_url(access_url: &str) -> Result<()> {
        if !access_url.starts_with("https://") {
            return Err(AppError::Validation(
                "Access URL must use HTTPS".to_string(),
            ));
        }

        if !access_url.contains("@") {
            return Err(AppError::Validation(
                "Access URL must contain embedded credentials".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_token_decoding_valid() {
        // "https://auth.simplefin.com/claim?token=abc123" in base64
        let encoded = "aHR0cHM6Ly9hdXRoLnNpbXBsZWZpbi5jb20vY2xhaW0/dG9rZW49YWJjMTIz";
        let decoded_bytes = general_purpose::STANDARD.decode(encoded);

        assert!(decoded_bytes.is_ok());
        let decoded_str = String::from_utf8(decoded_bytes.unwrap());
        assert!(decoded_str.is_ok());
        assert_eq!(
            decoded_str.unwrap(),
            "https://auth.simplefin.com/claim?token=abc123"
        );
    }

    #[test]
    fn test_setup_token_decoding_invalid_base64() {
        let invalid = "not-valid-base64!!!";
        let decoded = general_purpose::STANDARD.decode(invalid);

        assert!(decoded.is_err());
    }

    #[test]
    fn test_validate_access_url_https_required() {
        let http_url = "http://user:password@simplefin.com/api/v3/accounts";
        let result = SimpleFin::validate_access_url(http_url);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }

    #[test]
    fn test_validate_access_url_requires_credentials() {
        let url_no_creds = "https://simplefin.com/api/v3/accounts";
        let result = SimpleFin::validate_access_url(url_no_creds);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("credentials"));
    }

    #[test]
    fn test_validate_access_url_valid() {
        let valid_url = "https://user:password@simplefin.com/api/v3/accounts";
        let result = SimpleFin::validate_access_url(valid_url);

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_claim_token_network_error() {
        // Test that claim_token fails when network is unavailable
        // Using an invalid base64-decoded HTTPS URL that cannot be reached
        let invalid_token = general_purpose::STANDARD.encode("https://localhost:1/nonexistent");
        let result = SimpleFin::claim_token(&invalid_token).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to claim token"));
    }

    #[tokio::test]
    async fn test_fetch_accounts_success() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "accounts": [
                        {
                            "id": "acc_123",
                            "name": "Checking",
                            "currency": "USD",
                            "balance": {
                                "amount": 1000.0,
                                "timestamp": 1234567890
                            },
                            "account_type": "checking"
                        }
                    ]
                }));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_accounts().await;

        assert!(result.is_ok());
        let accounts = result.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "acc_123");
        assert_eq!(accounts[0].name, "Checking");
        assert_eq!(accounts[0].balance, 1000.0);
    }

    #[tokio::test]
    async fn test_fetch_accounts_multiple() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "accounts": [
                        {
                            "id": "acc_123",
                            "name": "Checking",
                            "currency": "USD",
                            "balance": { "amount": 1000.0, "timestamp": 1234567890 },
                            "account_type": "checking"
                        },
                        {
                            "id": "acc_456",
                            "name": "Credit Card",
                            "currency": "USD",
                            "balance": { "amount": 5000.0, "timestamp": 1234567890 },
                            "account_type": "credit"
                        }
                    ]
                }));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_accounts().await;

        assert!(result.is_ok());
        let accounts = result.unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].account_type, AccountType::Checking);
        assert_eq!(accounts[1].account_type, AccountType::CreditCard);
    }

    #[tokio::test]
    async fn test_fetch_accounts_api_error() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(403)
                .body("Forbidden");
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_accounts().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed"));
    }

    #[tokio::test]
    async fn test_fetch_accounts_empty_list() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"accounts": []}));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_accounts().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_transactions_success() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path_contains("/transactions")
                .query_param_exists("start");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "transactions": [
                        {
                            "id": "txn_123",
                            "posted_date": "2024-01-15T10:30:00Z",
                            "amount": -50.0,
                            "merchant": "Coffee Shop",
                            "description": "Coffee",
                            "transaction_type": "debit"
                        }
                    ]
                }));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_transactions(30).await;

        assert!(result.is_ok());
        let transactions = result.unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].id, "txn_123");
        assert_eq!(transactions[0].amount, -50.0);
        assert_eq!(transactions[0].merchant, Some("Coffee Shop".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_transactions_multiple() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path_contains("/transactions");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "transactions": [
                        {
                            "id": "txn_1",
                            "posted_date": "2024-01-15T10:00:00Z",
                            "amount": -50.0,
                            "merchant": "Store A",
                            "description": "Purchase",
                            "transaction_type": "debit"
                        },
                        {
                            "id": "txn_2",
                            "posted_date": "2024-01-16T10:00:00Z",
                            "amount": 2000.0,
                            "merchant": "Employer",
                            "description": "Salary",
                            "transaction_type": "credit"
                        }
                    ]
                }));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_transactions(90).await;

        assert!(result.is_ok());
        let transactions = result.unwrap();
        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].amount, -50.0);
        assert_eq!(transactions[1].amount, 2000.0);
    }

    #[tokio::test]
    async fn test_fetch_transactions_empty() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path_contains("/transactions");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"transactions": []}));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_transactions(30).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_transactions_api_error() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path_contains("/transactions");
            then.status(500)
                .body("Internal Server Error");
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_transactions(30).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed"));
    }

    #[tokio::test]
    async fn test_test_connection_success() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"accounts": []}));
        });

        let client = SimpleFin::new(access_url);
        let result = client.test_connection().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_test_connection_failure() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(401)
                .body("Unauthorized");
        });

        let client = SimpleFin::new(access_url);
        let result = client.test_connection().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_transactions_date_filtering() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path_contains("/transactions")
                .query_param_exists("start");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"transactions": []}));
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_transactions(30).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_accounts_malformed_response() {
        let server = MockServer::start();
        let access_url = format!("{}/accounts", server.base_url());

        server.mock(|when, then| {
            when.method(GET)
                .path("/accounts");
            then.status(200)
                .header("content-type", "application/json")
                .body("not valid json");
        });

        let client = SimpleFin::new(access_url);
        let result = client.fetch_accounts().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse accounts"));
    }
}
