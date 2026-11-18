use crate::db::models::{PlayableTrackCandidate, PlaybackHistoryItem, GuestRequest};
use crate::dj::moods::MoodDefinition;

pub struct TrackSelector;

impl TrackSelector {
    pub fn score_and_select(
        candidates: &[PlayableTrackCandidate],
        currently_playing: Option<&PlayableTrackCandidate>,
        mood: &MoodDefinition,
        secondary_mood: Option<&MoodDefinition>,
        target_energy: f64,
        recent_history: &[PlaybackHistoryItem],
        approved_requests: &[GuestRequest],
        excluded_ids: &[String],
    ) -> Option<PlayableTrackCandidate> {
        if candidates.is_empty() {
            return None;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let mut scored_candidates: Vec<(PlayableTrackCandidate, f64)> = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            // Strictly exclude tracks currently loaded, queued, or reserved
            if excluded_ids.contains(&candidate.track.id) && candidates.len() > excluded_ids.len() {
                continue;
            }

            // Check if track is already in recent history within cooldown
            let track_cooldown_ms = (mood.repetition_cooldown_mins as i64) * 60 * 1000;
            let artist_cooldown_ms = (mood.artist_cooldown_mins as i64) * 60 * 1000;

            let mut was_track_played_recently = false;
            let mut was_artist_played_recently = false;

            for hist in recent_history {
                if hist.track_id == candidate.track.id && (now - hist.played_at) < track_cooldown_ms {
                    was_track_played_recently = true;
                    break;
                }
                if hist.track_artist.to_lowercase() == candidate.track.artist.to_lowercase() && (now - hist.played_at) < artist_cooldown_ms {
                    was_artist_played_recently = true;
                }
            }

            // Exclude recently played track unless all available tracks are in cooldown (never silence the party)
            if was_track_played_recently {
                let unplayed_candidates_exist = candidates
                    .iter()
                    .any(|c| !excluded_ids.contains(&c.track.id) && !recent_history.iter().any(|h| h.track_id == c.track.id && (now - h.played_at).abs() < track_cooldown_ms));
                if unplayed_candidates_exist {
                    continue;
                }
            }

            // 1. Genre Score (0.0 to 1.0) with optional secondary mood blending (70% primary, 30% secondary)
            let mut primary_genre_score = 0.4;
            for (g, weight) in &mood.genre_weights {
                if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                    primary_genre_score = *weight;
                    break;
                }
            }

            let genre_score = match secondary_mood {
                Some(sec) => {
                    let mut sec_genre_score = 0.4;
                    for (g, weight) in &sec.genre_weights {
                        if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                            sec_genre_score = *weight;
                            break;
                        }
                    }
                    (0.70 * primary_genre_score) + (0.30 * sec_genre_score)
                }
                None => primary_genre_score,
            };

            // 2. BPM Compatibility Score
            let effective_min_bpm = match secondary_mood {
                Some(sec) => mood.min_bpm.min(sec.min_bpm),
                None => mood.min_bpm,
            };
            let effective_max_bpm = match secondary_mood {
                Some(sec) => mood.max_bpm.max(sec.max_bpm),
                None => mood.max_bpm,
            };

            let bpm_score = match currently_playing {
                Some(curr) => calculate_bpm_compatibility(curr.analysis.bpm, candidate.analysis.bpm),
                None => {
                    // Check if candidate falls in effective mood BPM range
                    if candidate.analysis.bpm >= effective_min_bpm && candidate.analysis.bpm <= effective_max_bpm {
                        1.0
                    } else {
                        0.5
                    }
                }
            };

            // 3. Key Compatibility Score (Camelot Wheel)
            let key_score = match currently_playing {
                Some(curr) => calculate_camelot_compatibility(&curr.analysis.camelot_key, &candidate.analysis.camelot_key),
                None => 0.8,
            };

            // 4. Energy Compatibility Score with secondary mood blending
            let effective_target_energy = match secondary_mood {
                Some(sec) => (0.70 * target_energy) + (0.30 * sec.target_energy),
                None => target_energy,
            };
            let energy_diff = (candidate.analysis.overall_energy - effective_target_energy).abs();
            let energy_score = (1.0 - energy_diff).clamp(0.1, 1.0);

            // 5. Repetition Penalty
            let repetition_penalty = if was_track_played_recently {
                0.60
            } else if was_artist_played_recently {
                0.35
            } else {
                0.0
            };

