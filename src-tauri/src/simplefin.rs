use crate::errors::{AppError, Result};
use crate::models::{Account, AccountType, RawTransaction};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

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
        let client = reqwest::Client::new();
        let request = ClaimTokenRequest {
            setup_token: setup_token.to_string(),
        };

        let response = client
            .post("https://auth.simplefin.com/claim")
            .json(&request)
            .header("User-Agent", "Momentum/1.0 (compatible with SimpleFIN API)")
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
