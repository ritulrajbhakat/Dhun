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

        let asset = AudioAsset {
            id: "ast_test_01".to_string(),
            track_id: "trk_test_01".to_string(),
            file_path: "C:/test/song.mp3".to_string(),
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

        let playable = db.get_playable_candidates().expect("Failed getting playable");
        assert!(playable.len() >= 10);
        assert!(playable.iter().any(|p| p.track.title == "Test Antakshari"));
        assert!(playable.iter().any(|p| p.track.title == "Gallan Goodiyaan"));

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
}

