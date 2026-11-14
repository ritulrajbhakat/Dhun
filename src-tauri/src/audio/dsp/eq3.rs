use crate::audio::dsp::biquad::{BiquadFilter, FilterType};

#[derive(Debug, Clone)]
pub struct Eq3Isolator {
    low_shelf: BiquadFilter,
    mid_peak: BiquadFilter,
    high_shelf: BiquadFilter,
    
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
    
    pub low_kill: bool,
    pub mid_kill: bool,
    pub high_kill: bool,
}

impl Eq3Isolator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            low_shelf: BiquadFilter::new(FilterType::LowShelf, sample_rate, 250.0, 0.707, 0.0),
            mid_peak: BiquadFilter::new(FilterType::Peaking, sample_rate, 1000.0, 0.9, 0.0),
            high_shelf: BiquadFilter::new(FilterType::HighShelf, sample_rate, 4000.0, 0.707, 0.0),
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
            low_kill: false,
            mid_kill: false,
            high_kill: false,
        }
    }

    pub fn set_gains(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        self.low_db = low_db.clamp(-48.0, 6.0);
        self.mid_db = mid_db.clamp(-48.0, 6.0);
        self.high_db = high_db.clamp(-48.0, 6.0);

        let effective_low = if self.low_kill { -96.0 } else { self.low_db };
        let effective_mid = if self.mid_kill { -96.0 } else { self.mid_db };
        let effective_high = if self.high_kill { -96.0 } else { self.high_db };

        self.low_shelf.set_gain_db(effective_low);
        self.mid_peak.set_gain_db(effective_mid);
        self.high_shelf.set_gain_db(effective_high);
    }

    pub fn set_kills(&mut self, low_kill: bool, mid_kill: bool, high_kill: bool) {
        self.low_kill = low_kill;
        self.mid_kill = mid_kill;
        self.high_kill = high_kill;
        self.set_gains(self.low_db, self.mid_db, self.high_db);
    }

    #[inline(always)]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let (l1, r1) = self.low_shelf.process_sample(in_l, in_r);
        let (l2, r2) = self.mid_peak.process_sample(l1, r1);
        self.high_shelf.process_sample(l2, r2)
    }

    pub fn reset(&mut self) {
        self.low_shelf.reset();
        self.mid_peak.reset();
        self.high_shelf.reset();
    }
}

