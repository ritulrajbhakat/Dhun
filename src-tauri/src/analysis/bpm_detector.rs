use std::sync::Arc;
use crate::db::models::{MeterAnalysis, TempoSegment};

pub struct BpmDetector;

impl BpmDetector {
    /// Internal helper: detects local BPM & confidence for a slice of interleaved stereo frames.
    fn detect_bpm_for_frame_range(
        samples: &[f32],
        start_frame: usize,
        end_frame: usize,
        sample_rate: u32,
    ) -> (f64, f64) {
        let frames = end_frame.saturating_sub(start_frame);
        let window_size = ((sample_rate as usize * 20) / 1000).max(1);
        let num_windows = (frames / window_size).min(3000);
        if num_windows < 80 {
            return (120.0, 0.25);
        }

        let mut envelope = Vec::with_capacity(num_windows);
        for w in 0..num_windows {
            let base = (start_frame + w * window_size) * 2;
            let mut sum_sq = 0.0f32;
            for i in 0..window_size {
                let idx = base + i * 2;
                if idx + 1 >= samples.len() {
                    break;
                }
                let l = samples[idx];
                let r = samples[idx + 1];
                sum_sq += l * l + r * r;
            }
            envelope.push((sum_sq / window_size as f32).sqrt());
        }

        let mut onsets = Vec::with_capacity(num_windows);
        for i in 1..num_windows {
            let diff = envelope[i] - envelope[i - 1];
            onsets.push(if diff > 0.0 { diff } else { 0.0 });
        }

        let min_bpm: f64 = 64.0;
        let max_bpm: f64 = 185.0;
        let fps: f64 = 50.0; // 20ms hop -> 50 Hz
        let min_lag = (fps * 60.0 / max_bpm).round() as usize;
        let max_lag = ((fps * 60.0 / min_bpm).round() as usize).min(onsets.len().saturating_sub(2));
        if max_lag <= min_lag {
            return (120.0, 0.25);
        }

        let mut best_lag = min_lag;
        let mut max_corr = 0.0f32;
        let mut total_corr = 0.0f32;
        let n = onsets.len();

        for lag in min_lag..=max_lag {
            let mut corr = 0.0f32;
            let limit = n - lag;
            for i in 0..limit {
                corr += onsets[i] * onsets[i + lag];
            }
            let norm_corr = if limit > 0 { corr / limit as f32 } else { 0.0 };
            total_corr += norm_corr;
            if norm_corr > max_corr {
                max_corr = norm_corr;
                best_lag = lag;
            }
        }

        let bpm: f64 = (fps * 60.0 / best_lag as f64).clamp(min_bpm, max_bpm);
        let avg_corr = total_corr / (max_lag - min_lag + 1) as f32;
        let confidence = if avg_corr > 1e-7 {
            (((max_corr / avg_corr - 1.0) / 3.0) as f64).clamp(0.15, 0.98)
        } else {
            0.20
        };

        ((bpm * 10.0).round() / 10.0, (confidence * 100.0).round() / 100.0)
    }

    /// Deterministic BPM detection using energy onset curve and autocorrelation
    pub fn detect_bpm(samples: &Arc<Vec<f32>>, sample_rate: u32) -> (f64, f64) {
        let total_frames = samples.len() / 2;
        if total_frames < sample_rate as usize * 5 {
            return (120.0, 0.3); // Fallback for very short snippets
        }
        let max_frames = total_frames.min(sample_rate as usize * 60);
        Self::detect_bpm_for_frame_range(samples, 0, max_frames, sample_rate)
    }

