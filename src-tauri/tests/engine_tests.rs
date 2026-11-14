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
}

