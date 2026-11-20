use std::sync::Arc;
use crate::db::models::PhraseMarker;

#[derive(Debug, Clone)]
pub struct PhraseAnalysisResult {
    pub phrases: Vec<PhraseMarker>,
    pub overall_energy: f64,
    pub dance_suitability: f64,
    pub mix_in_ms: i64,
    pub mix_out_ms: i64,
    pub sub_bass_energy: f64,
    pub mid_energy: f64,
    pub high_energy: f64,
    pub dynamic_range_db: f64,
    pub has_lead_vocal: bool,
    pub has_backing_vocal: bool,
    pub vocal_confidence: f64,
    // Multi-cue & non-musical structural classification:
    pub first_musical_ms: i64,
    pub dialogue_end_ms: i64,
    pub main_drop_ms: i64,
    pub first_vocal_entry_ms: i64,
    pub first_hook_ms: i64,
    pub has_theatrical_alaap: bool,
    pub theatrical_alaap_start_ms: i64,
    pub theatrical_alaap_end_ms: i64,
}

pub struct PhraseDetector;

impl PhraseDetector {
    /// Deterministic bar- and phrase-aligned DSP section analyzer.
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
    /// Preserves existing 5-tuple signature for compatibility.
    pub fn analyze_phrasing_with_timing(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
        beats_per_bar: u8,
        duration_ms: u64,
    ) -> (Vec<PhraseMarker>, f64, f64, i64, i64) {
        let res = Self::analyze_phrasing_full(samples, sample_rate, bpm, downbeat_offset_ms, beats_per_bar, duration_ms);
        (res.phrases, res.overall_energy, res.dance_suitability, res.mix_in_ms, res.mix_out_ms)
    }

