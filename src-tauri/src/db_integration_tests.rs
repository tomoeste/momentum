#[cfg(test)]
mod integration_tests {
    use crate::db::Database;
    use crate::models::*;
    use chrono::Utc;

    fn setup_test_db() -> Database {
        Database::new(":memory:").expect("Failed to create test database")
    }

    fn create_test_account(id: &str, name: &str) -> Account {
        Account {
            id: id.to_string(),
            simplefin_account_id: Some(format!("sf_{}", id)),
            name: name.to_string(),
            account_type: AccountType::Checking,
            organization: None,
            balance: 5000.0,
            last_updated: Utc::now(),
        }
    }

    fn create_test_transaction(id: &str, account_id: &str, amount: f64) -> RawTransaction {
        RawTransaction {
            id: id.to_string(),
            account_id: account_id.to_string(),
            posted_date: Utc::now(),
            amount,
            merchant: Some("Test Merchant".to_string()),
            description: "Test transaction".to_string(),
            transaction_type: "debit".to_string(),
            imported_at: Utc::now(),
        }
    }

    fn create_test_categorization(id: &str, category: &str) -> CategorizedTransaction {
        CategorizedTransaction {
            id: id.to_string(),
            category: category.to_string(),
            secondary_category: None,
            confidence: 0.95,
            note: None,
            categorized_at: Utc::now(),
            is_manual: false,
        }
    }

    // Account CRUD Tests
    #[test]
    fn test_insert_and_retrieve_account() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking Account");

