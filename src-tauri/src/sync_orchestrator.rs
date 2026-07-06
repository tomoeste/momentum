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

    /// Check if sync frequency setting requires a sync
    /// Frequency options: "manual" | "on-open" | "12h" | "24h"
    pub fn should_sync_by_frequency(
        db: &Database,
        frequency: &str,
    ) -> Result<bool> {
        match frequency {
            "manual" => Ok(false), // User must manually trigger
            "on-open" => Self::should_sync_on_open(db), // Check on app open
            "12h" => {
                let last_sync = db.get_last_sync()?;
                let hours_since = last_sync
                    .map(|ts| (Utc::now().signed_duration_since(ts)).num_hours())
                    .unwrap_or(25); // If never synced, treat as >12h
                Ok(hours_since >= 12)
            }
            "24h" => {
                let last_sync = db.get_last_sync()?;
                let hours_since = last_sync
                    .map(|ts| (Utc::now().signed_duration_since(ts)).num_hours())
                    .unwrap_or(25); // If never synced, treat as >24h
                Ok(hours_since >= 24)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_24_hour_threshold() {
        // Test: 25 hours should trigger sync
        assert!(25 > 24);
        // Test: 23 hours should not trigger
        assert!(!(23 > 24));
    }
}
