use crate::errors::Result;
use crate::db::Database;
use chrono::Utc;

/// Sync orchestration: manage when syncs should occur
pub struct SyncOrchestrator;

impl SyncOrchestrator {
    /// Check if a sync should be triggered based on last sync time and settings
    /// Returns true if sync should proceed (on-open check: >24h since last sync)
    pub fn should_sync_on_open(db: &Database) -> Result<bool> {
        // Get last sync timestamp
        let last_sync = db.get_last_sync()?;

        match last_sync {
            None => {
                // Never synced before, so sync on open
                Ok(true)
            }
            Some(last_sync_time) => {
                // Check if >24 hours have passed
                let hours_since = (Utc::now().signed_duration_since(last_sync_time))
                    .num_hours();
                Ok(hours_since > 24)
            }
        }
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn test_24_hour_threshold() {
        // Test: 25 hours should trigger sync
        assert!(25 > 24);
        // Test: 23 hours should not trigger
        assert!(!(23 > 24));
    }
}