            // 6. Guest Request Boost (STRICT: only applies if status is 'approved')
            let matching_approved = approved_requests.iter().find(|r| r.track_id == candidate.track.id && r.status == "approved");
            let request_boost = if let Some(req) = matching_approved {
                if req.priority_order <= 1 {
                    2.5
                } else {
                    1.5
                }
            } else {
                0.0
            };

            // Deterministic Weighted Sum
            let total_score = (genre_score * 0.25)
                + (bpm_score * 0.30)
                + (key_score * 0.25)
                + (energy_score * 0.20)
                - repetition_penalty
                + request_boost;

            let mut c = candidate.clone();
            c.compatibility_score = total_score;
            scored_candidates.push((c, total_score));
        }

        // Sort descending by total score
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_candidates.into_iter().next().map(|(candidate, _)| candidate)
    }
}

pub fn calculate_bpm_compatibility(bpm_a: f64, bpm_b: f64) -> f64 {
    if bpm_a <= 0.0 || bpm_b <= 0.0 {
        return 0.5;
    }

    // Direct comparison
    let diff = (bpm_a - bpm_b).abs();
    let pct_diff = diff / bpm_a;

    if pct_diff <= 0.03 {
        return 1.0; // Within 3%
    } else if pct_diff <= 0.08 {
        return 0.85; // Within 8% (can be tempo-synced easily)
    }

    // Half-time / Double-time check (e.g. 70 bpm with 140 bpm)
    let double_a = bpm_a * 2.0;
    let half_a = bpm_a * 0.5;

    let double_diff = (double_a - bpm_b).abs() / double_a;
    let half_diff = (half_a - bpm_b).abs() / half_a;

    if double_diff <= 0.04 || half_diff <= 0.04 {
        return 0.90; // Harmonic half-time or double-time match!
    }

    (1.0 - pct_diff).clamp(0.1, 0.7)
}

pub fn calculate_camelot_compatibility(key_a: &str, key_b: &str) -> f64 {
    if key_a == key_b {
        return 1.0; // Same key
    }

    // Parse Camelot key format: e.g. "8B", "11A"
    let parse_camelot = |k: &str| -> Option<(i32, char)> {
        let trimmed = k.trim().to_uppercase();
        if trimmed.len() < 2 {
            return None;
        }
        let (num_str, letter) = trimmed.split_at(trimmed.len() - 1);
        let num: i32 = num_str.parse().ok()?;
        let char_val = letter.chars().next()?;
        if (1..=12).contains(&num) && (char_val == 'A' || char_val == 'B') {
            Some((num, char_val))
        } else {
            None
        }
    };

    let (num_a, letter_a) = match parse_camelot(key_a) {
        Some(v) => v,
        None => return 0.5,
    };
    let (num_b, letter_b) = match parse_camelot(key_b) {
        Some(v) => v,
        None => return 0.5,
    };

    if num_a == num_b && letter_a != letter_b {
        return 0.90; // Relative major / minor swap (e.g. 8A to 8B)
    }

    if letter_a == letter_b {
        let diff = (num_a - num_b).abs();
        if diff == 1 || diff == 11 {
            return 0.90; // Adjacent key on wheel (+1 or -1 hour)
        }
        if diff == 2 || diff == 10 {
            return 0.75; // Energy jump (+2 hours)
        }
    }

    0.35 // Clash / chromatic distance
}

