#[cfg(test)]
mod tests {
    use dhun_lib::audio::dsp::crossfader::{CrossfaderCurve, CrossfaderUnit};
    use dhun_lib::audio::dsp::limiter::LookaheadLimiter;
    use dhun_lib::db::Database;
    use dhun_lib::db::models::*;
    use dhun_lib::dj::selector::{calculate_bpm_compatibility, calculate_camelot_compatibility};
    use dhun_lib::network::rate_limit::RateLimiter;

    #[test]
    fn test_bpm_compatibility() {
        // Identical BPM
        assert_eq!(calculate_bpm_compatibility(128.0, 128.0), 1.0);
        
        // Within 2% (e.g. 128 vs 129)
        assert!(calculate_bpm_compatibility(128.0, 129.0) >= 0.99);

        // Within 5% (128 vs 134)
        assert!(calculate_bpm_compatibility(128.0, 134.0) >= 0.85);

        // Half-time / Double-time harmonic relationship (70 vs 140)
        assert_eq!(calculate_bpm_compatibility(70.0, 140.0), 0.90);
        assert_eq!(calculate_bpm_compatibility(140.0, 70.0), 0.90);

        // Clashing BPM (e.g. 90 vs 135)
        assert!(calculate_bpm_compatibility(90.0, 135.0) < 0.60);
    }

    #[test]
    fn test_camelot_key_compatibility() {
        // Same key
        assert_eq!(calculate_camelot_compatibility("8B", "8B"), 1.0);

        // Relative major / minor swap (8B to 8A)
        assert_eq!(calculate_camelot_compatibility("8B", "8A"), 0.90);
        assert_eq!(calculate_camelot_compatibility("11A", "11B"), 0.90);

        // Adjacent keys on Camelot wheel (+1 or -1 hour)
        assert_eq!(calculate_camelot_compatibility("8B", "9B"), 0.90);
        assert_eq!(calculate_camelot_compatibility("8B", "7B"), 0.90);

        // Energy jump (+2 hours)
        assert_eq!(calculate_camelot_compatibility("8B", "10B"), 0.75);

        // Clashing key (8B vs 2A)
        assert_eq!(calculate_camelot_compatibility("8B", "2A"), 0.35);
    }

    #[test]
    fn test_crossfader_and_smart_bass_swap() {
        let mut cf = CrossfaderUnit::new();
        
        // Equal Power at center
        cf.set_position(0.0);
        cf.set_curve(CrossfaderCurve::EqualPower);
        let (ga, gb) = cf.gains();
        // ga^2 + gb^2 should be close to 1.0
        assert!((ga * ga + gb * gb - 1.0).abs() < 0.05);

        // Smart Bass Swap curve
        cf.set_curve(CrossfaderCurve::BassSwap);
        
        // Deck A dominant (-0.8)
        cf.set_position(-0.8);
        let (bass_a, bass_b) = cf.bass_attenuations_db();
        assert_eq!(bass_a, 0.0);
        assert_eq!(bass_b, -36.0); // Deck B bass completely killed

        // Deck B dominant (+0.8)
        cf.set_position(0.8);
        let (bass_a_r, bass_b_r) = cf.bass_attenuations_db();
        assert_eq!(bass_a_r, -36.0); // Deck A bass completely killed
        assert_eq!(bass_b_r, 0.0);
    }

    #[test]
    fn test_master_limiter_brickwall_ceiling() {
        let mut limiter = LookaheadLimiter::new(44100.0, 1.0, -0.1);

        // Feed massive signals (> +12 dBFS = 4.0)
        for _ in 0..1000 {
            let (out_l, out_r) = limiter.process_sample(4.0, 4.0);
            assert!(out_l <= 0.995);
            assert!(out_r <= 0.995);
            assert!(out_l >= -0.995);
            assert!(out_r >= -0.995);
        }
    }

    #[test]
    fn test_sqlite_in_memory_migrations_and_operations() {
        let db = Database::new_in_memory().expect("Failed to initialize in-memory DB");

        // Verify mood seed data exists
        let moods = db.get_mood_profiles().expect("Failed to get mood profiles");
        assert!(moods.len() >= 10);
        assert!(moods.iter().any(|m| m.id == "shaadi"));
        assert!(moods.iter().any(|m| m.id == "baraat"));
        assert!(moods.iter().any(|m| m.id == "garba"));

        // Insert track, asset and analysis
        let track = Track {
            id: "trk_test_01".to_string(),
            title: "Test Antakshari".to_string(),
            artist: "Test Artist".to_string(),
            album: None,
            duration_ms: 240000,
            year: Some(2024),
            genre: "Bollywood".to_string(),
            subgenre: None,
            created_at: 1000,
            updated_at: 1000,
        };

        let temp_file = std::env::temp_dir().join(format!("dhun_test_antakshari_{}.mp3", uuid::Uuid::new_v4()));
        std::fs::write(&temp_file, b"DUMMY_MP3_PAYLOAD").unwrap();

        let asset = AudioAsset {
            id: "ast_test_01".to_string(),
            track_id: "trk_test_01".to_string(),
            file_path: temp_file.to_string_lossy().to_string(),
            file_format: "mp3".to_string(),
            file_size_bytes: 5000000,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: "dummyhash".to_string(),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 1000,
        };

        let analysis = TrackAnalysis {
            track_id: "trk_test_01".to_string(),
            bpm: 128.0,
            bpm_confidence: 0.95,
            key_name: "C Major".to_string(),
            camelot_key: "8B".to_string(),
            key_confidence: 0.92,
            time_signature_num: 4,
            time_signature_den: 4,
            downbeat_offset_ms: 100,
            phrase_length_bars: 32,
            overall_energy: 0.85,
            dance_suitability: 0.90,
            recommended_mix_in_ms: 15000,
            recommended_mix_out_ms: 220000,
            analysis_version: 1,
            analyzed_at: 1000,
            manual_override: false,
        };

        db.insert_or_update_track(&track, &asset, &analysis).expect("Failed inserting track");

        let all_tracks = db.get_all_tracks().expect("Failed getting all tracks");
        assert!(all_tracks.len() >= 10);
        assert!(all_tracks.iter().any(|p| p.title == "Gallan Goodiyaan"));

        let playable = db.get_playable_candidates().expect("Failed getting playable");
        assert!(playable.iter().any(|p| p.track.title == "Test Antakshari"));
        // Strict verification: demo tracks MUST NOT appear in playable candidates
        assert!(!playable.iter().any(|p| p.asset.file_path.starts_with("demo://")));

        let _ = std::fs::remove_file(&temp_file);

        // Create party session
        let session = PartySession {
            id: "ses_test".to_string(),
            party_name: "Test Sangeet".to_string(),
            start_time: 1000,
            end_time: None,
            dj_level: 2,
            primary_mood_id: "shaadi".to_string(),
            secondary_mood_id: None,
            output_device_name: "Default".to_string(),
            status: "active".to_string(),
            total_tracks_played: 0,
            config_json: None,
        };
        db.create_session(&session).expect("Failed creating session");

        // Insert guest request
        let req = GuestRequest {
            id: "req_test_01".to_string(),
            session_id: "ses_test".to_string(),
            guest_id: "gst_01".to_string(),
            guest_name: Some("Aman".to_string()),
            track_id: "trk_test_01".to_string(),
            track_title: Some("Test Antakshari".to_string()),
            track_artist: Some("Test Artist".to_string()),
            submitted_at: 1050,
            status: "pending".to_string(),
            priority_order: 100,
            host_note: None,
            reviewed_at: None,
            scheduled_slot: None,
        };
        db.insert_guest_request(&req).expect("Failed inserting request");

        let requests = db.get_session_requests("ses_test").expect("Failed getting requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].status, "pending");

        // Host approves request
        db.update_request_status("req_test_01", "approved", Some(10)).expect("Failed approving request");
        let updated_requests = db.get_session_requests("ses_test").expect("Failed getting requests");
        assert_eq!(updated_requests[0].status, "approved");
        assert_eq!(updated_requests[0].priority_order, 10);
    }

    #[test]
    fn test_guest_rate_limiter() {
        let limiter = RateLimiter::new(3, 60); // 3 requests per 60 seconds

        assert!(limiter.check_and_record("guest_ip_1"));
        assert!(limiter.check_and_record("guest_ip_1"));
        assert!(limiter.check_and_record("guest_ip_1"));
        
        // 4th request must be blocked
        assert!(!limiter.check_and_record("guest_ip_1"));

        // Different guest is not blocked
        assert!(limiter.check_and_record("guest_ip_2"));
    }

    #[test]
    fn test_all_14_preset_moods_exist_in_rust() {
        use dhun_lib::dj::moods::get_preset_moods;
        let presets = get_preset_moods();
        assert_eq!(presets.len(), 14, "Expected exactly 14 complete preset moods in Rust backend");

        let expected_mood_ids = [
            "shaadi", "baraat", "mehendi", "sangeet", "desi_club",
            "garba", "dandiya", "retro_bollywood", "house_party",
            "haldi", "punjabi_tashan", "cocktail_night", "festival_mode", "international_party"
        ];

        for id in &expected_mood_ids {
            let found = presets.iter().any(|m| m.id == *id);
            assert!(found, "Missing preset mood: {}", id);
        }
    }

    #[test]
    fn test_key_detector_frequency_domain_analysis() {
        use std::sync::Arc;
        use dhun_lib::analysis::key_detector::KeyDetector;

        // Generate 3 seconds of 440 Hz (Note A4) tone at 44100 Hz stereo
        let sr = 44100;
        let num_frames = sr * 3;
        let mut samples = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f32 / sr as f32;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            samples.push(val); // L
            samples.push(val); // R
        }

        let (key_name, camelot_key, confidence) = KeyDetector::detect_key(&Arc::new(samples), sr as u32);
        assert!(confidence > 0.4);
        assert!(!key_name.is_empty());
        assert!(!camelot_key.is_empty());
        // Should detect A Major or A Minor or closely related harmonic
        assert!(key_name.contains('A') || camelot_key == "11B" || camelot_key == "8A");
    }

