#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioClock {
    sample_rate: u32,
    total_samples_rendered: u64,
}

impl AudioClock {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            total_samples_rendered: 0,
        }
    }

    #[inline(always)]
    pub fn tick(&mut self, frames: usize) {
        self.total_samples_rendered += frames as u64;
    }

    pub fn total_samples(&self) -> u64 {
        self.total_samples_rendered
    }

    pub fn elapsed_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.total_samples_rendered as f64 / self.sample_rate as f64
        }
    }
}

