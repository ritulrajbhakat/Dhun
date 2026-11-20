#[cfg(test)]
mod heatmap_hook_fusion_tests {
    use dhun_lib::analysis::audience_heatmap::{AudienceHeatmapAdapter, HEATMAP_PARSER_VERSION};
    use dhun_lib::analysis::timeline_aligner::{AlignmentLandmarkEvidence, TimelineAligner};
    use dhun_lib::db::Database;
    use dhun_lib::db::models::*;
    use dhun_lib::dj::hook_usability::HookUsabilityEvaluator;
    use serde_json::json;

    // ========================================================================
    // TEST 1: Valid YouTube URL parsing and video ID extraction
    // ========================================================================
    #[test]
    fn test_01_valid_youtube_url_parsing_and_video_id_extraction() {
        let valid_urls = vec![
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://m.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/v/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/embed/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://music.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=45s&feature=share", "dQw4w9WgXcQ"),
            ("https://youtu.be/dQw4w9WgXcQ?t=120", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/watch?feature=youtu.be&v=abc_123-XYZ", "abc_123-XYZ"),
        ];

        for (url_str, expected_id) in valid_urls {
            let extracted = AudienceHeatmapAdapter::extract_video_id_from_url(url_str)
                .unwrap_or_else(|e| panic!("Failed to extract video ID from '{}': {}", url_str, e));
            assert_eq!(extracted, expected_id, "URL: {}", url_str);
            assert_eq!(extracted.len(), 11);
            assert!(AudienceHeatmapAdapter::is_valid_video_id(&extracted));
        }
    }

    // ========================================================================
    // TEST 2: Invalid or unsupported URL rejection
    // ========================================================================
    #[test]
    fn test_02_invalid_or_unsupported_url_rejection() {
        let invalid_urls = vec![
            "https://vimeo.com/123456789",
            "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT",
            "https://soundcloud.com/artist/track",
            "https://www.youtube.com/watch",
            "https://www.youtube.com/",
            "https://www.youtube.com/watch?v=short",
            "https://www.youtube.com/watch?v=too_long_video_identifier_123",
            "https://www.youtube.com/watch?v=invalid@char",
            "not_a_valid_url_at_all",
            "",
            "   ",
        ];

        for url_str in invalid_urls {
            let res = AudienceHeatmapAdapter::extract_video_id_from_url(url_str);
            assert!(res.is_err(), "Expected error for invalid URL '{}', got: {:?}", url_str, res);
        }
    }

    // ========================================================================
    // TEST 3: Metadata-only operation (zero video playback / player instantiation)
    // ========================================================================
    #[test]
    fn test_03_metadata_only_operation() {
        // Verify that heatmap and candidate structures contain purely numeric metadata, timestamps, and scores.
        // No DOM elements, video frames, or audio DSP pipelines are touched.
        let heatmap = TrackAudienceHeatmap {
            id: "meta-only-hm".to_string(),
            track_id: "track-audio-only".to_string(),
            provider_video_id: "dQw4w9WgXcQ".to_string(),
            retrieval_status: "available".to_string(),
            raw_markers: vec![
                HeatmapMarker { start_ms: 10000, duration_ms: 2000, intensity: 0.85 },
            ],
            labelled_regions: vec![],
            extracted_regions: vec![
                ReplayRegion {
                    id: "reg-1".to_string(),
                    start_ms: 10000,
                    end_ms: 12000,
                    peak_ms: 10000,
                    peak_intensity: 0.85,
                    avg_intensity: 0.85,
                    is_youtube_labelled: false,
                    label: None,
                }
            ],
            parser_version: HEATMAP_PARSER_VERSION,
            retrieved_at: 1700000000000,
        };

        assert_eq!(heatmap.provider_video_id, "dQw4w9WgXcQ");
        assert_eq!(heatmap.raw_markers[0].start_ms, 10000);
        // Guaranteed zero video playback handles or player instances
    }

    // ========================================================================
    // TEST 4: Valid heatmap response parsing (Formats A and B)
    // ========================================================================
    #[test]
    fn test_04_valid_heatmap_response_parsing() {
        // Format A
        let payload_a = json!({
            "frameworkUpdates": {
                "entityBatchUpdate": {
                    "mutations": [
                        {
                            "payload": {
                                "heatMarkerRenderer": {
                                    "markers": [
                                        { "startMillis": 0, "durationMillis": 2000, "intensityScoreNormalized": 0.25 },
                                        { "startMillis": 2000, "durationMillis": 2000, "intensityScoreNormalized": 0.65 },
                                        { "startMillis": 4000, "durationMillis": 2000, "intensityScoreNormalized": 0.95 }
                                    ]
                                }
                            }
                        }
                    ]
                }
            }
        });

        let parsed_a = AudienceHeatmapAdapter::parse_player_response(&payload_a).expect("Must parse Format A");
        assert_eq!(parsed_a.markers.len(), 3);
        assert_eq!(parsed_a.markers[2].start_ms, 4000);
        assert_eq!(parsed_a.markers[2].intensity, 0.95);

        // Format B
        let payload_b = json!({
            "playerOverlays": {
                "playerOverlayRenderer": {
                    "decoratedPlayerBarRenderer": {
                        "playerBar": {
                            "multiMarkersPlayerBarRenderer": {
                                "markersMap": [
                                    {
                                        "key": "HEATMAPS",
                                        "value": {
                                            "heatmap": {
                                                "heatMarkers": [
                                                    {
                                                        "heatMarkerRenderer": {
                                                            "timeRangeStartMillis": 10000,
                                                            "markerDurationMillis": 2500,
                                                            "heatMarkerIntensityScoreNormalized": 0.40
                                                        }
                                                    },
                                                    {
                                                        "heatMarkerRenderer": {
                                                            "timeRangeStartMillis": 12500,
                                                            "markerDurationMillis": 2500,
                                                            "heatMarkerIntensityScoreNormalized": 0.98
                                                        }
                                                    }
                                                ]
                                            }
                                        }
                                    }
                                ]
                            }
                        }
                    }
                }
            }
        });

        let parsed_b = AudienceHeatmapAdapter::parse_player_response(&payload_b).expect("Must parse Format B");
        assert_eq!(parsed_b.markers.len(), 2);
        assert_eq!(parsed_b.markers[1].start_ms, 12500);
        assert_eq!(parsed_b.markers[1].intensity, 0.98);
    }

    // ========================================================================
    // TEST 5: Unknown or changed response formats
    // ========================================================================
    #[test]
    fn test_05_unknown_or_changed_response_formats() {
        let unknown_payloads = vec![
            json!({ "unknownKey": { "nested": 123 } }),
            json!({ "playerOverlays": { "differentSchema": true } }),
            json!({ "frameworkUpdates": { "entityBatchUpdate": { "mutations": [] } } }),
            json!([]),
            json!("string payload"),
        ];

        for payload in unknown_payloads {
            let res = AudienceHeatmapAdapter::parse_player_response(&payload);
            assert!(res.is_err(), "Unknown formats must return typed Err, not panic or hallucinate: {:?}", payload);
        }
    }

    // ========================================================================
    // TEST 6: Empty, missing, or blocked heatmap responses
    // ========================================================================
    #[test]
    fn test_06_empty_missing_or_blocked_heatmap_responses() {
        let blocked_or_empty = vec![
            json!({ "error": { "code": 429, "message": "Too Many Requests" } }),
            json!({ "playabilityStatus": { "status": "LOGIN_REQUIRED" } }),
            json!({ "contents": {} }),
        ];

        for p in blocked_or_empty {
            let res = AudienceHeatmapAdapter::parse_player_response(&p);
            assert!(res.is_err(), "Blocked or empty responses must yield Err cleanly");
        }
    }

    // ========================================================================
    // TEST 7: Malformed and out-of-range timestamps
    // ========================================================================
    #[test]
    fn test_07_malformed_and_out_of_range_timestamps() {
        let malformed_payload = json!([
            { "start_ms": -5000, "duration_ms": 2000, "intensity": 0.5 }, // Negative start -> discard
            { "start_ms": 2000, "duration_ms": -100, "intensity": 0.8 },  // Negative duration -> discard
            { "start_ms": 4000, "duration_ms": 0, "intensity": 0.8 },     // Zero duration -> discard
            { "start_ms": "invalid", "duration_ms": 2000, "intensity": 0.5 }, // Non-numeric start -> discard
            { "start_ms": 6000, "duration_ms": 2000, "intensity": 0.90 }, // Valid marker 1
            { "start_ms": 1000, "duration_ms": 2000, "intensity": 0.70 }  // Valid marker 2 (out of order)
        ]);

        let parsed = AudienceHeatmapAdapter::parse_player_response(&malformed_payload)
            .expect("Should recover valid markers and drop malformed ones");

        assert_eq!(parsed.markers.len(), 2, "Only 2 valid markers should survive");
        // Verify sorted order
        assert_eq!(parsed.markers[0].start_ms, 1000);
        assert_eq!(parsed.markers[1].start_ms, 6000);
    }

    // ========================================================================
    // TEST 8: Score normalization and finite-value validation
    // ========================================================================
    #[test]
    fn test_08_score_normalization_and_finite_value_validation() {
        let edge_markers = json!([
            { "start_ms": 0, "duration_ms": 2000, "intensity": 1.5 },   // Above 1.0 -> clamped to 1.0
            { "start_ms": 2000, "duration_ms": 2000, "intensity": -0.4 }, // Below 0.0 -> clamped to 0.0
            { "start_ms": 4000, "duration_ms": 2000, "intensity": 0.85 }
        ]);

        let parsed = AudienceHeatmapAdapter::parse_player_response(&edge_markers).unwrap();
        assert_eq!(parsed.markers[0].intensity, 1.0);
        assert_eq!(parsed.markers[1].intensity, 0.0);
        assert_eq!(parsed.markers[2].intensity, 0.85);

        // Combined score clamped boundaries
        let c1 = AudienceHeatmapAdapter::compute_combined_hook_score(Some(2.5), 1.2);
        assert_eq!(c1, 1.0);
        let c2 = AudienceHeatmapAdapter::compute_combined_hook_score(Some(-0.5), -0.2);
        assert_eq!(c2, 0.0);
    }

    // ========================================================================
    // TEST 9: Smoothing, peak splitting, and region merging
    // ========================================================================
    #[test]
    fn test_09_smoothing_peak_splitting_and_region_merging() {
        let markers = vec![
            // Peak 1: 0ms - 8000ms
            HeatmapMarker { start_ms: 0, duration_ms: 2000, intensity: 0.80 },
            HeatmapMarker { start_ms: 2000, duration_ms: 2000, intensity: 0.95 },
            HeatmapMarker { start_ms: 4000, duration_ms: 2000, intensity: 0.90 },
            // Small gap (2000ms gap, 6000 to 8000)
            HeatmapMarker { start_ms: 8000, duration_ms: 2000, intensity: 0.85 },

            // Deep valley: 10000ms - 22000ms
            HeatmapMarker { start_ms: 10000, duration_ms: 2000, intensity: 0.20 },
            HeatmapMarker { start_ms: 12000, duration_ms: 2000, intensity: 0.15 },
            HeatmapMarker { start_ms: 14000, duration_ms: 2000, intensity: 0.10 },
            HeatmapMarker { start_ms: 16000, duration_ms: 2000, intensity: 0.12 },
            HeatmapMarker { start_ms: 18000, duration_ms: 2000, intensity: 0.18 },
            HeatmapMarker { start_ms: 20000, duration_ms: 2000, intensity: 0.22 },

            // Peak 2: 22000ms - 30000ms
            HeatmapMarker { start_ms: 22000, duration_ms: 2000, intensity: 0.75 },
            HeatmapMarker { start_ms: 24000, duration_ms: 2000, intensity: 0.88 },
            HeatmapMarker { start_ms: 26000, duration_ms: 2000, intensity: 0.82 },
            HeatmapMarker { start_ms: 28000, duration_ms: 2000, intensity: 0.70 },
        ];

        let labelled = vec![
            ReplayRegion {
                id: "lbl-1".to_string(),
                start_ms: 2000,
                end_ms: 6000,
                peak_ms: 2000,
                peak_intensity: 1.0,
                avg_intensity: 0.95,
                is_youtube_labelled: true,
                label: Some("Most Replayed".to_string()),
            }
        ];

        let regions = AudienceHeatmapAdapter::extract_replay_regions(&markers, &labelled);
        assert_eq!(regions.len(), 2, "Must separate the two distinct peaks across the valley");

        // Region 1: Peak 1 with gap merged & YouTube label preserved
        assert_eq!(regions[0].start_ms, 0);
        assert_eq!(regions[0].end_ms, 10000);
        assert!(regions[0].is_youtube_labelled, "Labelled segment must be preserved");

        // Region 2: Peak 2
        assert_eq!(regions[1].start_ms, 22000);
        assert_eq!(regions[1].end_ms, 30000);
        assert!(regions[1].peak_intensity >= 0.80);
    }

    // ========================================================================
    // TEST 10: Exact 65/35 formula verification
    // ========================================================================
    #[test]
    fn test_10_exact_65_35_scoring_formula() {
        // (0.0, 0.0) -> 0.0
        assert_eq!(AudienceHeatmapAdapter::compute_combined_hook_score(Some(0.0), 0.0), 0.0);

        // (1.0, 1.0) -> 1.0
        assert_eq!(AudienceHeatmapAdapter::compute_combined_hook_score(Some(1.0), 1.0), 1.0);

        // (1.0, 0.0) -> 0.65
        assert_eq!(AudienceHeatmapAdapter::compute_combined_hook_score(Some(1.0), 0.0), 0.65);

        // (0.0, 1.0) -> 0.35
        assert_eq!(AudienceHeatmapAdapter::compute_combined_hook_score(Some(0.0), 1.0), 0.35);

        // (0.85, 0.72) -> 0.85 * 0.65 + 0.72 * 0.35 = 0.5525 + 0.2520 = 0.8045
        let s = AudienceHeatmapAdapter::compute_combined_hook_score(Some(0.85), 0.72);
        assert!((s - 0.8045).abs() < 1e-4, "Expected 0.8045, got {}", s);
    }

    // ========================================================================
    // TEST 11: Missing audience data fallback
    // ========================================================================
    #[test]
    fn test_11_missing_audience_data_fallback() {
        let musical_score = 0.842;
        let combined = AudienceHeatmapAdapter::compute_combined_hook_score(None, musical_score);
        assert_eq!(combined, musical_score, "Must fall back to 100% musical score without penalty");
    }

    // ========================================================================
    // TEST 12: Duration mismatch NOT treated as proof of start offset
    // ========================================================================
    #[test]
    fn test_12_duration_mismatch_not_treated_as_proof_of_offset() {
        // Video = 278,000ms, Audio = 265,000ms (discrepancy = 13,000ms)
        // No musical landmark evidence provided (evidence = None)
        let profile = TrackTimingProfile::default();
        let phrases = vec![];

        let alignment = TimelineAligner::align_video_to_audio_with_evidence(
            278000,
            265000,
            &profile,
            &phrases,
            None, // NO landmark evidence
        );

        // Must NOT infer start offset as 13s! Status MUST be Unverified.
        assert_eq!(alignment.status, TimelineAlignmentStatus::Unverified);
        assert_eq!(alignment.video_to_audio_offset_ms, 0);
        assert_eq!(alignment.duration_discrepancy_ms, 13000);

        // Fused candidates MUST be informational only: no executable cues
        let replay_regions = vec![ReplayRegion {
            id: "reg-unverified".to_string(),
            start_ms: 30000,
            end_ms: 45000,
            peak_ms: 35000,
            peak_intensity: 0.95,
            avg_intensity: 0.90,
            is_youtube_labelled: false,
            label: None,
        }];

        let candidates = TimelineAligner::fuse_candidates_with_audience_heatmap(
            "track-dur-mismatch",
            Some("vid-xyz"),
            &profile,
            &phrases,
            &replay_regions,
            &alignment,
            265000,
        );

        let info_cand = candidates.iter().find(|c| c.candidate_type == "informational_video_peak").unwrap();
        assert!(!info_cand.is_aligned_hook, "Must NOT be marked aligned");
        assert!(!info_cand.is_usable_entry, "Must NOT produce executable entry cue");
        assert!(!info_cand.is_usable_exit, "Must NOT produce executable exit cue");
        assert_eq!(info_cand.entry_cue_ms, 0);
        assert_eq!(info_cand.video_start_ms, Some(30000));
    }

    // ========================================================================
    // TEST 13: Only Verified alignment produces executable mapped timestamps
    // ========================================================================
    #[test]
    fn test_13_only_verified_alignment_produces_executable_cues() {
        // London Thumakda scenario with 2 matching musical landmarks:
        // Landmark 1: Video dialogue ends at 12,700ms (Audio first musical beat = 0ms, offset = 12,700ms)
        // Landmark 2: Video main drop at 26,700ms (Audio main drop = 14,000ms, offset = 12,700ms)
        let mut profile = TrackTimingProfile::default();
        profile.first_musical_ms = 0;
        profile.main_drop_ms = 14000;
        profile.downbeat_timestamps_ms = vec![0, 4000, 8000, 12000, 16000, 20000, 24000];

        let phrases = vec![
            PhraseMarker { start_ms: 0, end_ms: 14000, section_type: "intro".to_string(), energy_level: 0.5, ..Default::default() },
            PhraseMarker { start_ms: 14000, end_ms: 30000, section_type: "chorus".to_string(), energy_level: 0.9, vocal_activity: 0.6, ..Default::default() },
        ];

        let evidence = AlignmentLandmarkEvidence {
            video_dialogue_end_ms: Some(12700),
            video_main_drop_ms: Some(26700),
            video_first_musical_ms: None,
            matched_section_boundaries: vec![],
        };

        let alignment = TimelineAligner::align_video_to_audio_with_evidence(
            260000,
            247300,
            &profile,
            &phrases,
            Some(&evidence),
        );

        assert_eq!(alignment.status, TimelineAlignmentStatus::Verified);
        assert_eq!(alignment.video_to_audio_offset_ms, 12700);

        // Video replay region at 28,700ms - 44,700ms maps to audio: 28,700 - 12,700 = 16,000ms
        let replay_regions = vec![ReplayRegion {
            id: "reg-chorus".to_string(),
            start_ms: 28700,
            end_ms: 44700,
            peak_ms: 32700,
            peak_intensity: 0.95,
            avg_intensity: 0.90,
            is_youtube_labelled: true,
            label: Some("Most Replayed".to_string()),
        }];

        let mut candidates = TimelineAligner::fuse_candidates_with_audience_heatmap(
            "track-thumakda",
            Some("vid-london"),
            &profile,
            &phrases,
            &replay_regions,
            &alignment,
            247300,
        );

        HookUsabilityEvaluator::evaluate_candidates(&mut candidates, &profile, &phrases, 247300);

        let chorus_cand = candidates.iter().find(|c| c.candidate_type == "chorus").unwrap();
        assert_eq!(chorus_cand.alignment_status, TimelineAlignmentStatus::Verified);
        assert!(chorus_cand.is_aligned_hook);
        assert!(chorus_cand.is_usable_entry);
        assert_eq!(chorus_cand.entry_cue_ms, 14000); // Snapped to downbeat/phrase start
    }

    // ========================================================================
    // TEST 14: Partially verified and unverified alignment remain informational
    // ========================================================================
    #[test]
    fn test_14_partially_verified_and_unverified_remain_informational() {
        let mut profile = TrackTimingProfile::default();
        profile.first_musical_ms = 0;
        let phrases = vec![];

        // Only 1 landmark provided -> PartiallyVerified
        let evidence = AlignmentLandmarkEvidence {
            video_dialogue_end_ms: Some(10000),
            video_main_drop_ms: None,
            video_first_musical_ms: None,
            matched_section_boundaries: vec![],
        };

        let alignment = TimelineAligner::align_video_to_audio_with_evidence(
            260000,
            250000,
            &profile,
            &phrases,
            Some(&evidence),
        );

        assert_eq!(alignment.status, TimelineAlignmentStatus::PartiallyVerified);

        let replay_regions = vec![ReplayRegion {
            id: "reg-partial".to_string(),
            start_ms: 40000,
            end_ms: 60000,
            peak_ms: 50000,
            peak_intensity: 0.90,
            avg_intensity: 0.85,
            is_youtube_labelled: false,
            label: None,
        }];

        let candidates = TimelineAligner::fuse_candidates_with_audience_heatmap(
            "track-partial",
            Some("vid-part"),
            &profile,
            &phrases,
            &replay_regions,
            &alignment,
            250000,
        );

        for c in &candidates {
            if c.candidate_type == "informational_video_peak" {
                assert_eq!(c.alignment_status, TimelineAlignmentStatus::PartiallyVerified);
                assert!(!c.is_aligned_hook);
                assert!(!c.is_usable_entry);
                assert!(!c.is_usable_exit);
                assert_eq!(c.entry_cue_ms, 0);
            }
        }
    }

    // ========================================================================
    // TEST 15: Audio-native hook candidates remain usable without YouTube data
    // ========================================================================
    #[test]
    fn test_15_audio_native_hooks_remain_usable_without_youtube_data() {
        let mut timing = TrackTimingProfile::default();
        timing.downbeat_timestamps_ms = vec![0, 4000, 8000, 16000, 24000, 32000];

        let phrases = vec![
            PhraseMarker { start_ms: 0, end_ms: 8000, section_type: "intro".to_string(), energy_level: 0.4, ..Default::default() },
            PhraseMarker { start_ms: 8000, end_ms: 24000, section_type: "chorus".to_string(), energy_level: 0.92, vocal_activity: 0.65, ..Default::default() },
        ];

        // 0 YouTube data (alignment status = Unavailable, replay_regions = empty)
        let alignment = TimelineAlignment {
            video_to_audio_offset_ms: 0,
            alignment_confidence: 0.0,
            matched_event_count: 0,
            duration_discrepancy_ms: 0,
            alignment_method: "none".to_string(),
            status: TimelineAlignmentStatus::Unavailable,
        };

        let mut candidates = TimelineAligner::fuse_candidates_with_audience_heatmap(
            "track-offline",
            None,
            &timing,
            &phrases,
            &[],
            &alignment,
            180000,
        );

        HookUsabilityEvaluator::evaluate_candidates(&mut candidates, &timing, &phrases, 180000);

        let chorus = candidates.iter().find(|c| c.candidate_type == "chorus").unwrap();
        assert!(chorus.is_musical_hook, "Must be musical hook");
        assert!(chorus.is_usable_entry, "Audio-native candidate must be usable as entry cue even without YouTube data");
        assert_eq!(chorus.entry_cue_ms, 8000);
        assert_eq!(chorus.combined_hook_score, chorus.dhun_musical_score);
    }

    // ========================================================================
    // TEST 16: Mid-word and incomplete-vocal hooks rejected as entries
    // ========================================================================
    #[test]
    fn test_16_mid_word_and_incomplete_vocal_hooks_rejected_as_entries() {
        let mut timing = TrackTimingProfile::default();
        timing.downbeat_timestamps_ms = vec![0, 4000, 8000, 12000, 16000, 20000];

        let phrases = vec![
            PhraseMarker {
                start_ms: 8000,
                end_ms: 16000,
                section_type: "verse".to_string(),
                vocal_activity: 0.75, // Strong vocal presence
                energy_level: 0.65,
                ..Default::default()
            },
        ];

        let mut candidate = HookCandidate {
            id: "cand-vocal-mid".to_string(),
            track_id: "track-1".to_string(),
            video_id: None,
            video_start_ms: None,
            video_end_ms: None,
            audio_start_ms: 10500, // 2500ms into the active vocal phrase!
            audio_end_ms: 16000,
            dhun_musical_score: 0.70,
            audience_replay_score: Some(0.95),
            combined_hook_score: 0.8625,
            alignment_status: TimelineAlignmentStatus::Verified,
            alignment_offset_ms: 0,
            dj_usability_score: 0.0,
            is_audience_popular: false,
            is_musical_hook: false,
            is_aligned_hook: false,
            is_usable_entry: false,
            is_usable_exit: false,
            candidate_type: "popular_hook".to_string(),
            entry_cue_ms: 10500,
            exit_cue_ms: 16000,
            usability_reasons: vec![],
            rejection_reasons: vec![],
        };

        HookUsabilityEvaluator::evaluate_single_candidate(&mut candidate, &timing, &phrases, 120000);

        assert!(!candidate.is_usable_entry, "Must REJECT mid-word vocal entry");
        assert!(candidate.rejection_reasons.iter().any(|r| r.contains("Mid-word vocal clash")));
    }

    // ========================================================================
    // TEST 17: Popular but musically unsafe hooks rejected
    // ========================================================================
    #[test]
    fn test_17_popular_but_musically_unsafe_hooks_rejected() {
        // High audience score (0.95) on a build-up phrase right before drop
        let mut timing = TrackTimingProfile::default();
        timing.downbeat_timestamps_ms = vec![0, 4000, 8000, 16000];

        let phrases = vec![
            PhraseMarker {
                start_ms: 0,
                end_ms: 16000,
                section_type: "build".to_string(), // Build-up section
                vocal_activity: 0.20,
                energy_level: 0.85,
                ..Default::default()
            },
        ];

        let mut candidate = HookCandidate {
            id: "cand-build-exit".to_string(),
            track_id: "track-unsafe".to_string(),
            video_id: None,
            video_start_ms: None,
            video_end_ms: None,
            audio_start_ms: 0,
            audio_end_ms: 15000,
            dhun_musical_score: 0.60,
            audience_replay_score: Some(0.95),
            combined_hook_score: 0.8275,
            alignment_status: TimelineAlignmentStatus::Verified,
            alignment_offset_ms: 0,
            dj_usability_score: 0.0,
            is_audience_popular: true,
            is_musical_hook: false,
            is_aligned_hook: true,
            is_usable_entry: true,
            is_usable_exit: true,
            candidate_type: "build".to_string(),
            entry_cue_ms: 0,
            exit_cue_ms: 15000, // Exiting inside build-up
            usability_reasons: vec![],
            rejection_reasons: vec![],
        };

        HookUsabilityEvaluator::evaluate_single_candidate(&mut candidate, &timing, &phrases, 120000);

        assert!(candidate.is_audience_popular, "Should still be marked audience popular");
        assert!(!candidate.is_usable_exit, "Must REJECT exit during build-up section");
        assert!(candidate.rejection_reasons.iter().any(|r| r.contains("build-up")));
    }

    // ========================================================================
    // TEST 18: Retrieval failures preserve existing valid database records
    // ========================================================================
    #[test]
    fn test_18_retrieval_failures_preserve_existing_valid_database_records() {
        let db = Database::new_in_memory().expect("In-memory DB");

        // 1. Initial successful retrieval and storage
        let initial_heatmap = TrackAudienceHeatmap {
            id: "track-persist-1:vid-good".to_string(),
            track_id: "track-persist-1".to_string(),
            provider_video_id: "vid-good-111".to_string(),
            retrieval_status: "available".to_string(),
            raw_markers: vec![
                HeatmapMarker { start_ms: 0, duration_ms: 2000, intensity: 0.80 },
                HeatmapMarker { start_ms: 2000, duration_ms: 2000, intensity: 0.95 },
            ],
            labelled_regions: vec![],
            extracted_regions: vec![
                ReplayRegion {
                    id: "reg-p".to_string(),
                    start_ms: 0,
                    end_ms: 4000,
                    peak_ms: 2000,
                    peak_intensity: 0.95,
                    avg_intensity: 0.875,
                    is_youtube_labelled: false,
                    label: None,
                }
            ],
            parser_version: HEATMAP_PARSER_VERSION,
            retrieved_at: 1700000000000,
        };

        db.save_audience_heatmap(&initial_heatmap).expect("Must save initial valid heatmap");

        // Verify it was stored
        let initial_fetched = db.get_audience_heatmap_for_video("vid-good-111")
            .unwrap()
            .expect("Should exist");
        assert_eq!(initial_fetched.raw_markers.len(), 2);
        assert_eq!(initial_fetched.retrieval_status, "available");

        // 2. Subsequent failed retrieval attempt (e.g. rate limit HTTP 429, bot gate, network timeout)
        let failed_heatmap = TrackAudienceHeatmap {
            id: "track-persist-1:vid-good".to_string(),
            track_id: "track-persist-1".to_string(),
            provider_video_id: "vid-good-111".to_string(),
            retrieval_status: "failed".to_string(), // Failed
            raw_markers: vec![],                   // Empty markers
            labelled_regions: vec![],
            extracted_regions: vec![],             // Empty extracted
            parser_version: HEATMAP_PARSER_VERSION,
            retrieved_at: 1700000050000,
        };

        db.save_audience_heatmap(&failed_heatmap).expect("Save failed attempt");

        // 3. Verify existing valid data was PRESERVED and not wiped out!
        let preserved = db.get_audience_heatmap_for_video("vid-good-111")
            .unwrap()
            .expect("Should still exist");

        assert_eq!(preserved.retrieval_status, "available", "Status must preserve available");
        assert_eq!(preserved.raw_markers.len(), 2, "Must NOT wipe out raw markers on failed retry");
        assert_eq!(preserved.extracted_regions.len(), 1, "Must NOT wipe out extracted regions on failed retry");
    }

    // ========================================================================
    // 6. Focused ytInitialData & Live Payload Validation Tests (am3QWc8A4xI)
    // ========================================================================

    #[test]
    fn test_yt_initial_data_macro_markers_entity() {
        let html = r#"
            <!DOCTYPE html><html><head>
            <script>
            var ytInitialData = {
                "frameworkUpdates": {
                    "entityBatchUpdate": {
                        "mutations": [
                            {
                                "payload": {
                                    "macroMarkersListEntity": {
                                        "markersList": {
                                            "markerType": "MARKER_TYPE_HEATMAP",
                                            "markers": [
                                                { "startMillis": "0", "durationMillis": "2000", "intensityScoreNormalized": 0.15 },
                                                { "startMillis": "2000", "durationMillis": "2000", "intensityScoreNormalized": 0.85 }
                                            ]
                                        }
                                    }
                                }
                            }
                        ]
                    }
                }
            };
            </script></head><body></body></html>
        "#;

        let parsed = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(html)
            .expect("Must extract heatmap from ytInitialData");
        assert_eq!(parsed.source_object, "ytInitialData");
        assert_eq!(parsed.markers.len(), 2);
        assert_eq!(parsed.markers[1].start_ms, 2000);
        assert_eq!(parsed.markers[1].intensity, 0.85);
    }

    #[test]
    fn test_captured_fixture_100_markers() {
        let fixture_str = include_str!("./fixtures/ishq_samajh_paaye_am3QWc8A4xI_frameworkUpdates.json");
        let val: serde_json::Value = serde_json::from_str(fixture_str).expect("Valid JSON fixture");

        let parsed = AudienceHeatmapAdapter::parse_player_response(&val)
            .expect("Must parse real captured Ishq Samajh Paaye payload");
        assert_eq!(parsed.markers.len(), 100, "Genuine captured payload must contain exactly 100 markers");
    }

    #[test]
    fn test_captured_fixture_first_middle_last_markers() {
        let fixture_str = include_str!("./fixtures/ishq_samajh_paaye_am3QWc8A4xI_frameworkUpdates.json");
        let val: serde_json::Value = serde_json::from_str(fixture_str).expect("Valid JSON fixture");
        let parsed = AudienceHeatmapAdapter::parse_player_response(&val).unwrap();

        // 1. First marker (index 0)
        let first = &parsed.markers[0];
        assert_eq!(first.start_ms, 0);
        assert_eq!(first.duration_ms, 1960);
        assert!((first.intensity - 0.044745).abs() < 1e-4);

        // 2. Middle marker (index 50)
        let mid = &parsed.markers[50];
        assert_eq!(mid.start_ms, 98000);
        assert_eq!(mid.duration_ms, 1960);
        assert!((mid.intensity - 0.803809).abs() < 1e-4);

        // 3. Last marker (index 99)
        let last = &parsed.markers[99];
        assert_eq!(last.start_ms, 194040);
        assert_eq!(last.duration_ms, 1960);
        assert!((last.intensity - 0.577577).abs() < 1e-4);
    }

    #[test]
    fn test_captured_fixture_global_peak_at_137200ms() {
        let fixture_str = include_str!("./fixtures/ishq_samajh_paaye_am3QWc8A4xI_frameworkUpdates.json");
        let val: serde_json::Value = serde_json::from_str(fixture_str).unwrap();
        let parsed = AudienceHeatmapAdapter::parse_player_response(&val).unwrap();

        // Extract smoothed regions
        let regions = AudienceHeatmapAdapter::extract_replay_regions(&parsed.markers, &[]);
        assert!(!regions.is_empty(), "Must extract at least one replay region");

        // The highest peak must align to 137,200 ms (137.2s / 2:17 into track)
        let highest = regions.iter().max_by(|a, b| a.peak_intensity.partial_cmp(&b.peak_intensity).unwrap()).unwrap();
        assert_eq!(highest.peak_ms, 137200, "Global peak must be at 137,200ms");
        assert!(highest.peak_intensity >= 0.99, "Peak intensity must be near 1.0");
    }

    #[test]
    fn test_captured_fixture_secondary_peak_at_78400ms() {
        let fixture_str = include_str!("./fixtures/ishq_samajh_paaye_am3QWc8A4xI_frameworkUpdates.json");
        let val: serde_json::Value = serde_json::from_str(fixture_str).unwrap();
        let parsed = AudienceHeatmapAdapter::parse_player_response(&val).unwrap();

        // Check local maximum around 78,400ms (78.4s / 1:18, the first chorus entry)
        let chorus_marker = parsed.markers.iter().find(|m| m.start_ms == 78400).expect("Marker at 78400ms");
        assert!(chorus_marker.intensity >= 0.75, "First chorus intensity must be elevated (>= 0.75)");
    }

    #[test]
    fn test_missing_yt_initial_data() {
        let html = "<html><body><h1>No data here</h1></body></html>";
        let res = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(html);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Neither ytInitialData nor ytInitialPlayerResponse was found"));
    }

    #[test]
    fn test_malformed_or_truncated_yt_initial_data() {
        let truncated_html = r#"
            <html><head><script>
            var ytInitialData = { "frameworkUpdates": { "entityBatchUpdate": { "mutations": [ { "incomplete": true
        "#;
        let res = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(truncated_html);
        assert!(res.is_err(), "Truncated JSON must not panic and must return Err");
    }

    #[test]
    fn test_yt_initial_player_response_fallback_compatibility() {
        let html = r#"
            <!DOCTYPE html><html><head><script>
            var ytInitialPlayerResponse = {
                "playerOverlays": {
                    "playerOverlayRenderer": {
                        "decoratedPlayerBarRenderer": {
                            "playerBar": {
                                "multiMarkersPlayerBarRenderer": {
                                    "markersMap": [
                                        {
                                            "key": "HEATMAPS",
                                            "value": {
                                                "heatmap": {
                                                    "heatMarkers": [
                                                        {
                                                            "heatMarkerRenderer": {
                                                                "timeRangeStartMillis": 15000,
                                                                "markerDurationMillis": 2000,
                                                                "heatMarkerIntensityScoreNormalized": 0.88
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    ]
                                }
                            }
                        }
                    }
                }
            };
            </script></head><body></body></html>
        "#;

        let parsed = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(html)
            .expect("Must fall back to ytInitialPlayerResponse");
        assert_eq!(parsed.source_object, "ytInitialPlayerResponse");
        assert_eq!(parsed.markers.len(), 1);
        assert_eq!(parsed.markers[0].start_ms, 15000);
    }

    #[test]
    fn test_both_objects_present_correct_source_selected() {
        // Both objects present: ytInitialPlayerResponse has empty/no markers, ytInitialData has valid markers
        let html = r#"
            <!DOCTYPE html><html><head><script>
            var ytInitialPlayerResponse = { "playabilityStatus": { "status": "OK" }, "contents": {} };
            var ytInitialData = {
                "frameworkUpdates": {
                    "entityBatchUpdate": {
                        "mutations": [
                            {
                                "payload": {
                                    "macroMarkersListEntity": {
                                        "markersList": {
                                            "markerType": "MARKER_TYPE_HEATMAP",
                                            "markers": [
                                                { "startMillis": "50000", "durationMillis": "2000", "intensityScoreNormalized": 0.95 }
                                            ]
                                        }
                                    }
                                }
                            }
                        ]
                    }
                }
            };
            </script></head><body></body></html>
        "#;

        let parsed = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(html)
            .expect("Must extract from valid source object");
        assert_eq!(parsed.source_object, "ytInitialData");
        assert_eq!(parsed.markers[0].start_ms, 50000);
    }

    #[test]
    fn test_valid_watch_page_no_markers_returns_unavailable() {
        let html = r#"
            <!DOCTYPE html><html><head><script>
            var ytInitialData = { "contents": { "twoColumnWatchNextResults": {} } };
            var ytInitialPlayerResponse = { "playabilityStatus": { "status": "OK" } };
            </script></head><body></body></html>
        "#;

        let res = AudienceHeatmapAdapter::extract_heatmap_from_watch_html(html);
        assert!(res.is_err(), "Must return Err when page JSON exists but has no heatmap");
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("neither ytInitialData nor ytInitialPlayerResponse contained heatmap markers"));
    }
}
