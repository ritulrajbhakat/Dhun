use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySnapshot {
    pub session_id: String,
    pub party_name: String,
    pub dj_level: i32,
    pub primary_mood_id: String,
    pub active_deck: usize,
    pub current_track_id: Option<String>,
    pub current_position_ms: u64,
    pub snapshot_timestamp: i64,
}

pub struct RecoveryManager {
    last_audio_heartbeat: Arc<AtomicU64>,
    underrun_counter: Arc<AtomicU64>,
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            last_audio_heartbeat: Arc::new(AtomicU64::new(0)),
            underrun_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_heartbeat(&self) {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        self.last_audio_heartbeat.store(now, Ordering::Relaxed);
    }

    pub fn record_underrun(&self) {
        self.underrun_counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn is_audio_healthy(&self) -> bool {
        let last = self.last_audio_heartbeat.load(Ordering::Relaxed);
        if last == 0 {
            return true;
        }
        let now = chrono::Utc::now().timestamp_millis() as u64;
        (now - last) < 2000 // Healthy if heartbeat received in last 2 seconds
    }
}

