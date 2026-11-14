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
        for r in rows {
            candidates.push(r?);
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
}

