use crate::db::models::{
    HookCandidate, PhraseMarker, TimelineAlignmentStatus, TrackTimingProfile,
};

pub struct HookUsabilityEvaluator;

impl HookUsabilityEvaluator {
    /// Evaluates and enriches all hook candidates with Stage 4 DJ Usability decisions.
    /// Distinguishes:
    /// 1. Audience-popular moment (`is_audience_popular`)
    /// 2. Musically strong hook (`is_musical_hook`)
    /// 3. Verified audio-aligned hook (`is_aligned_hook`)
    /// 4. DJ-usable entry point (`is_usable_entry`)
    /// 5. DJ-usable exit point (`is_usable_exit`)
    pub fn evaluate_candidates(
        candidates: &mut [HookCandidate],
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
        total_duration_ms: u64,
    ) {
        for c in candidates.iter_mut() {
            Self::evaluate_single_candidate(c, timing_profile, phrases, total_duration_ms);
        }
    }

    /// Evaluates a single hook candidate against DHUN's deterministic DJ rules
    pub fn evaluate_single_candidate(
        c: &mut HookCandidate,
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
        total_duration_ms: u64,
    ) {
        // 1. Distinction flags 1, 2, 3
        c.is_audience_popular = c.audience_replay_score.map(|s| s >= 0.65).unwrap_or(false)
            || c.candidate_type == "most_replayed";
        c.is_musical_hook = c.dhun_musical_score >= 0.70
            || matches!(c.candidate_type.as_str(), "chorus" | "main_drop" | "peak");
        // ONLY `Verified` alignment establishes a verified audio-aligned hook:
        c.is_aligned_hook = c.alignment_status == TimelineAlignmentStatus::Verified;

        // 2. Downbeat and bar snapping
        // Find nearest downbeat to candidate raw audio start
        let raw_start = c.audio_start_ms;
        let mut snapped_entry = raw_start;
        let mut found_downbeat = false;

        if !timing_profile.downbeat_timestamps_ms.is_empty() {
            let mut min_diff = i64::MAX;
            let mut closest_db = raw_start;

            for &db in &timing_profile.downbeat_timestamps_ms {
                let diff = (db - raw_start).abs();
                if diff < min_diff {
                    min_diff = diff;
                    closest_db = db;
                }
            }

            // If within 350ms of a downbeat, snap to that exact downbeat
            if min_diff <= 350 {
                snapped_entry = closest_db;
                found_downbeat = true;
            } else {
                // Otherwise find closest preceding downbeat that begins the bar within 1200ms
                if let Some(&prev_db) = timing_profile
                    .downbeat_timestamps_ms
                    .iter()
                    .filter(|&&db| db <= raw_start && (raw_start - db) <= 1200)
                    .last()
                {
                    snapped_entry = prev_db;
                    found_downbeat = true;
                }
            }
        }
        c.entry_cue_ms = snapped_entry.max(0).min(total_duration_ms as i64);

        // 3. Exit cue calculation
        // Safe exit cue allows the entire hook to play through to the end of its phrase
        let raw_end = c.audio_end_ms;
        let mut safe_exit = raw_end;
        if let Some(matching_p) = phrases.iter().find(|p| p.start_ms <= raw_end && p.end_ms >= raw_end) {
            safe_exit = matching_p.end_ms;
        }
        c.exit_cue_ms = safe_exit.max(c.entry_cue_ms + 4000).min(total_duration_ms as i64);

        // 4. DJ Entry Usability Evaluation
        let mut entry_usable = true;
        let mut reasons = Vec::new();
        let mut rejections = Vec::new();

        // Guard A: Dialogue & comedy skit protection
        if timing_profile.dialogue_end_ms > 0 && c.entry_cue_ms < timing_profile.dialogue_end_ms {
            entry_usable = false;
            rejections.push(format!(
                "Candidate start ({}ms) lies inside non-musical dialogue skit (ends at {}ms).",
                c.entry_cue_ms, timing_profile.dialogue_end_ms
            ));
        }

        // Guard B: Vocal Phrase Completeness / No Mid-Word Entry
        // Find phrase containing the candidate raw audio start or entry cue
        let entry_phrase = phrases
            .iter()
            .find(|p| (p.start_ms <= c.entry_cue_ms && p.end_ms > c.entry_cue_ms)
                   || (p.start_ms <= c.audio_start_ms && p.end_ms > c.audio_start_ms));
        if let Some(p) = entry_phrase {
            let offset_from_phrase_start = (c.entry_cue_ms - p.start_ms).abs();
            let raw_offset_from_phrase_start = (c.audio_start_ms - p.start_ms).abs();
            // If the section is vocal-heavy (> 0.45) and we are entering more than 400ms after phrase start,
            // we are cutting right into the middle of a singer's phrase / word!
            if p.vocal_activity > 0.45 && (offset_from_phrase_start > 400 || raw_offset_from_phrase_start > 500) {
                entry_usable = false;
                rejections.push(format!(
                    "Mid-word vocal clash: entry cue ({}ms, raw: {}ms) is into vocal phrase (activity: {:.2}).",
                    c.entry_cue_ms, c.audio_start_ms, p.vocal_activity
                ));
            } else if p.vocal_activity <= 0.40 {
                reasons.push("Clean instrumental or low-vocal entry point.".to_string());
            } else {
                reasons.push("Vocal phrase start aligned to bar 1.".to_string());
            }
        }

        // Guard C: Sufficient track runway
        if total_duration_ms as i64 - c.entry_cue_ms < 8000 {
            entry_usable = false;
            rejections.push("Insufficient remaining duration (< 8s) for entry mix.".to_string());
        }

        // Guard D: Alignment verification requirement
        // ONLY `Verified` alignment may produce executable audio cue positions from audience heatmap timestamps.
        // `PartiallyVerified`, `Unverified`, `Incompatible`, and `Unavailable` remain INFORMATIONAL ONLY:
        // they preserve video timestamps for display, but NEVER execute as audio cues.
        // Audio-native DHUN hook candidates (where audience_replay_score is None) remain 100% usable!
        if c.audience_replay_score.is_some() && c.alignment_status != TimelineAlignmentStatus::Verified {
            entry_usable = false;
            rejections.push(format!(
                "Heatmap cue rejected: timeline alignment status is '{}' (only 'Verified' allows executable cue positions).",
                c.alignment_status.as_str()
            ));
        }

        c.is_usable_entry = entry_usable;

        // 5. DJ Exit Usability Evaluation
        let mut exit_usable = true;

        if c.audience_replay_score.is_some() && c.alignment_status != TimelineAlignmentStatus::Verified {
            exit_usable = false;
        }

        // Guard E: Never exit during a build-up right before its drop payoff!
        let exit_phrase = phrases.iter().find(|p| p.start_ms <= c.exit_cue_ms && p.end_ms >= c.exit_cue_ms);
        if let Some(p) = exit_phrase {
            if p.section_type == "build" {
                exit_usable = false;
                rejections.push("Exit during build-up cuts off drop climax.".to_string());
            }
        }

        // Guard F: Do not mix out prematurely in the first 15 seconds of the song
        if c.exit_cue_ms < 15000 {
            exit_usable = false;
            rejections.push("Exit cue too early in song lifecycle (< 15s).".to_string());
        }

        c.is_usable_exit = exit_usable;

        // 6. Calculate overall DJ Usability Score (0.0 to 1.0)
        let mut usability = 0.50f64;
        if c.is_usable_entry {
            usability += 0.25;
        }
        if c.is_usable_exit {
            usability += 0.15;
        }
        if found_downbeat {
            usability += 0.05;
        }
        if c.is_musical_hook {
            usability += 0.05;
        }
        if !rejections.is_empty() {
            usability -= (rejections.len() as f64 * 0.15).min(0.40);
        }

        c.dj_usability_score = (usability.clamp(0.05, 0.98) * 1000.0).round() / 1000.0;
        c.usability_reasons.extend(reasons);
        c.rejection_reasons.extend(rejections);
    }

