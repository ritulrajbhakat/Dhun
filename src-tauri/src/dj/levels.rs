use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DjLevel {
    Level1TwoDeckMixer = 1,
    Level2SmartMixing = 2,
    Level3AdvancedPerformance = 3,
    Level4CreativeTools = 4,
}

/// Complete, inheriting behavioral skill profile for each DJ level.
/// Higher levels strictly inherit all capabilities and techniques from lower levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjSkillProfile {
    pub level: i32,
    pub name: String,
    pub summary: String,
    pub exposed_decks: usize,
    pub internal_deck_capacity: usize,
    pub max_tempo_stretch_pct: f64,
    pub supports_auto_bpm_match: bool,
    pub supports_half_double_match: bool,
    pub supports_harmonic_key_match: bool,
    pub supports_gain_match: bool,
    pub supports_bass_swap: bool,
    pub supports_echo_out: bool,
    pub supports_beat_roll: bool,
    pub supports_four_deck_layering: bool,
    pub supports_stutter_gate: bool,
    pub supports_backspin_cut: bool,
    pub supports_echo_freeze: bool,
    pub supports_tempo_ramp: bool,
    pub supports_creative_mashup: bool,
    pub unlocked_techniques: Vec<String>,
}

impl DjLevel {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => DjLevel::Level1TwoDeckMixer,
            2 => DjLevel::Level2SmartMixing,
            3 => DjLevel::Level3AdvancedPerformance,
            4 => DjLevel::Level4CreativeTools,
            _ => DjLevel::Level2SmartMixing,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DjLevel::Level1TwoDeckMixer => "Professional 2-Deck Mixer",
            DjLevel::Level2SmartMixing => "Smart Mixing",
            DjLevel::Level3AdvancedPerformance => "Advanced Performance",
            DjLevel::Level4CreativeTools => "Creative DJ Tools",
        }
    }

    /// Number of decks exposed in the UI for this DJ level (2 for Levels 1-2, 4 for Levels 3-4).
    pub fn max_decks(&self) -> usize {
        match self {
            DjLevel::Level1TwoDeckMixer | DjLevel::Level2SmartMixing => 2,
            DjLevel::Level3AdvancedPerformance | DjLevel::Level4CreativeTools => 4,
        }
    }

    /// The underlying audio/DJ engine is always architected for 4-deck capability.
    pub fn internal_deck_capacity(&self) -> usize {
        4
    }

    /// All four DJ levels support automatic party playback once the host starts the party.
    pub fn supports_auto_dj(&self) -> bool {
        true
    }

    pub fn supports_auto_sync(&self) -> bool {
        *self >= DjLevel::Level2SmartMixing
    }

    pub fn supports_bass_swap(&self) -> bool {
        *self >= DjLevel::Level2SmartMixing
    }

    pub fn supports_performance_pads(&self) -> bool {
        *self >= DjLevel::Level3AdvancedPerformance
    }

    pub fn supports_creative_mashup(&self) -> bool {
        *self >= DjLevel::Level4CreativeTools
    }

    /// Returns the complete inheriting skill profile and audibly distinct techniques unlocked at this level.
    pub fn skill_profile(&self) -> DjSkillProfile {
        let mut techniques = vec![
            "safe_quick_cut".to_string(),
            "eq3_handover".to_string(),
            "phrase_long_blend".to_string(),
        ];

        if *self >= DjLevel::Level2SmartMixing {
            techniques.push("timed_bass_swap".to_string());
            techniques.push("filter_sweep_drop".to_string());
        }
        if *self >= DjLevel::Level3AdvancedPerformance {
            techniques.push("echo_out_blend".to_string());
            techniques.push("beat_roll_buildup".to_string());
            techniques.push("filtered_buildup_drop".to_string());
            techniques.push("four_deck_layer".to_string());
        }
        if *self >= DjLevel::Level4CreativeTools {
            techniques.push("stutter_gate_drop".to_string());
            techniques.push("backspin_cut".to_string());
            techniques.push("echo_freeze_drop".to_string());
            techniques.push("tempo_ramp_bridge".to_string());
            techniques.push("creative_mashup_layer".to_string());
        }

        let (summary, max_tempo_stretch_pct) = match self {
            DjLevel::Level1TwoDeckMixer => (
                "Clean 2-deck phrase blends, 3-band EQ handovers, gentle ±4% pitch assist, and safe cuts when beat grids are missing.",
                4.0,
            ),
            DjLevel::Level2SmartMixing => (
                "Inherits Level 1 + automatic BPM & half/double-time sync (±8%), RMS gain matching, Camelot harmonic selection, timed bass swaps, and filter sweeps.",
                8.0,
            ),
            DjLevel::Level3AdvancedPerformance => (
                "Inherits Levels 1–2 + 4-deck rhythmic layering, beat-synced echo-outs, beat-roll build-ups, and filtered drops.",
                10.0,
            ),
            DjLevel::Level4CreativeTools => (
                "Inherits Levels 1–3 + creative EQ-carved mashup layering, rhythmic stutter/gate drops, backspin brake cuts, echo freezes, and smooth tempo ramps.",
                14.0,
            ),
        };

        DjSkillProfile {
            level: *self as i32,
            name: self.display_name().to_string(),
            summary: summary.to_string(),
            exposed_decks: self.max_decks(),
            internal_deck_capacity: self.internal_deck_capacity(),
            max_tempo_stretch_pct,
            supports_auto_bpm_match: *self >= DjLevel::Level2SmartMixing,
            supports_half_double_match: *self >= DjLevel::Level2SmartMixing,
            supports_harmonic_key_match: *self >= DjLevel::Level2SmartMixing,
            supports_gain_match: *self >= DjLevel::Level2SmartMixing,
            supports_bass_swap: *self >= DjLevel::Level2SmartMixing,
            supports_echo_out: *self >= DjLevel::Level3AdvancedPerformance,
            supports_beat_roll: *self >= DjLevel::Level3AdvancedPerformance,
            supports_four_deck_layering: *self >= DjLevel::Level3AdvancedPerformance,
            supports_stutter_gate: *self >= DjLevel::Level4CreativeTools,
            supports_backspin_cut: *self >= DjLevel::Level4CreativeTools,
            supports_echo_freeze: *self >= DjLevel::Level4CreativeTools,
            supports_tempo_ramp: *self >= DjLevel::Level4CreativeTools,
            supports_creative_mashup: *self >= DjLevel::Level4CreativeTools,
            unlocked_techniques: techniques,
        }
    }
}
