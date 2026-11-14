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

    pub fn max_decks(&self) -> usize {
        match self {
            DjLevel::Level1TwoDeckMixer | DjLevel::Level2SmartMixing => 2,
            DjLevel::Level3AdvancedPerformance | DjLevel::Level4CreativeTools => 4,
        }
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

