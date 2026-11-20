use std::sync::Arc;
use crate::db::models::{MeterAnalysis, TempoSegment};

#[derive(Debug, Clone)]
pub struct RhythmAnalysisResult {
    pub kick_presence: f64,
    pub snare_clap_presence: f64,
    pub hihat_presence: f64,
    pub syncopation_index: f64,
    pub rhythmic_pattern_detected: String,
    pub rhythm_confidence: f64,
    pub alternative_bpms: Vec<f64>,
}

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
        let mut bpms: Vec<f64> = segments
            .iter()
            .filter(|s| s.confidence >= 0.45)
            .map(|s| s.bpm)
            .collect();
        if bpms.is_empty() {
            bpms = segments.iter().map(|s| s.bpm).collect();
        }
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

        let (global_bpm, global_bpm_conf) = Self::detect_bpm(samples, sample_rate);

        let avg_bpm = if w_sum > 0.0 {
            ((w_bpm / w_sum) * 10.0).round() / 10.0
        } else {
            median_bpm
        };

        // Compute separate, honest confidence metrics:
        // 1. Overall BPM confidence combines global 60s autocorrelation with confident segments
        let confident_confs: Vec<f64> = segments
            .iter()
            .filter(|s| s.confidence >= 0.45)
            .map(|s| s.confidence)
            .collect();
        let confident_conf_avg = if !confident_confs.is_empty() {
            confident_confs.iter().sum::<f64>() / confident_confs.len() as f64
        } else {
            conf_sum / segments.len().max(1) as f64
        };
        let bpm_confidence = ((global_bpm_conf * 0.70 + confident_conf_avg * 0.30) * 100.0).round() / 100.0;

        let drift_bpm = if min_bpm < f64::MAX {
            ((max_bpm - min_bpm) * 10.0).round() / 10.0
        } else {
            0.0
        };
        let is_variable_tempo = drift_bpm >= 3.5;

        (avg_bpm, bpm_confidence, segments, drift_bpm, is_variable_tempo)
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
                confidence: 0.35,
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
                confidence: 0.35,
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

        // Distinguish genuine 6/8 vs genuine 3/4 vs standard 4/4 (protecting syncopated 4/4 dance tracks):
        // In 4/4 syncopated rhythms (Dhol/Bhangra, Reggaeton, Funk, Latin tresillo), lag 3 has natural energy.
        // Therefore, 3/4 is ONLY declared when c3 strongly dominates c4 (c3 > c4 * 1.30 && c4 < 0.22).
        let (beats_per_bar, label, best_score, runner_up) = if c6 > c4 * 1.25 && c6 > c3 * 1.10 && c6 >= 0.22 {
            (6u8, "6/8", c6, c3.max(c4))
        } else if c3 > c4 * 1.30 && c4 < 0.22 && c3 >= 0.22 {
            (3u8, "3/4", c3, c4.max(c6))
        } else {
            (4u8, "4/4", c4.max(0.15), c3.max(c6))
        };

        let margin = (best_score - runner_up).max(0.0);
        let confidence = (0.40 + best_score.max(0.0) * 0.40 + margin * 0.70).clamp(0.25, 0.96);

        if best_score < 0.14 || confidence < 0.50 {
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

    /// Detects downbeat offset in milliseconds and confidence from onset peak-to-average ratio
    pub fn detect_downbeat(samples: &Arc<Vec<f32>>, sample_rate: u32, bpm: f64) -> (i64, f64) {
        let total_frames = samples.len() / 2;
        let beat_period_ms = 60000.0 / bpm.max(60.0);
        let beat_period_frames = ((sample_rate as f64 * beat_period_ms) / 1000.0) as usize;

        if total_frames < beat_period_frames || beat_period_frames == 0 {
            return (0, 0.40);
        }

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

        let mut best_step = 0;
        let mut max_onset = 0.0f32;
        let mut sum_onset = 0.0f32;
        let mut onset_count = 0usize;
        for s in 1..num_steps {
            let diff = (envelope[s] - envelope[s - 1]).max(0.0);
            sum_onset += diff;
            onset_count += 1;
            if diff > max_onset {
                max_onset = diff;
                best_step = s;
            }
        }

        let avg_onset = if onset_count > 0 { sum_onset / onset_count as f32 } else { 0.0 };
        let conf = if avg_onset > 1e-5 {
            (((max_onset / avg_onset - 1.0) / 2.5) as f64).clamp(0.25, 0.96)
        } else {
            0.45
        };

        let detected_offset_ms = (best_step * step_ms) as f64;
        let phase_aligned = (detected_offset_ms % beat_period_ms).round() as i64;
        (phase_aligned.max(0), ((conf * 100.0).round()) / 100.0)
    }

    /// Detects phase-aligned downbeat offset in milliseconds from audio onset energies
    pub fn detect_downbeat_offset(samples: &Arc<Vec<f32>>, sample_rate: u32, bpm: f64) -> i64 {
        Self::detect_downbeat(samples, sample_rate, bpm).0
    }

    /// Evaluates percussion characteristics (kick, snare, hi-hat presence, syncopation index)
    /// and classifies rhythmic groove patterns (including authentic Garba 3-Taali vs 4/4 EDM kicks,
    /// Dandiya stick transients, and Punjabi live dhol dagga/tilli signatures).
    pub fn detect_rhythm_and_percussion(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
        meter: &MeterAnalysis,
        _duration_ms: u64,
    ) -> RhythmAnalysisResult {
        let clean_bpm = bpm.clamp(60.0, 200.0);
        let alt_bpms = vec![
            ((clean_bpm * 0.5) * 10.0).round() / 10.0,
            ((clean_bpm * 2.0) * 10.0).round() / 10.0,
            ((clean_bpm * 0.75) * 10.0).round() / 10.0,
        ];

        let total_frames = samples.len() / 2;
        let sr = sample_rate.max(8000) as f64;
        let beat_ms = 60_000.0 / clean_bpm;
        let beat_frames = ((sr * beat_ms) / 1000.0).round() as usize;
        let offset_frames = ((sr * downbeat_offset_ms.max(0) as f64) / 1000.0).round() as usize;

        if beat_frames < 100 || total_frames <= offset_frames + beat_frames * 8 {
            return RhythmAnalysisResult {
                kick_presence: 0.60,
                snare_clap_presence: 0.55,
                hihat_presence: 0.50,
                syncopation_index: 0.20,
                rhythmic_pattern_detected: "unclassified".to_string(),
                rhythm_confidence: 0.40,
                alternative_bpms: alt_bpms,
            };
        }

        let dt = 1.0 / sr;
        let lp_90 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 90.0) + dt)).clamp(0.0005, 0.4);
        let lp_150 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 150.0) + dt)).clamp(0.001, 0.45);
        let lp_1500 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 1500.0) + dt)).clamp(0.01, 0.75);
        let lp_4000 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 4000.0) + dt)).clamp(0.02, 0.9);
        let lp_5000 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 5000.0) + dt)).clamp(0.02, 0.95);

        let max_beats = ((total_frames - offset_frames) / beat_frames).min(64);
        let probe_len = ((sr * 0.05).round() as usize).min(beat_frames / 3).max(1);

        let mut on_beat_low = 0.0f64;
        let mut on_beat_mid = 0.0f64;
        let mut on_beat_high = 0.0f64;
        let mut off_beat_mid = 0.0f64;
        let mut off_beat_high = 0.0f64;
        let mut dagga_low = 0.0f64;
        let mut tilli_high = 0.0f64;
        let mut beat_count = 0usize;

        for b in 0..max_beats {
            let beat_start = offset_frames + b * beat_frames;
            let half_beat = beat_start + beat_frames / 2;

            if beat_start + probe_len >= total_frames || half_beat + probe_len >= total_frames {
                break;
            }

            let mut s90 = 0.0f64;
            let mut s150 = 0.0f64;
            let mut s1500 = 0.0f64;
            let mut s4000 = 0.0f64;
            let mut s5000 = 0.0f64;

            let mut low_acc = 0.0f64;
            let mut mid_acc = 0.0f64;
            let mut high_acc = 0.0f64;
            let mut dagga_acc = 0.0f64;
            let mut tilli_acc = 0.0f64;

            for f in beat_start..(beat_start + probe_len) {
                let m = 0.5 * (samples[f * 2] as f64 + samples[f * 2 + 1] as f64);
                s90 += lp_90 * (m - s90);
                s150 += lp_150 * (m - s150);
                s1500 += lp_1500 * (m - s1500);
                s4000 += lp_4000 * (m - s4000);
                s5000 += lp_5000 * (m - s5000);

                let tilli_band = s4000 - s1500;
                let hi_band = m - s5000;

                low_acc += s150 * s150;
                mid_acc += tilli_band * tilli_band;
                high_acc += hi_band * hi_band;
                dagga_acc += s90 * s90;
                tilli_acc += tilli_band * tilli_band;
            }

            on_beat_low += (low_acc / probe_len as f64).sqrt();
            on_beat_mid += (mid_acc / probe_len as f64).sqrt();
            on_beat_high += (high_acc / probe_len as f64).sqrt();
            dagga_low += (dagga_acc / probe_len as f64).sqrt();
            tilli_high += (tilli_acc / probe_len as f64).sqrt();

            let mut off_s1500 = 0.0f64;
            let mut off_s4000 = 0.0f64;
            let mut off_s5000 = 0.0f64;
            let mut off_mid_acc = 0.0f64;
            let mut off_high_acc = 0.0f64;

            for f in half_beat..(half_beat + probe_len) {
                let m = 0.5 * (samples[f * 2] as f64 + samples[f * 2 + 1] as f64);
                off_s1500 += lp_1500 * (m - off_s1500);
                off_s4000 += lp_4000 * (m - off_s4000);
                off_s5000 += lp_5000 * (m - off_s5000);

                let tilli_band = off_s4000 - off_s1500;
                let hi_band = m - off_s5000;

                off_mid_acc += tilli_band * tilli_band;
                off_high_acc += hi_band * hi_band;
            }

            off_beat_mid += (off_mid_acc / probe_len as f64).sqrt();
            off_beat_high += (off_high_acc / probe_len as f64).sqrt();
            beat_count += 1;
        }

        let bc = beat_count.max(1) as f64;
        let avg_on_low = on_beat_low / bc;
        let avg_on_mid = on_beat_mid / bc;
        let avg_on_high = on_beat_high / bc;
        let avg_off_mid = off_beat_mid / bc;
        let avg_off_high = off_beat_high / bc;
        let avg_dagga = dagga_low / bc;
        let avg_tilli = tilli_high / bc;

        let kick_presence = (avg_on_low * 4.5).clamp(0.10, 0.98);
        let snare_clap_presence = (avg_on_mid * 4.2).clamp(0.10, 0.98);
        let hihat_presence = ((avg_on_high + avg_off_high) * 3.8).clamp(0.10, 0.98);

        let total_energy = avg_on_low + avg_on_mid + avg_off_mid + 1e-6;
        let syncopation_index = ((avg_off_mid * 1.8) / total_energy).clamp(0.05, 0.85);

        let is_triple_meter = meter.beats_per_bar == 3 || meter.beats_per_bar == 6 || meter.meter_label == "3/4" || meter.meter_label == "6/8";

        let (pattern, conf) = if is_triple_meter && meter.confidence >= 0.50 {
            ("authentic_3_taali", (0.75 + meter.confidence * 0.20).clamp(0.65, 0.95))
        } else if clean_bpm >= 122.0 && clean_bpm <= 146.0 && avg_on_high >= 0.08 && avg_off_high >= 0.06 {
            ("sharp_dandiya_sticks", 0.85)
        } else if avg_dagga >= 0.045 && avg_tilli >= 0.040 && syncopation_index >= 0.32 {
            ("live_punjabi_dhol", 0.88)
        } else if kick_presence >= 0.70 && syncopation_index <= 0.25 && meter.meter_label == "4/4" {
            ("four_on_floor", 0.86)
        } else if syncopation_index >= 0.42 {
            ("syncopated_dembow", 0.80)
        } else if kick_presence >= 0.40 && snare_clap_presence >= 0.35 {
            ("standard_pop_groove", 0.78)
        } else {
            ("unclassified", 0.50)
        };

        RhythmAnalysisResult {
            kick_presence: (kick_presence * 100.0).round() / 100.0,
            snare_clap_presence: (snare_clap_presence * 100.0).round() / 100.0,
            hihat_presence: (hihat_presence * 100.0).round() / 100.0,
            syncopation_index: (syncopation_index * 100.0).round() / 100.0,
            rhythmic_pattern_detected: pattern.to_string(),
            rhythm_confidence: (conf * 100.0).round() / 100.0,
            alternative_bpms: alt_bpms,
        }
    }
}
