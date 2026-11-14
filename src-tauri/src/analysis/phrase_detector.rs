use std::sync::Arc;
use crate::db::models::PhraseMarker;

pub struct PhraseDetector;

impl PhraseDetector {
    pub fn analyze_phrasing_and_energy(
        samples: &Arc<Vec<f32>>,
        _sample_rate: u32,
        bpm: f64,
        duration_ms: u64,
    ) -> (Vec<PhraseMarker>, f64, f64, i64, i64) {
        let beat_ms = (60.0 / bpm.max(60.0)) * 1000.0;
        let bar_ms = beat_ms * 4.0;
        let phrase_16_ms = bar_ms * 16.0;

        // Compute overall RMS energy
        let total_frames = samples.len() / 2;
        let step = (total_frames / 2000).max(1);
        let mut sum_sq = 0.0f32;
        let mut count = 0;
        for i in (0..total_frames).step_by(step) {
            let l = samples[i * 2];
            let r = samples[i * 2 + 1];
            sum_sq += l * l + r * r;
            count += 1;
        }
        let rms = if count > 0 { (sum_sq / count as f32).sqrt() } else { 0.2 };
        let overall_energy = (rms * 3.5).clamp(0.2, 0.98) as f64;
        let dance_suitability = (overall_energy * 0.9 + 0.1).clamp(0.1, 0.99);

        // Standard structural phrases
        let intro_end = (phrase_16_ms.round() as i64).min(duration_ms as i64 / 4);
        let outro_start = (duration_ms as i64 - phrase_16_ms.round() as i64).max(intro_end + 10000);

        let phrases = vec![
            PhraseMarker {
                section_type: "intro".to_string(),
                start_ms: 0,
                end_ms: intro_end,
                energy_level: (overall_energy * 0.7).clamp(0.1, 1.0),
            },
            PhraseMarker {
                section_type: "verse".to_string(),
                start_ms: intro_end,
                end_ms: (intro_end + outro_start) / 2,
                energy_level: overall_energy,
            },
            PhraseMarker {
                section_type: "chorus".to_string(),
                start_ms: (intro_end + outro_start) / 2,
                end_ms: outro_start,
                energy_level: (overall_energy * 1.15).clamp(0.1, 1.0),
            },
            PhraseMarker {
                section_type: "outro".to_string(),
                start_ms: outro_start,
                end_ms: duration_ms as i64,
                energy_level: (overall_energy * 0.6).clamp(0.1, 1.0),
            },
        ];

        let mix_in_ms = intro_end;
        let mix_out_ms = outro_start;

        (phrases, overall_energy, dance_suitability, mix_in_ms, mix_out_ms)
    }
}

