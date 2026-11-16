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

impl DeckTelemetry {
    pub fn empty(deck_id: usize) -> Self {
        Self {
            deck_id,
            track_id: None,
            position_ms: 0,
            duration_ms: 0,
            bpm: 128.0,
            tempo_rate: 1.0,
            is_playing: false,
            vu_l_db: -96.0,
            vu_r_db: -96.0,
        }
    }
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
            decks: vec![
                DeckTelemetry::empty(0),
                DeckTelemetry::empty(1),
                DeckTelemetry::empty(2),
                DeckTelemetry::empty(3),
            ],
        }
    }
}