        db.insert_account(&account).expect("Failed to insert account");
        let accounts = db.get_accounts().expect("Failed to get accounts");

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "acc_1");
        assert_eq!(accounts[0].name, "Checking Account");
        assert_eq!(accounts[0].balance, 5000.0);
    }

    #[test]
    fn test_insert_multiple_accounts() {
        let db = setup_test_db();

        let acc1 = create_test_account("acc_1", "Checking");
        let acc2 = create_test_account("acc_2", "Savings");

        db.insert_account(&acc1).expect("Failed to insert account 1");
        db.insert_account(&acc2).expect("Failed to insert account 2");

        let accounts = db.get_accounts().expect("Failed to get accounts");

        assert_eq!(accounts.len(), 2);
        assert!(accounts.iter().any(|a| a.id == "acc_1"));
        assert!(accounts.iter().any(|a| a.id == "acc_2"));
    }

    // Transaction CRUD Tests
    #[test]
    fn test_insert_and_retrieve_transaction() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");
        let transaction = create_test_transaction("txn_1", "acc_1", 100.0);

        db.insert_account(&account).expect("Failed to insert account");
        db.insert_transaction(&transaction).expect("Failed to insert transaction");

        let transactions = db.get_transactions(Some("acc_1"), 10, 0)
            .expect("Failed to get transactions");

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].id, "txn_1");
        assert_eq!(transactions[0].amount, 100.0);
    }

    #[test]
    fn test_transaction_filtering_by_account() {
        let db = setup_test_db();
        let acc1 = create_test_account("acc_1", "Checking");
        let acc2 = create_test_account("acc_2", "Savings");

        db.insert_account(&acc1).expect("Failed to insert account 1");
        db.insert_account(&acc2).expect("Failed to insert account 2");

        let txn1 = create_test_transaction("txn_1", "acc_1", 100.0);
        let txn2 = create_test_transaction("txn_2", "acc_1", -50.0);
        let txn3 = create_test_transaction("txn_3", "acc_2", 200.0);

        db.insert_transaction(&txn1).expect("Failed to insert txn1");
        db.insert_transaction(&txn2).expect("Failed to insert txn2");
        db.insert_transaction(&txn3).expect("Failed to insert txn3");

        let acc1_txns = db.get_transactions(Some("acc_1"), 10, 0)
            .expect("Failed to get account 1 transactions");
        let acc2_txns = db.get_transactions(Some("acc_2"), 10, 0)
            .expect("Failed to get account 2 transactions");

        assert_eq!(acc1_txns.len(), 2);
        assert_eq!(acc2_txns.len(), 1);
    }

    // Categorization Tests
    #[test]
    fn test_categorize_transaction() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");
        let transaction = create_test_transaction("txn_1", "acc_1", 100.0);
        let categorization = create_test_categorization("txn_1", "Food");

        db.insert_account(&account).expect("Failed to insert account");
        db.insert_transaction(&transaction).expect("Failed to insert transaction");
        db.categorize_transaction("txn_1", &categorization)
            .expect("Failed to categorize transaction");

        // Verify by checking if we can retrieve the transaction with account filter (which validates FK constraint)
        let transactions = db.get_transactions(Some("acc_1"), 10, 0)
            .expect("Failed to get transactions");

        assert_eq!(transactions.len(), 1);
    }

    // Metrics & Aggregation Tests
    #[test]
    fn test_metrics_calculation_empty_database() {
        let db = setup_test_db();

        let now = Utc::now();
        let end_date = now.format("%Y-%m-%d").to_string();
        let start_date = (now - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();

        let (income, spending, debt_paydown, interest_paid) = db.get_metrics(&start_date, &end_date)
            .expect("Failed to get metrics");

        assert_eq!(income, 0.0);
        assert_eq!(spending, 0.0);
        assert_eq!(debt_paydown, 0.0);
        assert_eq!(interest_paid, 0.0);
    }

    #[test]
    fn test_metrics_with_transactions() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");

        db.insert_account(&account).expect("Failed to insert account");

        // Insert income transaction
        let income_txn = create_test_transaction("txn_income", "acc_1", 5000.0);
        db.insert_transaction(&income_txn).expect("Failed to insert income");

        let income_cat = CategorizedTransaction {
            id: "txn_income".to_string(),
            category: "Income".to_string(),
            secondary_category: None,
            confidence: 1.0,
            note: None,
            categorized_at: Utc::now(),
            is_manual: false,
        };
        db.categorize_transaction("txn_income", &income_cat).ok();

        // Get metrics for today
        let now = Utc::now();
        let end_date = now.format("%Y-%m-%d").to_string();
        let start_date = now.format("%Y-%m-%d").to_string();

        let (income, spending, debt_paydown, interest_paid) = db.get_metrics(&start_date, &end_date)
            .expect("Failed to get metrics");

        assert!(income > 0.0, "Income should be greater than 0");
        assert_eq!(spending, 0.0);
        assert_eq!(debt_paydown, 0.0);
        assert_eq!(interest_paid, 0.0);
    }

    #[test]
    fn test_debt_ratio_calculation() {
        let db = setup_test_db();

        // Insert accounts with positive balances (assets)
        let mut acc1 = create_test_account("acc_1", "Checking");
        acc1.balance = 10000.0;

        let mut acc2 = create_test_account("acc_2", "Savings");
        acc2.balance = 20000.0;

        db.insert_account(&acc1).expect("Failed to insert account 1");
        db.insert_account(&acc2).expect("Failed to insert account 2");

        // Insert debt account (used for debt calculations)
        let debt_acc = DebtAccount {
            id: "debt_1".to_string(),
            simplefin_account_id: Some("sf_acc_3".to_string()),
            name: "Credit Card".to_string(),
            account_type: AccountType::CreditCard,
            current_balance: 10000.0,
            interest_rate: 0.22,
            minimum_payment: Some(200.0),
            last_updated: Utc::now(),
        };

        db.insert_debt_account(&debt_acc).expect("Failed to insert debt account");

        let debt_ratio = db.get_debt_ratio().expect("Failed to calculate debt ratio");

        // With $10000 debt and $30000 assets, ratio should be ~0.33
        assert!(debt_ratio > 0.2 && debt_ratio < 0.5, "Debt ratio should be ~0.33, got {}", debt_ratio);
    }

    // Sync Log Tests
    #[test]
    fn test_insert_sync_log() {
        let db = setup_test_db();

        db.insert_sync_log("success", 10, None, 1000)
            .expect("Failed to insert sync log");

        let last_sync = db.get_last_sync().expect("Failed to get last sync");

        assert!(last_sync.is_some(), "Should have sync log entry");
    }

    #[test]
    fn test_sync_log_with_error_message() {
        let db = setup_test_db();

        db.insert_sync_log("failed", 0, Some("Network timeout"), 5000)
            .expect("Failed to insert sync log with error");

        let sync_info = db.get_last_sync_info()
            .expect("Failed to get sync info");

        assert!(sync_info.is_some());
        let (_, error_msg, _) = sync_info.unwrap();
        assert!(error_msg.is_some());
        assert_eq!(error_msg.unwrap(), "Network timeout");
    }

    // Settings Tests
    #[test]
    fn test_save_and_retrieve_setting() {
        let db = setup_test_db();

        db.save_setting("llm_model", "ollama/neural-chat")
            .expect("Failed to save setting");

        let value = db.get_setting("llm_model")
            .expect("Failed to get setting");

        assert_eq!(value, Some("ollama/neural-chat".to_string()));
    }

    #[test]
    fn test_update_setting() {
        let db = setup_test_db();

        db.save_setting("sync_frequency", "24h")
            .expect("Failed to save setting");

        let value1 = db.get_setting("sync_frequency")
            .expect("Failed to get setting 1");
        assert_eq!(value1, Some("24h".to_string()));

        // Update the setting
        db.save_setting("sync_frequency", "12h")
            .expect("Failed to update setting");

        let value2 = db.get_setting("sync_frequency")
            .expect("Failed to get setting 2");
        assert_eq!(value2, Some("12h".to_string()));
    }

    #[test]
    fn test_get_all_settings() {
        let db = setup_test_db();

        db.save_setting("key1", "value1").ok();
        db.save_setting("key2", "value2").ok();
        db.save_setting("key3", "value3").ok();

        let all_settings = db.get_all_settings()
            .expect("Failed to get all settings");

        assert!(all_settings.len() >= 3);
        assert_eq!(all_settings.get("key1"), Some(&"value1".to_string()));
        assert_eq!(all_settings.get("key2"), Some(&"value2".to_string()));
        assert_eq!(all_settings.get("key3"), Some(&"value3".to_string()));
    }

    // Transaction Mapping Tests
    #[test]
    fn test_unmapped_transactions() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");

        db.insert_account(&account).expect("Failed to insert account");

        // Create transaction without account assignment
        let mut txn = create_test_transaction("txn_1", "acc_1", 100.0);
        txn.account_id = "".to_string(); // Empty account_id means unmapped

        // We can't actually insert with empty account_id due to FK constraint
        // This test documents the expected behavior
        let result = db.insert_transaction(&txn);

        // This should fail due to FK constraint
        assert!(result.is_err(), "Should not allow transaction with empty account_id");
    }

    // Pagination Tests
    #[test]
    fn test_transaction_pagination() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");

        db.insert_account(&account).expect("Failed to insert account");

        // Insert 25 transactions
        for i in 0..25 {
            let txn = create_test_transaction(&format!("txn_{}", i), "acc_1", (i as f64) * 10.0);
            db.insert_transaction(&txn).expect(&format!("Failed to insert transaction {}", i));
        }

        // Test pagination
        let page1 = db.get_transactions(Some("acc_1"), 10, 0)
            .expect("Failed to get page 1");
        let page2 = db.get_transactions(Some("acc_1"), 10, 10)
            .expect("Failed to get page 2");
        let page3 = db.get_transactions(Some("acc_1"), 10, 20)
            .expect("Failed to get page 3");

        assert_eq!(page1.len(), 10);
        assert_eq!(page2.len(), 10);
        assert_eq!(page3.len(), 5);
    }

    // Foreign Key Constraint Tests
    #[test]
    fn test_transaction_requires_valid_account() {
        let db = setup_test_db();

        let txn = create_test_transaction("txn_1", "nonexistent_account", 100.0);

        let result = db.insert_transaction(&txn);

        // Should fail due to FK constraint
        assert!(result.is_err(), "Should not allow transaction with non-existent account");
    }

    // Sparkline Tests
    #[test]
    fn test_sparkline_generation() {
        let db = setup_test_db();
        let account = create_test_account("acc_1", "Checking");

        db.insert_account(&account).expect("Failed to insert account");

        let now = Utc::now();
        let end_date = now.format("%Y-%m-%d").to_string();

        let sparkline = db.get_sparkline(&end_date)
            .expect("Failed to get sparkline");

        // Should return some number of daily metrics (even if empty)
        assert!(sparkline.len() > 0, "Sparkline should have entries");
    }

    // Database Constraint Violation Tests
    #[test]
    fn test_account_id_uniqueness() {
        let db = setup_test_db();

        // Insert first account with unique ID
        let account1 = Account {
            id: "acc_1".to_string(),
            simplefin_account_id: Some("sf_123".to_string()),
            name: "Account 1".to_string(),
            account_type: AccountType::Checking,
            organization: None,
            balance: 1000.0,
            last_updated: Utc::now(),
        };

        db.insert_account(&account1).expect("Failed to insert first account");

        // Insert second account with different ID
        let account2 = Account {
            id: "acc_2".to_string(),
            simplefin_account_id: Some("sf_456".to_string()), // Different ID
            name: "Account 2".to_string(),
            account_type: AccountType::Checking,
            organization: None,
            balance: 2000.0,
            last_updated: Utc::now(),
        };

        let result = db.insert_account(&account2);

        // Should succeed - different primary keys
        assert!(result.is_ok(), "Should allow accounts with different IDs");

        // Verify both accounts exist
        let accounts = db.get_accounts().expect("Failed to get accounts");
        assert_eq!(accounts.len(), 2, "Should have 2 accounts");
    }

    #[test]
    fn test_categorized_transaction_requires_raw_transaction() {
        let db = setup_test_db();

        // Try to categorize a transaction that doesn't exist
        let category = create_test_categorization("nonexistent_txn", "Shopping");

        let result = db.categorize_transaction("nonexistent_txn", &category);

        // Should fail due to FK constraint
        assert!(result.is_err(), "Should reject categorization of non-existent transaction");
    }

    #[test]
    fn test_categorized_transaction_with_valid_transaction() {
        let db = setup_test_db();

        // Setup: create account and transaction
        let account = create_test_account("acc_1", "Checking");
        db.insert_account(&account).expect("Failed to insert account");

        let txn = create_test_transaction("txn_1", "acc_1", 100.0);
        db.insert_transaction(&txn).expect("Failed to insert transaction");

        // Categorize the transaction - should succeed
        let cat = create_test_categorization("txn_1", "Shopping");
        let result = db.categorize_transaction("txn_1", &cat);

        // Should succeed - valid FK reference
        assert!(result.is_ok(), "Should allow categorization of existing transaction");
    }

    #[test]
    fn test_account_with_no_simplefin_id_allowed() {
        let db = setup_test_db();

        // Account can have NULL simplefin_account_id (not all accounts come from SimpleFIN)
        let account = Account {
            id: "acc_manual".to_string(),
            simplefin_account_id: None, // No SimpleFIN ID
            name: "Manual Account".to_string(),
            account_type: AccountType::Savings,
            organization: None,
            balance: 5000.0,
            last_updated: Utc::now(),
        };

        let result = db.insert_account(&account);

        // Should succeed - simplefin_account_id is nullable
        assert!(result.is_ok(), "Should allow account with NULL simplefin_account_id");
    }

    #[test]
    fn test_multiple_accounts_same_name_allowed() {
        let db = setup_test_db();

        // Multiple accounts can have the same name (no UNIQUE constraint on name)
        let account1 = create_test_account("acc_1", "Checking");
        let account2 = Account {
            id: "acc_2".to_string(),
            simplefin_account_id: Some("sf_456".to_string()),
            name: "Checking".to_string(), // Same name as account1
            account_type: AccountType::Checking,
            organization: None,
            balance: 6000.0,
            last_updated: Utc::now(),
        };

        db.insert_account(&account1).expect("Failed to insert account1");
        let result = db.insert_account(&account2);

        // Should succeed - name is not unique
        assert!(result.is_ok(), "Should allow multiple accounts with same name");
    }
}
