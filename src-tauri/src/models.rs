use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

// Account types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
    Loan,
}

impl AsRef<str> for AccountType {
    fn as_ref(&self) -> &str {
        match self {
            Self::Checking => "checking",
            Self::Savings => "savings",
            Self::CreditCard => "credit_card",
            Self::Loan => "loan",
        }
    }
}

// Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub simplefin_account_id: Option<String>,
    pub name: String,
    pub account_type: AccountType,
    pub organization: Option<String>,
    pub balance: f64,
    pub last_updated: DateTime<Utc>,
}

// Raw transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub id: String,
    pub account_id: String,
    pub posted_date: DateTime<Utc>,
    pub amount: f64,
    pub merchant: Option<String>,
    pub description: String,
    pub transaction_type: String,
    pub imported_at: DateTime<Utc>,
}

// Categorized transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizedTransaction {
    pub id: String,
    pub category: String,
    pub secondary_category: Option<String>,
    pub confidence: f64,
    pub note: Option<String>,
    pub categorized_at: DateTime<Utc>,
    pub is_manual: bool,
}

// Debt account (for APR and minimum payment tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtAccount {
    pub id: String,
    pub simplefin_account_id: Option<String>,
    pub name: String,
    pub account_type: AccountType,
    pub current_balance: f64,
    pub interest_rate: f64, // APR as decimal (0.2199 = 21.99%)
    pub minimum_payment: Option<f64>,
    pub last_updated: DateTime<Utc>,
}

// Dashboard metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub period: Period,
    pub period_start: String,  // ISO 8601 date (YYYY-MM-DD)
    pub period_end: String,    // ISO 8601 date (YYYY-MM-DD)
    pub income: f64,
    pub spending: f64,
    pub debt_paydown: f64,
    pub interest_paid: f64,
    pub debt_ratio: f64,
    pub interest_as_pct_income: f64,  // percentage 0.0..100.0+
    pub sparkline_data: Vec<DailyMetrics>,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Week,
    Month,
}

// Daily metrics for sparkline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMetrics {
    pub date: String,  // YYYY-MM-DD
    pub income: f64,
    pub spending: f64,
    pub debt_paydown: f64,
    pub interest_paid: f64,
}

// Sparkline data point (legacy, kept for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparklinePoint {
    pub date: DateTime<Utc>,
    pub value: f64,
}

// Sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub in_progress: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub transaction_count: i32,
}

// Category for categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Category {
    Income,
    Groceries,
    DiningOut,
    Transportation,
    Utilities,
    HomeAndProperty,
    Subscriptions,
    Shopping,
    Healthcare,
    PersonalCare,
    Entertainment,
    Transfers,
    Interest,
    DebtPayments,
    Uncategorized,
}

impl AsRef<str> for Category {
    fn as_ref(&self) -> &str {
        match self {
            Self::Income => "Income",
            Self::Groceries => "Groceries",
            Self::DiningOut => "Dining Out",
            Self::Transportation => "Transportation",
            Self::Utilities => "Utilities",
            Self::HomeAndProperty => "Home & Property",
            Self::Subscriptions => "Subscriptions",
            Self::Shopping => "Shopping",
            Self::Healthcare => "Healthcare",
            Self::PersonalCare => "Personal Care",
            Self::Entertainment => "Entertainment",
            Self::Transfers => "Transfers",
            Self::Interest => "Interest",
            Self::DebtPayments => "Debt Payments",
            Self::Uncategorized => "Uncategorized",
        }
    }
}
