use serde::{Deserialize, Serialize};

use crate::analysis::audience_heatmap::AudienceHeatmapAdapter;
use crate::db::models::{
    HookCandidate, PhraseMarker, ReplayRegion, TimelineAlignment,
    TimelineAlignmentStatus, TrackTimingProfile,
};

/// Explicit musical landmarks and structural events used to establish timeline alignment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlignmentLandmarkEvidence {
    pub video_first_musical_ms: Option<i64>,
    pub video_dialogue_end_ms: Option<i64>,
    pub video_main_drop_ms: Option<i64>,
    pub matched_section_boundaries: Vec<(i64, i64)>, // (video_timestamp_ms, audio_timestamp_ms)
}

pub struct TimelineAligner;

impl TimelineAligner {
    /// Computes vertical timeline alignment between YouTube video and authorized audio.
    /// Strictly rejects inferring offsets from duration differences alone.
    /// Alignment must use actual corresponding musical landmarks or defensible matching evidence.
    pub fn align_video_to_audio(
        video_duration_ms: u64,
        audio_duration_ms: u64,
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
    ) -> TimelineAlignment {
        Self::align_video_to_audio_with_evidence(
            video_duration_ms,
            audio_duration_ms,
            timing_profile,
            phrases,
            None,
        )
    }

    /// Computes vertical timeline alignment with optional explicit landmark evidence.
    pub fn align_video_to_audio_with_evidence(
        video_duration_ms: u64,
        audio_duration_ms: u64,
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
        evidence: Option<&AlignmentLandmarkEvidence>,
    ) -> TimelineAlignment {
        if video_duration_ms == 0 || audio_duration_ms == 0 {
            return TimelineAlignment {
                video_to_audio_offset_ms: 0,
                alignment_confidence: 0.0,
                matched_event_count: 0,
                duration_discrepancy_ms: (video_duration_ms as i64 - audio_duration_ms as i64).abs(),
                alignment_method: "missing_duration_metadata".to_string(),
                status: TimelineAlignmentStatus::Unavailable,
            };
        }

        let duration_diff_ms = video_duration_ms as i64 - audio_duration_ms as i64;
        let abs_duration_diff_ms = duration_diff_ms.abs();

        // Case A: Near-identical timeline (within 1200ms / ~1 bar tolerance)
        // Same edit, duration matches within audio boundary tolerance
        if abs_duration_diff_ms <= 1200 {
            let mut matched_events = 2; // duration match + start match
            if timing_profile.main_drop_ms > 0 {
                matched_events += 1;
            }
            if !phrases.is_empty() {
                matched_events += phrases.len().min(4);
            }

            return TimelineAlignment {
                video_to_audio_offset_ms: 0,
                alignment_confidence: 0.96,
                matched_event_count: matched_events,
                duration_discrepancy_ms: abs_duration_diff_ms,
                alignment_method: "near_identical_duration_and_meter".to_string(),
                status: TimelineAlignmentStatus::Verified,
            };
        }

        // Case B: Duration mismatch WITH explicit landmark evidence
        // (e.g. video dialogue end and main drop match audio first musical beat and main drop)
        if let Some(ev) = evidence {
            let mut observed_offsets: Vec<i64> = Vec::new();

            // 1. Dialogue / skit end matching
            if let Some(v_dia) = ev.video_dialogue_end_ms {
                // If audio first musical sound begins near 0ms
                let a_mus = timing_profile.first_musical_ms;
                observed_offsets.push(v_dia - a_mus);
            }

            // 2. Main drop matching
            if let Some(v_drop) = ev.video_main_drop_ms {
                if timing_profile.main_drop_ms > 0 {
                    observed_offsets.push(v_drop - timing_profile.main_drop_ms);
                }
            }

            // 3. First musical entry matching
            if let Some(v_first) = ev.video_first_musical_ms {
                if timing_profile.first_musical_ms >= 0 {
                    observed_offsets.push(v_first - timing_profile.first_musical_ms);
                }
            }

            // 4. Section boundary pairs
            for &(v_ms, a_ms) in &ev.matched_section_boundaries {
                observed_offsets.push(v_ms - a_ms);
            }

            if !observed_offsets.is_empty() {
                // Check if observed landmark offsets agree with each other (within +-350ms)
                let first_offset = observed_offsets[0];
                let all_agree = observed_offsets.iter().all(|&off| (off - first_offset).abs() <= 350);

                if all_agree {
                    let avg_offset = (observed_offsets.iter().sum::<i64>() as f64 / observed_offsets.len() as f64).round() as i64;
                    let matched_count = observed_offsets.len();

                    if matched_count >= 2 {
                        return TimelineAlignment {
                            video_to_audio_offset_ms: avg_offset,
                            alignment_confidence: 0.92,
                            matched_event_count: matched_count + 1,
                            duration_discrepancy_ms: abs_duration_diff_ms,
                            alignment_method: "multiple_corresponding_musical_landmarks".to_string(),
                            status: TimelineAlignmentStatus::Verified,
                        };
                    } else {
                        // Single landmark: partially verified (informational only, not eligible for executable cues)
                        return TimelineAlignment {
                            video_to_audio_offset_ms: avg_offset,
                            alignment_confidence: 0.65,
                            matched_event_count: 1,
                            duration_discrepancy_ms: abs_duration_diff_ms,
                            alignment_method: "single_landmark_partial_evidence".to_string(),
                            status: TimelineAlignmentStatus::PartiallyVerified,
                        };
                    }
                }
            }
        }

        // Case C: Duration mismatch WITHOUT matching landmark evidence
        // CRITICAL PRINCIPLE: A duration mismatch is NOT proof of a start offset!
        // Different edits may contain different intros, outros, cuts, or rearranged sections.
        // Therefore, without landmark proof, we NEVER infer an offset from duration difference alone.
        if abs_duration_diff_ms > 15000 {
            TimelineAlignment {
                video_to_audio_offset_ms: 0,
                alignment_confidence: 0.15,
                matched_event_count: 0,
                duration_discrepancy_ms: abs_duration_diff_ms,
                alignment_method: "duration_mismatch_different_edit".to_string(),
                status: TimelineAlignmentStatus::Incompatible,
            }
        } else {
            TimelineAlignment {
                video_to_audio_offset_ms: 0,
                alignment_confidence: 0.30,
                matched_event_count: 0,
                duration_discrepancy_ms: abs_duration_diff_ms,
                alignment_method: "duration_discrepancy_unverified_landmarks".to_string(),
                status: TimelineAlignmentStatus::Unverified,
            }
        }
    }

