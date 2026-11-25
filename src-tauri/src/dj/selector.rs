use crate::db::models::{GuestRequest, LocalRoomSubmission, PlayableTrackCandidate, PlaybackHistoryItem};
use crate::dj::levels::DjLevel;
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
            let mut primary_genre_score = 0.35;
            for (g, weight) in &mood.genre_weights {
                if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                    primary_genre_score = if *weight >= 0.40 {
                        1.0
                    } else if *weight >= 0.25 {
                        0.90
                    } else {
                        0.80
                    };
                    break;
                }
            }

            let genre_score = match secondary_mood {
                Some(sec) => {
                    let mut sec_genre_score = 0.35;
                    for (g, weight) in &sec.genre_weights {
                        if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                            sec_genre_score = if *weight >= 0.40 {
                                1.0
                            } else if *weight >= 0.25 {
                                0.90
                            } else {
                                0.80
                            };
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
            let energy_score = if energy_diff <= 0.08 {
                1.0
            } else if energy_diff <= 0.15 {
                0.95
            } else if energy_diff <= 0.22 {
                0.85
            } else {
                (1.0 - energy_diff * 1.5).clamp(0.05, 0.75)
            };

            // 5. Repetition Penalty
            let repetition_penalty = if was_track_played_recently {
                0.60
            } else if was_artist_played_recently {
                0.35
            } else {
                0.0
            };

            // 6. Timbre & Texture clash prevention (prevents acoustic ballad vs high-energy EDM shock)
            let mut timbre_penalty = 0.0;
            let mut contrast_bonus = 0.0;
            if let Some(curr) = currently_playing {
                let curr_is_dance = curr.track.genre.to_lowercase().contains("dance")
                    || curr.track.genre.to_lowercase().contains("club")
                    || curr.track.genre.to_lowercase().contains("house")
                    || curr.track.genre.to_lowercase().contains("edm");
                let cand_is_acoustic = candidate.track.genre.to_lowercase().contains("acoustic")
                    || candidate.track.genre.to_lowercase().contains("ghazal")
                    || candidate.track.genre.to_lowercase().contains("ballad")
                    || candidate.analysis.overall_energy < 0.35;
                if curr_is_dance && cand_is_acoustic && (curr.analysis.overall_energy - candidate.analysis.overall_energy) > 0.35 {
                    timbre_penalty = 0.30;
                }

                // Exciting genre contrast bonus when harmonic key and energy match
                if key_score >= 0.75
                    && candidate.analysis.overall_energy >= 0.70
                    && curr.analysis.overall_energy >= 0.70
                    && !candidate.track.genre.eq_ignore_ascii_case(&curr.track.genre)
                {
                    contrast_bonus = 0.08;
                }
            }

            // 7. Guest Request Boost (STRICT: only applies if status is 'approved' AND passes musical viability)
            // Essential musical compatibility guardrail: approved requests cannot force severe timbre/tempo clashes.
            let matching_approved = approved_requests.iter().find(|r| r.track_id == candidate.track.id && r.status == "approved");
            let request_boost = if let Some(req) = matching_approved {
                let is_extreme_clash = timbre_penalty > 0.0 || bpm_score < 0.35 || energy_score < 0.20;
                if is_extreme_clash {
                    0.0 // Request cannot overwhelm essential musical compatibility!
                } else if req.priority_order <= 1 {
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
                - timbre_penalty
                + contrast_bonus
                + request_boost;

            let mut c = candidate.clone();
            c.compatibility_score = total_score;
            scored_candidates.push((c, total_score));
        }

        // Sort descending by total score
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_candidates.into_iter().next().map(|(candidate, _)| candidate)
    }

    /// Selects next track using multi-criteria robust prioritization under uncertainty.
    /// Balances genre compatibility, BPM, key harmony, energy trajectory, and request priority
    /// while penalizing high uncertainty and respecting hard safety constraints.
    pub fn score_and_select_robust(
        candidates: &[PlayableTrackCandidate],
        currently_playing: Option<&PlayableTrackCandidate>,
        mood: &MoodDefinition,
        secondary_mood: Option<&MoodDefinition>,
        target_energy: f64,
        recent_history: &[PlaybackHistoryItem],
        approved_requests: &[GuestRequest],
        excluded_ids: &[String],
        storyteller: Option<&crate::dj::energy::SetStoryteller>,
        prioritizer_config: &crate::dj::intelligence::PrioritizerConfig,
    ) -> Option<PlayableTrackCandidate> {
        if candidates.is_empty() {
            return None;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let track_cooldown_ms = (mood.repetition_cooldown_mins as i64) * 60 * 1000;
        let artist_cooldown_ms = (mood.artist_cooldown_mins as i64) * 60 * 1000;

        let mut candidate_inputs: Vec<(PlayableTrackCandidate, Vec<crate::dj::intelligence::DecisionCriterion>)> =
            Vec::with_capacity(candidates.len());

        for candidate in candidates {
            let mut criteria = Vec::new();

            // 1. Hard constraint: Excluded IDs
            let is_excluded = excluded_ids.contains(&candidate.track.id) && candidates.len() > excluded_ids.len();
            criteria.push(crate::dj::intelligence::DecisionCriterion::hard(
                "NotExcluded",
                if is_excluded { 0.0 } else { 1.0 },
                1.0,
            ));

            // 2. Cooldown check
            let mut was_track_played_recently = false;
            let mut was_artist_played_recently = false;

            for hist in recent_history {
                if hist.track_id == candidate.track.id && (now - hist.played_at) < track_cooldown_ms {
                    was_track_played_recently = true;
                    break;
                }
                if hist.track_artist.to_lowercase() == candidate.track.artist.to_lowercase()
                    && (now - hist.played_at) < artist_cooldown_ms
                {
                    was_artist_played_recently = true;
                }
            }

            let unplayed_candidates_exist = candidates.iter().any(|c| {
                !excluded_ids.contains(&c.track.id)
                    && !recent_history
                        .iter()
                        .any(|h| h.track_id == c.track.id && (now - h.played_at).abs() < track_cooldown_ms)
            });

            if was_track_played_recently && unplayed_candidates_exist {
                criteria.push(crate::dj::intelligence::DecisionCriterion::hard("Cooldown", 0.0, 1.0));
            } else {
                criteria.push(crate::dj::intelligence::DecisionCriterion::hard("Cooldown", 1.0, 1.0));
            }

            // Artist repetition soft criterion
            let artist_score = if was_artist_played_recently { 0.70 } else { 1.0 };
            criteria.push(crate::dj::intelligence::DecisionCriterion::soft("ArtistVariety", artist_score, 0.10, 0.0));

            // Soft criteria:
            // A. Genre Compatibility
            let mut primary_genre_score = 0.10;
            for (g, weight) in &mood.genre_weights {
                if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                    primary_genre_score = if *weight >= 0.40 {
                        1.0
                    } else if *weight >= 0.25 {
                        0.90
                    } else {
                        0.80
                    };
                    break;
                }
            }
            let genre_score = match secondary_mood {
                Some(sec) => {
                    let mut sec_genre_score = 0.10;
                    for (g, weight) in &sec.genre_weights {
                        if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                            sec_genre_score = if *weight >= 0.40 {
                                1.0
                            } else if *weight >= 0.25 {
                                0.90
                            } else {
                                0.80
                            };
                            break;
                        }
                    }
                    (0.70 * primary_genre_score) + (0.30 * sec_genre_score)
                }
                None => primary_genre_score,
            };
            criteria.push(crate::dj::intelligence::DecisionCriterion::soft("Genre", genre_score, 0.25, 0.05));

            // B. BPM Transition Compatibility
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
                    if candidate.analysis.bpm >= effective_min_bpm && candidate.analysis.bpm <= effective_max_bpm {
                        1.0
                    } else {
                        0.5
                    }
                }
            };
            let bpm_uncertainty = (1.0 - candidate.analysis.bpm_confidence).clamp(0.05, 0.50);
            criteria.push(crate::dj::intelligence::DecisionCriterion::soft("BpmCompatibility", bpm_score, 0.25, bpm_uncertainty));

            // C. Key Compatibility (Camelot)
            let key_score = match currently_playing {
                Some(curr) => calculate_camelot_compatibility(&curr.analysis.camelot_key, &candidate.analysis.camelot_key),
                None => 0.8,
            };
            let key_uncertainty = (1.0 - candidate.analysis.key_confidence).clamp(0.05, 0.50);
            criteria.push(crate::dj::intelligence::DecisionCriterion::soft("KeyHarmony", key_score, 0.25, key_uncertainty));

            // D. Energy Fit / Narrative Trajectory
            let (energy_score, energy_uncertainty) = if let Some(st) = storyteller {
                let profile = crate::dj::energy::SectionEnergyProfile::from_phrase_markers(
                    &[],
                    candidate.analysis.overall_energy,
                );
                let (raw_score, _, _) = st.evaluate_candidate_energy_fit(&profile, candidate.analysis.overall_energy);
                let normalized = (raw_score / 30.0).clamp(0.0, 1.0);
                (normalized, 0.10)
            } else {
                let effective_target = match secondary_mood {
                    Some(sec) => (0.70 * target_energy) + (0.30 * sec.target_energy),
                    None => target_energy,
                };
                let diff = (candidate.analysis.overall_energy - effective_target).abs();
                ((1.0 - diff).clamp(0.1, 1.0), 0.15)
            };
            criteria.push(crate::dj::intelligence::DecisionCriterion::soft("EnergyNarrative", energy_score, 0.25, energy_uncertainty));

            // E. Request Priority (standardized across candidates when party requests are active)
            // Musical compatibility guardrail: approved request cannot cause extreme BPM/energy clash
            if !approved_requests.is_empty() {
                let is_requested = approved_requests.iter().any(|r| r.track_id == candidate.track.id && r.status == "approved");
                let is_extreme_clash = bpm_score < 0.35 || energy_score < 0.20;
                let request_score = if is_requested && !is_extreme_clash { 1.0 } else { 0.0 };
                criteria.push(crate::dj::intelligence::DecisionCriterion::soft("GuestRequest", request_score, 0.20, 0.0));
            }

            candidate_inputs.push((candidate.clone(), criteria));
        }

        let evaluated = crate::dj::intelligence::rank_candidates(candidate_inputs, prioritizer_config);

        evaluated
            .into_iter()
            .find(|c| c.is_eligible)
            .map(|mut c| {
                c.payload.compatibility_score = c.robust_score;
                c.payload
            })
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

    let diff = (num_a - num_b).abs();
    if letter_a == letter_b {
        if diff == 1 || diff == 11 {
            return 0.90; // Adjacent key on wheel (+1 or -1 hour)
        }
        if diff == 2 || diff == 10 {
            return 0.75; // Energy jump (+2 hours)
        }
    } else {
        // Diagonal modulation (+1 or -1 hour with mode swap, e.g. 8A -> 9B or 8A -> 7B)
        // Represents modulation to dominant/subdominant of relative key
        if diff == 1 || diff == 11 {
            return 0.70; // Diagonal harmonic modulation
        }
    }

    0.35 // Clash / chromatic distance
}

