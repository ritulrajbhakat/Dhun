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
        Self::analyze_phrasing_with_timing(samples, sample_rate, bpm, downbeat_offset_ms, 4, duration_ms)
    }

    /// Meter-aware, bar-aligned phrase/section & vocal-activity DSP analyzer.
    /// Measures per-phrase RMS energy, sub-200 Hz bass energy, and 300–3400 Hz center-channel
    /// vocal presence (`vocal_activity`, 0.0–1.0) so downstream transition planning avoids
    /// vocal-vs-vocal clashes and respects 4/4, 3/4, and 6/8 phrase groupings.
    pub fn analyze_phrasing_with_timing(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
        beats_per_bar: u8,
        duration_ms: u64,
    ) -> (Vec<PhraseMarker>, f64, f64, i64, i64) {
        let sr = sample_rate.max(8000) as f64;
        let clean_bpm = bpm.clamp(60.0, 200.0);
        let bar_beats = (beats_per_bar.max(3) as f64).min(6.0);
        let beat_ms = (60.0 / clean_bpm) * 1000.0;
        let bar_ms = beat_ms * bar_beats;
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

        // 1-pole filter coefficients:
        // - sub-200 Hz low-pass for bass-band energy
        // - 300 Hz high-pass + 3400 Hz low-pass on mid (L+R) channel for deterministic vocal-band estimation
        let dt = 1.0 / sr;
        let lp_200 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 200.0) + dt)).clamp(0.001, 0.5);
        let lp_300 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 300.0) + dt)).clamp(0.001, 0.55);
        let lp_3400 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 3400.0) + dt)).clamp(0.01, 0.9);

        // Measure per-phrase RMS, bass-band RMS, and vocal-band dominance aligned to downbeat_offset_ms
        let start_anchor_ms = downbeat_offset_ms.clamp(0, (bar_ms as i64).min(duration_ms as i64 / 4));
        let mut raw_windows: Vec<(i64, i64, f64, f64, f64, i32)> = Vec::new();
        let mut cursor_ms = if start_anchor_ms > 500 {
            0.0
        } else {
            start_anchor_ms as f64
        };
        let mut bar_counter: i32 = 0;

        while (cursor_ms as u64) < duration_ms {
            let next_ms = if cursor_ms == 0.0 && start_anchor_ms > 500 {
                start_anchor_ms as f64 + phrase_ms
            } else {
                cursor_ms + phrase_ms
            };
            let end_ms = next_ms.min(duration_ms as f64);
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
            let stride = if frames_in_win > 120_000 { 4 } else { 1 };
            let a200 = (lp_200 * stride as f64).min(0.8);
            let a300 = (lp_300 * stride as f64).min(0.85);
            let a3400 = (lp_3400 * stride as f64).min(0.95);

            let mut sum_sq = 0.0f64;
            let mut bass_sq = 0.0f64;
            let mut vocal_mid_sq = 0.0f64;
            let mut side_sq = 0.0f64;
            let mut lp200_st = 0.0f64;
            let mut lp300_st = 0.0f64;
            let mut lp3400_st = 0.0f64;
            let mut n = 0usize;

            let mut idx = start_frame;
            while idx < end_frame {
                let l = samples[idx * 2] as f64;
                let r = samples[idx * 2 + 1] as f64;
                let mid = 0.5 * (l + r);
                let side = 0.5 * (l - r);

                lp200_st += a200 * (mid - lp200_st);
                lp300_st += a300 * (mid - lp300_st);
                lp3400_st += a3400 * (mid - lp3400_st);
                let vocal_band = lp3400_st - lp300_st;

                sum_sq += mid * mid;
                bass_sq += lp200_st * lp200_st;
                vocal_mid_sq += vocal_band * vocal_band;
                side_sq += side * side;
                n += 1;
                idx += stride;
            }

            let win_rms = if n > 0 { (sum_sq / n as f64).sqrt() } else { 0.0 };
            let win_bass = if n > 0 { (bass_sq / n as f64).sqrt() } else { 0.0 };
            let win_vocal_rms = if n > 0 { (vocal_mid_sq / n as f64).sqrt() } else { 0.0 };
            let win_side_rms = if n > 0 { (side_sq / n as f64).sqrt() } else { 0.0 };

            // Vocal presence heuristic: ratio of center-channel 300–3400 Hz energy to total energy,
            // weighted by center focus (vocals are centered; wide stereo pads/reverb have higher side energy)
            let center_focus = (win_rms / (win_rms + win_side_rms + 1e-6)).clamp(0.4, 1.0);
            let vocal_band_ratio = if win_rms > 0.015 {
                (win_vocal_rms / (win_rms + 1e-6)) * center_focus
            } else {
                0.0
            };
            let vocal_activity = ((vocal_band_ratio - 0.22) * 1.45).clamp(0.0, 1.0);

            raw_windows.push((
                cursor_ms.round() as i64,
                end_ms.round() as i64,
                win_rms,
                win_bass,
                vocal_activity,
                bar_counter,
            ));
            bar_counter += phrase_bars as i32;
            cursor_ms = end_ms;
        }

        if raw_windows.is_empty() {
            return (Vec::new(), overall_energy, (overall_energy * 0.9 + 0.1).clamp(0.1, 0.99), 0, duration_ms as i64);
        }

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
            let (s_ms, e_ms, rms, bass, vocal_act, bar_idx) = raw_windows[i];
            let prev_rms = if i > 0 { raw_windows[i - 1].2 } else { rms };
            let next_rms = if i + 1 < count { raw_windows[i + 1].2 } else { rms };
            let rel_e = rms / median_e;
            let rel_bass = bass / median_bass;
            let norm_e = (rms * 3.5).clamp(0.05, 0.99);
            let norm_bass = (bass * 4.2).clamp(0.02, 0.99);

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
                let jump = ((rms - prev_rms) / median_e).clamp(0.0, 0.8);
                ("drop", (0.65 + jump * 0.4).clamp(0.60, 0.95))
            } else if i + 1 < count && (next_rms - rms) / median_e >= 0.22 && rel_e >= 0.55 {
                let rise = ((next_rms - rms) / median_e).clamp(0.0, 0.7);
                ("build", (0.60 + rise * 0.45).clamp(0.58, 0.92))
            } else if rel_e < 0.72 || (rel_bass < 0.62 && rel_e < 0.88) {
                let dip = (1.0 - rel_e).clamp(0.0, 0.6);
                ("breakdown", (0.62 + dip * 0.5).clamp(0.58, 0.92))
            } else if rel_e >= 1.08 && rel_bass >= 0.90 {
                let lift = (rel_e - 1.0).clamp(0.0, 0.5);
                if vocal_act >= 0.58 {
                    ("chorus", (0.64 + lift * 0.55).clamp(0.60, 0.94))
                } else {
                    ("peak", (0.64 + lift * 0.55).clamp(0.60, 0.94))
                }
            } else if vocal_act >= 0.60 {
                ("verse", (0.58 + dynamic_spread * 0.22).clamp(0.52, 0.85))
            } else if vocal_act <= 0.32 && rel_bass >= 0.80 {
                ("instrumental", (0.60 + dynamic_spread * 0.25).clamp(0.54, 0.88))
            } else {
                ("groove", (0.58 + dynamic_spread * 0.25).clamp(0.50, 0.85))
            };

            phrases.push(PhraseMarker {
                section_type: section_type.to_string(),
                start_ms: s_ms,
                end_ms: e_ms,
                energy_level: (norm_e * 1000.0).round() / 1000.0,
                bass_energy: (norm_bass * 1000.0).round() / 1000.0,
                vocal_activity: (vocal_act * 100.0).round() / 100.0,
                confidence: (confidence * 100.0).round() / 100.0,
                bar_index: bar_idx,
            });
        }

        let bass_ratio = (median_bass / median_e).clamp(0.1, 1.0);
        let dance_suitability = (overall_energy * 0.72 + bass_ratio * 0.28).clamp(0.1, 0.99);

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


