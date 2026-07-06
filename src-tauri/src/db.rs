use rusqlite::{Connection, params, OptionalExtension};
use std::sync::Mutex;
use chrono::Utc;
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
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc)),
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
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc)),
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
}