/// Evaluates an unplayed private room submission (from either guest or host — zero approval queue)
/// against the active mood, energy progression, current BPM (with half/double-time matching),
/// Camelot harmonic key, and phrase/playability readiness.
pub fn score_local_submission_for_dj(
    sub: &crate::db::models::LocalRoomSubmission,
    primary_mood: &crate::dj::moods::MoodDefinition,
    secondary_mood: Option<&crate::dj::moods::MoodDefinition>,
    current_bpm: Option<f64>,
    current_camelot: Option<&str>,
    target_energy: f64,
) -> f64 {
    let mut score = 0.0;

    // 1. BPM compatibility (with current playing track + mood BPM window)
    let sub_bpm = sub.bpm.unwrap_or((primary_mood.min_bpm + primary_mood.max_bpm) / 2.0);
    let bpm_fit = if let Some(cur) = current_bpm {
        calculate_bpm_compatibility(cur, sub_bpm)
    } else if sub_bpm >= primary_mood.min_bpm && sub_bpm <= primary_mood.max_bpm {
        1.0
    } else {
        0.65
    };
    score += bpm_fit * 30.0;

    // 2. Camelot harmonic key compatibility
    let key_fit = match (current_camelot, sub.camelot_key.as_deref()) {
        (Some(cur_k), Some(sub_k)) => calculate_camelot_compatibility(cur_k, sub_k),
        _ => 0.70,
    };
    score += key_fit * 25.0;

    // 3. Energy curve progression fit
    let sub_energy = sub.overall_energy.unwrap_or(primary_mood.target_energy);
    let energy_fit = (1.0 - (sub_energy - target_energy).abs()).clamp(0.1, 1.0);
    score += energy_fit * 20.0;

    // 4. Mood & genre affinity (primary mood + 30% secondary mood blend)
    let sub_moods: Vec<String> = serde_json::from_str(&sub.mood_tags_json).unwrap_or_default();
    let sub_genres: Vec<String> = serde_json::from_str(&sub.genre_tags_json).unwrap_or_default();
    let mut mood_affinity = if sub_moods.iter().any(|m| m.eq_ignore_ascii_case(&primary_mood.id)) {
        1.0
    } else if primary_mood
        .genre_weights
        .iter()
        .any(|(g, _)| sub_genres.iter().any(|sg| sg.to_lowercase().contains(&g.to_lowercase())))
    {
        0.85
    } else {
        0.55
    };

    if let Some(sec) = secondary_mood {
        let sec_hit = if sub_moods.iter().any(|m| m.eq_ignore_ascii_case(&sec.id)) {
            1.0
        } else {
            0.5
        };
        mood_affinity = (mood_affinity * 0.70) + (sec_hit * 0.30);
    }
    score += mood_affinity * 15.0;

    // 5. Local cache playability readiness bonus (prioritize locally cached authorized audio)
    if sub.acquisition_status == "cached_authorized" {
        score += 10.0;
    }

    (score * 100.0).round() / 100.0
}

/// Computes dynamic energy target based on the selected mood and elapsed party duration
/// (gradual warm-up -> peak celebration -> wave modulation).
pub fn compute_mood_energy_target(
    primary_mood: &crate::dj::moods::MoodDefinition,
    elapsed_mins: f64,
) -> f64 {
    let base = primary_mood.target_energy;
    let progression_offset = if elapsed_mins < 20.0 {
        // Gentle warmup (-0.10 ramping up toward base)
        -0.10 * (1.0 - (elapsed_mins / 20.0))
    } else if elapsed_mins < 90.0 {
        // Peak build (+0.00 to +0.08)
        ((elapsed_mins - 20.0) / 70.0) * 0.08
    } else {
        // Cyclic wave breathing to avoid crowd fatigue
        ((elapsed_mins - 90.0) / 15.0).sin() * 0.06
    };
    (base + progression_offset).clamp(0.35, 1.0)
}

/// Evaluates all queued room submissions and selects the highest-scoring NEXT PLAYABLE candidate
/// WITHOUT ever interrupting an actively playing track, avoiding unnecessary repeats, and
/// strictly refusing to select restricted/unavailable metadata-only tracks as playable.
pub fn select_next_submission_without_interrupting(
    submissions: &[crate::db::models::LocalRoomSubmission],
    primary_mood: &crate::dj::moods::MoodDefinition,
    secondary_mood: Option<&crate::dj::moods::MoodDefinition>,
    target_energy: f64,
) -> (Vec<(String, f64)>, Option<String>) {
    let currently_playing = submissions.iter().find(|s| s.dj_status == "playing");
    let current_bpm = currently_playing.and_then(|s| s.bpm);
    let current_camelot = currently_playing.and_then(|s| s.camelot_key.as_deref());

    let recently_played_videos: std::collections::HashSet<&str> = submissions
        .iter()
        .filter(|s| s.dj_status == "played" || s.dj_status == "playing")
        .map(|s| s.provider_video_id.as_str())
        .collect();

    let mut scored_with_meta: Vec<(String, f64, bool, bool)> = submissions
        .iter()
        .filter(|s| s.dj_status == "queued_for_dj" || s.dj_status == "prepared")
        .map(|s| {
            let mut sc = score_local_submission_for_dj(
                s,
                primary_mood,
                secondary_mood,
                current_bpm,
                current_camelot,
                target_energy,
            );
            let is_repeat = recently_played_videos.contains(s.provider_video_id.as_str());
            if is_repeat {
                sc = (sc - 40.0).max(0.0);
            }
            let is_playable = s.acquisition_status == "cached_authorized"
                && (s.cache_id.is_some() || s.local_cache_path.is_some());
            (s.id.clone(), sc, is_playable, is_repeat)
        })
        .collect();

    scored_with_meta.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Prefer unplayed playable tracks first; never select restricted/unavailable tracks as playable!
    let best_playable_next_id = scored_with_meta
        .iter()
        .find(|(_, _, is_playable, is_repeat)| *is_playable && !*is_repeat)
        .or_else(|| scored_with_meta.iter().find(|(_, _, is_playable, _)| *is_playable))
        .map(|(id, _, _, _)| id.clone());

    let scored: Vec<(String, f64)> = scored_with_meta
        .into_iter()
        .map(|(id, sc, _, _)| (id, sc))
        .collect();

    (scored, best_playable_next_id)
}

