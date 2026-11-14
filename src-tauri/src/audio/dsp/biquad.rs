use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
    LowPass,
    HighPass,
}

#[derive(Debug, Clone)]
pub struct BiquadFilter {
    filter_type: FilterType,
    sample_rate: f32,
    freq_hz: f32,
    q: f32,
    gain_db: f32,
    
    // Normalized coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    
    // Direct Form II Transposed delay registers (Left & Right channels)
    d1_l: f32,
    d2_l: f32,
    d1_r: f32,
    d2_r: f32,
}

impl BiquadFilter {
    pub fn new(filter_type: FilterType, sample_rate: f32, freq_hz: f32, q: f32, gain_db: f32) -> Self {
        let mut filter = Self {
            filter_type,
            sample_rate,
            freq_hz,
            q: q.max(0.1),
            gain_db,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            d1_l: 0.0,
            d2_l: 0.0,
            d1_r: 0.0,
            d2_r: 0.0,
        };
        filter.recalculate();
        filter
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        if (self.gain_db - gain_db).abs() > 0.01 {
            self.gain_db = gain_db;
            self.recalculate();
        }
    }

    pub fn set_frequency(&mut self, freq_hz: f32) {
        let clamped = freq_hz.clamp(20.0, self.sample_rate * 0.49);
        if (self.freq_hz - clamped).abs() > 0.1 {
            self.freq_hz = clamped;
            self.recalculate();
        }
    }

    pub fn recalculate(&mut self) {
        let a = 10.0f32.powf(self.gain_db / 40.0);
        let w0 = 2.0 * PI * (self.freq_hz / self.sample_rate);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    #[inline(always)]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Direct Form II Transposed for Left channel
        let out_l = self.b0 * in_l + self.d1_l;
        self.d1_l = self.b1 * in_l - self.a1 * out_l + self.d2_l;
        self.d2_l = self.b2 * in_l - self.a2 * out_l;

        // Direct Form II Transposed for Right channel
        let out_r = self.b0 * in_r + self.d1_r;
        self.d1_r = self.b1 * in_r - self.a1 * out_r + self.d2_r;
        self.d2_r = self.b2 * in_r - self.a2 * out_r;

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.d1_l = 0.0;
        self.d2_l = 0.0;
        self.d1_r = 0.0;
        self.d2_r = 0.0;
    }
}

