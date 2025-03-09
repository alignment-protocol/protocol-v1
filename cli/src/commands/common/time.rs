use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current Unix timestamp
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