#[derive(Debug, Clone)]
pub struct TrackMusicalProfile {
    pub bpm: f64,
    pub camelot_key: Option<String>,
    pub energy: f64,
    pub danceability: f64,
    pub vocal_density: f64,
    pub is_acoustic: bool,
    pub is_electronic_dance: bool,
    pub is_percussion_heavy: bool,
    pub meter_label: String,
    pub rhythmic_pattern_detected: String,
    pub rhythm_confidence: f64,
    pub syncopation_index: f64,
}

pub fn extract_musical_profile(
    sub: &LocalRoomSubmission,
    fallback_bpm: f64,
    fallback_energy: f64,
) -> TrackMusicalProfile {
    let bpm = sub.bpm.unwrap_or(fallback_bpm);
    let energy = sub.overall_energy.unwrap_or(fallback_energy);
    let danceability = sub.danceability.unwrap_or(0.70);
    let sub_genres: Vec<String> = serde_json::from_str(&sub.genre_tags_json).unwrap_or_default();
    let sub_moods: Vec<String> = serde_json::from_str(&sub.mood_tags_json).unwrap_or_default();
    let title_lower = sub.title.to_lowercase();

    let is_acoustic = sub_genres.iter().any(|g| {
        let gl = g.to_lowercase();
        gl.contains("acoustic")
            || gl.contains("ghazal")
            || gl.contains("sufi")
            || gl.contains("ballad")
            || gl.contains("classical")
            || gl.contains("piano")
            || gl.contains("unplugged")
    }) || title_lower.contains("unplugged")
        || title_lower.contains("acoustic")
        || title_lower.contains("reprise")
        || energy < 0.38;

    let is_electronic_dance = sub_genres.iter().any(|g| {
        let gl = g.to_lowercase();
        gl.contains("edm")
            || gl.contains("house")
            || gl.contains("techno")
            || gl.contains("club")
            || gl.contains("dance")
            || gl.contains("electro")
    }) || sub_moods
        .iter()
        .any(|m| m == "desi_club" || m == "house_party" || m == "festival_mode")
        || (energy >= 0.78 && danceability >= 0.75);

    let is_percussion_heavy = sub_genres.iter().any(|g| {
        let gl = g.to_lowercase();
        gl.contains("bhangra")
            || gl.contains("dhol")
            || gl.contains("punjabi")
            || gl.contains("garba")
            || gl.contains("dandiya")
    }) || sub_moods
        .iter()
        .any(|m| m == "baraat" || m == "garba" || m == "dandiya" || m == "punjabi_tashan");

    let is_instrumental = sub_genres.iter().any(|g| {
        let gl = g.to_lowercase();
        gl.contains("instrumental") || gl.contains("dub") || gl.contains("groove") || gl.contains("beat")
    }) || title_lower.contains("instrumental")
        || title_lower.contains("dub mix")
        || title_lower.contains("club mix");

    let vocal_density = if is_instrumental {
        0.12
    } else if is_acoustic {
        0.78
    } else if is_percussion_heavy {
        0.50
    } else if is_electronic_dance {
        0.35
    } else {
        0.65
    };

    let meter_label = if sub_moods.iter().any(|m| m == "garba" || m == "dandiya")
        || sub_genres.iter().any(|g| g.to_lowercase().contains("garba"))
    {
        "3/4".to_string()
    } else {
        "4/4".to_string()
    };

    let (rhythmic_pattern_detected, rhythm_confidence, syncopation_index) = {
        let has_garba = sub_genres.iter().any(|g| {
            let gl = g.to_lowercase();
            gl.contains("garba") || gl.contains("taali") || gl.contains("sanedo") || gl.contains("gujarati")
        }) || title_lower.contains("garba") || title_lower.contains("taali");

        let has_dandiya = sub_genres.iter().any(|g| {
            let gl = g.to_lowercase();
            gl.contains("dandiya") || gl.contains("raas")
        }) || title_lower.contains("dandiya");

        let has_dhol = sub_genres.iter().any(|g| {
            let gl = g.to_lowercase();
            gl.contains("dhol") || gl.contains("bhangra") || gl.contains("punjabi")
        }) || title_lower.contains("dhol") || title_lower.contains("bhangra");

        let is_four_on_floor = is_electronic_dance && (meter_label == "4/4" || meter_label == "uncertain");

        if has_garba && (meter_label == "3/4" || meter_label == "6/8") {
            ("authentic_3_taali".to_string(), 0.88, 0.45)
        } else if has_garba && is_four_on_floor {
            ("four_on_floor".to_string(), 0.82, 0.15)
        } else if has_dandiya && bpm >= 122.0 && bpm <= 146.0 {
            ("sharp_dandiya_sticks".to_string(), 0.85, 0.40)
        } else if has_dhol {
            ("live_punjabi_dhol".to_string(), 0.90, 0.55)
        } else if is_four_on_floor {
            ("four_on_floor".to_string(), 0.85, 0.20)
        } else if is_percussion_heavy {
            ("syncopated_dembow".to_string(), 0.75, 0.50)
        } else {
            ("standard_pop_groove".to_string(), 0.70, 0.25)
        }
    };

    TrackMusicalProfile {
        bpm,
        camelot_key: sub.camelot_key.clone(),
        energy,
        danceability,
        vocal_density,
        is_acoustic,
        is_electronic_dance,
        is_percussion_heavy,
        meter_label,
        rhythmic_pattern_detected,
        rhythm_confidence,
        syncopation_index,
    }
}

