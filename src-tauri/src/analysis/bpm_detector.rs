use std::sync::Arc;

pub struct BpmDetector;

impl BpmDetector {
    /// Deterministic BPM detection using energy onset curve and autocorrelation
    pub fn detect_bpm(samples: &Arc<Vec<f32>>, sample_rate: u32) -> (f64, f64) {
        let total_frames = samples.len() / 2;
        if total_frames < sample_rate as usize * 5 {
            return (120.0, 0.3); // Fallback for very short snippets
        }

        // Subsample energy envelopes in 20ms windows
        let window_size = (sample_rate as usize * 20) / 1000;
        let num_windows = (total_frames / window_size).min(3000); // Analyze up to 60 seconds
        let mut envelope = Vec::with_capacity(num_windows);

        for w in 0..num_windows {
            let start = w * window_size * 2;
            let mut sum_sq = 0.0f32;
            for i in 0..window_size {
                let l = samples[start + i * 2];
                let r = samples[start + i * 2 + 1];
                sum_sq += l * l + r * r;
            }
            envelope.push((sum_sq / window_size as f32).sqrt());
        }

        // Differentiate envelope to get onsets
        let mut onsets = Vec::with_capacity(num_windows);
        for i in 1..num_windows {
            let diff = envelope[i] - envelope[i - 1];
            onsets.push(if diff > 0.0 { diff } else { 0.0 });
        }

        // Autocorrelation over BPM range 70.0 to 180.0 BPM
        let min_bpm: f64 = 70.0;
        let max_bpm: f64 = 180.0;
        let fps: f64 = 50.0; // 50 frames per second
        let min_lag = (fps * 60.0 / max_bpm).round() as usize;
        let max_lag = (fps * 60.0 / min_bpm).round() as usize;

        let mut best_lag = min_lag;
        let mut max_corr = 0.0f32;
        let mut total_corr = 0.0f32;
        let n = onsets.len();

        for lag in min_lag..=max_lag {
            let mut corr = 0.0f32;
            for i in 0..(n - lag) {
                corr += onsets[i] * onsets[i + lag];
            }
            total_corr += corr;
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }

        let bpm: f64 = (fps * 60.0 / best_lag as f64).clamp(70.0, 180.0);
        
        // Confidence estimation: ratio of peak correlation to average correlation
        let avg_corr = total_corr / (max_lag - min_lag + 1) as f32;
        let confidence = if avg_corr > 0.0 {
            (((max_corr / avg_corr - 1.0) / 3.0) as f64).clamp(0.2, 0.98)
        } else {
            0.5
        };

        (bpm, confidence)
    }
}