    /// Evaluates hook candidates with full Progression Meaningful Region integration.
    /// Distinguishes sustained engagement from transient spikes.
    pub fn evaluate_candidates_with_regions(
        candidates: &mut [HookCandidate],
        timing_profile: &TrackTimingProfile,
        phrases: &[PhraseMarker],
        total_duration_ms: u64,
        meaningful_regions: &[crate::dj::intelligence::MeaningfulRegion],
    ) {
        Self::evaluate_candidates(candidates, timing_profile, phrases, total_duration_ms);

        for c in candidates.iter_mut() {
            let c_start = c.audio_start_ms;
            let c_end = c.audio_end_ms;
            for r in meaningful_regions {
                if r.onset_ms <= c_end && r.offset_ms >= c_start {
                    if r.sustained_duration_ms >= 1000 && r.integral_mass >= 0.20 {
                        c.usability_reasons.push(format!(
                            "Audience Intelligence: Verified sustained replay progression (mass: {:.2}, sustained: {}ms)",
                            r.integral_mass, r.sustained_duration_ms
                        ));
                        c.dj_usability_score = (c.dj_usability_score + (r.integral_mass / (1.0 + r.integral_mass) * 0.08)).clamp(0.05, 0.98);
                    } else if r.duration_ms < 500 && r.integral_mass < 0.10 {
                        c.rejection_reasons.push("Audience Intelligence: Transient replay spike without sustained engagement".to_string());
                        c.dj_usability_score = (c.dj_usability_score - 0.05).clamp(0.05, 0.98);
                    }
                }
            }
        }
    }
}
