#[derive(Debug, Clone)]
pub struct LookaheadLimiter {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    buffer_pos: usize,
    lookahead_samples: usize,
    
    threshold_linear: f32,
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
    
    _underrun_count: u64,
}

impl LookaheadLimiter {
    pub fn new(sample_rate: f32, lookahead_ms: f32, threshold_db: f32) -> Self {
        let lookahead_samples = ((sample_rate * lookahead_ms / 1000.0).round() as usize).max(8);
        let threshold_linear = 10.0f32.powf(threshold_db / 20.0);
        
        let attack_time_sec = 0.0005; // 0.5ms
        let release_time_sec = 0.050; // 50ms
        
        let attack_coeff = (-1.0 / (sample_rate * attack_time_sec)).exp();
        let release_coeff = (-1.0 / (sample_rate * release_time_sec)).exp();

        Self {
            buffer_l: vec![0.0; lookahead_samples],
            buffer_r: vec![0.0; lookahead_samples],
            buffer_pos: 0,
            lookahead_samples,
            threshold_linear,
            envelope: 0.0,
            attack_coeff,
            release_coeff,
            _underrun_count: 0,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Read the delayed lookahead sample
        let delayed_l = self.buffer_l[self.buffer_pos];
        let delayed_r = self.buffer_r[self.buffer_pos];

        // Store new incoming sample
        self.buffer_l[self.buffer_pos] = in_l;
        self.buffer_r[self.buffer_pos] = in_r;

        self.buffer_pos = (self.buffer_pos + 1) % self.lookahead_samples;

        // Instantaneous peak across both channels
        let peak = in_l.abs().max(in_r.abs());

        // Envelope follower with fast attack, smooth release
        if peak > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * peak;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * peak;
        }

        // Calculate gain reduction
        let gain = if self.envelope > self.threshold_linear {
            self.threshold_linear / self.envelope
        } else {
            1.0
        };

        // Apply gain to delayed sample and hard safety clip at 0.995 (-0.05 dBFS)
        let out_l = (delayed_l * gain).clamp(-0.995, 0.995);
        let out_r = (delayed_r * gain).clamp(-0.995, 0.995);

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.buffer_pos = 0;
        self.envelope = 0.0;
    }
}

