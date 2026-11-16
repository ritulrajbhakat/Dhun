use std::f32::consts::FRAC_PI_2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossfaderCurve {
    Linear,
    EqualPower,
    SharpCut,
    BassSwap,
}

#[derive(Debug, Clone)]
pub struct CrossfaderUnit {
    position: f32, // -1.0 (Deck A full), 0.0 (Center), +1.0 (Deck B full)
    curve: CrossfaderCurve,
    gain_a: f32,
    gain_b: f32,
    bass_attenuation_a_db: f32,
    bass_attenuation_b_db: f32,

    // Continuous smooth ramping
    target_position: f32,
    ramp_frames_remaining: u32,
    ramp_step: f32,
}

impl CrossfaderUnit {
    pub fn new() -> Self {
        let mut cf = Self {
            position: 0.0,
            curve: CrossfaderCurve::EqualPower,
            gain_a: 1.0,
            gain_b: 1.0,
            bass_attenuation_a_db: 0.0,
            bass_attenuation_b_db: 0.0,
            target_position: 0.0,
            ramp_frames_remaining: 0,
            ramp_step: 0.0,
        };
        cf.recalculate();
        cf
    }

    pub fn set_position(&mut self, pos: f32) {
        self.ramp_frames_remaining = 0;
        self.position = pos.clamp(-1.0, 1.0);
        self.target_position = self.position;
        self.recalculate();
    }

    pub fn set_curve(&mut self, curve: CrossfaderCurve) {
        self.curve = curve;
        self.recalculate();
    }

    /// Begins a continuous smooth transition ramp over duration_ms
    pub fn start_ramp(&mut self, target_pos: f32, duration_ms: u32, sample_rate: u32, curve: CrossfaderCurve) {
        self.curve = curve;
        let clamped_target = target_pos.clamp(-1.0, 1.0);
        if duration_ms == 0 || sample_rate == 0 {
            self.set_position(clamped_target);
            return;
        }

        let total_frames = ((duration_ms as f64 / 1000.0) * (sample_rate as f64)).round() as u32;
        self.ramp_frames_remaining = total_frames.max(1);
        self.target_position = clamped_target;
        self.ramp_step = (self.target_position - self.position) / (self.ramp_frames_remaining as f32);
    }

    /// Advance interpolation by one audio frame
    #[inline(always)]
    pub fn tick_frame(&mut self) {
        if self.ramp_frames_remaining > 0 {
            self.position += self.ramp_step;
            self.ramp_frames_remaining -= 1;
            if self.ramp_frames_remaining == 0 {
                self.position = self.target_position;
            }
            self.recalculate();
        }
    }

    pub fn position(&self) -> f32 {
        self.position
    }

    pub fn gains(&self) -> (f32, f32) {
        (self.gain_a, self.gain_b)
    }

    pub fn bass_attenuations_db(&self) -> (f32, f32) {
        (self.bass_attenuation_a_db, self.bass_attenuation_b_db)
    }

    fn recalculate(&mut self) {
        // Map position from [-1.0, 1.0] to [0.0, 1.0]
        let norm = (self.position + 1.0) * 0.5;

        match self.curve {
            CrossfaderCurve::Linear => {
                self.gain_a = (1.0 - norm).clamp(0.0, 1.0);
                self.gain_b = norm.clamp(0.0, 1.0);
                self.bass_attenuation_a_db = 0.0;
                self.bass_attenuation_b_db = 0.0;
            }
            CrossfaderCurve::EqualPower => {
                // Constant power: gain_a^2 + gain_b^2 = 1.0
                let angle = norm * FRAC_PI_2;
                self.gain_a = angle.cos();
                self.gain_b = angle.sin();
                self.bass_attenuation_a_db = 0.0;
                self.bass_attenuation_b_db = 0.0;
            }
            CrossfaderCurve::SharpCut => {
                // DJ Scratch cut: full volume almost instantly
                self.gain_a = if norm > 0.95 { 0.0 } else { 1.0 };
                self.gain_b = if norm < 0.05 { 0.0 } else { 1.0 };
                self.bass_attenuation_a_db = 0.0;
                self.bass_attenuation_b_db = 0.0;
            }
            CrossfaderCurve::BassSwap => {
                let angle = norm * FRAC_PI_2;
                self.gain_a = angle.cos();
                self.gain_b = angle.sin();

                // Smart Bass Swap:
                // When moving from A to B (norm > 0.5), sharply cut Deck A's low end by up to -36dB
                // While Deck A is dominant (norm <= 0.5), Deck B's bass is cut by up to -36dB
                if norm <= 0.45 {
                    self.bass_attenuation_a_db = 0.0;
                    self.bass_attenuation_b_db = -36.0;
                } else if norm >= 0.55 {
                    self.bass_attenuation_a_db = -36.0;
                    self.bass_attenuation_b_db = 0.0;
                } else {
                    // Smooth transition zone across the 10% midpoint
                    let mid_norm = (norm - 0.45) / 0.10; // 0.0 to 1.0
                    self.bass_attenuation_a_db = -36.0 * mid_norm;
                    self.bass_attenuation_b_db = -36.0 * (1.0 - mid_norm);
                }
            }
        }
    }
}