    /// Comprehensive Version 3 DSP analyzer measuring structural sections (including pre_chorus and bridge),
    /// 3-band frequency energy profiles (sub-bass, mid, high), dynamic range, and vocal classification.
    pub fn analyze_phrasing_full(
        samples: &Arc<Vec<f32>>,
        sample_rate: u32,
        bpm: f64,
        downbeat_offset_ms: i64,
        beats_per_bar: u8,
        duration_ms: u64,
    ) -> PhraseAnalysisResult {
        let sr = sample_rate.max(8000) as f64;
        let clean_bpm = bpm.clamp(60.0, 200.0);
        let bar_beats = (beats_per_bar.max(3) as f64).min(6.0);
        let beat_ms = (60.0 / clean_bpm) * 1000.0;
        let bar_ms = beat_ms * bar_beats;
        let phrase_bars = if duration_ms >= 45_000 { 8.0 } else { 4.0 };
        let phrase_ms = (bar_ms * phrase_bars).max(2_000.0);

        let total_frames = samples.len() / 2;
        if total_frames == 0 || duration_ms == 0 {
            return PhraseAnalysisResult {
                phrases: Vec::new(),
                overall_energy: 0.5,
                dance_suitability: 0.5,
                mix_in_ms: 0,
                mix_out_ms: 0,
                sub_bass_energy: 0.35,
                mid_energy: 0.45,
                high_energy: 0.40,
                dynamic_range_db: 12.0,
                has_lead_vocal: false,
                has_backing_vocal: false,
                vocal_confidence: 0.40,
                first_musical_ms: 0,
                dialogue_end_ms: 0,
                main_drop_ms: 0,
                first_vocal_entry_ms: 0,
                first_hook_ms: 0,
                has_theatrical_alaap: false,
                theatrical_alaap_start_ms: 0,
                theatrical_alaap_end_ms: 0,
            };
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
        // - Sub-bass: < 90 Hz
        // - Bass: < 250 Hz
        // - Vocal lower: 300 Hz, vocal upper / mid: 3400 Hz
        // - Mid cutoff: 3500 Hz
        let dt = 1.0 / sr;
        let lp_90 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 90.0) + dt)).clamp(0.0005, 0.4);
        let lp_250 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 250.0) + dt)).clamp(0.001, 0.5);
        let lp_300 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 300.0) + dt)).clamp(0.001, 0.55);
        let lp_3400 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 3400.0) + dt)).clamp(0.01, 0.9);
        let lp_3500 = (dt / (1.0 / (2.0 * std::f64::consts::PI * 3500.0) + dt)).clamp(0.01, 0.9);

        let start_anchor_ms = downbeat_offset_ms.clamp(0, (bar_ms as i64).min(duration_ms as i64 / 4));
        // raw_windows tuple: (start_ms, end_ms, win_rms, win_bass, win_sub_bass, win_mid, win_high, vocal_activity, center_focus, bar_counter)
        let mut raw_windows: Vec<(i64, i64, f64, f64, f64, f64, f64, f64, f64, i32)> = Vec::new();
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
            let a90 = (lp_90 * stride as f64).min(0.75);
            let a250 = (lp_250 * stride as f64).min(0.8);
            let a300 = (lp_300 * stride as f64).min(0.85);
            let a3400 = (lp_3400 * stride as f64).min(0.95);
            let a3500 = (lp_3500 * stride as f64).min(0.95);

            let mut sum_sq = 0.0f64;
            let mut sub_bass_sq = 0.0f64;
            let mut bass_sq = 0.0f64;
            let mut vocal_mid_sq = 0.0f64;
            let mut mid_sq = 0.0f64;
            let mut high_sq = 0.0f64;
            let mut side_sq = 0.0f64;

            let mut lp90_st = 0.0f64;
            let mut lp250_st = 0.0f64;
            let mut lp300_st = 0.0f64;
            let mut lp3400_st = 0.0f64;
            let mut lp3500_st = 0.0f64;
            let mut n = 0usize;

            let mut idx = start_frame;
            while idx < end_frame {
                let l = samples[idx * 2] as f64;
                let r = samples[idx * 2 + 1] as f64;
                let mid = 0.5 * (l + r);
                let side = 0.5 * (l - r);

                lp90_st += a90 * (mid - lp90_st);
                lp250_st += a250 * (mid - lp250_st);
                lp300_st += a300 * (mid - lp300_st);
                lp3400_st += a3400 * (mid - lp3400_st);
                lp3500_st += a3500 * (mid - lp3500_st);

                let vocal_band = lp3400_st - lp300_st;
                let mid_band = lp3500_st - lp250_st;
                let high_band = mid - lp3500_st;

                sum_sq += mid * mid;
                sub_bass_sq += lp90_st * lp90_st;
                bass_sq += lp250_st * lp250_st;
                vocal_mid_sq += vocal_band * vocal_band;
                mid_sq += mid_band * mid_band;
                high_sq += high_band * high_band;
                side_sq += side * side;
                n += 1;
                idx += stride;
            }

            let win_rms = if n > 0 { (sum_sq / n as f64).sqrt() } else { 0.0 };
            let win_sub_bass = if n > 0 { (sub_bass_sq / n as f64).sqrt() } else { 0.0 };
            let win_bass = if n > 0 { (bass_sq / n as f64).sqrt() } else { 0.0 };
            let win_mid = if n > 0 { (mid_sq / n as f64).sqrt() } else { 0.0 };
            let win_high = if n > 0 { (high_sq / n as f64).sqrt() } else { 0.0 };
            let win_vocal_rms = if n > 0 { (vocal_mid_sq / n as f64).sqrt() } else { 0.0 };
            let win_side_rms = if n > 0 { (side_sq / n as f64).sqrt() } else { 0.0 };

            let center_focus = (win_rms / (win_rms + win_side_rms + 1e-6)).clamp(0.35, 1.0);
            let vocal_band_ratio = if win_rms > 0.015 {
                (win_vocal_rms / (win_rms + 1e-6)) * center_focus
            } else {
                0.0
            };
            let vocal_activity = ((vocal_band_ratio - 0.20) * 1.5).clamp(0.0, 1.0);

            raw_windows.push((
                cursor_ms.round() as i64,
                end_ms.round() as i64,
                win_rms,
                win_bass,
                win_sub_bass,
                win_mid,
                win_high,
                vocal_activity,
                center_focus,
                bar_counter,
            ));
            bar_counter += phrase_bars as i32;
            cursor_ms = end_ms;
        }

        if raw_windows.is_empty() {
            return PhraseAnalysisResult {
                phrases: Vec::new(),
                overall_energy,
                dance_suitability: (overall_energy * 0.9 + 0.1).clamp(0.1, 0.99),
                mix_in_ms: 0,
                mix_out_ms: duration_ms as i64,
                sub_bass_energy: 0.35,
                mid_energy: 0.45,
                high_energy: 0.40,
                dynamic_range_db: 12.0,
                has_lead_vocal: false,
                has_backing_vocal: false,
                vocal_confidence: 0.40,
                first_musical_ms: 0,
                dialogue_end_ms: 0,
                main_drop_ms: 0,
                first_vocal_entry_ms: 0,
                first_hook_ms: 0,
                has_theatrical_alaap: false,
                theatrical_alaap_start_ms: 0,
                theatrical_alaap_end_ms: 0,
            };
        }

        let mut energies: Vec<f64> = raw_windows.iter().map(|w| w.2).collect();
        energies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min_e = energies.first().copied().unwrap_or(0.0001).max(0.0001);
        let max_e = energies.last().copied().unwrap_or(0.001).max(0.001);
        let median_e = energies[energies.len() / 2].max(0.001);
        let dynamic_spread = ((max_e - min_e) / max_e).clamp(0.0, 1.0);
        let dynamic_range_db = (20.0 * (max_e / min_e).log10()).clamp(3.0, 30.0);

        let mut basses: Vec<f64> = raw_windows.iter().map(|w| w.3).collect();
        basses.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_bass = basses[basses.len() / 2].max(0.0005);

        let count = raw_windows.len();
        let mut phrases: Vec<PhraseMarker> = Vec::with_capacity(count);

        let mut has_lead_vocal = false;
        let mut has_backing_vocal = false;
        let mut max_vocal_act = 0.0f64;

        let mut sum_sub_bass = 0.0;
        let mut sum_mid = 0.0;
        let mut sum_high = 0.0;

        for i in 0..count {
            let (s_ms, e_ms, rms, bass, sub_bass, mid_band, high_band, vocal_act, center_focus, bar_idx) = raw_windows[i];
            let prev_rms = if i > 0 { raw_windows[i - 1].2 } else { rms };
            let next_rms = if i + 1 < count { raw_windows[i + 1].2 } else { rms };
            let rel_e = rms / median_e;
            let rel_bass = bass / median_bass;
            let norm_e = (rms * 3.5).clamp(0.05, 0.99);
            let norm_bass = (bass * 4.2).clamp(0.02, 0.99);
            let norm_sub_bass = (sub_bass * 4.5).clamp(0.02, 0.99);
            let norm_mid = (mid_band * 3.8).clamp(0.05, 0.99);
            let norm_high = (high_band * 4.0).clamp(0.05, 0.99);

            sum_sub_bass += norm_sub_bass;
            sum_mid += norm_mid;
            sum_high += norm_high;

            if vocal_act > max_vocal_act {
                max_vocal_act = vocal_act;
            }
            if vocal_act >= 0.52 && center_focus >= 0.62 {
                has_lead_vocal = true;
            } else if vocal_act >= 0.35 && center_focus < 0.62 {
                has_backing_vocal = true;
            }

            // Estimate phrase LUFS
            let loudness_lufs = (-0.691 + 10.0 * (rms * rms + 1e-12).log10()).clamp(-70.0, -0.1);

            // Structural Section Classification with pre_chorus and bridge detection
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
            } else if i > 0 && i + 1 < count && vocal_act >= 0.50 && rms > prev_rms * 1.10 && next_rms > rms {
                // Rising vocal build between verse and chorus -> pre_chorus
                ("pre_chorus", 0.75)
            } else if i >= count / 2 && i + 2 < count && rel_bass < 0.78 && vocal_act >= 0.45 && rel_e < 1.05 {
                // Mid/late track structural departure with shifted texture -> bridge
                ("bridge", 0.72)
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
                sub_bass_energy: (norm_sub_bass * 1000.0).round() / 1000.0,
                mid_energy: (norm_mid * 1000.0).round() / 1000.0,
                high_energy: (norm_high * 1000.0).round() / 1000.0,
                loudness_lufs_est: (loudness_lufs * 10.0).round() / 10.0,
            });
        }

        let bass_ratio = (median_bass / median_e).clamp(0.1, 1.0);
        let dance_suitability = (overall_energy * 0.72 + bass_ratio * 0.28).clamp(0.1, 0.99);

        let avg_sub_bass = if count > 0 { (sum_sub_bass / count as f64 * 100.0).round() / 100.0 } else { 0.35 };
        let avg_mid = if count > 0 { (sum_mid / count as f64 * 100.0).round() / 100.0 } else { 0.45 };
        let avg_high = if count > 0 { (sum_high / count as f64 * 100.0).round() / 100.0 } else { 0.40 };
        let vocal_confidence = (0.50 + max_vocal_act * 0.45).clamp(0.35, 0.98);

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

        let mut dialogue_end_ms: i64 = 0;
        let mut in_dialogue = true;
        for p in &phrases {
            if in_dialogue {
                let is_dialogue_phrase = p.vocal_activity >= 0.35
                    && p.bass_energy < 0.32
                    && p.sub_bass_energy < 0.24
                    && p.energy_level < 0.52;
                let is_ambient_lead = p.energy_level < 0.22 && p.bass_energy < 0.20;
                if is_dialogue_phrase || is_ambient_lead {
                    dialogue_end_ms = p.end_ms;
                } else {
                    in_dialogue = false;
                }
            }
        }

        let first_musical_ms: i64 = phrases
            .iter()
            .find(|p| p.energy_level >= 0.18 || p.bass_energy >= 0.20 || p.mid_energy >= 0.25)
            .map(|p| p.start_ms)
            .unwrap_or(0);

        // Theatrical alaap detection (unmetered/cinematic vocal or melodic intro before rhythmic drop)
        let mut has_theatrical_alaap = false;
        let mut theatrical_alaap_start_ms: i64 = 0;
        let mut theatrical_alaap_end_ms: i64 = 0;
        if let Some(first_p) = phrases.first() {
            if first_p.start_ms <= 2_000 && first_p.vocal_activity >= 0.45 && first_p.sub_bass_energy < 0.36 {
                // Check if this vocal-heavy, low-sub-bass state continues for multiple phrases or >= 8 seconds
                let mut alaap_dur = 0i64;
                for p in &phrases {
                    if p.vocal_activity >= 0.40 && p.sub_bass_energy < 0.40 {
                        alaap_dur = p.end_ms;
                    } else {
                        break;
                    }
                }
                if alaap_dur >= 8_000 {
                    has_theatrical_alaap = true;
                    theatrical_alaap_start_ms = first_p.start_ms;
                    theatrical_alaap_end_ms = alaap_dur;
                }
            }
        }

        let main_drop_ms: i64 = phrases
            .iter()
            .find(|p| {
                p.section_type == "drop"
                    || (p.energy_level >= 0.60 && p.bass_energy >= 0.50 && p.sub_bass_energy >= 0.38)
                    || (p.energy_level >= 0.55 && p.bass_energy >= 0.48 && (p.section_type == "chorus" || p.section_type == "peak" || p.section_type == "groove"))
            })
            .map(|p| p.start_ms)
            .unwrap_or(mix_in_ms);

        let first_vocal_entry_ms: i64 = phrases
            .iter()
            .find(|p| p.vocal_activity >= 0.45)
            .map(|p| p.start_ms)
            .unwrap_or(0);

        let first_hook_ms: i64 = phrases
            .iter()
            .find(|p| p.section_type == "chorus" || (p.section_type == "peak" && p.vocal_activity >= 0.40))
            .map(|p| p.start_ms)
            .unwrap_or_else(|| {
                phrases
                    .iter()
                    .find(|p| p.vocal_activity >= 0.55 && p.energy_level >= 0.58)
                    .map(|p| p.start_ms)
                    .unwrap_or(main_drop_ms)
            });

        PhraseAnalysisResult {
            phrases,
            overall_energy,
            dance_suitability,
            mix_in_ms: mix_in_ms.max(dialogue_end_ms),
            mix_out_ms,
            sub_bass_energy: avg_sub_bass,
            mid_energy: avg_mid,
            high_energy: avg_high,
            dynamic_range_db: (dynamic_range_db * 10.0).round() / 10.0,
            has_lead_vocal,
            has_backing_vocal,
            vocal_confidence: (vocal_confidence * 100.0).round() / 100.0,
            first_musical_ms,
            dialogue_end_ms,
            main_drop_ms,
            first_vocal_entry_ms,
            first_hook_ms,
            has_theatrical_alaap,
            theatrical_alaap_start_ms,
            theatrical_alaap_end_ms,
        }
    }
}
