use rusqlite::{Connection, params, OptionalExtension};
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use crate::errors::{AppError, Result};
use crate::models::*;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Database {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                simplefin_account_id TEXT UNIQUE,
                name TEXT NOT NULL,
                account_type TEXT NOT NULL,
                organization TEXT,
                balance REAL NOT NULL,
                last_updated TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS raw_transactions (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                posted_date TEXT NOT NULL,
                amount REAL NOT NULL,
                merchant TEXT,
                description TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                source TEXT NOT NULL,
                FOREIGN KEY(account_id) REFERENCES accounts(id)
            );

            CREATE INDEX IF NOT EXISTS idx_raw_transactions_posted_date ON raw_transactions(posted_date);
            CREATE INDEX IF NOT EXISTS idx_raw_transactions_account_id ON raw_transactions(account_id);

            CREATE TABLE IF NOT EXISTS categorized_transactions (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                secondary_category TEXT,
                confidence REAL NOT NULL,
                note TEXT,
                categorized_at TEXT NOT NULL,
                is_manual BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY(id) REFERENCES raw_transactions(id)
            );

            CREATE TABLE IF NOT EXISTS debt_accounts (
                id TEXT PRIMARY KEY,
                simplefin_account_id TEXT UNIQUE,
                name TEXT NOT NULL,
                account_type TEXT NOT NULL,
                current_balance REAL NOT NULL,
                interest_rate REAL NOT NULL,
                minimum_payment REAL,
                last_updated TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sync_date TEXT NOT NULL,
                status TEXT NOT NULL,
                transaction_count INTEGER,
                error_message TEXT,
                duration_ms INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_sync_log_sync_date ON sync_log(sync_date);
            CREATE INDEX IF NOT EXISTS idx_categorized_transactions_category ON categorized_transactions(category);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS transaction_mappings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                transaction_id TEXT NOT NULL UNIQUE,
                original_account_id TEXT NOT NULL,
                mapped_account_id TEXT NOT NULL,
                mapped_at TEXT NOT NULL,
                FOREIGN KEY(transaction_id) REFERENCES raw_transactions(id),
                FOREIGN KEY(original_account_id) REFERENCES accounts(id),
                FOREIGN KEY(mapped_account_id) REFERENCES accounts(id)
            );

            CREATE INDEX IF NOT EXISTS idx_transaction_mappings_transaction_id ON transaction_mappings(transaction_id);
            CREATE INDEX IF NOT EXISTS idx_transaction_mappings_mapped_account_id ON transaction_mappings(mapped_account_id);
            "

        ).map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_accounts(&self) -> Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, simplefin_account_id, name, account_type, organization, balance, last_updated FROM accounts")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let accounts = stmt
            .query_map([], |row| {
                Ok(Account {
                    id: row.get(0)?,
                    simplefin_account_id: row.get(1)?,
                    name: row.get(2)?,
                    account_type: match row.get::<_, String>(3)?.as_str() {
                        "checking" => AccountType::Checking,
                        "savings" => AccountType::Savings,
                        "credit_card" => AccountType::CreditCard,
                        "loan" => AccountType::Loan,
                        _ => AccountType::Checking,
                    },
                    organization: row.get(4)?,
                    balance: row.get(5)?,
                    last_updated: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(accounts)
    }

    pub fn insert_account(&self, account: &Account) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let account_type_str = match account.account_type {
            AccountType::Checking => "checking",
            AccountType::Savings => "savings",
            AccountType::CreditCard => "credit_card",
            AccountType::Loan => "loan",
        };

        conn.execute(
            "INSERT OR REPLACE INTO accounts (id, simplefin_account_id, name, account_type, organization, balance, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &account.id,
                &account.simplefin_account_id,
                &account.name,
                account_type_str,
                &account.organization,
                account.balance,
                account.last_updated.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_debt_accounts(&self) -> Result<Vec<DebtAccount>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, simplefin_account_id, name, account_type, current_balance, interest_rate, minimum_payment, last_updated FROM debt_accounts")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let accounts = stmt
            .query_map([], |row| {
                Ok(DebtAccount {
                    id: row.get(0)?,
                    simplefin_account_id: row.get(1)?,
                    name: row.get(2)?,
                    account_type: match row.get::<_, String>(3)?.as_str() {
                        "checking" => AccountType::Checking,
                        "savings" => AccountType::Savings,
                        "credit_card" => AccountType::CreditCard,
                        "loan" => AccountType::Loan,
                        _ => AccountType::Checking,
                    },
                    current_balance: row.get(4)?,
                    interest_rate: row.get(5)?,
                    minimum_payment: row.get(6)?,
                    last_updated: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(accounts)
    }

    pub fn insert_debt_account(&self, account: &DebtAccount) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let account_type_str = match account.account_type {
            AccountType::Checking => "checking",
            AccountType::Savings => "savings",
            AccountType::CreditCard => "credit_card",
            AccountType::Loan => "loan",
        };

        conn.execute(
            "INSERT OR REPLACE INTO debt_accounts (id, simplefin_account_id, name, account_type, current_balance, interest_rate, minimum_payment, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &account.id,
                &account.simplefin_account_id,
                &account.name,
                account_type_str,
                account.current_balance,
                account.interest_rate,
                account.minimum_payment,
                account.last_updated.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn insert_transaction(&self, tx: &RawTransaction) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO raw_transactions (id, account_id, posted_date, amount, merchant, description, transaction_type, imported_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &tx.id,
                &tx.account_id,
                tx.posted_date.to_rfc3339(),
                tx.amount,
                &tx.merchant,
                &tx.description,
                &tx.transaction_type,
                tx.imported_at.to_rfc3339(),
                "simplefin",
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn categorize_transaction(&self, id: &str, category: &CategorizedTransaction) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO categorized_transactions (id, category, secondary_category, confidence, note, categorized_at, is_manual)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                &category.category,
                &category.secondary_category,
                category.confidence,
                &category.note,
                category.categorized_at.to_rfc3339(),
                category.is_manual as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn categorize_transaction_with_params(
        &self,
        id: &str,
        category: &str,
        secondary_category: Option<&str>,
        confidence: f64,
        note: Option<&str>,
        is_manual: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO categorized_transactions (id, category, secondary_category, confidence, note, categorized_at, is_manual)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                category,
                secondary_category,
                confidence,
                note,
                chrono::Utc::now().to_rfc3339(),
                is_manual as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_transactions(&self, account_id: Option<&str>, limit: i32, offset: i32) -> Result<Vec<RawTransaction>> {
        let conn = self.conn.lock().unwrap();
        let query = if account_id.is_some() {
            "SELECT id, account_id, posted_date, amount, merchant, description, transaction_type, imported_at FROM raw_transactions WHERE account_id = ?1 ORDER BY posted_date DESC LIMIT ?2 OFFSET ?3"
        } else {
            "SELECT id, account_id, posted_date, amount, merchant, description, transaction_type, imported_at FROM raw_transactions ORDER BY posted_date DESC LIMIT ?2 OFFSET ?3"
        };

        let mut stmt = conn.prepare(query).map_err(|e| AppError::Database(e.to_string()))?;

        let transactions = if let Some(acc_id) = account_id {
            stmt
                .query_map(params![acc_id, limit, offset], |row| {
                    Ok(RawTransaction {
                        id: row.get(0)?,
                        account_id: row.get(1)?,
                        posted_date: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now),
                        amount: row.get(3)?,
                        merchant: row.get(4)?,
                        description: row.get(5)?,
                        transaction_type: row.get(6)?,
                        imported_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now),
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        } else {
            stmt
                .query_map(params![limit, offset], |row| {
                    Ok(RawTransaction {
                        id: row.get(0)?,
                        account_id: row.get(1)?,
                        posted_date: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now),
                        amount: row.get(3)?,
                        merchant: row.get(4)?,
                        description: row.get(5)?,
                        transaction_type: row.get(6)?,
                        imported_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now),
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        };

        Ok(transactions)
    }

    pub fn get_metrics(&self, start_date: &str, end_date: &str) -> Result<(f64, f64, f64, f64)> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT
            COALESCE((
                SELECT SUM(rt.amount)
                FROM raw_transactions rt
                JOIN accounts a ON a.id = rt.account_id
                WHERE rt.amount > 0
                  AND a.account_type IN ('checking', 'savings')
                  AND date(rt.posted_date) BETWEEN date(?) AND date(?)
            ), 0.0) AS income,

            COALESCE((
                SELECT SUM(ABS(rt.amount))
                FROM raw_transactions rt
                JOIN accounts a ON a.id = rt.account_id
                LEFT JOIN categorized_transactions ct ON ct.id = rt.id
                WHERE rt.amount < 0
                  AND a.account_type IN ('checking', 'savings')
                  AND date(rt.posted_date) BETWEEN date(?) AND date(?)
                  AND COALESCE(ct.category, '') NOT IN ('Transfers', 'Interest', 'Debt Payments')
            ), 0.0) AS spending,

            COALESCE((
                SELECT SUM(rt.amount)
                FROM raw_transactions rt
                JOIN accounts a ON a.id = rt.account_id
                WHERE rt.amount > 0
                  AND a.account_type IN ('credit_card', 'loan')
                  AND date(rt.posted_date) BETWEEN date(?) AND date(?)
            ), 0.0) AS debt_paydown,

            COALESCE((
                SELECT SUM(ABS(rt.amount))
                FROM raw_transactions rt
                JOIN categorized_transactions ct ON ct.id = rt.id
                WHERE ct.category = 'Interest'
                  AND date(rt.posted_date) BETWEEN date(?) AND date(?)
            ), 0.0) AS interest_paid";

        let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
        let (income, spending, debt_paydown, interest_paid) = stmt.query_row(
            params![start_date, end_date, start_date, end_date, start_date, end_date, start_date, end_date],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).map_err(|e| AppError::Database(e.to_string()))?;

        Ok((income, spending, debt_paydown, interest_paid))
    }

    pub fn get_debt_ratio(&self) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT
            COALESCE((SELECT SUM(current_balance) FROM debt_accounts), 0.0) AS total_debt,
            COALESCE((SELECT SUM(balance) FROM accounts), 0.0) AS total_assets";

        let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
        let (total_debt, total_assets): (f64, f64) = stmt.query_row(
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| AppError::Database(e.to_string()))?;

        Ok(if total_assets > 0.0 {
            total_debt / total_assets
        } else {
            0.0
        })
    }

    pub fn get_sparkline(&self, end_date: &str) -> Result<Vec<crate::models::DailyMetrics>> {
        let conn = self.conn.lock().unwrap();
        let sql = "WITH RECURSIVE days(day) AS (
            SELECT date(?, '-27 days')
            UNION ALL
            SELECT date(day, '+1 day') FROM days WHERE day < date(?)
        ),
        income_daily AS (
            SELECT date(rt.posted_date) AS day, SUM(rt.amount) AS total
            FROM raw_transactions rt
            JOIN accounts a ON a.id = rt.account_id
            WHERE rt.amount > 0 AND a.account_type IN ('checking', 'savings')
            GROUP BY date(rt.posted_date)
        ),
        spending_daily AS (
            SELECT date(rt.posted_date) AS day, SUM(ABS(rt.amount)) AS total
            FROM raw_transactions rt
            JOIN accounts a ON a.id = rt.account_id
            LEFT JOIN categorized_transactions ct ON ct.id = rt.id
            WHERE rt.amount < 0
              AND a.account_type IN ('checking', 'savings')
              AND COALESCE(ct.category, '') NOT IN ('Transfers', 'Interest', 'Debt Payments')
            GROUP BY date(rt.posted_date)
        ),
        debt_paydown_daily AS (
            SELECT date(rt.posted_date) AS day, SUM(rt.amount) AS total
            FROM raw_transactions rt
            JOIN accounts a ON a.id = rt.account_id
            WHERE rt.amount > 0 AND a.account_type IN ('credit_card', 'loan')
            GROUP BY date(rt.posted_date)
        ),
        interest_daily AS (
            SELECT date(rt.posted_date) AS day, SUM(ABS(rt.amount)) AS total
            FROM raw_transactions rt
            JOIN categorized_transactions ct ON ct.id = rt.id
            WHERE ct.category = 'Interest'
            GROUP BY date(rt.posted_date)
        )
        SELECT
            d.day AS date,
            COALESCE(i.total, 0.0) AS income,
            COALESCE(s.total, 0.0) AS spending,
            COALESCE(dp.total, 0.0) AS debt_paydown,
            COALESCE(it.total, 0.0) AS interest_paid
        FROM days d
        LEFT JOIN income_daily i ON i.day = d.day
        LEFT JOIN spending_daily s ON s.day = d.day
        LEFT JOIN debt_paydown_daily dp ON dp.day = d.day
        LEFT JOIN interest_daily it ON it.day = d.day
        ORDER BY d.day ASC";

        let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
        let sparkline = stmt.query_map(
            params![end_date, end_date],
            |row| {
                Ok(crate::models::DailyMetrics {
                    date: row.get(0)?,
                    income: row.get(1)?,
                    spending: row.get(2)?,
                    debt_paydown: row.get(3)?,
                    interest_paid: row.get(4)?,
                })
            },
        ).map_err(|e| AppError::Database(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sparkline)
    }

    pub fn get_last_sync(&self) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT sync_date FROM sync_log WHERE status = 'success' ORDER BY sync_date DESC LIMIT 1";

        let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
        let result = stmt.query_row([], |row| {
            let date_str: String = row.get(0)?;
            Ok(chrono::DateTime::parse_from_rfc3339(&date_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()))
        }).optional().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    pub fn get_last_sync_info(&self) -> Result<Option<(DateTime<Utc>, Option<String>, i32)>> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT sync_date, error_message, transaction_count FROM sync_log ORDER BY sync_date DESC LIMIT 1";

        let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
        let result = stmt.query_row([], |row| {
            let date_str: String = row.get(0)?;
            let error_msg: Option<String> = row.get(1)?;
            let txn_count: i32 = row.get(2)?;
            let dt = chrono::DateTime::parse_from_rfc3339(&date_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok((dt, error_msg, txn_count))
        }).optional().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    pub fn insert_sync_log(&self, status: &str, transaction_count: i32, error_message: Option<&str>, duration_ms: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_log (sync_date, status, transaction_count, error_message, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Utc::now().to_rfc3339(),
                status,
                transaction_count,
                error_message,
                duration_ms,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, Utc::now().to_rfc3339()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result: Option<String> = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).optional().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let settings = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<std::result::Result<std::collections::HashMap<_, _>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(settings)
    }

    /// Get unmapped transactions (those without a mapping entry)
    pub fn get_unmapped_transactions(&self) -> Result<Vec<RawTransaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, posted_date, amount, merchant, description,
                        transaction_type, imported_at
                 FROM raw_transactions
                 WHERE id NOT IN (SELECT transaction_id FROM transaction_mappings)
                 ORDER BY posted_date DESC"
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let transactions = stmt
            .query_map([], |row| {
                Ok(RawTransaction {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    posted_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    amount: row.get(3)?,
                    merchant: row.get(4)?,
                    description: row.get(5)?,
                    transaction_type: row.get(6)?,
                    imported_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(transactions)
    }

    /// Record a transaction mapping decision
    pub fn record_transaction_mapping(
        &self,
        transaction_id: &str,
        original_account_id: &str,
        mapped_account_id: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO transaction_mappings
             (transaction_id, original_account_id, mapped_account_id, mapped_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![transaction_id, original_account_id, mapped_account_id, now],
        ).map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Bulk update transaction account assignments and record mappings
    pub fn bulk_update_transaction_accounts(
        &self,
        mappings: &[(String, String)], // (transaction_id, new_account_id)
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for (transaction_id, new_account_id) in mappings {
            // Get the current account_id
            let current_account_id: String = conn
                .query_row(
                    "SELECT account_id FROM raw_transactions WHERE id = ?1",
                    params![transaction_id],
                    |row| row.get(0),
                )
                .map_err(|e| AppError::Database(e.to_string()))?;

            // Record the mapping
            conn.execute(
                "INSERT OR REPLACE INTO transaction_mappings
                 (transaction_id, original_account_id, mapped_account_id, mapped_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![transaction_id, current_account_id, new_account_id, now],
            ).map_err(|e| AppError::Database(e.to_string()))?;

            // Update the transaction account
            conn.execute(
                "UPDATE raw_transactions SET account_id = ?1 WHERE id = ?2",
                params![new_account_id, transaction_id],
            ).map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }
}
