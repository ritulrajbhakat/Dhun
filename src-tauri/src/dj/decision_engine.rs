use serde::{Deserialize, Serialize};
use crate::db::models::{PhraseMarker, TrackTimingProfile};
use crate::dj::levels::DjLevel;
use crate::dj::selector::calculate_camelot_compatibility;

/// Configurable tempo-matching & audio-quality policy (never a single hard-coded BPM delta).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoQualityPolicy {
    pub max_direct_stretch_pct: f64,
    pub max_half_double_stretch_pct: f64,
    pub max_gradual_ramp_pct: f64,
    pub min_beatgrid_confidence: f64,
    pub max_variable_drift_bpm: f64,
    pub allow_three_quarter_bridge: bool,
}

impl TempoQualityPolicy {
    pub fn for_dj_level(level: DjLevel) -> Self {
        match level {
            DjLevel::Level1TwoDeckMixer => Self {
                max_direct_stretch_pct: 4.0,
                max_half_double_stretch_pct: 0.0,
                max_gradual_ramp_pct: 0.0,
                min_beatgrid_confidence: 0.55,
                max_variable_drift_bpm: 3.0,
                allow_three_quarter_bridge: false,
            },
            DjLevel::Level2SmartMixing => Self {
                max_direct_stretch_pct: 8.0,
                max_half_double_stretch_pct: 6.0,
                max_gradual_ramp_pct: 0.0,
                min_beatgrid_confidence: 0.50,
                max_variable_drift_bpm: 3.5,
                allow_three_quarter_bridge: false,
            },
            DjLevel::Level3AdvancedPerformance => Self {
                max_direct_stretch_pct: 10.0,
                max_half_double_stretch_pct: 8.0,
                max_gradual_ramp_pct: 11.0,
                min_beatgrid_confidence: 0.48,
                max_variable_drift_bpm: 4.0,
                allow_three_quarter_bridge: true,
            },
            DjLevel::Level4CreativeTools => Self {
                max_direct_stretch_pct: 12.0,
                max_half_double_stretch_pct: 10.0,
                max_gradual_ramp_pct: 16.0,
                min_beatgrid_confidence: 0.45,
                max_variable_drift_bpm: 4.5,
                allow_three_quarter_bridge: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BpmRelationship {
    Direct,
    HalfTime,
    DoubleTime,
    ThreeQuarterBridge,
    GradualTempoRamp,
    NoBeatmatch,
}

impl BpmRelationship {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::HalfTime => "half_time",
            Self::DoubleTime => "double_time",
            Self::ThreeQuarterBridge => "three_quarter_bridge",
            Self::GradualTempoRamp => "gradual_tempo_ramp",
            Self::NoBeatmatch => "no_beatmatch",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBassHandoverSchedule {
    pub incoming_low_cut_until_frac: f64,
    pub bass_swap_at_frac: f64,
    pub outgoing_low_cut_at_frac: f64,
    pub mid_carve_db: f64,
    pub high_handover_frac: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedTechniqueAlternative {
    pub technique: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackTransitionInput {
    pub track_id: String,
    pub deck_id: usize,
    pub title: String,
    pub artist: String,
    pub duration_sec: f64,
    pub current_pos_sec: f64,
    pub bpm: Option<f64>,
    pub camelot_key: Option<String>,
    pub overall_energy: Option<f64>,
    pub genre_tags: Vec<String>,
    pub sections: Vec<PhraseMarker>,
    pub timing_profile: Option<TrackTimingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyDecisionContext {
    pub dj_level: DjLevel,
    pub mood_id: String,
    pub target_energy: f64,
    pub recent_techniques: Vec<String>,
    pub tempo_policy: TempoQualityPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDecisionPlan {
    /// `"planned"` | `"rejected"` | `"deferred"`
    pub status: String,
    pub outgoing_track_id: String,
    pub outgoing_deck_id: usize,
    pub incoming_track_id: String,
    pub incoming_deck_id: usize,
    pub outgoing_mix_out_sec: f64,
    pub outgoing_phrase_boundary_sec: f64,
    pub incoming_mix_in_sec: f64,
    pub incoming_phrase_boundary_sec: f64,
    pub outgoing_section_type: String,
    pub incoming_section_type: String,
    pub current_bpm: Option<f64>,
    pub incoming_bpm: Option<f64>,
    pub target_bpm: Option<f64>,
    pub bpm_relationship: BpmRelationship,
    pub tempo_adjustment_pct: f64,
    pub tempo_direction: String, // "up" | "down" | "none"
    pub incoming_playback_rate: f64,
    pub outgoing_target_rate: f64,
    pub beat_alignment_offset_sec: f64,
    pub meter_compatible: bool,
    pub meter_summary: String,
    pub beatgrid_confidence: f64,
    #[serde(default = "default_metric_conf")]
    pub bpm_confidence: f64,
    #[serde(default = "default_metric_conf")]
    pub downbeat_confidence: f64,
    #[serde(default = "default_metric_conf")]
    pub meter_confidence: f64,
    pub duration_beats: u32,
    pub duration_bars: u32,
    pub duration_sec: f64,
    #[serde(default = "default_crossfade_sec")]
    pub audio_crossfade_sec: f64,
    #[serde(default = "default_crossfade_sec")]
    pub musical_duration_sec: f64,
    pub vocal_overlap_policy: String,
    pub energy_progression: String,
    pub eq_bass_handover_schedule: EqBassHandoverSchedule,
    pub selected_technique: String,
    pub scheduled_execution_sec: f64,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub rejected_alternatives: Vec<RejectedTechniqueAlternative>,
    pub rejection_reason: Option<String>,
}

fn default_crossfade_sec() -> f64 {
    2.0
}

fn default_metric_conf() -> f64 {
    0.85
}

#[derive(Debug, Clone)]
struct MoodRhythmProfile {
    is_indian_vocal_centric: bool,
    is_percussion_heavy_dhol: bool,
    is_cyclic_folk_garba: bool,
    is_electronic_club: bool,
    prefer_short_clean_mix: bool,
}

fn classify_mood_rhythm(mood_id: &str, out_genres: &[String], in_genres: &[String]) -> MoodRhythmProfile {
    let m = mood_id.to_lowercase();
    let all_genres = out_genres
        .iter()
        .chain(in_genres.iter())
        .map(|g| g.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    let is_cyclic_folk_garba = m.contains("garba")
        || m.contains("dandiya")
        || all_genres.contains("garba")
        || all_genres.contains("dandiya");
    let is_percussion_heavy_dhol = m.contains("baraat")
        || all_genres.contains("dhol")
        || all_genres.contains("bhangra");
    let is_indian_vocal_centric = m.contains("shaadi")
        || m.contains("bollywood")
        || m.contains("mehendi")
        || m.contains("haldi")
        || m.contains("sangeet")
        || m.contains("retro_bollywood")
        || m.contains("punjabi")
        || all_genres.contains("bollywood")
        || all_genres.contains("punjabi")
        || all_genres.contains("sufi");
    let is_electronic_club = m.contains("desi_club")
        || m.contains("house")
        || m.contains("club")
        || all_genres.contains("edm")
        || all_genres.contains("house")
        || all_genres.contains("techno")
        || all_genres.contains("remix");

    MoodRhythmProfile {
        is_indian_vocal_centric,
        is_percussion_heavy_dhol,
        is_cyclic_folk_garba,
        is_electronic_club,
        prefer_short_clean_mix: is_percussion_heavy_dhol || is_cyclic_folk_garba,
    }
}

/// Evaluates local BPM at `position_sec` using the track's variable tempo map when available.
pub fn local_bpm_at_sec(track: &TrackTransitionInput, position_sec: f64) -> Option<f64> {
    if let Some(ref tp) = track.timing_profile {
        let pos_ms = (position_sec * 1000.0).round() as i64;
        if let Some(seg) = tp
            .tempo_map
            .iter()
            .find(|s| pos_ms >= s.start_ms && pos_ms <= s.end_ms)
            .or_else(|| tp.tempo_map.last())
        {
            if seg.bpm > 40.0 {
                return Some(seg.bpm);
            }
        }
        if tp.average_bpm > 40.0 {
            return Some(tp.average_bpm);
        }
    }
    track.bpm.filter(|b| *b > 40.0)
}

#[derive(Debug, Clone)]
pub struct BpmDecisionResult {
    pub relationship: BpmRelationship,
    pub target_bpm: Option<f64>,
    pub incoming_playback_rate: f64,
    pub outgoing_target_rate: f64,
    pub adjustment_pct: f64,
    pub direction: String,
    pub reason: String,
}

/// Evaluates all 6 BPM relationships deterministically under `TempoQualityPolicy`,
/// respecting beatgrid confidence, variable-tempo stability, and meter compatibility.
pub fn evaluate_bpm_relationship(
    out_bpm: Option<f64>,
    in_bpm: Option<f64>,
    beatgrid_conf: f64,
    meter_compatible: bool,
    out_variable: bool,
    in_variable: bool,
    policy: &TempoQualityPolicy,
) -> BpmDecisionResult {
    let (Some(a), Some(b)) = (out_bpm, in_bpm) else {
        return BpmDecisionResult {
            relationship: BpmRelationship::NoBeatmatch,
            target_bpm: None,
            incoming_playback_rate: 1.0,
            outgoing_target_rate: 1.0,
            adjustment_pct: 0.0,
            direction: "none".to_string(),
            reason: "BPM unavailable on one or both tracks — refusing forced beatmatch.".to_string(),
        };
    };

    if beatgrid_conf < policy.min_beatgrid_confidence {
        return BpmDecisionResult {
            relationship: BpmRelationship::NoBeatmatch,
            target_bpm: Some(b),
            incoming_playback_rate: 1.0,
            outgoing_target_rate: 1.0,
            adjustment_pct: 0.0,
            direction: "none".to_string(),
            reason: format!(
                "Beatgrid confidence ({:.0}%) below quality threshold ({:.0}%) — no beatmatch.",
                beatgrid_conf * 100.0,
                policy.min_beatgrid_confidence * 100.0
            ),
        };
    }

    if !meter_compatible {
        return BpmDecisionResult {
            relationship: BpmRelationship::NoBeatmatch,
            target_bpm: Some(b),
            incoming_playback_rate: 1.0,
            outgoing_target_rate: 1.0,
            adjustment_pct: 0.0,
            direction: "none".to_string(),
            reason: "Incompatible rhythmic meters (e.g., 3/4 or 6/8 vs 4/4) — preserving natural rhythm without beatmatch.".to_string(),
        };
    }

    if out_variable || in_variable {
        let direct_pct = ((a - b).abs() / a) * 100.0;
        if direct_pct <= 3.0 {
            let rate = (a / b).clamp(0.97, 1.03);
            return BpmDecisionResult {
                relationship: BpmRelationship::Direct,
                target_bpm: Some(a),
                incoming_playback_rate: rate,
                outgoing_target_rate: 1.0,
                adjustment_pct: ((rate - 1.0).abs() * 1000.0).round() / 10.0,
                direction: if rate > 1.001 { "up" } else if rate < 0.999 { "down" } else { "none" }.to_string(),
                reason: "Variable-tempo tracks within 3% local BPM — light direct beat-assist.".to_string(),
            };
        }
        return BpmDecisionResult {
            relationship: BpmRelationship::NoBeatmatch,
            target_bpm: Some(b),
            incoming_playback_rate: 1.0,
            outgoing_target_rate: 1.0,
            adjustment_pct: 0.0,
            direction: "none".to_string(),
            reason: "Variable live percussion tempo drift detected — avoiding pitch stretch to preserve rhythmic character.".to_string(),
        };
    }

    let make_dir = |rate: f64| -> String {
        if rate > 1.001 {
            "up".to_string()
        } else if rate < 0.999 {
            "down".to_string()
        } else {
            "none".to_string()
        }
    };

    // 1. Direct match
    let direct_rate = a / b;
    let direct_pct = (direct_rate - 1.0).abs() * 100.0;
    if direct_pct <= policy.max_direct_stretch_pct {
        return BpmDecisionResult {
            relationship: BpmRelationship::Direct,
            target_bpm: Some(a),
            incoming_playback_rate: (direct_rate * 1000.0).round() / 1000.0,
            outgoing_target_rate: 1.0,
            adjustment_pct: (direct_pct * 10.0).round() / 10.0,
            direction: make_dir(direct_rate),
            reason: format!(
                "Direct BPM match ({:.1} → {:.1} BPM, {:.1}% pitch shift within {:.0}% quality policy).",
                b, a, direct_pct, policy.max_direct_stretch_pct
            ),
        };
    }

    // 2. Half-time relationship (incoming is ~half of outgoing, e.g. 64 BPM into 128 BPM)
    if policy.max_half_double_stretch_pct > 0.0 {
        let half_rate = a / (b * 2.0);
        let half_pct = (half_rate - 1.0).abs() * 100.0;
        if half_pct <= policy.max_half_double_stretch_pct {
            return BpmDecisionResult {
                relationship: BpmRelationship::HalfTime,
                target_bpm: Some(a * 0.5),
                incoming_playback_rate: (half_rate * 1000.0).round() / 1000.0,
                outgoing_target_rate: 1.0,
                adjustment_pct: (half_pct * 10.0).round() / 10.0,
                direction: make_dir(half_rate),
                reason: format!(
                    "Half-time rhythmic lock ({:.1} BPM groove aligned 1:2 with {:.1} BPM, {:.1}% trim).",
                    b, a, half_pct
                ),
            };
        }

        // 3. Double-time relationship (incoming is ~double of outgoing, e.g. 140 BPM into 70 BPM)
        let double_rate = a / (b * 0.5);
        let double_pct = (double_rate - 1.0).abs() * 100.0;
        if double_pct <= policy.max_half_double_stretch_pct {
            return BpmDecisionResult {
                relationship: BpmRelationship::DoubleTime,
                target_bpm: Some(a * 2.0),
                incoming_playback_rate: (double_rate * 1000.0).round() / 1000.0,
                outgoing_target_rate: 1.0,
                adjustment_pct: (double_pct * 10.0).round() / 10.0,
                direction: make_dir(double_rate),
                reason: format!(
                    "Double-time rhythmic lock ({:.1} BPM aligned 2:1 with {:.1} BPM, {:.1}% trim).",
                    b, a, double_pct
                ),
            };
        }
    }

    // 4. Gradual tempo ramp (Level 3–4 when delta exceeds direct pitch-lock but stays musical)
    if policy.max_gradual_ramp_pct > 0.0 && direct_pct <= policy.max_gradual_ramp_pct {
        let out_target_rate = (b / a).clamp(0.84, 1.16);
        let in_start_rate = (a / b).clamp(0.84, 1.16);
        return BpmDecisionResult {
            relationship: BpmRelationship::GradualTempoRamp,
            target_bpm: Some(b),
            incoming_playback_rate: (in_start_rate * 1000.0).round() / 1000.0,
            outgoing_target_rate: (out_target_rate * 1000.0).round() / 1000.0,
            adjustment_pct: (direct_pct * 10.0).round() / 10.0,
            direction: if b > a { "up".to_string() } else { "down".to_string() },
            reason: format!(
                "Gradual tempo ramp bridging {:.1} → {:.1} BPM ({:.1}% shift) smoothly over transition.",
                a, b, direct_pct
            ),
        };
    }

    // 5. 3:4 / 4:3 rhythmic bridge (e.g., 96 BPM <-> 128 BPM) on Level 3–4
    if policy.allow_three_quarter_bridge {
        let ratio_3_4 = (a * 0.75) / b;
        let ratio_4_3 = (a * (4.0 / 3.0)) / b;
        let pct_3_4 = (ratio_3_4 - 1.0).abs() * 100.0;
        let pct_4_3 = (ratio_4_3 - 1.0).abs() * 100.0;
        if pct_3_4 <= 4.0 || pct_4_3 <= 4.0 {
            let best_rate = if pct_3_4 <= pct_4_3 { ratio_3_4 } else { ratio_4_3 };
            let best_pct = pct_3_4.min(pct_4_3);
            return BpmDecisionResult {
                relationship: BpmRelationship::ThreeQuarterBridge,
                target_bpm: Some(b),
                incoming_playback_rate: (best_rate * 1000.0).round() / 1000.0,
                outgoing_target_rate: 1.0,
                adjustment_pct: (best_pct * 10.0).round() / 10.0,
                direction: make_dir(best_rate),
                reason: format!(
                    "3:4 rhythmic tempo bridge ({:.1} → {:.1} BPM) using echo/cut phrasing.",
                    a, b
                ),
            };
        }
    }

    // 6. Excessive tempo change -> refuse destructive pitch stretch!
    BpmDecisionResult {
        relationship: BpmRelationship::NoBeatmatch,
        target_bpm: Some(b),
        incoming_playback_rate: 1.0,
        outgoing_target_rate: 1.0,
        adjustment_pct: (direct_pct * 10.0).round() / 10.0,
        direction: "none".to_string(),
        reason: format!(
            "BPM gap ({:.1} → {:.1} BPM, {:.1}%) exceeds musical quality limit — refusing beatmatch in favor of clean cut/echo.",
            a, b, direct_pct
        ),
    }
}

/// Core deterministic DJ Transition Decision Engine.
/// Separates musical planning (WHEN, WHERE, HOW, HOW LONG, WHICH BPM relationship) from DSP execution.
pub fn plan_dj_transition(
    outgoing: &TrackTransitionInput,
    incoming: &TrackTransitionInput,
    party: &PartyDecisionContext,
) -> TransitionDecisionPlan {
    let remaining_sec = (outgoing.duration_sec - outgoing.current_pos_sec).max(0.0);
    let mut reasons = Vec::new();
    let mut rejected_alternatives = Vec::new();

    // 1. Meter & separate, honest confidence evaluations
    let out_conf = outgoing
        .timing_profile
        .as_ref()
        .map(|t| t.beatgrid_confidence)
        .unwrap_or(if outgoing.bpm.is_some() { 0.85 } else { 0.40 });
    let in_conf = incoming
        .timing_profile
        .as_ref()
        .map(|t| t.beatgrid_confidence)
        .unwrap_or(if incoming.bpm.is_some() { 0.85 } else { 0.40 });
    let beatgrid_confidence = ((out_conf.min(in_conf)) * 100.0).round() / 100.0;

    let out_bpm_conf = outgoing
        .timing_profile
        .as_ref()
        .map(|t| t.bpm_confidence)
        .unwrap_or(if outgoing.bpm.is_some() { 0.90 } else { 0.45 });
    let in_bpm_conf = incoming
        .timing_profile
        .as_ref()
        .map(|t| t.bpm_confidence)
        .unwrap_or(if incoming.bpm.is_some() { 0.90 } else { 0.45 });
    let bpm_confidence = ((out_bpm_conf.min(in_bpm_conf)) * 100.0).round() / 100.0;

    let downbeat_confidence = outgoing
        .timing_profile
        .as_ref()
        .map(|t| t.downbeat_confidence)
        .unwrap_or(0.80);

    let out_meter = outgoing
        .timing_profile
        .as_ref()
        .map(|t| (t.meter.beats_per_bar, t.meter.meter_label.as_str(), t.meter.confidence))
        .unwrap_or((4, "4/4", 0.60));
    let in_meter = incoming
        .timing_profile
        .as_ref()
        .map(|t| (t.meter.beats_per_bar, t.meter.meter_label.as_str(), t.meter.confidence))
        .unwrap_or((4, "4/4", 0.60));

    let meter_confidence = ((out_meter.2.min(in_meter.2)) * 100.0).round() / 100.0;
    let meter_uncertain = out_meter.1 == "uncertain" || in_meter.1 == "uncertain";
    // Uncertain meter is never treated as confirmed incompatibility! Assume standard 4/4 phrasing:
    let meter_compatible = if meter_uncertain {
        true
    } else {
        out_meter.0 == in_meter.0
    };
    let meter_summary = format!("{} → {}", out_meter.1, in_meter.1);
    let beats_per_bar = if meter_compatible && out_meter.1 != "uncertain" { out_meter.0.max(3) } else { 4 };

    let out_variable = outgoing
        .timing_profile
        .as_ref()
        .map(|t| t.is_variable_tempo || t.tempo_drift_bpm >= party.tempo_policy.max_variable_drift_bpm)
        .unwrap_or(false);
    let in_variable = incoming
        .timing_profile
        .as_ref()
        .map(|t| t.is_variable_tempo || t.tempo_drift_bpm >= party.tempo_policy.max_variable_drift_bpm)
        .unwrap_or(false);

    // Guard: reject if outgoing track has almost no remaining playable time (< 1.2s)
    if remaining_sec < 1.2 {
        return TransitionDecisionPlan {
            status: "rejected".to_string(),
            outgoing_track_id: outgoing.track_id.clone(),
            outgoing_deck_id: outgoing.deck_id,
            incoming_track_id: incoming.track_id.clone(),
            incoming_deck_id: incoming.deck_id,
            outgoing_mix_out_sec: outgoing.duration_sec,
            outgoing_phrase_boundary_sec: outgoing.duration_sec,
            incoming_mix_in_sec: 0.0,
            incoming_phrase_boundary_sec: 0.0,
            outgoing_section_type: "end".to_string(),
            incoming_section_type: "intro".to_string(),
            current_bpm: local_bpm_at_sec(outgoing, outgoing.current_pos_sec),
            incoming_bpm: local_bpm_at_sec(incoming, 0.0),
            target_bpm: local_bpm_at_sec(incoming, 0.0),
            bpm_relationship: BpmRelationship::NoBeatmatch,
            tempo_adjustment_pct: 0.0,
            tempo_direction: "none".to_string(),
            incoming_playback_rate: 1.0,
            outgoing_target_rate: 1.0,
            beat_alignment_offset_sec: 0.0,
            meter_compatible,
            meter_summary,
            beatgrid_confidence,
            bpm_confidence,
            downbeat_confidence,
            meter_confidence,
            duration_beats: 1,
            duration_bars: 0,
            duration_sec: 0.25,
            audio_crossfade_sec: 0.25,
            musical_duration_sec: 0.50,
            vocal_overlap_policy: "clean_cut_handover".to_string(),
            energy_progression: "reset".to_string(),
            eq_bass_handover_schedule: EqBassHandoverSchedule {
                incoming_low_cut_until_frac: 0.0,
                bass_swap_at_frac: 0.5,
                outgoing_low_cut_at_frac: 0.5,
                mid_carve_db: 0.0,
                high_handover_frac: 0.5,
            },
            selected_technique: "safe_quick_cut".to_string(),
            scheduled_execution_sec: outgoing.current_pos_sec,
            confidence: 0.2,
            reasons: vec!["Insufficient remaining playback time (< 1.2s) for a musical transition.".to_string()],
            rejected_alternatives: vec![],
            rejection_reason: Some("Insufficient remaining playback time (< 1.2s).".to_string()),
        };
    }

    let mood_rhythm = classify_mood_rhythm(&party.mood_id, &outgoing.genre_tags, &incoming.genre_tags);
    let genres_conflict = !outgoing.genre_tags.is_empty()
        && !incoming.genre_tags.is_empty()
        && !outgoing.genre_tags.iter().any(|og| {
            incoming
                .genre_tags
                .iter()
                .any(|ig| og.eq_ignore_ascii_case(ig))
        });

    // 2. Intelligent Mix-Out & Mix-In Section Pairing (Section 4)
    let upcoming_out_sections: Vec<&PhraseMarker> = outgoing
        .sections
        .iter()
        .filter(|s| (s.end_ms as f64 / 1000.0) > outgoing.current_pos_sec + 2.0)
        .collect();

    // Check if outgoing track is currently inside a build-up right before a drop/peak payoff!
    let active_out_section = outgoing.sections.iter().find(|s| {
        let st = s.start_ms as f64 / 1000.0;
        let en = s.end_ms as f64 / 1000.0;
        outgoing.current_pos_sec >= st && outgoing.current_pos_sec < en
    });
    let protecting_buildup_payoff = active_out_section
        .map(|s| s.section_type == "build" && s.confidence >= 0.55)
        .unwrap_or(false);

    // Choose best outgoing exit phrase boundary (never cutting mid-build or mid-hook prematurely!)
    let (out_mix_out_sec, out_section_type, out_vocal_act, out_energy) = if !upcoming_out_sections.is_empty() {
        let mut best_sec: Option<(&PhraseMarker, f64, f64)> = None;
        for (idx, sec) in upcoming_out_sections.iter().enumerate() {
            let st_sec = (sec.start_ms as f64 / 1000.0).max(outgoing.current_pos_sec + 1.0);
            let en_sec = (sec.end_ms as f64 / 1000.0).min(outgoing.duration_sec);
            let sec_type = sec.section_type.as_str();

            // Do not cut a build-up right before its drop!
            if sec_type == "build" {
                let has_following_drop = upcoming_out_sections
                    .get(idx + 1)
                    .map(|n| n.section_type == "drop" || n.section_type == "peak" || n.section_type == "chorus")
                    .unwrap_or(false);
                if has_following_drop {
                    continue;
                }
            }

            let mut sc = 0.0f64;
            let exit_point = match sec_type {
                "outro" => {
                    sc += 5.0;
                    st_sec
                }
                "breakdown" => {
                    sc += 4.2;
                    st_sec
                }
                "instrumental" | "groove" => {
                    sc += 3.8;
                    st_sec
                }
                "chorus" | "peak" => {
                    // Wait until END of chorus/hook so memorable hook plays completely!
                    sc += 3.4;
                    (en_sec - 8.0).max(st_sec)
                }
                "drop" => {
                    // Never immediately fade out a fresh drop — wait for end of drop phrase
                    sc += 2.6;
                    (en_sec - 4.0).max(st_sec)
                }
                "verse" => {
                    sc += 2.2;
                    en_sec.min(outgoing.duration_sec - 4.0).max(st_sec)
                }
                _ => {
                    sc += 1.5;
                    st_sec
                }
            };

            // Avoid picking an exit point that has too little runway left
            if outgoing.duration_sec - exit_point < 2.0 {
                sc -= 2.5;
            }
            if sec.vocal_activity < 0.40 {
                sc += 0.8;
            }
            if best_sec.map(|b| sc > b.2).unwrap_or(true) {
                best_sec = Some((sec, exit_point, sc));
            }
        }

        if let Some((sec, exit_pt, _)) = best_sec {
            if protecting_buildup_payoff {
                reasons.push("Protected active build-up so its drop/payoff completes before mix-out.".to_string());
            }
            (exit_pt, sec.section_type.clone(), sec.vocal_activity, sec.energy_level)
        } else {
            let safe_out = (outgoing.duration_sec - 8.0).max(outgoing.current_pos_sec + 1.0);
            (safe_out, "phrase".to_string(), 0.45, outgoing.overall_energy.unwrap_or(0.7))
        }
    } else {
        let safe_out = (outgoing.duration_sec - 8.0).max(outgoing.current_pos_sec + 1.0);
        (safe_out, "phrase".to_string(), 0.45, outgoing.overall_energy.unwrap_or(0.7))
    };

    // Choose best incoming entry point given outgoing exit context (avoiding vocal clash & competing drops!)
    let (in_mix_in_sec, in_section_type, in_vocal_act, in_energy) = if !incoming.sections.is_empty() {
        let mut best_in: Option<(&PhraseMarker, f64)> = None;
        for sec in &incoming.sections {
            let st_sec = (sec.start_ms as f64 / 1000.0).max(0.0);
            if incoming.duration_sec - st_sec < 12.0 {
                continue;
            }
            let stype = sec.section_type.as_str();
            if stype == "outro" {
                continue;
            }
            let mut sc = 0.0f64;

            // Preferred musical pairings (Section 4)
            if stype == "intro" && matches!(out_section_type.as_str(), "chorus" | "peak" | "outro" | "verse") {
                sc += 4.5;
            } else if matches!(stype, "instrumental" | "groove") && out_vocal_act >= 0.50 {
                sc += 4.8; // Incoming instrumental over outgoing vocal section
            } else if stype == "breakdown" && matches!(out_section_type.as_str(), "outro" | "peak" | "chorus") {
                sc += 4.0;
            } else if stype == "build" && matches!(out_section_type.as_str(), "breakdown" | "groove" | "outro") {
                sc += 4.3;
            } else if matches!(stype, "drop" | "chorus" | "peak") && out_section_type == "breakdown" {
                sc += 4.6; // Breakdown-to-drop high-impact entry!
            } else if stype == "intro" {
                sc += 3.2;
            } else if matches!(stype, "groove" | "instrumental") {
                sc += 3.5;
            } else if stype == "chorus" {
                sc += 2.4;
            }

            // Penalize vocal-vs-vocal clashes heavily
            if out_vocal_act >= 0.55 && sec.vocal_activity >= 0.55 {
                sc -= 4.5;
            }
            // Penalize competing drops
            if out_section_type == "drop" && stype == "drop" {
                sc -= 5.0;
            }
            // Match party energy target
            sc -= (sec.energy_level - party.target_energy).abs() * 1.2;

            if best_in.map(|b| sc > b.1).unwrap_or(true) {
                best_in = Some((sec, sc));
            }
        }
        if let Some((sec, _)) = best_in {
            (
                (sec.start_ms as f64 / 1000.0).max(0.0),
                sec.section_type.clone(),
                sec.vocal_activity,
                sec.energy_level,
            )
        } else {
            (0.0, "intro".to_string(), 0.25, incoming.overall_energy.unwrap_or(0.7))
        }
    } else {
        (0.0, "intro".to_string(), 0.25, incoming.overall_energy.unwrap_or(0.7))
    };

    // 3. Evaluate BPM Relationship at the ACTUAL local mix-out & mix-in positions (Tempo Map aware!)
    let out_local_bpm = local_bpm_at_sec(outgoing, out_mix_out_sec);
    let in_local_bpm = local_bpm_at_sec(incoming, in_mix_in_sec);
    let bpm_decision = evaluate_bpm_relationship(
        out_local_bpm,
        in_local_bpm,
        beatgrid_confidence,
        meter_compatible,
        out_variable,
        in_variable,
        &party.tempo_policy,
    );
    reasons.push(bpm_decision.reason.clone());

    // 4. Vocal overlap policy & harmonic key evaluation
    let vocal_clash = out_vocal_act >= 0.55 && in_vocal_act >= 0.55;
    let vocal_overlap_policy = if vocal_clash {
        reasons.push("Detected high vocal activity on both decks — enforcing vocal-clash avoidance (clean handover / mid carve).".to_string());
        "avoid_vocal_clash".to_string()
    } else if out_vocal_act >= 0.50 && in_vocal_act <= 0.35 {
        reasons.push("Incoming instrumental/intro cleanly supports outgoing vocal phrase.".to_string());
        "instrumental_under_vocal".to_string()
    } else if bpm_decision.relationship == BpmRelationship::NoBeatmatch {
        "clean_cut_handover".to_string()
    } else {
        "vocal_clear_blend".to_string()
    };

    let key_compat = match (outgoing.camelot_key.as_deref(), incoming.camelot_key.as_deref()) {
        (Some(ka), Some(kb)) => calculate_camelot_compatibility(ka, kb),
        _ => 0.65,
    };

    let energy_delta = in_energy - out_energy;
    let energy_progression = if energy_delta >= 0.12 {
        "build"
    } else if energy_delta <= -0.14 {
        "drop_relief"
    } else if out_energy >= 0.80 && in_energy >= 0.80 {
        "peak_hold"
    } else {
        "sustain"
    }
    .to_string();

    // 5. Deterministic Transition Strategy Selection across DJ Levels 1–4 (Sections 6, 8, 9)
    let recent_used = |name: &str| party.recent_techniques.iter().take(2).any(|t| t == name);

    let selected_technique = if bpm_decision.relationship == BpmRelationship::NoBeatmatch {
        rejected_alternatives.push(RejectedTechniqueAlternative {
            technique: "phrase_long_blend".to_string(),
            reason: "Rejected: no reliable beatmatch/meter alignment for multi-bar blend.".to_string(),
        });
        rejected_alternatives.push(RejectedTechniqueAlternative {
            technique: "timed_bass_swap".to_string(),
            reason: "Rejected: requires beat-locked tempo match.".to_string(),
        });
        if party.dj_level >= DjLevel::Level4CreativeTools && genres_conflict && !recent_used("backspin_cut") {
            reasons.push("Level 4 dramatic genre/tempo shift without forced pitch stretch: vinyl backspin cut.".to_string());
            "backspin_cut".to_string()
        } else if party.dj_level >= DjLevel::Level3AdvancedPerformance && !mood_rhythm.is_percussion_heavy_dhol {
            reasons.push("Beatmatch unavailable/unsuitable — Level 3+ beat-free echo-out tail into clean incoming drop.".to_string());
            "echo_out_blend".to_string()
        } else if party.dj_level == DjLevel::Level2SmartMixing
            && beatgrid_confidence >= 0.50
            && remaining_sec >= 5.0
            && !recent_used("filter_sweep_drop")
        {
            reasons.push("Level 2 filter sweep drop resolves cleanly into incoming track without tempo clash.".to_string());
            "filter_sweep_drop".to_string()
        } else if (party.dj_level == DjLevel::Level1TwoDeckMixer || party.dj_level == DjLevel::Level2SmartMixing)
            && beatgrid_confidence >= 0.50
            && (remaining_sec >= 6.0 || out_section_type == "outro" || in_section_type == "intro")
        {
            reasons.push("Phrase-aligned 2-deck EQ handover selected to transition cleanly between unbeatmatched / differing meter tracks.".to_string());
            "eq3_handover".to_string()
        } else {
            reasons.push("Safe clean quick cut preserves rhythmic integrity when remaining time is short or beatgrid is unreliable.".to_string());
            "safe_quick_cut".to_string()
        }
    } else if bpm_decision.relationship == BpmRelationship::GradualTempoRamp {
        rejected_alternatives.push(RejectedTechniqueAlternative {
            technique: "timed_bass_swap".to_string(),
            reason: "Rejected: BPM difference exceeds static pitch-lock limit; gradual ramp required.".to_string(),
        });
        if party.dj_level >= DjLevel::Level4CreativeTools {
            reasons.push("Level 4 continuous tempo-ramp bridge smoothly aligns tempo delta.".to_string());
            "tempo_ramp_bridge".to_string()
        } else {
            reasons.push("Level 3 resonant filter sweep / echo-out bridges tempo shift cleanly.".to_string());
            "echo_out_blend".to_string()
        }
    } else if bpm_decision.relationship == BpmRelationship::ThreeQuarterBridge {
        rejected_alternatives.push(RejectedTechniqueAlternative {
            technique: "phrase_long_blend".to_string(),
            reason: "Rejected: 3:4 polyrhythmic bridge works best via echo-out/filter rather than sustained overlap.".to_string(),
        });
        if party.dj_level >= DjLevel::Level3AdvancedPerformance {
            "echo_out_blend".to_string()
        } else if party.dj_level == DjLevel::Level2SmartMixing {
            "filter_sweep_drop".to_string()
        } else {
            "eq3_handover".to_string()
        }
    } else if vocal_clash || mood_rhythm.prefer_short_clean_mix {
        rejected_alternatives.push(RejectedTechniqueAlternative {
            technique: "phrase_long_blend".to_string(),
            reason: if vocal_clash {
                "Rejected: long overlap would cause vocal-vs-vocal clash.".to_string()
            } else {
                "Rejected: Indian dhol/garba percussion clarity degrades under long overlapping blends.".to_string()
            },
        });
        match party.dj_level {
            DjLevel::Level1TwoDeckMixer => "eq3_handover".to_string(),
            DjLevel::Level2SmartMixing => {
                if mood_rhythm.is_percussion_heavy_dhol && genres_conflict {
                    "safe_quick_cut".to_string()
                } else {
                    "timed_bass_swap".to_string()
                }
            }
            DjLevel::Level3AdvancedPerformance => "echo_out_blend".to_string(),
            DjLevel::Level4CreativeTools => {
                if out_section_type == "breakdown" || energy_delta >= 0.10 {
                    "echo_freeze_drop".to_string()
                } else {
                    "echo_out_blend".to_string()
                }
            }
        }
    } else {
        // Beat-locked & vocal-clear musical pairing — choose idiomatic level technique!
        match party.dj_level {
            DjLevel::Level1TwoDeckMixer => {
                rejected_alternatives.push(RejectedTechniqueAlternative {
                    technique: "timed_bass_swap".to_string(),
                    reason: "Rejected: Level 1 preserves classic 2-deck EQ/phrase mixing.".to_string(),
                });
                if bpm_decision.adjustment_pct <= 3.0 && key_compat >= 0.75 {
                    "phrase_long_blend".to_string()
                } else {
                    "eq3_handover".to_string()
                }
            }
            DjLevel::Level2SmartMixing => {
                rejected_alternatives.push(RejectedTechniqueAlternative {
                    technique: "beat_roll_buildup".to_string(),
                    reason: "Rejected: performance FX unlocked at Level 3+.".to_string(),
                });
                if key_compat >= 0.80 && !recent_used("timed_bass_swap") {
                    "timed_bass_swap".to_string()
                } else {
                    "filter_sweep_drop".to_string()
                }
            }
            DjLevel::Level3AdvancedPerformance => {
                if (out_section_type == "breakdown" || energy_delta >= 0.08) && !recent_used("beat_roll_buildup") {
                    "beat_roll_buildup".to_string()
                } else if mood_rhythm.is_electronic_club && key_compat >= 0.85 && !recent_used("four_deck_layer") {
                    "four_deck_layer".to_string()
                } else if energy_delta <= -0.10 {
                    "echo_out_blend".to_string()
                } else {
                    "filtered_buildup_drop".to_string()
                }
            }
            DjLevel::Level4CreativeTools => {
                if key_compat >= 0.88 && in_vocal_act <= 0.45 && !recent_used("creative_mashup_layer") {
                    "creative_mashup_layer".to_string()
                } else if (out_section_type == "breakdown" || energy_delta >= 0.08) && !recent_used("stutter_gate_drop") {
                    "stutter_gate_drop".to_string()
                } else if !recent_used("echo_freeze_drop") {
                    "echo_freeze_drop".to_string()
                } else {
                    "four_deck_layer".to_string()
                }
            }
        }
    };

    // 6. Adaptive Transition Length in Beats, Bars & Seconds (Section 5)
    let ref_bpm = out_local_bpm.or(in_local_bpm).unwrap_or(124.0).clamp(60.0, 200.0);
    let beat_sec = 60.0 / ref_bpm;
    let raw_beats: u32 = match selected_technique.as_str() {
        "safe_quick_cut" => 4,
        "backspin_cut" => 4,
        "stutter_gate_drop" | "beat_roll_buildup" | "echo_freeze_drop" => 8,
        "echo_out_blend" => {
            if vocal_clash || mood_rhythm.prefer_short_clean_mix { 8 } else { 16 }
        }
        "eq3_handover" | "timed_bass_swap" | "filter_sweep_drop" | "filtered_buildup_drop" | "tempo_ramp_bridge" => {
            if vocal_clash || mood_rhythm.is_indian_vocal_centric || genres_conflict {
                8
            } else if mood_rhythm.is_electronic_club && in_vocal_act < 0.35 {
                32
            } else {
                16
            }
        }
        "phrase_long_blend" => {
            if mood_rhythm.is_electronic_club && out_vocal_act < 0.30 && in_vocal_act < 0.30 && key_compat >= 0.85 {
                64
            } else if vocal_clash || mood_rhythm.is_indian_vocal_centric {
                16
            } else {
                32
            }
        }
        "four_deck_layer" | "creative_mashup_layer" => {
            if mood_rhythm.is_electronic_club && out_vocal_act < 0.35 && in_vocal_act < 0.35 {
                64
            } else {
                32
            }
        }
        _ => 16,
    };

    // Respect remaining outgoing runway & incoming duration — dynamically shorten cleanly to musical beat multiples!
    let max_fit_sec = (outgoing.duration_sec - out_mix_out_sec)
        .max(remaining_sec * 0.5)
        .min(remaining_sec - 0.3)
        .min((incoming.duration_sec - in_mix_in_sec) * 0.45)
        .max(0.5);

    let allowed_lengths: [u32; 5] = [64, 32, 16, 8, 4];
    let duration_beats = allowed_lengths
        .iter()
        .copied()
        .find(|&b| b <= raw_beats && (b as f64) * beat_sec <= max_fit_sec)
        .unwrap_or(4);

    let duration_bars = (duration_beats / (beats_per_bar as u32).max(3)).max(1);
    let musical_duration_sec = (((duration_beats as f64) * beat_sec) * 100.0).round() / 100.0;

    let audio_crossfade_sec = if selected_technique == "safe_quick_cut" {
        (beat_sec * 0.70).clamp(0.25, 0.50)
    } else if selected_technique == "backspin_cut" {
        (beat_sec * 2.0).clamp(0.65, 1.35)
    } else {
        musical_duration_sec.min(max_fit_sec).max(0.5)
    };
    let duration_sec = audio_crossfade_sec; // For backwards-compatible audio execution

    // Snap scheduled execution time to nearest outgoing beatgrid downbeat when confident
    let mut scheduled_execution_sec = out_mix_out_sec
        .max(outgoing.current_pos_sec)
        .min((outgoing.duration_sec - duration_sec).max(outgoing.current_pos_sec));
    let mut beat_alignment_offset_sec = 0.0;

    if downbeat_confidence >= 0.50 || beatgrid_confidence >= 0.50 {
        if let Some(ref tp) = outgoing.timing_profile {
            if !tp.downbeat_timestamps_ms.is_empty() {
                let target_ms = (scheduled_execution_sec * 1000.0).round() as i64;
                if let Some(&nearest_ms) = tp
                    .downbeat_timestamps_ms
                    .iter()
                    .filter(|&&ms| (ms as f64 / 1000.0) >= outgoing.current_pos_sec)
                    .min_by_key(|&&ms| (ms - target_ms).abs())
                {
                    let snapped = nearest_ms as f64 / 1000.0;
                    if (snapped - scheduled_execution_sec).abs() <= beat_sec * 4.0
                        && snapped + duration_sec <= outgoing.duration_sec
                    {
                        beat_alignment_offset_sec = ((snapped - scheduled_execution_sec) * 1000.0).round() / 1000.0;
                        scheduled_execution_sec = snapped;
                    }
                }
            }
        }
    }

    // 7. EQ & Bass Handover Schedule tailored to vocal policy and technique
    let eq_bass_handover_schedule = EqBassHandoverSchedule {
        incoming_low_cut_until_frac: if selected_technique == "safe_quick_cut" { 0.0 } else { 0.48 },
        bass_swap_at_frac: match selected_technique.as_str() {
            "filter_sweep_drop" | "filtered_buildup_drop" | "beat_roll_buildup" | "stutter_gate_drop" => 0.86,
            "echo_freeze_drop" => 0.56,
            _ => 0.50,
        },
        outgoing_low_cut_at_frac: if vocal_clash { 0.40 } else { 0.50 },
        mid_carve_db: if vocal_clash { -10.0 } else if out_vocal_act >= 0.50 { -6.0 } else { -3.0 },
        high_handover_frac: 0.35,
    };

    reasons.push(format!(
        "Exit {} at {:.1}s ({}) → Enter {} at {:.1}s ({}) over {} beats ({} bars, {:.1}s) [crossfade: {:.2}s] via {}.",
        outgoing.title,
        scheduled_execution_sec,
        out_section_type,
        incoming.title,
        in_mix_in_sec,
        in_section_type,
        duration_beats,
        duration_bars,
        musical_duration_sec,
        audio_crossfade_sec,
        selected_technique
    ));

    let plan_confidence = (beatgrid_confidence * 0.45
        + (if meter_compatible { 0.25 } else { 0.05 })
        + (if !vocal_clash { 0.15 } else { 0.05 })
        + (key_compat * 0.15))
        .clamp(0.20, 0.98);

    TransitionDecisionPlan {
        status: "planned".to_string(),
        outgoing_track_id: outgoing.track_id.clone(),
        outgoing_deck_id: outgoing.deck_id,
        incoming_track_id: incoming.track_id.clone(),
        incoming_deck_id: incoming.deck_id,
        outgoing_mix_out_sec: (scheduled_execution_sec * 100.0).round() / 100.0,
        outgoing_phrase_boundary_sec: (out_mix_out_sec * 100.0).round() / 100.0,
        incoming_mix_in_sec: (in_mix_in_sec * 100.0).round() / 100.0,
        incoming_phrase_boundary_sec: (in_mix_in_sec * 100.0).round() / 100.0,
        outgoing_section_type: out_section_type,
        incoming_section_type: in_section_type,
        current_bpm: out_local_bpm,
        incoming_bpm: in_local_bpm,
        target_bpm: bpm_decision.target_bpm,
        bpm_relationship: bpm_decision.relationship,
        tempo_adjustment_pct: bpm_decision.adjustment_pct,
        tempo_direction: bpm_decision.direction,
        incoming_playback_rate: bpm_decision.incoming_playback_rate,
        outgoing_target_rate: bpm_decision.outgoing_target_rate,
        beat_alignment_offset_sec,
        meter_compatible,
        meter_summary,
        beatgrid_confidence,
        bpm_confidence,
        downbeat_confidence,
        meter_confidence,
        duration_beats,
        duration_bars,
        duration_sec: (duration_sec * 100.0).round() / 100.0,
        audio_crossfade_sec: (audio_crossfade_sec * 100.0).round() / 100.0,
        musical_duration_sec: (musical_duration_sec * 100.0).round() / 100.0,
        vocal_overlap_policy,
        energy_progression,
        eq_bass_handover_schedule,
        selected_technique,
        scheduled_execution_sec: (scheduled_execution_sec * 100.0).round() / 100.0,
        confidence: (plan_confidence * 100.0).round() / 100.0,
        reasons,
        rejected_alternatives,
        rejection_reason: None,
    }
}
