pub mod migrations;
pub mod models;

use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::db::models::*;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        
        migrations::run_migrations(&mut conn)?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::run_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Opens an isolated SQLite database containing ONLY the Global Shared Song Metadata Catalogue tables.
    /// Guarantees zero party rooms, playlists, guest requests, or audio file records exist in cloud storage.
    pub fn new_global_catalogue_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        migrations::run_global_catalogue_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new_global_catalogue_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::run_global_catalogue_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    // --- Diagnostic logging ---
    pub fn log_diagnostic(&self, session_id: Option<&str>, subsystem: &str, severity: &str, message: &str, context: Option<&str>) {
        if let Ok(conn) = self.conn.lock() {
            let now = chrono::Utc::now().timestamp_millis();
            let _ = conn.execute(
                "INSERT INTO diagnostic_events (session_id, event_timestamp, subsystem, severity, message, context_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, now, subsystem, severity, message, context],
            );
        }
    }

    // --- Track & Library Operations ---
    pub fn get_all_tracks(&self) -> Result<Vec<Track>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, artist, album, duration_ms, year, genre, subgenre, created_at, updated_at FROM tracks ORDER BY artist, title")?;
        let rows = stmt.query_map([], |row| {
            Ok(Track {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_ms: row.get(4)?,
                year: row.get(5)?,
                genre: row.get(6)?,
                subgenre: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }
        Ok(tracks)
    }

    pub fn insert_or_update_track(&self, track: &Track, asset: &AudioAsset, analysis: &TrackAnalysis) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "INSERT INTO tracks (id, title, artist, album, duration_ms, year, genre, subgenre, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                duration_ms = excluded.duration_ms,
                year = excluded.year,
                genre = excluded.genre,
                subgenre = excluded.subgenre,
                updated_at = excluded.updated_at",
            params![
                track.id, track.title, track.artist, track.album, track.duration_ms,
                track.year, track.genre, track.subgenre, track.created_at, track.updated_at
            ],
        )?;

        tx.execute(
            "INSERT INTO audio_assets (
                id, track_id, file_path, file_format, file_size_bytes, sample_rate,
                bit_depth, channels, sha256_hash, verification_status, availability_status, last_verified_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(file_path) DO UPDATE SET
                track_id = excluded.track_id,
                file_size_bytes = excluded.file_size_bytes,
                sample_rate = excluded.sample_rate,
                bit_depth = excluded.bit_depth,
                channels = excluded.channels,
                sha256_hash = excluded.sha256_hash,
                verification_status = excluded.verification_status,
                availability_status = excluded.availability_status,
                last_verified_at = excluded.last_verified_at",
            params![
                asset.id, asset.track_id, asset.file_path, asset.file_format, asset.file_size_bytes,
                asset.sample_rate, asset.bit_depth, asset.channels, asset.sha256_hash,
                asset.verification_status, asset.availability_status, asset.last_verified_at
            ],
        )?;

        tx.execute(
            "INSERT INTO track_analysis (
                track_id, bpm, bpm_confidence, key_name, camelot_key, key_confidence,
                time_signature_num, time_signature_den, downbeat_offset_ms, phrase_length_bars,
                overall_energy, dance_suitability, recommended_mix_in_ms, recommended_mix_out_ms,
                analysis_version, analyzed_at, manual_override
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(track_id) DO UPDATE SET
                bpm = excluded.bpm,
                bpm_confidence = excluded.bpm_confidence,
                key_name = excluded.key_name,
                camelot_key = excluded.camelot_key,
                key_confidence = excluded.key_confidence,
                overall_energy = excluded.overall_energy,
                analyzed_at = excluded.analyzed_at
            WHERE manual_override = 0",
            params![
                analysis.track_id, analysis.bpm, analysis.bpm_confidence, analysis.key_name,
                analysis.camelot_key, analysis.key_confidence, analysis.time_signature_num,
                analysis.time_signature_den, analysis.downbeat_offset_ms, analysis.phrase_length_bars,
                analysis.overall_energy, analysis.dance_suitability, analysis.recommended_mix_in_ms,
                analysis.recommended_mix_out_ms, analysis.analysis_version, analysis.analyzed_at,
                analysis.manual_override
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn insert_track_analysis(&self, analysis: &TrackAnalysis) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO track_analysis (
                track_id, bpm, bpm_confidence, key_name, camelot_key, key_confidence,
                time_signature_num, time_signature_den, downbeat_offset_ms, phrase_length_bars,
                overall_energy, dance_suitability, recommended_mix_in_ms, recommended_mix_out_ms,
                analysis_version, analyzed_at, manual_override
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(track_id) DO UPDATE SET
                bpm = excluded.bpm,
                bpm_confidence = excluded.bpm_confidence,
                key_name = excluded.key_name,
                camelot_key = excluded.camelot_key,
                key_confidence = excluded.key_confidence,
                overall_energy = excluded.overall_energy,
                analyzed_at = excluded.analyzed_at
            WHERE manual_override = 0",
            params![
                analysis.track_id, analysis.bpm, analysis.bpm_confidence, analysis.key_name,
                analysis.camelot_key, analysis.key_confidence, analysis.time_signature_num,
                analysis.time_signature_den, analysis.downbeat_offset_ms, analysis.phrase_length_bars,
                analysis.overall_energy, analysis.dance_suitability, analysis.recommended_mix_in_ms,
                analysis.recommended_mix_out_ms, analysis.analysis_version, analysis.analyzed_at,
                analysis.manual_override
            ],
        )?;
        Ok(())
    }

    pub fn get_playable_candidates(&self) -> Result<Vec<PlayableTrackCandidate>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT 
                t.id, t.title, t.artist, t.album, t.duration_ms, t.year, t.genre, t.subgenre, t.created_at, t.updated_at,
                a.id, a.track_id, a.file_path, a.file_format, a.file_size_bytes, a.sample_rate, a.bit_depth, a.channels, a.sha256_hash, a.verification_status, a.availability_status, a.last_verified_at,
                an.track_id, an.bpm, an.bpm_confidence, an.key_name, an.camelot_key, an.key_confidence, an.time_signature_num, an.time_signature_den, an.downbeat_offset_ms, an.phrase_length_bars, an.overall_energy, an.dance_suitability, an.recommended_mix_in_ms, an.recommended_mix_out_ms, an.analysis_version, an.analyzed_at, an.manual_override
             FROM tracks t
             JOIN audio_assets a ON t.id = a.track_id
             JOIN track_analysis an ON t.id = an.track_id
             WHERE a.verification_status = 'verified' AND a.availability_status IN ('available', 'locked_playback')"
        )?;

        let rows = stmt.query_map([], |row| {
            let track = Track {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_ms: row.get(4)?,
                year: row.get(5)?,
                genre: row.get(6)?,
                subgenre: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            };
            let asset = AudioAsset {
                id: row.get(10)?,
                track_id: row.get(11)?,
                file_path: row.get(12)?,
                file_format: row.get(13)?,
                file_size_bytes: row.get(14)?,
                sample_rate: row.get(15)?,
                bit_depth: row.get(16)?,
                channels: row.get(17)?,
                sha256_hash: row.get(18)?,
                verification_status: row.get(19)?,
                availability_status: row.get(20)?,
                last_verified_at: row.get(21)?,
            };
            let analysis = TrackAnalysis {
                track_id: row.get(22)?,
                bpm: row.get(23)?,
                bpm_confidence: row.get(24)?,
                key_name: row.get(25)?,
                camelot_key: row.get(26)?,
                key_confidence: row.get(27)?,
                time_signature_num: row.get(28)?,
                time_signature_den: row.get(29)?,
                downbeat_offset_ms: row.get(30)?,
                phrase_length_bars: row.get(31)?,
                overall_energy: row.get(32)?,
                dance_suitability: row.get(33)?,
                recommended_mix_in_ms: row.get(34)?,
                recommended_mix_out_ms: row.get(35)?,
                analysis_version: row.get(36)?,
                analyzed_at: row.get(37)?,
                manual_override: row.get(38)?,
            };

            Ok(PlayableTrackCandidate {
                track,
                asset,
                analysis,
                compatibility_score: 0.0,
                transition_style: "phrase_sync".to_string(),
            })
        })?;

        let mut candidates = Vec::new();
        let mut missing_ids = Vec::new();
        for r in rows {
            let candidate: PlayableTrackCandidate = r?;
            // Exclude demo records
            if candidate.asset.file_path.starts_with("demo://") {
                continue;
            }
            // Exclude non-existent physical files on disk
            let path = Path::new(&candidate.asset.file_path);
            if !path.exists() {
                missing_ids.push(candidate.asset.id);
                continue;
            }
            candidates.push(candidate);
        }

        drop(stmt);
        for id in missing_ids {
            let _ = conn.execute(
                "UPDATE audio_assets SET availability_status = 'missing' WHERE id = ?1",
                params![id],
            );
        }

        Ok(candidates)
    }

    // --- Mood Profiles ---
    pub fn get_mood_profiles(&self) -> Result<Vec<MoodProfile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, display_name, description, min_bpm, max_bpm, target_energy, transition_style, repetition_cooldown_mins, artist_cooldown_mins, explicit_content_allowed, is_custom FROM mood_profiles")?;
        let rows = stmt.query_map([], |row| {
            Ok(MoodProfile {
                id: row.get(0)?,
                display_name: row.get(1)?,
                description: row.get(2)?,
                min_bpm: row.get(3)?,
                max_bpm: row.get(4)?,
                target_energy: row.get(5)?,
                transition_style: row.get(6)?,
                repetition_cooldown_mins: row.get(7)?,
                artist_cooldown_mins: row.get(8)?,
                explicit_content_allowed: row.get(9)?,
                is_custom: row.get(10)?,
            })
        })?;

        let mut moods = Vec::new();
        for r in rows {
            moods.push(r?);
        }
        Ok(moods)
    }

    // --- Guest Requests ---
    pub fn register_guest(&self, id: &str, session_id: &str, client_ip: &str, display_name: &str, token_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO guests (id, session_id, client_ip, display_name, token_hash, first_seen_at, last_request_at, request_count, is_blocked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 0, 0)
             ON CONFLICT(id) DO UPDATE SET
                display_name = excluded.display_name,
                token_hash = excluded.token_hash,
                client_ip = excluded.client_ip",
            params![id, session_id, client_ip, display_name, token_hash, now],
        )?;
        Ok(())
    }

    pub fn lookup_guest_by_token_hash(&self, session_id: &str, token_hash: &str) -> Result<Option<Guest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, client_ip, display_name, token_hash, first_seen_at, last_request_at, request_count, is_blocked
             FROM guests WHERE session_id = ?1 AND token_hash = ?2"
        )?;
        let mut rows = stmt.query_map(params![session_id, token_hash], |row| {
            let is_blocked_val: i64 = row.get(8)?;
            Ok(Guest {
                id: row.get(0)?,
                session_id: row.get(1)?,
                client_ip: row.get(2)?,
                display_name: row.get(3)?,
                token_hash: row.get(4)?,
                first_seen_at: row.get(5)?,
                last_request_at: row.get(6)?,
                request_count: row.get(7)?,
                is_blocked: is_blocked_val != 0,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn increment_guest_request_count(&self, guest_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE guests SET request_count = request_count + 1, last_request_at = ?1 WHERE id = ?2",
            params![now, guest_id],
        )?;
        Ok(())
    }

    pub fn insert_guest_request(&self, req: &GuestRequest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Ensure guest record exists for foreign key constraint
        conn.execute(
            "INSERT OR IGNORE INTO guests (id, session_id, client_ip, display_name, token_hash, first_seen_at, last_request_at, request_count, is_blocked)
             VALUES (?1, ?2, '127.0.0.1', ?3, 'token', ?4, ?4, 1, 0)",
            params![req.guest_id, req.session_id, req.guest_name.as_deref().unwrap_or("Guest"), req.submitted_at],
        )?;

        conn.execute(
            "INSERT INTO guest_requests (id, session_id, guest_id, track_id, submitted_at, status, priority_order, host_note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![req.id, req.session_id, req.guest_id, req.track_id, req.submitted_at, req.status, req.priority_order, req.host_note],
        )?;
        Ok(())
    }

    pub fn get_session_requests(&self, session_id: &str) -> Result<Vec<GuestRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT r.id, r.session_id, r.guest_id, g.display_name, r.track_id, t.title, t.artist, r.submitted_at, r.status, r.priority_order, r.host_note, r.reviewed_at, r.scheduled_slot
             FROM guest_requests r
             LEFT JOIN guests g ON r.guest_id = g.id
             LEFT JOIN tracks t ON r.track_id = t.id
             WHERE r.session_id = ?1
             ORDER BY r.priority_order ASC, r.submitted_at ASC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(GuestRequest {
                id: row.get(0)?,
                session_id: row.get(1)?,
                guest_id: row.get(2)?,
                guest_name: row.get(3)?,
                track_id: row.get(4)?,
                track_title: row.get(5)?,
                track_artist: row.get(6)?,
                submitted_at: row.get(7)?,
                status: row.get(8)?,
                priority_order: row.get(9)?,
                host_note: row.get(10)?,
                reviewed_at: row.get(11)?,
                scheduled_slot: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_request_status(&self, request_id: &str, status: &str, priority: Option<i32>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        if let Some(p) = priority {
            conn.execute(
                "UPDATE guest_requests SET status = ?1, priority_order = ?2, reviewed_at = ?3 WHERE id = ?4",
                params![status, p, now, request_id],
            )?;
        } else {
            conn.execute(
                "UPDATE guest_requests SET status = ?1, reviewed_at = ?2 WHERE id = ?3",
                params![status, now, request_id],
            )?;
        }
        Ok(())
    }

    // --- Party Sessions ---
    pub fn create_session(&self, session: &PartySession) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO party_sessions (id, party_name, start_time, end_time, dj_level, primary_mood_id, secondary_mood_id, output_device_name, status, total_tracks_played, config_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session.id, session.party_name, session.start_time, session.end_time,
                session.dj_level, session.primary_mood_id, session.secondary_mood_id,
                session.output_device_name, session.status, session.total_tracks_played, session.config_json
            ],
        )?;
        Ok(())
    }

    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<PartySession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, party_name, start_time, end_time, dj_level, primary_mood_id, secondary_mood_id, output_device_name, status, total_tracks_played, config_json
             FROM party_sessions ORDER BY start_time DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(PartySession {
                id: row.get(0)?,
                party_name: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                dj_level: row.get(4)?,
                primary_mood_id: row.get(5)?,
                secondary_mood_id: row.get(6)?,
                output_device_name: row.get(7)?,
                status: row.get(8)?,
                total_tracks_played: row.get(9)?,
                config_json: row.get(10)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_track_id_by_path(&self, file_path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT track_id FROM audio_assets WHERE file_path = ?1")?;
        let mut rows = stmt.query_map(params![file_path], |row| row.get(0))?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_track_by_id(&self, track_id: &str) -> Result<Option<Track>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, album, duration_ms, year, genre, subgenre, created_at, updated_at
             FROM tracks WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![track_id], |row| {
            Ok(Track {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_ms: row.get(4)?,
                year: row.get(5)?,
                genre: row.get(6)?,
                subgenre: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_track_analysis_by_id(&self, track_id: &str) -> Result<Option<TrackAnalysis>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT track_id, bpm, bpm_confidence, key_name, camelot_key, key_confidence,
                    time_signature_num, time_signature_den, downbeat_offset_ms, phrase_length_bars,
                    overall_energy, dance_suitability, recommended_mix_in_ms, recommended_mix_out_ms,
                    analysis_version, analyzed_at, manual_override
             FROM track_analysis WHERE track_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![track_id], |row| {
            Ok(TrackAnalysis {
                track_id: row.get(0)?,
                bpm: row.get(1)?,
                bpm_confidence: row.get(2)?,
                key_name: row.get(3)?,
                camelot_key: row.get(4)?,
                key_confidence: row.get(5)?,
                time_signature_num: row.get(6)?,
                time_signature_den: row.get(7)?,
                downbeat_offset_ms: row.get(8)?,
                phrase_length_bars: row.get(9)?,
                overall_energy: row.get(10)?,
                dance_suitability: row.get(11)?,
                recommended_mix_in_ms: row.get(12)?,
                recommended_mix_out_ms: row.get(13)?,
                analysis_version: row.get(14)?,
                analyzed_at: row.get(15)?,
                manual_override: row.get(16)?,
            })
        })?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_asset_by_sha256(&self, hash: &str) -> Result<Option<AudioAsset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, track_id, file_path, file_format, file_size_bytes, sample_rate, bit_depth, channels, sha256_hash, verification_status, availability_status, last_verified_at
             FROM audio_assets WHERE sha256_hash = ?1"
        )?;
        let mut rows = stmt.query_map(params![hash], |row| {
            Ok(AudioAsset {
                id: row.get(0)?,
                track_id: row.get(1)?,
                file_path: row.get(2)?,
                file_format: row.get(3)?,
                file_size_bytes: row.get(4)?,
                sample_rate: row.get(5)?,
                bit_depth: row.get(6)?,
                channels: row.get(7)?,
                sha256_hash: row.get(8)?,
                verification_status: row.get(9)?,
                availability_status: row.get(10)?,
                last_verified_at: row.get(11)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_source_reference(&self, track_id: &str, source_type: &str, source_uri: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = format!("src_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO source_references (id, track_id, source_type, source_uri, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, track_id, source_type, source_uri, now],
        )?;
        Ok(())
    }

    pub fn update_session_dj_level(&self, session_id: &str, level: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE party_sessions SET dj_level = ?1 WHERE id = ?2",
            params![level, session_id],
        )?;
        Ok(())
    }

    pub fn update_session_mood(&self, session_id: &str, mood_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE party_sessions SET primary_mood_id = ?1 WHERE id = ?2",
            params![mood_id, session_id],
        )?;
        Ok(())
    }

    // --- Playback History ---
    pub fn log_playback(&self, session_id: &str, track_id: &str, deck_id: i32, duration_played_ms: i64, was_skipped: bool, transition_type: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO playback_history (session_id, track_id, deck_id, played_at, duration_played_ms, was_skipped, transition_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session_id, track_id, deck_id, now, duration_played_ms, was_skipped, transition_type],
        )?;
        conn.execute(
            "UPDATE party_sessions SET total_tracks_played = total_tracks_played + 1 WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn get_session_playback_history(&self, session_id: &str) -> Result<Vec<PlaybackHistoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT h.id, h.session_id, h.track_id, COALESCE(t.title, 'Unknown Track'), COALESCE(t.artist, 'Various Artists'),
                    h.deck_id, h.played_at, h.duration_played_ms, h.was_skipped, h.transition_type
             FROM playback_history h
             LEFT JOIN tracks t ON h.track_id = t.id
             WHERE h.session_id = ?1 ORDER BY h.played_at ASC, h.id ASC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            let id: i64 = row.get(0)?;
            let session_id: String = row.get(1)?;
            let track_id: String = row.get(2)?;
            let track_title: String = row.get(3)?;
            let track_artist: String = row.get(4)?;
            let deck_id: i64 = row.get(5)?;
            let played_at: i64 = row.get(6)?;
            let duration_played_ms: i64 = row.get(7)?;
            let was_skipped: bool = row.get::<_, i64>(8).map(|v| v != 0).unwrap_or(false);
            let transition_type: String = row.get(9)?;

            Ok(PlaybackHistoryItem {
                id,
                session_id,
                track_id,
                track_title,
                track_artist,
                deck_id: deck_id as i32,
                played_at,
                duration_played_ms,
                was_skipped,
                transition_type,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // --- Download Tasks ---
    pub fn create_download_task(&self, task: &DownloadTask) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO download_tasks (id, url, destination_path, expected_sha256, status, bytes_downloaded, total_bytes, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                task.id, task.url, task.destination_path, task.expected_sha256,
                task.status, task.bytes_downloaded, task.total_bytes, task.error_message,
                task.created_at, task.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn update_download_progress(&self, task_id: &str, bytes_downloaded: i64, total_bytes: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE download_tasks SET bytes_downloaded = ?1, total_bytes = COALESCE(?2, total_bytes), updated_at = ?3 WHERE id = ?4",
            params![bytes_downloaded, total_bytes, now, task_id],
        )?;
        Ok(())
    }

    pub fn update_download_status(&self, task_id: &str, status: &str, error_message: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE download_tasks SET status = ?1, error_message = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, error_message, now, task_id],
        )?;
        Ok(())
    }

    pub fn get_download_task(&self, task_id: &str) -> Result<Option<DownloadTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, destination_path, expected_sha256, status, bytes_downloaded, total_bytes, error_message, created_at, updated_at
             FROM download_tasks WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![task_id], |row| {
            Ok(DownloadTask {
                id: row.get(0)?,
                url: row.get(1)?,
                destination_path: row.get(2)?,
                expected_sha256: row.get(3)?,
                status: row.get(4)?,
                bytes_downloaded: row.get(5)?,
                total_bytes: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn list_download_tasks(&self, limit: usize) -> Result<Vec<DownloadTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, destination_path, expected_sha256, status, bytes_downloaded, total_bytes, error_message, created_at, updated_at
             FROM download_tasks ORDER BY created_at DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(DownloadTask {
                id: row.get(0)?,
                url: row.get(1)?,
                destination_path: row.get(2)?,
                expected_sha256: row.get(3)?,
                status: row.get(4)?,
                bytes_downloaded: row.get(5)?,
                total_bytes: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut tasks = Vec::new();
        for r in rows {
            tasks.push(r?);
        }
        Ok(tasks)
    }

    // --- Cache Manifest & Rolling Eviction ---
    pub fn upsert_cache_entry(&self, entry: &CacheEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO cache_manifest (asset_id, cached_file_path, size_bytes, is_locked, lock_reason, last_accessed_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(asset_id) DO UPDATE SET
                cached_file_path = excluded.cached_file_path,
                size_bytes = excluded.size_bytes,
                is_locked = excluded.is_locked,
                lock_reason = excluded.lock_reason,
                last_accessed_at = excluded.last_accessed_at",
            params![
                entry.asset_id, entry.cached_file_path, entry.size_bytes,
                if entry.is_locked { 1 } else { 0 }, entry.lock_reason,
                entry.last_accessed_at, entry.created_at
            ],
        )?;
        Ok(())
    }

    pub fn set_asset_cache_lock(&self, asset_id: &str, is_locked: bool, reason: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE cache_manifest SET is_locked = ?1, lock_reason = ?2, last_accessed_at = ?3 WHERE asset_id = ?4",
            params![if is_locked { 1 } else { 0 }, reason, now, asset_id],
        )?;
        let avail_status = if is_locked { "locked_playback" } else { "available" };
        let _ = conn.execute(
            "UPDATE audio_assets SET availability_status = ?1 WHERE id = ?2",
            params![avail_status, asset_id],
        );
        Ok(())
    }

    pub fn get_cache_entries(&self) -> Result<Vec<CacheEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT asset_id, cached_file_path, size_bytes, is_locked, lock_reason, last_accessed_at, created_at
             FROM cache_manifest ORDER BY last_accessed_at ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            let is_locked_val: i64 = row.get(3)?;
            Ok(CacheEntry {
                asset_id: row.get(0)?,
                cached_file_path: row.get(1)?,
                size_bytes: row.get(2)?,
                is_locked: is_locked_val != 0,
                lock_reason: row.get(4)?,
                last_accessed_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_cache_stats(&self, max_cache_bytes: i64) -> Result<CacheStats> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT 
                COUNT(*) as total_files,
                COALESCE(SUM(size_bytes), 0) as total_bytes,
                COALESCE(SUM(CASE WHEN is_locked = 1 THEN 1 ELSE 0 END), 0) as locked_files,
                COALESCE(SUM(CASE WHEN is_locked = 1 THEN size_bytes ELSE 0 END), 0) as locked_bytes
             FROM cache_manifest"
        )?;

        let stats = stmt.query_row([], |row| {
            let total_files: i64 = row.get(0)?;
            let total_bytes: i64 = row.get(1)?;
            let locked_files: i64 = row.get(2)?;
            let locked_bytes: i64 = row.get(3)?;

            Ok(CacheStats {
                total_files: total_files as usize,
                total_bytes,
                locked_files: locked_files as usize,
                locked_bytes,
                max_cache_bytes,
            })
        })?;

        Ok(stats)
    }

    /// Prunes unlocked LRU cache entries when cache size exceeds limit or when requested.
    /// Returns paths of files that should be deleted from the filesystem.
    pub fn prune_unlocked_cache_entries(&self, max_bytes: i64) -> Result<Vec<(String, String)>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let total_bytes: i64 = tx.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_manifest",
            [],
            |r| r.get(0),
        )?;

        if total_bytes <= max_bytes {
            return Ok(Vec::new());
        }

        let bytes_to_free = total_bytes - max_bytes;
        let mut freed_so_far = 0i64;
        let mut candidates_to_remove = Vec::new();

        {
            let mut stmt = tx.prepare(
                "SELECT asset_id, cached_file_path, size_bytes
                 FROM cache_manifest
                 WHERE is_locked = 0
                 ORDER BY last_accessed_at ASC"
            )?;

            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
            })?;

            for r in rows {
                let (asset_id, path, size) = r?;
                candidates_to_remove.push((asset_id, path));
                freed_so_far += size;
                if freed_so_far >= bytes_to_free {
                    break;
                }
            }
        }

        for (asset_id, _) in &candidates_to_remove {
            tx.execute("DELETE FROM cache_manifest WHERE asset_id = ?1", params![asset_id])?;
            tx.execute(
                "UPDATE audio_assets SET availability_status = 'offline' WHERE id = ?1",
                params![asset_id],
            )?;
        }

        tx.commit()?;
        Ok(candidates_to_remove)
    }

    // --- Sync Outbox ---
    pub fn enqueue_sync_item(&self, item: &SyncOutboxItem) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_outbox (id, entity_type, entity_id, payload_json, retry_count, status, last_attempt_at, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.id, item.entity_type, item.entity_id, item.payload_json,
                item.retry_count, item.status, item.last_attempt_at, item.error_message,
                item.created_at, item.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_pending_sync_items(&self, limit: usize) -> Result<Vec<SyncOutboxItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, entity_id, payload_json, retry_count, status, last_attempt_at, error_message, created_at, updated_at
             FROM sync_outbox
             WHERE status IN ('pending', 'failed') AND retry_count < 5
             ORDER BY retry_count ASC, created_at ASC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let retry_val: i64 = row.get(4)?;
            Ok(SyncOutboxItem {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                entity_id: row.get(2)?,
                payload_json: row.get(3)?,
                retry_count: retry_val as i32,
                status: row.get(5)?,
                last_attempt_at: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_sync_item_status(&self, id: &str, status: &str, error: Option<&str>, increment_retry: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let retry_inc = if increment_retry { 1 } else { 0 };
        conn.execute(
            "UPDATE sync_outbox
             SET status = ?1, last_attempt_at = ?2, error_message = ?3, retry_count = retry_count + ?4, updated_at = ?2
             WHERE id = ?5",
            params![status, now, error, retry_inc, id],
        )?;
        Ok(())
    }

    pub fn get_sync_overview(&self) -> Result<SyncStatusOverview> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT
                COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'in_flight' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'synced' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0)
             FROM sync_outbox"
        )?;

        let overview = stmt.query_row([], |row| {
            let p: i64 = row.get(0)?;
            let inflight: i64 = row.get(1)?;
            let s: i64 = row.get(2)?;
            let f: i64 = row.get(3)?;
            Ok(SyncStatusOverview {
                pending_count: p as usize,
                in_flight_count: inflight as usize,
                synced_count: s as usize,
                failed_count: f as usize,
            })
        })?;

        Ok(overview)
    }

    pub fn clear_all_asset_cache_locks(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE cache_manifest SET is_locked = 0, lock_reason = NULL", [])?;
        conn.execute("UPDATE audio_assets SET availability_status = 'available' WHERE availability_status = 'locked_playback'", [])?;
        Ok(())
    }

    pub fn recover_in_flight_sync_items(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let count = conn.execute(
            "UPDATE sync_outbox SET status = 'pending', updated_at = ?1 WHERE status = 'in_flight'",
            params![now],
        )?;
        Ok(count)
    }

    pub fn update_asset_verification_status(&self, asset_id: &str, status: &str, availability: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_assets SET verification_status = ?1, availability_status = ?2 WHERE id = ?3",
            params![status, availability, asset_id],
        )?;
        Ok(())
    }

    pub fn get_app_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn set_app_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }

    // ========================================================================
    // Cloud Party Operations (Phase 1)
    // ========================================================================

    pub fn create_cloud_party(&self, party: &CloudParty, config: &PartyConfiguration) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "INSERT INTO parties (id, party_name, status, host_token_hash, created_at, updated_at, started_at, ended_at, last_heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                party.id, party.party_name, party.status, party.host_token_hash,
                party.created_at, party.updated_at, party.started_at, party.ended_at, party.last_heartbeat_at
            ],
        )?;

        tx.execute(
            "INSERT INTO party_configurations (party_id, dj_level, primary_mood_id, secondary_mood_id, energy_progression, allow_guest_requests, max_requests_per_guest, request_cooldown_mins, require_host_approval, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                config.party_id, config.dj_level, config.primary_mood_id, config.secondary_mood_id,
                config.energy_progression, if config.allow_guest_requests { 1 } else { 0 },
                config.max_requests_per_guest, config.request_cooldown_mins,
                if config.require_host_approval { 1 } else { 0 }, config.updated_at
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_cloud_party(&self, party_id: &str) -> Result<Option<CloudParty>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, party_name, status, host_token_hash, created_at, updated_at, started_at, ended_at, last_heartbeat_at
             FROM parties WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![party_id], |row| {
            Ok(CloudParty {
                id: row.get(0)?,
                party_name: row.get(1)?,
                status: row.get(2)?,
                host_token_hash: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
                last_heartbeat_at: row.get(8)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_party_config(&self, party_id: &str) -> Result<Option<PartyConfiguration>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT party_id, dj_level, primary_mood_id, secondary_mood_id, energy_progression, allow_guest_requests, max_requests_per_guest, request_cooldown_mins, require_host_approval, updated_at
             FROM party_configurations WHERE party_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![party_id], |row| {
            let allow_req: i32 = row.get(5)?;
            let req_approval: i32 = row.get(8)?;
            Ok(PartyConfiguration {
                party_id: row.get(0)?,
                dj_level: row.get(1)?,
                primary_mood_id: row.get(2)?,
                secondary_mood_id: row.get(3)?,
                energy_progression: row.get(4)?,
                allow_guest_requests: allow_req != 0,
                max_requests_per_guest: row.get(6)?,
                request_cooldown_mins: row.get(7)?,
                require_host_approval: req_approval != 0,
                updated_at: row.get(9)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn update_cloud_party_status(&self, party_id: &str, status: &str, timestamp: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if status == "live" {
            conn.execute(
                "UPDATE parties SET status = ?1, updated_at = ?2, started_at = COALESCE(started_at, ?2), last_heartbeat_at = ?2 WHERE id = ?3",
                params![status, timestamp, party_id],
            )?;
        } else if status == "ended" {
            conn.execute(
                "UPDATE parties SET status = ?1, updated_at = ?2, ended_at = ?2 WHERE id = ?3",
                params![status, timestamp, party_id],
            )?;
        } else {
            conn.execute(
                "UPDATE parties SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, timestamp, party_id],
            )?;
        }
        Ok(())
    }

    pub fn update_party_heartbeat(&self, party_id: &str, timestamp: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE parties SET last_heartbeat_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![timestamp, party_id],
        )?;
        Ok(())
    }

    pub fn update_party_config(&self, config: &PartyConfiguration) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO party_configurations (party_id, dj_level, primary_mood_id, secondary_mood_id, energy_progression, allow_guest_requests, max_requests_per_guest, request_cooldown_mins, require_host_approval, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(party_id) DO UPDATE SET
                dj_level = excluded.dj_level,
                primary_mood_id = excluded.primary_mood_id,
                secondary_mood_id = excluded.secondary_mood_id,
                energy_progression = excluded.energy_progression,
                allow_guest_requests = excluded.allow_guest_requests,
                max_requests_per_guest = excluded.max_requests_per_guest,
                request_cooldown_mins = excluded.request_cooldown_mins,
                require_host_approval = excluded.require_host_approval,
                updated_at = excluded.updated_at",
            params![
                config.party_id, config.dj_level, config.primary_mood_id, config.secondary_mood_id,
                config.energy_progression, if config.allow_guest_requests { 1 } else { 0 },
                config.max_requests_per_guest, config.request_cooldown_mins,
                if config.require_host_approval { 1 } else { 0 }, config.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn insert_party_source(&self, source: &PartySourceReference) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO party_source_references (
                id, party_id, provider, original_url, provider_video_id, title, artist,
                thumbnail_url, duration_seconds, verification_status, playback_method,
                playability_status, metadata_status, error_reason, registered_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                thumbnail_url = excluded.thumbnail_url,
                duration_seconds = excluded.duration_seconds,
                verification_status = excluded.verification_status,
                playback_method = excluded.playback_method,
                playability_status = excluded.playability_status,
                metadata_status = excluded.metadata_status,
                error_reason = excluded.error_reason,
                updated_at = excluded.updated_at",
            params![
                source.id, source.party_id, source.provider, source.original_url, source.provider_video_id,
                source.title, source.artist, source.thumbnail_url, source.duration_seconds,
                source.verification_status, source.playback_method, source.playability_status,
                source.metadata_status, source.error_reason, source.registered_at, source.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_party_sources(&self, party_id: &str) -> Result<Vec<PartySourceReference>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, party_id, provider, original_url, provider_video_id, title, artist,
                    thumbnail_url, duration_seconds, verification_status, playback_method,
                    playability_status, metadata_status, error_reason, registered_at, updated_at
             FROM party_source_references WHERE party_id = ?1 ORDER BY registered_at ASC"
        )?;
        let rows = stmt.query_map(params![party_id], |row| {
            Ok(PartySourceReference {
                id: row.get(0)?,
                party_id: row.get(1)?,
                provider: row.get(2)?,
                original_url: row.get(3)?,
                provider_video_id: row.get(4)?,
                title: row.get(5)?,
                artist: row.get(6)?,
                thumbnail_url: row.get(7)?,
                duration_seconds: row.get(8)?,
                verification_status: row.get(9)?,
                playback_method: row.get(10)?,
                playability_status: row.get(11)?,
                metadata_status: row.get(12)?,
                error_reason: row.get(13)?,
                registered_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?;

        let mut sources = Vec::new();
        for r in rows {
            sources.push(r?);
        }
        Ok(sources)
    }

    pub fn get_party_source_by_video_id(&self, party_id: &str, video_id: &str) -> Result<Option<PartySourceReference>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, party_id, provider, original_url, provider_video_id, title, artist,
                    thumbnail_url, duration_seconds, verification_status, playback_method,
                    playability_status, metadata_status, error_reason, registered_at, updated_at
             FROM party_source_references WHERE party_id = ?1 AND provider_video_id = ?2"
        )?;
        let mut rows = stmt.query_map(params![party_id, video_id], |row| {
            Ok(PartySourceReference {
                id: row.get(0)?,
                party_id: row.get(1)?,
                provider: row.get(2)?,
                original_url: row.get(3)?,
                provider_video_id: row.get(4)?,
                title: row.get(5)?,
                artist: row.get(6)?,
                thumbnail_url: row.get(7)?,
                duration_seconds: row.get(8)?,
                verification_status: row.get(9)?,
                playback_method: row.get(10)?,
                playability_status: row.get(11)?,
                metadata_status: row.get(12)?,
                error_reason: row.get(13)?,
                registered_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn register_party_guest(&self, guest: &PartyGuest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO party_guests (id, party_id, nickname, fingerprint, token_hash, client_ip, request_count, is_blocked, created_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                guest.id, guest.party_id, guest.nickname, guest.fingerprint, guest.token_hash,
                guest.client_ip, guest.request_count, if guest.is_blocked { 1 } else { 0 },
                guest.created_at, guest.last_seen_at
            ],
        )?;
        Ok(())
    }

    pub fn lookup_party_guest_by_token(&self, party_id: &str, token_hash: &str) -> Result<Option<PartyGuest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, party_id, nickname, fingerprint, token_hash, client_ip, request_count, is_blocked, created_at, last_seen_at
             FROM party_guests WHERE party_id = ?1 AND token_hash = ?2"
        )?;
        let mut rows = stmt.query_map(params![party_id, token_hash], |row| {
            let blocked: i32 = row.get(7)?;
            Ok(PartyGuest {
                id: row.get(0)?,
                party_id: row.get(1)?,
                nickname: row.get(2)?,
                fingerprint: row.get(3)?,
                token_hash: row.get(4)?,
                client_ip: row.get(5)?,
                request_count: row.get(6)?,
                is_blocked: blocked != 0,
                created_at: row.get(8)?,
                last_seen_at: row.get(9)?,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn increment_party_guest_request_count(&self, guest_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE party_guests SET request_count = request_count + 1, last_seen_at = ?1 WHERE id = ?2",
            params![now, guest_id],
        )?;
        Ok(())
    }

    pub fn insert_party_song_request(&self, req: &PartySongRequest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO party_song_requests (id, party_id, guest_id, source_id, custom_note, status, rejection_reason, priority_score, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                req.id, req.party_id, req.guest_id, req.source_id, req.custom_note,
                req.status, req.rejection_reason, req.priority_score, req.created_at, req.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_party_song_requests(&self, party_id: &str) -> Result<Vec<PartySongRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT r.id, r.party_id, r.guest_id, g.nickname, r.source_id, s.title, s.artist, s.thumbnail_url,
                    r.custom_note, r.status, r.rejection_reason, r.priority_score, r.created_at, r.updated_at
             FROM party_song_requests r
             LEFT JOIN party_guests g ON r.guest_id = g.id
             LEFT JOIN party_source_references s ON r.source_id = s.id
             WHERE r.party_id = ?1
             ORDER BY r.priority_score DESC, r.created_at ASC"
        )?;
        let rows = stmt.query_map(params![party_id], |row| {
            Ok(PartySongRequest {
                id: row.get(0)?,
                party_id: row.get(1)?,
                guest_id: row.get(2)?,
                guest_name: row.get(3)?,
                source_id: row.get(4)?,
                source_title: row.get(5)?,
                source_artist: row.get(6)?,
                thumbnail_url: row.get(7)?,
                custom_note: row.get(8)?,
                status: row.get(9)?,
                rejection_reason: row.get(10)?,
                priority_score: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_party_request_status(
        &self,
        request_id: &str,
        party_id: &str,
        new_status: &str,
        reason: Option<&str>,
        actor_type: &str,
        actor_id: &str,
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let current_status: String = tx.query_row(
            "SELECT status FROM party_song_requests WHERE id = ?1 AND party_id = ?2",
            params![request_id, party_id],
            |row| row.get(0),
        )?;

        let now = chrono::Utc::now().timestamp_millis();
        tx.execute(
            "UPDATE party_song_requests SET status = ?1, rejection_reason = ?2, updated_at = ?3 WHERE id = ?4 AND party_id = ?5",
            params![new_status, reason, now, request_id, party_id],
        )?;

        let hist_id = format!("rh_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        tx.execute(
            "INSERT INTO request_status_history (id, request_id, from_status, to_status, actor_type, actor_id, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![hist_id, request_id, current_status, new_status, actor_type, actor_id, reason, now],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn log_playback_event(&self, party_id: &str, event_type: &str, track_id: Option<&str>, payload_json: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = format!("pbe_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO playback_events (id, party_id, event_type, track_id, payload_json, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, party_id, event_type, track_id, payload_json, now],
        )?;
        Ok(())
    }

    pub fn log_audit_event(&self, party_id: &str, action: &str, actor: &str, details_json: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = format!("aud_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO audit_events (id, party_id, action, actor, details_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, party_id, action, actor, details_json, now],
        )?;
        Ok(())
    }

    // ========================================================================
    // Local Party Helper Operations (Host Machine Only — Zero Cloud Parties)
    // ========================================================================

    pub fn create_local_party_room(&self, room: &LocalPartyRoom) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO local_party_rooms (
                room_id, room_code, party_name, status, dj_level, primary_mood_id, secondary_mood_id,
                host_token_hash, room_access_token_hash, created_at, started_at, closed_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                room.room_id, room.room_code, room.party_name, room.status, room.dj_level,
                room.primary_mood_id, room.secondary_mood_id, room.host_token_hash,
                room.room_access_token_hash, room.created_at, room.started_at, room.closed_at, room.updated_at
            ],
        )?;
        Ok(())
    }

    fn map_local_party_room(row: &rusqlite::Row) -> Result<LocalPartyRoom> {
        Ok(LocalPartyRoom {
            room_id: row.get(0)?,
            room_code: row.get(1)?,
            party_name: row.get(2)?,
            status: row.get(3)?,
            dj_level: row.get(4)?,
            primary_mood_id: row.get(5)?,
            secondary_mood_id: row.get(6)?,
            host_token_hash: row.get(7)?,
            room_access_token_hash: row.get(8)?,
            created_at: row.get(9)?,
            started_at: row.get(10)?,
            closed_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    }

    pub fn get_local_party_room(&self, room_id: &str) -> Result<Option<LocalPartyRoom>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT room_id, room_code, party_name, status, dj_level, primary_mood_id, secondary_mood_id,
                    host_token_hash, room_access_token_hash, created_at, started_at, closed_at, updated_at
             FROM local_party_rooms WHERE room_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![room_id], Self::map_local_party_room)?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_local_party_room_by_code(&self, room_code: &str) -> Result<Option<LocalPartyRoom>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT room_id, room_code, party_name, status, dj_level, primary_mood_id, secondary_mood_id,
                    host_token_hash, room_access_token_hash, created_at, started_at, closed_at, updated_at
             FROM local_party_rooms WHERE UPPER(room_code) = UPPER(?1)"
        )?;
        let mut rows = stmt.query_map(params![room_code], Self::map_local_party_room)?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn update_local_room_status(&self, room_id: &str, status: &str, ts: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if status == "live" {
            conn.execute(
                "UPDATE local_party_rooms SET status = 'live', started_at = COALESCE(started_at, ?1), updated_at = ?1 WHERE room_id = ?2",
                params![ts, room_id],
            )?;
        } else if status == "closed" {
            conn.execute(
                "UPDATE local_party_rooms SET status = 'closed', closed_at = ?1, updated_at = ?1 WHERE room_id = ?2",
                params![ts, room_id],
            )?;
        } else {
            conn.execute(
                "UPDATE local_party_rooms SET status = ?1, updated_at = ?2 WHERE room_id = ?3",
                params![status, ts, room_id],
            )?;
        }
        Ok(())
    }

    pub fn update_local_room_config(
        &self,
        room_id: &str,
        dj_level: i32,
        primary_mood_id: &str,
        secondary_mood_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE local_party_rooms SET dj_level = ?1, primary_mood_id = ?2, secondary_mood_id = ?3, updated_at = ?4 WHERE room_id = ?5",
            params![dj_level.clamp(1, 4), primary_mood_id, secondary_mood_id, now, room_id],
        )?;
        Ok(())
    }

    pub fn register_local_room_guest(&self, guest: &LocalRoomGuest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO local_room_guests (id, room_id, nickname, token_hash, client_ip, submission_count, created_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                guest.id, guest.room_id, guest.nickname, guest.token_hash,
                guest.client_ip, guest.submission_count, guest.created_at, guest.last_seen_at
            ],
        )?;
        Ok(())
    }

    pub fn lookup_local_room_guest(&self, room_id: &str, token_hash: &str) -> Result<Option<LocalRoomGuest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, room_id, nickname, token_hash, client_ip, submission_count, created_at, last_seen_at
             FROM local_room_guests WHERE room_id = ?1 AND token_hash = ?2"
        )?;
        let mut rows = stmt.query_map(params![room_id, token_hash], |row| {
            Ok(LocalRoomGuest {
                id: row.get(0)?,
                room_id: row.get(1)?,
                nickname: row.get(2)?,
                token_hash: row.get(3)?,
                client_ip: row.get(4)?,
                submission_count: row.get(5)?,
                created_at: row.get(6)?,
                last_seen_at: row.get(7)?,
            })
        })?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_local_room_guests(&self, room_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM local_room_guests WHERE room_id = ?1", params![room_id])?;
        Ok(deleted)
    }

    pub fn increment_local_guest_submission_count(&self, guest_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE local_room_guests SET submission_count = submission_count + 1, last_seen_at = ?1 WHERE id = ?2",
            params![now, guest_id],
        )?;
        Ok(())
    }

    pub fn insert_local_room_submission(&self, sub: &LocalRoomSubmission) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO local_room_submissions (
                id, room_id, submitted_by_type, submitted_by_id, submitted_by_name,
                provider, source_url, provider_video_id, song_id, title, artist, version_tag,
                thumbnail_url, duration_ms, bpm, half_time_bpm, double_time_bpm,
                musical_key, camelot_key, overall_energy, danceability,
                genre_tags_json, mood_tags_json, metadata_source, acquisition_status,
                acquisition_note, dj_status, dj_score, local_cache_path, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31
            )",
            params![
                sub.id, sub.room_id, sub.submitted_by_type, sub.submitted_by_id, sub.submitted_by_name,
                sub.provider, sub.source_url, sub.provider_video_id, sub.song_id, sub.title, sub.artist,
                sub.version_tag, sub.thumbnail_url, sub.duration_ms, sub.bpm, sub.half_time_bpm,
                sub.double_time_bpm, sub.musical_key, sub.camelot_key, sub.overall_energy,
                sub.danceability, sub.genre_tags_json, sub.mood_tags_json, sub.metadata_source,
                sub.acquisition_status, sub.acquisition_note, sub.dj_status, sub.dj_score,
                sub.local_cache_path, sub.created_at, sub.updated_at
            ],
        )?;
        Ok(())
    }

    fn map_local_room_submission(row: &rusqlite::Row) -> Result<LocalRoomSubmission> {
        Ok(LocalRoomSubmission {
            id: row.get(0)?,
            room_id: row.get(1)?,
            submitted_by_type: row.get(2)?,
            submitted_by_id: row.get(3)?,
            submitted_by_name: row.get(4)?,
            provider: row.get(5)?,
            source_url: row.get(6)?,
            provider_video_id: row.get(7)?,
            song_id: row.get(8)?,
            title: row.get(9)?,
            artist: row.get(10)?,
            version_tag: row.get(11)?,
            thumbnail_url: row.get(12)?,
            duration_ms: row.get(13)?,
            bpm: row.get(14)?,
            half_time_bpm: row.get(15)?,
            double_time_bpm: row.get(16)?,
            musical_key: row.get(17)?,
            camelot_key: row.get(18)?,
            overall_energy: row.get(19)?,
            danceability: row.get(20)?,
            genre_tags_json: row.get(21)?,
            mood_tags_json: row.get(22)?,
            metadata_source: row.get(23)?,
            acquisition_status: row.get(24)?,
            acquisition_note: row.get(25)?,
            dj_status: row.get(26)?,
            dj_score: row.get(27)?,
            local_cache_path: row.get(28)?,
            created_at: row.get(29)?,
            updated_at: row.get(30)?,
        })
    }

    pub fn get_local_room_submissions(&self, room_id: &str) -> Result<Vec<LocalRoomSubmission>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, room_id, submitted_by_type, submitted_by_id, submitted_by_name,
                    provider, source_url, provider_video_id, song_id, title, artist, version_tag,
                    thumbnail_url, duration_ms, bpm, half_time_bpm, double_time_bpm,
                    musical_key, camelot_key, overall_energy, danceability,
                    genre_tags_json, mood_tags_json, metadata_source, acquisition_status,
                    acquisition_note, dj_status, dj_score, local_cache_path, created_at, updated_at
             FROM local_room_submissions WHERE room_id = ?1 ORDER BY dj_score DESC, created_at ASC"
        )?;
        let rows = stmt.query_map(params![room_id], Self::map_local_room_submission)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Privacy-isolated query: returns ONLY submissions created by a specific guest in a specific room.
    pub fn get_guest_own_submissions(&self, room_id: &str, guest_id: &str) -> Result<Vec<GuestOwnSubmissionView>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, provider, dj_status, created_at
             FROM local_room_submissions
             WHERE room_id = ?1 AND submitted_by_type = 'guest' AND submitted_by_id = ?2
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![room_id, guest_id], |row| {
            let raw_status: String = row.get(4)?;
            let guest_safe_status = match raw_status.as_str() {
                "playing" => "playing",
                "played" => "played",
                "prepared" => "queued_next",
                _ => "received_by_dj",
            }
            .to_string();
            Ok(GuestOwnSubmissionView {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                provider: row.get(3)?,
                status: guest_safe_status,
                created_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn update_submission_dj_state(&self, sub_id: &str, dj_status: &str, dj_score: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE local_room_submissions SET dj_status = ?1, dj_score = ?2, updated_at = ?3 WHERE id = ?4",
            params![dj_status, dj_score, now, sub_id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Local Temporary Audio Cache Operations (10-15 Track Rolling Cap + Expiry)
    // ========================================================================

    pub fn upsert_local_temp_cache(&self, entry: &LocalTempCacheEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO local_temp_cache (
                id, room_id, submission_id, song_id, file_path, size_bytes,
                priority_tier, is_locked, created_at, last_accessed_at, expires_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                priority_tier = excluded.priority_tier,
                is_locked = excluded.is_locked,
                last_accessed_at = excluded.last_accessed_at,
                expires_at = excluded.expires_at",
            params![
                entry.id, entry.room_id, entry.submission_id, entry.song_id, entry.file_path,
                entry.size_bytes, entry.priority_tier, if entry.is_locked { 1 } else { 0 },
                entry.created_at, entry.last_accessed_at, entry.expires_at
            ],
        )?;
        Ok(())
    }

    pub fn update_local_temp_cache_priority(&self, cache_id: &str, priority_tier: i32, is_locked: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE local_temp_cache SET priority_tier = ?1, is_locked = ?2, last_accessed_at = ?3 WHERE id = ?4",
            params![priority_tier, if is_locked { 1 } else { 0 }, now, cache_id],
        )?;
        Ok(())
    }

    pub fn get_local_temp_cache_entry(&self, cache_id: &str) -> Result<Option<LocalTempCacheEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, room_id, submission_id, song_id, file_path, size_bytes, priority_tier, is_locked, created_at, last_accessed_at, expires_at
             FROM local_temp_cache WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![cache_id], |row| {
            let locked: i32 = row.get(7)?;
            Ok(LocalTempCacheEntry {
                id: row.get(0)?,
                room_id: row.get(1)?,
                submission_id: row.get(2)?,
                song_id: row.get(3)?,
                file_path: row.get(4)?,
                size_bytes: row.get(5)?,
                priority_tier: row.get(6)?,
                is_locked: locked != 0,
                created_at: row.get(8)?,
                last_accessed_at: row.get(9)?,
                expires_at: row.get(10)?,
            })
        })?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_local_temp_cache_stats(&self, max_tracks: usize) -> Result<LocalTempCacheStats> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0), SUM(CASE WHEN is_locked != 0 THEN 1 ELSE 0 END)
             FROM local_temp_cache"
        )?;
        let stats = stmt.query_row([], |row| {
            let count: i64 = row.get(0)?;
            let bytes: i64 = row.get(1)?;
            let locked: Option<i64> = row.get(2)?;
            Ok(LocalTempCacheStats {
                track_count: count as usize,
                max_tracks,
                total_bytes: bytes,
                locked_tracks: locked.unwrap_or(0) as usize,
            })
        })?;
        Ok(stats)
    }

    /// Evicts lowest-priority, oldest unlocked tracks whenever cached track count exceeds max_tracks (10-15).
    pub fn prune_local_temp_cache_over_cap(&self, max_tracks: usize) -> Result<Vec<(String, String)>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let total_count: usize = tx.query_row(
            "SELECT COUNT(*) FROM local_temp_cache",
            [],
            |row| row.get::<_, i64>(0).map(|c| c as usize),
        )?;

        if total_count <= max_tracks {
            return Ok(Vec::new());
        }

        let excess = total_count - max_tracks;
        let mut to_evict = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT id, file_path FROM local_temp_cache
                 WHERE is_locked = 0
                 ORDER BY priority_tier ASC, last_accessed_at ASC
                 LIMIT ?1"
            )?;
            let rows = stmt.query_map(params![excess as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for r in rows {
                to_evict.push(r?);
            }
        }

        for (id, _) in &to_evict {
            tx.execute("DELETE FROM local_temp_cache WHERE id = ?1", params![id])?;
        }

        tx.commit()?;
        Ok(to_evict)
    }

    pub fn delete_local_temp_cache_for_room(&self, room_id: &str) -> Result<Vec<(String, String)>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut entries = Vec::new();
        {
            let mut stmt = tx.prepare("SELECT id, file_path FROM local_temp_cache WHERE room_id = ?1")?;
            let rows = stmt.query_map(params![room_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for r in rows {
                entries.push(r?);
            }
        }
        tx.execute("DELETE FROM local_temp_cache WHERE room_id = ?1", params![room_id])?;
        tx.commit()?;
        Ok(entries)
    }

    pub fn delete_expired_local_temp_cache(&self, now_ms: i64) -> Result<Vec<(String, String)>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut entries = Vec::new();
        {
            let mut stmt = tx.prepare("SELECT id, file_path FROM local_temp_cache WHERE expires_at <= ?1")?;
            let rows = stmt.query_map(params![now_ms], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for r in rows {
                entries.push(r?);
            }
        }
        tx.execute("DELETE FROM local_temp_cache WHERE expires_at <= ?1", params![now_ms])?;
        tx.commit()?;
        Ok(entries)
    }

    // ========================================================================
    // Global Shared Song Metadata Catalogue Operations (Reusable Metadata Only)
    // ========================================================================

    fn load_global_song_with_urls(conn: &Connection, song_id: &str) -> Result<Option<GlobalSongMetadata>> {
        let mut stmt = conn.prepare(
            "SELECT song_id, canonical_fingerprint, title, artist, version_tag, duration_ms,
                    original_bpm, half_time_bpm, double_time_bpm, bpm_confidence,
                    musical_key, camelot_key, key_confidence, beat_grid_json, phrase_markers_json,
                    overall_energy, danceability, genre_tags_json, mood_tags_json,
                    analysis_version, analysis_confidence, processing_status, created_at, updated_at
             FROM global_song_catalogue WHERE song_id = ?1"
        )?;
        let mut rows = stmt.query_map(params![song_id], |row| {
            let genres_str: String = row.get(17)?;
            let moods_str: String = row.get(18)?;
            Ok(GlobalSongMetadata {
                song_id: row.get(0)?,
                canonical_fingerprint: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                version_tag: row.get(4)?,
                duration_ms: row.get(5)?,
                original_bpm: row.get(6)?,
                half_time_bpm: row.get(7)?,
                double_time_bpm: row.get(8)?,
                bpm_confidence: row.get(9)?,
                musical_key: row.get(10)?,
                camelot_key: row.get(11)?,
                key_confidence: row.get(12)?,
                beat_grid_json: row.get(13)?,
                phrase_markers_json: row.get(14)?,
                overall_energy: row.get(15)?,
                danceability: row.get(16)?,
                genre_tags: serde_json::from_str(&genres_str).unwrap_or_default(),
                mood_tags: serde_json::from_str(&moods_str).unwrap_or_default(),
                analysis_version: row.get(19)?,
                analysis_confidence: row.get(20)?,
                processing_status: row.get(21)?,
                source_urls: Vec::new(),
                created_at: row.get(22)?,
                updated_at: row.get(23)?,
            })
        })?;

        let mut song = match rows.next() {
            Some(r) => r?,
            None => return Ok(None),
        };

        let mut url_stmt = conn.prepare(
            "SELECT source_url FROM global_song_source_urls WHERE song_id = ?1 ORDER BY created_at ASC"
        )?;
        let url_rows = url_stmt.query_map(params![song_id], |row| row.get::<_, String>(0))?;
        for u in url_rows {
            song.source_urls.push(u?);
        }

        Ok(Some(song))
    }

    pub fn lookup_global_song_by_video_id(&self, video_id: &str) -> Result<Option<GlobalSongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let song_id_opt: Option<String> = {
            let mut stmt = conn.prepare(
                "SELECT song_id FROM global_song_source_urls WHERE provider_video_id = ?1 LIMIT 1"
            )?;
            let mut rows = stmt.query_map(params![video_id], |row| row.get(0))?;
            rows.next().transpose()?
        };
        if let Some(song_id) = song_id_opt {
            Self::load_global_song_with_urls(&conn, &song_id)
        } else {
            Ok(None)
        }
    }

    pub fn lookup_global_song_by_url(&self, source_url: &str) -> Result<Option<GlobalSongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let song_id_opt: Option<String> = {
            let mut stmt = conn.prepare(
                "SELECT song_id FROM global_song_source_urls WHERE source_url = ?1 LIMIT 1"
            )?;
            let mut rows = stmt.query_map(params![source_url], |row| row.get(0))?;
            rows.next().transpose()?
        };
        if let Some(song_id) = song_id_opt {
            Self::load_global_song_with_urls(&conn, &song_id)
        } else {
            Ok(None)
        }
    }

    pub fn lookup_global_song_by_fingerprint(&self, canonical_fingerprint: &str) -> Result<Option<GlobalSongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let song_id_opt: Option<String> = {
            let mut stmt = conn.prepare(
                "SELECT song_id FROM global_song_catalogue WHERE canonical_fingerprint = ?1 LIMIT 1"
            )?;
            let mut rows = stmt.query_map(params![canonical_fingerprint], |row| row.get(0))?;
            rows.next().transpose()?
        };
        if let Some(song_id) = song_id_opt {
            Self::load_global_song_with_urls(&conn, &song_id)
        } else {
            Ok(None)
        }
    }

    /// Deduplicates by provider_video_id or canonical_fingerprint while preserving distinct remixes/live/edits.
    /// Upgrades existing records when incoming analysis has higher version or confidence.
    pub fn upsert_global_song_metadata(
        &self,
        record: &GlobalSongMetadata,
        source_link: Option<(&str, &str, &str)>, // (source_url, provider, provider_video_id)
    ) -> Result<GlobalSongMetadata> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().timestamp_millis();

        // 1. Check if provider_video_id already maps to an existing song_id
        let existing_id_by_vid: Option<String> = if let Some((_, _, vid)) = source_link {
            let mut stmt = tx.prepare("SELECT song_id FROM global_song_source_urls WHERE provider_video_id = ?1 LIMIT 1")?;
            let mut rows = stmt.query_map(params![vid], |r| r.get(0))?;
            rows.next().transpose()?
        } else {
            None
        };

        // 2. Otherwise check if canonical_fingerprint (normalized title+artist+version_tag+duration bucket) matches
        let existing_id_by_fp: Option<String> = if existing_id_by_vid.is_none() {
            let mut stmt = tx.prepare("SELECT song_id FROM global_song_catalogue WHERE canonical_fingerprint = ?1 LIMIT 1")?;
            let mut rows = stmt.query_map(params![record.canonical_fingerprint], |r| r.get(0))?;
            rows.next().transpose()?
        } else {
            None
        };

        let target_song_id = existing_id_by_vid
            .or(existing_id_by_fp)
            .unwrap_or_else(|| record.song_id.clone());

        let genre_json = serde_json::to_string(&record.genre_tags).unwrap_or_else(|_| "[]".to_string());
        let mood_json = serde_json::to_string(&record.mood_tags).unwrap_or_else(|_| "[]".to_string());

        tx.execute(
            "INSERT INTO global_song_catalogue (
                song_id, canonical_fingerprint, title, artist, version_tag, duration_ms,
                original_bpm, half_time_bpm, double_time_bpm, bpm_confidence,
                musical_key, camelot_key, key_confidence, beat_grid_json, phrase_markers_json,
                overall_energy, danceability, genre_tags_json, mood_tags_json,
                analysis_version, analysis_confidence, processing_status, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
            )
            ON CONFLICT(song_id) DO UPDATE SET
                duration_ms = CASE WHEN excluded.duration_ms > 0 THEN excluded.duration_ms ELSE duration_ms END,
                original_bpm = COALESCE(excluded.original_bpm, original_bpm),
                half_time_bpm = COALESCE(excluded.half_time_bpm, half_time_bpm),
                double_time_bpm = COALESCE(excluded.double_time_bpm, double_time_bpm),
                bpm_confidence = COALESCE(excluded.bpm_confidence, bpm_confidence),
                musical_key = COALESCE(excluded.musical_key, musical_key),
                camelot_key = COALESCE(excluded.camelot_key, camelot_key),
                key_confidence = COALESCE(excluded.key_confidence, key_confidence),
                beat_grid_json = COALESCE(excluded.beat_grid_json, beat_grid_json),
                phrase_markers_json = COALESCE(excluded.phrase_markers_json, phrase_markers_json),
                overall_energy = COALESCE(excluded.overall_energy, overall_energy),
                danceability = COALESCE(excluded.danceability, danceability),
                genre_tags_json = CASE WHEN excluded.genre_tags_json != '[]' THEN excluded.genre_tags_json ELSE genre_tags_json END,
                mood_tags_json = CASE WHEN excluded.mood_tags_json != '[]' THEN excluded.mood_tags_json ELSE mood_tags_json END,
                analysis_version = MAX(analysis_version, excluded.analysis_version),
                analysis_confidence = MAX(analysis_confidence, excluded.analysis_confidence),
                processing_status = CASE WHEN excluded.processing_status = 'analyzed' OR excluded.processing_status = 'verified' THEN excluded.processing_status ELSE processing_status END,
                updated_at = excluded.updated_at",
            params![
                target_song_id, record.canonical_fingerprint, record.title, record.artist, record.version_tag,
                record.duration_ms, record.original_bpm, record.half_time_bpm, record.double_time_bpm,
                record.bpm_confidence, record.musical_key, record.camelot_key, record.key_confidence,
                record.beat_grid_json, record.phrase_markers_json, record.overall_energy, record.danceability,
                genre_json, mood_json, record.analysis_version, record.analysis_confidence,
                record.processing_status, record.created_at, now
            ],
        )?;

        if let Some((source_url, provider, vid)) = source_link {
            tx.execute(
                "INSERT OR IGNORE INTO global_song_source_urls (source_url, provider, provider_video_id, song_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![source_url, provider, vid, target_song_id, now],
            )?;
        }

        tx.commit()?;
        Ok(Self::load_global_song_with_urls(&conn, &target_song_id)?.unwrap())
    }
}

