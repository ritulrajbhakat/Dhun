use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckTelemetry {
    pub deck_id: usize,
    pub track_id: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub bpm: f64,
    pub tempo_rate: f64,
    pub is_playing: bool,
    pub vu_l_db: f32,
    pub vu_r_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterTelemetry {
    pub vu_master_l_db: f32,
    pub vu_master_r_db: f32,
    pub crossfader_pos: f32,
    pub underrun_count: u64,
    pub cpu_load_pct: f32,
    pub timestamp_ms: i64,
    pub decks: Vec<DeckTelemetry>,
}

impl MasterTelemetry {
    pub fn empty() -> Self {
        Self {
            vu_master_l_db: -96.0,
            vu_master_r_db: -96.0,
            crossfader_pos: 0.0,
            underrun_count: 0,
            cpu_load_pct: 0.0,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            decks: Vec::new(),
        }
    }
}