#[derive(Debug, Clone)]
pub struct DetailedDjEvaluation {
    pub total_score: f64,
    pub harmonic_score: f64,
    pub bpm_score: f64,
    pub energy_score: f64,
    pub predicted_energy_direction: String, // "boost" | "sustain" | "relief"
    pub groove_and_meter_score: f64,
    pub vocal_score: f64,
    pub timbre_score: f64,
    pub sequence_lookahead_score: f64,
    pub is_exciting_contrast: bool,
    pub reasons: Vec<String>,
    pub penalties: Vec<String>,
}

/// Evaluates a song submission with deep DJ intelligence:
/// - Harmonic Camelot key relationship
/// - Decoupled BPM and half/double-time groove
/// - Energy trajectory and party progression
/// - Timbre and texture compatibility (prevents acoustic ballad vs EDM shock)
/// - Vocal density and clash avoidance
/// - Exciting contrast vs jarring shock
/// - Multi-track set lookahead sequence planning
pub fn evaluate_submission_dj_intelligence(
    candidate: &LocalRoomSubmission,
    currently_playing: Option<&LocalRoomSubmission>,
    primary_mood: &MoodDefinition,
    secondary_mood: Option<&MoodDefinition>,
    target_energy: f64,
    unplayed_queue: &[LocalRoomSubmission],
) -> DetailedDjEvaluation {
    let mut reasons = Vec::new();
    let mut penalties = Vec::new();

    let cand_prof = extract_musical_profile(
        candidate,
        (primary_mood.min_bpm + primary_mood.max_bpm) / 2.0,
        primary_mood.target_energy,
    );

    let cur_prof = currently_playing.map(|c| {
        extract_musical_profile(
            c,
            (primary_mood.min_bpm + primary_mood.max_bpm) / 2.0,
            primary_mood.target_energy,
        )
    });

    // 1. Harmonic Key Compatibility (25 pts max)
    let (key_compat, harmonic_score) = match (&cur_prof, &cand_prof.camelot_key) {
        (Some(cur), Some(cand_k)) if cur.camelot_key.is_some() => {
            let cur_k = cur.camelot_key.as_ref().unwrap();
            let compat = calculate_camelot_compatibility(cur_k, cand_k);
            if compat >= 0.98 {
                reasons.push(format!("Harmonic Camelot lock ({} → {})", cur_k, cand_k));
                (compat, 25.0)
            } else if compat >= 0.88 {
                reasons.push(format!("Smooth harmonic Camelot pivot ({} → {})", cur_k, cand_k));
                (compat, 23.0)
            } else if compat >= 0.74 {
                reasons.push(format!("Energizing key lift (+2 hours: {} → {})", cur_k, cand_k));
                (compat, 20.0)
            } else {
                reasons.push(format!("Incompatible harmonic key ({} → {}) — tonal clash risk", cur_k, cand_k));
                (compat, 7.5)
            }
        }
        _ => (0.70, 17.5),
    };

    // 2. BPM & Groove Relationship (20 pts max)
    let (bpm_compat, mut bpm_score) = match &cur_prof {
        Some(cur) => {
            let compat = calculate_bpm_compatibility(cur.bpm, cand_prof.bpm);
            let pct = (cur.bpm - cand_prof.bpm).abs() / cur.bpm.max(1.0);
            let double_pct = (cur.bpm * 2.0 - cand_prof.bpm).abs() / (cur.bpm * 2.0);
            let half_pct = (cur.bpm * 0.5 - cand_prof.bpm).abs() / (cur.bpm * 0.5);

            if double_pct <= 0.04 {
                reasons.push(format!("Harmonic double-time groove lift ({:.0} → {:.0} BPM)", cur.bpm, cand_prof.bpm));
                (0.92, 18.5)
            } else if half_pct <= 0.04 {
                reasons.push(format!("Harmonic half-time heavy groove transition ({:.0} → {:.0} BPM)", cur.bpm, cand_prof.bpm));
                (0.92, 18.5)
            } else if pct <= 0.03 {
                reasons.push(format!("Tight tempo lock ({:.0} → {:.0} BPM)", cur.bpm, cand_prof.bpm));
                (1.0, 20.0)
            } else if pct <= 0.08 {
                reasons.push(format!("Mixable tempo step ({:.0} → {:.0} BPM)", cur.bpm, cand_prof.bpm));
                (0.85, 17.0)
            } else {
                reasons.push(format!("Wide tempo shift ({:.0} → {:.0} BPM)", cur.bpm, cand_prof.bpm));
                (compat, 9.0)
            }
        }
        None => {
            if cand_prof.bpm >= primary_mood.min_bpm && cand_prof.bpm <= primary_mood.max_bpm {
                reasons.push(format!("Inside {} BPM zone ({:.0} BPM)", primary_mood.display_name, cand_prof.bpm));
                (1.0, 20.0)
            } else {
                (0.65, 13.0)
            }
        }
    };

    // Mood-specific BPM flexibility and tempo relationship preferences
    if let Some(ref cur) = cur_prof {
        let double_pct = (cur.bpm * 2.0 - cand_prof.bpm).abs() / (cur.bpm * 2.0);
        let half_pct = (cur.bpm * 0.5 - cand_prof.bpm).abs() / (cur.bpm * 0.5);
        if primary_mood.acceptable_tempo_relationships.iter().any(|r| r == "half_time" || r == "double_time") {
            if half_pct <= 0.05 || double_pct <= 0.05 {
                reasons.push(format!("{} actively embraces harmonic tempo relationship ({:.0} → {:.0} BPM)", primary_mood.display_name, cur.bpm, cand_prof.bpm));
                bpm_score = 20.0;
            }
        }
        if primary_mood.bpm_flexibility == "wide_multi_tempo" && cand_prof.bpm >= primary_mood.min_bpm && cand_prof.bpm <= primary_mood.max_bpm {
            if bpm_score < 14.0 {
                bpm_score = 14.0;
                reasons.push(format!("Multi-tempo mood {} supports versatile tempo shift ({:.0} → {:.0} BPM)", primary_mood.display_name, cur.bpm, cand_prof.bpm));
            }
        }
    }
    if cand_prof.bpm >= primary_mood.min_bpm && cand_prof.bpm <= primary_mood.max_bpm {
        reasons.push(format!("In the pocket for {} BPM sweet spot ({:.0} BPM)", primary_mood.display_name, cand_prof.bpm));
    } else if (cand_prof.bpm < primary_mood.min_bpm - 8.0 || cand_prof.bpm > primary_mood.max_bpm + 8.0) && primary_mood.bpm_flexibility == "tight" {
        penalties.push(format!("Tempo ({:.0} BPM) outside tight {} window ({:.0}-{:.0} BPM)", cand_prof.bpm, primary_mood.display_name, primary_mood.min_bpm, primary_mood.max_bpm));
    }

    // Groove & Meter compatibility
    let mut groove_and_meter_score = 10.0;
    if let Some(ref cur) = cur_prof {
        if cur.meter_label != cand_prof.meter_label {
            penalties.push(format!(
                "Meter shift ({} vs {}): rhythmic groove bridge required",
                cur.meter_label, cand_prof.meter_label
            ));
            groove_and_meter_score = 3.5;
        } else if cand_prof.is_percussion_heavy && cur.is_percussion_heavy {
            reasons.push("Matching rhythmic percussion intensity".to_string());
            groove_and_meter_score = 10.0;
        }
    }

    // 3. Energy Trajectory & Dancefloor Progression (20 pts max)
    let cur_energy = cur_prof.as_ref().map(|c| c.energy).unwrap_or(primary_mood.target_energy);
    let delta_energy = cand_prof.energy - cur_energy;

    let predicted_energy_direction = if delta_energy >= 0.07 {
        "boost".to_string()
    } else if delta_energy <= -0.10 {
        "relief".to_string()
    } else {
        "sustain".to_string()
    };

    let energy_score = if delta_energy >= 0.07 {
        if cur_energy < target_energy {
            reasons.push(format!("Lifts party energy toward target ({:.0}% → {:.0}%)", cur_energy * 100.0, cand_prof.energy * 100.0));
            20.0
        } else {
            reasons.push("Elevates party peak energy".to_string());
            18.0
        }
    } else if delta_energy <= -0.10 {
        if (cand_prof.energy - target_energy).abs() <= 0.08 {
            reasons.push(format!("Brings dancefloor back to {} target energy ({:.0}%)", primary_mood.display_name, target_energy * 100.0));
            20.0
        } else if cur_energy >= 0.82 && delta_energy >= -0.22 {
            reasons.push("Intentional crowd breathing room before next build".to_string());
            17.5
        } else if delta_energy <= -0.15 {
            reasons.push(format!("Conflicting energy drop ({:.0}% → {:.0}%)", cur_energy * 100.0, cand_prof.energy * 100.0));
            if cur_energy >= 0.78 && cand_prof.energy <= 0.40 {
                penalties.push(format!("Abrupt energy collapse ({:.0}% → {:.0}%) risks clearing dancefloor", cur_energy * 100.0, cand_prof.energy * 100.0));
                3.0
            } else {
                8.0
            }
        } else {
            reasons.push(format!("Gentle energy relief ({:.0}% → {:.0}%)", cur_energy * 100.0, cand_prof.energy * 100.0));
            12.0
        }
    } else {
        reasons.push("Maintains steady dancefloor momentum".to_string());
        20.0
    };

    // 4. Timbre & Vocal Balance (20 pts max)
    let mut timbre_score = 10.0;
    let mut vocal_score = 8.0;
    if let Some(ref cur) = cur_prof {
        // Timbre Shock Prevention: Acoustic vs Electronic Dance
        if cur.is_electronic_dance && cand_prof.is_acoustic && delta_energy <= -0.30 {
            penalties.push("Jarring texture clash: heavy electronic club groove into acoustic ballad".to_string());
            timbre_score = 0.0;
        } else if cur.is_acoustic && cand_prof.is_electronic_dance && delta_energy >= 0.35 {
            penalties.push("Unprepared texture shock: acoustic song straight into heavy electronic beats".to_string());
            timbre_score = 2.0;
        }

        // Vocal Density Interaction
        if cur.vocal_density >= 0.60 && cand_prof.vocal_density >= 0.60 {
            penalties.push("Consecutive high-density vocals risk crowd lyrical exhaustion".to_string());
            vocal_score = 2.0;
        } else if cur.vocal_density > 0.55 && cand_prof.vocal_density <= 0.30 {
            reasons.push("Instrumental groove provides crowd breathing room after vocal singalong".to_string());
            vocal_score = 10.0;
        } else if cur.vocal_density <= 0.30 && cand_prof.vocal_density > 0.60 {
            reasons.push("Anticipated vocal hook drops seamlessly after instrumental groove".to_string());
            vocal_score = 10.0;
        }
    }

    // 5. Genre Affinity, Granular Subgenre Weights & Exciting Contrast
    let sub_genres: Vec<String> = serde_json::from_str(&candidate.genre_tags_json).unwrap_or_default();
    let sub_moods: Vec<String> = serde_json::from_str(&candidate.mood_tags_json).unwrap_or_default();
    let genre_match = primary_mood.genre_weights.iter().any(|(g, _)| {
        sub_genres.iter().any(|sg| sg.to_lowercase().contains(&g.to_lowercase()))
    });

    let mut is_exciting_contrast = false;
    let mut genre_score = if genre_match || sub_moods.iter().any(|m| m.eq_ignore_ascii_case(&primary_mood.id)) {
        reasons.push(format!("Genre fits {} mood palette", primary_mood.display_name));
        8.0
    } else {
        let genre_name = sub_genres.first().map(|s| s.as_str()).unwrap_or("Different");
        reasons.push(format!("Cross-genre shift ({} vs {} mood)", genre_name, primary_mood.display_name));
        0.0
    };

    let all_cand_genres = sub_genres.join(" ").to_lowercase();

    // Granular Subgenre Scoring from mood definition
    let mut subgenre_match_score = 0.0;
    for (sg, weight) in &primary_mood.subgenre_weights {
        if all_cand_genres.contains(&sg.to_lowercase()) {
            reasons.push(format!("Signature subgenre '{}' active for {}", sg, primary_mood.display_name));
            subgenre_match_score += weight * 2.0;
            break;
        }
    }

    // Recognize when contrast is exciting and acceptable under mood contrast_tolerance
    if let Some(ref cur) = cur_prof {
        if !genre_match
            && key_compat >= 0.75
            && cand_prof.energy >= 0.72
            && cur.energy >= 0.70
            && cand_prof.danceability >= 0.70
            && primary_mood.contrast_tolerance >= 0.40
        {
            is_exciting_contrast = true;
            reasons.push(format!("Exciting genre contrast: energetic switch elevates {}", primary_mood.display_name));
            genre_score += 3.0;
        }
    }

    // 5b. Mood Percussion, Atmosphere & Vocal Preference Scoring
    let mut mood_affinity_score = subgenre_match_score;

    match primary_mood.preferred_percussion.as_str() {
        "live_punjabi_dhol" => {
            if cand_prof.rhythmic_pattern_detected == "live_punjabi_dhol" {
                reasons.push("Authentic live Punjabi dhol (dagga + tilli) verified by DSP analysis".to_string());
                mood_affinity_score += 6.0;
            } else if cand_prof.is_percussion_heavy || all_cand_genres.contains("dhol") || all_cand_genres.contains("bhangra") {
                reasons.push("Acoustic live Punjabi dhol driving groove".to_string());
                mood_affinity_score += 4.5;
            }
        }
        "cyclic_3_taali" => {
            let has_garba_text = all_cand_genres.contains("garba") || all_cand_genres.contains("taali") || all_cand_genres.contains("sanedo") || all_cand_genres.contains("gujarati");
            let is_dsp_triple_meter = cand_prof.meter_label == "3/4" || cand_prof.meter_label == "6/8";
            if cand_prof.rhythmic_pattern_detected == "authentic_3_taali" || (has_garba_text && is_dsp_triple_meter) {
                reasons.push("Authentic 3-Taali cyclic folk meter (3/4 or 6/8) verified by audio analysis".to_string());
                mood_affinity_score += 6.5;
            } else if cand_prof.rhythmic_pattern_detected == "four_on_floor" || (cand_prof.is_electronic_dance && cand_prof.meter_label == "4/4") {
                penalties.push("Rigid 4/4 electronic kick overrides authentic 3-Taali cyclic stepping".to_string());
                mood_affinity_score += 1.0;
            } else if has_garba_text && !cand_prof.is_electronic_dance {
                reasons.push("Acoustic Gujarati folk cadence aligns with Garba circular stepping".to_string());
                mood_affinity_score += 5.0;
            } else if has_garba_text {
                reasons.push("Authentic 3-Taali cyclic folk rhythm aligns with Garba".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "sharp_dandiya_sticks" => {
            if cand_prof.rhythmic_pattern_detected == "sharp_dandiya_sticks" {
                reasons.push("Sharp wooden stick transients and tempo verified by DSP analysis".to_string());
                mood_affinity_score += 6.0;
            } else if all_cand_genres.contains("dandiya") || all_cand_genres.contains("raas") || (cand_prof.bpm >= 122.0 && cand_prof.is_percussion_heavy) {
                reasons.push("Sharp stick-striking cadence matches Dandiya tempo".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "deep_house_analog" => {
            if all_cand_genres.contains("deep") || all_cand_genres.contains("lounge") || all_cand_genres.contains("chill") || all_cand_genres.contains("jazz") {
                reasons.push("Warm Rhodes and hypnotic deep house groove".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "stadium_bigroom_kick" => {
            if cand_prof.is_electronic_dance && (all_cand_genres.contains("edm") || all_cand_genres.contains("big room") || all_cand_genres.contains("festival") || all_cand_genres.contains("drop")) {
                reasons.push("Massive stadium drop payoff aligns with Festival Mode".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "organic_vintage_drums" => {
            if all_cand_genres.contains("retro") || all_cand_genres.contains("disco") || all_cand_genres.contains("70s") || all_cand_genres.contains("80s") || all_cand_genres.contains("classic") {
                reasons.push("Organic acoustic drums and live brass instrumentation".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "afro_latin_syncopated" => {
            if all_cand_genres.contains("afrobeat") || all_cand_genres.contains("reggaeton") || all_cand_genres.contains("latin") || all_cand_genres.contains("dembow") {
                reasons.push("Syncopated Afro-Latin body groove".to_string());
                mood_affinity_score += 5.0;
            }
        }
        "four_on_the_floor_sub" => {
            if cand_prof.is_electronic_dance && (all_cand_genres.contains("club") || all_cand_genres.contains("remix") || all_cand_genres.contains("tech")) {
                reasons.push("Driving 4-on-the-floor kick and sub-bass".to_string());
                mood_affinity_score += 4.0;
            }
        }
        _ => {}
    }

    // Secondary Mood Fusion
    if let Some(sec) = secondary_mood {
        let sec_match = sec.genre_weights.iter().any(|(g, _)| all_cand_genres.contains(&g.to_lowercase()))
            || sub_moods.iter().any(|m| m.eq_ignore_ascii_case(&sec.id));
        if sec_match {
            reasons.push(format!("Secondary mood blend ({}) supported", sec.display_name));
            mood_affinity_score += 3.0;
        }
    }

    // 6. Multi-Track Set Lookahead Sequence Planning (5 pts max)
    let mut sequence_lookahead_score = 4.0;
    if unplayed_queue.len() > 1 {
        let other_candidates: Vec<&LocalRoomSubmission> = unplayed_queue
            .iter()
            .filter(|q| q.id != candidate.id)
            .collect();

        if !other_candidates.is_empty() {
            let mut strong_bridges = 0usize;
            for other in &other_candidates {
                let other_prof = extract_musical_profile(other, cand_prof.bpm, target_energy);
                let other_key_compat = match (&cand_prof.camelot_key, &other_prof.camelot_key) {
                    (Some(k1), Some(k2)) => calculate_camelot_compatibility(k1, k2),
                    _ => 0.70,
                };
                let other_bpm_compat = calculate_bpm_compatibility(cand_prof.bpm, other_prof.bpm);
                if other_key_compat >= 0.75 || other_bpm_compat >= 0.85 {
                    strong_bridges += 1;
                }
            }

            if strong_bridges >= 2 {
                reasons.push("Strong set anchor: provides smooth transition paths to upcoming queue tracks".to_string());
                sequence_lookahead_score = 5.0;
            } else if strong_bridges == 0 && other_candidates.len() >= 3 {
                penalties.push("Orphan track: lacks compatible transition follow-ups in active queue".to_string());
                sequence_lookahead_score = 1.0;
            }
        }
    }

    // 7. Playability Bonus & Penalty Deductions
    let cache_bonus = if candidate.acquisition_status == "cached_authorized" {
        5.0
    } else {
        0.0
    };

    let mut penalty_deduction = 0.0;
    if cur_prof
        .as_ref()
        .map(|c| c.is_electronic_dance && cand_prof.is_acoustic && delta_energy <= -0.30)
        .unwrap_or(false)
    {
        penalty_deduction += 20.0; // Severe texture shock
    }
    if cur_energy >= 0.78 && cand_prof.energy <= 0.40 {
        penalty_deduction += 20.0; // Sudden energy collapse
    }
    if cur_prof
        .as_ref()
        .map(|c| c.meter_label != cand_prof.meter_label)
        .unwrap_or(false)
    {
        penalty_deduction += 10.0; // Meter clash without bridge
    }
    if cur_prof
        .as_ref()
        .map(|c| c.vocal_density >= 0.60 && cand_prof.vocal_density >= 0.60)
        .unwrap_or(false)
    {
        penalty_deduction += 10.0; // Vocal clutter
    }

    // Mood-specific atmosphere checks (softened dynamically during intentional recovery / warm-up phases)
    let is_intentional_recovery = target_energy <= (primary_mood.target_energy - 0.12);
    let is_intentional_peak_lift = target_energy >= (primary_mood.target_energy + 0.05);

    if primary_mood.id == "cocktail_night" {
        if cand_prof.energy > 0.74 || all_cand_genres.contains("dhol") || all_cand_genres.contains("edm") {
            if is_exciting_contrast || is_intentional_peak_lift {
                penalty_deduction += 10.0;
                reasons.push("Controlled energetic contrast allowed for peak lounge lift".to_string());
            } else {
                penalty_deduction += 25.0;
                penalties.push(format!("Energy ({:.0}%) or timbre too aggressive for Cocktail Night lounge ambiance", cand_prof.energy * 100.0));
            }
        }
    } else if primary_mood.id == "baraat" {
        if cand_prof.energy < 0.72 || (cand_prof.is_acoustic && !all_cand_genres.contains("dhol")) {
            if is_intentional_recovery {
                reasons.push("Intentional processional breather / groom arrival pause supported".to_string());
            } else {
                penalty_deduction += 25.0;
                penalties.push(format!("Low energy ({:.0}%) or acoustic texture will deflate high-octane Baraat procession", cand_prof.energy * 100.0));
            }
        }
    } else if primary_mood.id == "festival_mode" {
        if cand_prof.energy < 0.70 {
            if is_intentional_recovery {
                reasons.push("Melodic vocal breakdown breather allowed before next stadium drop".to_string());
            } else {
                penalty_deduction += 25.0;
                penalties.push(format!("Subdued energy ({:.0}%) lacks Stadium Festival mainstage impact", cand_prof.energy * 100.0));
            }
        }
    } else if primary_mood.id == "retro_bollywood" {
        if cand_prof.energy > 0.82 || all_cand_genres.contains("dhol") || all_cand_genres.contains("bhangra") || all_cand_genres.contains("edm") || all_cand_genres.contains("trap") || (all_cand_genres.contains("club") && cand_prof.is_electronic_dance) {
            penalty_deduction += 25.0;
            penalties.push("Modern electronic club production or aggressive dhol clashes with Retro Bollywood golden vintage warmth".to_string());
        }
    } else if primary_mood.id == "garba" {
        if cand_prof.is_electronic_dance && all_cand_genres.contains("house") {
            penalty_deduction += 20.0;
            penalties.push("Western four-on-the-floor kick breaks cyclic Garba taali stepping".to_string());
        }
    } else if primary_mood.id == "dandiya" {
        if cand_prof.bpm < 120.0 {
            if is_intentional_recovery {
                reasons.push("Intentional tempo relief for dancers between fast stick rounds".to_string());
            } else {
                penalty_deduction += 20.0;
                penalties.push(format!("Tempo ({:.0} BPM) too slow for fast synchronized Dandiya stick striking", cand_prof.bpm));
            }
        }
    } else if primary_mood.id == "mehendi" {
        if cand_prof.energy > 0.82 || all_cand_genres.contains("trap") {
            penalty_deduction += 20.0;
            penalties.push("Heavy club energy overpowers relaxed daytime Mehendi celebration".to_string());
        }
    }

    let total: f64 = harmonic_score
        + bpm_score
        + groove_and_meter_score
        + energy_score
        + timbre_score
        + vocal_score
        + genre_score
        + mood_affinity_score
        + sequence_lookahead_score
        + cache_bonus
        - penalty_deduction;

    // Normalizing against maximum theoretical score (~115.0) prevents artificial ceiling clamping at 100.0,
    // preserving true candidate differentiation and tie-breaking for distinct DJ choices.
    let normalized = (total / 115.0) * 100.0;
    let clamped_total = (normalized.clamp(0.0_f64, 100.0_f64) * 10.0).round() / 10.0;

    DetailedDjEvaluation {
        total_score: clamped_total,
        harmonic_score,
        bpm_score,
        energy_score,
        predicted_energy_direction,
        groove_and_meter_score,
        vocal_score,
        timbre_score,
        sequence_lookahead_score,
        is_exciting_contrast,
        reasons,
        penalties,
    }
}

/// Evaluates an unplayed private room submission (from either guest or host — zero approval queue)
/// against the active mood, energy progression, current BPM (with half/double-time matching),
/// Camelot harmonic key, and phrase/playability readiness.
pub fn score_local_submission_for_dj(
    sub: &LocalRoomSubmission,
    primary_mood: &MoodDefinition,
    secondary_mood: Option<&MoodDefinition>,
    current_bpm: Option<f64>,
    current_camelot: Option<&str>,
    target_energy: f64,
) -> f64 {
    let dummy_cur = current_bpm.map(|bpm| {
        let mut s = sub.clone();
        s.bpm = Some(bpm);
        s.camelot_key = current_camelot.map(|k| k.to_string());
        s.overall_energy = Some(target_energy);
        s
    });

    let eval = evaluate_submission_dj_intelligence(
        sub,
        dummy_cur.as_ref(),
        primary_mood,
        secondary_mood,
        target_energy,
        &[],
    );

    eval.total_score
}

/// Computes dynamic energy target based on the selected mood and elapsed party duration
/// (gradual warm-up -> peak celebration -> wave modulation).
pub fn compute_mood_energy_target(
    primary_mood: &MoodDefinition,
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
    submissions: &[LocalRoomSubmission],
    primary_mood: &MoodDefinition,
    secondary_mood: Option<&MoodDefinition>,
    target_energy: f64,
) -> (Vec<(String, f64)>, Option<String>) {
    let currently_playing = submissions.iter().find(|s| s.dj_status == "playing");

    let recently_played_videos: std::collections::HashSet<&str> = submissions
        .iter()
        .filter(|s| s.dj_status == "played" || s.dj_status == "playing")
        .map(|s| s.provider_video_id.as_str())
        .collect();

    let queued_candidates: Vec<&LocalRoomSubmission> = submissions
        .iter()
        .filter(|s| s.dj_status == "queued_for_dj" || s.dj_status == "prepared")
        .collect();

    let unplayed_owned: Vec<LocalRoomSubmission> = queued_candidates.iter().map(|&s| s.clone()).collect();

    let mut scored_with_meta: Vec<(String, f64, bool, bool)> = queued_candidates
        .iter()
        .map(|&s| {
            let eval = evaluate_submission_dj_intelligence(
                s,
                currently_playing,
                primary_mood,
                secondary_mood,
                target_energy,
                &unplayed_owned,
            );
            let mut sc = eval.total_score;
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
    #[serde(default = "default_energy_direction")]
    pub predicted_energy_direction: String,
}

fn default_energy_direction() -> String {
    "sustain".to_string()
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
    dj_level: DjLevel,
    current: Option<&LocalRoomSubmission>,
    candidate: &LocalRoomSubmission,
    primary_mood: &MoodDefinition,
    confidence: f64,
) -> (String, String) {
    let cur_bpm = current.and_then(|c| c.bpm);
    let cand_bpm = candidate.bpm;
    let bpm_known = cur_bpm.is_some() && cand_bpm.is_some();
    let bpm_pct_diff = match (cur_bpm, cand_bpm) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => {
            let direct = (a - b).abs() / a;
            if dj_level >= DjLevel::Level2SmartMixing {
                let half = (a - b * 2.0).abs() / a;
                let double = (a - b * 0.5).abs() / a;
                direct.min(half).min(double)
            } else {
                direct
            }
        }
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
    submissions: &[LocalRoomSubmission],
    primary_mood: &MoodDefinition,
    secondary_mood: Option<&MoodDefinition>,
    target_energy: f64,
    dj_level: DjLevel,
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

    let unplayed_candidates: Vec<LocalRoomSubmission> = submissions
        .iter()
        .filter(|s| {
            s.dj_status != "playing"
                && s.dj_status != "played"
                && s.dj_status != "skipped_unplayable"
        })
        .cloned()
        .collect();

    let mut recs: Vec<DjRecommendation> = unplayed_candidates
        .iter()
        .map(|s| {
            let mut known = Vec::new();
            let mut missing = Vec::new();

            // Confidence starts from real available metadata (never goes silent when partial!)
            let mut confidence = 0.25_f64;

            if let Some(bpm) = s.bpm {
                known.push(format!("BPM {:.1}", bpm));
                confidence += 0.25;
            } else {
                missing.push("BPM not yet verified".to_string());
            }

            let mut key_compat = 0.5;
            if let Some(ref k) = s.camelot_key {
                known.push(format!("Camelot {}", k));
                confidence += 0.20;
                if let Some(ck) = current_camelot {
                    key_compat = calculate_camelot_compatibility(ck, k);
                }
            } else {
                missing.push("Musical key unknown".to_string());
            }

            if let Some(en) = s.overall_energy {
                known.push(format!("Energy {:.0}%", en * 100.0));
                confidence += 0.15;
            } else {
                missing.push("Energy profile unanalyzed".to_string());
            }

            let sub_genres: Vec<String> = serde_json::from_str(&s.genre_tags_json).unwrap_or_default();
            if !sub_genres.is_empty() {
                known.push(format!("Genre {}", sub_genres[0]));
            }

            let has_local_analysis = s.metadata_source == "local_analysis" && s.duration_ms.unwrap_or(0) > 0;
            if has_local_analysis {
                known.push("Beat grid & phrases analyzed".to_string());
                confidence += 0.05;
            } else {
                if s.duration_ms.unwrap_or(0) > 0 {
                    known.push(format!("Duration {}s", s.duration_ms.unwrap_or(0) / 1000));
                }
                missing.push("Beat grid & phrase alignment unverified".to_string());
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

            // Perform deep DJ intelligence evaluation
            let eval = evaluate_submission_dj_intelligence(
                s,
                currently_playing,
                primary_mood,
                secondary_mood,
                target_energy,
                &unplayed_candidates,
            );

            let mut reasons = eval.reasons;
            let mut score = eval.total_score;

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
            } else if eval.is_exciting_contrast {
                ("exciting_contrast", "Exciting Contrast")
            } else if key_compat >= 0.90 && s.camelot_key.is_some() {
                if dj_level >= DjLevel::Level4CreativeTools {
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
                predicted_energy_direction: eval.predicted_energy_direction,
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
        if top.is_playable && top.category != "exciting_contrast" {
            top.category = "best_next".to_string();
            top.category_label = "Best Next Track".to_string();
        }
    }

    recs.truncate(6);
    recs
}

