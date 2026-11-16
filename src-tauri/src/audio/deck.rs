use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::audio::dsp::eq3::Eq3Isolator;
use crate::audio::dsp::filter::DjFilterSweep;
use crate::audio::telemetry::DeckTelemetry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Stopped,
    Paused,
    Playing,
    Cueing,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VirtualDeck {
    pub id: usize, // 0 = Deck A, 1 = Deck B, 2 = Deck C, 3 = Deck D
    pub state: TransportState,
    pub track_id: Option<String>,
    
    // Decoded audio PCM stereo buffer (interleaved L, R, L, R...)
    samples: Option<Arc<Vec<f32>>>,
    pub sample_rate: u32,
    pub output_sample_rate: u32,
    
    // Playhead position in stereo frames (1 frame = 1 L + 1 R)
    pub cursor_frame: f64,
    pub cue_frame: u64,
    pub hot_cues: [Option<u64>; 8],
    
    // Pitch & Tempo
    pub tempo_rate: f64, // 1.0 = normal, 1.05 = +5% faster
    pub bpm: f64,
    pub target_bpm: f64,
    
    // Looping
    pub loop_enabled: bool,
    pub loop_start_frame: u64,
    pub loop_end_frame: u64,
    
    // DSP & Channel Strip
    pub gain: f32, // 0.0 to 1.5
    pub eq: Eq3Isolator,
    pub filter: DjFilterSweep,
    
    // Scratching / Jog wheel
    pub scratch_rate: f64,
    pub is_scratching: bool,
}

impl VirtualDeck {
    pub fn new(id: usize, output_sample_rate: u32) -> Self {
        Self {
            id,
            state: TransportState::Stopped,
            track_id: None,
            samples: None,
            sample_rate: output_sample_rate,
            output_sample_rate,
            cursor_frame: 0.0,
            cue_frame: 0,
            hot_cues: [None; 8],
            tempo_rate: 1.0,
            bpm: 120.0,
            target_bpm: 120.0,
            loop_enabled: false,
            loop_start_frame: 0,
            loop_end_frame: 0,
            gain: 1.0,
            eq: Eq3Isolator::new(output_sample_rate as f32),
            filter: DjFilterSweep::new(output_sample_rate as f32),
            scratch_rate: 0.0,
            is_scratching: false,
        }
    }

    pub fn load_audio(&mut self, track_id: String, samples: Arc<Vec<f32>>, sample_rate: u32, bpm: f64) {
        self.track_id = Some(track_id);
        self.samples = Some(samples);
        self.sample_rate = sample_rate;
        self.cursor_frame = 0.0;
        self.cue_frame = 0;
        self.bpm = bpm;
        self.target_bpm = bpm;
        self.tempo_rate = 1.0;
        self.state = TransportState::Paused;
    }

    pub fn play(&mut self) {
        if self.samples.is_some() {
            self.state = TransportState::Playing;
        }
    }

    pub fn pause(&mut self) {
        self.state = TransportState::Paused;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.cursor_frame = self.cue_frame as f64;
    }

    pub fn set_cue(&mut self) {
        self.cue_frame = self.cursor_frame.round() as u64;
    }

    pub fn jump_to_cue(&mut self) {
        self.cursor_frame = self.cue_frame as f64;
        self.state = TransportState::Paused;
    }

    pub fn set_hot_cue(&mut self, index: usize) {
        if index < 8 {
            self.hot_cues[index] = Some(self.cursor_frame.round() as u64);
        }
    }

    pub fn jump_to_hot_cue(&mut self, index: usize) {
        if index < 8 {
            if let Some(frame) = self.hot_cues[index] {
                self.cursor_frame = frame as f64;
            }
        }
    }

    pub fn set_tempo_rate(&mut self, rate: f64) {
        self.tempo_rate = rate.clamp(0.5, 2.0);
    }

    pub fn seek_to_ms(&mut self, ms: u64) {
        let frame = (ms as f64 / 1000.0) * (self.sample_rate as f64);
        if let Some(buf) = &self.samples {
            let max_frames = (buf.len() / 2) as f64;
            self.cursor_frame = frame.clamp(0.0, max_frames - 1.0);
        }
    }

    pub fn current_position_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            0
        } else {
            ((self.cursor_frame / self.sample_rate as f64) * 1000.0).round() as u64
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match &self.samples {
            Some(buf) => {
                let total_frames = buf.len() / 2;
                ((total_frames as f64 / self.sample_rate as f64) * 1000.0).round() as u64
            }
            None => 0,
        }
    }

    pub fn telemetry(&self) -> DeckTelemetry {
        let to_db = |lin: f32| -> f32 {
            if lin <= 0.00001 {
                -96.0
            } else {
                (20.0 * lin.log10()).max(-96.0)
            }
        };

        DeckTelemetry {
            deck_id: self.id,
            track_id: self.track_id.clone(),
            position_ms: self.current_position_ms(),
            duration_ms: self.duration_ms(),
            bpm: self.bpm,
            tempo_rate: self.tempo_rate,
            is_playing: self.state == TransportState::Playing,
            vu_l_db: to_db(self.gain),
            vu_r_db: to_db(self.gain),
        }
    }

    /// Pull the next stereo frame from this deck, applying gain, EQ, and filter
    #[inline(always)]
    pub fn next_frame(&mut self) -> (f32, f32) {
        if self.state != TransportState::Playing || self.samples.is_none() {
            return (0.0, 0.0);
        }

        let samples = self.samples.as_ref().unwrap();
        let total_frames = samples.len() / 2;
        if total_frames == 0 {
            return (0.0, 0.0);
        }

        let idx0 = self.cursor_frame.floor() as usize;
        let frac = (self.cursor_frame - idx0 as f64) as f32;

        if idx0 >= total_frames - 1 {
            self.state = TransportState::Stopped;
            return (0.0, 0.0);
        }

        // Linear interpolation between consecutive frames
        let s0_l = samples[idx0 * 2];
        let s0_r = samples[idx0 * 2 + 1];
        let s1_l = samples[(idx0 + 1) * 2];
        let s1_r = samples[(idx0 + 1) * 2 + 1];

        let raw_l = s0_l + frac * (s1_l - s0_l);
        let raw_r = s0_r + frac * (s1_r - s0_r);

        // Resample ratio: converts source sample rate (e.g. 44100) to output hardware sample rate (e.g. 48000)
        let resample_ratio = if self.output_sample_rate > 0 && self.sample_rate > 0 {
            self.sample_rate as f64 / self.output_sample_rate as f64
        } else {
            1.0
        };

        // Advance cursor based on tempo rate or scratch rate, scaled by resampling ratio
        let base_rate = if self.is_scratching {
            self.scratch_rate
        } else {
            self.tempo_rate
        };
        self.cursor_frame += base_rate * resample_ratio;

        // Loop check
        if self.loop_enabled && self.loop_end_frame > self.loop_start_frame {
            if self.cursor_frame >= self.loop_end_frame as f64 {
                self.cursor_frame = self.loop_start_frame as f64;
            }
        }

        // Apply channel Gain, 3-Band EQ, and Filter Sweep
        let gained_l = raw_l * self.gain;
        let gained_r = raw_r * self.gain;

        let (eq_l, eq_r) = self.eq.process_sample(gained_l, gained_r);
        self.filter.process_sample(eq_l, eq_r)
    }
}

