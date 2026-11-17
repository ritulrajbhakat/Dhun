use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DjLevel {
    Level1TwoDeckMixer = 1,
    Level2SmartMixing = 2,
    Level3AdvancedPerformance = 3,
    Level4CreativeTools = 4,
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
}
