-- ========================================================================
-- DHUN Database Schema (v1)
-- 24 Entities Supporting Offline-First DJ Operations
-- ========================================================================

PRAGMA foreign_keys = ON;

-- 1. Schema Migrations History
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT NOT NULL
);

-- 2. Tracks (Permanent Musical Identity)
CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    duration_ms INTEGER NOT NULL,
    year INTEGER,
    genre TEXT NOT NULL,
    subgenre TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tracks_artist_title ON tracks(artist, title);
CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre);

-- 3. Audio Assets (Physical Audio Files & Verification)
CREATE TABLE IF NOT EXISTS audio_assets (
    id TEXT PRIMARY KEY NOT NULL,
    track_id TEXT NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    file_format TEXT NOT NULL,          -- 'mp3', 'wav', 'flac', 'aac', 'm4a', 'ogg'
    file_size_bytes INTEGER NOT NULL,
    sample_rate INTEGER NOT NULL,
    bit_depth INTEGER,
    channels INTEGER NOT NULL,
    sha256_hash TEXT NOT NULL,
    verification_status TEXT NOT NULL,   -- 'unverified', 'verified', 'corrupt', 'missing'
    availability_status TEXT NOT NULL,   -- 'offline', 'available', 'locked_playback'
    last_verified_at INTEGER NOT NULL,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_assets_track ON audio_assets(track_id);
CREATE INDEX IF NOT EXISTS idx_assets_status ON audio_assets(availability_status, verification_status);

-- 4. Source References (Where Audio/Metadata came from)
CREATE TABLE IF NOT EXISTS source_references (
    id TEXT PRIMARY KEY NOT NULL,
    track_id TEXT NOT NULL,
    source_type TEXT NOT NULL,          -- 'local_filesystem', 'itunes_import', 'rekordbox_xml', 'serato_crate'
    source_uri TEXT NOT NULL,
    external_id TEXT,
    imported_at INTEGER NOT NULL,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 5. Track Analysis (Deterministic Signal Attributes)
CREATE TABLE IF NOT EXISTS track_analysis (
    track_id TEXT PRIMARY KEY NOT NULL,
    bpm REAL NOT NULL,
    bpm_confidence REAL NOT NULL,
    key_name TEXT NOT NULL,             -- e.g. '8B', 'C Major'
    camelot_key TEXT NOT NULL,          -- '8B', '11A'
    key_confidence REAL NOT NULL,
    time_signature_num INTEGER DEFAULT 4,
    time_signature_den INTEGER DEFAULT 4,
    downbeat_offset_ms INTEGER NOT NULL,
    phrase_length_bars INTEGER DEFAULT 32,
    overall_energy REAL NOT NULL,       -- 0.0 to 1.0
    dance_suitability REAL NOT NULL,    -- 0.0 to 1.0
    recommended_mix_in_ms INTEGER NOT NULL,
    recommended_mix_out_ms INTEGER NOT NULL,
    analysis_version INTEGER NOT NULL,
    analyzed_at INTEGER NOT NULL,
    manual_override BOOLEAN DEFAULT 0,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 6. Beat Grids (Sample-Accurate Downbeats & Beat Markers)
CREATE TABLE IF NOT EXISTS beat_grids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    beat_number INTEGER NOT NULL,
    time_offset_ms INTEGER NOT NULL,
    is_downbeat BOOLEAN NOT NULL,
    confidence REAL DEFAULT 1.0,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_beat_grids_track ON beat_grids(track_id, beat_number);

-- 7. Phrase Markers (8/16/32-bar Structural Sections)
CREATE TABLE IF NOT EXISTS phrase_markers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    section_type TEXT NOT NULL,         -- 'intro', 'verse', 'chorus', 'bridge', 'drop', 'outro'
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    energy_level REAL NOT NULL,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 8. Vocal Regions (Singing Detection to Avoid Clashes)
CREATE TABLE IF NOT EXISTS vocal_regions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    vocal_density REAL NOT NULL,        -- 0.0 to 1.0
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 9. Transition Profiles (Mix In/Out Rules per Track)
CREATE TABLE IF NOT EXISTS transition_profiles (
    track_id TEXT PRIMARY KEY NOT NULL,
    default_transition_type TEXT NOT NULL, -- 'bass_swap', 'filter_sweep', 'phrase_sync', 'quick_cut'
    transition_duration_beats INTEGER DEFAULT 32,
    cut_frequency_hz REAL DEFAULT 250.0,
    gain_trim_db REAL DEFAULT 0.0,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 10. Mood Profiles (Indian Party Presets & Custom Moods)
CREATE TABLE IF NOT EXISTS mood_profiles (
    id TEXT PRIMARY KEY NOT NULL,       -- 'shaadi', 'baraat', 'desi_club', etc.
    display_name TEXT NOT NULL,
    description TEXT,
    min_bpm REAL NOT NULL,
    max_bpm REAL NOT NULL,
    target_energy REAL NOT NULL,
    transition_style TEXT NOT NULL,     -- 'phrase_sync', 'bass_swap', 'quick_cut', 'echo_tail'
    repetition_cooldown_mins INTEGER DEFAULT 120,
    artist_cooldown_mins INTEGER DEFAULT 45,
    explicit_content_allowed BOOLEAN DEFAULT 0,
    is_custom BOOLEAN DEFAULT 0
);

-- 11. Genre Weights per Mood Profile
CREATE TABLE IF NOT EXISTS genre_weights (
    mood_id TEXT NOT NULL,
    genre TEXT NOT NULL,
    weight REAL NOT NULL,               -- 0.0 to 1.0
    PRIMARY KEY(mood_id, genre),
    FOREIGN KEY(mood_id) REFERENCES mood_profiles(id) ON DELETE CASCADE
);

-- 12. Energy Profiles (Steady, Build-up, Peak Party, Waves)
CREATE TABLE IF NOT EXISTS energy_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    curve_type TEXT NOT NULL,           -- 'steady', 'buildup', 'peak', 'waves', 'custom'
    stage_durations_json TEXT NOT NULL  -- e.g. [{"stage": "warmup", "energy": 0.4, "minutes": 30}]
);

-- 13. Crates (DJ Crate Collections)
CREATE TABLE IF NOT EXISTS crates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    color_tag TEXT,
    created_at INTEGER NOT NULL
);

-- 14. Crate Tracks (Crate Membership)
CREATE TABLE IF NOT EXISTS crate_tracks (
    crate_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    PRIMARY KEY(crate_id, track_id),
    FOREIGN KEY(crate_id) REFERENCES crates(id) ON DELETE CASCADE,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 15. Playlists
CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 16. Party Sessions (Active & Past Events)
CREATE TABLE IF NOT EXISTS party_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    party_name TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    dj_level INTEGER NOT NULL,          -- 1, 2, 3, 4
    primary_mood_id TEXT NOT NULL,
    secondary_mood_id TEXT,
    output_device_name TEXT NOT NULL,
    status TEXT NOT NULL,               -- 'active', 'paused', 'completed', 'recovered'
    total_tracks_played INTEGER DEFAULT 0,
    config_json TEXT,
    FOREIGN KEY(primary_mood_id) REFERENCES mood_profiles(id)
);

-- 17. Guests (Mobile Requestors)
CREATE TABLE IF NOT EXISTS guests (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    client_ip TEXT NOT NULL,
    display_name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_request_at INTEGER,
    request_count INTEGER DEFAULT 0,
    is_blocked BOOLEAN DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES party_sessions(id) ON DELETE CASCADE
);

-- 18. Guest Requests (Queue with Host Moderation)
CREATE TABLE IF NOT EXISTS guest_requests (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    guest_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    submitted_at INTEGER NOT NULL,
    status TEXT NOT NULL,               -- 'pending', 'approved', 'rejected', 'scheduled', 'playing', 'played', 'deferred'
    priority_order INTEGER DEFAULT 100, -- 1 = Force Next, 10 = Host Priority, 100 = Guest Normal
    host_note TEXT,
    reviewed_at INTEGER,
    scheduled_slot INTEGER,
    FOREIGN KEY(session_id) REFERENCES party_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(guest_id) REFERENCES guests(id) ON DELETE CASCADE,
    FOREIGN KEY(track_id) REFERENCES tracks(id)
);
CREATE INDEX IF NOT EXISTS idx_requests_session_status ON guest_requests(session_id, status);

-- 19. Playback Queue (Active & Staged Tracks)
CREATE TABLE IF NOT EXISTS playback_queue (
    position INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    source TEXT NOT NULL,               -- 'auto_dj', 'guest_request', 'host_override'
    assigned_deck INTEGER,             -- 1, 2, 3, 4
    status TEXT NOT NULL,               -- 'staged', 'preloaded', 'playing', 'finished'
    request_id TEXT,
    FOREIGN KEY(session_id) REFERENCES party_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(track_id) REFERENCES tracks(id),
    FOREIGN KEY(request_id) REFERENCES guest_requests(id)
);

-- 20. Playback History (Session History Log)
CREATE TABLE IF NOT EXISTS playback_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    deck_id INTEGER NOT NULL,
    played_at INTEGER NOT NULL,
    duration_played_ms INTEGER NOT NULL,
    was_skipped BOOLEAN DEFAULT 0,
    transition_type TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES party_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(track_id) REFERENCES tracks(id)
);
CREATE INDEX IF NOT EXISTS idx_history_session_track ON playback_history(session_id, track_id);

-- 21. Cache Manifest (Rolling Cache Protection)
CREATE TABLE IF NOT EXISTS cache_manifest (
    asset_id TEXT PRIMARY KEY NOT NULL,
    cached_file_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    is_locked BOOLEAN DEFAULT 0,        -- 1 if active, queued, or playing
    lock_reason TEXT,                   -- 'deck_1', 'deck_2', 'preloaded_next'
    last_accessed_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(asset_id) REFERENCES audio_assets(id) ON DELETE CASCADE
);

-- 22. Device Settings & Audio Preferences
CREATE TABLE IF NOT EXISTS device_settings (
    id TEXT PRIMARY KEY NOT NULL,
    output_device_name TEXT NOT NULL,
    sample_rate INTEGER DEFAULT 44100,
    buffer_size INTEGER DEFAULT 512,
    headphone_cue_device TEXT,
    latency_ms REAL DEFAULT 11.6,
    limiter_enabled BOOLEAN DEFAULT 1,
    limiter_threshold_db REAL DEFAULT -0.1
);

-- 23. Application Settings
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 24. Analysis Jobs (Offline Processing Queue)
CREATE TABLE IF NOT EXISTS analysis_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    track_id TEXT NOT NULL,
    status TEXT NOT NULL,               -- 'pending', 'processing', 'completed', 'failed'
    retry_count INTEGER DEFAULT 0,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- 25. Diagnostic Events (Telemetry & Watchdog Log)
CREATE TABLE IF NOT EXISTS diagnostic_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    event_timestamp INTEGER NOT NULL,
    subsystem TEXT NOT NULL,            -- 'audio', 'db', 'network', 'recovery', 'dj_engine'
    severity TEXT NOT NULL,             -- 'info', 'warning', 'error', 'critical'
    message TEXT NOT NULL,
    context_json TEXT
);

