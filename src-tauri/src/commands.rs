use serde::{Serialize, Deserialize};
use crate::errors::Result;
use crate::models::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDashboardMetricsRequest {
    pub period: Period,
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
