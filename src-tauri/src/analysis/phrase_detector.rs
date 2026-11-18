use std::sync::Arc;
use crate::db::models::PhraseMarker;

pub struct PhraseDetector;

impl PhraseDetector {
    /// Deterministic bar- and phrase-aligned DSP section analyzer.
    ///
    /// Measures actual per-phrase RMS energy, sub-200 Hz bass energy (1-pole low-pass),
    /// and phrase-to-phrase energy contrast aligned to real beat/bar boundaries.
    /// Assigns honest confidence scores (0.0–1.0) and falls back to `"phrase"` when
    /// energy contrast is too flat to distinguish structural sections reliably.
    pub fn analyze_phrasing_and_energy(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        duration_ms: u64,
    ) -> (Vec<PhraseMarker>, f64, f64, i64, i64) {
        Self::analyze_phrasing_with_downbeat(samples, sample_rate, bpm, 0, duration_ms)
    }

    pub fn analyze_phrasing_with_downbeat(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
        duration_ms: u64,
    ) -> (Vec<PhraseMarker>, f64, f64, i64, i64) {
        let sr = sample_rate.max(8000) as f64;
        let clean_bpm = bpm.clamp(60.0, 200.0);
        let beat_ms = (60.0 / clean_bpm) * 1000.0;
        let bar_ms = beat_ms * 4.0;
        // Use 8-bar phrases for standard tracks (>= 45s) and 4-bar phrases for shorter tracks
        let phrase_bars = if duration_ms >= 45_000 { 8.0 } else { 4.0 };
        let phrase_ms = (bar_ms * phrase_bars).max(2_000.0);

        let total_frames = samples.len() / 2;
        if total_frames == 0 || duration_ms == 0 {
            return (Vec::new(), 0.5, 0.5, 0, 0);
        }

        // Compute global RMS energy
        let global_step = (total_frames / 4000).max(1);
        let mut global_sum_sq = 0.0f64;
        let mut global_count = 0usize;
        for i in (0..total_frames).step_by(global_step) {
            let mono = 0.5 * (samples[i * 2] as f64 + samples[i * 2 + 1] as f64);
            global_sum_sq += mono * mono;
            global_count += 1;
        }
        let global_rms = if global_count > 0 {
            (global_sum_sq / global_count as f64).sqrt()
        } else {
            0.2
        };
        let overall_energy = (global_rms * 3.5).clamp(0.15, 0.98);

        // 1-pole low-pass coefficient at ~200 Hz for deterministic bass-band energy measurement
        let dt = 1.0 / sr;
        let rc = 1.0 / (2.0 * std::f64::consts::PI * 200.0);
        let lp_alpha = (dt / (rc + dt)).clamp(0.001, 0.5);

        // Measure per-phrase RMS and bass-band RMS aligned to downbeat_offset_ms
        let start_anchor_ms = downbeat_offset_ms.clamp(0, (bar_ms as i64).min(duration_ms as i64 / 4));
        let mut raw_windows: Vec<(i64, i64, f64, f64)> = Vec::new();
        let mut cursor_ms = start_anchor_ms as f64;
        if start_anchor_ms > 500 {
            // Preserve pre-downbeat pickup as part of opening window start
            cursor_ms = 0.0;
        }

        while (cursor_ms as u64) < duration_ms {
            let next_ms = if cursor_ms == 0.0 && start_anchor_ms > 500 {
                start_anchor_ms as f64 + phrase_ms
            } else {
                cursor_ms + phrase_ms
            };
            let end_ms = next_ms.min(duration_ms as f64);
            // Merge very tiny trailing stubs (< 2 bars) into previous phrase
            if (end_ms - cursor_ms) < bar_ms * 2.0 && !raw_windows.is_empty() {
                if let Some(last) = raw_windows.last_mut() {
                    last.1 = duration_ms as i64;
                }
                break;
            }

            let start_frame = ((cursor_ms / 1000.0) * sr).round() as usize;
            let end_frame = (((end_ms / 1000.0) * sr).round() as usize).min(total_frames);
            if end_frame <= start_frame {
                break;
            }

            let frames_in_win = end_frame - start_frame;
            // Subsample lightly for very long windows while keeping bass LPF coherent
            let stride = if frames_in_win > 120_000 { 4 } else { 1 };
            let eff_alpha = (lp_alpha * stride as f64).min(0.8);
            let mut sum_sq = 0.0f64;
            let mut bass_sq = 0.0f64;
            let mut lp_state = 0.0f64;
            let mut n = 0usize;

            let mut idx = start_frame;
            while idx < end_frame {
                let mono = 0.5 * (samples[idx * 2] as f64 + samples[idx * 2 + 1] as f64);
                lp_state += eff_alpha * (mono - lp_state);
                sum_sq += mono * mono;
                bass_sq += lp_state * lp_state;
                n += 1;
                idx += stride;
            }

            let win_rms = if n > 0 { (sum_sq / n as f64).sqrt() } else { 0.0 };
            let win_bass = if n > 0 { (bass_sq / n as f64).sqrt() } else { 0.0 };
            raw_windows.push((cursor_ms.round() as i64, end_ms.round() as i64, win_rms, win_bass));
            cursor_ms = end_ms;
        }

        if raw_windows.is_empty() {
            return (Vec::new(), overall_energy, (overall_energy * 0.9 + 0.1).clamp(0.1, 0.99), 0, duration_ms as i64);
        }

        // Compute dynamic range & quartiles across phrases for honest structural classification
        let mut energies: Vec<f64> = raw_windows.iter().map(|w| w.2).collect();
        energies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min_e = energies.first().copied().unwrap_or(0.0);
        let max_e = energies.last().copied().unwrap_or(0.001).max(0.001);
        let median_e = energies[energies.len() / 2].max(0.001);
        let dynamic_spread = ((max_e - min_e) / max_e).clamp(0.0, 1.0);

        let mut basses: Vec<f64> = raw_windows.iter().map(|w| w.3).collect();
        basses.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_bass = basses[basses.len() / 2].max(0.0005);

        let count = raw_windows.len();
        let mut phrases: Vec<PhraseMarker> = Vec::with_capacity(count);

        for i in 0..count {
            let (s_ms, e_ms, rms, bass) = raw_windows[i];
            let prev_rms = if i > 0 { raw_windows[i - 1].2 } else { rms };
            let next_rms = if i + 1 < count { raw_windows[i + 1].2 } else { rms };
            let rel_e = rms / median_e;
            let rel_bass = bass / median_bass;
            let norm_e = (rms * 3.5).clamp(0.05, 0.99);
            let norm_bass = (bass * 4.2).clamp(0.02, 0.99);

            // If energy contrast across the whole track is flat (< 12% spread), stay honest:
            // label generic beat/phrase-aligned "phrase" with low structural confidence.
            let (section_type, confidence) = if dynamic_spread < 0.12 && count > 2 {
                if i == 0 {
                    ("intro", 0.45)
                } else if i == count - 1 {
                    ("outro", 0.45)
                } else {
                    ("phrase", 0.35)
                }
            } else if i == 0 && rel_e < 0.95 {
                ("intro", (0.65 + (1.0 - rel_e).clamp(0.0, 0.3)).clamp(0.55, 0.95))
            } else if i == count - 1 && rel_e < 0.92 {
                ("outro", (0.65 + (1.0 - rel_e).clamp(0.0, 0.3)).clamp(0.55, 0.95))
            } else if i > 0 && (rms - prev_rms) / median_e >= 0.28 && rel_bass >= 0.95 {
                // Sharp upward energy jump with strong bass = deterministic drop!
                let jump = ((rms - prev_rms) / median_e).clamp(0.0, 0.8);
                ("drop", (0.65 + jump * 0.4).clamp(0.60, 0.95))
            } else if i + 1 < count && (next_rms - rms) / median_e >= 0.22 && rel_e >= 0.55 {
                // Rising energy phrase right before a high-energy section = build!
                let rise = ((next_rms - rms) / median_e).clamp(0.0, 0.7);
                ("build", (0.60 + rise * 0.45).clamp(0.58, 0.92))
            } else if rel_e < 0.72 || (rel_bass < 0.62 && rel_e < 0.88) {
                // Significant dip in energy and/or bass = breakdown!
                let dip = (1.0 - rel_e).clamp(0.0, 0.6);
                ("breakdown", (0.62 + dip * 0.5).clamp(0.58, 0.92))
            } else if rel_e >= 1.08 && rel_bass >= 0.90 {
                // Sustained top energy + bass = peak / chorus!
                let lift = (rel_e - 1.0).clamp(0.0, 0.5);
                ("peak", (0.64 + lift * 0.55).clamp(0.60, 0.94))
            } else {
                // Steady danceable body = groove
                ("groove", (0.58 + dynamic_spread * 0.25).clamp(0.50, 0.85))
            };

            phrases.push(PhraseMarker {
                section_type: section_type.to_string(),
                start_ms: s_ms,
                end_ms: e_ms,
                energy_level: (norm_e * 1000.0).round() / 1000.0,
                bass_energy: (norm_bass * 1000.0).round() / 1000.0,
                confidence: (confidence * 100.0).round() / 100.0,
            });
        }

        // Dance suitability reflects both overall energy and bass-presence steadiness
        let bass_ratio = (median_bass / median_e).clamp(0.1, 1.0);
        let dance_suitability = (overall_energy * 0.72 + bass_ratio * 0.28).clamp(0.1, 0.99);

        // Recommended phrase-aligned mix-in (skip low-energy intro if present) and mix-out (before outro)
        let mix_in_ms = phrases
            .iter()
            .find(|p| p.section_type != "intro" && p.confidence >= 0.55)
            .map(|p| p.start_ms)
            .unwrap_or(phrases.first().map(|p| p.start_ms).unwrap_or(0));

        let mix_out_ms = phrases
            .iter()
            .rev()
            .find(|p| p.section_type == "outro" && p.confidence >= 0.55)
            .map(|p| p.start_ms)
            .unwrap_or_else(|| {
                let fallback = (duration_ms as i64 - phrase_ms.round() as i64).max(mix_in_ms + 4_000);
                fallback.min(duration_ms as i64)
            });

        (phrases, overall_energy, dance_suitability, mix_in_ms, mix_out_ms)
    }
}


