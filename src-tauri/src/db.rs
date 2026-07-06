use rusqlite::{Connection, Result as SqliteResult};
use std::sync::Mutex;
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
        // TODO: implement
        Ok(Vec::new())
    }

    pub fn insert_account(&self, account: &Account) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    pub fn get_debt_accounts(&self) -> Result<Vec<DebtAccount>> {
        // TODO: implement
        Ok(Vec::new())
    }

    pub fn insert_debt_account(&self, account: &DebtAccount) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    pub fn insert_transaction(&self, tx: &RawTransaction) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    pub fn categorize_transaction(&self, id: &str, category: &CategorizedTransaction) -> Result<()> {
        // TODO: implement
        Ok(())
    }
}
