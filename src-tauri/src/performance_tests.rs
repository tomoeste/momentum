#[cfg(test)]
mod integration_tests {
    use crate::db::Database;
    use crate::models::*;
    use std::time::Instant;
    use chrono::{Utc, Duration};

    #[test]
    fn test_large_transaction_dataset_performance() {
        // Create in-memory database
        let db = Database::new(":memory:").expect("Failed to create test database");

        // Create test account
        let now = Utc::now();
        let test_account = Account {
            id: "test_checking".to_string(),
            simplefin_account_id: Some("simplefin_123".to_string()),
            name: "Test Checking".to_string(),
            account_type: AccountType::Checking,
            organization: Some("Test Bank".to_string()),
            balance: 10000.0,
            last_updated: now,
        };

        db.insert_account(&test_account).expect("Failed to insert test account");

        // Insert 1000 transactions
        let start_insert = Instant::now();
        let base_time = Utc::now();
        for i in 0..1000 {
            let txn = RawTransaction {
                id: format!("txn_{}", i),
                account_id: "test_checking".to_string(),
                posted_date: base_time.checked_sub_signed(Duration::days(i as i64)).unwrap(),
                amount: (i as f64 % 100.0) + 0.99,
                merchant: Some(format!("Merchant {}", i)),
                description: format!("Test transaction {}", i),
                transaction_type: if i % 2 == 0 { "debit" } else { "credit" }.to_string(),
                imported_at: Utc::now(),
            };
            db.insert_transaction(&txn)
                .expect(&format!("Failed to insert transaction {}", i));
        }
        let insert_duration = start_insert.elapsed();

        // Query all transactions with pagination
        let start_query = Instant::now();
        let result = db.get_transactions(
            None,
            100,
            0,
        ).expect("Failed to query transactions");
        let query_duration = start_query.elapsed();

        // Query with account filter
        let start_filter = Instant::now();
        let filtered = db.get_transactions(
            Some("test_checking"),
            100,
            0,
        ).expect("Failed to query with filter");
        let filter_duration = start_filter.elapsed();

        // Calculate metrics for the dataset
        let start_metrics = Instant::now();
        let _metrics = db.get_metrics(
            &base_time.checked_sub_signed(Duration::days(30)).unwrap().to_rfc3339(),
            &base_time.to_rfc3339(),
        ).expect("Failed to calculate metrics");
        let metrics_duration = start_metrics.elapsed();

        println!("\n=== Large Dataset Performance Test Results ===");
        println!("Inserted 1000 transactions in: {:.2}ms", insert_duration.as_secs_f64() * 1000.0);
        println!("Query 100 transactions in: {:.2}ms", query_duration.as_secs_f64() * 1000.0);
        println!("Filtered query in: {:.2}ms", filter_duration.as_secs_f64() * 1000.0);
        println!("Calculated metrics in: {:.2}ms", metrics_duration.as_secs_f64() * 1000.0);

        // Assertions - should complete in reasonable time
        assert!(insert_duration.as_secs_f64() < 5.0, "Inserting 1000 transactions took too long: {:.2}s", insert_duration.as_secs_f64());
        assert!(query_duration.as_millis() < 100, "Querying transactions took too long: {:.0}ms", query_duration.as_millis());
        assert!(filter_duration.as_millis() < 100, "Filtered query took too long: {:.0}ms", filter_duration.as_millis());
        assert!(metrics_duration.as_millis() < 500, "Metrics calculation took too long: {:.0}ms", metrics_duration.as_millis());
        assert_eq!(result.len(), 100, "Expected 100 transactions returned");
        assert_eq!(filtered.len(), 100, "Expected 100 filtered transactions");
    }

