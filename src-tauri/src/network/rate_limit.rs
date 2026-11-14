use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<i64>>>>, // Guest IP/ID -> timestamps of recent requests
    max_requests_per_window: usize,
    window_secs: i64,
}

impl RateLimiter {
    pub fn new(max_requests_per_window: usize, window_secs: i64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_window,
            window_secs,
        }
    }

    pub fn check_and_record(&self, guest_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - self.window_secs;

        let mut map = self.state.lock().unwrap();
        let timestamps = map.entry(guest_id.to_string()).or_default();

        // Prune older requests
        timestamps.retain(|&t| t > cutoff);

        if timestamps.len() >= self.max_requests_per_window {
            false // Rate limit exceeded
        } else {
            timestamps.push(now);
            true
        }
    }
}

