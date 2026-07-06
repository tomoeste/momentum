use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use tauri::State;
use crate::errors::{Result, AppError};
use crate::models::*;
use crate::calculator;
use crate::db::Database;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDashboardMetricsRequest {
    pub period: Period,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOpportunityScenariosResponse {
    pub scenarios: Vec<ScenarioResponse>,
    pub total_debt: f64,
    pub weighted_apr: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioResponse {
    pub monthly_cut: f64,
    pub months_saved: f64,
    pub interest_saved: f64,
    pub new_payoff_months: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionsRequest {
    pub account_id: Option<String>,
    pub category: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetDebtTermsRequest {
    pub account_id: String,
    pub interest_rate: f64,
    pub minimum_payment: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecategorizeTransactionRequest {
    pub transaction_id: String,
    pub category: String,
    pub secondary_category: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncSimplefinRequest {
    pub days_back: Option<u32>,  // Default: 90 days
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimSetupTokenRequest {
    pub setup_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimSetupTokenResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleFINStatusResponse {
    pub connected: bool,
    pub account_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisconnectSimpleFINResponse {
    pub success: bool,
    pub message: String,
}

// Tauri command handlers
#[tauri::command]
pub async fn get_dashboard_metrics(
    req: GetDashboardMetricsRequest,
    db: State<'_, Database>,
) -> Result<DashboardMetrics> {
    // Calculate date range based on period
    let now = Utc::now();
    let (start_date, _days_back) = match req.period {
        Period::Week => (now - Duration::days(7), 7),
        Period::Month => (now - Duration::days(30), 30),
    };

    let start_date_str = start_date.format("%Y-%m-%d").to_string();
    let end_date_str = now.format("%Y-%m-%d").to_string();

    // Get metrics from database
    let (income, spending, debt_paydown, interest_paid) = db.get_metrics(&start_date_str, &end_date_str)?;

    // Calculate debt ratio
    let debt_ratio = db.get_debt_ratio()?;

    // Calculate interest as percentage of income
    let interest_as_pct_income = if income > 0.0 {
        (interest_paid / income) * 100.0
    } else {
        0.0
    };

    // Get sparkline data (last 28 days)
    let sparkline_data = db.get_sparkline(&end_date_str)?;

    // Get last sync timestamp
    let last_sync = db.get_last_sync()?;

    Ok(DashboardMetrics {
        period: req.period,
        period_start: start_date_str,
        period_end: end_date_str,
        income,
        spending,
        debt_paydown,
        interest_paid,
        debt_ratio,
        interest_as_pct_income,
        sparkline_data,
        last_sync,
    })
}

#[tauri::command]
pub async fn get_transactions(
    req: GetTransactionsRequest,
    db: State<'_, Database>,
) -> Result<Vec<RawTransaction>> {
    let limit = req.limit.unwrap_or(100).min(5000).max(1);
    let offset = req.offset.unwrap_or(0).max(0);

    // Get base transactions
    let mut transactions = db.get_transactions(
        req.account_id.as_deref(),
        10000,  // Get more from DB to filter
        0,
    )?;

    // Filter by date range if provided
    if req.start_date.is_some() || req.end_date.is_some() {
        let start = req.start_date.as_ref().and_then(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| d.and_hms_opt(0, 0, 0).unwrap_or(chrono::NaiveDateTime::MIN).and_utc())
        });
        let end = req.end_date.as_ref().and_then(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| d.and_hms_opt(23, 59, 59).unwrap_or(chrono::NaiveDateTime::MAX).and_utc())
        });

        if let Some(start_date) = start {
            transactions.retain(|tx| tx.posted_date >= start_date);
        }
        if let Some(end_date) = end {
            transactions.retain(|tx| tx.posted_date <= end_date);
        }
    }

    // Note: Category filtering would require joining with categorized_transactions
    // which is more complex and better suited to a database method if needed frequently
    // For now, basic filtering is here; advanced filtering can be added to db.rs if needed

    // Sort by date descending (most recent first)
    transactions.sort_by(|a, b| b.posted_date.cmp(&a.posted_date));

    // Apply limit and offset
    let paginated: Vec<_> = transactions
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(paginated)
}

#[tauri::command]
pub async fn claim_setup_token(req: ClaimSetupTokenRequest) -> Result<ClaimSetupTokenResponse> {
    // Claim the setup token from SimpleFIN
    let access_url = crate::simplefin::SimpleFin::claim_token(&req.setup_token).await?;

    // Validate access URL format
    crate::simplefin::SimpleFin::validate_access_url(&access_url)?;

    // Test connection to ensure credentials are valid
    let client = crate::simplefin::SimpleFin::new(access_url.clone());
    client.test_connection().await?;

    // Store in keychain (never expose to frontend)
    crate::keychain::Keychain::store_simplefin_access_url(&access_url)?;

    Ok(ClaimSetupTokenResponse {
        success: true,
        message: "SimpleFIN connected successfully".to_string(),
    })
}

#[tauri::command]
pub async fn sync_simplefin(
    req: SyncSimplefinRequest,
    db: State<'_, Database>,
    llm: State<'_, crate::llm::LlmClient>,
) -> Result<SyncStatus> {
    let start_time = std::time::Instant::now();
    let days_back = req.days_back.unwrap_or(90);

    // Retrieve access URL from keychain
    let access_url = crate::keychain::Keychain::retrieve_simplefin_access_url()?;

    // Validate access URL format
    crate::simplefin::SimpleFin::validate_access_url(&access_url)?;

    // Create SimpleFIN client
    let client = crate::simplefin::SimpleFin::new(access_url);

    // Fetch accounts
    let accounts = match client.fetch_accounts().await {
        Ok(accounts) => accounts,
        Err(e) => {
            let error_msg = format!("{:?}", e);
            db.insert_sync_log("failed", 0, Some(&error_msg), start_time.elapsed().as_millis() as i32)?;
            return Err(e);
        }
    };

    // Upsert accounts into database
    let mut transaction_count = 0;
    for account in accounts {
        if let Err(e) = db.insert_account(&account) {
            tracing::error!("Failed to insert account {}: {:?}", account.id, e);
        }
    }

    // Fetch transactions
    let transactions = match client.fetch_transactions(days_back).await {
        Ok(txns) => txns,
        Err(e) => {
            let error_msg = format!("{:?}", e);
            db.insert_sync_log("failed", transaction_count, Some(&error_msg), start_time.elapsed().as_millis() as i32)?;
            return Err(e);
        }
    };

    // Upsert transactions and auto-categorize
    for txn in transactions {
        // SimpleFIN doesn't give account_id; we'd need to match by merchant or prompt user
        // For now, skip or log this limitation
        if let Err(e) = db.insert_transaction(&txn) {
            tracing::error!("Failed to insert transaction {}: {:?}", txn.id, e);
        } else {
            transaction_count += 1;

            // Auto-categorize the transaction using LLM
            let merchant = txn.merchant.as_deref().unwrap_or(&txn.description);
            let description = &txn.description;

            match llm.categorize(merchant, description).await {
                Ok(categorization) => {
                    // Insert categorization into database
                    match db.categorize_transaction_with_params(
                        &txn.id,
                        &categorization.category,
                        categorization.secondary_category.as_deref(),
                        categorization.confidence,
                        categorization.note.as_deref(),
                        false, // is_manual = false for auto-categorization
                    ) {
                        Ok(_) => {
                            tracing::debug!(
                                "Categorized transaction {} as {} (confidence: {:.2})",
                                txn.id,
                                categorization.category,
                                categorization.confidence
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to store categorization for {}: {:?}", txn.id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to categorize transaction {}: {:?}", txn.id, e);
                    // Don't fail the sync, just log the warning
                }
            }
        }
    }

    // Log successful sync
    db.insert_sync_log(
        "success",
        transaction_count,
        None,
        start_time.elapsed().as_millis() as i32,
    )?;

    Ok(SyncStatus {
        in_progress: false,
        last_sync: Some(Utc::now()),
        last_error: None,
        transaction_count,
    })
}

#[tauri::command]
pub async fn get_accounts(db: State<'_, Database>) -> Result<Vec<Account>> {
    db.get_accounts()
}

#[tauri::command]
pub async fn set_debt_terms(
    req: SetDebtTermsRequest,
    db: State<'_, Database>,
) -> Result<()> {
    // Validate APR is between 0 and 1 (0-100% expressed as decimal)
    if req.interest_rate < 0.0 || req.interest_rate > 1.0 {
        return Err(AppError::Validation(
            "Interest rate must be between 0 and 1 (0-100%)".to_string(),
        ));
    }

    // Validate minimum payment is positive if provided
    if let Some(min_pay) = req.minimum_payment {
        if min_pay < 0.0 {
            return Err(AppError::Validation(
                "Minimum payment must be non-negative".to_string(),
            ));
        }
    }

    // Get the account to ensure it exists and use its details
    let accounts = db.get_accounts()?;
    let account = accounts
        .iter()
        .find(|a| a.id == req.account_id)
        .ok_or_else(|| AppError::NotFound(format!("Account {} not found", req.account_id)))?;

    // Create debt account record
    let debt_account = DebtAccount {
        id: format!("debt_{}", req.account_id),
        simplefin_account_id: account.simplefin_account_id.clone(),
        name: account.name.clone(),
        account_type: account.account_type,
        current_balance: account.balance,
        interest_rate: req.interest_rate,
        minimum_payment: req.minimum_payment,
        last_updated: Utc::now(),
    };

    db.insert_debt_account(&debt_account)
}

#[tauri::command]
pub async fn recategorize_transaction(
    req: RecategorizeTransactionRequest,
    db: State<'_, Database>,
) -> Result<()> {
    let categorized = CategorizedTransaction {
        id: req.transaction_id,
        category: req.category,
        secondary_category: req.secondary_category,
        confidence: 1.0, // Manual categorization has 100% confidence
        note: req.note,
        categorized_at: Utc::now(),
        is_manual: true,
    };

    db.categorize_transaction(&categorized.id, &categorized)
}

#[tauri::command]
pub async fn get_opportunity_scenarios(db: State<'_, Database>) -> Result<GetOpportunityScenariosResponse> {
    let standard_cuts = vec![200.0, 500.0];

    let debt_accounts = db.get_debt_accounts()?;

    if debt_accounts.is_empty() {
        return Ok(GetOpportunityScenariosResponse {
            scenarios: vec![],
            total_debt: 0.0,
            weighted_apr: 0.0,
        });
    }

    let scenarios = calculator::calculate_scenarios(&debt_accounts, &standard_cuts);

    let total_debt: f64 = debt_accounts.iter().map(|a| a.current_balance).sum();
    let weighted_apr = if total_debt > 0.0 {
        debt_accounts
            .iter()
            .map(|a| a.interest_rate * (a.current_balance / total_debt))
            .sum()
    } else {
        0.0
    };

    Ok(GetOpportunityScenariosResponse {
        scenarios: scenarios
            .into_iter()
            .map(|s| ScenarioResponse {
                monthly_cut: s.monthly_cut,
                months_saved: s.months_saved,
                interest_saved: s.interest_saved,
                new_payoff_months: s.new_payoff_months,
            })
            .collect(),
        total_debt,
        weighted_apr,
    })
}

#[tauri::command]
pub async fn get_simplefin_status(db: State<'_, Database>) -> Result<SimpleFINStatusResponse> {
    let connected = crate::keychain::Keychain::has_simplefin_access_url()?;

    let account_count = if connected {
        match db.get_accounts() {
            Ok(accounts) => Some(accounts.len()),
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(SimpleFINStatusResponse {
        connected,
        account_count,
    })
}

#[tauri::command]
pub async fn disconnect_simplefin() -> Result<DisconnectSimpleFINResponse> {
    crate::keychain::Keychain::delete_simplefin_access_url()?;

    Ok(DisconnectSimpleFINResponse {
        success: true,
        message: "SimpleFIN disconnected successfully".to_string(),
    })
}