    #[test]
    fn test_transaction_pagination_with_large_dataset() {
        let db = Database::new(":memory:").expect("Failed to create test database");

        let now = Utc::now();
        let test_account = Account {
            id: "test_savings".to_string(),
            simplefin_account_id: Some("simplefin_456".to_string()),
            name: "Test Savings".to_string(),
            account_type: AccountType::Savings,
            organization: Some("Test Bank".to_string()),
            balance: 50000.0,
            last_updated: now,
        };

        db.insert_account(&test_account).expect("Failed to insert account");

        // Insert 500 transactions
        let base_time = Utc::now();
        for i in 0..500 {
            let txn = RawTransaction {
                id: format!("savings_txn_{}", i),
                account_id: "test_savings".to_string(),
                posted_date: base_time.checked_sub_signed(Duration::days(i as i64)).unwrap(),
                amount: (i as f64 % 200.0) + 50.0,
                merchant: Some(format!("Bank {}", i)),
                description: format!("Savings transaction {}", i),
                transaction_type: "credit".to_string(),
                imported_at: Utc::now(),
            };
            db.insert_transaction(&txn)
                .expect(&format!("Failed to insert transaction {}", i));
        }

        // Test pagination: 25 items per page (standard UI page size)
        let start_pagination = Instant::now();
        let page1 = db.get_transactions(
            Some("test_savings"),
            25,
            0,
        ).expect("Failed to query page 1");

        let page2 = db.get_transactions(
            Some("test_savings"),
            25,
            25,
        ).expect("Failed to query page 2");

        let page20 = db.get_transactions(
            Some("test_savings"),
            25,
            475,
        ).expect("Failed to query page 20");

        let pagination_duration = start_pagination.elapsed();

        println!("\n=== Pagination Performance Test Results ===");
        println!("Paginated 3 queries from 500 transactions in: {:.2}ms", pagination_duration.as_secs_f64() * 1000.0);
        println!("Page 1 items: {}", page1.len());
        println!("Page 2 items: {}", page2.len());
        println!("Page 20 items: {}", page20.len());

        // Assertions
        assert_eq!(page1.len(), 25, "Expected 25 items on page 1");
        assert_eq!(page2.len(), 25, "Expected 25 items on page 2");
        assert_eq!(page20.len(), 25, "Expected 25 items on page 20");
        assert!(pagination_duration.as_millis() < 200, "Pagination queries took too long: {:.0}ms", pagination_duration.as_millis());
    }

    #[test]
    fn test_large_categorized_transaction_dataset() {
        let db = Database::new(":memory:").expect("Failed to create test database");

        let now = Utc::now();
        let test_account = Account {
            id: "test_credit".to_string(),
            simplefin_account_id: Some("simplefin_789".to_string()),
            name: "Test Credit Card".to_string(),
            account_type: AccountType::CreditCard,
            organization: Some("Test Bank".to_string()),
            balance: -5000.0,
            last_updated: now,
        };

        db.insert_account(&test_account).expect("Failed to insert account");

        // Insert raw transactions
        let base_time = Utc::now();
        for i in 0..500 {
            let txn = RawTransaction {
                id: format!("cc_txn_{}", i),
                account_id: "test_credit".to_string(),
                posted_date: base_time.checked_sub_signed(Duration::days(i as i64)).unwrap(),
                amount: -((i as f64 % 150.0) + 10.0),
                merchant: Some(format!("Vendor {}", i)),
                description: format!("Credit card purchase {}", i),
                transaction_type: "debit".to_string(),
                imported_at: Utc::now(),
            };
            db.insert_transaction(&txn)
                .expect(&format!("Failed to insert transaction {}", i));
        }

        // Categorize transactions
        let start_categorize = Instant::now();
        for i in 0..500 {
            let category = match i % 5 {
                0 => "Groceries",
                1 => "Utilities",
                2 => "Entertainment",
                3 => "Transportation",
                _ => "Shopping",
            };

            let cat_txn = CategorizedTransaction {
                id: format!("cc_txn_{}", i),
                category: category.to_string(),
                secondary_category: None,
                confidence: 0.95,
                note: None,
                categorized_at: Utc::now(),
                is_manual: false,
            };
            let _ = db.categorize_transaction(&format!("cc_txn_{}", i), &cat_txn);
        }
        let categorize_duration = start_categorize.elapsed();

        // Query all transactions on the account
        let start_query = Instant::now();
        let all_txns = db.get_transactions(
            Some("test_credit"),
            100,
            0,
        ).expect("Failed to query credit card transactions");
        let query_duration = start_query.elapsed();

        println!("\n=== Categorized Transaction Performance Test Results ===");
        println!("Categorized 500 transactions in: {:.2}ms", categorize_duration.as_secs_f64() * 1000.0);
        println!("Query transactions in: {:.2}ms", query_duration.as_secs_f64() * 1000.0);
        println!("Transaction results: {}", all_txns.len());

        // Assertions
        assert!(categorize_duration.as_secs_f64() < 2.0, "Categorization took too long");
        assert!(query_duration.as_millis() < 100, "Query took too long");
        assert_eq!(all_txns.len(), 100, "Expected 100 transactions from pagination");
    }
}
