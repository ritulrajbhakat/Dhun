use rusqlite::{Connection, Result, params};
use tracing::info;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL,
            description TEXT NOT NULL
        );"
    )?;

    let current_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    if current_version < 1 {
        info!("Applying DHUN schema migration v1 (24 entities + indexes)...");
        let schema_sql = include_str!("schema.sql");
        conn.execute_batch(schema_sql)?;

        seed_mood_profiles(conn)?;
        seed_energy_profiles(conn)?;
        seed_default_settings(conn)?;
        seed_sample_tracks(conn)?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (1, ?1, 'Initial 24 tables, indexes, and mood seed data')",
            params![now],
        )?;
        info!("Migration v1 applied successfully.");
    }

    if current_version < 2 {
        info!("Applying DHUN schema migration v2 (download_tasks & sync_outbox)...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS download_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                url TEXT NOT NULL,
                destination_path TEXT NOT NULL,
                expected_sha256 TEXT,
                status TEXT NOT NULL,
                bytes_downloaded INTEGER NOT NULL DEFAULT 0,
                total_bytes INTEGER,
                error_message TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_download_tasks_status ON download_tasks(status);

            CREATE TABLE IF NOT EXISTS sync_outbox (
                id TEXT PRIMARY KEY NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                last_attempt_at INTEGER,
                error_message TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sync_outbox_status ON sync_outbox(status);
            CREATE INDEX IF NOT EXISTS idx_sync_outbox_retry ON sync_outbox(retry_count, status);
            ",
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (2, ?1, 'Added download_tasks and sync_outbox tables')",
            params![now],
        )?;
        info!("Migration v2 applied successfully.");
    }

    if current_version < 3 {
        info!("Applying DHUN schema migration v3 (Cloud Party foundation & YouTube sources)...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS parties (
                id TEXT PRIMARY KEY NOT NULL,
                party_name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                host_token_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                ended_at INTEGER,
                last_heartbeat_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_parties_status ON parties(status);

            CREATE TABLE IF NOT EXISTS party_configurations (
                party_id TEXT PRIMARY KEY NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                dj_level TEXT NOT NULL DEFAULT 'balanced',
                primary_mood_id TEXT NOT NULL DEFAULT 'house_party',
                secondary_mood_id TEXT,
                energy_progression TEXT NOT NULL DEFAULT 'buildup',
                allow_guest_requests INTEGER NOT NULL DEFAULT 1,
                max_requests_per_guest INTEGER NOT NULL DEFAULT 3,
                request_cooldown_mins INTEGER NOT NULL DEFAULT 10,
                require_host_approval INTEGER NOT NULL DEFAULT 1,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS party_source_references (
                id TEXT PRIMARY KEY NOT NULL,
                party_id TEXT NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                provider TEXT NOT NULL,
                original_url TEXT NOT NULL,
                provider_video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                thumbnail_url TEXT,
                duration_seconds INTEGER,
                verification_status TEXT NOT NULL DEFAULT 'verified',
                playback_method TEXT NOT NULL DEFAULT 'youtube_iframe_embed',
                playability_status TEXT NOT NULL DEFAULT 'playback_unverified',
                metadata_status TEXT NOT NULL DEFAULT 'fetched',
                error_reason TEXT,
                registered_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_party_sources_party ON party_source_references(party_id);
            CREATE INDEX IF NOT EXISTS idx_party_sources_video_id ON party_source_references(party_id, provider_video_id);

            CREATE TABLE IF NOT EXISTS party_guests (
                id TEXT PRIMARY KEY NOT NULL,
                party_id TEXT NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                nickname TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                client_ip TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                is_blocked INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_party_guests_lookup ON party_guests(party_id, token_hash);

            CREATE TABLE IF NOT EXISTS party_song_requests (
                id TEXT PRIMARY KEY NOT NULL,
                party_id TEXT NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                guest_id TEXT NOT NULL REFERENCES party_guests(id) ON DELETE CASCADE,
                source_id TEXT NOT NULL REFERENCES party_source_references(id) ON DELETE CASCADE,
                custom_note TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                rejection_reason TEXT,
                priority_score REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_party_requests_party_status ON party_song_requests(party_id, status);

            CREATE TABLE IF NOT EXISTS request_status_history (
                id TEXT PRIMARY KEY NOT NULL,
                request_id TEXT NOT NULL REFERENCES party_song_requests(id) ON DELETE CASCADE,
                from_status TEXT NOT NULL,
                to_status TEXT NOT NULL,
                actor_type TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                reason TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_req_history_request ON request_status_history(request_id);

            CREATE TABLE IF NOT EXISTS shared_track_metadata (
                provider_video_id TEXT PRIMARY KEY NOT NULL,
                provider TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                thumbnail_url TEXT,
                duration_seconds INTEGER,
                cached_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playback_events (
                id TEXT PRIMARY KEY NOT NULL,
                party_id TEXT NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                track_id TEXT,
                payload_json TEXT,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_playback_events_party ON playback_events(party_id);

            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY NOT NULL,
                party_id TEXT NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
                action TEXT NOT NULL,
                actor TEXT NOT NULL,
                details_json TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_events_party ON audit_events(party_id);
            ",
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (3, ?1, 'Added Cloud Party foundation, YouTube sources, and guest request tables')",
            params![now],
        )?;
        info!("Migration v3 applied successfully.");
    }

    if current_version < 4 {
        info!("Applying DHUN schema migration v4 (Local Party Helper rooms, private submissions, 10-15 track temp cache, & global metadata cache)...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS local_party_rooms (
                room_id TEXT PRIMARY KEY NOT NULL,
                room_code TEXT NOT NULL UNIQUE,
                party_name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created', -- 'created', 'live', 'closed' (explicit closure only)
                dj_level INTEGER NOT NULL DEFAULT 2,
                primary_mood_id TEXT NOT NULL DEFAULT 'house_party',
                secondary_mood_id TEXT,
                host_token_hash TEXT NOT NULL,
                room_access_token_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                started_at INTEGER,
                closed_at INTEGER,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_local_rooms_status ON local_party_rooms(status);

            CREATE TABLE IF NOT EXISTS local_room_guests (
                id TEXT PRIMARY KEY NOT NULL,
                room_id TEXT NOT NULL REFERENCES local_party_rooms(room_id) ON DELETE CASCADE,
                nickname TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                client_ip TEXT NOT NULL,
                submission_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_local_room_guests_auth ON local_room_guests(room_id, token_hash);

            CREATE TABLE IF NOT EXISTS local_room_submissions (
                id TEXT PRIMARY KEY NOT NULL,
                room_id TEXT NOT NULL REFERENCES local_party_rooms(room_id) ON DELETE CASCADE,
                submitted_by_type TEXT NOT NULL, -- 'guest' | 'host'
                submitted_by_id TEXT NOT NULL,
                submitted_by_name TEXT NOT NULL,
                provider TEXT NOT NULL, -- 'youtube' | 'youtube_music'
                source_url TEXT NOT NULL,
                provider_video_id TEXT NOT NULL,
                song_id TEXT,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                version_tag TEXT NOT NULL DEFAULT 'original',
                thumbnail_url TEXT,
                duration_ms INTEGER,
                bpm REAL,
                half_time_bpm REAL,
                double_time_bpm REAL,
                musical_key TEXT,
                camelot_key TEXT,
                overall_energy REAL,
                danceability REAL,
                genre_tags_json TEXT NOT NULL DEFAULT '[]',
                mood_tags_json TEXT NOT NULL DEFAULT '[]',
                metadata_source TEXT NOT NULL DEFAULT 'pending',
                acquisition_status TEXT NOT NULL DEFAULT 'restricted_by_provider',
                acquisition_note TEXT,
                dj_status TEXT NOT NULL DEFAULT 'queued_for_dj', -- 'queued_for_dj', 'prepared', 'playing', 'played', 'skipped_unplayable'
                dj_score REAL NOT NULL DEFAULT 0.0,
                local_cache_path TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_local_sub_room_status ON local_room_submissions(room_id, dj_status);
            CREATE INDEX IF NOT EXISTS idx_local_sub_guest ON local_room_submissions(room_id, submitted_by_id);

            CREATE TABLE IF NOT EXISTS local_temp_cache (
                id TEXT PRIMARY KEY NOT NULL,
                room_id TEXT NOT NULL,
                submission_id TEXT,
                song_id TEXT NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                size_bytes INTEGER NOT NULL,
                priority_tier INTEGER NOT NULL DEFAULT 0, -- 3=playing, 2=next_deck, 1=upcoming, 0=evictable
                is_locked INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_accessed_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_local_temp_cache_room ON local_temp_cache(room_id);
            CREATE INDEX IF NOT EXISTS idx_local_temp_cache_expiry ON local_temp_cache(expires_at);
            ",
        )?;

        apply_global_catalogue_schema(conn)?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (4, ?1, 'Local Party Helper tables, temp cache, and Global Song Metadata Catalogue schema')",
            params![now],
        )?;
        info!("Migration v4 applied successfully.");
    }

    if current_version < 5 {
        info!("Applying DHUN schema migration v5 (Removing obsolete Phase 1 cloud-party tables)...");
        conn.execute_batch(
            "DROP TABLE IF EXISTS audit_events;
             DROP TABLE IF EXISTS playback_events;
             DROP TABLE IF EXISTS shared_track_metadata;
             DROP TABLE IF EXISTS request_status_history;
             DROP TABLE IF EXISTS party_song_requests;
             DROP TABLE IF EXISTS party_guests;
             DROP TABLE IF EXISTS party_source_references;
             DROP TABLE IF EXISTS party_configurations;
             DROP TABLE IF EXISTS parties;
            ",
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (5, ?1, 'Removed obsolete Phase 1 cloud-party tables; local party helper tables are authoritative')",
            params![now],
        )?;
        info!("Migration v5 applied successfully.");
    }

    if current_version < 6 {
        info!("Applying DHUN schema migration v6 (Persistent async submission intake queue)...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS submission_intake_jobs (
                id TEXT PRIMARY KEY NOT NULL,
                room_id TEXT NOT NULL REFERENCES local_party_rooms(room_id) ON DELETE CASCADE,
                submission_id TEXT NOT NULL REFERENCES local_room_submissions(id) ON DELETE CASCADE,
                raw_url TEXT NOT NULL,
                authorized_stream_json TEXT,
                status TEXT NOT NULL DEFAULT 'received',
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_intake_jobs_status_created
                ON submission_intake_jobs(status, created_at);
            CREATE INDEX IF NOT EXISTS idx_intake_jobs_room
                ON submission_intake_jobs(room_id, status);
            ",
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (6, ?1, 'Persistent asynchronous submission intake queue surviving helper restarts')",
            params![now],
        )?;
        info!("Migration v6 applied successfully.");
    }

    if current_version < 7 {
        info!("Applying DHUN schema migration v7 (Audience heatmaps & hook intelligence fusion tables)...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS track_audience_heatmaps (
                id TEXT PRIMARY KEY NOT NULL,
                track_id TEXT NOT NULL,
                provider_video_id TEXT NOT NULL,
                retrieval_status TEXT NOT NULL, -- 'available', 'unavailable', 'failed', 'blocked'
                raw_markers_json TEXT NOT NULL DEFAULT '[]',
                labelled_regions_json TEXT NOT NULL DEFAULT '[]',
                extracted_regions_json TEXT NOT NULL DEFAULT '[]',
                parser_version INTEGER NOT NULL DEFAULT 1,
                retrieved_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_heatmaps_track ON track_audience_heatmaps(track_id);
            CREATE INDEX IF NOT EXISTS idx_heatmaps_video ON track_audience_heatmaps(provider_video_id);

            CREATE TABLE IF NOT EXISTS track_hook_candidates (
                id TEXT PRIMARY KEY NOT NULL,
                track_id TEXT NOT NULL,
                video_id TEXT,
                video_start_ms INTEGER,
                video_end_ms INTEGER,
                audio_start_ms INTEGER NOT NULL,
                audio_end_ms INTEGER NOT NULL,
                dhun_musical_score REAL NOT NULL,
                audience_replay_score REAL,
                combined_hook_score REAL NOT NULL,
                alignment_status TEXT NOT NULL,
                alignment_offset_ms INTEGER NOT NULL DEFAULT 0,
                alignment_confidence REAL NOT NULL DEFAULT 0.0,
                dj_usability_score REAL NOT NULL DEFAULT 0.0,
                is_audience_popular INTEGER NOT NULL DEFAULT 0,
                is_musical_hook INTEGER NOT NULL DEFAULT 0,
                is_aligned_hook INTEGER NOT NULL DEFAULT 0,
                is_usable_entry INTEGER NOT NULL DEFAULT 0,
                is_usable_exit INTEGER NOT NULL DEFAULT 0,
                candidate_type TEXT NOT NULL,
                entry_cue_ms INTEGER NOT NULL,
                exit_cue_ms INTEGER NOT NULL,
                usability_reasons_json TEXT NOT NULL DEFAULT '[]',
                rejection_reasons_json TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_hook_cand_track ON track_hook_candidates(track_id);
            CREATE INDEX IF NOT EXISTS idx_hook_cand_scores ON track_hook_candidates(combined_hook_score, dj_usability_score);
            ",
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (7, ?1, 'Audience heatmaps and hook candidates persistence')",
            params![now],
        )?;
        info!("Migration v7 applied successfully.");
    }

    Ok(())
}

/// Strictly isolated schema for the separate Cloud Global Shared Song Metadata Catalogue Service.
/// Contains ZERO party rooms, ZERO playlists, ZERO guest requests, and ZERO audio files.
pub fn apply_global_catalogue_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS global_song_catalogue (
            song_id TEXT PRIMARY KEY NOT NULL,
            canonical_fingerprint TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            version_tag TEXT NOT NULL DEFAULT 'original',
            duration_ms INTEGER NOT NULL DEFAULT 0,
            original_bpm REAL,
            half_time_bpm REAL,
            double_time_bpm REAL,
            bpm_confidence REAL,
            musical_key TEXT,
            camelot_key TEXT,
            key_confidence REAL,
            beat_grid_json TEXT,
            phrase_markers_json TEXT,
            overall_energy REAL,
            danceability REAL,
            genre_tags_json TEXT NOT NULL DEFAULT '[]',
            mood_tags_json TEXT NOT NULL DEFAULT '[]',
            analysis_version INTEGER NOT NULL DEFAULT 1,
            analysis_confidence REAL NOT NULL DEFAULT 0.0,
            processing_status TEXT NOT NULL DEFAULT 'metadata_only',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_global_song_fp ON global_song_catalogue(canonical_fingerprint);

        CREATE TABLE IF NOT EXISTS global_song_source_urls (
            source_url TEXT PRIMARY KEY NOT NULL,
            provider TEXT NOT NULL,
            provider_video_id TEXT NOT NULL,
            song_id TEXT NOT NULL REFERENCES global_song_catalogue(song_id) ON DELETE CASCADE,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_global_source_vid ON global_song_source_urls(provider_video_id);
        CREATE INDEX IF NOT EXISTS idx_global_source_song ON global_song_source_urls(song_id);
        ",
    )?;
    Ok(())
}

/// Runs migrations strictly for the standalone Global Metadata Catalogue database (zero party tables).
pub fn run_global_catalogue_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL,
            description TEXT NOT NULL
        );"
    )?;
    let current_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if current_version < 1 {
        apply_global_catalogue_schema(conn)?;
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (1, ?1, 'Global Shared Song Metadata Catalogue isolated schema')",
            params![now],
        )?;
        info!("Global Metadata Catalogue isolated schema v1 applied.");
    }
    Ok(())
}

fn seed_mood_profiles(conn: &Connection) -> Result<()> {
    let moods = [
        ("shaadi", "Shaadi", "Grand wedding celebrations combining Bollywood, Punjabi, and timeless wedding anthems.", 100.0, 130.0, 0.75, "bass_swap", 120, 45, 0),
        ("baraat", "Baraat", "Electrifying high-octane procession dhol beats, brass hits, and Punjabi hype.", 126.0, 145.0, 0.95, "quick_cut", 90, 30, 0),
        ("mehendi", "Mehendi", "Melodic folk, contemporary acoustic Bollywood, and warm joyful Sufi rhythms.", 85.0, 110.0, 0.55, "phrase_sync", 150, 60, 0),
        ("haldi", "Haldi", "Upbeat traditional folk celebration, lively acoustic percussion, and festive melodies.", 95.0, 118.0, 0.65, "phrase_sync", 150, 60, 0),
        ("sangeet", "Sangeet", "Dynamic dance competition and choreography anthems with energetic stage peaks.", 115.0, 135.0, 0.85, "bass_swap", 120, 40, 0),
        ("desi_club", "Desi Club", "Heavy 4x4 commercial desi remixes, tech house fusions, and club bangers.", 124.0, 132.0, 0.90, "filter_sweep", 120, 45, 0),
        ("punjabi_tashan", "Punjabi Tashan", "Heavy 808s, UK Punjabi bhangra, swagger anthems, and modern urban vibes.", 128.0, 142.0, 0.90, "quick_cut", 90, 30, 0),
        ("garba", "Garba", "Traditional 3-Taali and 2-Taali continuous cyclic rhythm with accelerating tempo.", 110.0, 140.0, 0.80, "phrase_sync", 180, 60, 0),
        ("dandiya", "Dandiya", "Fast synchronized sticks tempo with driving percussion and folk celebration.", 120.0, 144.0, 0.88, "phrase_sync", 180, 60, 0),
        ("retro_bollywood", "Retro Bollywood", "Timeless 70s and 80s RD Burman disco funk, Kishore Kumar, and golden hits.", 90.0, 125.0, 0.60, "phrase_sync", 180, 60, 0),
        ("house_party", "House Party", "Versatile mix of international pop, trending hip-hop, and easy-dance tunes.", 105.0, 126.0, 0.70, "bass_swap", 120, 45, 1),
        ("cocktail_night", "Cocktail Night", "Sophisticated deep house, melodic chill, smooth lounge, and classy reworks.", 118.0, 124.0, 0.60, "filter_sweep", 180, 60, 0),
        ("festival_mode", "Festival Mode", "Peak stadium energy, massive drops, EDM fusions, and explosive transitions.", 126.0, 138.0, 0.98, "quick_cut", 90, 30, 1),
        ("international_party", "International Party", "Global chart-toppers, reggaeton, Afrobeat, and commercial dance.", 100.0, 130.0, 0.80, "bass_swap", 120, 45, 1),
    ];

    for (id, name, desc, min_bpm, max_bpm, energy, transition, rep_cd, art_cd, explicit) in moods {
        conn.execute(
            "INSERT OR IGNORE INTO mood_profiles (
                id, display_name, description, min_bpm, max_bpm, target_energy,
                transition_style, repetition_cooldown_mins, artist_cooldown_mins, explicit_content_allowed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, name, desc, min_bpm, max_bpm, energy, transition, rep_cd, art_cd, explicit],
        )?;
    }

    // Seed genre weights for primary Indian moods
    let genre_weights = [
        ("shaadi", "Bollywood", 0.50),
        ("shaadi", "Punjabi", 0.30),
        ("shaadi", "Wedding Remix", 0.20),
        ("baraat", "Bhangra Dhol", 0.45),
        ("baraat", "Punjabi Pop", 0.35),
        ("baraat", "Brass Band", 0.20),
        ("desi_club", "Desi Remix", 0.50),
        ("desi_club", "Tech House", 0.30),
        ("desi_club", "Club EDM", 0.20),
        ("garba", "Garba Folk", 0.60),
        ("garba", "Sanedo", 0.40),
        ("dandiya", "Dandiya Raas", 0.70),
        ("dandiya", "Fusion Beats", 0.30),
        ("retro_bollywood", "Disco Bollywood", 0.60),
        ("retro_bollywood", "Golden Classics", 0.40),
    ];

    for (mood, genre, weight) in genre_weights {
        conn.execute(
            "INSERT OR IGNORE INTO genre_weights (mood_id, genre, weight) VALUES (?1, ?2, ?3)",
            params![mood, genre, weight],
        )?;
    }

    Ok(())
}

fn seed_energy_profiles(conn: &Connection) -> Result<()> {
    let profiles = [
        ("steady", "Steady", "Maintains an even party energy level throughout the session.", "steady", r#"[{"stage":"steady","energy":0.75,"duration_mins":240}]"#),
        ("buildup", "Build-up", "Starts with warm melodies and progressively elevates energy.", "buildup", r#"[{"stage":"warmup","energy":0.5,"duration_mins":45},{"stage":"groove","energy":0.7,"duration_mins":60},{"stage":"peak","energy":0.9,"duration_mins":90}]"#),
        ("peak_party", "Peak Party", "Instant high energy sustained for dancing and celebrations.", "peak", r#"[{"stage":"peak","energy":0.95,"duration_mins":180}]"#),
        ("waves", "Waves", "Oscillates between high-energy bangers and moderate breather grooves.", "waves", r#"[{"stage":"high","energy":0.9,"duration_mins":40},{"stage":"breather","energy":0.65,"duration_mins":20},{"stage":"high","energy":0.95,"duration_mins":45}]"#),
    ];

    for (id, name, desc, curve, json_data) in profiles {
        conn.execute(
            "INSERT OR IGNORE INTO energy_profiles (id, name, description, curve_type, stage_durations_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, desc, curve, json_data],
        )?;
    }

    Ok(())
}

fn seed_sample_tracks(conn: &Connection) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    
    let sample_tracks = [
        ("trk_01", "Gallan Goodiyaan", "Diljit Dosanjh, Shankar Mahadevan", Some("Dil Dhadakne Do"), 296000, Some(2015), "Bollywood Dance", 128.0, 0.98, "D Major", "10B", 0.95, 0.92, 0.96),
        ("trk_02", "London Thumakda", "Labh Janjua, Sonu Kakkar, Neha Kakkar", Some("Queen"), 230000, Some(2014), "Punjabi Wedding", 128.0, 0.96, "G Major", "9B", 0.92, 0.88, 0.94),
        ("trk_03", "Sauda Khara Khara", "Diljit Dosanjh, Sukhbir, Dhvani Bhanushali", Some("Good Newwz"), 228000, Some(2019), "Bhangra Dhol", 130.0, 0.99, "F Minor", "4A", 0.94, 0.95, 0.98),
        ("trk_04", "Kala Chashma", "Amar Arshi, Badshah, Neha Kakkar", Some("Baar Baar Dekho"), 247000, Some(2016), "Desi Club", 106.0, 0.97, "C# Minor", "12A", 0.91, 0.91, 0.95),
        ("trk_05", "Chogada", "Darshan Raval, Asees Kaur", Some("Loveyatri"), 249000, Some(2018), "Garba Folk", 122.0, 0.98, "A Major", "11B", 0.93, 0.87, 0.96),
        ("trk_06", "Balam Pichkari", "Vishal Dadlani, Shalmali Kholgade", Some("Yeh Jawaani Hai Deewani"), 289000, Some(2013), "Bollywood Dance", 120.0, 0.97, "C Major", "8B", 0.93, 0.90, 0.94),
        ("trk_07", "Mundian To Bach Ke", "Panjabi MC", Some("Legalised"), 244000, Some(1998), "Punjabi Pop", 100.0, 0.98, "Ab Minor", "1A", 0.95, 0.93, 0.96),
        ("trk_08", "Aankh Marey", "Neha Kakkar, Mika Singh, Kumar Sanu", Some("Simmba"), 212000, Some(2018), "Desi Club", 124.0, 0.97, "F Major", "7B", 0.92, 0.91, 0.95),
        ("trk_09", "Kamariya", "Darshan Raval", Some("Mitron"), 188000, Some(2018), "Garba Folk", 124.0, 0.96, "F# Major", "2B", 0.91, 0.89, 0.95),
        ("trk_10", "Disco Deewane (The Disco Song)", "Benny Dayal, Sunidhi Chauhan, Vishal Dadlani", Some("Student of the Year"), 342000, Some(2012), "Retro Bollywood", 128.0, 0.98, "Bb Major", "6B", 0.94, 0.85, 0.92),
    ];

    for (id, title, artist, album, dur, yr, genre, bpm, bpm_c, key_name, camelot, key_c, energy, dance) in sample_tracks {
        conn.execute(
            "INSERT OR IGNORE INTO tracks (
                id, title, artist, album, duration_ms, year, genre, subgenre, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?8)",
            params![id, title, artist, album, dur, yr, genre, now],
        )?;

        let asset_id = format!("ast_{}", id);
        let dummy_path = format!("demo://songs/{}.mp3", id);
        conn.execute(
            "INSERT OR IGNORE INTO audio_assets (
                id, track_id, file_path, file_format, file_size_bytes, sample_rate, bit_depth, channels, sha256_hash,
                verification_status, availability_status, last_verified_at
            ) VALUES (?1, ?2, ?3, 'mp3', 5000000, 44100, 16, 2, 'demosha256hash', 'unverified', 'demo_catalog', ?4)",
            params![asset_id, id, dummy_path, now],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO track_analysis (
                track_id, bpm, bpm_confidence, key_name, camelot_key, key_confidence,
                time_signature_num, time_signature_den, downbeat_offset_ms, phrase_length_bars,
                overall_energy, dance_suitability, recommended_mix_in_ms, recommended_mix_out_ms,
                analysis_version, analyzed_at, manual_override
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 4, 4, 120, 32, ?7, ?8, 15000, ?9 - 15000, 1, ?10, 0)",
            params![id, bpm, bpm_c, key_name, camelot, key_c, energy, dance, dur, now],
        )?;
    }

    Ok(())
}

fn seed_default_settings(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO device_settings (
            id, output_device_name, sample_rate, buffer_size, headphone_cue_device, latency_ms, limiter_enabled, limiter_threshold_db
        ) VALUES ('default', 'Default System Output', 44100, 512, NULL, 11.6, 1, -0.1)",
        [],
    )?;

    let now = chrono::Utc::now().timestamp_millis();
    let app_defaults = [
        ("auto_dj_enabled", "true"),
        ("crossfade_duration_beats", "32"),
        ("smart_bass_swap", "true"),
        ("guest_requests_enabled", "true"),
        ("guest_request_limit_per_user", "3"),
        ("guest_request_cooldown_mins", "10"),
        ("require_host_approval", "true"),
    ];

    for (key, val) in app_defaults {
        conn.execute(
            "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, val, now],
        )?;
    }

    Ok(())
}

