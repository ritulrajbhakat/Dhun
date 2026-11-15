use std::sync::Arc;

pub struct KeyDetector;

impl KeyDetector {
    /// Deterministic musical key detection using true frequency-domain chromagram
    /// and Krumhansl-Schmuckler key profile correlation
    pub fn detect_key(samples: &Arc<Vec<f32>>, sample_rate: u32) -> (String, String, f64) {
        let total_frames = samples.len() / 2;
        let sr = sample_rate as f32;

        if total_frames < sample_rate as usize * 2 || sr <= 0.0 {
            return ("C Major".to_string(), "8B".to_string(), 0.5);
        }

        // Krumhansl-Kessler Key Profiles for 12 pitch classes (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
        let major_profile: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
        let minor_profile: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

        let pitch_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

        // Camelot Wheel mappings (aligned to pitch_names index 0..11)
        // C=8B, C#=3B, D=10B, D#=5B, E=12B, F=7B, F#=2B, G=9B, G#=4B, A=11B, A#=6B, B=1B
        let major_camelot = ["8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B"];
        // Cm=5A, C#m=12A, Dm=7A, D#m=2A, Em=9A, Fm=4A, F#m=11A, Gm=6A, G#m=1A, Am=8A, A#m=3A, Bm=10A
        let minor_camelot = ["5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A"];

        // 1. Compute genuine 12-semitone Chromagram across musical octaves 2..6 (approx 65Hz to 2000Hz)
        // using Goertzel algorithm at fundamental pitch frequencies
        let mut chroma = [0.0f32; 12];

        // Sample windows evenly across the track (e.g. up to 16 analysis windows of 4096 samples)
        let window_size = 4096.min(total_frames);
        let num_windows = 16.min(total_frames / window_size).max(1);
        let stride = (total_frames - window_size) / num_windows;

        for w in 0..num_windows {
            let offset = w * stride;
            let mut mono_window = Vec::with_capacity(window_size);
            for i in 0..window_size {
                let frame_idx = offset + i;
                let l = samples[frame_idx * 2];
                let r = samples[frame_idx * 2 + 1];
                // Hann window smoothing
                let hanning = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / window_size as f32).cos());
                mono_window.push((l + r) * 0.5 * hanning);
            }

            // Accumulate energy for each of the 12 pitch classes across octaves 2..5
            for pitch in 0..12 {
                for octave in 2..6 {
                    // MIDI note number: C0 = 12, A4 (440Hz) = 69
                    // pitch 0 = C -> MIDI note = (octave + 1) * 12 + pitch
                    let midi_note = (octave + 1) * 12 + pitch;
                    let target_freq = 440.0 * 2.0f32.powf((midi_note as f32 - 69.0) / 12.0);

                    if target_freq < sr * 0.45 {
                        let energy = goertzel_energy(&mono_window, target_freq, sr);
                        chroma[pitch] += energy;
                    }
                }
            }
        }

        // Normalize chroma vector
        let chroma_sum: f32 = chroma.iter().sum();
        if chroma_sum > 0.0 {
            for c in chroma.iter_mut() {
                *c /= chroma_sum;
            }
        }

        // 2. Pearson Correlation with 12 Major and 12 Minor Rotations
        let mut best_score = -2.0f32;
        let mut best_is_major = true;
        let mut best_shift = 0;
        let mut second_best = -2.0f32;

        for shift in 0..12 {
            // Rotate profiles to compare with current chroma
            let mut rotated_chroma = [0.0f32; 12];
            for i in 0..12 {
                rotated_chroma[i] = chroma[(i + shift) % 12];
            }

            let major_corr = pearson_correlation(&rotated_chroma, &major_profile);
            let minor_corr = pearson_correlation(&rotated_chroma, &minor_profile);

            if major_corr > best_score {
                second_best = best_score;
                best_score = major_corr;
                best_is_major = true;
                best_shift = shift;
            } else if major_corr > second_best {
                second_best = major_corr;
            }

            if minor_corr > best_score {
                second_best = best_score;
                best_score = minor_corr;
                best_is_major = false;
                best_shift = shift;
            } else if minor_corr > second_best {
                second_best = minor_corr;
            }
        }

        // 3. Format output string without Box::leak
        let key_name = if best_is_major {
            format!("{} Major", pitch_names[best_shift])
        } else {
            format!("{} Minor", pitch_names[best_shift])
        };

        let camelot_key = if best_is_major {
            major_camelot[best_shift].to_string()
        } else {
            minor_camelot[best_shift].to_string()
        };

        let confidence = if second_best > -1.0 && best_score > second_best {
            let margin = (best_score - second_best) / (best_score.abs() + 0.001);
            (0.5 + margin * 0.5).clamp(0.4, 0.98) as f64
        } else {
            0.65
        };

        (key_name, camelot_key, confidence)
    }
}

/// Goertzel filter to compute spectral power at a single target frequency
fn goertzel_energy(samples: &[f32], target_freq: f32, sample_rate: f32) -> f32 {
    let n = samples.len() as f32;
    let k = (0.5 + (n * target_freq / sample_rate)).floor();
    let omega = (2.0 * std::f32::consts::PI * k) / n;
    let coeff = 2.0 * omega.cos();

    let mut q0: f32;
    let mut q1: f32 = 0.0;
    let mut q2: f32 = 0.0;

    for &sample in samples {
        q0 = coeff * q1 - q2 + sample;
        q2 = q1;
        q1 = q0;
    }

    (q1 * q1 + q2 * q2 - q1 * q2 * coeff).max(0.0).sqrt()
}

/// Pearson correlation coefficient between two 12-element vectors
fn pearson_correlation(x: &[f32; 12], y: &[f32; 12]) -> f32 {
    let mean_x: f32 = x.iter().sum::<f32>() / 12.0;
    let mean_y: f32 = y.iter().sum::<f32>() / 12.0;

    let mut num = 0.0f32;
    let mut den_x = 0.0f32;
    let mut den_y = 0.0f32;

    for i in 0..12 {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }

    let den = (den_x * den_y).sqrt();
    if den > 1e-6 {
        num / den
    } else {
        0.0
    }
}