    /// Builds a local Tempo Map across the entire track to capture live percussion drift,
    /// Garba/Dandiya tempo acceleration, and multi-tempo Bollywood/club transitions.
    pub fn detect_tempo_map(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        duration_ms: u64,
    ) -> (f64, f64, Vec<TempoSegment>, f64, bool) {
        let total_frames = samples.len() / 2;
        if total_frames < sample_rate as usize * 8 || duration_ms < 8_000 {
            let (bpm, conf) = Self::detect_bpm(samples, sample_rate);
            return (
                bpm,
                conf,
                vec![TempoSegment {
                    start_ms: 0,
                    end_ms: duration_ms as i64,
                    bpm,
                    confidence: conf,
                }],
                0.0,
                false,
            );
        }

        let seg_sec = if duration_ms >= 60_000 { 16usize } else { 10usize };
        let seg_frames = sample_rate as usize * seg_sec;
        let mut segments = Vec::new();
        let mut cursor_frame = 0usize;

        while cursor_frame < total_frames {
            let mut end_frame = (cursor_frame + seg_frames).min(total_frames);
            // Fold tiny trailing remainder (< 6s) into current segment
            if total_frames.saturating_sub(end_frame) < sample_rate as usize * 6 {
                end_frame = total_frames;
            }
            let start_ms = ((cursor_frame as f64 / sample_rate as f64) * 1000.0).round() as i64;
            let end_ms = ((end_frame as f64 / sample_rate as f64) * 1000.0).round() as i64;
            let (bpm, conf) = Self::detect_bpm_for_frame_range(samples, cursor_frame, end_frame, sample_rate);
            segments.push(TempoSegment {
                start_ms,
                end_ms,
                bpm,
                confidence: conf,
            });
            cursor_frame = end_frame;
        }

        // Compute weighted average BPM & genuine tempo drift across confident windows
        let confident: Vec<&TempoSegment> = segments.iter().filter(|s| s.confidence >= 0.45).collect();
        let ref_pool: Vec<&TempoSegment> = if confident.is_empty() {
            segments.iter().collect()
        } else {
            confident
        };

        let mut bpms: Vec<f64> = ref_pool.iter().map(|s| s.bpm).collect();
        bpms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_bpm = bpms[bpms.len() / 2];

        // Normalize pure octave-flip outliers in individual windows while preserving real tempo shifts
        for seg in &mut segments {
            if (seg.bpm * 2.0 - median_bpm).abs() < 6.0 {
                seg.bpm = ((seg.bpm * 2.0) * 10.0).round() / 10.0;
            } else if (seg.bpm * 0.5 - median_bpm).abs() < 6.0 {
                seg.bpm = ((seg.bpm * 0.5) * 10.0).round() / 10.0;
            }
        }

        let mut min_bpm = f64::MAX;
        let mut max_bpm = 0.0f64;
        let mut w_sum = 0.0f64;
        let mut w_bpm = 0.0f64;
        let mut conf_sum = 0.0f64;

        for seg in &segments {
            let w = seg.confidence.max(0.1);
            w_sum += w;
            w_bpm += seg.bpm * w;
            conf_sum += seg.confidence;
            if seg.confidence >= 0.40 {
                min_bpm = min_bpm.min(seg.bpm);
                max_bpm = max_bpm.max(seg.bpm);
            }
        }

        let avg_bpm = if w_sum > 0.0 {
            ((w_bpm / w_sum) * 10.0).round() / 10.0
        } else {
            median_bpm
        };
        let avg_conf = ((conf_sum / segments.len().max(1) as f64) * 100.0).round() / 100.0;
        let drift_bpm = if min_bpm < f64::MAX {
            ((max_bpm - min_bpm) * 10.0).round() / 10.0
        } else {
            0.0
        };
        let is_variable_tempo = drift_bpm >= 3.5;

        (avg_bpm, avg_conf, segments, drift_bpm, is_variable_tempo)
    }