    /// Fuses audience replay regions with DHUN's musical candidates.
    /// Strictly enforces the acceptance rules:
    /// - ONLY `Verified` alignment may produce executable audio cue positions from audience heatmap timestamps.
    /// - `PartiallyVerified`, `Unverified`, `Incompatible`, and `Unavailable` remain INFORMATIONAL ONLY:
    ///   they preserve video timestamps for UI/audit display, but do NOT produce executable audio cue timestamps.
    /// - Audio-native DHUN hook candidates remain usable in all cases.
    pub fn fuse_candidates_with_audience_heatmap(
        track_id: &str,
        video_id: Option<&str>,
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
        replay_regions: &[ReplayRegion],
        alignment: &TimelineAlignment,
        audio_duration_ms: u64,
    ) -> Vec<HookCandidate> {
        // Step 1: Always retrieve Stage 1 initial musical candidates from DHUN's own analysis
        let mut candidates = AudienceHeatmapAdapter::extract_musical_hook_candidates(
            track_id,
            phrases,
            timing_profile,
            audio_duration_ms,
        );

        // Case A: Missing or unavailable audience heatmap
        if alignment.status == TimelineAlignmentStatus::Unavailable || replay_regions.is_empty() {
            for c in &mut candidates {
                c.alignment_status = TimelineAlignmentStatus::Unavailable;
                c.video_id = video_id.map(|s| s.to_string());
                c.combined_hook_score = AudienceHeatmapAdapter::compute_combined_hook_score(None, c.dhun_musical_score);
            }
            return candidates;
        }

        // Case B: Non-Verified alignment (`PartiallyVerified`, `Unverified`, `Incompatible`)
        // Rule: Informational only! Video timestamps preserved, but NO executable audio cues mapped.
        if alignment.status != TimelineAlignmentStatus::Verified {
            for c in &mut candidates {
                c.alignment_status = alignment.status;
                c.video_id = video_id.map(|s| s.to_string());
                c.combined_hook_score = AudienceHeatmapAdapter::compute_combined_hook_score(None, c.dhun_musical_score);
                c.rejection_reasons.push(format!(
                    "Audience heatmap kept in video time only: alignment status is '{}' (duration discrepancy: {}ms).",
                    alignment.status.as_str(),
                    alignment.duration_discrepancy_ms
                ));
            }

            // Append informational-only audience regions with video coordinates
            for reg in replay_regions {
                let cand_id = format!("{}-info-{}", track_id, reg.start_ms / 1000);
                candidates.push(HookCandidate {
                    id: cand_id,
                    track_id: track_id.to_string(),
                    video_id: video_id.map(|s| s.to_string()),
                    video_start_ms: Some(reg.start_ms),
                    video_end_ms: Some(reg.end_ms),
                    audio_start_ms: 0,
                    audio_end_ms: 0,
                    dhun_musical_score: 0.50,
                    audience_replay_score: Some(reg.peak_intensity),
                    combined_hook_score: AudienceHeatmapAdapter::compute_combined_hook_score(Some(reg.peak_intensity), 0.50),
                    alignment_status: alignment.status,
                    alignment_offset_ms: 0,
                    dj_usability_score: 0.0,
                    is_audience_popular: reg.peak_intensity >= 0.65 || reg.is_youtube_labelled,
                    is_musical_hook: false,
                    is_aligned_hook: false, // NOT aligned: informational only
                    is_usable_entry: false, // NEVER usable as audio entry
                    is_usable_exit: false,  // NEVER usable as audio exit
                    candidate_type: "informational_video_peak".to_string(),
                    entry_cue_ms: 0,
                    exit_cue_ms: 0,
                    usability_reasons: vec![format!(
                        "Informational video replay region [{}ms..{}ms] (alignment '{}' does not produce executable audio cues).",
                        reg.start_ms, reg.end_ms, alignment.status.as_str()
                    )],
                    rejection_reasons: vec![format!(
                        "Non-verified alignment ('{}'): video timestamps cannot be converted to executable audio cues.",
                        alignment.status.as_str()
                    )],
                });
            }

            return candidates;
        }

        // Case C: ONLY `Verified` alignment proceeds to map executable audio coordinates
        let offset = alignment.video_to_audio_offset_ms;

        for reg in replay_regions {
            let mapped_audio_start = reg.start_ms - offset;
            let mapped_audio_end = reg.end_ms - offset;

            // Check if mapped region lies within playable audio duration
            if mapped_audio_end <= 0 || mapped_audio_start >= audio_duration_ms as i64 {
                continue;
            }

            let clamped_audio_start = mapped_audio_start.max(0).min(audio_duration_ms as i64);
            let clamped_audio_end = mapped_audio_end.max(clamped_audio_start + 1000).min(audio_duration_ms as i64);

            // Find matching musical phrase in DHUN's analysis
            let matching_phrase = phrases.iter().find(|p| {
                (p.start_ms <= clamped_audio_start && p.end_ms >= clamped_audio_start)
                    || (p.start_ms >= clamped_audio_start && p.start_ms < clamped_audio_end)
            });

            let dhun_musical_score = if let Some(p) = matching_phrase {
                AudienceHeatmapAdapter::evaluate_dhun_musical_score(p, timing_profile, audio_duration_ms)
            } else {
                0.50
            };

            let audience_score = reg.peak_intensity.max(reg.avg_intensity).clamp(0.0, 1.0);
            let combined_score = AudienceHeatmapAdapter::compute_combined_hook_score(
                Some(audience_score),
                dhun_musical_score,
            );

            // Check if this overlaps with an existing Stage 1 candidate
            let mut merged = false;
            for c in &mut candidates {
                let overlap = (c.audio_start_ms - clamped_audio_start).abs() < 4000
                    || (clamped_audio_start >= c.audio_start_ms && clamped_audio_start < c.audio_end_ms);
                if overlap {
                    c.video_id = video_id.map(|s| s.to_string());
                    c.video_start_ms = Some(reg.start_ms);
                    c.video_end_ms = Some(reg.end_ms);
                    c.audience_replay_score = Some(audience_score);
                    c.combined_hook_score = combined_score;
                    c.alignment_status = TimelineAlignmentStatus::Verified;
                    c.alignment_offset_ms = offset;
                    c.is_aligned_hook = true;
                    if audience_score >= 0.65 || reg.is_youtube_labelled {
                        c.is_audience_popular = true;
                    }
                    c.usability_reasons.push(format!(
                        "Fused audience replay peak ({:.2}) with DHUN musical score ({:.2}) -> Combined: {:.2}.",
                        audience_score, dhun_musical_score, combined_score
                    ));
                    merged = true;
                    break;
                }
            }

            if !merged {
                let cand_id = format!("{}-aud-{}", track_id, reg.start_ms / 1000);
                candidates.push(HookCandidate {
                    id: cand_id,
                    track_id: track_id.to_string(),
                    video_id: video_id.map(|s| s.to_string()),
                    video_start_ms: Some(reg.start_ms),
                    video_end_ms: Some(reg.end_ms),
                    audio_start_ms: clamped_audio_start,
                    audio_end_ms: clamped_audio_end,
                    dhun_musical_score,
                    audience_replay_score: Some(audience_score),
                    combined_hook_score: combined_score,
                    alignment_status: TimelineAlignmentStatus::Verified,
                    alignment_offset_ms: offset,
                    dj_usability_score: 0.0,
                    is_audience_popular: audience_score >= 0.65 || reg.is_youtube_labelled,
                    is_musical_hook: dhun_musical_score >= 0.70,
                    is_aligned_hook: true,
                    is_usable_entry: false,
                    is_usable_exit: false,
                    candidate_type: if reg.is_youtube_labelled { "most_replayed".to_string() } else { "audience_hook".to_string() },
                    entry_cue_ms: clamped_audio_start,
                    exit_cue_ms: clamped_audio_end,
                    usability_reasons: vec![format!(
                        "Verified landmark offset ({}ms) mapped video [{}ms..{}ms] to audio [{}ms..{}ms].",
                        offset, reg.start_ms, reg.end_ms, clamped_audio_start, clamped_audio_end
                    )],
                    rejection_reasons: vec![],
                });
            }
        }

        candidates.sort_by(|a, b| b.combined_hook_score.partial_cmp(&a.combined_hook_score).unwrap());
        candidates
    }
}
