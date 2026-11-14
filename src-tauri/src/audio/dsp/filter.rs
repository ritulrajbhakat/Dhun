use crate::audio::dsp::biquad::{BiquadFilter, FilterType};

#[derive(Debug, Clone)]
pub struct DjFilterSweep {
    _sample_rate: f32,
    position: f32, // -1.0 (full LPF) to 0.0 (neutral bypass) to +1.0 (full HPF)
    lpf: BiquadFilter,
    hpf: BiquadFilter,
}

impl DjFilterSweep {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            _sample_rate: sample_rate,
            position: 0.0,
            lpf: BiquadFilter::new(FilterType::LowPass, sample_rate, 20000.0, 1.2, 0.0),
            hpf: BiquadFilter::new(FilterType::HighPass, sample_rate, 20.0, 1.2, 0.0),
        }
    }

    pub fn set_position(&mut self, pos: f32) {
        self.position = pos.clamp(-1.0, 1.0);
        if self.position < -0.02 {
            // Sweep LPF down from 20kHz to 200Hz
            let norm = (-self.position).clamp(0.0, 1.0);
            let cutoff = 20000.0 * 0.01f32.powf(norm); // Exponential curve down to 200Hz
            self.lpf.set_frequency(cutoff);
        } else if self.position > 0.02 {
            // Sweep HPF up from 20Hz to 6000Hz
            let norm = self.position.clamp(0.0, 1.0);
            let cutoff = 20.0 * 300.0f32.powf(norm); // Exponential curve up to 6000Hz
            self.hpf.set_frequency(cutoff);
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.position < -0.02 {
            self.lpf.process_sample(in_l, in_r)
        } else if self.position > 0.02 {
            self.hpf.process_sample(in_l, in_r)
        } else {
            (in_l, in_r)
        }
    }

    pub fn reset(&mut self) {
        self.lpf.reset();
        self.hpf.reset();
    }
}

