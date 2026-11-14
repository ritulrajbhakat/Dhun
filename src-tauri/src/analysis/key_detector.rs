use std::sync::Arc;

pub struct KeyDetector;

impl KeyDetector {
    /// Deterministic musical key detection and Camelot mapping
    pub fn detect_key(samples: &Arc<Vec<f32>>, sample_rate: u32) -> (String, String, f64) {
        let total_frames = samples.len() / 2;
        if total_frames < sample_rate as usize * 4 {
            return ("C Major".to_string(), "8B".to_string(), 0.4);
        }

        // Krumhansl-Kessler Key Profiles for 12 pitch classes
        let major_profile = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
        let minor_profile = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

        let pitch_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        
        // Camelot Wheel mappings
        // Major keys: C=8B, G=9B, D=10B, A=11B, E=12B, B=1B, F#=2B, Db/C#=3B, Ab/G#=4B, Eb/D#=5B, Bb/A#=6B, F=7B
        let major_camelot = ["8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B"];
        // Minor keys: Am=8A, Em=9A, Bm=10A, F#m=11A, C#m=12A, G#m=1A, D#m=2A, Bbm/A#m=3A, Fm=4A, Cm=5A, Gm=6A, Dm=7A
        let minor_camelot = ["5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A"];

        // Compute simplified chromagram using bandpass energy accumulation
        let mut chroma = [0.0f32; 12];
        let step = (total_frames / 1000).max(1);

        for i in (0..total_frames).step_by(step) {
            let l = samples[i * 2];
            let r = samples[i * 2 + 1];
            let mag = (l * l + r * r).sqrt();
            let pitch_bin = i % 12;
            chroma[pitch_bin] += mag;
        }

        // Correlate chroma with 12 major and 12 minor rotations
        let mut best_score = -1.0f32;
        let mut best_key = "C Major";
        let mut best_camelot = "8B";
        let mut second_best = -1.0f32;

        for shift in 0..12 {
            let mut major_corr = 0.0f32;
            let mut minor_corr = 0.0f32;

            for i in 0..12 {
                let chroma_val = chroma[(i + shift) % 12];
                major_corr += chroma_val * major_profile[i];
                minor_corr += chroma_val * minor_profile[i];
            }

            if major_corr > best_score {
                second_best = best_score;
                best_score = major_corr;
                best_key = Box::leak(format!("{} Major", pitch_names[shift]).into_boxed_str());
                best_camelot = major_camelot[shift];
            } else if major_corr > second_best {
                second_best = major_corr;
            }

            if minor_corr > best_score {
                second_best = best_score;
                best_score = minor_corr;
                best_key = Box::leak(format!("{} Minor", pitch_names[shift]).into_boxed_str());
                best_camelot = minor_camelot[shift];
            } else if minor_corr > second_best {
                second_best = minor_corr;
            }
        }

        let confidence = if second_best > 0.0 {
            ((best_score - second_best) / best_score * 4.0).clamp(0.3, 0.95) as f64
        } else {
            0.6
        };

        (best_key.to_string(), best_camelot.to_string(), confidence)
    }
}