    #[test]
    fn test_track_selector_excludes_reserved_and_recent_history() {
        use dhun_lib::dj::moods::get_preset_moods;
        use dhun_lib::dj::selector::TrackSelector;

        let preset_moods = get_preset_moods();
        let mood = &preset_moods[0]; // Shaadi

        let make_candidate = |id: &str, title: &str, artist: &str, bpm: f64, camelot: &str| PlayableTrackCandidate {
            track: Track {
                id: id.to_string(),
                title: title.to_string(),
                artist: artist.to_string(),
                album: None,
                duration_ms: 200000,
                year: None,
                genre: "Bollywood".to_string(),
                subgenre: None,
                created_at: 0,
                updated_at: 0,
            },
            asset: AudioAsset {
                id: format!("ast_{}", id),
                track_id: id.to_string(),
                file_path: format!("C:/{}.mp3", id),
                file_format: "mp3".to_string(),
                file_size_bytes: 1000,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: "hash".to_string(),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: 0,
            },
            analysis: TrackAnalysis {
                track_id: id.to_string(),
                bpm,
                bpm_confidence: 0.9,
                key_name: "C Major".to_string(),
                camelot_key: camelot.to_string(),
                key_confidence: 0.9,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 0,
                phrase_length_bars: 32,
                overall_energy: 0.8,
                dance_suitability: 0.9,
                recommended_mix_in_ms: 0,
                recommended_mix_out_ms: 180000,
                analysis_version: 1,
                analyzed_at: 0,
                manual_override: false,
            },
            compatibility_score: 0.0,
            transition_style: "bass_swap".to_string(),
        };

        let t1 = make_candidate("t1", "Song 1", "Artist A", 128.0, "8B");
        let t2 = make_candidate("t2", "Song 2", "Artist B", 128.0, "8B");
        let t3 = make_candidate("t3", "Song 3", "Artist C", 128.0, "9B");
        let candidates = vec![t1.clone(), t2.clone(), t3.clone()];

        // When t1 is excluded (e.g. already playing), t1 must NEVER be selected
        let excluded = vec!["t1".to_string()];
        let selected = TrackSelector::score_and_select(
            &candidates,
            Some(&t1),
            mood,
            None,
            0.8,
            &[],
            &[],
            &excluded,
        );

        assert!(selected.is_some());
        assert_ne!(selected.unwrap().track.id, "t1", "Excluded track t1 must not be selected");
    }

    #[test]
    fn test_continuous_crossfader_ramping() {
        let mut cf = CrossfaderUnit::new();
        cf.set_position(-1.0); // Side A (Deck 0 / 2)
        assert_eq!(cf.position(), -1.0);

        // Start a 1000-frame ramp toward Side B (+1.0)
        // duration_ms: 100, sample_rate: 10000 => 1000 frames
        cf.start_ramp(1.0, 100, 10000, CrossfaderCurve::Linear);

        // After 500 frames, should be at center (0.0)
        for _ in 0..500 {
            cf.tick_frame();
        }
        assert!((cf.position() - 0.0).abs() < 0.01, "Expected ~0.0 at halfway, got {}", cf.position());

        // After 500 more frames, should reach 1.0
        for _ in 0..500 {
            cf.tick_frame();
        }
        assert!((cf.position() - 1.0).abs() < 0.001, "Expected 1.0 at completion, got {}", cf.position());

        // Further ticks should remain pegged at target
        cf.tick_frame();
        assert_eq!(cf.position(), 1.0);
    }

