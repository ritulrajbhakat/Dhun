use crate::db::models::{PlayableTrackCandidate, PlaybackHistoryItem, GuestRequest};
use crate::dj::moods::MoodDefinition;

pub struct TrackSelector;

impl TrackSelector {
    pub fn score_and_select(
        candidates: &[PlayableTrackCandidate],
        currently_playing: Option<&PlayableTrackCandidate>,
        mood: &MoodDefinition,
        target_energy: f64,
        recent_history: &[PlaybackHistoryItem],
        approved_requests: &[GuestRequest],
    ) -> Option<PlayableTrackCandidate> {
        if candidates.is_empty() {
            return None;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let mut scored_candidates: Vec<(PlayableTrackCandidate, f64)> = Vec::with_capacity(candidates.len());

        for candidate in candidates {
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

            // Exclude recently played track unless library has very few tracks
            if was_track_played_recently && candidates.len() > 3 {
                continue;
            }

            // 1. Genre Score (0.0 to 1.0)
            let mut genre_score = 0.4;
            for (g, weight) in &mood.genre_weights {
                if candidate.track.genre.to_lowercase().contains(&g.to_lowercase()) {
                    genre_score = *weight;
                    break;
                }
            }

            // 2. BPM Compatibility Score
            let bpm_score = match currently_playing {
                Some(curr) => calculate_bpm_compatibility(curr.analysis.bpm, candidate.analysis.bpm),
                None => {
                    // Check if candidate falls in mood BPM range
                    if candidate.analysis.bpm >= mood.min_bpm && candidate.analysis.bpm <= mood.max_bpm {
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

            // 4. Energy Compatibility Score
            let energy_diff = (candidate.analysis.overall_energy - target_energy).abs();
            let energy_score = (1.0 - energy_diff).clamp(0.1, 1.0);

            // 5. Repetition Penalty
            let repetition_penalty = if was_artist_played_recently { 0.35 } else { 0.0 };

            // 6. Guest Request Boost
            let is_approved_request = approved_requests.iter().any(|r| r.track_id == candidate.track.id);
            let request_boost = if is_approved_request { 1.5 } else { 0.0 };

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