/// Explainable DJ recommendation surfaced directly to the host UI.
/// Works even when the catalogue is small or BPM/key analysis is incomplete:
/// missing signals lower confidence and trigger safe transition fallbacks rather than going silent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DjRecommendation {
    pub submission_id: String,
    pub title: String,
    pub artist: String,
    /// `"best_next"` | `"harmonic_match"` | `"energy_builder"` | `"safe_transition"` | `"creative_layer"`
    pub category: String,
    pub category_label: String,
    pub compatibility_score: f64,
    /// 0.0–1.0 confidence based on how much real analysis metadata is known
    pub confidence: f64,
    pub is_playable: bool,
    /// `"playable_now"` | `"processing"` | `"metadata_only_restricted"` | `"failed"`
    pub playability_label: String,
    pub reasons: Vec<String>,
    pub known_signals: Vec<String>,
    pub missing_signals: Vec<String>,
    pub suggested_technique: String,
    pub technique_label: String,
    pub technique_reason: String,
}

pub fn technique_display_label(technique_id: &str) -> &'static str {
    match technique_id {
        "safe_quick_cut" => "Safe Quick Cut",
        "eq3_handover" => "3-Band EQ Handover",
        "phrase_long_blend" => "Phrase-Aligned Long Blend",
        "timed_bass_swap" => "Timed Bass Swap",
        "filter_sweep_drop" => "Resonant Filter Sweep Drop",
        "echo_out_blend" => "Beat-Synced Echo-Out Blend",
        "beat_roll_buildup" => "Beat Roll Build-Up",
        "filtered_buildup_drop" => "Filtered Build-Up & Drop",
        "four_deck_layer" => "4-Deck Groove Layering",
        "stutter_gate_drop" => "Rhythmic Stutter Gate Drop",
        "backspin_cut" => "Vinyl Backspin Brake Cut",
        "echo_freeze_drop" => "Echo Freeze Drop",
        "tempo_ramp_bridge" => "Tempo Ramp Bridge",
        "creative_mashup_layer" => "Creative Dual-Deck Mashup Layer",
        _ => "3-Band EQ Handover",
    }
}

