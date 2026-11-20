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
        let offline_sync_mgr = MetadataSyncManager::new(None, None).expect("Init failed");

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
                    assert!(headers.contains_key("x-dhun-timestamp"));
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
                    assert!(headers.contains_key("x-dhun-timestamp"));
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
            let live_sync_mgr = MetadataSyncManager::new(Some(endpoint), Some(test_secret.to_string())).expect("Init failed");
            let _ = live_sync_mgr.process_pending_outbox(10, &db).await;
            let overview = live_sync_mgr.get_sync_overview(&db).expect("Overview failed");
            assert!(overview.pending_count > 0 || overview.failed_count > 0);
            server_task.abort();
            return;
        }

        // 3. Dispatch to the live HTTP server with authentication and request signing
        let live_sync_mgr = MetadataSyncManager::new(Some(endpoint), Some(test_secret.to_string())).expect("Init failed");
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

        // IPv4-mapped IPv6 rejections
        assert!(validate_safe_public_url("https://[::ffff:127.0.0.1]/song.mp3").is_err());
        assert!(validate_safe_public_url("https://[::ffff:192.168.1.1]/song.mp3").is_err());
        assert!(validate_safe_public_url("https://[::ffff:10.0.0.1]/song.mp3").is_err());
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

        // 2. Reject request with missing token or unauthenticated guest_id bypass attempt
        let unauth_req = submit_request(
            State(state.clone()),
            ConnectInfo(dummy_addr),
            axum::Json(CreateRequestPayload {
                token: None,
                guest_id: Some("gst_bypassed_attacker".to_string()),
                display_name: Some("Hacker".to_string()),
                track_id: "trk_wedding_01".to_string(),
                host_note: Some("Should be rejected".to_string()),
            }),
        ).await;
        assert_eq!(unauth_req.unwrap_err().0, StatusCode::UNAUTHORIZED, "Unauthenticated requests without token must be strictly rejected");

        // Reject request with forged token
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

        // 5. Unauthenticated callers cannot view queue
        let unauth_queue = get_session_requests(
            State(state.clone()),
            Query(GetRequestsQuery { token: None }),
        ).await;
        assert_eq!(unauth_queue.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 6. Another authenticated guest (Guest B) sees masked "Party Guest" and None host_note
        let guest_b_auth = guest_auth(
            State(state.clone()),
            ConnectInfo("192.168.1.51:54322".parse().unwrap()),
            axum::Json(AuthPayload {
                display_name: "Ayesha Khan".to_string(),
            }),
        ).await.expect("Guest B auth failed").0;

        let other_requests = get_session_requests(
            State(state.clone()),
            Query(GetRequestsQuery { token: Some(guest_b_auth.token) }),
        ).await.expect("Query failed").0;

        assert_eq!(other_requests.len(), 1);
        assert_eq!(other_requests[0].guest_id, "anonymized");
        assert_eq!(other_requests[0].guest_name, Some("Party Guest".to_string()));
        assert_eq!(other_requests[0].host_note, None);

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
        let sync_mgr = MetadataSyncManager::new(None, None).expect("Init failed");

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

    /// Helper to generate synthetic WAV bytes with customizable declared and actual frame counts
    fn create_test_wav_bytes(sample_rate: u32, channels: u16, declared_frames: u32, actual_frames: u32) -> Vec<u8> {
        let bits_per_sample: u16 = 16;
        let bytes_per_sample: u16 = bits_per_sample / 8;
        let block_align: u16 = channels * bytes_per_sample;
        let byte_rate: u32 = sample_rate * block_align as u32;

        let declared_data_bytes = declared_frames * block_align as u32;
        let actual_data_bytes = actual_frames * block_align as u32;
        let riff_chunk_size = 36 + declared_data_bytes;

        let mut buf = Vec::with_capacity(44 + actual_data_bytes as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_chunk_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&declared_data_bytes.to_le_bytes());

        // Generate sinusoidal PCM 16-bit audio samples for actual_frames
        for f in 0..actual_frames {
            let t = f as f32 / sample_rate as f32;
            let sample_val = ((t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 16000.0) as i16;
            for _ in 0..channels {
                buf.extend_from_slice(&sample_val.to_le_bytes());
            }
        }
        buf
    }

    #[test]
    fn test_decoder_rejects_truncated_damaged_audio() {
        use dhun_lib::audio::decoder::decode_audio_file;

        let temp_dir = std::env::temp_dir().join(format!("dhun_trunc_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 1. Broken / corrupted container header
        let corrupted_file = temp_dir.join("broken_header.mp3");
        std::fs::write(&corrupted_file, b"ID3\x03\x00\x00\x00\x00\x00\x10TRUNCATED_CORRUPT_BUFFER").unwrap();
        let res_corrupt = decode_audio_file(&corrupted_file);
        assert!(res_corrupt.is_err(), "Decoder must strictly reject damaged/truncated audio streams");

        // 2. Real truncated audio fixture: RIFF WAV header declares 100,000 frames (~2.27s), but file cuts off at 50,000 frames (~1.13s)
        let truncated_wav_file = temp_dir.join("truncated_fixture.wav");
        let truncated_bytes = create_test_wav_bytes(44100, 2, 100_000, 50_000);
        std::fs::write(&truncated_wav_file, &truncated_bytes).unwrap();
        let res_trunc = decode_audio_file(&truncated_wav_file);
        assert!(
            res_trunc.is_err(),
            "Decoder must reject premature stream truncation where actual frames < declared frames"
        );
        let err_text = res_trunc.err().unwrap().to_string();
        assert!(
            err_text.contains("Premature stream truncation"),
            "Error message must specify premature truncation: {}", err_text
        );

        // 3. Short audio file fixture (< 1.0 second) with undeclared/tiny frames (500 frames at 44.1kHz = 11.3ms)
        let tiny_wav_file = temp_dir.join("tiny_fixture.wav");
        let tiny_bytes = create_test_wav_bytes(44100, 2, 500, 500);
        std::fs::write(&tiny_wav_file, &tiny_bytes).unwrap();
        let res_tiny = decode_audio_file(&tiny_wav_file);
        assert!(
            res_tiny.is_err(),
            "Decoder must reject files containing less than 1.0 second of audio"
        );

        // 4. Valid complete audio fixture (44100 frames = exactly 1.0 second stereo)
        let valid_wav_file = temp_dir.join("valid_fixture.wav");
        let valid_bytes = create_test_wav_bytes(44100, 2, 44100, 44100);
        std::fs::write(&valid_wav_file, &valid_bytes).unwrap();
        let res_valid = decode_audio_file(&valid_wav_file);
        assert!(res_valid.is_ok(), "Complete valid 1.0s stereo WAV file must decode cleanly");
        let decoded = res_valid.unwrap();
        assert_eq!(decoded.sample_rate, 44100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.duration_ms, 1000);
        assert_eq!(decoded.samples.len(), 44100 * 2);

        // 5. Layout-aware conversion: Mono WAV (1 channel) correctly upmixed and normalized to stereo [M, M]
        let mono_wav_file = temp_dir.join("mono_fixture.wav");
        let mono_bytes = create_test_wav_bytes(44100, 1, 44100, 44100);
        std::fs::write(&mono_wav_file, &mono_bytes).unwrap();
        let res_mono = decode_audio_file(&mono_wav_file);
        assert!(res_mono.is_ok(), "Mono audio file must decode and upmix to standardized stereo");
        let decoded_mono = res_mono.unwrap();
        assert_eq!(decoded_mono.channels, 2);
        assert_eq!(decoded_mono.samples.len(), 44100 * 2);
        // Verify channel duplication [M, M]
        assert_eq!(decoded_mono.samples[0], decoded_mono.samples[1]);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_hmac_sha256_canonical_signing_and_tampering() {
        use dhun_lib::sync::{sign_request, verify_signature};

        let secret = "party_secret_key_98765";
        let method = "POST";
        let path = "/api/v1/sync/track_metadata";
        let now = chrono::Utc::now().timestamp();
        let payload = r#"{"track_id":"trk_test_01","bpm":128.0,"genre":"Bollywood"}"#;

        // 1. Signature generation
        let sig = sign_request(secret, method, path, now, payload);
        assert_eq!(sig.len(), 64, "HMAC-SHA256 hex signature must be exactly 64 characters");

        // 2. Positive verification
        assert!(
            verify_signature(secret, method, path, now, payload, &sig),
            "Valid signature and un-tampered payload must verify successfully"
        );
        assert!(
            verify_signature(secret, method, path, now, payload, &format!("sha256={}", sig)),
            "Signature with sha256= prefix must verify"
        );

        // 3. Tampering: Altered payload must fail
        let tampered_payload = r#"{"track_id":"trk_test_01","bpm":140.0,"genre":"Bollywood"}"#;
        assert!(
            !verify_signature(secret, method, path, now, tampered_payload, &sig),
            "Tampered payload must fail signature verification"
        );

        // 4. Tampering: Altered HTTP path must fail
        assert!(
            !verify_signature(secret, method, "/api/v1/sync/other", now, payload, &sig),
            "Tampered request path must fail signature verification"
        );

        // 5. Tampering: Wrong secret must fail
        assert!(
            !verify_signature("wrong_secret", method, path, now, payload, &sig),
            "Incorrect API secret must fail signature verification"
        );

        // 6. Replay prevention: Expired timestamp (>300 seconds) must fail
        let expired_timestamp = now - 301;
        assert!(
            !verify_signature(secret, method, path, expired_timestamp, payload, &sig),
            "Expired timestamp beyond 300-second window must be rejected"
        );

        // 7. Robustness against extreme timestamps: i64::MIN and i64::MAX must not overflow or panic
        assert!(
            !verify_signature(secret, method, path, i64::MIN, payload, &sig),
            "i64::MIN timestamp must be safely rejected without integer overflow"
        );
        assert!(
            !verify_signature(secret, method, path, i64::MAX, payload, &sig),
            "i64::MAX timestamp must be safely rejected without integer overflow"
        );
    }

    #[test]
    fn test_sync_endpoint_https_and_mandatory_key_enforcement() {
        use dhun_lib::sync::MetadataSyncManager;

        // Insecure HTTP remote endpoint rejected
        let insecure = MetadataSyncManager::new(
            Some("http://api.dhun-party.com/sync".to_string()),
            Some("valid_key".to_string()),
        );
        assert!(insecure.is_err(), "Remote sync endpoint must enforce HTTPS");

        // Missing API key rejected
        let missing_key = MetadataSyncManager::new(
            Some("https://api.dhun-party.com/sync".to_string()),
            None,
        );
        assert!(missing_key.is_err(), "Configured remote sync endpoint requires mandatory API key");

        // Blank API key rejected
        let blank_key = MetadataSyncManager::new(
            Some("https://api.dhun-party.com/sync".to_string()),
            Some("   ".to_string()),
        );
        assert!(blank_key.is_err(), "Blank API key must be rejected");

        // Valid HTTPS remote endpoint with non-empty key accepted
        let valid = MetadataSyncManager::new(
            Some("https://api.dhun-party.com/sync".to_string()),
            Some("super_secret_key_123".to_string()),
        );
        assert!(valid.is_ok(), "Valid HTTPS endpoint and API key must succeed");

        // Offline / local unconfigured mode accepted
        let offline = MetadataSyncManager::new(None, None);
        assert!(offline.is_ok(), "Offline unconfigured mode must succeed");
    }

    #[test]
    fn test_duplicate_metadata_database_integrity() {
        use dhun_lib::library::ingest::ingest_audio_asset;
        use dhun_lib::db::models::AudioAsset;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_dup_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let audio_path = temp_dir.join("sample.mp3");
        std::fs::write(&audio_path, b"DUMMY_MP3_CONTENT_FOR_HASH").unwrap();

        // Simulate corrupt DB state: audio_asset exists referencing non-existent track
        let orphan_asset = AudioAsset {
            id: "ast_orphan".to_string(),
            track_id: "trk_nonexistent".to_string(),
            file_path: audio_path.to_string_lossy().to_string(),
            file_format: "mp3".to_string(),
            file_size_bytes: 26,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: dhun_lib::library::ingest::compute_file_sha256(&audio_path).unwrap(),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 1000,
        };

        // Insert audio asset directly into DB without matching track record (disabling foreign keys temporarily to simulate data corruption)
        {
            let conn = db.conn();
            let c = conn.lock().unwrap();
            c.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
            c.execute(
                "INSERT INTO audio_assets (id, track_id, file_path, file_format, file_size_bytes, sample_rate, bit_depth, channels, sha256_hash, verification_status, availability_status, last_verified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    orphan_asset.id, orphan_asset.track_id, orphan_asset.file_path, orphan_asset.file_format,
                    orphan_asset.file_size_bytes, orphan_asset.sample_rate, orphan_asset.bit_depth, orphan_asset.channels,
                    orphan_asset.sha256_hash, orphan_asset.verification_status, orphan_asset.availability_status, orphan_asset.last_verified_at
                ],
            ).unwrap();
            c.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        }

        // Ingestion of duplicate referencing orphan track must fail with DB integrity error rather than inventing fake metadata
        let ingest_res = ingest_audio_asset(&audio_path, "local", None, false, &db);
        assert!(
            ingest_res.is_err(),
            "Duplicate ingestion must fail with database integrity error if canonical track is missing"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_downloader_redirect_anti_ssrf_and_hop_limits() {
        use dhun_lib::library::downloader::{validate_safe_public_url, AudioDownloader};
        use reqwest::Url;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!("dhun_redirect_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let downloader = AudioDownloader::new(&temp_dir);

        // 1. Initial URL validation blocks SSRF before any connection attempt
        let ssrf_res = downloader.download_audio("https://127.0.0.1/track.mp3", None, None, &db).await;
        assert!(ssrf_res.is_err(), "Direct download to loopback must fail security validation");

        let tasks = db.list_download_tasks(10).unwrap();
        // Failed task was audited in DB
        let failed_task = tasks.iter().find(|t| t.url == "https://127.0.0.1/track.mp3");
        assert!(failed_task.is_some(), "Failed download task must be recorded for audit trail");
        assert_eq!(failed_task.unwrap().status, "failed");

        // 2. Redirect target validation: relative and absolute redirect hops to private / insecure addresses
        let base_url = Url::parse("https://safe-cdn.dhun-party.com/audio/v1/track.mp3").unwrap();

        // Redirect hop to insecure scheme
        let insecure_redirect = base_url.join("http://safe-cdn.dhun-party.com/audio/v1/track.mp3").unwrap();
        assert!(
            validate_safe_public_url(insecure_redirect.as_str()).is_err(),
            "Redirect target with non-HTTPS scheme must fail validation"
        );

        // Redirect hop to loopback / internal service
        let loopback_redirect = base_url.join("https://127.0.0.1/internal/admin.mp3").unwrap();
        assert!(
            validate_safe_public_url(loopback_redirect.as_str()).is_err(),
            "Redirect target pointing to loopback must fail anti-SSRF validation"
        );

        // Redirect hop to private network
        let private_redirect = base_url.join("https://192.168.1.50/audio.mp3").unwrap();
        assert!(
            validate_safe_public_url(private_redirect.as_str()).is_err(),
            "Redirect target pointing to private IP must fail anti-SSRF validation"
        );

        // Redirect hop to cloud metadata service
        let metadata_redirect = base_url.join("https://169.254.169.254/latest/meta-data/").unwrap();
        assert!(
            validate_safe_public_url(metadata_redirect.as_str()).is_err(),
            "Redirect target pointing to link-local/cloud metadata must fail anti-SSRF validation"
        );

        // Redirect hop to localhost hostname
        let localhost_redirect = base_url.join("https://localhost:8443/secret.wav").unwrap();
        assert!(
            validate_safe_public_url(localhost_redirect.as_str()).is_err(),
            "Redirect target pointing to localhost hostname must fail anti-SSRF validation"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_youtube_url_parsing_and_security_validation() {
        use dhun_lib::library::sources::{parse_youtube_url, MusicSourceProvider};

        // 1. Valid Standard YouTube Watch URL
        let res = parse_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(res.provider, MusicSourceProvider::YouTube);
        assert_eq!(res.video_id, "dQw4w9WgXcQ");
        assert_eq!(res.canonical_url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");

        // 2. Valid YouTube URL with additional query parameters (timestamps, playlists)
        let res2 = parse_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s&list=PL12345").unwrap();
        assert_eq!(res2.provider, MusicSourceProvider::YouTube);
        assert_eq!(res2.video_id, "dQw4w9WgXcQ");

        // 3. Valid Short URL (youtu.be)
        let res3 = parse_youtube_url("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(res3.provider, MusicSourceProvider::YouTube);
        assert_eq!(res3.video_id, "dQw4w9WgXcQ");

        // 4. Valid YouTube Music URL
        let res4 = parse_youtube_url("https://music.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(res4.provider, MusicSourceProvider::YouTubeMusic);
        assert_eq!(res4.video_id, "dQw4w9WgXcQ");

        // 5. Valid Embed URL
        let res5 = parse_youtube_url("https://youtube.com/embed/dQw4w9WgXcQ").unwrap();
        assert_eq!(res5.provider, MusicSourceProvider::YouTube);
        assert_eq!(res5.video_id, "dQw4w9WgXcQ");

        // 6. Security Rejections: Unauthorized Domains
        assert!(parse_youtube_url("https://vimeo.com/12345678901").is_err());
        assert!(parse_youtube_url("https://attacker.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(parse_youtube_url("https://not-youtube.com/watch?v=dQw4w9WgXcQ").is_err());

        // 7. Security Rejections: Invalid Schemes
        assert!(parse_youtube_url("ftp://youtube.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(parse_youtube_url("file:///etc/passwd").is_err());
        assert!(parse_youtube_url("javascript:alert(1)").is_err());

        // 8. Security Rejections: Malformed Video IDs (not 11 chars or containing illegal characters)
        assert!(parse_youtube_url("https://www.youtube.com/watch?v=short").is_err());
        assert!(parse_youtube_url("https://www.youtube.com/watch?v=toolongvideoid1234").is_err());
        assert!(parse_youtube_url("https://www.youtube.com/watch?v=invalid<char>").is_err());
    }

    /// In-process handler test: verifies local party room creation, hashed secrets (zero raw storage),
    /// genuine LAN QR generation (and honest lan_reachable=false when offline), explicit-only closure,
    /// guest token invalidation, and post-party temporary cache cleanup.
    #[tokio::test]
    async fn test_local_party_creation_qr_and_explicit_closure_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue};
        use axum::Json;
        use dhun_lib::library::cache::{CACHE_PRIORITY_PLAYING, DEFAULT_MAX_TEMP_TRACKS};
        use dhun_lib::network::local_helper_routes::{
            close_local_room, create_local_room, get_host_room_dashboard, local_health,
            start_local_room, CloseLocalRoomRequest, CreateLocalRoomRequest, LocalHelperState,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;

        let db = Database::new_in_memory().expect("Failed to init DB");
        let temp_cache = std::env::temp_dir().join(format!("dhun_p2_room_{}", uuid::Uuid::new_v4()));
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();

        // 1. With real reachable RFC1918 LAN IP
        let state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr.clone(),
            Some(Some("192.168.10.25".to_string())),
            vec!["https://dhun.vercel.app".to_string()],
        ));
        let host_addr: SocketAddr = "127.0.0.1:52000".parse().unwrap();
        let mut ws_rx = state.ws_tx.subscribe();

        let Json(health) = local_health(State(state.clone())).await;
        assert_eq!(health["service"], "dhun-local-helper");
        assert_eq!(health["lan_reachable"], true);
        assert_eq!(health["detected_lan_ip"], "192.168.10.25");

        let Json(created) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Sharma Wedding Sangeet".to_string(),
                dj_level: Some(3),
                primary_mood_id: Some("sangeet".to_string()),
                secondary_mood_id: Some("punjabi_tashan".to_string()),
            }),
        )
        .await
        .expect("Local room creation must succeed");

        assert!(created.lan_reachable);
        let guest_url = created.guest_url.as_ref().expect("Real LAN guest URL must be present");
        assert!(guest_url.starts_with("http://192.168.10.25:47821/join/"));
        assert!(guest_url.contains(&created.room.room_id));
        assert!(guest_url.contains(&created.room_access_token));
        assert!(created.qr_svg.as_ref().unwrap().contains("<svg"));

        // Verify stored tokens in DB are SHA-256 hashed (NEVER raw plaintext!)
        let stored_room = db.get_local_party_room(&created.room.room_id).unwrap().unwrap();
        assert_ne!(stored_room.host_token_hash, created.host_token);
        assert_ne!(stored_room.room_access_token_hash, created.room_access_token);
        assert_eq!(stored_room.host_token_hash.len(), 64);
        assert_eq!(stored_room.room_access_token_hash.len(), 64);

        // 2. Without reachable LAN IP -> MUST NOT fabricate fake LAN IP or QR code!
        let offline_lan_state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(None),
            vec![],
        ));
        let Json(no_lan_room) = create_local_room(
            State(offline_lan_state),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Offline Host Only".to_string(),
                dj_level: Some(1),
                primary_mood_id: Some("shaadi".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();
        assert!(!no_lan_room.lan_reachable);
        assert!(no_lan_room.guest_url.is_none(), "Must not fabricate guest URL without LAN IP");
        assert!(no_lan_room.qr_svg.is_none(), "Must not fabricate QR without LAN IP");
        assert!(no_lan_room.lan_notice.is_some());

        // 3. Start party & attach a temporary cached audio file to the room
        let mut host_headers = HeaderMap::new();
        host_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", created.host_token)).unwrap(),
        );

        let Json(started) = start_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .expect("Host must start local room");
        assert_eq!(started.status, "live");

        let evt_live = ws_rx.try_recv().expect("WS broadcast expected on start");
        assert_eq!(evt_live.event_type, "room.status_changed");

        let dummy_temp_audio = temp_cache.join("temp_party_track.wav");
        std::fs::write(&dummy_temp_audio, vec![1u8; 2048]).unwrap();
        state
            .cache_manager
            .register_temp_track(
                &created.room.room_id,
                None,
                "gsng_temp_01",
                &dummy_temp_audio,
                CACHE_PRIORITY_PLAYING,
                true,
                None,
                DEFAULT_MAX_TEMP_TRACKS,
                &db,
            )
            .unwrap();
        assert!(dummy_temp_audio.exists());

        // Verify dashboard reports live status (no auto-expiry)
        let Json(dash) = get_host_room_dashboard(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .unwrap();
        assert_eq!(dash.room.status, "live");
        assert_eq!(dash.exposed_decks, 4);
        assert_eq!(dash.internal_deck_capacity, 4);
        assert!(dash.auto_dj_enabled);

        // 4. Explicit host closure with force_release_audio=true invalidates room and purges temporary cached audio file
        let Json(closed) = close_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers,
            Some(Json(CloseLocalRoomRequest {
                force_release_audio: Some(true),
            })),
        )
        .await
        .expect("Explicit close must succeed");
        assert_eq!(closed["status"], "closed");
        assert_eq!(closed["purged_cache_files"], 1);
        assert_eq!(closed["deferred_locked_files"], 0);
        assert!(!dummy_temp_audio.exists(), "Temporary audio must be deleted on explicit party close");

        let evt_closed = ws_rx.try_recv().expect("WS broadcast expected on close");
        assert_eq!(evt_closed.event_type, "room.closed");

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// In-process handler + router test: verifies LAN guest auth (room_id alone rejected),
    /// cross-room isolation, strict guest privacy (cannot see playlist or other guests' requests),
    /// unified host & guest song submission, host-only control protection, and absence of approval/rejection routes.
    #[tokio::test]
    async fn test_lan_guest_isolation_privacy_unified_submission_and_no_approval_workflow_in_process() {
        use axum::body::Body;
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
        use axum::Json;
        use dhun_lib::network::local_helper_routes::{
            create_local_helper_router, create_local_room, get_guest_own_submissions,
            get_host_room_dashboard, guest_join_local_room, submit_song_to_local_room,
            CreateLocalRoomRequest, GuestJoinRoomRequest, LocalHelperState,
            UnifiedSongSubmissionRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;
        use tower_service::Service;

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_p2_iso_{}", uuid::Uuid::new_v4()));
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let state = Arc::new(LocalHelperState::new(
            db,
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(Some("192.168.1.50".to_string())),
            vec![],
        ));
        let lan_guest_addr: SocketAddr = "192.168.1.88:55123".parse().unwrap();

        // Create Room A and Room B
        let Json(room_a) = create_local_room(
            State(state.clone()),
            ConnectInfo("127.0.0.1:50000".parse().unwrap()),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Room A".to_string(),
                dj_level: Some(2),
                primary_mood_id: Some("shaadi".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let Json(room_b) = create_local_room(
            State(state.clone()),
            ConnectInfo("127.0.0.1:50000".parse().unwrap()),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Room B".to_string(),
                dj_level: Some(2),
                primary_mood_id: Some("baraat".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        // 1. Bare room_id alone WITHOUT access token or room code MUST be rejected (401)
        let bare_join = guest_join_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            HeaderMap::new(),
            Json(GuestJoinRoomRequest {
                nickname: "Intruder".to_string(),
                room_access_token: None,
                room_code: None,
            }),
        )
        .await;
        assert_eq!(bare_join.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 2. Guest 1 joins Room A using QR access token; Guest 2 joins Room A using Room Code
        let Json(g1) = guest_join_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            HeaderMap::new(),
            Json(GuestJoinRoomRequest {
                nickname: "Riya".to_string(),
                room_access_token: Some(room_a.room_access_token.clone()),
                room_code: None,
            }),
        )
        .await
        .unwrap();

        let Json(g2) = guest_join_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo("192.168.1.99:55124".parse().unwrap()),
            HeaderMap::new(),
            Json(GuestJoinRoomRequest {
                nickname: "Kabir".to_string(),
                room_access_token: None,
                room_code: Some(room_a.room.room_code.clone()),
            }),
        )
        .await
        .unwrap();

        let mut g1_headers = HeaderMap::new();
        g1_headers.insert("authorization", HeaderValue::from_str(&format!("Bearer {}", g1.guest_token)).unwrap());
        let mut g2_headers = HeaderMap::new();
        g2_headers.insert("authorization", HeaderValue::from_str(&format!("Bearer {}", g2.guest_token)).unwrap());
        let mut host_a_headers = HeaderMap::new();
        host_a_headers.insert("authorization", HeaderValue::from_str(&format!("Bearer {}", room_a.host_token)).unwrap());

        // 3. Cross-room isolation: Guest 1 from Room A MUST be rejected on Room B
        let cross_room = submit_song_to_local_room(
            State(state.clone()),
            Path(room_b.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            g1_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                authorized_stream: None,
            }),
        )
        .await;
        assert_eq!(cross_room.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 4. Host-only operations MUST reject guest tokens (403 Forbidden)
        let guest_dashboard_attempt = get_host_room_dashboard(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            g1_headers.clone(),
        )
        .await;
        assert_eq!(guest_dashboard_attempt.unwrap_err().0, StatusCode::FORBIDDEN);

        // 5. Unified submission: Guest 1, Guest 2, AND Host all use the exact same endpoint (zero approval queue)
        let Json(sub_g1) = submit_song_to_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            g1_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(sub_g1.submitted_by_type, "guest");
        assert_eq!(sub_g1.status, "received_by_dj");
        assert!(!sub_g1.interrupted_current_track);

        let Json(sub_g2) = submit_song_to_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo("192.168.1.99:55124".parse().unwrap()),
            g2_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://youtu.be/9bZkp7q19f0".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(sub_g2.submitted_by_type, "guest");

        let Json(sub_host) = submit_song_to_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo("127.0.0.1:50000".parse().unwrap()),
            host_a_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://music.youtube.com/watch?v=kJQP7kiw5Fk".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(sub_host.submitted_by_type, "host");

        // 6. Strict Guest Privacy: Guest 1 sees ONLY Guest 1's own song (NEVER Guest 2's or Host's!)
        let Json(g1_mine) = get_guest_own_submissions(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(lan_guest_addr),
            g1_headers,
        )
        .await
        .unwrap();
        assert_eq!(g1_mine.len(), 1);
        assert_eq!(g1_mine[0].id, sub_g1.id);

        // Host sees all 3 private candidate pool entries
        let Json(host_dash) = get_host_room_dashboard(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo("127.0.0.1:50000".parse().unwrap()),
            host_a_headers,
        )
        .await
        .unwrap();
        assert_eq!(host_dash.submissions.len(), 3);

        // 7. Verify absence of approval/rejection routes on the Local Helper Router (404 Not Found)
        let mut router = create_local_helper_router(state);
        for removed_path in [
            format!("/api/local/rooms/{}/requests/{}/approve", room_a.room.room_id, sub_g1.id),
            format!("/api/local/rooms/{}/requests/{}/reject", room_a.room.room_id, sub_g1.id),
            format!("/api/parties/{}/requests/{}/approve", room_a.room.room_id, sub_g1.id),
        ] {
            let req = Request::builder()
                .method("POST")
                .uri(&removed_path)
                .header("host", "127.0.0.1:47821")
                .body(Body::empty())
                .unwrap();
            let resp = router.call(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "Approval/rejection route '{}' must not exist",
                removed_path
            );
        }

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// In-process security test: enforces LAN-only peer IP filtering (blocks external non-LAN IPs)
    /// and Host-header DNS rebinding protection.
    #[test]
    fn test_local_helper_lan_ip_and_dns_rebinding_security_boundaries() {
        use axum::http::{HeaderMap, HeaderValue, StatusCode};
        use dhun_lib::network::local_helper_routes::verify_lan_and_host_headers;
        use std::net::IpAddr;

        let lan_ip = Some("192.168.1.42");

        // 1. External public IP outside LAN MUST be blocked (403)
        let public_ip: IpAddr = "203.0.113.55".parse().unwrap();
        let res_public = verify_lan_and_host_headers(public_ip, &HeaderMap::new(), lan_ip);
        assert_eq!(res_public.unwrap_err().0, StatusCode::FORBIDDEN);

        // 2. DNS rebinding Host header MUST be blocked (403) even from 127.0.0.1
        let mut rebind_headers = HeaderMap::new();
        rebind_headers.insert("host", HeaderValue::from_static("evil.rebind.attacker.com:47821"));
        let res_rebind = verify_lan_and_host_headers("127.0.0.1".parse().unwrap(), &rebind_headers, lan_ip);
        assert_eq!(res_rebind.unwrap_err().0, StatusCode::FORBIDDEN);

        // 3. Legitimate LAN peer + matching LAN Host header is permitted
        let mut valid_lan_headers = HeaderMap::new();
        valid_lan_headers.insert("host", HeaderValue::from_static("192.168.1.42:47821"));
        assert!(verify_lan_and_host_headers("192.168.1.91".parse().unwrap(), &valid_lan_headers, lan_ip).is_ok());

        // 4. Legitimate host loopback + localhost Host header is permitted
        let mut valid_local_headers = HeaderMap::new();
        valid_local_headers.insert("host", HeaderValue::from_static("localhost:47821"));
        assert!(verify_lan_and_host_headers("127.0.0.1".parse().unwrap(), &valid_local_headers, lan_ip).is_ok());
    }

    /// In-process service test: verifies Global Shared Song Metadata Catalogue isolation
    /// (zero party tables), HMAC-SHA256 engine-only auth (browser blocked), forbidden audio/party field rejection,
    /// duplicate URL merging, and distinct identities for remixes/live/original versions.
    #[tokio::test]
    async fn test_global_metadata_catalogue_isolation_dedup_and_engine_auth_in_process() {
        use axum::body::Bytes;
        use axum::extract::State;
        use axum::http::{HeaderMap, HeaderValue, StatusCode};
        use axum::Json;
        use dhun_lib::metadata_service::{
            catalogue_lookup, catalogue_upsert, extract_version_tag, GlobalCatalogueState,
        };
        use dhun_lib::sync::sign_request;
        use std::sync::Arc;

        // 1. Isolated cloud DB MUST contain ZERO party or guest tables
        let cloud_db = Database::new_global_catalogue_in_memory().unwrap();
        {
            let conn = cloud_db.conn();
            let guard = conn.lock().unwrap();
            let party_tables: i64 = guard
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('parties', 'local_party_rooms', 'party_guests', 'local_room_submissions')",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(party_tables, 0, "Global metadata database must contain ZERO party or guest tables");
        }

        let service_key = "dhun_engine_hmac_secret_2026".to_string();
        let state = Arc::new(GlobalCatalogueState::new(cloud_db, service_key.clone()));

        let make_signed_headers = |path: &str, body: &str| {
            let ts = chrono::Utc::now().timestamp();
            let sig = sign_request(&service_key, "POST", path, ts, body);
            let mut h = HeaderMap::new();
            h.insert("x-dhun-timestamp", HeaderValue::from_str(&ts.to_string()).unwrap());
            h.insert("x-dhun-signature", HeaderValue::from_str(&format!("sha256={}", sig)).unwrap());
            h
        };

        // 2. Unauthenticated browser request MUST fail with 401 Unauthorized
        let unauth_lookup = catalogue_lookup(
            State(state.clone()),
            HeaderMap::new(),
            Bytes::from_static(br#"{"provider_video_id":"dQw4w9WgXcQ"}"#),
        )
        .await;
        assert_eq!(unauth_lookup.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 3. Forbidden audio/party fields MUST be rejected with 400 Bad Request
        let bad_body = r#"{"source_url":"https://www.youtube.com/watch?v=dQw4w9WgXcQ","title":"Test","artist":"Artist","audio_base64":"AAAA"}"#;
        let bad_res = catalogue_upsert(
            State(state.clone()),
            make_signed_headers("/api/v1/catalogue/upsert", bad_body),
            Bytes::from(bad_body),
        )
        .await;
        assert_eq!(bad_res.unwrap_err().0, StatusCode::BAD_REQUEST);

        // 4. Upsert Original Song with BPM -> verifies half-time & double-time BPM calculation
        let orig_body = serde_json::json!({
            "source_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "title": "Chaiyya Chaiyya (Official Video)",
            "artist": "Sukhwinder Singh",
            "duration_ms": 296000,
            "original_bpm": 128.0,
            "bpm_confidence": 0.97,
            "musical_key": "D Major",
            "camelot_key": "10B",
            "key_confidence": 0.94,
            "overall_energy": 0.89,
            "danceability": 0.95,
            "genre_tags": ["Bollywood", "Wedding"],
            "mood_tags": ["shaadi", "sangeet"],
            "processing_status": "analyzed"
        })
        .to_string();

        let Json(saved_orig) = catalogue_upsert(
            State(state.clone()),
            make_signed_headers("/api/v1/catalogue/upsert", &orig_body),
            Bytes::from(orig_body),
        )
        .await
        .expect("Valid signed engine metadata upsert must succeed");
        assert_eq!(saved_orig.version_tag, "original");
        assert_eq!(saved_orig.original_bpm, Some(128.0));
        assert_eq!(saved_orig.half_time_bpm, Some(64.0));
        assert_eq!(saved_orig.double_time_bpm, Some(256.0));

        // 5. Duplicate YouTube Music URL for same video ID resolves to SAME song_id
        let dup_body = serde_json::json!({
            "source_url": "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
            "title": "Chaiyya Chaiyya",
            "artist": "Sukhwinder Singh",
            "duration_ms": 296000
        })
        .to_string();
        let Json(saved_dup) = catalogue_upsert(
            State(state.clone()),
            make_signed_headers("/api/v1/catalogue/upsert", &dup_body),
            Bytes::from(dup_body),
        )
        .await
        .unwrap();
        assert_eq!(saved_dup.song_id, saved_orig.song_id, "Duplicate URLs must deduplicate to same song_id");

        // 6. Remix & Live recordings MUST remain distinct song identities!
        assert_eq!(extract_version_tag("Chaiyya Chaiyya (DJ Remix)"), "remix");
        assert_eq!(extract_version_tag("Chaiyya Chaiyya (Live Concert)"), "live");

        let remix_body = serde_json::json!({
            "source_url": "https://www.youtube.com/watch?v=9bZkp7q19f0",
            "title": "Chaiyya Chaiyya (DJ Remix)",
            "artist": "Sukhwinder Singh",
            "duration_ms": 240000,
            "original_bpm": 134.0
        })
        .to_string();
        let Json(saved_remix) = catalogue_upsert(
            State(state.clone()),
            make_signed_headers("/api/v1/catalogue/upsert", &remix_body),
            Bytes::from(remix_body),
        )
        .await
        .unwrap();
        assert_eq!(saved_remix.version_tag, "remix");
        assert_ne!(saved_remix.song_id, saved_orig.song_id, "Remix must have distinct song identity");

        // 7. Reusable engine lookup by video ID
        let lookup_body = r#"{"provider_video_id":"dQw4w9WgXcQ"}"#;
        let Json(lookup_res) = catalogue_lookup(
            State(state.clone()),
            make_signed_headers("/api/v1/catalogue/lookup", lookup_body),
            Bytes::from_static(lookup_body.as_bytes()),
        )
        .await
        .unwrap();
        assert!(lookup_res.found);
        assert_eq!(lookup_res.metadata.unwrap().camelot_key.as_deref(), Some("10B"));

        // 8. Verify ZERO party routes and ZERO user-facing catalogue browse/search routes exist on cloud router
        use axum::body::Body;
        use axum::http::Request;
        use dhun_lib::metadata_service::create_global_catalogue_router;
        use tower_service::Service;
        let mut cloud_router = create_global_catalogue_router(state);
        for (method, forbidden_path) in [
            ("POST", "/api/parties"),
            ("GET", "/api/v1/catalogue/search"),
            ("GET", "/api/v1/catalogue/browse"),
            ("GET", "/api/v1/catalogue"),
        ] {
            let req = Request::builder()
                .method(method)
                .uri(forbidden_path)
                .body(Body::empty())
                .unwrap();
            let resp = cloud_router.call(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "Cloud metadata router must not expose '{} {}'",
                method,
                forbidden_path
            );
        }
    }

    /// In-process pipeline test: verifies the 6-stage local processing pipeline including
    /// honest RestrictedByProvider status for standard YouTube URLs, local Symphonia DSP analysis
    /// on locally cached authorized audio, and offline metadata-only sync outbox queueing.
    #[tokio::test]
    async fn test_local_6_stage_pipeline_analysis_and_offline_metadata_sync_queue() {
        use dhun_lib::library::cache::{RollingCacheManager, DEFAULT_MAX_CACHE_BYTES};
        use dhun_lib::library::downloader::AudioDownloader;
        use dhun_lib::library::pipeline::{run_local_song_pipeline, AcquisitionStatus};
        use dhun_lib::sync::MetadataSyncManager;

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_p2_pipe_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();

        let cache_mgr = RollingCacheManager::new(&temp_cache, DEFAULT_MAX_CACHE_BYTES);
        let downloader = AudioDownloader::new(&temp_cache);
        let offline_sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let client = reqwest::Client::new();

        // 1. Standard YouTube URL without unauthorized extraction -> RestrictedByProvider
        let restricted = run_local_song_pipeline(
            "rm_pipe_1",
            "sub_pipe_1",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            None,
            None,
            &db,
            &cache_mgr,
            &downloader,
            &offline_sync_mgr,
            &client,
        )
        .await
        .unwrap();
        assert_eq!(restricted.acquisition_status, AcquisitionStatus::RestrictedByProvider);
        assert!(restricted.cached_entry.is_none());
        assert!(restricted.sync_outbox_id.is_some());

        // 2. Authorized locally cached WAV audio -> full Stage 4 cache + Stage 5 local DSP analysis + Stage 6 offline sync queue
        let wav_path = temp_cache.join("authorized_cached_track.wav");
        let wav_bytes = create_test_wav_bytes(44100, 2, 132_300, 132_300); // 3.0 seconds valid stereo WAV
        std::fs::write(&wav_path, wav_bytes).unwrap();

        let analyzed = run_local_song_pipeline(
            "rm_pipe_1",
            "sub_pipe_2",
            "https://music.youtube.com/watch?v=9bZkp7q19f0",
            None,
            Some(&wav_path),
            &db,
            &cache_mgr,
            &downloader,
            &offline_sync_mgr,
            &client,
        )
        .await
        .unwrap();

        assert_eq!(analyzed.acquisition_status, AcquisitionStatus::CachedAuthorized);
        assert!(analyzed.cached_entry.is_some());
        let local_analysis = analyzed.local_analysis.expect("Stage 5 local audio analysis must succeed");
        assert!(local_analysis.original_bpm > 0.0);
        assert!((local_analysis.half_time_bpm - (local_analysis.original_bpm / 2.0)).abs() < 0.1);
        assert!((local_analysis.double_time_bpm - (local_analysis.original_bpm * 2.0)).abs() < 0.1);
        assert!(!local_analysis.camelot_key.is_empty());

        // Verify offline outbox queued strictly metadata-only items (zero audio bytes in payload)
        let pending = db.get_pending_sync_items(10).unwrap();
        assert!(pending.iter().any(|i| i.entity_type == "global_song_metadata" && i.entity_id == "9bZkp7q19f0"));
        for item in &pending {
            assert!(!item.payload_json.contains("audio_bytes"));
            assert!(!item.payload_json.contains("audio_base64"));
        }

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// In-process cache test: verifies 10-15 track rolling cap, priority locks for playing/upcoming tracks,
    /// launch-time expired cache file cleanup, and explicit room purge.
    #[test]
    fn test_local_temp_cache_15_track_cap_priority_and_expiry_cleanup() {
        use dhun_lib::library::cache::{
            RollingCacheManager, CACHE_PRIORITY_EVICTABLE, CACHE_PRIORITY_NEXT_DECK,
            CACHE_PRIORITY_PLAYING, CACHE_PRIORITY_UPCOMING, DEFAULT_MAX_CACHE_BYTES,
            DEFAULT_MAX_TEMP_TRACKS,
        };

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_p2_cache_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();
        let cache_mgr = RollingCacheManager::new(&temp_cache, DEFAULT_MAX_CACHE_BYTES);

        // Register 18 temporary files: first 3 are locked (playing, next_deck, upcoming)
        let mut locked_paths = Vec::new();
        for i in 0..18 {
            let p = temp_cache.join(format!("track_{:02}.wav", i));
            std::fs::write(&p, vec![0u8; 512]).unwrap();
            let (tier, locked) = match i {
                0 => (CACHE_PRIORITY_PLAYING, true),
                1 => (CACHE_PRIORITY_NEXT_DECK, true),
                2 => (CACHE_PRIORITY_UPCOMING, true),
                _ => (CACHE_PRIORITY_EVICTABLE, false),
            };
            if locked {
                locked_paths.push(p.clone());
            }
            cache_mgr
                .register_temp_track(
                    "rm_cache_test",
                    None,
                    &format!("gsng_{}", i),
                    &p,
                    tier,
                    locked,
                    None,
                    DEFAULT_MAX_TEMP_TRACKS,
                    &db,
                )
                .unwrap();
        }

        let stats = cache_mgr.get_temp_cache_stats(DEFAULT_MAX_TEMP_TRACKS, &db).unwrap();
        assert_eq!(stats.track_count, 15, "Rolling temp cache must cap at 15 tracks");
        assert_eq!(stats.locked_tracks, 3);
        for lp in &locked_paths {
            assert!(lp.exists(), "Priority-locked playing/upcoming track {:?} must never be evicted", lp);
        }

        // Insert an already-expired file and verify launch-time expiry sweep deletes it
        let expired_path = temp_cache.join("stale_expired_track.wav");
        std::fs::write(&expired_path, vec![0u8; 256]).unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        db.upsert_local_temp_cache(&LocalTempCacheEntry {
            id: "lcache_expired_1".to_string(),
            room_id: "rm_old_crashed_room".to_string(),
            submission_id: None,
            song_id: "gsng_old".to_string(),
            file_path: expired_path.to_string_lossy().to_string(),
            size_bytes: 256,
            priority_tier: 0,
            is_locked: false,
            created_at: now - 100_000,
            last_accessed_at: now - 100_000,
            expires_at: now - 5_000,
        })
        .unwrap();

        let expired_deleted = cache_mgr.cleanup_expired_on_launch(&db).unwrap();
        assert_eq!(expired_deleted, 1);
        assert!(!expired_path.exists(), "Expired cache file must be deleted on service launch");

        // Explicit room close purge removes all 15 remaining files for rm_cache_test
        let purged = cache_mgr.purge_room_cache("rm_cache_test", &db).unwrap();
        assert_eq!(purged, 15);
        assert_eq!(
            cache_mgr.get_temp_cache_stats(DEFAULT_MAX_TEMP_TRACKS, &db).unwrap().track_count,
            0
        );

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// In-process DJ engine test: verifies all 14 moods, all 4 DJ levels (with 4-deck internal capacity
    /// and universal automatic DJ support), and non-interrupting candidate evaluation.
    #[test]
    fn test_all_14_moods_4_dj_levels_and_non_interrupting_dj_evaluation() {
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::dj::moods::{get_mood_by_id, get_preset_moods};
        use dhun_lib::dj::selector::select_next_submission_without_interrupting;

        // 1. Verify all 14 mandatory moods
        let moods = get_preset_moods();
        assert_eq!(moods.len(), 14);
        for required_id in [
            "shaadi", "baraat", "mehendi", "haldi", "sangeet", "desi_club",
            "punjabi_tashan", "garba", "dandiya", "retro_bollywood", "house_party",
            "cocktail_night", "festival_mode", "international_party",
        ] {
            let m = get_mood_by_id(required_id).expect("Required mood must exist");
            assert!(m.min_bpm < m.max_bpm);
            assert!(!m.genre_weights.is_empty());
        }

        // 2. Verify all 4 DJ levels support automatic DJ playback & 4-deck internal architecture
        for (lvl_num, expected_exposed) in [(1, 2), (2, 2), (3, 4), (4, 4)] {
            let lvl = DjLevel::from_i32(lvl_num);
            assert!(lvl.supports_auto_dj(), "Level {} must support automatic party playback", lvl_num);
            assert_eq!(lvl.internal_deck_capacity(), 4, "Internal architecture must always support 4 decks");
            assert_eq!(lvl.max_decks(), expected_exposed);
        }

        // 3. Non-interrupting evaluation: currently playing track is preserved while best mood/BPM/key candidate is chosen
        let shaadi = get_mood_by_id("shaadi").unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let make_sub = |id: &str, status: &str, bpm: f64, camelot: &str, mood_tags: &str, acq: &str| LocalRoomSubmission {
            id: id.to_string(),
            room_id: "rm_dj".to_string(),
            submitted_by_type: "guest".to_string(),
            submitted_by_id: "g1".to_string(),
            submitted_by_name: "Guest".to_string(),
            provider: "youtube".to_string(),
            source_url: format!("https://www.youtube.com/watch?v={:0<11}", id),
            provider_video_id: format!("{:0<11}", id),
            song_id: None,
            title: id.to_string(),
            artist: "Artist".to_string(),
            version_tag: "original".to_string(),
            thumbnail_url: None,
            duration_ms: Some(210_000),
            bpm: Some(bpm),
            half_time_bpm: Some(bpm / 2.0),
            double_time_bpm: Some(bpm * 2.0),
            musical_key: Some("C Major".to_string()),
            camelot_key: Some(camelot.to_string()),
            overall_energy: Some(0.76),
            danceability: Some(0.90),
            genre_tags_json: r#"["Bollywood"]"#.to_string(),
            mood_tags_json: mood_tags.to_string(),
            metadata_source: "local_analysis".to_string(),
            acquisition_status: acq.to_string(),
            acquisition_note: None,
            dj_status: status.to_string(),
            dj_score: 0.0,
            local_cache_path: if acq == "cached_authorized" {
                Some(format!("/tmp/{}.wav", id))
            } else {
                None
            },
            cache_id: if acq == "cached_authorized" {
                Some(format!("lcache_{}", id))
            } else {
                None
            },
            created_at: now,
            updated_at: now,
        };

        let pool = vec![
            make_sub("playing_now", "playing", 124.0, "8B", r#"["shaadi"]"#, "cached_authorized"),
            make_sub("clash_track", "queued_for_dj", 92.0, "2A", r#"["cocktail_night"]"#, "restricted_by_provider"),
            make_sub("perfect_fit", "queued_for_dj", 125.0, "9B", r#"["shaadi"]"#, "cached_authorized"),
        ];

        let (scored, best_next) = select_next_submission_without_interrupting(&pool, &shaadi, None, shaadi.target_energy);
        assert!(
            scored.iter().all(|(id, _)| id != "playing_now"),
            "Currently playing track must NEVER be interrupted or re-queued during evaluation"
        );
        assert_eq!(best_next.as_deref(), Some("perfect_fit"));

        // 4. Metadata-only restricted_by_provider tracks must NEVER be chosen as best_playable_next
        let restricted_only_pool = vec![
            make_sub("restricted_hit", "queued_for_dj", 124.0, "8B", r#"["shaadi"]"#, "restricted_by_provider"),
        ];
        let (_, best_from_restricted) =
            select_next_submission_without_interrupting(&restricted_only_pool, &shaadi, None, shaadi.target_energy);
        assert!(
            best_from_restricted.is_none(),
            "restricted_by_provider tracks must NEVER be selected as playable DJ candidates"
        );
    }

    /// In-process audio streaming & security test: verifies `GET /api/local/rooms/:room_id/cache/:cache_id/stream`
    /// enforces host-only auth (401/403), cross-room ownership (404), canonical path traversal blocking (403),
    /// expired entry rejection (410 Gone), missing file handling (404), MIME sniffing, and Byte-Range (200/206/416).
    #[tokio::test]
    async fn test_local_cache_audio_streaming_auth_ownership_range_and_traversal_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue, StatusCode};
        use axum::Json;
        use dhun_lib::library::cache::{CACHE_PRIORITY_NEXT_DECK, DEFAULT_MAX_TEMP_TRACKS};
        use dhun_lib::network::local_helper_routes::{
            create_local_room, guest_join_local_room, stream_local_cached_audio,
            CreateLocalRoomRequest, GuestJoinRoomRequest, LocalHelperState,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;

        let db = Database::new_in_memory().unwrap();
        let temp_root = std::env::temp_dir().join(format!("dhun_p3_stream_{}", uuid::Uuid::new_v4()));
        let temp_cache = temp_root.join("cache");
        let outside_dir = temp_root.join("outside_secret");
        std::fs::create_dir_all(&temp_cache).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();

        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(Some("192.168.1.40".to_string())),
            vec!["https://dhun.vercel.app".to_string()],
        ));
        let host_addr: SocketAddr = "127.0.0.1:52100".parse().unwrap();
        let guest_addr: SocketAddr = "192.168.1.77:52101".parse().unwrap();

        let Json(room_a) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Stream Room A".to_string(),
                dj_level: Some(3),
                primary_mood_id: Some("shaadi".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let Json(room_b) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Stream Room B".to_string(),
                dj_level: Some(2),
                primary_mood_id: Some("baraat".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let Json(guest_a) = guest_join_local_room(
            State(state.clone()),
            Path(room_a.room.room_id.clone()),
            ConnectInfo(guest_addr),
            HeaderMap::new(),
            Json(GuestJoinRoomRequest {
                nickname: "Ananya".to_string(),
                room_access_token: Some(room_a.room_access_token.clone()),
                room_code: None,
            }),
        )
        .await
        .unwrap();

        let mut host_a_headers = HeaderMap::new();
        host_a_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", room_a.host_token)).unwrap(),
        );
        let mut host_b_headers = HeaderMap::new();
        host_b_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", room_b.host_token)).unwrap(),
        );
        let mut guest_a_headers = HeaderMap::new();
        guest_a_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", guest_a.guest_token)).unwrap(),
        );

        // Write valid 1.0s WAV file into Room A's managed cache dir
        let wav_bytes = create_test_wav_bytes(44100, 2, 44100, 44100);
        let wav_path = temp_cache.join("stream_ready_track.wav");
        std::fs::write(&wav_path, &wav_bytes).unwrap();
        let entry = state
            .cache_manager
            .register_temp_track(
                &room_a.room.room_id,
                None,
                "gsng_stream_1",
                &wav_path,
                CACHE_PRIORITY_NEXT_DECK,
                true,
                None,
                DEFAULT_MAX_TEMP_TRACKS,
                &db,
            )
            .unwrap();

        // 1. Unauthenticated caller -> 401 Unauthorized
        let unauth = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            HeaderMap::new(),
        )
        .await;
        assert_eq!(unauth.unwrap_err().0, StatusCode::UNAUTHORIZED);

        // 2. Guest caller -> 403 Forbidden (guests NEVER stream party audio!)
        let guest_res = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(guest_addr),
            guest_a_headers,
        )
        .await;
        assert_eq!(guest_res.unwrap_err().0, StatusCode::FORBIDDEN);

        // 3. Cross-room access (Host B requesting Room A's cache entry under Room B) -> 404 Not Found
        let cross_room = stream_local_cached_audio(
            State(state.clone()),
            Path((room_b.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            host_b_headers,
        )
        .await;
        assert_eq!(cross_room.unwrap_err().0, StatusCode::NOT_FOUND);

        // 4. Authorized Host A full stream -> 200 OK with audio/wav + Accept-Ranges: bytes
        let ok_resp = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            host_a_headers.clone(),
        )
        .await
        .expect("Host must be able to stream authorized cached audio");
        assert_eq!(ok_resp.status(), StatusCode::OK);
        assert_eq!(ok_resp.headers().get("content-type").unwrap(), "audio/wav");
        assert_eq!(ok_resp.headers().get("accept-ranges").unwrap(), "bytes");
        let full_body = axum::body::to_bytes(ok_resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(full_body.len(), wav_bytes.len());
        assert_eq!(&full_body[0..4], b"RIFF");

        // 5. Byte-Range request (bytes=0-1023) -> 206 Partial Content
        let mut range_headers = host_a_headers.clone();
        range_headers.insert("range", HeaderValue::from_static("bytes=0-1023"));
        let partial_resp = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            range_headers,
        )
        .await
        .unwrap();
        assert_eq!(partial_resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            partial_resp.headers().get("content-range").unwrap(),
            &format!("bytes 0-1023/{}", wav_bytes.len())
        );
        let partial_body = axum::body::to_bytes(partial_resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(partial_body.len(), 1024);

        // 6. Out-of-bounds Byte-Range -> 416 Range Not Satisfiable
        let mut bad_range_headers = host_a_headers.clone();
        bad_range_headers.insert("range", HeaderValue::from_static("bytes=99999999-"));
        let unsatisfiable = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            bad_range_headers,
        )
        .await
        .unwrap();
        assert_eq!(unsatisfiable.status(), StatusCode::RANGE_NOT_SATISFIABLE);

        // 7. Path traversal outside cache_dir -> 403 Forbidden
        let secret_file = outside_dir.join("host_secret.txt");
        std::fs::write(&secret_file, b"TOP_SECRET_OUTSIDE_CACHE").unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        db.upsert_local_temp_cache(&LocalTempCacheEntry {
            id: "lcache_traversal".to_string(),
            room_id: room_a.room.room_id.clone(),
            submission_id: None,
            song_id: "gsng_evil".to_string(),
            file_path: secret_file.to_string_lossy().to_string(),
            size_bytes: 24,
            priority_tier: 0,
            is_locked: false,
            created_at: now,
            last_accessed_at: now,
            expires_at: now + 3_600_000,
        })
        .unwrap();
        let traversal = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), "lcache_traversal".to_string())),
            ConnectInfo(host_addr),
            host_a_headers.clone(),
        )
        .await;
        assert_eq!(traversal.unwrap_err().0, StatusCode::FORBIDDEN);

        // 8. Expired cache entry -> 410 Gone
        let expired_wav_path = temp_cache.join("expired_stream_track.wav");
        std::fs::write(&expired_wav_path, &wav_bytes).unwrap();
        db.upsert_local_temp_cache(&LocalTempCacheEntry {
            id: "lcache_expired_stream".to_string(),
            room_id: room_a.room.room_id.clone(),
            submission_id: None,
            song_id: "gsng_exp".to_string(),
            file_path: expired_wav_path.to_string_lossy().to_string(),
            size_bytes: wav_bytes.len() as i64,
            priority_tier: 0,
            is_locked: false,
            created_at: now - 7_200_000,
            last_accessed_at: now - 7_200_000,
            expires_at: now - 1_000,
        })
        .unwrap();
        let expired = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), "lcache_expired_stream".to_string())),
            ConnectInfo(host_addr),
            host_a_headers.clone(),
        )
        .await;
        assert_eq!(expired.unwrap_err().0, StatusCode::GONE);

        // 9. Deleted/missing backing file -> 404 Not Found
        std::fs::remove_file(&wav_path).unwrap();
        let missing = stream_local_cached_audio(
            State(state.clone()),
            Path((room_a.room.room_id.clone(), entry.id.clone())),
            ConnectInfo(host_addr),
            host_a_headers,
        )
        .await;
        assert_eq!(missing.unwrap_err().0, StatusCode::NOT_FOUND);

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    /// In-process playback lifecycle test: verifies `POST /dj/playback-event` cache locking,
    /// decode failure recovery without interrupting active deck, safe deferred room close
    /// preserving locked in-use audio (60m expiry fallback) while deleting unlocked files,
    /// `POST /cache/release` final cleanup, and repeat-avoiding DJ candidate selection.
    #[tokio::test]
    async fn test_playback_event_locking_safe_close_and_non_repeating_dj_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue};
        use axum::Json;
        use dhun_lib::library::cache::{
            CACHE_PRIORITY_EVICTABLE, DEFAULT_MAX_TEMP_TRACKS, DEFAULT_TEMP_CACHE_TTL_MS,
        };
        use dhun_lib::network::local_helper_routes::{
            close_local_room, create_local_room, get_host_room_dashboard, record_dj_playback_event,
            release_room_cache, CreateLocalRoomRequest, LocalHelperState, PlaybackEventRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;

        // Verify shortened cache TTL fallback is strictly 60 minutes (within 30m-2h window)
        assert_eq!(DEFAULT_TEMP_CACHE_TTL_MS, 60 * 60 * 1000);

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_p3_life_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(Some("192.168.1.41".to_string())),
            vec![],
        ));
        let host_addr: SocketAddr = "127.0.0.1:52200".parse().unwrap();

        let Json(created) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Safe Close Lifecycle Test".to_string(),
                dj_level: Some(3),
                primary_mood_id: Some("shaadi".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let mut host_headers = HeaderMap::new();
        host_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", created.host_token)).unwrap(),
        );

        // Seed 3 cached tracks: sub_1 (will play), sub_2 (corrupt decode fail), sub_3 (clean next track)
        let now = chrono::Utc::now().timestamp_millis();
        let mut paths = Vec::new();
        for (idx, (sub_id, vid, title)) in [
            ("sub_play_1", "vidAAA11111", "Track One"),
            ("sub_bad_2", "vidBBB22222", "Track Two Broken"),
            ("sub_good_3", "vidCCC33333", "Track Three Clean"),
        ]
        .iter()
        .enumerate()
        {
            let p = temp_cache.join(format!("{}.wav", sub_id));
            std::fs::write(&p, create_test_wav_bytes(44100, 2, 44100, 44100)).unwrap();
            paths.push(p.clone());

            db.insert_local_room_submission(&LocalRoomSubmission {
                id: sub_id.to_string(),
                room_id: created.room.room_id.clone(),
                submitted_by_type: "host".to_string(),
                submitted_by_id: "host".to_string(),
                submitted_by_name: "Host".to_string(),
                provider: "youtube".to_string(),
                source_url: format!("https://www.youtube.com/watch?v={}", vid),
                provider_video_id: vid.to_string(),
                song_id: Some(format!("gsng_{}", idx)),
                title: title.to_string(),
                artist: "Artist".to_string(),
                version_tag: "original".to_string(),
                thumbnail_url: None,
                duration_ms: Some(180_000),
                bpm: Some(126.0),
                half_time_bpm: Some(63.0),
                double_time_bpm: Some(252.0),
                musical_key: Some("C Major".to_string()),
                camelot_key: Some("8B".to_string()),
                overall_energy: Some(0.80),
                danceability: Some(0.90),
                genre_tags_json: r#"["Bollywood"]"#.to_string(),
                mood_tags_json: r#"["shaadi"]"#.to_string(),
                metadata_source: "local_analysis".to_string(),
                acquisition_status: "cached_authorized".to_string(),
                acquisition_note: None,
                dj_status: "queued_for_dj".to_string(),
                dj_score: 0.0,
                local_cache_path: Some(p.to_string_lossy().to_string()),
                cache_id: None,
                created_at: now + idx as i64,
                updated_at: now + idx as i64,
            })
            .unwrap();

            state
                .cache_manager
                .register_temp_track(
                    &created.room.room_id,
                    Some(sub_id),
                    &format!("gsng_{}", idx),
                    &p,
                    CACHE_PRIORITY_EVICTABLE,
                    false,
                    None,
                    DEFAULT_MAX_TEMP_TRACKS,
                    &db,
                )
                .unwrap();
        }

        // Verify dashboard exposes opaque `cache_id` and NEVER raw `local_cache_path` in JSON
        let Json(dash) = get_host_room_dashboard(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .unwrap();
        let dash_json = serde_json::to_string(&dash).unwrap();
        assert!(!dash_json.contains("local_cache_path"), "Raw filesystem paths must NEVER be serialized to API JSON");
        assert!(dash.submissions.iter().all(|s| s.cache_id.is_some()));

        // 1. Mark sub_play_1 as actively `playing` -> locks its cache file!
        let Json(ev_play) = record_dj_playback_event(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(PlaybackEventRequest {
                submission_id: "sub_play_1".to_string(),
                event: "playing".to_string(),
                deck_id: Some(0),
                error_detail: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(ev_play.dj_status, "playing");
        let c1 = db
            .get_temp_cache_entry_by_submission(&created.room.room_id, "sub_play_1")
            .unwrap()
            .unwrap();
        assert!(c1.is_locked, "Actively playing track must be cache-locked");

        // 2. Simulate decode failure on sub_bad_2 -> marked `skipped_unplayable`, sub_play_1 untouched, next playable advances to sub_good_3
        let Json(ev_fail) = record_dj_playback_event(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(PlaybackEventRequest {
                submission_id: "sub_bad_2".to_string(),
                event: "decode_failed".to_string(),
                deck_id: Some(1),
                error_detail: Some("Simulated Web Audio decode error".to_string()),
            }),
        )
        .await
        .unwrap();
        assert_eq!(ev_fail.dj_status, "skipped_unplayable");
        assert_eq!(
            ev_fail.next_playable_candidate.as_ref().map(|c| c.submission_id.as_str()),
            Some("sub_good_3"),
            "Next playable candidate must cleanly recover to sub_good_3"
        );

        // 3. Safe room closure (`force_release_audio: false` / None):
        //    deletes unlocked files (sub_bad_2, sub_good_3) immediately, preserves locked sub_play_1!
        let Json(safe_closed) = close_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(safe_closed["status"], "closed");
        assert_eq!(safe_closed["purged_cache_files"], 2);
        assert_eq!(safe_closed["deferred_locked_files"], 1);
        assert!(paths[0].exists(), "In-use locked audio file MUST NOT be deleted mid-playback on close");
        assert!(!paths[1].exists(), "Unlocked cache file must be deleted immediately on close");
        assert!(!paths[2].exists(), "Unlocked cache file must be deleted immediately on close");

        // 4. Once Web Audio finishes fade-out, `release_room_cache` purges the deferred file cleanly
        let Json(released) = release_room_cache(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers,
        )
        .await
        .unwrap();
        assert_eq!(released["purged_cache_files"], 1);
        assert!(!paths[0].exists(), "Deferred locked audio file must be deleted upon release");

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// In-process concurrency & schema hygiene test: verifies Migration v5 removed all obsolete
    /// Phase 1 cloud-party tables and tests concurrent multi-guest song submissions against SQLite.
    #[tokio::test]
    async fn test_concurrent_song_submissions_and_migration_v5_cleanup_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue};
        use axum::Json;
        use dhun_lib::network::local_helper_routes::{
            create_local_room, guest_join_local_room, submit_song_to_local_room,
            CreateLocalRoomRequest, GuestJoinRoomRequest, LocalHelperState,
            UnifiedSongSubmissionRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;

        let db = Database::new_in_memory().unwrap();

        // 1. Verify Migration v5 dropped obsolete Phase 1 cloud-party tables
        {
            let conn = db.conn();
            let guard = conn.lock().unwrap();
            let obsolete_count: i64 = guard
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN (
                        'parties', 'party_configurations', 'party_source_references',
                        'party_guests', 'party_song_requests', 'request_status_history'
                    )",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                obsolete_count, 0,
                "Migration v5 must drop all obsolete Phase 1 cloud-party tables"
            );
        }

        let temp_cache = std::env::temp_dir().join(format!("dhun_p3_conc_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(Some("192.168.1.42".to_string())),
            vec![],
        ));
        let host_addr: SocketAddr = "127.0.0.1:52300".parse().unwrap();

        let Json(room) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Concurrency Stress Party".to_string(),
                dj_level: Some(2),
                primary_mood_id: Some("desi_club".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        // Admit 4 guests on LAN and submit 4 different YouTube URLs concurrently
        let mut guest_tokens = Vec::new();
        for i in 0..4 {
            let g_addr: SocketAddr = format!("192.168.1.{}:5500{}", 100 + i, i).parse().unwrap();
            let Json(g) = guest_join_local_room(
                State(state.clone()),
                Path(room.room.room_id.clone()),
                ConnectInfo(g_addr),
                HeaderMap::new(),
                Json(GuestJoinRoomRequest {
                    nickname: format!("Guest_{}", i),
                    room_access_token: Some(room.room_access_token.clone()),
                    room_code: None,
                }),
            )
            .await
            .unwrap();
            guest_tokens.push((g_addr, g.guest_token));
        }

        let video_ids = ["dQw4w9WgXcQ", "9bZkp7q19f0", "kJQP7kiw5Fk", "JGwWNGJdvx8"];
        let mut futs = Vec::new();
        for (idx, (g_addr, tok)) in guest_tokens.into_iter().enumerate() {
            let st = state.clone();
            let rid = room.room.room_id.clone();
            let vid = video_ids[idx];
            futs.push(tokio::spawn(async move {
                let mut h = HeaderMap::new();
                h.insert("authorization", HeaderValue::from_str(&format!("Bearer {}", tok)).unwrap());
                submit_song_to_local_room(
                    State(st),
                    Path(rid),
                    ConnectInfo(g_addr),
                    h,
                    Json(UnifiedSongSubmissionRequest {
                        url: format!("https://www.youtube.com/watch?v={}", vid),
                        authorized_stream: None,
                    }),
                )
                .await
            }));
        }

        for f in futs {
            let res = f.await.expect("Task must not panic").expect("Concurrent submission must succeed");
            assert_eq!(res.0.status, "received_by_dj");
        }

        let all_subs = db.get_local_room_submissions(&room.room.room_id).unwrap();
        assert_eq!(all_subs.len(), 4, "All 4 concurrent guest submissions must be persisted cleanly");

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// Verifies AuthorizedResolverConfig anti-SSRF/HTTPS enforcement and direct audio byte
    /// Symphonia validation + cache ingestion (accepting valid WAV, rejecting garbage cleanly).
    #[test]
    fn test_authorized_resolver_validation_and_direct_audio_cache_ingest() {
        use dhun_lib::library::resolver::{
            save_and_ingest_audio_bytes_to_cache, AuthorizedResolverConfig, ResolverJsonResponse,
        };

        // 1. Resolver config strictly enforces HTTPS and blocks localhost/private IPs
        assert!(AuthorizedResolverConfig::new("http://example.com/resolve".to_string(), None).is_err());
        assert!(AuthorizedResolverConfig::new("https://127.0.0.1/resolve".to_string(), None).is_err());
        assert!(AuthorizedResolverConfig::new("https://localhost/resolve".to_string(), None).is_err());
        assert!(AuthorizedResolverConfig::new("https://192.168.1.10/resolve".to_string(), None).is_err());
        assert!(AuthorizedResolverConfig::new("   ".to_string(), None).is_err());

        // 2. JSON response contract deserialization
        let parsed_json: ResolverJsonResponse = serde_json::from_str(
            r#"{"direct_https_audio_url":"https://media.example.com/a.mp3","license_or_authorization_note":"CC-BY","expected_sha256":null}"#,
        )
        .unwrap();
        assert_eq!(parsed_json.direct_https_audio_url, "https://media.example.com/a.mp3");
        assert_eq!(parsed_json.license_or_authorization_note.as_deref(), Some("CC-BY"));

        // 3. Direct audio byte ingestion validates via Symphonia, caches WAV, and cleans up garbage
        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_resolver_ingest_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();

        let bad_res = save_and_ingest_audio_bytes_to_cache(
            b"<html>not audio</html>",
            "dQw4w9WgXcQ",
            "https://resolver.example.com/audio",
            &temp_cache,
            &db,
        );
        assert!(bad_res.is_err(), "Non-audio HTML payload must be rejected");
        let leftover_parts: Vec<_> = std::fs::read_dir(&temp_cache)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(leftover_parts.is_empty(), "Rejected payloads must not leave orphaned temp files");

        let wav_bytes = create_test_wav_bytes(44100, 2, 44100, 44100);
        let (cached_path, sha256) = save_and_ingest_audio_bytes_to_cache(
            &wav_bytes,
            "dQw4w9WgXcQ",
            "https://resolver.example.com/audio",
            &temp_cache,
            &db,
        )
        .expect("Valid WAV bytes from resolver must ingest and cache cleanly");
        assert!(cached_path.exists());
        assert_eq!(sha256.len(), 64);

        // 4. Explicit loopback resolver opt-in accepts 127.0.0.1/localhost without Cloudflare Tunnel,
        //    while STILL rejecting private LAN IPs, metadata IPs, and non-HTTPS remote hosts
        let loopback_cfg = AuthorizedResolverConfig::new_with_loopback(
            "http://127.0.0.1:8000/resolve".to_string(),
            None,
            true,
        )
        .expect("Loopback opt-in must accept http://127.0.0.1:8000/resolve");
        assert!(loopback_cfg.allow_loopback);
        assert!(AuthorizedResolverConfig::new_with_loopback(
            "http://192.168.1.5:8000/resolve".to_string(),
            None,
            true
        )
        .is_err());
        assert!(AuthorizedResolverConfig::new_with_loopback(
            "http://169.254.169.254/latest".to_string(),
            None,
            true
        )
        .is_err());
        assert!(AuthorizedResolverConfig::new_with_loopback(
            "http://example.com/resolve".to_string(),
            None,
            true
        )
        .is_err());

        // 5. YouTube keyword/word-match & userinfo spoofing validation
        use dhun_lib::library::youtube::parse_youtube_url;
        assert!(parse_youtube_url("https://youtube.com@evil.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(parse_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&x=../etc").is_err());
        assert!(parse_youtube_url("https://www.youtube.com/watch?v=dQw4w\n9WgXcQ").is_err());
        assert!(parse_youtube_url("https://example.com/watch?v=dQw4w9WgXcQ").is_err());

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// Verifies persistent asynchronous submission intake queue:
    /// - Submitting multiple songs returns immediately with `received` (zero blocking on metadata/audio)
    /// - Jobs persist in SQLite `submission_intake_jobs`
    /// - Interrupted in-flight jobs recover cleanly on restart and process to completion
    #[tokio::test]
    async fn test_async_persistent_intake_queue_non_blocking_and_crash_recovery_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue};
        use axum::Json;
        use dhun_lib::network::local_helper_routes::{
            create_local_room, get_host_room_dashboard, process_single_intake_job,
            submit_song_to_local_room, CreateLocalRoomRequest, LocalHelperState,
            UnifiedSongSubmissionRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use std::net::SocketAddr;
        use std::sync::Arc;

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_async_q_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_cache).unwrap();
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let state = Arc::new(LocalHelperState::new(
            db.clone(),
            47821,
            temp_cache.clone(),
            sync_mgr,
            Some(Some("192.168.1.55".to_string())),
            vec![],
        ));
        let host_addr: SocketAddr = "127.0.0.1:53000".parse().unwrap();

        let Json(created) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Async Queue Test".to_string(),
                dj_level: Some(3),
                primary_mood_id: Some("shaadi".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let mut host_headers = HeaderMap::new();
        host_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", created.host_token)).unwrap(),
        );

        // Submit 3 links back-to-back -> each must return immediately as `received`
        for vid in ["dQw4w9WgXcQ", "9bZkp7q19f0", "kJQP7kiw5Fk"] {
            let Json(sub) = submit_song_to_local_room(
                State(state.clone()),
                Path(created.room.room_id.clone()),
                ConnectInfo(host_addr),
                host_headers.clone(),
                Json(UnifiedSongSubmissionRequest {
                    url: format!("https://www.youtube.com/watch?v={}", vid),
                    authorized_stream: None,
                }),
            )
            .await
            .unwrap();
            assert_eq!(sub.acquisition_status, "received");
            assert_eq!(sub.status, "received_by_dj");
        }

        assert_eq!(db.get_pending_intake_job_count(&created.room.room_id).unwrap(), 3);

        // Claim job 1 (simulating mid-flight processing), then simulate crash/restart recovery
        let claimed = db.claim_next_submission_intake_job().unwrap().expect("Must claim job 1");
        assert_eq!(claimed.status, "metadata_processing");

        let recovered = db.recover_interrupted_intake_jobs().unwrap();
        assert_eq!(recovered, 1, "Interrupted in-flight intake job must recover cleanly on restart");

        // Drain all 3 jobs cleanly via the background worker helper
        let mut drained = 0;
        while let Ok(Some(job)) = db.claim_next_submission_intake_job() {
            assert!(process_single_intake_job(&state, &job).await);
            drained += 1;
        }
        assert_eq!(drained, 3);
        assert_eq!(db.get_pending_intake_job_count(&created.room.room_id).unwrap(), 0);

        // Dashboard must report truthful recommendations + skill profile + 0 pending intake jobs
        let Json(dash) = get_host_room_dashboard(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers,
        )
        .await
        .unwrap();
        assert_eq!(dash.pending_intake_jobs, 0);
        assert_eq!(dash.dj_skill_profile.level, 3);
        assert!(!dash.recommendations.is_empty(), "Recommendations must be produced even for metadata-only tracks");
        assert!(
            dash.recommendations.iter().all(|r| !r.is_playable && r.playability_label == "metadata_only_restricted"),
            "Metadata-only recommendations must be honestly labeled as not playable"
        );

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// Verifies DJ Levels 1–4 have genuinely distinct, strictly inheriting skill profiles and select
    /// different mixing techniques for the exact same pair of tracks, while falling back safely when
    /// BPM/key metadata is incomplete.
    #[test]
    fn test_dj_level_1_to_4_distinct_techniques_and_incomplete_metadata_recommendations() {
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::dj::moods::get_mood_by_id;
        use dhun_lib::dj::selector::{build_dj_recommendations, choose_technique_for_pair};

        // 1. Strict capability & technique inheritance across Levels 1 -> 4
        let p1 = DjLevel::Level1TwoDeckMixer.skill_profile();
        let p2 = DjLevel::Level2SmartMixing.skill_profile();
        let p3 = DjLevel::Level3AdvancedPerformance.skill_profile();
        let p4 = DjLevel::Level4CreativeTools.skill_profile();

        assert!(p1.unlocked_techniques.len() < p2.unlocked_techniques.len());
        assert!(p2.unlocked_techniques.len() < p3.unlocked_techniques.len());
        assert!(p3.unlocked_techniques.len() < p4.unlocked_techniques.len());
        for t in &p1.unlocked_techniques {
            assert!(p2.unlocked_techniques.contains(t));
            assert!(p3.unlocked_techniques.contains(t));
            assert!(p4.unlocked_techniques.contains(t));
        }

        let shaadi = get_mood_by_id("shaadi").unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        let make_track = |id: &str, status: &str, bpm: Option<f64>, key: Option<&str>, energy: Option<f64>, acq: &str| {
            LocalRoomSubmission {
                id: id.to_string(),
                room_id: "rm_lvl".to_string(),
                submitted_by_type: "host".to_string(),
                submitted_by_id: "host".to_string(),
                submitted_by_name: "Host".to_string(),
                provider: "youtube".to_string(),
                source_url: format!("https://www.youtube.com/watch?v={:0<11}", id),
                provider_video_id: format!("{:0<11}", id),
                song_id: Some(format!("gsng_{}", id)),
                title: format!("Song {}", id),
                artist: format!("Artist {}", id),
                version_tag: "original".to_string(),
                thumbnail_url: None,
                duration_ms: Some(200_000),
                bpm,
                half_time_bpm: bpm.map(|b| b / 2.0),
                double_time_bpm: bpm.map(|b| b * 2.0),
                musical_key: key.map(|k| k.to_string()),
                camelot_key: key.map(|k| k.to_string()),
                overall_energy: energy,
                danceability: Some(0.88),
                genre_tags_json: r#"["Bollywood"]"#.to_string(),
                mood_tags_json: r#"["shaadi"]"#.to_string(),
                metadata_source: "local_analysis".to_string(),
                acquisition_status: acq.to_string(),
                acquisition_note: None,
                dj_status: status.to_string(),
                dj_score: 0.0,
                local_cache_path: if acq == "cached_authorized" { Some("/tmp/t.wav".to_string()) } else { None },
                cache_id: if acq == "cached_authorized" { Some(format!("c_{}", id)) } else { None },
                created_at: now,
                updated_at: now,
            }
        };

        // 2. Acceptance Test: Exact same harmonic pair produces 4 distinct techniques at L1, L2, L3, L4!
        let playing = make_track("cur", "playing", Some(126.0), Some("8B"), Some(0.80), "cached_authorized");
        let candidate = make_track("nxt", "queued_for_dj", Some(128.0), Some("9B"), Some(0.84), "cached_authorized");

        let (t1, _) = choose_technique_for_pair(DjLevel::Level1TwoDeckMixer, Some(&playing), &candidate, &shaadi, 0.95);
        let (t2, _) = choose_technique_for_pair(DjLevel::Level2SmartMixing, Some(&playing), &candidate, &shaadi, 0.95);
        let (t3, _) = choose_technique_for_pair(DjLevel::Level3AdvancedPerformance, Some(&playing), &candidate, &shaadi, 0.95);
        let (t4, _) = choose_technique_for_pair(DjLevel::Level4CreativeTools, Some(&playing), &candidate, &shaadi, 0.95);

        assert_eq!(t1, "phrase_long_blend");
        assert_eq!(t2, "timed_bass_swap");
        assert_eq!(t3, "four_deck_layer");
        assert_eq!(t4, "creative_mashup_layer");

        // 3. Incomplete metadata test: missing BPM & key lowers confidence, explains what's missing,
        //    and selects a safe transition fallback instead of pretending beat alignment!
        let incomplete = make_track("inc", "queued_for_dj", None, None, None, "restricted_by_provider");
        let recs = build_dj_recommendations(
            &[playing.clone(), incomplete],
            &shaadi,
            None,
            shaadi.target_energy,
            DjLevel::Level2SmartMixing,
        );
        assert_eq!(recs.len(), 1);
        assert!(!recs[0].is_playable);
        assert_eq!(recs[0].playability_label, "metadata_only_restricted");
        assert!(recs[0].confidence < 0.45);
        assert!(recs[0].missing_signals.iter().any(|m| m.contains("BPM")));
        assert!(recs[0].missing_signals.iter().any(|m| m.contains("key")));
        assert_eq!(recs[0].suggested_technique, "safe_quick_cut");
    }

    /// Regression test: verifies PhraseDetector performs real deterministic DSP bar/phrase analysis
    /// (detecting intro, groove, breakdown, build, drop/peak, outro with confidence) and stays honest
    /// on flat audio (low-confidence "phrase" fallback, never fabricating semantic drops).
    #[test]
    fn test_phrase_detector_real_dsp_sections_and_flat_audio_honesty() {
        use dhun_lib::analysis::phrase_detector::PhraseDetector;
        use std::sync::Arc;

        let sr = 44_100u32;
        let bpm = 120.0; // 1 beat = 0.5s, 1 bar = 2s, 8-bar phrase = 16s
        let duration_sec = 96usize; // 6 x 16s phrases
        let total_frames = sr as usize * duration_sec;
        let mut samples = Vec::with_capacity(total_frames * 2);

        // Build 6 distinct 16-second structural sections:
        // 0: intro (low amp 0.08, high tone)
        // 1: groove (mid amp 0.28, steady 90 Hz bass + 440 Hz lead)
        // 2: breakdown (low amp 0.10, no bass — only 880 Hz pad)
        // 3: build (rising amp 0.14 -> 0.40)
        // 4: drop/peak (high amp 0.55, strong 65 Hz sub-bass + 440 Hz synth)
        // 5: outro (quiet amp 0.09)
        for frame in 0..total_frames {
            let t = frame as f64 / sr as f64;
            let phrase_idx = (frame / (sr as usize * 16)).min(5);
            let local_frac = (frame % (sr as usize * 16)) as f64 / (sr as f64 * 16.0);
            let s = match phrase_idx {
                0 => 0.08 * (2.0 * std::f64::consts::PI * 520.0 * t).sin(),
                1 => {
                    0.22 * (2.0 * std::f64::consts::PI * 90.0 * t).sin()
                        + 0.14 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()
                }
                2 => 0.10 * (2.0 * std::f64::consts::PI * 880.0 * t).sin(),
                3 => {
                    let amp = 0.14 + 0.24 * local_frac;
                    amp * (2.0 * std::f64::consts::PI * 330.0 * t).sin()
                }
                4 => {
                    0.45 * (2.0 * std::f64::consts::PI * 65.0 * t).sin()
                        + 0.28 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()
                }
                _ => 0.09 * (2.0 * std::f64::consts::PI * 392.0 * t).sin(),
            } as f32;
            samples.push(s);
            samples.push(s);
        }

        let (phrases, energy, _dance, mix_in_ms, mix_out_ms) =
            PhraseDetector::analyze_phrasing_with_downbeat(
                &Arc::new(samples),
                sr,
                bpm,
                0,
                (duration_sec * 1000) as u64,
            );

        assert_eq!(phrases.len(), 6, "Expected 6 beat/phrase-aligned 8-bar sections");
        assert!(energy > 0.2);
        assert_eq!(phrases[0].section_type, "intro");
        assert_eq!(phrases[5].section_type, "outro");
        assert!(
            phrases.iter().any(|p| p.section_type == "breakdown" && p.confidence >= 0.55),
            "Must detect low-bass/low-energy breakdown cleanly: {:?}",
            phrases
        );
        assert!(
            phrases.iter().any(|p| (p.section_type == "drop" || p.section_type == "peak") && p.confidence >= 0.60),
            "Must detect high-energy bass drop/peak cleanly: {:?}",
            phrases
        );
        assert!(mix_in_ms > 0, "Recommended mix-in should skip quiet intro");
        assert!(mix_out_ms < (duration_sec * 1000) as i64, "Recommended mix-out should precede outro");

        // Flat constant-energy sine wave: MUST stay honest and NOT fabricate confident drops/breakdowns
        let flat_frames = sr as usize * 48;
        let mut flat_samples = Vec::with_capacity(flat_frames * 2);
        for frame in 0..flat_frames {
            let t = frame as f64 / sr as f64;
            let s = (0.25 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as f32;
            flat_samples.push(s);
            flat_samples.push(s);
        }
        let (flat_phrases, _, _, _, _) = PhraseDetector::analyze_phrasing_with_downbeat(
            &Arc::new(flat_samples),
            sr,
            bpm,
            0,
            48_000,
        );
        assert!(
            flat_phrases.iter().all(|p| p.confidence <= 0.50),
            "Flat audio must report low structural confidence rather than faking drops: {:?}",
            flat_phrases
        );
        assert!(
            flat_phrases.iter().any(|p| p.section_type == "phrase"),
            "Flat audio body must honestly fall back to generic 'phrase': {:?}",
            flat_phrases
        );
    }

    /// Regression test for async submission intake queue:
    /// - Rapid duplicate URL deduplication (returns same submission_id, zero duplicate rows/jobs)
    /// - Authorized audio end-to-end persistence of `cache_id` + `next_playable_candidate` + real `sections`
    /// - Transient failure automatic retry (attempts < 3) without duplicating submissions
    /// - Failed submission manual re-submission reuses the SAME `submission_id` (`retry_queued`)
    /// - Stale in-flight job recovery (`recover_stale_intake_jobs`)
    /// - WebSocket `dj.submission_ready` payload matches persisted SQLite DB state (`cache_id`, `acquisition_status`)
    #[tokio::test]
    async fn test_async_intake_queue_authorized_audio_cache_id_dedup_retry_and_stale_recovery_in_process() {
        use axum::extract::{ConnectInfo, Path, State};
        use axum::http::{HeaderMap, HeaderValue};
        use axum::routing::get;
        use axum::{Json, Router};
        use dhun_lib::library::pipeline::AuthorizedStreamProof;
        use dhun_lib::library::resolver::AuthorizedResolverConfig;
        use dhun_lib::network::local_helper_routes::{
            create_local_room, get_host_room_dashboard, process_single_intake_job,
            start_local_room, submit_song_to_local_room, CreateLocalRoomRequest,
            LocalHelperState, UnifiedSongSubmissionRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use sha2::{Digest, Sha256};
        use std::net::SocketAddr;
        use std::sync::Arc;

        // 1. Build a valid synthetic 4-second 16-bit stereo 44.1kHz WAV fixture in memory
        let sample_rate: u32 = 44_100;
        let num_frames: u32 = sample_rate * 4;
        let data_bytes: u32 = num_frames * 2 * 2;
        let mut wav_bytes = Vec::with_capacity((44 + data_bytes) as usize);
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVEfmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav_bytes.extend_from_slice(&2u16.to_le_bytes()); // stereo
        wav_bytes.extend_from_slice(&sample_rate.to_le_bytes());
        wav_bytes.extend_from_slice(&(sample_rate * 4).to_le_bytes());
        wav_bytes.extend_from_slice(&4u16.to_le_bytes());
        wav_bytes.extend_from_slice(&16u16.to_le_bytes());
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&data_bytes.to_le_bytes());
        for f in 0..num_frames {
            let t = f as f64 / sample_rate as f64;
            let sample = ((0.35 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) * 32767.0) as i16;
            wav_bytes.extend_from_slice(&sample.to_le_bytes());
            wav_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let wav_sha = format!("{:x}", Sha256::digest(&wav_bytes));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let wav_bytes_arc = Arc::new(wav_bytes.clone());
        let audio_url_for_resolver = format!("http://127.0.0.1:{}/audio/test.wav", addr.port());
        let sha_for_resolver = wav_sha.clone();

        let mock_router = Router::new()
            .route(
                "/resolve",
                axum::routing::post(move || {
                    let url = audio_url_for_resolver.clone();
                    let sha = sha_for_resolver.clone();
                    async move {
                        Json(serde_json::json!({
                            "direct_https_audio_url": url,
                            "license_or_authorization_note": "Automated test fixture license",
                            "expected_sha256": sha
                        }))
                    }
                }),
            )
            .route(
                "/audio/test.wav",
                get(move || {
                    let bytes = wav_bytes_arc.clone();
                    async move {
                        (
                            [(axum::http::header::CONTENT_TYPE, "audio/wav")],
                            (*bytes).clone(),
                        )
                    }
                }),
            );
        tokio::spawn(async move {
            let _ = axum::serve(listener, mock_router).await;
        });

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_async_auth_{}", uuid::Uuid::new_v4()));
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let resolver_cfg = AuthorizedResolverConfig::new_with_loopback(
            format!("http://127.0.0.1:{}/resolve", addr.port()),
            None,
            true,
        )
        .unwrap();
        let state = Arc::new(
            LocalHelperState::new(
                db.clone(),
                47821,
                temp_cache.clone(),
                sync_mgr,
                Some(Some("192.168.1.50".to_string())),
                vec![],
            )
            .with_audio_resolver(Some(resolver_cfg)),
        );
        let mut ws_rx = state.ws_tx.subscribe();
        let host_addr: SocketAddr = "127.0.0.1:50000".parse().unwrap();

        let Json(created) = create_local_room(
            State(state.clone()),
            ConnectInfo(host_addr),
            HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Async Auth Regression".to_string(),
                dj_level: Some(3),
                primary_mood_id: Some("house_party".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let mut host_headers = HeaderMap::new();
        host_headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", created.host_token)).unwrap(),
        );
        let _ = start_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .unwrap();

        // 2. Rapid submission + duplicate URL submission must deduplicate cleanly
        let Json(sub1) = submit_song_to_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(sub1.status, "received_by_dj");

        let Json(dup1) = submit_song_to_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://youtu.be/dQw4w9WgXcQ".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(dup1.status, "already_queued", "Duplicate video ID in same room must deduplicate");
        assert_eq!(dup1.id, sub1.id, "Duplicate URL must return the existing persistent submission ID");
        assert_eq!(db.get_pending_intake_job_count(&created.room.room_id).unwrap(), 1);

        // 3. Process the queued job -> verify `cache_id` IS persisted in SQLite & `next_playable_candidate` populated!
        let job1 = db.claim_next_submission_intake_job().unwrap().expect("Job 1 must be claimable");
        assert!(process_single_intake_job(&state, &job1).await);

        let Json(dash) = get_host_room_dashboard(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .unwrap();
        let next_playable = dash
            .next_playable_candidate
            .expect("Async-processed authorized track MUST persist cache_id and become next_playable_candidate");
        assert_eq!(next_playable.submission_id, sub1.id);
        assert!(!next_playable.cache_id.is_empty());
        assert!(next_playable.bpm.is_some());
        assert!(
            !next_playable.sections.is_empty(),
            "Async-analyzed playable track MUST expose real DSP phrase/section markers from global_song_catalogue"
        );

        // Verify WS `dj.submission_ready` reflects the exact DB `cache_id`
        let mut saw_matching_ws = false;
        while let Ok(evt) = ws_rx.try_recv() {
            if evt.event_type == "dj.submission_ready"
                && evt.payload.get("submission_id").and_then(|v| v.as_str()) == Some(sub1.id.as_str())
            {
                assert_eq!(
                    evt.payload.get("cache_id").and_then(|v| v.as_str()),
                    Some(next_playable.cache_id.as_str()),
                    "WS submission_ready cache_id must match persisted DB cache_id"
                );
                assert_eq!(
                    evt.payload.get("acquisition_status").and_then(|v| v.as_str()),
                    Some("cached_authorized")
                );
                saw_matching_ws = true;
            }
        }
        assert!(saw_matching_ws, "Expected matching WS dj.submission_ready event");

        // 4. Simulate non-retryable failure (empty license note) -> fails cleanly -> re-submitting retries SAME submission_id
        let Json(sub_fail) = submit_song_to_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=9bZkp7q19f0".to_string(),
                authorized_stream: Some(AuthorizedStreamProof {
                    direct_https_audio_url: "https://example.com/a.wav".to_string(),
                    license_or_authorization_note: "".to_string(),
                    expected_sha256: None,
                }),
            }),
        )
        .await
        .unwrap();

        let job_fail = db.claim_next_submission_intake_job().unwrap().expect("Fail job claimable");
        assert!(process_single_intake_job(&state, &job_fail).await);
        let subs_after_fail = db.get_local_room_submissions(&created.room.room_id).unwrap();
        let failed_row = subs_after_fail.iter().find(|s| s.id == sub_fail.id).unwrap();
        assert_eq!(failed_row.acquisition_status, "failed");

        // Re-submit same URL -> must retry on SAME submission_id without creating a duplicate row!
        let Json(retried) = submit_song_to_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            ConnectInfo(host_addr),
            host_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=9bZkp7q19f0".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(retried.status, "retry_queued");
        assert_eq!(retried.id, sub_fail.id, "Retry must reuse existing submission_id");
        assert_eq!(
            db.get_local_room_submissions(&created.room.room_id).unwrap().len(),
            2,
            "No duplicate submission rows should exist after dedup + retry"
        );

        // 5. Stale in-flight job recovery test
        let stale_job = db.claim_next_submission_intake_job().unwrap().unwrap();
        {
            let conn = db.conn();
            let guard = conn.lock().unwrap();
            let old_ts = chrono::Utc::now().timestamp_millis() - 300_000;
            guard
                .execute(
                    "UPDATE submission_intake_jobs SET updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![old_ts, stale_job.id],
                )
                .unwrap();
        }
        let recovered = db.recover_stale_intake_jobs(60_000).unwrap();
        assert_eq!(recovered, 1, "Stale in-flight job must be recovered automatically");

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    #[tokio::test]
    async fn test_loopback_resolver_cross_port_and_direct_proof_download() {
        use axum::extract::{Path, State};
        use axum::routing::get;
        use axum::{Json, Router};
        use dhun_lib::db::Database;
        use dhun_lib::library::downloader::AudioDownloader;
        use dhun_lib::library::pipeline::{stage_1_validate_and_identify, AuthorizedStreamProof};
        use dhun_lib::library::resolver::{download_authorized_proof, AuthorizedResolverConfig};
        use dhun_lib::network::local_helper_routes::{
            create_local_room, process_single_intake_job, submit_song_to_local_room,
            CreateLocalRoomRequest, LocalHelperState, UnifiedSongSubmissionRequest,
        };
        use dhun_lib::sync::MetadataSyncManager;
        use sha2::{Digest, Sha256};
        use std::net::SocketAddr;
        use std::sync::Arc;

        // 1. Verify loopback opt-in in AuthorizedResolverConfig::new_with_loopback
        let auto_loop = AuthorizedResolverConfig::new_with_loopback(
            "http://127.0.0.1:8000/resolve".to_string(),
            None,
            true,
        )
        .expect("Should accept 127.0.0.1 loopback when allowed");
        assert!(auto_loop.allow_loopback);

        let auto_local = AuthorizedResolverConfig::new_with_loopback(
            "http://localhost:8080/resolve".to_string(),
            None,
            true,
        )
        .expect("Should accept localhost loopback when allowed");
        assert!(auto_local.allow_loopback);

        // Remote insecure HTTP must still be rejected
        assert!(AuthorizedResolverConfig::new_with_loopback(
            "http://example.com/resolve".to_string(),
            None,
            false,
        )
        .is_err());

        // 2. Build a valid synthetic 2-second WAV in memory
        let sample_rate: u32 = 44_100;
        let num_frames: u32 = sample_rate * 2;
        let data_bytes: u32 = num_frames * 2 * 2;
        let mut wav_bytes = Vec::with_capacity((44 + data_bytes) as usize);
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVEfmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1u16.to_le_bytes());
        wav_bytes.extend_from_slice(&2u16.to_le_bytes());
        wav_bytes.extend_from_slice(&sample_rate.to_le_bytes());
        wav_bytes.extend_from_slice(&(sample_rate * 4).to_le_bytes());
        wav_bytes.extend_from_slice(&4u16.to_le_bytes());
        wav_bytes.extend_from_slice(&16u16.to_le_bytes());
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&data_bytes.to_le_bytes());
        for f in 0..num_frames {
            let t = f as f64 / sample_rate as f64;
            let sample = ((0.30 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) * 32767.0) as i16;
            wav_bytes.extend_from_slice(&sample.to_le_bytes());
            wav_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let wav_sha = format!("{:x}", Sha256::digest(&wav_bytes));

        // 3. Bind TWO separate loopback listeners (Cross-Port: Resolver on Port A, Audio Server on Port B)
        let audio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let audio_port = audio_listener.local_addr().unwrap().port();
        let wav_bytes_arc = Arc::new(wav_bytes.clone());

        let audio_router = Router::new().route(
            "/cache/test_crossport.wav",
            get(move || {
                let bytes = wav_bytes_arc.clone();
                async move {
                    (
                        [(axum::http::header::CONTENT_TYPE, "audio/wav")],
                        (*bytes).clone(),
                    )
                }
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(audio_listener, audio_router).await;
        });

        let audio_url = format!("http://127.0.0.1:{}/cache/test_crossport.wav", audio_port);
        let audio_url_clone = audio_url.clone();
        let sha_clone = wav_sha.clone();

        let resolver_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let resolver_port = resolver_listener.local_addr().unwrap().port();
        assert_ne!(resolver_port, audio_port, "Ports must be different to test cross-port loopback");

        let resolver_router = Router::new().route(
            "/resolve",
            axum::routing::post(move || {
                let u = audio_url_clone.clone();
                let s = sha_clone.clone();
                async move {
                    Json(serde_json::json!({
                        "direct_https_audio_url": u,
                        "license_or_authorization_note": "Cross-port local resolver test license",
                        "expected_sha256": s
                    }))
                }
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(resolver_listener, resolver_router).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let db = Database::new_in_memory().unwrap();
        let temp_cache = std::env::temp_dir().join(format!("dhun_cross_port_{}", uuid::Uuid::new_v4()));
        let sync_mgr = MetadataSyncManager::new(None, None).unwrap();
        let resolver_cfg = AuthorizedResolverConfig::new_with_loopback(
            format!("http://127.0.0.1:{}/resolve", resolver_port),
            None,
            true,
        )
        .unwrap();

        let state = Arc::new(
            LocalHelperState::new(
                db.clone(),
                47822,
                temp_cache.clone(),
                sync_mgr,
                Some(Some("192.168.1.50".to_string())),
                vec![],
            )
            .with_audio_resolver(Some(resolver_cfg)),
        );

        let host_addr: SocketAddr = "127.0.0.1:50001".parse().unwrap();
        let Json(created) = create_local_room(
            State(state.clone()),
            axum::extract::ConnectInfo(host_addr),
            axum::http::HeaderMap::new(),
            Json(CreateLocalRoomRequest {
                party_name: "Cross Port Loopback Room".to_string(),
                dj_level: Some(2),
                primary_mood_id: Some("house_party".to_string()),
                secondary_mood_id: None,
            }),
        )
        .await
        .unwrap();

        let mut host_headers = axum::http::HeaderMap::new();
        host_headers.insert("X-Host-Token", created.host_token.parse().unwrap());

        let _ = dhun_lib::network::local_helper_routes::start_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            axum::extract::ConnectInfo(host_addr),
            host_headers.clone(),
        )
        .await
        .unwrap();

        // Submit YouTube URL -> worker processes via cross-port resolver -> must SUCCEED with cached_authorized!
        let Json(sub) = submit_song_to_local_room(
            State(state.clone()),
            Path(created.room.room_id.clone()),
            axum::extract::ConnectInfo(host_addr),
            host_headers.clone(),
            Json(UnifiedSongSubmissionRequest {
                url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                authorized_stream: None,
            }),
        )
        .await
        .unwrap();

        let job = db.claim_next_submission_intake_job().unwrap().expect("Job should be queued");
        let processed = process_single_intake_job(&state, &job).await;
        assert!(processed, "Intake job should process successfully across ports");

        let subs = db.get_local_room_submissions(&created.room.room_id).unwrap();
        let sub_row = subs.iter().find(|s| s.id == sub.id).unwrap();
        println!("TEST DEBUG SUB ROW: status={}, note={:?}", sub_row.acquisition_status, sub_row.acquisition_note);
        assert_eq!(
            sub_row.acquisition_status, "cached_authorized",
            "Cross-port loopback audio MUST be acquired and cached as cached_authorized (not failed!)"
        );
        assert!(sub_row.local_cache_path.is_some());
        assert!(sub_row.cache_id.is_some());

        // 4. Verify direct download_authorized_proof with loopback URL
        let parsed = stage_1_validate_and_identify("https://www.youtube.com/watch?v=oHg5SJYRHA0").unwrap();
        let proof = AuthorizedStreamProof {
            direct_https_audio_url: audio_url.clone(),
            license_or_authorization_note: "Direct proof loopback license".to_string(),
            expected_sha256: Some(wav_sha.clone()),
        };
        let downloader = AudioDownloader::new(temp_cache.clone());
        let (proof_status, note, path) = download_authorized_proof(&parsed, &proof, &downloader, &db).await;
        assert_eq!(
            proof_status,
            dhun_lib::library::pipeline::AcquisitionStatus::CachedAuthorized,
            "Direct proof with loopback URL must acquire successfully"
        );
        assert!(path.is_some());
        assert!(note.contains("cached locally"));

        let _ = std::fs::remove_dir_all(&temp_cache);
    }

    /// Comprehensive recommendation engine edge-case validation:
    /// small pool, missing BPM/key/beat grid, conflicting energy, incompatible harmonic key,
    /// cross-genre, half/double-time BPM on Level 2, and mixed playable/processing/metadata-only tracks.
    #[test]
    fn test_recommendations_edge_cases_small_pool_missing_meta_clashing_key_energy_and_half_double_bpm() {
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::dj::moods::get_mood_by_id;
        use dhun_lib::dj::selector::{build_dj_recommendations, choose_technique_for_pair};

        let mood = get_mood_by_id("house_party").unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        let make = |id: &str,
                    dj_status: &str,
                    acq: &str,
                    bpm: Option<f64>,
                    key: Option<&str>,
                    energy: Option<f64>,
                    genre: &str,
                    meta_src: &str| LocalRoomSubmission {
            id: id.to_string(),
            room_id: "rm_rec".to_string(),
            submitted_by_type: "host".to_string(),
            submitted_by_id: "host".to_string(),
            submitted_by_name: "Host".to_string(),
            provider: "youtube".to_string(),
            source_url: format!("https://www.youtube.com/watch?v={:0<11}", id),
            provider_video_id: format!("{:0<11}", id),
            song_id: Some(format!("gsng_{}", id)),
            title: format!("Track {}", id),
            artist: format!("Artist {}", id),
            version_tag: "original".to_string(),
            thumbnail_url: None,
            duration_ms: Some(190_000),
            bpm,
            half_time_bpm: bpm.map(|b| b / 2.0),
            double_time_bpm: bpm.map(|b| b * 2.0),
            musical_key: key.map(|k| k.to_string()),
            camelot_key: key.map(|k| k.to_string()),
            overall_energy: energy,
            danceability: Some(0.8),
            genre_tags_json: format!(r#"["{}"]"#, genre),
            mood_tags_json: "[]".to_string(),
            metadata_source: meta_src.to_string(),
            acquisition_status: acq.to_string(),
            acquisition_note: None,
            dj_status: dj_status.to_string(),
            dj_score: 0.0,
            local_cache_path: if acq == "cached_authorized" { Some("/tmp/x.wav".to_string()) } else { None },
            cache_id: if acq == "cached_authorized" { Some(format!("c_{}", id)) } else { None },
            created_at: now,
            updated_at: now,
        };

        // 1. Very small pool: 1 metadata-only track with missing BPM, key, energy
        let solo = make("solo", "queued_for_dj", "restricted_by_provider", None, None, None, "Pop", "oembed");
        let solo_recs = build_dj_recommendations(&[solo], &mood, None, mood.target_energy, DjLevel::Level2SmartMixing);
        assert_eq!(solo_recs.len(), 1);
        assert!(!solo_recs[0].is_playable);
        assert_eq!(solo_recs[0].playability_label, "metadata_only_restricted");
        assert!(solo_recs[0].missing_signals.iter().any(|s| s.contains("Beat grid")));

        // 2. Clashing key (8A -> 2B), conflicting energy drop (0.88 -> 0.35), cross-genre, + processing + playable
        let playing = make("cur", "playing", "cached_authorized", Some(128.0), Some("8A"), Some(0.88), "House", "local_analysis");
        let clash_playable = make("clash", "queued_for_dj", "cached_authorized", Some(64.0), Some("2B"), Some(0.35), "Classical", "local_analysis");
        let processing = make("proc", "queued_for_dj", "metadata_processing", None, None, None, "EDM", "pending");
        let meta_only = make("meta", "queued_for_dj", "restricted_by_provider", Some(128.0), Some("8A"), Some(0.90), "House", "global_catalogue");

        let recs = build_dj_recommendations(
            &[playing.clone(), clash_playable.clone(), processing, meta_only],
            &mood,
            None,
            0.85,
            DjLevel::Level2SmartMixing,
        );

        assert_eq!(recs.len(), 3);
        // Playable track MUST come before non-playable tracks, and unavailable tracks NEVER labeled playable!
        assert!(recs[0].is_playable, "First recommendation must be the genuinely playable track");
        assert_eq!(recs[0].submission_id, "clash");
        assert!(recs[0].reasons.iter().any(|r| r.contains("Incompatible harmonic key")));
        assert!(recs[0].reasons.iter().any(|r| r.contains("Conflicting energy drop")));
        assert!(recs[0].reasons.iter().any(|r| r.contains("Cross-genre shift")));

        let proc_rec = recs.iter().find(|r| r.submission_id == "proc").unwrap();
        assert!(!proc_rec.is_playable);
        assert_eq!(proc_rec.playability_label, "processing");

        let meta_rec = recs.iter().find(|r| r.submission_id == "meta").unwrap();
        assert!(!meta_rec.is_playable);
        assert_eq!(meta_rec.playability_label, "metadata_only_restricted");

        // 3. Half/double-time BPM check on Level 2 (128 BPM -> 64 BPM harmonic match => timed_bass_swap!)
        let half_time_harmonic = make("ht", "queued_for_dj", "cached_authorized", Some(64.0), Some("8A"), Some(0.85), "House", "local_analysis");
        let (tech_l2, _) = choose_technique_for_pair(
            DjLevel::Level2SmartMixing,
            Some(&playing),
            &half_time_harmonic,
            &mood,
            0.95,
        );
        assert_eq!(
            tech_l2, "timed_bass_swap",
            "Level 2 must recognize 64 BPM as half-time match for 128 BPM"
        );
    }

    /// Real PCM audio-buffer DSP test: verifies variable tempo map (accelerating BPM),
    /// non-4/4 meter detection (3/4 rhythmic grouping), and vocal-band activity separation.
    #[test]
    fn test_tempo_map_variable_bpm_meter_3_4_and_vocal_activity_dsp() {
        use dhun_lib::analysis::bpm_detector::BpmDetector;
        use dhun_lib::analysis::phrase_detector::PhraseDetector;
        use std::sync::Arc;

        let sr = 44_100u32;

        // 1. Variable tempo track: first 16s at 100 BPM clicks, next 16s at 140 BPM clicks
        let var_sec = 32usize;
        let mut var_samples = vec![0.0f32; sr as usize * var_sec * 2];
        let add_clicks = |buf: &mut [f32], start_s: f64, end_s: f64, bpm: f64, accent_every: usize| {
            let step = 60.0 / bpm;
            let mut t = start_s;
            let mut idx = 0usize;
            while t < end_s {
                let start_f = (t * sr as f64) as usize;
                let click_len = (sr as usize * 45) / 1000;
                let amp = if idx % accent_every == 0 { 0.85 } else { 0.35 };
                for k in 0..click_len {
                    let f = start_f + k;
                    if f * 2 + 1 >= buf.len() {
                        break;
                    }
                    let env = 1.0 - (k as f64 / click_len as f64);
                    let v = (amp * env * (2.0 * std::f64::consts::PI * 120.0 * (k as f64 / sr as f64)).sin()) as f32;
                    buf[f * 2] = (buf[f * 2] + v).clamp(-1.0, 1.0);
                    buf[f * 2 + 1] = (buf[f * 2 + 1] + v).clamp(-1.0, 1.0);
                }
                t += step;
                idx += 1;
            }
        };
        add_clicks(&mut var_samples, 0.0, 16.0, 100.0, 4);
        add_clicks(&mut var_samples, 16.0, 32.0, 140.0, 4);

        let (_avg, _conf, tempo_map, drift_bpm, is_variable) =
            BpmDetector::detect_tempo_map(&Arc::new(var_samples), sr, (var_sec * 1000) as u64);
        assert!(tempo_map.len() >= 2, "Expected multi-segment tempo map");
        assert!(
            is_variable && drift_bpm >= 10.0,
            "Accelerating 100->140 BPM buffer must be flagged as variable tempo (drift={:.1}, map={:?})",
            drift_bpm,
            tempo_map
        );

        // 2. 3/4 Meter buffer: strong accent every 3 beats at 120 BPM for 24s
        let mut waltz_samples = vec![0.0f32; sr as usize * 24 * 2];
        add_clicks(&mut waltz_samples, 0.0, 24.0, 120.0, 3);
        let meter_3_4 = BpmDetector::detect_meter(&Arc::new(waltz_samples), sr, 120.0, 0);
        assert_eq!(
            meter_3_4.meter_label, "3/4",
            "Strong 3-beat accent pattern must detect 3/4 meter (got {:?})",
            meter_3_4
        );
        assert_eq!(meter_3_4.beats_per_bar, 3);

        // 3. Vocal activity separation: Phrase 0 = sub-bass instrumental (75 Hz), Phrase 1 = center vocal-band (900 Hz)
        let mut voc_samples = Vec::with_capacity(sr as usize * 32 * 2);
        for f in 0..(sr as usize * 32) {
            let t = f as f64 / sr as f64;
            let s = if f < sr as usize * 16 {
                (0.42 * (2.0 * std::f64::consts::PI * 75.0 * t).sin()) as f32
            } else {
                (0.42 * (2.0 * std::f64::consts::PI * 900.0 * t).sin()) as f32
            };
            voc_samples.push(s);
            voc_samples.push(s);
        }
        let (phrases, _, _, _, _) =
            PhraseDetector::analyze_phrasing_with_timing(&Arc::new(voc_samples), sr, 120.0, 0, 4, 32_000);
        assert!(phrases.len() >= 2);
        let inst_voc = phrases[0].vocal_activity;
        let vocal_voc = phrases.last().unwrap().vocal_activity;
        assert!(
            vocal_voc > inst_voc + 0.35,
            "Center 900 Hz vocal-band phrase ({:.2}) must score far higher vocal_activity than 75 Hz bass instrumental ({:.2})",
            vocal_voc,
            inst_voc
        );
    }

    /// Comprehensive deterministic test of the DJ Transition Decision Engine covering:
    /// same-BPM, half-time, double-time, gradual tempo ramp, 3:4 bridge, low-confidence beatgrid,
    /// meter conflict (3/4 vs 4/4), vocal clash avoidance, build-up protection, intro-over-chorus,
    /// breakdown-to-drop, genre change, Indian moods (Baraat, Garba, Shaadi, Desi Club), and
    /// insufficient remaining playback time rejection.
    #[test]
    fn test_dj_transition_decision_engine_all_bpm_relationships_vocal_clash_meters_and_indian_moods() {
        use dhun_lib::dj::decision_engine::{
            plan_dj_transition, BpmRelationship, PartyDecisionContext, TempoQualityPolicy,
            TrackTransitionInput,
        };
        use dhun_lib::dj::levels::DjLevel;

        let make_timing = |bpm: f64, conf: f64, beats_per_bar: u8, meter_label: &str, var: bool| TrackTimingProfile {
            average_bpm: bpm,
            bpm_confidence: conf,
            beatgrid_confidence: conf,
            downbeat_offset_ms: 0,
            downbeat_confidence: conf,
            is_variable_tempo: var,
            tempo_drift_bpm: if var { 9.0 } else { 0.4 },
            meter: MeterAnalysis {
                beats_per_bar,
                meter_label: meter_label.to_string(),
                confidence: conf,
            },
            tempo_map: vec![TempoSegment {
                start_ms: 0,
                end_ms: 180_000,
                bpm,
                confidence: conf,
            }],
            beat_timestamps_ms: (0..360).map(|i| (i as f64 * (60_000.0 / bpm)) as i64).collect(),
            downbeat_timestamps_ms: (0..90)
                .map(|i| (i as f64 * beats_per_bar as f64 * (60_000.0 / bpm)) as i64)
                .collect(),
            ..Default::default()
        };

        let make_sec = |stype: &str, s_ms: i64, e_ms: i64, energy: f64, vocal: f64| PhraseMarker {
            section_type: stype.to_string(),
            start_ms: s_ms,
            end_ms: e_ms,
            energy_level: energy,
            bass_energy: energy * 0.85,
            vocal_activity: vocal,
            confidence: 0.88,
            bar_index: 0,
            ..Default::default()
        };

        let base_track = |id: &str, deck: usize, bpm: f64, key: &str, genre: &str, sections: Vec<PhraseMarker>| {
            TrackTransitionInput {
                track_id: id.to_string(),
                deck_id: deck,
                title: format!("Track {}", id),
                artist: format!("Artist {}", id),
                duration_sec: 160.0,
                current_pos_sec: 60.0,
                bpm: Some(bpm),
                camelot_key: Some(key.to_string()),
                overall_energy: Some(0.80),
                genre_tags: vec![genre.to_string()],
                sections,
                timing_profile: Some(make_timing(bpm, 0.90, 4, "4/4", false)),
                ..Default::default()
            }
        };

        let ctx = |level: DjLevel, mood: &str| PartyDecisionContext {
            dj_level: level,
            mood_id: mood.to_string(),
            target_energy: 0.82,
            recent_techniques: vec![],
            tempo_policy: TempoQualityPolicy::for_dj_level(level),
        };

        // 1. Same-BPM Desi Club instrumental tracks -> long 64-beat phrase/layer blend
        let club_out = base_track(
            "c1",
            0,
            128.0,
            "8A",
            "Tech House",
            vec![
                make_sec("groove", 0, 96_000, 0.82, 0.15),
                make_sec("outro", 96_000, 160_000, 0.65, 0.10),
            ],
        );
        let club_in = base_track(
            "c2",
            1,
            128.0,
            "8A",
            "Tech House",
            vec![
                make_sec("intro", 0, 48_000, 0.68, 0.10),
                make_sec("drop", 48_000, 128_000, 0.92, 0.15),
            ],
        );
        let p_club = plan_dj_transition(&club_out, &club_in, &ctx(DjLevel::Level3AdvancedPerformance, "desi_club"));
        assert_eq!(p_club.status, "planned");
        assert_eq!(p_club.bpm_relationship, BpmRelationship::Direct);
        assert_eq!(p_club.duration_beats, 64, "Instrumental Desi Club blend should select 64 beats");
        assert!(p_club.meter_compatible);

        // 2. Half-time (128 -> 64) & Double-time (70 -> 140) on Level 2
        let mut ht_in = club_in.clone();
        ht_in.bpm = Some(64.0);
        ht_in.timing_profile = Some(make_timing(64.0, 0.90, 4, "4/4", false));
        let p_ht = plan_dj_transition(&club_out, &ht_in, &ctx(DjLevel::Level2SmartMixing, "house_party"));
        assert_eq!(p_ht.bpm_relationship, BpmRelationship::HalfTime);

        let mut dt_out = club_out.clone();
        dt_out.bpm = Some(70.0);
        dt_out.timing_profile = Some(make_timing(70.0, 0.90, 4, "4/4", false));
        let mut dt_in = club_in.clone();
        dt_in.bpm = Some(140.0);
        dt_in.timing_profile = Some(make_timing(140.0, 0.90, 4, "4/4", false));
        let p_dt = plan_dj_transition(&dt_out, &dt_in, &ctx(DjLevel::Level2SmartMixing, "house_party"));
        assert_eq!(p_dt.bpm_relationship, BpmRelationship::DoubleTime);

        // 3. Gradual tempo ramp (112 -> 128 BPM = 14.3% diff) on Level 4 -> tempo_ramp_bridge
        let mut ramp_out = club_out.clone();
        ramp_out.bpm = Some(112.0);
        ramp_out.timing_profile = Some(make_timing(112.0, 0.90, 4, "4/4", false));
        let p_ramp = plan_dj_transition(&ramp_out, &club_in, &ctx(DjLevel::Level4CreativeTools, "house_party"));
        assert_eq!(p_ramp.bpm_relationship, BpmRelationship::GradualTempoRamp);
        assert_eq!(p_ramp.selected_technique, "tempo_ramp_bridge");

        // 4. Low-confidence beatgrid -> refuses forced beatmatch (`NoBeatmatch`) + explains rejected alternatives
        let mut bad_grid = club_in.clone();
        bad_grid.timing_profile = Some(make_timing(128.0, 0.30, 4, "4/4", false));
        let p_low = plan_dj_transition(&club_out, &bad_grid, &ctx(DjLevel::Level2SmartMixing, "house_party"));
        assert_eq!(p_low.bpm_relationship, BpmRelationship::NoBeatmatch);
        assert_eq!(p_low.selected_technique, "safe_quick_cut");
        assert!(!p_low.rejected_alternatives.is_empty());

        // 5. Incompatible meter (3/4 Garba/Folk vs 4/4) -> refuses forced beatmatch!
        let mut meter3 = club_in.clone();
        meter3.timing_profile = Some(make_timing(128.0, 0.90, 3, "3/4", false));
        let p_meter = plan_dj_transition(&club_out, &meter3, &ctx(DjLevel::Level3AdvancedPerformance, "garba"));
        assert!(!p_meter.meter_compatible);
        assert_eq!(p_meter.bpm_relationship, BpmRelationship::NoBeatmatch);

        // 6. Vocal-vs-vocal clash in Bollywood/Shaadi -> avoids long blend & sets `avoid_vocal_clash`
        let voc_out = base_track(
            "v1",
            0,
            122.0,
            "9B",
            "Bollywood",
            vec![make_sec("chorus", 40_000, 120_000, 0.85, 0.82)],
        );
        let voc_in = base_track(
            "v2",
            1,
            122.0,
            "9B",
            "Bollywood",
            vec![make_sec("chorus", 0, 80_000, 0.86, 0.85)],
        );
        let p_voc = plan_dj_transition(&voc_out, &voc_in, &ctx(DjLevel::Level2SmartMixing, "shaadi"));
        assert_eq!(p_voc.vocal_overlap_policy, "avoid_vocal_clash");
        assert!(p_voc.duration_beats <= 16, "Vocal clash must shorten transition (got {} beats)", p_voc.duration_beats);
        assert!(p_voc.rejected_alternatives.iter().any(|r| r.technique == "phrase_long_blend"));

        // 7. Intro-over-chorus & Breakdown-to-drop preferred pairing + Build-up payoff protection!
        let mut build_out = base_track(
            "b1",
            0,
            126.0,
            "8A",
            "EDM",
            vec![
                make_sec("build", 48_000, 64_000, 0.70, 0.15),
                make_sec("drop", 64_000, 96_000, 0.95, 0.15),
                make_sec("breakdown", 96_000, 120_000, 0.40, 0.10),
            ],
        );
        build_out.current_pos_sec = 52.0; // Currently inside build-up right before drop!
        let drop_in = base_track(
            "d2",
            1,
            126.0,
            "8A",
            "EDM",
            vec![
                make_sec("intro", 0, 16_000, 0.40, 0.10),
                make_sec("drop", 16_000, 80_000, 0.94, 0.15),
            ],
        );
        let p_bd = plan_dj_transition(&build_out, &drop_in, &ctx(DjLevel::Level3AdvancedPerformance, "desi_club"));
        assert!(
            p_bd.outgoing_mix_out_sec >= 90.0,
            "Must protect active build-up & drop payoff before mixing out at breakdown (got {:.1}s)",
            p_bd.outgoing_mix_out_sec
        );
        assert_eq!(p_bd.outgoing_section_type, "breakdown");
        assert_eq!(p_bd.incoming_section_type, "drop", "Breakdown outgoing should welcome high-impact incoming drop");

        // 8. Baraat Dhol Indian percussion mood -> prefers short clean mix, rejects long blend
        let dhol_out = base_track("dh1", 0, 134.0, "6A", "Bhangra Dhol", vec![make_sec("groove", 0, 140_000, 0.95, 0.45)]);
        let dhol_in = base_track("dh2", 1, 134.0, "6A", "Bhangra Dhol", vec![make_sec("intro", 0, 40_000, 0.90, 0.25)]);
        let p_dhol = plan_dj_transition(&dhol_out, &dhol_in, &ctx(DjLevel::Level2SmartMixing, "baraat"));
        assert!(p_dhol.duration_beats <= 16);
        assert!(p_dhol.rejected_alternatives.iter().any(|r| r.technique == "phrase_long_blend"));

        // 9. Insufficient remaining playback time (< 1.2s) -> explicitly REJECTED
        let mut urgent = club_out.clone();
        urgent.current_pos_sec = 159.3;
        let p_rej = plan_dj_transition(&urgent, &club_in, &ctx(DjLevel::Level2SmartMixing, "house_party"));
        assert_eq!(p_rej.status, "rejected");
        assert!(p_rej.rejection_reason.is_some());
    }

    #[test]
    fn test_post_11c1475_timing_and_confidence_audit() {
        use dhun_lib::dj::decision_engine::{
            plan_dj_transition, PartyDecisionContext, TempoQualityPolicy,
            BpmRelationship, TrackTransitionInput,
        };
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::db::models::{TrackTimingProfile, MeterAnalysis, PhraseMarker};

        // 1. Explicit timing calculations at 120.0 BPM:
        let bpm = 120.0;
        let beat_sec = 60.0 / bpm; // 0.50s per beat

        // Four beats at 120 BPM = 2.0s
        let four_beats_sec = 4.0 * beat_sec;
        assert_eq!(four_beats_sec, 2.0, "Four beats at 120 BPM must equal 2.0s");

        // One 4/4 bar at 120 BPM = 2.0s
        let beats_per_bar_4_4 = 4.0;
        let bar_4_4_sec = beats_per_bar_4_4 * beat_sec;
        assert_eq!(bar_4_4_sec, 2.0, "One 4/4 bar at 120 BPM must equal 2.0s");

        // One 3/4 bar at 120 BPM = 1.5s
        let beats_per_bar_3_4 = 3.0;
        let bar_3_4_sec = beats_per_bar_3_4 * beat_sec;
        assert_eq!(bar_3_4_sec, 1.5, "One 3/4 bar at 120 BPM must equal 1.5s");

        // One 6/8 bar using correctly inferred beat/subdivision convention:
        // Compound duple: 2 compound dotted-quarter beats at 120 compound BPM = 1.0s
        // Or 6 eighth-note subdivisions where quarter note = 0.5s -> eighth note = 0.25s:
        // 6 * 0.25s = 1.50s
        let eighth_note_sec = beat_sec / 2.0;
        let bar_6_8_subdivisions = 6.0;
        let bar_6_8_sec = bar_6_8_subdivisions * eighth_note_sec;
        assert_eq!(bar_6_8_sec, 1.5, "One 6/8 bar with 6 eighth-note subdivisions must equal 1.5s");

        let compound_beats = 2.0;
        let bar_6_8_compound_sec = compound_beats * beat_sec;
        assert_eq!(bar_6_8_compound_sec, 1.0, "One 6/8 bar with 2 compound beats must equal 1.0s");

        // 2. Strict separation: musical_duration_sec vs audio_crossfade_sec
        let t_out = TrackTransitionInput {
            track_id: "t_out".to_string(),
            deck_id: 0,
            title: "Out Track".to_string(),
            artist: "Artist".to_string(),
            duration_sec: 180.0,
            current_pos_sec: 0.0,
            bpm: Some(120.0),
            camelot_key: Some("8A".to_string()),
            overall_energy: Some(0.80),
            genre_tags: vec!["Dance".to_string()],
            sections: vec![PhraseMarker {
                section_type: "groove".to_string(),
                start_ms: 0,
                end_ms: 120_000,
                energy_level: 0.80,
                bass_energy: 0.70,
                vocal_activity: 0.10,
                confidence: 0.85,
                bar_index: 0,
                ..Default::default()
            }],
            timing_profile: Some(TrackTimingProfile {
                average_bpm: 120.0,
                beatgrid_confidence: 0.30, // Unreliable beatgrid -> forces safe_quick_cut
                bpm_confidence: 0.90,
                downbeat_confidence: 0.30,
                downbeat_offset_ms: 0,
                is_variable_tempo: false,
                tempo_drift_bpm: 0.2,
                meter: MeterAnalysis {
                    beats_per_bar: 4,
                    meter_label: "4/4".to_string(),
                    confidence: 0.85,
                },
                tempo_map: vec![],
                beat_timestamps_ms: vec![],
                downbeat_timestamps_ms: vec![],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut t_in = t_out.clone();
        t_in.track_id = "t_in".to_string();
        t_in.deck_id = 1;

        let ctx = PartyDecisionContext {
            dj_level: DjLevel::Level1TwoDeckMixer,
            mood_id: "house_party".to_string(),
            target_energy: 0.75,
            recent_techniques: vec![],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level1TwoDeckMixer),
        };

        let plan = plan_dj_transition(&t_out, &t_in, &ctx);
        assert_eq!(plan.selected_technique, "safe_quick_cut");
        assert_eq!(plan.duration_beats, 4);
        assert_eq!(plan.musical_duration_sec, 2.0); // 4 * 0.5s = 2.0s musical duration
        assert!(plan.audio_crossfade_sec <= 0.50);  // 0.35s fast crossfade curve
        assert!(plan.audio_crossfade_sec < plan.musical_duration_sec, "Audio crossfade must be separated from musical length");

        // 3. Confidence consistency and threshold comparison
        // BPM match compares bpm_confidence (0.90 >= 0.50)
        // Beat-lock compares beatgrid_confidence (0.30 < 0.50 -> NoBeatmatch)
        assert_eq!(plan.bpm_relationship, BpmRelationship::NoBeatmatch);
    }

    #[test]
    fn test_dj_musical_intelligence_timbre_vocal_and_halftime_decoupling() {
        use dhun_lib::dj::selector::evaluate_submission_dj_intelligence;
        use dhun_lib::dj::moods::get_mood_by_id;
        use dhun_lib::db::models::LocalRoomSubmission;

        let now = chrono::Utc::now().timestamp_millis();
        let make_sub = |id: &str, status: &str, bpm: f64, camelot: &str, energy: f64, genres: &str, acq: &str| LocalRoomSubmission {
            id: id.to_string(),
            room_id: "rm_music_intel".to_string(),
            submitted_by_type: "host".to_string(),
            submitted_by_id: "h1".to_string(),
            submitted_by_name: "DJ Host".to_string(),
            provider: "youtube".to_string(),
            source_url: format!("https://youtube.com/watch?v={}", id),
            provider_video_id: id.to_string(),
            song_id: None,
            title: id.to_string(),
            artist: "Test Artist".to_string(),
            version_tag: "original".to_string(),
            thumbnail_url: None,
            duration_ms: Some(210_000),
            bpm: Some(bpm),
            half_time_bpm: Some(bpm / 2.0),
            double_time_bpm: Some(bpm * 2.0),
            musical_key: Some("C Major".to_string()),
            camelot_key: Some(camelot.to_string()),
            overall_energy: Some(energy),
            danceability: Some(0.85),
            genre_tags_json: genres.to_string(),
            mood_tags_json: r#"["desi_club"]"#.to_string(),
            metadata_source: "local_analysis".to_string(),
            acquisition_status: acq.to_string(),
            acquisition_note: None,
            dj_status: status.to_string(),
            dj_score: 0.0,
            local_cache_path: Some(format!("/tmp/{}.wav", id)),
            cache_id: Some(format!("c_{}", id)),
            created_at: now,
            updated_at: now,
        };

        let desi_club = get_mood_by_id("desi_club").unwrap();

        // 1. Current playing: 128 BPM, 8A, High Energy Tech House Club
        let playing = make_sub("playing_club", "playing", 128.0, "8A", 0.82, r#"["Tech House", "Desi Club"]"#, "cached_authorized");

        // Candidate 1: Identical 128 BPM & 8A, but quiet Acoustic Ghazal/Ballad
        let acoustic_trap = make_sub("acoustic_trap", "queued_for_dj", 128.0, "8A", 0.28, r#"["Acoustic", "Ghazal", "Ballad"]"#, "cached_authorized");

        // Candidate 2: 126 BPM, 9A (+1 hour), High Energy Bhangra Groove
        let driving_bhangra = make_sub("driving_bhangra", "queued_for_dj", 126.0, "9A", 0.84, r#"["Bhangra", "Desi Club", "Instrumental"]"#, "cached_authorized");

        let eval_acoustic = evaluate_submission_dj_intelligence(&acoustic_trap, Some(&playing), &desi_club, None, 0.85, &[]);
        let eval_bhangra = evaluate_submission_dj_intelligence(&driving_bhangra, Some(&playing), &desi_club, None, 0.85, &[]);

        // Assert: Acoustic ballad is heavily penalized for timbre shock and energy collapse despite matching 128 BPM & 8A!
        assert!(eval_acoustic.total_score <= 55.0, "Acoustic ballad must be penalized for timbre shock (got score: {})", eval_acoustic.total_score);
        assert!(eval_bhangra.total_score >= 95.0, "Driving Bhangra groove must score near top (got score: {})", eval_bhangra.total_score);
        assert!(eval_bhangra.total_score > eval_acoustic.total_score + 35.0);

        // 2. Decoupled BPM: 138 BPM Dhol -> 69 BPM Trap Anthem with Camelot Lock
        let baraat = get_mood_by_id("baraat").unwrap();
        let playing_dhol = make_sub("playing_dhol", "playing", 138.0, "6A", 0.90, r#"["Bhangra", "Dhol"]"#, "cached_authorized");
        let halftime_trap = make_sub("halftime_trap", "queued_for_dj", 69.0, "6A", 0.91, r#"["Punjabi", "Trap"]"#, "cached_authorized");
        let flat_pop = make_sub("flat_pop", "queued_for_dj", 137.9, "1B", 0.52, r#"["Pop"]"#, "cached_authorized");

        let eval_trap = evaluate_submission_dj_intelligence(&halftime_trap, Some(&playing_dhol), &baraat, None, 0.92, &[]);
        let eval_pop = evaluate_submission_dj_intelligence(&flat_pop, Some(&playing_dhol), &baraat, None, 0.92, &[]);

        assert!(eval_trap.total_score > eval_pop.total_score + 10.0, "Harmonic half-time with key lock must beat flat pop with raw BPM closeness");
        assert!(eval_trap.reasons.iter().any(|r| r.contains("half-time")));
    }

    #[test]
    fn test_all_14_dj_moods_musical_identity_contrasts_and_set_progressions() {
        use dhun_lib::dj::moods::{get_mood_by_id, get_preset_moods};
        use dhun_lib::dj::selector::evaluate_submission_dj_intelligence;
        use dhun_lib::dj::decision_engine::{plan_dj_transition, TrackTransitionInput, PartyDecisionContext};
        use dhun_lib::db::models::LocalRoomSubmission;
        use dhun_lib::dj::levels::DjLevel;

        let all_moods = get_preset_moods();
        assert_eq!(all_moods.len(), 14);

        // 1. Verify rich profile completeness across all 14 moods
        for m in &all_moods {
            assert!(!m.musical_identity.is_empty(), "Mood {} must have non-empty musical_identity", m.id);
            assert!(!m.energy_curve.is_empty(), "Mood {} must have non-empty energy_curve", m.id);
            assert!(!m.preferred_percussion.is_empty(), "Mood {} must have non-empty preferred_percussion", m.id);
            assert!(!m.vocal_preference.is_empty(), "Mood {} must have non-empty vocal_preference", m.id);
            assert!(!m.transition_preferences.is_empty(), "Mood {} must have transition preferences", m.id);
            assert!(m.min_transition_beats >= 4, "Mood {} min_transition_beats must be >= 4", m.id);
            assert!(m.max_transition_beats >= m.min_transition_beats, "Mood {} max beats must be >= min beats", m.id);
            assert!(m.contrast_tolerance >= 0.10 && m.contrast_tolerance <= 1.0, "Mood {} contrast tolerance invalid", m.id);
        }

        let now = chrono::Utc::now().timestamp_millis();
        let make_cand = |id: &str, bpm: f64, camelot: &str, energy: f64, genres: &str| LocalRoomSubmission {
            id: id.to_string(),
            room_id: "rm_contrasts".to_string(),
            submitted_by_type: "host".to_string(),
            submitted_by_id: "h1".to_string(),
            submitted_by_name: "DJ Host".to_string(),
            provider: "youtube".to_string(),
            source_url: format!("https://youtube.com/watch?v={}", id),
            provider_video_id: id.to_string(),
            song_id: None,
            title: id.to_string(),
            artist: "Test Artist".to_string(),
            version_tag: "original".to_string(),
            thumbnail_url: None,
            duration_ms: Some(210_000),
            bpm: Some(bpm),
            half_time_bpm: Some(bpm / 2.0),
            double_time_bpm: Some(bpm * 2.0),
            musical_key: Some("C Major".to_string()),
            camelot_key: Some(camelot.to_string()),
            overall_energy: Some(energy),
            danceability: Some(0.85),
            genre_tags_json: genres.to_string(),
            mood_tags_json: "[]".to_string(),
            metadata_source: "local_analysis".to_string(),
            acquisition_status: "cached_authorized".to_string(),
            acquisition_note: None,
            dj_status: "queued_for_dj".to_string(),
            dj_score: 0.0,
            local_cache_path: Some(format!("/tmp/{}.wav", id)),
            cache_id: Some(format!("c_{}", id)),
            created_at: now,
            updated_at: now,
        };

        // Contrast 1: Shaadi vs Cocktail Night
        {
            let shaadi = get_mood_by_id("shaadi").unwrap();
            let cocktail = get_mood_by_id("cocktail_night").unwrap();
            let wedding_anthem = make_cand("shaadi_anthem", 118.0, "9A", 0.82, r#"["Bollywood", "Wedding Dholak", "Singalong"]"#);
            let deep_house = make_cand("deep_house", 120.0, "9A", 0.58, r#"["Deep House", "Sunset House", "Melodic Lounge"]"#);
            let playing = make_cand("cur", 120.0, "9A", 0.70, r#"["Pop"]"#);

            let eval_shaadi_a = evaluate_submission_dj_intelligence(&wedding_anthem, Some(&playing), &shaadi, None, 0.76, &[]);
            let eval_shaadi_b = evaluate_submission_dj_intelligence(&deep_house, Some(&playing), &shaadi, None, 0.76, &[]);
            let eval_cocktail_a = evaluate_submission_dj_intelligence(&wedding_anthem, Some(&playing), &cocktail, None, 0.60, &[]);
            let eval_cocktail_b = evaluate_submission_dj_intelligence(&deep_house, Some(&playing), &cocktail, None, 0.60, &[]);

            assert!(eval_shaadi_a.total_score > eval_shaadi_b.total_score, "Shaadi must rank celebratory wedding singalong higher");
            assert!(eval_cocktail_b.total_score > eval_cocktail_a.total_score, "Cocktail Night must rank deep house higher");
            assert!(eval_cocktail_a.penalties.iter().any(|p| p.contains("Cocktail Night")), "Cocktail Night must penalize aggressive wedding tracks");
        }

        // Contrast 2: Baraat vs Retro Bollywood
        {
            let baraat = get_mood_by_id("baraat").unwrap();
            let retro = get_mood_by_id("retro_bollywood").unwrap();
            let live_dhol = make_cand("live_dhol", 138.0, "6A", 0.96, r#"["Bhangra Dhol", "Live Dhol", "Baraat Brass"]"#);
            let rd_burman = make_cand("rd_burman", 116.0, "6A", 0.62, r#"["Disco Bollywood", "70s Funk", "RD Burman Brass"]"#);
            let playing = make_cand("cur", 130.0, "6A", 0.80, r#"["Pop"]"#);

            let eval_baraat_a = evaluate_submission_dj_intelligence(&live_dhol, Some(&playing), &baraat, None, 0.96, &[]);
            let eval_baraat_b = evaluate_submission_dj_intelligence(&rd_burman, Some(&playing), &baraat, None, 0.96, &[]);
            let eval_retro_a = evaluate_submission_dj_intelligence(&live_dhol, Some(&playing), &retro, None, 0.62, &[]);
            let eval_retro_b = evaluate_submission_dj_intelligence(&rd_burman, Some(&playing), &retro, None, 0.62, &[]);

            assert!(eval_baraat_a.total_score > eval_baraat_b.total_score, "Baraat must rank live dhol #1");
            assert!(eval_retro_b.total_score > eval_retro_a.total_score, "Retro Bollywood must rank 70s funk #1");
            assert!(eval_baraat_b.penalties.iter().any(|p| p.contains("Baraat")), "Baraat must penalize subdued retro tracks");
        }

        // Contrast 3: Garba vs Dandiya
        {
            let garba = get_mood_by_id("garba").unwrap();
            let dandiya = get_mood_by_id("dandiya").unwrap();
            let garba_3taali = make_cand("garba_3taali", 114.0, "8A", 0.78, r#"["Garba Folk", "3-Taali Dhol", "Sanedo"]"#);
            let fast_dandiya = make_cand("fast_dandiya", 136.0, "8A", 0.90, r#"["Dandiya Raas", "Raas Beat", "Stick Rhythm"]"#);
            let playing = make_cand("cur", 115.0, "8A", 0.75, r#"["Folk"]"#);

            let eval_garba_a = evaluate_submission_dj_intelligence(&garba_3taali, Some(&playing), &garba, None, 0.84, &[]);
            let eval_garba_b = evaluate_submission_dj_intelligence(&fast_dandiya, Some(&playing), &garba, None, 0.84, &[]);
            let eval_dandiya_a = evaluate_submission_dj_intelligence(&garba_3taali, Some(&playing), &dandiya, None, 0.90, &[]);
            let eval_dandiya_b = evaluate_submission_dj_intelligence(&fast_dandiya, Some(&playing), &dandiya, None, 0.90, &[]);

            assert!(eval_garba_a.total_score > eval_garba_b.total_score, "Garba must rank 3-Taali cyclic folk #1");
            assert!(eval_dandiya_b.total_score > eval_dandiya_a.total_score, "Dandiya must rank fast stick raas #1");
            assert!(eval_dandiya_a.penalties.iter().any(|p| p.contains("Dandiya")), "Dandiya must penalize tempo < 120 BPM");
        }

        // Contrast 4: Punjabi Tashan vs House Party (Half-Time Groove)
        {
            let punjabi = get_mood_by_id("punjabi_tashan").unwrap();
            let house_party = get_mood_by_id("house_party").unwrap();
            let urban_trap = make_cand("urban_trap", 80.0, "10A", 0.92, r#"["Punjabi Trap", "808 Trap", "UK Drill Punjabi"]"#);
            let pop_dance = make_cand("pop_dance", 122.0, "10A", 0.75, r#"["Pop", "Chart Singalong", "Dance"]"#);
            let playing_bhangra = make_cand("cur", 140.0, "10A", 0.88, r#"["Bhangra"]"#);

            let eval_punjabi_a = evaluate_submission_dj_intelligence(&urban_trap, Some(&playing_bhangra), &punjabi, None, 0.92, &[]);
            let eval_punjabi_b = evaluate_submission_dj_intelligence(&pop_dance, Some(&playing_bhangra), &punjabi, None, 0.92, &[]);
            let eval_house_a = evaluate_submission_dj_intelligence(&urban_trap, Some(&playing_bhangra), &house_party, None, 0.72, &[]);
            let eval_house_b = evaluate_submission_dj_intelligence(&pop_dance, Some(&playing_bhangra), &house_party, None, 0.72, &[]);

            assert!(eval_punjabi_a.total_score > eval_punjabi_b.total_score, "Punjabi Tashan must rank 808 Trap #1 via half-time");
            assert!(eval_house_b.total_score > eval_house_a.total_score, "House Party must rank accessible pop dance #1");
        }

        // Contrast 5: Mehendi vs Festival Mode
        {
            let mehendi = get_mood_by_id("mehendi").unwrap();
            let festival = get_mood_by_id("festival_mode").unwrap();
            let sufi_ballad = make_cand("sufi_ballad", 96.0, "11B", 0.56, r#"["Folk Bollywood", "Folk Ballad", "Sufi"]"#);
            let stadium_edm = make_cand("stadium_edm", 130.0, "11B", 0.98, r#"["Festival Big Room", "Mainstage Big Room", "EDM Drop"]"#);
            let playing = make_cand("cur", 100.0, "11B", 0.60, r#"["Acoustic"]"#);

            let eval_mehendi_a = evaluate_submission_dj_intelligence(&sufi_ballad, Some(&playing), &mehendi, None, 0.58, &[]);
            let eval_mehendi_b = evaluate_submission_dj_intelligence(&stadium_edm, Some(&playing), &mehendi, None, 0.58, &[]);
            let eval_festival_a = evaluate_submission_dj_intelligence(&sufi_ballad, Some(&playing), &festival, None, 0.98, &[]);
            let eval_festival_b = evaluate_submission_dj_intelligence(&stadium_edm, Some(&playing), &festival, None, 0.98, &[]);

            assert!(eval_mehendi_a.total_score > eval_mehendi_b.total_score, "Mehendi must rank Sufi ballad #1");
            assert!(eval_festival_b.total_score > eval_festival_a.total_score, "Festival Mode must rank stadium EDM #1");
            assert!(eval_mehendi_b.penalties.iter().any(|p| p.contains("Mehendi")), "Mehendi must penalize heavy club energy");
            assert!(eval_festival_a.penalties.iter().any(|p| p.contains("Festival")), "Festival Mode must penalize soft ballad energy");
        }

        // Contrast 6: Desi Club vs International Party
        {
            let desi_club = get_mood_by_id("desi_club").unwrap();
            let intl = get_mood_by_id("international_party").unwrap();
            let club_remix = make_cand("club_remix", 128.0, "7A", 0.90, r#"["Desi Remix", "Bollywood 4x4", "Club EDM"]"#);
            let afrobeat = make_cand("afrobeat", 104.0, "7A", 0.80, r#"["Afrobeat", "Afrobeats Amapiano", "Latin Dembow"]"#);
            let playing = make_cand("cur", 126.0, "7A", 0.85, r#"["Dance"]"#);

            let eval_desi_a = evaluate_submission_dj_intelligence(&club_remix, Some(&playing), &desi_club, None, 0.90, &[]);
            let eval_desi_b = evaluate_submission_dj_intelligence(&afrobeat, Some(&playing), &desi_club, None, 0.90, &[]);
            let eval_intl_a = evaluate_submission_dj_intelligence(&club_remix, Some(&playing), &intl, None, 0.82, &[]);
            let eval_intl_b = evaluate_submission_dj_intelligence(&afrobeat, Some(&playing), &intl, None, 0.82, &[]);

            assert!(eval_desi_a.total_score > eval_desi_b.total_score, "Desi Club must rank 4x4 Bollywood club remix #1");
            assert!(eval_intl_b.total_score > eval_intl_a.total_score, "International Party must rank Afrobeat dembow #1");
        }

        // 3. Verify Transition Planning Differences Between Moods
        {
            let t_out = TrackTransitionInput {
                track_id: "t_out".to_string(),
                deck_id: 0,
                duration_sec: 240.0,
                current_pos_sec: 210.0,
                bpm: Some(136.0),
                camelot_key: Some("6A".to_string()),
                overall_energy: Some(0.94),
                genre_tags: vec!["Bhangra Dhol".to_string()],
                is_percussion_heavy: true,
                ..Default::default()
            };
            let t_in = TrackTransitionInput {
                track_id: "t_in".to_string(),
                deck_id: 1,
                duration_sec: 220.0,
                current_pos_sec: 0.0,
                bpm: Some(138.0),
                camelot_key: Some("6A".to_string()),
                overall_energy: Some(0.96),
                genre_tags: vec!["Bhangra Dhol".to_string()],
                is_percussion_heavy: true,
                predicted_energy_direction: Some("boost".to_string()),
                ..Default::default()
            };

            // Baraat Transition
            let ctx_baraat = PartyDecisionContext {
                dj_level: DjLevel::Level2SmartMixing,
                mood_id: "baraat".to_string(),
                target_energy: 0.96,
                recent_techniques: vec![],
                tempo_policy: dhun_lib::dj::decision_engine::TempoQualityPolicy::for_dj_level(DjLevel::Level2SmartMixing),
            };
            let plan_baraat = plan_dj_transition(&t_out, &t_in, &ctx_baraat);
            assert!(plan_baraat.duration_beats <= 8, "Baraat must enforce fast punchy transition (<= 8 beats)");
            assert_eq!(plan_baraat.musical_purpose, "build_excitement");

            // Cocktail Night Transition
            let t_cocktail_out = TrackTransitionInput {
                track_id: "t_c_out".to_string(),
                deck_id: 0,
                duration_sec: 300.0,
                current_pos_sec: 250.0,
                bpm: Some(120.0),
                camelot_key: Some("8A".to_string()),
                overall_energy: Some(0.60),
                genre_tags: vec!["Deep House".to_string()],
                is_electronic_dance: true,
                ..Default::default()
            };
            let t_cocktail_in = TrackTransitionInput {
                track_id: "t_c_in".to_string(),
                deck_id: 1,
                duration_sec: 280.0,
                current_pos_sec: 0.0,
                bpm: Some(121.0),
                camelot_key: Some("8A".to_string()),
                overall_energy: Some(0.58),
                genre_tags: vec!["Deep House".to_string()],
                is_electronic_dance: true,
                predicted_energy_direction: Some("sustain".to_string()),
                ..Default::default()
            };
            let ctx_cocktail = PartyDecisionContext {
                dj_level: DjLevel::Level2SmartMixing,
                mood_id: "cocktail_night".to_string(),
                target_energy: 0.60,
                recent_techniques: vec![],
                tempo_policy: dhun_lib::dj::decision_engine::TempoQualityPolicy::for_dj_level(DjLevel::Level2SmartMixing),
            };
            let plan_cocktail = plan_dj_transition(&t_cocktail_out, &t_cocktail_in, &ctx_cocktail);
            assert!(plan_cocktail.duration_beats >= 32, "Cocktail Night must enforce long luxurious blend (>= 32 beats)");
            assert_eq!(plan_cocktail.musical_purpose, "preserve_dance_momentum");
            assert_eq!(plan_cocktail.eq_bass_handover_schedule.incoming_low_cut_until_frac, 0.50, "Phase Guardrail: incoming low cut held until 0.50 swap");
            assert!(plan_cocktail.eq_bass_handover_schedule.outgoing_low_cut_at_frac <= 0.50, "Phase Guardrail: outgoing low cut at or before 0.50 swap");
            assert!(plan_cocktail.eq_bass_handover_schedule.mid_carve_db <= -6.0, "Phase Guardrail: mid carve at least -6 dB");
        }
    }

    /// Comprehensive deterministic test of Prompt 4 Energy Management & Mood Storytelling:
    /// 1. Section-level energy analysis vs flat track average.
    /// 2. Iconic chorus/drop protection preventing premature mix-out.
    /// 3. Sub-bass phase cancellation guardrail on 32-64 beat blends.
    /// 4. DSP meter Garba rhythm validation (checking 3/4 or 6/8 meter vs 4/4 EDM spoof).
    /// 5. Dynamic guest request replanning within set narrative.
    /// 6. Anti-fatigue guardrail capping consecutive peak-energy tracks.
    /// 7. Complete 30-track simulated set evaluation for all 14 preset moods.
    #[test]
    fn test_dj_energy_storytelling_and_full_set_narrative_simulation() {
        use dhun_lib::db::models::PhraseMarker;
        use dhun_lib::dj::energy::{
            MoodStoryCurve, SectionEnergyProfile, SetPhase, SetStoryteller,
        };
        use dhun_lib::dj::decision_engine::{
            plan_dj_transition, PartyDecisionContext, TempoQualityPolicy, TrackTransitionInput,
        };
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::dj::moods::get_preset_moods;

        // 1. Verify Section-Level Energy Analysis
        let markers = vec![
            PhraseMarker { section_type: "intro".to_string(), start_ms: 0, end_ms: 9000, energy_level: 0.50, bass_energy: 0.40, vocal_activity: 0.10, confidence: 0.85, bar_index: 0, ..Default::default() },
            PhraseMarker { section_type: "verse".to_string(), start_ms: 9000, end_ms: 27000, energy_level: 0.75, bass_energy: 0.65, vocal_activity: 0.80, confidence: 0.88, bar_index: 4, ..Default::default() },
            PhraseMarker { section_type: "build".to_string(), start_ms: 27000, end_ms: 36000, energy_level: 0.85, bass_energy: 0.70, vocal_activity: 0.40, confidence: 0.90, bar_index: 12, ..Default::default() },
            PhraseMarker { section_type: "chorus".to_string(), start_ms: 36000, end_ms: 54000, energy_level: 0.96, bass_energy: 0.92, vocal_activity: 0.90, confidence: 0.95, bar_index: 16, ..Default::default() },
            PhraseMarker { section_type: "outro".to_string(), start_ms: 54000, end_ms: 63000, energy_level: 0.55, bass_energy: 0.45, vocal_activity: 0.15, confidence: 0.85, bar_index: 24, ..Default::default() },
        ];

        let profile = SectionEnergyProfile::from_phrase_markers(&markers, 0.80);
        assert_eq!(profile.intro_energy, 0.50);
        assert_eq!(profile.peak_energy, 0.96);
        assert_eq!(profile.mix_out_energy, 0.55);
        assert!(profile.has_iconic_hook, "Chorus section must be identified as iconic hook");
        assert_eq!(profile.hook_sections.len(), 1);

        // Check hook protection query: playhead at 30s (during build heading into chorus at 36s)
        let hook_prot = profile.is_point_in_protected_hook(30_000, 10_000);
        assert!(hook_prot.is_some(), "Must detect upcoming chorus hook when within 10s lead time");

        // 2. Verify Transition Hook Protection: Mix-out held until hook finishes
        let t_out_hook = TrackTransitionInput {
            track_id: "kc_out".to_string(),
            deck_id: 0,
            title: "Kala Chashma".to_string(),
            artist: "Amar Arshi".to_string(),
            duration_sec: 180.0,
            current_pos_sec: 28.0, // Near build at 27s heading into 36s chorus!
            bpm: Some(106.0),
            camelot_key: Some("9A".to_string()),
            overall_energy: Some(0.85),
            genre_tags: vec!["Bollywood".to_string()],
            sections: markers.clone(),
            ..Default::default()
        };
        let t_in_dummy = TrackTransitionInput {
            track_id: "lt_in".to_string(),
            deck_id: 1,
            title: "London Thumakda".to_string(),
            artist: "Labh Janjua".to_string(),
            duration_sec: 210.0,
            current_pos_sec: 0.0,
            bpm: Some(106.0),
            camelot_key: Some("9A".to_string()),
            overall_energy: Some(0.82),
            genre_tags: vec!["Bollywood".to_string()],
            sections: vec![PhraseMarker { section_type: "intro".to_string(), start_ms: 0, end_ms: 12000, energy_level: 0.65, bass_energy: 0.50, vocal_activity: 0.10, confidence: 0.85, bar_index: 0, ..Default::default() }],
            ..Default::default()
        };
        let party_ctx = PartyDecisionContext {
            dj_level: DjLevel::Level2SmartMixing,
            mood_id: "shaadi".to_string(),
            target_energy: 0.76,
            recent_techniques: vec![],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level2SmartMixing),
        };
        let plan_hook = plan_dj_transition(&t_out_hook, &t_in_dummy, &party_ctx);
        assert!(
            plan_hook.outgoing_mix_out_sec >= 46.0,
            "Engine must protect iconic chorus, scheduling mix-out only after chorus phrase completes (got {:.1}s)",
            plan_hook.outgoing_mix_out_sec
        );

        // 3. Verify Anti-Fatigue Guardrail in SetStoryteller
        let mut storyteller = SetStoryteller::new("baraat", 30);
        // Simulate playing 3 consecutive peak tracks
        storyteller.record_track_played(0.98, false, false);
        storyteller.record_track_played(0.97, false, false);
        storyteller.record_track_played(0.99, false, false);

        assert_eq!(
            storyteller.current_phase(),
            SetPhase::Recovery,
            "After 3 consecutive peak tracks, storyteller must enforce a Recovery breathing phase"
        );

        // When evaluating another peak candidate during forced recovery, evaluate_candidate_energy_fit must penalize it
        let (fit_score, _, penalties) = storyteller.evaluate_candidate_energy_fit(&profile, 0.98);
        assert!(
            penalties.iter().any(|p| p.contains("Anti-Fatigue Guardrail")),
            "Must flag anti-fatigue penalty when attempting 4th consecutive peak track"
        );
        assert!(fit_score <= 15.0);

        // 4. Verify Dynamic Guest Request Replanning
        let req_action = storyteller.replan_for_guest_request("r1", "Bolo Ta Ra Ra", 0.98, true);
        assert_eq!(
            req_action.target_phase,
            SetPhase::RenewedPeak,
            "High-energy request arriving during recovery must be scheduled for Renewed Peak to protect active breathing window"
        );

        // 5. Complete 30-Track Simulation Across ALL 14 Preset Moods
        let all_moods = get_preset_moods();
        assert_eq!(all_moods.len(), 14, "Must test all 14 preset moods");

        for mood in &all_moods {
            let mut set_sim = SetStoryteller::new(&mood.id, 30);
            let _curve = MoodStoryCurve::for_mood(&mood.id);

            for i in 0..30 {
                let target_e = set_sim.calculate_target_energy(mood.target_energy);
                let track_e = (target_e + if i % 2 == 0 { 0.01 } else { -0.01 }).clamp(0.20, 1.0);
                set_sim.record_track_played(track_e, i == 15, false);
            }

            let report = set_sim.generate_set_report(30, 30, 0);
            assert_eq!(report.total_tracks, 30);
            assert!(
                report.peak_count >= 4 && report.peak_count <= 15,
                "Mood {} peak count must be balanced (got {} peaks)",
                mood.id,
                report.peak_count
            );
            assert!(
                report.recovery_count >= 2,
                "Mood {} must have at least 2 recovery breathing intervals (got {})",
                mood.id,
                report.recovery_count
            );
            assert_eq!(report.repetition_violations, 0);
            assert_eq!(report.mood_consistency_pct, 100.0);
            assert_eq!(report.hook_protection_events, 1);
        }
    }

    #[test]
    fn test_advanced_musical_analysis_and_dj_levels_1_to_4_functional_distinction() {
        use std::sync::Arc;
        use dhun_lib::analysis::phrase_detector::PhraseDetector;
        use dhun_lib::analysis::bpm_detector::BpmDetector;
        use dhun_lib::db::models::{MeterAnalysis, PhraseMarker, TrackTimingProfile, TempoSegment};
        use dhun_lib::dj::levels::DjLevel;
        use dhun_lib::dj::decision_engine::{plan_dj_transition, TrackTransitionInput, PartyDecisionContext, TempoQualityPolicy};
        use dhun_lib::dj::moods::get_preset_moods;

        // 1. Synthesize multi-section audio to verify DSP extraction (sub-bass, mid, high, vocals)
        let sr = 44100u32;
        let dur_ms = 40_000u64;
        let total_frames = (sr as f64 * (dur_ms as f64 / 1000.0)) as usize;
        let mut samples = vec![0.0f32; total_frames * 2];

        for i in 0..total_frames {
            let t = i as f64 / sr as f64;
            // Section 1 (0-16s): Verse with centered lead vocal (400 Hz) + light beat
            // Section 2 (16-32s): Chorus with heavy sub-bass (60 Hz) + wide pads + strong beat
            // Section 3 (32-40s): Outro decay
            let (sub, voc, side) = if t < 16.0 {
                (0.10 * (2.0 * std::f64::consts::PI * 60.0 * t).sin(), 0.40 * (2.0 * std::f64::consts::PI * 400.0 * t).sin(), 0.05)
            } else if t < 32.0 {
                (0.50 * (2.0 * std::f64::consts::PI * 60.0 * t).sin(), 0.35 * (2.0 * std::f64::consts::PI * 500.0 * t).sin(), 0.20)
            } else {
                (0.15 * (2.0 * std::f64::consts::PI * 60.0 * t).sin(), 0.05 * (2.0 * std::f64::consts::PI * 400.0 * t).sin(), 0.05)
            };
            let mid = sub + voc;
            samples[i * 2] = (mid + side) as f32;
            samples[i * 2 + 1] = (mid - side) as f32;
        }

        let samples_arc = Arc::new(samples);
        let phrase_res = PhraseDetector::analyze_phrasing_full(&samples_arc, sr, 120.0, 0, 4, dur_ms);

        assert!(!phrase_res.phrases.is_empty(), "Phrases must be detected");
        assert!(phrase_res.sub_bass_energy > 0.0, "Sub-bass energy (<90 Hz) must be measured");
        assert!(phrase_res.mid_energy > 0.0, "Mid energy (250-3500 Hz) must be measured");
        assert!(phrase_res.high_energy > 0.0, "High energy (>3500 Hz) must be measured");
        assert!(phrase_res.dynamic_range_db >= 3.0, "Dynamic range dB must be calculated");
        assert!(phrase_res.has_lead_vocal, "Lead vocal must be detected from centered mid frequencies");

        let meter_4_4 = MeterAnalysis { beats_per_bar: 4, meter_label: "4/4".to_string(), confidence: 0.90 };
        let rhythm_res = BpmDetector::detect_rhythm_and_percussion(&samples_arc, sr, 120.0, 0, &meter_4_4, dur_ms);

        assert_eq!(rhythm_res.alternative_bpms, vec![60.0, 240.0, 90.0], "Alternative BPMs (half, double, 3:4) must be generated");
        assert!(rhythm_res.kick_presence > 0.0, "Kick presence must be detected");
        assert!(rhythm_res.snare_clap_presence > 0.0, "Snare/clap presence must be detected");

        // 2. Verify Authentic Garba 3-Taali vs 4/4 EDM kick spoof detection
        let meter_3_4 = MeterAnalysis { beats_per_bar: 3, meter_label: "3/4".to_string(), confidence: 0.88 };
        let garba_rhythm = BpmDetector::detect_rhythm_and_percussion(&samples_arc, sr, 128.0, 0, &meter_3_4, dur_ms);
        assert_eq!(garba_rhythm.rhythmic_pattern_detected, "authentic_3_taali");
        assert!(garba_rhythm.rhythm_confidence >= 0.70);

        // 3. Confidence Decoupling: High BPM confidence (0.95) must NOT mask poor beatgrid (0.35)
        let make_timing = |bpm: f64, bg_conf: f64, bpm_conf: f64| TrackTimingProfile {
            average_bpm: bpm,
            beatgrid_confidence: bg_conf,
            bpm_confidence: bpm_conf,
            downbeat_confidence: 0.80,
            downbeat_offset_ms: 0,
            is_variable_tempo: false,
            tempo_drift_bpm: 0.0,
            meter: meter_4_4.clone(),
            tempo_map: vec![TempoSegment { start_ms: 0, end_ms: 180_000, bpm, confidence: bpm_conf }],
            beat_timestamps_ms: vec![0, 500, 1000, 1500],
            downbeat_timestamps_ms: vec![0],
            alternative_bpms: vec![bpm * 0.5, bpm * 2.0, bpm * 0.75],
            sub_bass_energy: 0.50,
            mid_energy: 0.50,
            high_energy: 0.40,
            dynamic_range_db: 12.0,
            kick_presence: 0.80,
            snare_clap_presence: 0.70,
            hihat_presence: 0.65,
            syncopation_index: 0.20,
            rhythmic_pattern_detected: "standard_pop_groove".to_string(),
            rhythm_confidence: 0.80,
            has_lead_vocal: true,
            has_backing_vocal: false,
            vocal_confidence: 0.85,
            first_musical_ms: 0,
            dialogue_end_ms: 0,
            main_drop_ms: 0,
            first_vocal_entry_ms: 0,
            first_hook_ms: 0,
            has_theatrical_alaap: false,
            theatrical_alaap_start_ms: 0,
            theatrical_alaap_end_ms: 0,
            mix_in_point_ms: 0,
            mix_out_point_ms: 180_000,
            hook_candidates: Vec::new(),
            timeline_alignment: None,
            profile_version: 4,
        };

        let out_track = TrackTransitionInput {
            track_id: "out_test".to_string(),
            deck_id: 0,
            title: "Out Track".to_string(),
            artist: "Artist 1".to_string(),
            duration_sec: 180.0,
            current_pos_sec: 155.0,
            bpm: Some(124.0),
            camelot_key: Some("8B".to_string()),
            overall_energy: Some(0.75),
            genre_tags: vec!["club".to_string(), "vocal".to_string()],
            sections: vec![
                PhraseMarker {
                    section_type: "chorus".to_string(),
                    start_ms: 120_000,
                    end_ms: 170_000,
                    energy_level: 0.85,
                    bass_energy: 0.70,
                    vocal_activity: 0.85,
                    confidence: 0.90,
                    bar_index: 32,
                    sub_bass_energy: 0.60,
                    mid_energy: 0.65,
                    high_energy: 0.50,
                    loudness_lufs_est: -9.5,
                },
                PhraseMarker {
                    section_type: "outro".to_string(),
                    start_ms: 170_000,
                    end_ms: 180_000,
                    energy_level: 0.60,
                    bass_energy: 0.40,
                    vocal_activity: 0.20,
                    confidence: 0.85,
                    bar_index: 48,
                    sub_bass_energy: 0.30,
                    mid_energy: 0.40,
                    high_energy: 0.35,
                    loudness_lufs_est: -14.0,
                },
            ],
            timing_profile: Some(make_timing(124.0, 0.35, 0.95)), // Poor beatgrid!
            ..Default::default()
        };

        let in_track = TrackTransitionInput {
            track_id: "in_test".to_string(),
            deck_id: 1,
            title: "In Track".to_string(),
            artist: "Artist 2".to_string(),
            duration_sec: 180.0,
            current_pos_sec: 0.0,
            bpm: Some(124.0),
            camelot_key: Some("9B".to_string()),
            overall_energy: Some(0.80),
            genre_tags: vec!["club".to_string(), "drop".to_string()],
            sections: vec![
                PhraseMarker {
                    section_type: "intro".to_string(),
                    start_ms: 0,
                    end_ms: 30_000,
                    energy_level: 0.65,
                    bass_energy: 0.45,
                    vocal_activity: 0.15,
                    confidence: 0.88,
                    bar_index: 0,
                    sub_bass_energy: 0.40,
                    mid_energy: 0.45,
                    high_energy: 0.40,
                    loudness_lufs_est: -12.5,
                },
            ],
            timing_profile: Some(make_timing(124.0, 0.35, 0.95)),
            ..Default::default()
        };

        let party_l1 = PartyDecisionContext {
            dj_level: DjLevel::Level1TwoDeckMixer,
            mood_id: "desi_club".to_string(),
            target_energy: 0.80,
            recent_techniques: vec![],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level1TwoDeckMixer),
        };

        let plan_l1_low_grid = plan_dj_transition(&out_track, &in_track, &party_l1);
        assert_ne!(
            plan_l1_low_grid.selected_technique, "phrase_long_blend",
            "Level 1 must refuse phrase_long_blend when beatgrid confidence is 0.35"
        );
        assert!(
            plan_l1_low_grid.selected_technique == "safe_quick_cut" || plan_l1_low_grid.selected_technique == "eq3_handover",
            "Level 1 must fall back safely on low beatgrid confidence"
        );

        // 4. Four Genuinely Different DJ Levels Comparison (Level 1 vs 2 vs 3 vs 4)
        let mut out_good = out_track.clone();
        out_good.timing_profile = Some(make_timing(124.0, 0.92, 0.95));
        let mut in_good = in_track.clone();
        in_good.timing_profile = Some(make_timing(128.0, 0.90, 0.95));
        in_good.bpm = Some(128.0);

        let plan_1 = plan_dj_transition(&out_good, &in_good, &party_l1);
        assert!(matches!(plan_1.selected_technique.as_str(), "safe_quick_cut" | "eq3_handover" | "phrase_long_blend"));
        assert!(plan_1.layered_deck_id.is_none(), "Level 1 must never layer a 3rd/4th deck");

        let party_l2 = PartyDecisionContext {
            dj_level: DjLevel::Level2SmartMixing,
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level2SmartMixing),
            ..party_l1.clone()
        };
        let plan_2 = plan_dj_transition(&out_good, &in_good, &party_l2);
        assert!(matches!(plan_2.selected_technique.as_str(), "timed_bass_swap" | "filter_sweep_drop" | "eq3_handover"));
        assert!(plan_2.layered_deck_id.is_none(), "Level 2 must never layer a 3rd/4th deck");

        let party_l3 = PartyDecisionContext {
            dj_level: DjLevel::Level3AdvancedPerformance,
            recent_techniques: vec!["beat_roll_buildup".to_string(), "filtered_buildup_drop".to_string()],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level3AdvancedPerformance),
            ..party_l1.clone()
        };
        let plan_3 = plan_dj_transition(&out_good, &in_good, &party_l3);
        assert!(matches!(plan_3.selected_technique.as_str(), "four_deck_layer" | "beat_roll_buildup" | "echo_out_blend" | "filtered_buildup_drop"));
        if plan_3.selected_technique == "four_deck_layer" {
            assert_eq!(plan_3.layered_deck_id, Some(2));
            assert_eq!(plan_3.layered_role.as_deref(), Some("percussion_groove_layer"));
        }

        let party_l4 = PartyDecisionContext {
            dj_level: DjLevel::Level4CreativeTools,
            recent_techniques: vec!["stutter_gate_drop".to_string(), "echo_freeze_drop".to_string()],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level4CreativeTools),
            ..party_l1.clone()
        };
        let plan_4 = plan_dj_transition(&out_good, &in_good, &party_l4);
        assert!(matches!(plan_4.selected_technique.as_str(), "creative_mashup_layer" | "stutter_gate_drop" | "backspin_cut" | "echo_freeze_drop" | "tempo_ramp_bridge" | "four_deck_layer" | "echo_out_blend"));
        if plan_4.selected_technique == "creative_mashup_layer" {
            assert_eq!(plan_4.layered_deck_id, Some(2));
            assert_eq!(plan_4.layered_role.as_deref(), Some("vocal_mashup_over_groove"));
        }

        // 5. Test all 14 moods across all 4 levels (56 scenarios)
        let preset_moods = get_preset_moods();
        assert_eq!(preset_moods.len(), 14);
        for m in &preset_moods {
            for lvl in [DjLevel::Level1TwoDeckMixer, DjLevel::Level2SmartMixing, DjLevel::Level3AdvancedPerformance, DjLevel::Level4CreativeTools] {
                let p = PartyDecisionContext {
                    dj_level: lvl,
                    mood_id: m.id.clone(),
                    target_energy: m.target_energy,
                    recent_techniques: vec![],
                    tempo_policy: TempoQualityPolicy::for_dj_level(lvl),
                };
                let plan = plan_dj_transition(&out_good, &in_good, &p);
                assert_eq!(plan.status, "planned");
                assert!(plan.duration_beats >= 4);
                assert!(plan.confidence >= 0.20);
            }
        }
    }
}


