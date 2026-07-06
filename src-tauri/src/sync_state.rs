use std::sync::{Arc, Mutex};

/// Thread-safe sync state tracker for real-time progress display
#[derive(Clone)]
pub struct SyncState {
    in_progress: Arc<Mutex<bool>>,
}

impl SyncState {
    /// Create a new sync state tracker (initially not syncing)
    pub fn new() -> Self {
        SyncState {
            in_progress: Arc::new(Mutex::new(false)),
        }
    }

    /// Mark sync as in progress
    pub fn start_sync(&self) {
        if let Ok(mut state) = self.in_progress.lock() {
            *state = true;
        }
    }

    /// Mark sync as complete
    pub fn end_sync(&self) {
        if let Ok(mut state) = self.in_progress.lock() {
            *state = false;
        }
    }

    /// Check if sync is currently in progress
    pub fn is_in_progress(&self) -> bool {
        self.in_progress.lock().ok().map(|s| *s).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_lifecycle() {
        let state = SyncState::new();
        assert!(!state.is_in_progress());

        state.start_sync();
        assert!(state.is_in_progress());

        state.end_sync();
        assert!(!state.is_in_progress());
    }

    #[test]
    fn test_sync_state_is_cloneable() {
        let state1 = SyncState::new();
        let state2 = state1.clone();

        state1.start_sync();
        assert!(state2.is_in_progress());

        state2.end_sync();
        assert!(!state1.is_in_progress());
    }
}
