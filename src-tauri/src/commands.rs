use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use crate::errors::{Result, AppError};
use crate::models::*;
use crate::calculator;

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

// Tauri command handlers
#[tauri::command]
pub async fn get_dashboard_metrics(req: GetDashboardMetricsRequest) -> Result<DashboardMetrics> {
    // TODO: implement metrics calculation
    Ok(DashboardMetrics {
        period: req.period,
        income: 0.0,
        spending: 0.0,
        debt_paydown: 0.0,
        interest_paid: 0.0,
        debt_ratio: 0.0,
        last_sync: None,
    })
}

#[tauri::command]
pub async fn get_transactions(req: GetTransactionsRequest) -> Result<Vec<RawTransaction>> {
    // TODO: implement transaction retrieval
    Ok(Vec::new())
}

#[tauri::command]
pub async fn sync_simplefin() -> Result<SyncStatus> {
    // TODO: implement SimpleFIN sync
    Ok(SyncStatus {
        in_progress: false,
        last_sync: None,
        last_error: None,
        transaction_count: 0,
    })
}

#[tauri::command]
pub async fn get_accounts() -> Result<Vec<Account>> {
    // TODO: implement account retrieval
    Ok(Vec::new())
}

#[tauri::command]
pub async fn set_debt_terms(req: SetDebtTermsRequest) -> Result<()> {
    // TODO: implement debt terms setting
    Ok(())
}

#[tauri::command]
pub async fn recategorize_transaction(req: RecategorizeTransactionRequest) -> Result<()> {
    // TODO: implement transaction recategorization
    Ok(())
}

#[tauri::command]
pub async fn get_opportunity_scenarios() -> Result<GetOpportunityScenariosResponse> {
    // In production: get_debt_accounts() from database
    // For now: return mock scenarios with standard reductions ($200, $500)
    let standard_cuts = vec![200.0, 500.0];

    // Mock debt accounts for demonstration
    let mock_debt_accounts = vec![
        DebtAccount {
            id: "debt_1".to_string(),
            simplefin_account_id: Some("sf_123".to_string()),
            name: "Chase Credit Card".to_string(),
            account_type: AccountType::CreditCard,
            current_balance: 5000.0,
            interest_rate: 0.2199, // 21.99% APR
            minimum_payment: Some(150.0),
            last_updated: Utc::now(),
        },
    ];

    let scenarios = calculator::calculate_scenarios(&mock_debt_accounts, &standard_cuts);

    let total_debt: f64 = mock_debt_accounts.iter().map(|a| a.current_balance).sum();
    let weighted_apr = if total_debt > 0.0 {
        mock_debt_accounts
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