/// Deterministically selects a genuine level-appropriate DJ technique based on:
/// - Active DJ Level (1–4, with strict capability inheritance)
/// - Metadata confidence (falls back to safe cut / EQ handover when BPM or beat grid is missing!)
/// - BPM delta, Camelot harmonic fit, energy jump, and mood style.
pub fn choose_technique_for_pair(
    dj_level: crate::dj::levels::DjLevel,
    current: Option<&crate::db::models::LocalRoomSubmission>,
    candidate: &crate::db::models::LocalRoomSubmission,
    primary_mood: &crate::dj::moods::MoodDefinition,
    confidence: f64,
) -> (String, String) {
    use crate::dj::levels::DjLevel;

    let cur_bpm = current.and_then(|c| c.bpm);
    let cand_bpm = candidate.bpm;
    let bpm_known = cur_bpm.is_some() && cand_bpm.is_some();
    let bpm_pct_diff = match (cur_bpm, cand_bpm) {
        (Some(a), Some(b)) if a > 0.0 => (a - b).abs() / a,
        _ => 1.0,
    };

    let key_compat = match (
        current.and_then(|c| c.camelot_key.as_deref()),
        candidate.camelot_key.as_deref(),
    ) {
        (Some(ka), Some(kb)) => calculate_camelot_compatibility(ka, kb),
        _ => 0.5,
    };

    let energy_delta = match (
        current.and_then(|c| c.overall_energy),
        candidate.overall_energy,
    ) {
        (Some(ea), Some(eb)) => eb - ea,
        _ => 0.0,
    };

    // Strict safety fallback: never pretend accurate beat/phrase sync when BPM or analysis is missing
    if !bpm_known || confidence < 0.45 {
        if dj_level >= DjLevel::Level3AdvancedPerformance {
            return (
                "echo_out_blend".to_string(),
                "Beat grid/BPM unverified — using Level 3 echo-out decay into clean incoming start instead of forced beatmatch.".to_string(),
            );
        }
        return (
            "safe_quick_cut".to_string(),
            "BPM or beat grid not yet verified — safe quick cut avoids off-beat trainwrecks.".to_string(),
        );
    }

    match dj_level {
        DjLevel::Level1TwoDeckMixer => {
            if bpm_pct_diff <= 0.04 {
                (
                    "phrase_long_blend".to_string(),
                    "Level 1: BPMs within 4% — clean 2-deck phrase-aligned long blend.".to_string(),
                )
            } else if bpm_pct_diff <= 0.08 {
                (
                    "eq3_handover".to_string(),
                    "Level 1: Clean 2-deck 3-band EQ handover (high → mid → low).".to_string(),
                )
            } else {
                (
                    "safe_quick_cut".to_string(),
                    "Level 1: Large BPM gap — safe quick cut on phrase boundary.".to_string(),
                )
            }
        }
        DjLevel::Level2SmartMixing => {
            if bpm_pct_diff <= 0.08 && key_compat >= 0.85 {
                (
                    "timed_bass_swap".to_string(),
                    "Level 2: BPM & Camelot key compatible — auto-synced timed bass swap with gain matching.".to_string(),
                )
            } else if primary_mood.transition_style == "filter_sweep" || bpm_pct_diff <= 0.12 {
                (
                    "filter_sweep_drop".to_string(),
                    "Level 2: Smart resonant filter sweep bridging tempo/key change cleanly.".to_string(),
                )
            } else {
                (
                    "safe_quick_cut".to_string(),
                    "Level 2: Wide BPM delta (>12%) — smart fallback to clean quick cut.".to_string(),
                )
            }
        }
        DjLevel::Level3AdvancedPerformance => {
            if energy_delta >= 0.08 {
                (
                    "beat_roll_buildup".to_string(),
                    "Level 3: Rising energy — rhythmic 1/2→1/8 beat roll build-up into incoming drop.".to_string(),
                )
            } else if key_compat >= 0.90 && bpm_pct_diff <= 0.06 {
                (
                    "four_deck_layer".to_string(),
                    "Level 3: Tight harmonic & BPM match — 4-deck filtered groove layering.".to_string(),
                )
            } else if bpm_pct_diff > 0.08 {
                (
                    "echo_out_blend".to_string(),
                    "Level 3: Beat-synced 3/4-beat echo-out tail bridging tempo shift seamlessly.".to_string(),
                )
            } else {
                (
                    "filtered_buildup_drop".to_string(),
                    "Level 3: Resonant HPF tension sweep into synchronized full-bass drop.".to_string(),
                )
            }
        }
        DjLevel::Level4CreativeTools => {
            if key_compat >= 0.90 && bpm_pct_diff <= 0.06 {
                (
                    "creative_mashup_layer".to_string(),
                    "Level 4: Harmonic lock — creative EQ-carved dual-deck mashup layering before bass swap.".to_string(),
                )
            } else if bpm_pct_diff > 0.07 && bpm_pct_diff <= 0.14 {
                (
                    "tempo_ramp_bridge".to_string(),
                    "Level 4: Continuous tempo-ramp bridge smoothly pulling deck BPM into incoming tempo.".to_string(),
                )
            } else if energy_delta >= 0.07 {
                (
                    "stutter_gate_drop".to_string(),
                    "Level 4: Peak energy lift — 16th-note rhythmic stutter gate chopping into incoming drop.".to_string(),
                )
            } else if bpm_pct_diff > 0.14 {
                (
                    "backspin_cut".to_string(),
                    "Level 4: Dramatic genre/tempo switch — vinyl backspin brake cut directly onto downbeat.".to_string(),
                )
            } else {
                (
                    "echo_freeze_drop".to_string(),
                    "Level 4: High-feedback 1/2-beat echo freeze wash releasing into full-energy incoming groove.".to_string(),
                )
            }
        }
    }
}

