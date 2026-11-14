use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub year: Option<i32>,
    pub genre: String,
    pub subgenre: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAsset {
    pub id: String,
    pub track_id: String,
    pub file_path: String,
    pub file_format: String,
    pub file_size_bytes: i64,
    pub sample_rate: i32,
    pub bit_depth: Option<i32>,
    pub channels: i32,
    pub sha256_hash: String,
    pub verification_status: String,   // 'unverified', 'verified', 'corrupt', 'missing'
    pub availability_status: String,   // 'offline', 'available', 'locked_playback'
    pub last_verified_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub track_id: String,
    pub bpm: f64,
    pub bpm_confidence: f64,
    pub key_name: String,
    pub camelot_key: String,
    pub key_confidence: f64,
    pub time_signature_num: i32,
    pub time_signature_den: i32,
    pub downbeat_offset_ms: i64,
    pub phrase_length_bars: i32,
    pub overall_energy: f64,
    pub dance_suitability: f64,
    pub recommended_mix_in_ms: i64,
    pub recommended_mix_out_ms: i64,
    pub analysis_version: i32,
    pub analyzed_at: i64,
    pub manual_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatGridPoint {
    pub beat_number: i32,
    pub time_offset_ms: i64,
    pub is_downbeat: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseMarker {
    pub section_type: String, // 'intro', 'verse', 'chorus', 'bridge', 'drop', 'outro'
    pub start_ms: i64,
    pub end_ms: i64,
    pub energy_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodProfile {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub target_energy: f64,
    pub transition_style: String,
    pub repetition_cooldown_mins: i32,
    pub artist_cooldown_mins: i32,
    pub explicit_content_allowed: bool,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartySession {
    pub id: String,
    pub party_name: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub dj_level: i32, // 1 to 4
    pub primary_mood_id: String,
    pub secondary_mood_id: Option<String>,
    pub output_device_name: String,
    pub status: String, // 'active', 'paused', 'completed', 'recovered'
    pub total_tracks_played: i32,
    pub config_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guest {
    pub id: String,
    pub session_id: String,
    pub client_ip: String,
    pub display_name: String,
    pub token_hash: String,
    pub first_seen_at: i64,
    pub last_request_at: Option<i64>,
    pub request_count: i32,
    pub is_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestRequest {
    pub id: String,
    pub session_id: String,
    pub guest_id: String,
    pub guest_name: Option<String>,
    pub track_id: String,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub submitted_at: i64,
    pub status: String, // 'pending', 'approved', 'rejected', 'scheduled', 'playing', 'played', 'deferred'
    pub priority_order: i32,
    pub host_note: Option<String>,
    pub reviewed_at: Option<i64>,
    pub scheduled_slot: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub position: i64,
    pub session_id: String,
    pub track_id: String,
    pub track: Option<Track>,
    pub analysis: Option<TrackAnalysis>,
    pub source: String, // 'auto_dj', 'guest_request', 'host_override'
    pub assigned_deck: Option<i32>,
    pub status: String, // 'staged', 'preloaded', 'playing', 'finished'
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackHistoryItem {
    pub id: i64,
    pub session_id: String,
    pub track_id: String,
    pub track_title: String,
    pub track_artist: String,
    pub deck_id: i32,
    pub played_at: i64,
    pub duration_played_ms: i64,
    pub was_skipped: bool,
    pub transition_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub id: Option<i64>,
    pub session_id: Option<String>,
    pub event_timestamp: i64,
    pub subsystem: String,
    pub severity: String,
    pub message: String,
    pub context_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayableTrackCandidate {
    pub track: Track,
    pub asset: AudioAsset,
    pub analysis: TrackAnalysis,
    pub compatibility_score: f64,
    pub transition_style: String,
}

