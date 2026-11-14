use serde::{Deserialize, Serialize};
use crate::audio::dsp::crossfader::CrossfaderCurve;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPlan {
    pub from_deck: usize,
    pub to_deck: usize,
    pub duration_ms: u64,
    pub duration_beats: u32,
    pub curve: CrossfaderCurve,
    pub smart_bass_swap: bool,
    pub filter_sweep: bool,
    pub scheduled_start_ms: u64,
}

impl TransitionPlan {
    pub fn default_phrase_swap(from_deck: usize, to_deck: usize, bpm: f64) -> Self {
        let beat_ms = (60.0 / bpm.max(60.0)) * 1000.0;
        let duration_beats = 32; // 8 bars = 32 beats
        let duration_ms = (duration_beats as f64 * beat_ms).round() as u64;

        Self {
            from_deck,
            to_deck,
            duration_ms,
            duration_beats,
            curve: CrossfaderCurve::BassSwap,
            smart_bass_swap: true,
            filter_sweep: false,
            scheduled_start_ms: 0,
        }
    }

    pub fn quick_cut(from_deck: usize, to_deck: usize) -> Self {
        Self {
            from_deck,
            to_deck,
            duration_ms: 250, // 250ms snappy cut
            duration_beats: 1,
            curve: CrossfaderCurve::SharpCut,
            smart_bass_swap: false,
            filter_sweep: false,
            scheduled_start_ms: 0,
        }
    }
}