    #[test]
    fn test_session_dj_level_and_mood_updates() {
        let db = Database::new_in_memory().expect("Failed to initialize in-memory DB");
        let session = PartySession {
            id: "ses_update_test".to_string(),
            party_name: "Baraat Blast".to_string(),
            start_time: 1000,
            end_time: None,
            dj_level: 2,
            primary_mood_id: "shaadi".to_string(),
            secondary_mood_id: None,
            output_device_name: "Default".to_string(),
            status: "active".to_string(),
            total_tracks_played: 0,
            config_json: None,
        };
        db.create_session(&session).expect("Failed creating session");

        // Update DJ level to 4
        db.update_session_dj_level("ses_update_test", 4).expect("Failed updating dj level");

        // Update mood to "punjabi_tashan"
        db.update_session_mood("ses_update_test", "punjabi_tashan").expect("Failed updating mood");

        let recent = db.get_recent_sessions(1).expect("Failed to get recent sessions");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].dj_level, 4);
        assert_eq!(recent[0].primary_mood_id, "punjabi_tashan");
    }

    #[test]
    fn test_approved_request_prioritization_and_status_filter() {
        use dhun_lib::dj::moods::get_preset_moods;
        use dhun_lib::dj::selector::TrackSelector;

        let moods = get_preset_moods();
        let mood = moods.iter().find(|m| m.id == "house_party").unwrap();

        let make_candidate = |id: &str, title: &str, bpm: f64| PlayableTrackCandidate {
            track: Track {
                id: id.to_string(),
                title: title.to_string(),
                artist: "Artist".to_string(),
                album: None,
                duration_ms: 200000,
                year: None,
                genre: "Bollywood".to_string(),
                subgenre: None,
                created_at: 0,
                updated_at: 0,
            },
            asset: AudioAsset {
                id: format!("ast_{}", id),
                track_id: id.to_string(),
                file_path: format!("C:/{}.mp3", id),
                file_format: "mp3".to_string(),
                file_size_bytes: 1000,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: "hash".to_string(),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: 0,
            },
            analysis: TrackAnalysis {
                track_id: id.to_string(),
                bpm,
                bpm_confidence: 0.9,
                key_name: "C Major".to_string(),
                camelot_key: "8B".to_string(),
                key_confidence: 0.9,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 0,
                phrase_length_bars: 32,
                overall_energy: 0.75,
                dance_suitability: 0.8,
                recommended_mix_in_ms: 0,
                recommended_mix_out_ms: 180000,
                analysis_version: 1,
                analyzed_at: 0,
                manual_override: false,
            },
            compatibility_score: 0.0,
            transition_style: "bass_swap".to_string(),
        };

        let t1 = make_candidate("t1", "Track 1", 128.0);
        let t2 = make_candidate("t2", "Track 2", 128.0);
        let candidates = vec![t1.clone(), t2.clone()];

        // Request for t2 with 'pending' status — must NOT boost t2 over t1
        let pending_req = vec![GuestRequest {
            id: "req_pending".to_string(),
            session_id: "s1".to_string(),
            guest_id: "g1".to_string(),
            guest_name: None,
            track_id: "t2".to_string(),
            track_title: Some("Track 2".to_string()),
            track_artist: None,
            submitted_at: 0,
            status: "pending".to_string(),
            priority_order: 100,
            host_note: None,
            reviewed_at: None,
            scheduled_slot: None,
        }];

        let selected_pending = TrackSelector::score_and_select(
            &candidates,
            None,
            mood,
            None,
            0.75,
            &[],
            &pending_req,
            &[],
        );
        assert!(selected_pending.is_some());

        // Approved request for t2 — MUST strictly prioritize t2
        let approved_req = vec![GuestRequest {
            id: "req_approved".to_string(),
            session_id: "s1".to_string(),
            guest_id: "g1".to_string(),
            guest_name: None,
            track_id: "t2".to_string(),
            track_title: Some("Track 2".to_string()),
            track_artist: None,
            submitted_at: 0,
            status: "approved".to_string(),
            priority_order: 1, // High priority host approved
            host_note: None,
            reviewed_at: None,
            scheduled_slot: None,
        }];

        let selected_approved = TrackSelector::score_and_select(
            &candidates,
            None,
            mood,
            None,
            0.75,
            &[],
            &approved_req,
            &[],
        );
        assert!(selected_approved.is_some());
        assert_eq!(selected_approved.unwrap().track.id, "t2", "Approved guest request must be prioritized");
    }

    #[test]
    fn test_persistent_playback_history_restoration() {
        let db = Database::new_in_memory().expect("Failed in-memory DB");
        let session = PartySession {
            id: "ses_history_test".to_string(),
            party_name: "Test Party".to_string(),
            start_time: 1000,
            end_time: None,
            dj_level: 2,
            primary_mood_id: "shaadi".to_string(),
            secondary_mood_id: None,
            output_device_name: "Default".to_string(),
            status: "active".to_string(),
            total_tracks_played: 2,
            config_json: None,
        };
        db.create_session(&session).unwrap();

        let t1 = Track {
            id: "trk_hist_1".to_string(),
            title: "Song One".to_string(),
            artist: "Artist One".to_string(),
            album: None,
            duration_ms: 180000,
            year: None,
            genre: "Bollywood".to_string(),
            subgenre: None,
            created_at: 0,
            updated_at: 0,
        };
        let t2 = Track {
            id: "trk_hist_2".to_string(),
            title: "Song Two".to_string(),
            artist: "Artist Two".to_string(),
            album: None,
            duration_ms: 190000,
            year: None,
            genre: "Bollywood".to_string(),
            subgenre: None,
            created_at: 0,
            updated_at: 0,
        };
        let asset1 = AudioAsset {
            id: "ast_h1".to_string(),
            track_id: "trk_hist_1".to_string(),
            file_path: "/music/1.mp3".to_string(),
            file_format: "mp3".to_string(),
            file_size_bytes: 1000,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: "h1".to_string(),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 0,
        };
        let asset2 = AudioAsset {
            id: "ast_h2".to_string(),
            track_id: "trk_hist_2".to_string(),
            file_path: "/music/2.mp3".to_string(),
            file_format: "mp3".to_string(),
            file_size_bytes: 1000,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: "h2".to_string(),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 0,
        };
        let analysis1 = TrackAnalysis {
            track_id: "trk_hist_1".to_string(),
            bpm: 128.0,
            bpm_confidence: 0.9,
            key_name: "C".to_string(),
            camelot_key: "8B".to_string(),
            key_confidence: 0.9,
            time_signature_num: 4,
            time_signature_den: 4,
            downbeat_offset_ms: 0,
            phrase_length_bars: 32,
            overall_energy: 0.8,
            dance_suitability: 0.8,
            recommended_mix_in_ms: 0,
            recommended_mix_out_ms: 180000,
            analysis_version: 1,
            analyzed_at: 0,
            manual_override: false,
        };
        let analysis2 = TrackAnalysis {
            track_id: "trk_hist_2".to_string(),
            bpm: 128.0,
            bpm_confidence: 0.9,
            key_name: "C".to_string(),
            camelot_key: "8B".to_string(),
            key_confidence: 0.9,
            time_signature_num: 4,
            time_signature_den: 4,
            downbeat_offset_ms: 0,
            phrase_length_bars: 32,
            overall_energy: 0.8,
            dance_suitability: 0.8,
            recommended_mix_in_ms: 0,
            recommended_mix_out_ms: 180000,
            analysis_version: 1,
            analyzed_at: 0,
            manual_override: false,
        };

        db.insert_or_update_track(&t1, &asset1, &analysis1).unwrap();
        db.insert_or_update_track(&t2, &asset2, &analysis2).unwrap();

        // Log 2 tracks played
        db.log_playback("ses_history_test", "trk_hist_1", 0, 180000, false, "phrase_swap").unwrap();
        db.log_playback("ses_history_test", "trk_hist_2", 1, 190000, false, "phrase_swap").unwrap();

        // Restore scheduler with new_with_db
        use dhun_lib::dj::scheduler::DjScheduler;
        let sched = DjScheduler::new_with_db(session, &db);
        assert_eq!(sched.playback_history.len(), 2);
        assert!(sched.playback_history.iter().any(|h| h.track_id == "trk_hist_1"));
        assert!(sched.playback_history.iter().any(|h| h.track_id == "trk_hist_2"));
    }

    #[test]
    fn test_dj_levels_deck_limits_and_rotation() {
        use dhun_lib::dj::levels::DjLevel;

        let lvl1 = DjLevel::from_i32(1);
        let lvl2 = DjLevel::from_i32(2);
        let lvl3 = DjLevel::from_i32(3);
        let lvl4 = DjLevel::from_i32(4);

        assert_eq!(lvl1.max_decks(), 2);
        assert_eq!(lvl2.max_decks(), 2);
        assert_eq!(lvl3.max_decks(), 4);
        assert_eq!(lvl4.max_decks(), 4);

        assert!(!lvl1.supports_auto_sync());
        assert!(lvl2.supports_auto_sync());
        assert!(lvl3.supports_performance_pads());
        assert!(lvl4.supports_creative_mashup());
    }

    #[test]
    fn test_hot_recovery_audio_engine() {
        use dhun_lib::audio::AudioEngine;
        use std::sync::atomic::Ordering;

        let mut dummy = AudioEngine::new_dummy();
        // Simulate a hardware disconnect event
        dummy.has_device_error.store(true, Ordering::SeqCst);

        // check_and_recover_stream catches error and resets without panicking
        let recovered = dummy.check_and_recover_stream();
        assert!(recovered.is_ok());
        assert_eq!(dummy.has_device_error.load(Ordering::SeqCst), false);
    }

    #[test]
    fn test_schema_migration_v2_and_outbox_tables() {
        let db = Database::new_in_memory().expect("Failed to initialize in-memory DB");

        // Verify download_tasks table CRUD
        let now = chrono::Utc::now().timestamp_millis();
        let task = DownloadTask {
            id: "dl_test_1".to_string(),
            url: "https://example.com/audio/test_track.mp3".to_string(),
            destination_path: "/tmp/cache/test_track.mp3".to_string(),
            expected_sha256: Some("abcd1234ef5678".to_string()),
            status: "pending".to_string(),
            bytes_downloaded: 0,
            total_bytes: Some(10485760),
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.create_download_task(&task).expect("Failed to create download task");
        db.update_download_progress("dl_test_1", 5242880, Some(10485760)).expect("Failed to update download progress");
        db.update_download_status("dl_test_1", "downloading", None).expect("Failed to update status");

        let fetched = db.get_download_task("dl_test_1").expect("Query failed").expect("Task not found");
        assert_eq!(fetched.status, "downloading");
        assert_eq!(fetched.bytes_downloaded, 5242880);
        assert_eq!(fetched.total_bytes, Some(10485760));

        let tasks = db.list_download_tasks(10).expect("Failed to list tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "dl_test_1");
    }

    #[test]
    fn test_managed_rolling_cache_eviction_and_locking() {
        use dhun_lib::library::cache::RollingCacheManager;
        use std::fs;

        let db = Database::new_in_memory().expect("Failed to initialize DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_test_cache_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let file1_path = temp_dir.join("song1.mp3");
        let file2_path = temp_dir.join("song2.mp3");
        let file3_path = temp_dir.join("song3.mp3");

        // Create 3 dummy files of size 100 bytes each
        fs::write(&file1_path, vec![0u8; 100]).unwrap();
        fs::write(&file2_path, vec![0u8; 100]).unwrap();
        fs::write(&file3_path, vec![0u8; 100]).unwrap();

        let now = chrono::Utc::now().timestamp_millis();

        // Seed 3 tracks and audio_assets in DB
        for i in 1..=3 {
            let trk_id = format!("trk_cache_{}", i);
            let ast_id = format!("ast_cache_{}", i);
            let path_str = match i {
                1 => file1_path.to_string_lossy().to_string(),
                2 => file2_path.to_string_lossy().to_string(),
                _ => file3_path.to_string_lossy().to_string(),
            };

            let track = Track {
                id: trk_id.clone(),
                title: format!("Test Song {}", i),
                artist: "Test Artist".to_string(),
                album: None,
                duration_ms: 180000,
                year: Some(2024),
                genre: "Bollywood Dance".to_string(),
                subgenre: None,
                created_at: now,
                updated_at: now,
            };

            let asset = AudioAsset {
                id: ast_id.clone(),
                track_id: trk_id.clone(),
                file_path: path_str.clone(),
                file_format: "mp3".to_string(),
                file_size_bytes: 100,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: format!("hash_{}", i),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: now,
            };

            let analysis = TrackAnalysis {
                track_id: trk_id,
                bpm: 128.0,
                bpm_confidence: 0.95,
                key_name: "8B".to_string(),
                camelot_key: "8B".to_string(),
                key_confidence: 0.95,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 0,
                phrase_length_bars: 32,
                overall_energy: 0.8,
                dance_suitability: 0.8,
                recommended_mix_in_ms: 10000,
                recommended_mix_out_ms: 170000,
                analysis_version: 1,
                analyzed_at: now,
                manual_override: false,
            };

            db.insert_or_update_track(&track, &asset, &analysis).unwrap();

            // Insert into cache manifest with staggered last_accessed_at
            let entry = CacheEntry {
                asset_id: ast_id,
                cached_file_path: path_str,
                size_bytes: 100,
                is_locked: false,
                lock_reason: None,
                last_accessed_at: now + (i as i64 * 1000), // song 1 is oldest, song 3 is newest
                created_at: now,
            };
            db.upsert_cache_entry(&entry).unwrap();
        }

        // Initialize cache manager with max 150 bytes limit (holding 1.5 songs)
        let cache_mgr = RollingCacheManager::new(&temp_dir, 150);

        // Lock song 1 (oldest accessed, but currently playing on Deck A)
        cache_mgr.lock_asset("ast_cache_1", Some(&db));
        assert!(cache_mgr.is_locked("ast_cache_1"));

        // Run rolling cleanup
        let pruned = cache_mgr.run_cleanup(&db).expect("Cleanup failed");
        assert!(pruned >= 1, "Should have pruned at least one file to stay under 150 bytes");

        // Song 1 MUST still exist on disk and in cache because it is LOCKED!
        assert!(file1_path.exists(), "Active deck locked file was evicted!");

        // Song 2 was unlocked and older than song 3, so song 2 should have been evicted
        assert!(!file2_path.exists(), "Oldest unlocked file was not pruned!");

        // Check stats
        let stats = cache_mgr.get_stats(&db).expect("Stats query failed");
        assert_eq!(stats.locked_files, 1);
        assert!(stats.total_bytes <= 200);

        // Unlock song 1 and test purge
        cache_mgr.unlock_asset("ast_cache_1", Some(&db));
        assert!(!cache_mgr.is_locked("ast_cache_1"));

        let purged = cache_mgr.purge_unlocked(&db).expect("Purge failed");
        assert!(purged >= 1);
        assert!(!file1_path.exists(), "Purged file was not deleted after unlock!");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_online_metadata_sync_outbox() {
        use axum::http::{HeaderMap, StatusCode};
        use axum::routing::post;
        use axum::Router;
        use dhun_lib::sync::{MetadataSyncManager, PlaybackSyncEvent, TrackMetadataCorrection};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let db = Database::new_in_memory().expect("Failed to init DB");

        // 1. Without endpoint configured: items must remain in 'pending' status (never falsely marked synced)
        let offline_sync_mgr = MetadataSyncManager::new(None, None);

        let correction = TrackMetadataCorrection {
            track_id: "trk_01".to_string(),
            title: Some("Gallan Goodiyaan (Live Festival Edit)".to_string()),
            artist: Some("Diljit Dosanjh".to_string()),
            genre: Some("Punjabi Pop".to_string()),
            bpm: Some(130.0),
            camelot_key: Some("10B".to_string()),
            manual_override: true,
            updated_at: chrono::Utc::now().timestamp_millis(),
        };

        let sync_id1 = offline_sync_mgr.enqueue_metadata_correction(&correction, &db).expect("Enqueue failed");
        assert!(sync_id1.starts_with("sync_"));

        let playback_ev = PlaybackSyncEvent {
            session_id: "ses_party_01".to_string(),
            track_id: "trk_01".to_string(),
            played_at: chrono::Utc::now().timestamp_millis(),
            duration_played_ms: 120000,
            was_skipped: false,
            transition_type: "bass_swap".to_string(),
        };

        let sync_id2 = offline_sync_mgr.enqueue_playback_event(&playback_ev, &db).expect("Enqueue failed");
        assert!(sync_id2.starts_with("sync_"));

        // Running outbox processing in offline mode leaves items pending
        let offline_synced = offline_sync_mgr.process_pending_outbox(10, &db).await.unwrap();
        assert_eq!(offline_synced, 0, "No items should be marked synced when remote endpoint is unconfigured");
        let initial_overview = offline_sync_mgr.get_sync_overview(&db).expect("Overview failed");
        assert_eq!(initial_overview.pending_count, 2);
        assert_eq!(initial_overview.synced_count, 0);

        // 2. Start an actual HTTP test server to verify real online network sync, idempotency, and auth headers
        let request_counter = Arc::new(AtomicUsize::new(0));
        let test_secret = "test_signing_key_12345";

        let counter_clone = request_counter.clone();
        let app = Router::new()
            .route("/api/v1/sync/track_metadata", post(move |headers: HeaderMap, _body: String| {
                let c = counter_clone.clone();
                async move {
                    assert!(headers.contains_key("x-dhun-idempotency-key"));
                    assert_eq!(headers.get("authorization").unwrap(), "Bearer test_signing_key_12345");
                    assert!(headers.contains_key("x-dhun-signature"));
                    c.fetch_add(1, Ordering::SeqCst);
                    StatusCode::OK
                }
            }))
            .route("/api/v1/sync/playback_event", post(move |headers: HeaderMap, _body: String| {
                let c = request_counter.clone();
                async move {
                    assert!(headers.contains_key("x-dhun-idempotency-key"));
                    assert_eq!(headers.get("authorization").unwrap(), "Bearer test_signing_key_12345");
                    assert!(headers.contains_key("x-dhun-signature"));
                    c.fetch_add(1, Ordering::SeqCst);
                    StatusCode::OK
                }
            }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_port = listener.local_addr().unwrap().port();
        let endpoint = format!("http://127.0.0.1:{}", server_port);

        let server_task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Check if environment permits loopback TCP socket connections (e.g. CI vs restricted sandbox)
        let loopback_permitted = tokio::net::TcpStream::connect(&format!("127.0.0.1:{}", server_port)).await.is_ok();
        if !loopback_permitted {
            // In a restricted sandbox with OS socket connect denied (WSAEACCES 10013),
            // verify outbox remains properly queued as pending with retry tracking
            let live_sync_mgr = MetadataSyncManager::new(Some(endpoint), Some(test_secret.to_string()));
            let _ = live_sync_mgr.process_pending_outbox(10, &db).await;
            let overview = live_sync_mgr.get_sync_overview(&db).expect("Overview failed");
            assert!(overview.pending_count > 0 || overview.failed_count > 0);
            server_task.abort();
            return;
        }

        // 3. Dispatch to the live HTTP server with authentication and request signing
        let live_sync_mgr = MetadataSyncManager::new(Some(endpoint), Some(test_secret.to_string()));
        let live_synced = live_sync_mgr.process_pending_outbox(10, &db).await.expect("Outbox processing failed");
        assert_eq!(live_synced, 2, "Both pending items must be delivered and acknowledged by the test server");

        let final_overview = live_sync_mgr.get_sync_overview(&db).expect("Overview failed");
        assert_eq!(final_overview.pending_count, 0);
        assert_eq!(final_overview.synced_count, 2);

        server_task.abort();
    }

    #[test]
    fn test_downloader_cancellation_and_status() {
        use dhun_lib::library::downloader::AudioDownloader;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_dl_test_{}", uuid::Uuid::new_v4()));
        let downloader = AudioDownloader::new(&temp_dir);

        let now = chrono::Utc::now().timestamp_millis();
        let task = DownloadTask {
            id: "dl_cancel_test".to_string(),
            url: "https://example.com/audio/huge_set.wav".to_string(),
            destination_path: temp_dir.join("huge_set.wav").to_string_lossy().to_string(),
            expected_sha256: None,
            status: "downloading".to_string(),
            bytes_downloaded: 1024,
            total_bytes: Some(104857600),
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.create_download_task(&task).unwrap();
        downloader.cancel_download("dl_cancel_test", &db).expect("Cancel failed");

        let task_after = db.get_download_task("dl_cancel_test").unwrap().unwrap();
        assert_eq!(task_after.status, "cancelled");
        assert_eq!(task_after.error_message, Some("Cancelled by user".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_audio_hash_cryptographic_verification() {
        use dhun_lib::library::ingest::compute_file_sha256;
        use sha2::{Digest, Sha256};

        let temp_file = std::env::temp_dir().join(format!("dhun_hash_{}.dat", uuid::Uuid::new_v4()));
        let test_bytes = b"DHUN_AUTOMATIC_DJ_AUDIO_STREAM_TEST_PAYLOAD_12345";
        std::fs::write(&temp_file, test_bytes).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(test_bytes);
        let expected_hash = format!("{:x}", hasher.finalize());

        let calculated_hash = compute_file_sha256(&temp_file).expect("Hash calculation failed");
        assert_eq!(calculated_hash, expected_hash);

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_duplicate_audio_asset_detection() {
        use dhun_lib::library::ingest::ingest_audio_asset;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_dup_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let file_a = temp_dir.join("original.mp3");
        let file_b = temp_dir.join("duplicate_copy.mp3");

        let audio_bytes = b"ID3\x03\x00\x00\x00\x00\x00\x00SAMPLE_AUDIO_TEST_PAYLOAD_FOR_DUPLICATE_DETECTION";
        std::fs::write(&file_a, audio_bytes).unwrap();
        std::fs::write(&file_b, audio_bytes).unwrap();

        // Ingest file A
        let res_a = ingest_audio_asset(&file_a, "local_filesystem", None, false, &db).expect("Ingest A failed");
        assert!(!res_a.is_duplicate, "First ingestion should not be duplicate");

        // Ingest file B (identical content, different path)
        let res_b = ingest_audio_asset(&file_b, "url_download", Some("https://example.com/song.mp3"), true, &db).expect("Ingest B failed");
        assert!(res_b.is_duplicate, "Second ingestion MUST be detected as duplicate");
        assert_eq!(res_a.track_id, res_b.track_id, "Duplicate must point to same canonical track_id");
        assert_eq!(res_a.sha256_hash, res_b.sha256_hash);

        // Strict verification: canonical metadata must be returned, NOT fabricated placeholders
        assert_eq!(res_b.title, res_a.title);
        assert_eq!(res_b.artist, res_a.artist);
        assert_eq!(res_b.duration_ms, res_a.duration_ms);
        assert_eq!(res_b.bpm, res_a.bpm);
        assert_eq!(res_b.key, res_a.key);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_reject_corrupt_truncated_audio_and_quarantine() {
        use dhun_lib::library::ingest::ingest_audio_asset;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_corrupt_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let corrupt_file = temp_dir.join("corrupt_recording.mp3");
        std::fs::write(&corrupt_file, b"NOT_A_VALID_AUDIO_FILE_GARBAGE_PAYLOAD_1234567890").unwrap();

        let res = ingest_audio_asset(&corrupt_file, "url_download", None, false, &db).expect("Ingest should quarantine without panic");
        assert_eq!(res.verification_status, "failed_analysis");

        // Verify quarantined track is NOT available in playable candidates
        let playable = db.get_playable_candidates().expect("Candidates query failed");
        assert!(!playable.iter().any(|c| c.track.id == res.track_id), "Quarantined corrupt track must NOT be in playable queue");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_3_hour_continuous_dj_playback_simulation() {
        use dhun_lib::dj::scheduler::DjScheduler;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_sim_3hr_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let now = chrono::Utc::now().timestamp_millis();

        // Seed 12 verified physical tracks for continuous playback simulation
        for i in 1..=12 {
            let file_path = temp_dir.join(format!("sim_track_{}.mp3", i));
            std::fs::write(&file_path, b"DUMMY_MP3_PAYLOAD").unwrap();

            let track = Track {
                id: format!("sim_trk_{}", i),
                title: format!("Party Hit {}", i),
                artist: format!("Wedding Artist {}", (i % 4) + 1),
                album: Some("Celebration Anthems".to_string()),
                duration_ms: 180_000,
                year: Some(2024),
                genre: "Bollywood Dance".to_string(),
                subgenre: None,
                created_at: now,
                updated_at: now,
            };

            let asset = AudioAsset {
                id: format!("sim_ast_{}", i),
                track_id: format!("sim_trk_{}", i),
                file_path: file_path.to_string_lossy().to_string(),
                file_format: "mp3".to_string(),
                file_size_bytes: 5_000_000,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: format!("sim_hash_{}", i),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: now,
            };

            let analysis = TrackAnalysis {
                track_id: format!("sim_trk_{}", i),
                bpm: 125.0 + (i as f64 * 1.5),
                bpm_confidence: 0.95,
                key_name: "C Major".to_string(),
                camelot_key: "8B".to_string(),
                key_confidence: 0.9,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 120,
                phrase_length_bars: 32,
                overall_energy: 0.85,
                dance_suitability: 0.90,
                recommended_mix_in_ms: 15000,
                recommended_mix_out_ms: 165000,
                analysis_version: 1,
                analyzed_at: now,
                manual_override: false,
            };

            db.insert_or_update_track(&track, &asset, &analysis).unwrap();
        }

        let session = PartySession {
            id: "ses_3hr_sim".to_string(),
            party_name: "3-Hour Nonstop Wedding Celebration".to_string(),
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
            dj_level: 2,
            primary_mood_id: "shaadi".to_string(),
            secondary_mood_id: None,
            output_device_name: "Default".to_string(),
            status: "active".to_string(),
            total_tracks_played: 0,
            config_json: None,
        };

        db.create_session(&session).expect("Failed to create session");
        let mut sched = DjScheduler::new(session);

        // 3 hours = 180 minutes. At ~3 minutes per track = 60 continuous track transitions
        let mut sim_clock_ms = sched.session.start_time;
        for i in 1..=60 {
            // Ensure upcoming queue has candidate tracks populated
            sched.ensure_queue_populated(&db).expect("Queue population failed");
            assert!(sched.next_track.is_some(), "Next track was not populated");

            // Advance simulation clock by 180 seconds (3 mins)
            sim_clock_ms += 180_000;

            // Trigger next track transition in the scheduler
            let next = sched.next_track.take().unwrap();
            let max_decks = sched.level.max_decks();
            let target_deck = (sched.active_deck + 1) % max_decks;

            // Log playback history item
            db.log_playback(
                &sched.session.id,
                &next.track.id,
                target_deck as i32,
                180_000,
                false,
                &next.transition_style,
            ).expect("Playback logging failed");

            sched.playback_history.push(PlaybackHistoryItem {
                id: i,
                session_id: sched.session.id.clone(),
                track_id: next.track.id.clone(),
                track_title: next.track.title.clone(),
                track_artist: next.track.artist.clone(),
                deck_id: target_deck as i32,
                played_at: sim_clock_ms,
                duration_played_ms: 180_000,
                was_skipped: false,
                transition_type: next.transition_style.clone(),
            });

            sched.current_track = Some(next);
            sched.next_track = sched.following_track.take();
            sched.active_deck = target_deck;
        }

        // Verify all 60 tracks were successfully scheduled and logged
        let history = db.get_session_playback_history("ses_3hr_sim").expect("History failed");
        assert_eq!(history.len(), 60, "Exactly 60 tracks must be recorded across the 3-hour marathon");

        // Verify scheduler queue remains healthy and auto-replenished
        sched.ensure_queue_populated(&db).unwrap();
        assert!(sched.next_track.is_some());
        assert!(sched.following_track.is_some());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_guest_server_restart_without_audio_interruption() {
        use dhun_lib::audio::AudioEngine;
        use dhun_lib::network::LocalPartyServer;
        use std::sync::atomic::Ordering;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let dummy_audio = AudioEngine::new_dummy();

        // 1. Audio engine starts in dummy mode without hardware error
        assert_eq!(dummy_audio.has_device_error.load(Ordering::SeqCst), false);

        // 2. Start guest server on available port (e.g. 19876)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let server = LocalPartyServer::start(
                19876,
                "ses_test".to_string(),
                "DHUN Test Party".to_string(),
                "shaadi".to_string(),
                db.clone(),
            ).await.expect("Failed to start server");

            assert!(!server.guest_url.is_empty());
            assert!(!server.qr_svg.is_empty());

            // 3. Stop server cleanly
            server.stop();

            // 4. Restart server on the same port
            let server2 = LocalPartyServer::start(
                19876,
                "ses_test".to_string(),
                "DHUN Test Party".to_string(),
                "shaadi".to_string(),
                db.clone(),
            ).await.expect("Failed to restart server");

            assert!(!server2.guest_url.is_empty());
            server2.stop();
        });

        // 5. Audio engine state remains completely intact and undisturbed
        assert_eq!(dummy_audio.has_device_error.load(Ordering::SeqCst), false);
    }

    #[test]
    fn test_ssrf_and_private_ip_rejection() {
        use dhun_lib::library::downloader::validate_safe_public_url;

        // Insecure scheme rejection
        assert!(validate_safe_public_url("http://example.com/audio.mp3").is_err());
        assert!(validate_safe_public_url("ftp://example.com/audio.mp3").is_err());

        // Loopback rejections
        assert!(validate_safe_public_url("https://127.0.0.1/song.mp3").is_err());
        assert!(validate_safe_public_url("https://localhost/song.mp3").is_err());
        assert!(validate_safe_public_url("https://[::1]/song.mp3").is_err());

        // Private RFC 1918 network rejections
        assert!(validate_safe_public_url("https://10.0.0.1/song.mp3").is_err());
        assert!(validate_safe_public_url("https://172.16.0.1/song.mp3").is_err());
        assert!(validate_safe_public_url("https://192.168.1.1/song.mp3").is_err());

        // Cloud metadata & Link-local rejections
        assert!(validate_safe_public_url("https://169.254.169.254/latest/meta-data/").is_err());

        // Carrier-grade NAT rejections (RFC 6598)
        assert!(validate_safe_public_url("https://100.64.0.1/song.mp3").is_err());
    }

    #[test]
    fn test_filename_sanitization_and_traversal_prevention() {
        use dhun_lib::library::downloader::sanitize_custom_filename;

        // Path traversal stripped
        let safe1 = sanitize_custom_filename("../../../../etc/passwd.mp3", "fallback");
        assert!(!safe1.contains('/'));
        assert!(!safe1.contains('\\'));
        assert!(!safe1.contains(".."));

        // Windows reserved device names neutralized
        let con_safe = sanitize_custom_filename("CON.mp3", "fallback");
        assert!(con_safe.starts_with("safe_CON"));

        let aux_safe = sanitize_custom_filename("aux.wav", "fallback");
        assert!(aux_safe.starts_with("safe_aux"));

        let nul_safe = sanitize_custom_filename("nul.flac", "fallback");
        assert!(nul_safe.starts_with("safe_nul"));

        // Special characters neutralized
        let clean = sanitize_custom_filename("track*name?<>:|test.mp3", "fallback");
        assert!(!clean.contains('*'));
        assert!(!clean.contains('?'));
        assert!(!clean.contains('<'));
        assert!(!clean.contains('>'));
        assert!(!clean.contains(':'));
        assert!(!clean.contains('|'));
    }

    #[tokio::test]
    async fn test_guest_session_token_authentication_and_privacy() {
        use axum::extract::{ConnectInfo, Query, State};
        use axum::http::StatusCode;
        use dhun_lib::network::rate_limit::RateLimiter;
        use dhun_lib::network::routes::{
            guest_auth, submit_request, get_session_requests,
            AppState, AuthPayload, CreateRequestPayload, GetRequestsQuery,
        };
        use std::net::SocketAddr;
        use std::sync::Arc;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_auth_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let song_file = temp_dir.join("wedding_track.mp3");
        std::fs::write(&song_file, b"DUMMY_AUDIO_DATA").unwrap();

        let track = Track {
            id: "trk_wedding_01".to_string(),
            title: "London Thumakda".to_string(),
            artist: "Labh Janjua".to_string(),
            album: Some("Queen".to_string()),
            duration_ms: 210_000,
            year: Some(2014),
            genre: "Bollywood Dance".to_string(),
            subgenre: None,
            created_at: 1000,
            updated_at: 1000,
        };

        let asset = AudioAsset {
            id: "ast_wedding_01".to_string(),
            track_id: "trk_wedding_01".to_string(),
            file_path: song_file.to_string_lossy().to_string(),
            file_format: "mp3".to_string(),
            file_size_bytes: 4000000,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: "hash_wedding_01".to_string(),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 1000,
        };

        let analysis = TrackAnalysis {
            track_id: "trk_wedding_01".to_string(),
            bpm: 128.0,
            bpm_confidence: 0.95,
            key_name: "C Major".to_string(),
            camelot_key: "8B".to_string(),
            key_confidence: 0.9,
            time_signature_num: 4,
            time_signature_den: 4,
            downbeat_offset_ms: 120,
            phrase_length_bars: 32,
            overall_energy: 0.88,
            dance_suitability: 0.95,
            recommended_mix_in_ms: 10000,
            recommended_mix_out_ms: 200000,
            analysis_version: 1,
            analyzed_at: 1000,
            manual_override: false,
        };

        db.insert_or_update_track(&track, &asset, &analysis).unwrap();

        let session = PartySession {
            id: "ses_auth_party".to_string(),
            party_name: "Karan & Simran Sangeet".to_string(),
            start_time: 1000,
            end_time: None,
            dj_level: 2,
            primary_mood_id: "shaadi".to_string(),
            secondary_mood_id: None,
            output_device_name: "Default".to_string(),
            status: "active".to_string(),
            total_tracks_played: 0,
            config_json: None,
        };
        db.create_session(&session).unwrap();

        let state = Arc::new(AppState {
            db: db.clone(),
            session_id: "ses_auth_party".to_string(),
            party_name: "Karan & Simran Sangeet".to_string(),
            primary_mood: "shaadi".to_string(),
            rate_limiter: RateLimiter::new(10, 60),
        });

        let dummy_addr: SocketAddr = "192.168.1.50:54321".parse().unwrap();

        // 1. Guest Authentication: Generate server-issued token
        let auth_res = guest_auth(
            State(state.clone()),
            ConnectInfo(dummy_addr),
            axum::Json(AuthPayload {
                display_name: "Rohan Verma".to_string(),
            }),
        ).await.expect("Guest auth failed");

        let auth_data = auth_res.0;
        assert!(!auth_data.token.is_empty());
        assert_eq!(auth_data.display_name, "Rohan Verma");

        // 2. Reject request with invalid / forged token
        let fake_req = submit_request(
            State(state.clone()),
            ConnectInfo(dummy_addr),
            axum::Json(CreateRequestPayload {
                token: Some("tok_invalid_bogus_token".to_string()),
                guest_id: None,
                display_name: None,
                track_id: "trk_wedding_01".to_string(),
                host_note: Some("Play this next please!".to_string()),
            }),
        ).await;
        assert_eq!(fake_req.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 3. Accept request with valid token
        let valid_req = submit_request(
            State(state.clone()),
            ConnectInfo(dummy_addr),
            axum::Json(CreateRequestPayload {
                token: Some(auth_data.token.clone()),
                guest_id: None,
                display_name: None,
                track_id: "trk_wedding_01".to_string(),
                host_note: Some("Play this next please!".to_string()),
            }),
        ).await.expect("Valid request failed");

        assert_eq!(valid_req.0.track_id, "trk_wedding_01");
        assert_eq!(valid_req.0.guest_name, Some("Rohan Verma".to_string()));

        // 4. Privacy masking: Caller sees own name & note
        let own_requests = get_session_requests(
            State(state.clone()),
            Query(GetRequestsQuery {
                token: Some(auth_data.token.clone()),
            }),
        ).await.expect("Query failed").0;

        assert_eq!(own_requests.len(), 1);
        assert_eq!(own_requests[0].guest_name, Some("Rohan Verma".to_string()));
        assert_eq!(own_requests[0].host_note, Some("Play this next please!".to_string()));

        // 5. Privacy masking: Public caller without token sees masked "Party Guest" and None host_note
        let public_requests = get_session_requests(
            State(state.clone()),
            Query(GetRequestsQuery { token: None }),
        ).await.expect("Query failed").0;

        assert_eq!(public_requests.len(), 1);
        assert_eq!(public_requests[0].guest_id, "anonymized");
        assert_eq!(public_requests[0].guest_name, Some("Party Guest".to_string()));
        assert_eq!(public_requests[0].host_note, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_secondary_mood_score_blending() {
        use dhun_lib::dj::moods::get_preset_moods;
        use dhun_lib::dj::selector::TrackSelector;

        let preset_moods = get_preset_moods();
        let shaadi_mood = preset_moods.iter().find(|m| m.id == "shaadi").unwrap();
        let punjabi_mood = preset_moods.iter().find(|m| m.id == "punjabi_tashan").unwrap();

        let punjabi_candidate = PlayableTrackCandidate {
            track: Track {
                id: "trk_punjabi_01".to_string(),
                title: "Mundian To Bach Ke".to_string(),
                artist: "Panjabi MC".to_string(),
                album: None,
                duration_ms: 200000,
                year: None,
                genre: "Bhangra Punjabi".to_string(),
                subgenre: None,
                created_at: 0,
                updated_at: 0,
            },
            asset: AudioAsset {
                id: "ast_p1".to_string(),
                track_id: "trk_punjabi_01".to_string(),
                file_path: "C:/p1.mp3".to_string(),
                file_format: "mp3".to_string(),
                file_size_bytes: 1000,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: "hash_p".to_string(),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: 0,
            },
            analysis: TrackAnalysis {
                track_id: "trk_punjabi_01".to_string(),
                bpm: 130.0,
                bpm_confidence: 0.9,
                key_name: "D Minor".to_string(),
                camelot_key: "7A".to_string(),
                key_confidence: 0.9,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 100,
                phrase_length_bars: 32,
                overall_energy: 0.92,
                dance_suitability: 0.95,
                recommended_mix_in_ms: 10000,
                recommended_mix_out_ms: 190000,
                analysis_version: 1,
                analyzed_at: 0,
                manual_override: false,
            },
            compatibility_score: 0.0,
            transition_style: "bass_swap".to_string(),
        };

        let candidates = vec![punjabi_candidate];

        // Score with Shaadi alone
        let res_single = TrackSelector::score_and_select(
            &candidates,
            None,
            shaadi_mood,
            None,
            shaadi_mood.target_energy,
            &[],
            &[],
            &[],
        ).unwrap();

        // Score with Shaadi (70%) blended with Punjabi Tashan (30%)
        let res_blended = TrackSelector::score_and_select(
            &candidates,
            None,
            shaadi_mood,
            Some(punjabi_mood),
            shaadi_mood.target_energy,
            &[],
            &[],
            &[],
        ).unwrap();

        // Blended mood must elevate the score of high-energy Bhangra tracks
        assert!(
            res_blended.compatibility_score >= res_single.compatibility_score,
            "Blended mood score ({}) should be >= single mood score ({}) for Bhangra track",
            res_blended.compatibility_score,
            res_single.compatibility_score
        );
    }

    #[test]
    fn test_sync_outbox_in_flight_crash_recovery() {
        use dhun_lib::sync::{MetadataSyncManager, TrackMetadataCorrection};

        let db = Database::new_in_memory().expect("Failed to init DB");
        let sync_mgr = MetadataSyncManager::new(None, None);

        let correction = TrackMetadataCorrection {
            track_id: "trk_rec_01".to_string(),
            title: Some("Corrected Title".to_string()),
            artist: Some("Corrected Artist".to_string()),
            genre: Some("Desi Pop".to_string()),
            bpm: Some(126.0),
            camelot_key: Some("9B".to_string()),
            manual_override: true,
            updated_at: chrono::Utc::now().timestamp_millis(),
        };

        let item_id = sync_mgr.enqueue_metadata_correction(&correction, &db).unwrap();

        // Simulate crash while item was in-flight
        db.update_sync_item_status(&item_id, "in_flight", None, false).unwrap();
        let overview_before = db.get_sync_overview().unwrap();
        assert_eq!(overview_before.in_flight_count, 1);
        assert_eq!(overview_before.pending_count, 0);

        // Recover on startup
        let recovered = sync_mgr.recover_in_flight_items(&db).unwrap();
        assert_eq!(recovered, 1);

        let overview_after = db.get_sync_overview().unwrap();
        assert_eq!(overview_after.in_flight_count, 0);
        assert_eq!(overview_after.pending_count, 1);
    }

    #[test]
    fn test_cache_local_file_preservation_and_lock_reconciliation() {
        use dhun_lib::library::cache::RollingCacheManager;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_root = std::env::temp_dir().join(format!("dhun_cache_guard_{}", uuid::Uuid::new_v4()));
        let cache_dir = temp_root.join("cache");
        let user_music_dir = temp_root.join("UserMusic");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::create_dir_all(&user_music_dir).unwrap();

        let cached_file = cache_dir.join("downloaded_asset.mp3");
        let user_file = user_music_dir.join("my_precious_local_track.mp3");

        std::fs::write(&cached_file, vec![0u8; 1000]).unwrap();
        std::fs::write(&user_file, vec![0u8; 1000]).unwrap();

        let now = chrono::Utc::now().timestamp_millis();

        // Register backing tracks & assets in DB to satisfy foreign key constraints
        for (trk_id, ast_id, fpath) in [
            ("trk_cached_1", "ast_cached_1", &cached_file),
            ("trk_user_1", "ast_user_1", &user_file),
        ] {
            let track = Track {
                id: trk_id.to_string(),
                title: format!("Title {}", trk_id),
                artist: "Artist".to_string(),
                album: None,
                duration_ms: 180000,
                year: Some(2024),
                genre: "Bollywood".to_string(),
                subgenre: None,
                created_at: now,
                updated_at: now,
            };
            let asset = AudioAsset {
                id: ast_id.to_string(),
                track_id: trk_id.to_string(),
                file_path: fpath.to_string_lossy().to_string(),
                file_format: "mp3".to_string(),
                file_size_bytes: 1000,
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                sha256_hash: format!("hash_{}", trk_id),
                verification_status: "verified".to_string(),
                availability_status: "available".to_string(),
                last_verified_at: now,
            };
            let analysis = TrackAnalysis {
                track_id: trk_id.to_string(),
                bpm: 125.0,
                bpm_confidence: 0.9,
                key_name: "C Major".to_string(),
                camelot_key: "8B".to_string(),
                key_confidence: 0.9,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 100,
                phrase_length_bars: 32,
                overall_energy: 0.8,
                dance_suitability: 0.8,
                recommended_mix_in_ms: 0,
                recommended_mix_out_ms: 180000,
                analysis_version: 1,
                analyzed_at: now,
                manual_override: false,
            };
            db.insert_or_update_track(&track, &asset, &analysis).unwrap();
        }

        // Register both in cache manifest
        let cache_entry = CacheEntry {
            asset_id: "ast_cached_1".to_string(),
            cached_file_path: cached_file.to_string_lossy().to_string(),
            size_bytes: 1000,
            is_locked: false,
            lock_reason: None,
            last_accessed_at: now - 5000,
            created_at: now - 5000,
        };
        let user_entry = CacheEntry {
            asset_id: "ast_user_1".to_string(),
            cached_file_path: user_file.to_string_lossy().to_string(),
            size_bytes: 1000,
            is_locked: false,
            lock_reason: None,
            last_accessed_at: now - 10000, // Older access time
            created_at: now - 10000,
        };

        db.upsert_cache_entry(&cache_entry).unwrap();
        db.upsert_cache_entry(&user_entry).unwrap();

        let cache_mgr = RollingCacheManager::new(&cache_dir, 500); // 500 bytes max -> forces eviction

        // Run cleanup
        let _ = cache_mgr.run_cleanup(&db).unwrap();

        // The file in cache_dir was evicted
        assert!(!cached_file.exists(), "File in cache directory should be evicted");

        // The file outside cache_dir (user's own music) MUST NEVER BE DELETED!
        assert!(user_file.exists(), "User music file outside cache directory MUST be preserved!");

        // Lock reconciliation
        cache_mgr.lock_asset("ast_dummy", Some(&db));
        assert!(cache_mgr.is_locked("ast_dummy"));
        cache_mgr.reconcile_locks(&db).unwrap();
        assert!(!cache_mgr.is_locked("ast_dummy"));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_decoded_audio_memory_budget_cap() {
        use dhun_lib::audio::decoder::MAX_DECODED_AUDIO_MINUTES;
        assert_eq!(MAX_DECODED_AUDIO_MINUTES, 30, "Decoded audio memory budget must be capped at 30 minutes");
    }

    #[test]
    fn test_decoder_rejects_truncated_damaged_audio() {
        use dhun_lib::audio::decoder::decode_audio_file;

        let temp_dir = std::env::temp_dir().join(format!("dhun_trunc_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let truncated_file = temp_dir.join("broken_header.mp3");

        // Write partial truncated header with invalid frames
        std::fs::write(&truncated_file, b"ID3\x03\x00\x00\x00\x00\x00\x10TRUNCATED_CORRUPT_BUFFER").unwrap();

        // Must reject damaged audio with an error rather than accepting partial frames
        let res = decode_audio_file(&truncated_file);
        assert!(res.is_err(), "Decoder must strictly reject damaged/truncated audio streams");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