    /// Detects rhythmic meter grouping (4/4, 3/4, 6/8, or honest `"uncertain"` fallback)
    /// by measuring onset-accent periodicity across downbeat-aligned beat positions.
    pub fn detect_meter(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
    ) -> MeterAnalysis {
        let total_frames = samples.len() / 2;
        let clean_bpm = bpm.clamp(60.0, 200.0);
        let beat_ms = 60_000.0 / clean_bpm;
        let beat_frames = ((sample_rate as f64 * beat_ms) / 1000.0).round() as usize;
        let offset_frames = ((sample_rate as f64 * downbeat_offset_ms.max(0) as f64) / 1000.0).round() as usize;

        if beat_frames < 100 || total_frames <= offset_frames + beat_frames * 12 {
            return MeterAnalysis {
                beats_per_bar: 4,
                meter_label: "uncertain".to_string(),
                confidence: 0.30,
            };
        }

        // Measure transient/accent energy in a 60ms window at each beat (up to 96 beats)
        let probe_frames = ((sample_rate as usize * 60) / 1000).min(beat_frames / 2).max(1);
        let max_beats = ((total_frames - offset_frames) / beat_frames).min(96);
        let mut beat_accents = Vec::with_capacity(max_beats);

        for b in 0..max_beats {
            let start = offset_frames + b * beat_frames;
            let end = (start + probe_frames).min(total_frames);
            let mut sum_sq = 0.0f64;
            let mut n = 0usize;
            for f in start..end {
                let m = 0.5 * (samples[f * 2] as f64 + samples[f * 2 + 1] as f64);
                sum_sq += m * m;
                n += 1;
            }
            beat_accents.push(if n > 0 { (sum_sq / n as f64).sqrt() } else { 0.0 });
        }

        // Compute normalized autocorrelation of beat accent strengths at lags 3, 4, and 6
        let mean = beat_accents.iter().sum::<f64>() / beat_accents.len().max(1) as f64;
        let centered: Vec<f64> = beat_accents.iter().map(|v| v - mean).collect();
        let energy: f64 = centered.iter().map(|v| v * v).sum();
        if energy < 1e-6 {
            return MeterAnalysis {
                beats_per_bar: 4,
                meter_label: "uncertain".to_string(),
                confidence: 0.30,
            };
        }

        let autocorr = |lag: usize| -> f64 {
            if centered.len() <= lag {
                return 0.0;
            }
            let mut sum = 0.0;
            for i in 0..(centered.len() - lag) {
                sum += centered[i] * centered[i + lag];
            }
            sum / energy
        };

        let c3 = autocorr(3);
        let c4 = autocorr(4);
        let c6 = autocorr(6);

        // Distinguish 6/8 (compound duple: strong 6-beat cycle, weaker 3-beat sub-accent) vs pure 3/4 vs 4/4
        let (beats_per_bar, label, best_score, runner_up) = if c4 >= c3 && c4 >= c6 {
            (4u8, "4/4", c4, c3.max(c6))
        } else if c6 > c4 && c6 > c3 * 1.08 {
            (6u8, "6/8", c6, c3.max(c4))
        } else {
            (3u8, "3/4", c3, c4.max(c6))
        };

        let margin = (best_score - runner_up).max(0.0);
        let confidence = (0.35 + best_score.max(0.0) * 0.45 + margin * 0.75).clamp(0.20, 0.96);

        if best_score < 0.15 || confidence < 0.52 {
            MeterAnalysis {
                beats_per_bar: 4,
                meter_label: "uncertain".to_string(),
                confidence: (confidence * 100.0).round() / 100.0,
            }
        } else {
            MeterAnalysis {
                beats_per_bar,
                meter_label: label.to_string(),
                confidence: (confidence * 100.0).round() / 100.0,
            }
        }
    }

    /// Detects phase-aligned downbeat offset in milliseconds from audio onset energies
    pub fn detect_downbeat_offset(samples: &Arc<Vec<f32>>, sample_rate: u32, bpm: f64) -> i64 {
        let total_frames = samples.len() / 2;
        let beat_period_ms = 60000.0 / bpm.max(60.0);
        let beat_period_frames = ((sample_rate as f64 * beat_period_ms) / 1000.0) as usize;

        if total_frames < beat_period_frames || beat_period_frames == 0 {
            return 0;
        }

        // Subsample energy onset curve in 10ms steps across the first 4 bars (up to ~6s)
        let step_ms = 10;
        let step_frames = (sample_rate as usize * step_ms) / 1000;
        let max_search_ms = (beat_period_ms * 4.0).min(6000.0) as usize;
        let num_steps = (max_search_ms / step_ms).min(total_frames / step_frames.max(1));

        let mut envelope = Vec::with_capacity(num_steps);
        for s in 0..num_steps {
            let start = s * step_frames * 2;
            let mut sum_sq = 0.0f32;
            let chunk_size = step_frames.min((total_frames - s * step_frames).max(1));
            for i in 0..chunk_size {
                let l = samples[start + i * 2];
                let r = samples[start + i * 2 + 1];
                sum_sq += l * l + r * r;
            }
            envelope.push((sum_sq / chunk_size as f32).sqrt());
        }

        // Calculate onset peaks
        let mut best_step = 0;
        let mut max_onset = 0.0f32;
        for s in 1..num_steps {
            let diff = envelope[s] - envelope[s - 1];
            if diff > max_onset {
                max_onset = diff;
                best_step = s;
            }
        }

        let detected_offset_ms = (best_step * step_ms) as f64;
        let phase_aligned = (detected_offset_ms % beat_period_ms).round() as i64;
        phase_aligned.max(0)
    }
}
