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
    pub section_type: String, // 'intro', 'groove', 'breakdown', 'build', 'drop', 'peak', 'outro', 'phrase'
    pub start_ms: i64,
    pub end_ms: i64,
    pub energy_level: f64,
    #[serde(default)]
    pub bass_energy: f64,
    #[serde(default)]
    pub confidence: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub destination_path: String,
    pub expected_sha256: Option<String>,
    pub status: String, // 'pending', 'downloading', 'verifying', 'ingesting', 'completed', 'failed', 'cancelled'
    pub bytes_downloaded: i64,
    pub total_bytes: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub asset_id: String,
    pub cached_file_path: String,
    pub size_bytes: i64,
    pub is_locked: bool,
    pub lock_reason: Option<String>,
    pub last_accessed_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_bytes: i64,
    pub locked_files: usize,
    pub locked_bytes: i64,
    pub max_cache_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutboxItem {
    pub id: String,
    pub entity_type: String, // 'track_metadata', 'playback_event', 'guest_rating'
    pub entity_id: String,
    pub payload_json: String,
    pub retry_count: i32,
    pub status: String, // 'pending', 'in_flight', 'synced', 'failed'
    pub last_attempt_at: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusOverview {
    pub pending_count: usize,
    pub in_flight_count: usize,
    pub synced_count: usize,
    pub failed_count: usize,
}

// ========================================================================
// Local Party Helper & Global Shared Song Metadata Catalogue Models
// ========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPartyRoom {
    pub room_id: String,
    pub room_code: String,
    pub party_name: String,
    pub status: String, // 'created' | 'live' | 'closed' (explicit closure only)
    pub dj_level: i32,  // 1..=4
    pub primary_mood_id: String,
    pub secondary_mood_id: Option<String>,
    #[serde(skip_serializing)]
    pub host_token_hash: String,
    #[serde(skip_serializing)]
    pub room_access_token_hash: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRoomGuest {
    pub id: String,
    pub room_id: String,
    pub nickname: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub client_ip: String,
    pub submission_count: i32,
    pub created_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRoomSubmission {
    pub id: String,
    pub room_id: String,
    pub submitted_by_type: String, // 'guest' | 'host'
    pub submitted_by_id: String,
    pub submitted_by_name: String,
    pub provider: String, // 'youtube' | 'youtube_music'
    pub source_url: String,
    pub provider_video_id: String,
    pub song_id: Option<String>,
    pub title: String,
    pub artist: String,
    pub version_tag: String,
    pub thumbnail_url: Option<String>,
    pub duration_ms: Option<i64>,
    pub bpm: Option<f64>,
    pub half_time_bpm: Option<f64>,
    pub double_time_bpm: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot_key: Option<String>,
    pub overall_energy: Option<f64>,
    pub danceability: Option<f64>,
    pub genre_tags_json: String,
    pub mood_tags_json: String,
    pub metadata_source: String,
    pub acquisition_status: String, // 'restricted_by_provider' | 'cached_authorized' | 'pending' | 'failed'
    pub acquisition_note: Option<String>,
    pub dj_status: String, // 'queued_for_dj' | 'prepared' | 'playing' | 'played' | 'skipped_unplayable'
    pub dj_score: f64,
    #[serde(skip_serializing)]
    pub local_cache_path: Option<String>,
    /// Opaque local temporary cache identifier (never exposes raw host filesystem paths)
    #[serde(default)]
    pub cache_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Strictly privacy-isolated view returned to an authenticated guest for their OWN submissions only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestOwnSubmissionView {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub provider: String,
    pub status: String, // 'received_by_dj' | 'queued_next' | 'playing' | 'played'
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTempCacheEntry {
    pub id: String,
    pub room_id: String,
    pub submission_id: Option<String>,
    pub song_id: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub priority_tier: i32, // 3=playing, 2=next_deck, 1=upcoming, 0=evictable
    pub is_locked: bool,
    pub created_at: i64,
    pub last_accessed_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTempCacheStats {
    pub track_count: usize,
    pub max_tracks: usize,
    pub total_bytes: i64,
    pub locked_tracks: usize,
}

/// Persistent background intake job stored in SQLite (`submission_intake_jobs`)
/// so async song submissions survive local helper restarts without data loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionIntakeJob {
    pub id: String,
    pub room_id: String,
    pub submission_id: String,
    pub raw_url: String,
    pub authorized_stream_json: Option<String>,
    pub status: String, // 'received' | 'metadata_processing' | 'awaiting_authorized_audio' | 'analysing' | 'ready' | 'restricted_by_provider' | 'failed'
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Global Shared Song Metadata Catalogue Record (Reusable Musical Intelligence ONLY).
/// Never contains party IDs, playlists, guest requests, or raw audio files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSongMetadata {
    pub song_id: String,
    pub canonical_fingerprint: String,
    pub title: String,
    pub artist: String,
    pub version_tag: String, // 'original', 'remix', 'live', 'edit', 'acoustic', 'extended', etc.
    pub duration_ms: i64,
    pub original_bpm: Option<f64>,
    pub half_time_bpm: Option<f64>,
    pub double_time_bpm: Option<f64>,
    pub bpm_confidence: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot_key: Option<String>,
    pub key_confidence: Option<f64>,
    pub beat_grid_json: Option<String>,
    pub phrase_markers_json: Option<String>,
    pub overall_energy: Option<f64>,
    pub danceability: Option<f64>,
    pub genre_tags: Vec<String>,
    pub mood_tags: Vec<String>,
    pub analysis_version: i32,
    pub analysis_confidence: f64,
    pub processing_status: String, // 'metadata_only' | 'analyzed' | 'verified'
    pub source_urls: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}