/// Builds explainable DJ recommendations for the host console, even when the catalogue is small
/// or BPM/key analysis is incomplete. Honestly marks playability vs processing vs metadata-only.
pub fn build_dj_recommendations(
    submissions: &[crate::db::models::LocalRoomSubmission],
    primary_mood: &crate::dj::moods::MoodDefinition,
    secondary_mood: Option<&crate::dj::moods::MoodDefinition>,
    target_energy: f64,
    dj_level: crate::dj::levels::DjLevel,
) -> Vec<DjRecommendation> {
    let currently_playing = submissions.iter().find(|s| s.dj_status == "playing");
    let current_bpm = currently_playing.and_then(|s| s.bpm);
    let current_camelot = currently_playing.and_then(|s| s.camelot_key.as_deref());
    let current_energy = currently_playing.and_then(|s| s.overall_energy);
    let current_artist_lower = currently_playing.map(|s| s.artist.to_lowercase());

    let recently_played_videos: std::collections::HashSet<&str> = submissions
        .iter()
        .filter(|s| s.dj_status == "played" || s.dj_status == "playing")
        .map(|s| s.provider_video_id.as_str())
        .collect();

    let mut recs: Vec<DjRecommendation> = submissions
        .iter()
        .filter(|s| {
            s.dj_status != "playing"
                && s.dj_status != "played"
                && s.dj_status != "skipped_unplayable"
        })
        .map(|s| {
            let mut known = Vec::new();
            let mut missing = Vec::new();
            let mut reasons = Vec::new();

            // Confidence starts from real available metadata (never goes silent when partial!)
            let mut confidence = 0.25_f64;

            if let Some(bpm) = s.bpm {
                known.push(format!("BPM {:.1}", bpm));
                confidence += 0.25;
                if let Some(cbpm) = current_bpm {
                    let compat = calculate_bpm_compatibility(cbpm, bpm);
                    if compat >= 0.90 {
                        reasons.push(format!("Tight tempo fit ({:.0} → {:.0} BPM)", cbpm, bpm));
                    } else if compat >= 0.75 {
                        reasons.push(format!("Mixable tempo shift ({:.0} → {:.0} BPM)", cbpm, bpm));
                    } else {
                        reasons.push(format!("Wide BPM change ({:.0} → {:.0} BPM)", cbpm, bpm));
                    }
                } else if bpm >= primary_mood.min_bpm && bpm <= primary_mood.max_bpm {
                    reasons.push(format!("Inside {} BPM window ({:.0} BPM)", primary_mood.display_name, bpm));
                }
            } else {
                missing.push("BPM not yet verified".to_string());
            }

            let mut key_compat = 0.5;
            if let Some(ref k) = s.camelot_key {
                known.push(format!("Camelot {}", k));
                confidence += 0.20;
                if let Some(ck) = current_camelot {
                    key_compat = calculate_camelot_compatibility(ck, k);
                    if key_compat >= 0.90 {
                        reasons.push(format!("Harmonic Camelot lock ({} → {})", ck, k));
                    } else if key_compat >= 0.75 {
                        reasons.push(format!("Compatible energy key lift ({} → {})", ck, k));
                    }
                }
            } else {
                missing.push("Musical key unknown".to_string());
            }

            if let Some(en) = s.overall_energy {
                known.push(format!("Energy {:.0}%", en * 100.0));
                confidence += 0.15;
                if let Some(ce) = current_energy {
                    if en - ce >= 0.06 {
                        reasons.push("Raises dancefloor energy".to_string());
                    } else if (en - ce).abs() < 0.06 {
                        reasons.push("Maintains steady party groove".to_string());
                    }
                } else if (en - target_energy).abs() <= 0.15 {
                    reasons.push(format!("Fits {} target energy", primary_mood.display_name));
                }
            } else {
                missing.push("Energy profile unanalyzed".to_string());
            }

            if s.duration_ms.unwrap_or(0) > 0 {
                known.push(format!("Duration {}s", s.duration_ms.unwrap_or(0) / 1000));
                confidence += 0.05;
            } else {
                missing.push("Phrase alignment unknown".to_string());
            }

            let is_playable = s.acquisition_status == "cached_authorized"
                && (s.cache_id.is_some() || s.local_cache_path.is_some());
            if is_playable {
                known.push("Cached audio ready".to_string());
                confidence += 0.10;
            } else if s.acquisition_status == "restricted_by_provider" {
                missing.push("No authorized local audio (metadata-only)".to_string());
            } else if matches!(
                s.acquisition_status.as_str(),
                "received" | "metadata_processing" | "awaiting_authorized_audio" | "analysing"
            ) {
                missing.push("Background intake still processing".to_string());
            }

            let playability_label = if is_playable {
                "playable_now"
            } else if matches!(
                s.acquisition_status.as_str(),
                "received" | "metadata_processing" | "awaiting_authorized_audio" | "analysing"
            ) {
                "processing"
            } else if s.acquisition_status == "failed" {
                "failed"
            } else {
                "metadata_only_restricted"
            }
            .to_string();

            let mut score = score_local_submission_for_dj(
                s,
                primary_mood,
                secondary_mood,
                current_bpm,
                current_camelot,
                target_energy,
            );

            // Artist & repeat diversity reasoning
            let is_repeat = recently_played_videos.contains(s.provider_video_id.as_str());
            if is_repeat {
                score = (score - 40.0).max(0.0);
                reasons.push("Already played earlier (deprioritized)".to_string());
            } else if current_artist_lower
                .as_deref()
                .map(|a| !a.is_empty() && a == s.artist.to_lowercase())
                .unwrap_or(false)
            {
                score = (score - 8.0).max(0.0);
                reasons.push("Same artist as current track".to_string());
            } else {
                reasons.push("Fresh artist & track for the crowd".to_string());
            }

            // Ensure at least one clear mood/genre reason when BPM/key are absent
            if s.bpm.is_none() && s.camelot_key.is_none() {
                reasons.insert(
                    0,
                    format!(
                        "Ranked via {} mood/metadata signals while BPM & key are pending",
                        primary_mood.display_name
                    ),
                );
            }

            let clamped_conf = (confidence.clamp(0.15, 1.0) * 100.0).round() / 100.0;
            let (technique_id, technique_reason) = choose_technique_for_pair(
                dj_level,
                currently_playing,
                s,
                primary_mood,
                clamped_conf,
            );

            // Classify recommendation role
            let (category, category_label) = if clamped_conf < 0.45 || technique_id == "safe_quick_cut" {
                ("safe_transition", "Safe Transition")
            } else if key_compat >= 0.90 && s.camelot_key.is_some() {
                if dj_level >= crate::dj::levels::DjLevel::Level4CreativeTools {
                    ("creative_layer", "Creative Mashup Option")
                } else {
                    ("harmonic_match", "Harmonic Option")
                }
            } else if s.overall_energy.unwrap_or(0.0) > current_energy.unwrap_or(target_energy) + 0.05 {
                ("energy_builder", "Energy Builder")
            } else {
                ("best_next", "Best Next Track")
            };

            DjRecommendation {
                submission_id: s.id.clone(),
                title: s.title.clone(),
                artist: s.artist.clone(),
                category: category.to_string(),
                category_label: category_label.to_string(),
                compatibility_score: score,
                confidence: clamped_conf,
                is_playable,
                playability_label,
                reasons,
                known_signals: known,
                missing_signals: missing,
                technique_label: technique_display_label(&technique_id).to_string(),
                suggested_technique: technique_id,
                technique_reason,
            }
        })
        .collect();

    // Sort playable tracks first by score, then non-playable recommendations by score
    recs.sort_by(|a, b| {
        b.is_playable
            .cmp(&a.is_playable)
            .then_with(|| {
                b.compatibility_score
                    .partial_cmp(&a.compatibility_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Label the #1 playable (or top overall if none playable yet) explicitly as "Best Next Track"
    if let Some(top) = recs.first_mut() {
        if top.is_playable {
            top.category = "best_next".to_string();
            top.category_label = "Best Next Track".to_string();
        }
    }

    recs.truncate(6);
    recs
}

