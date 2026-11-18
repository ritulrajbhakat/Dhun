use crate::db::models::{BeatGridPoint, TempoSegment};

pub struct BeatGridGenerator;

impl BeatGridGenerator {
    pub fn generate_grid(bpm: f64, downbeat_offset_ms: i64, duration_ms: u64) -> Vec<BeatGridPoint> {
        let beat_interval_ms = (60.0 / bpm.max(60.0)) * 1000.0;
        let total_beats = ((duration_ms as f64 - downbeat_offset_ms as f64) / beat_interval_ms).max(0.0) as i32;

        let mut grid = Vec::with_capacity(total_beats as usize);
        for b in 0..total_beats {
            let offset_ms = downbeat_offset_ms + ((b as f64) * beat_interval_ms).round() as i64;
            let is_downbeat = (b % 4) == 0;
            grid.push(BeatGridPoint {
                beat_number: b + 1,
                time_offset_ms: offset_ms,
                is_downbeat,
                confidence: 0.95,
            });
        }

        grid
    }

    /// Generates a variable-tempo, meter-aware timestamped beatgrid following local `TempoSegment`s.
    pub fn generate_variable_grid(
        tempo_map: &[TempoSegment],
        downbeat_offset_ms: i64,
        beats_per_bar: u8,
        duration_ms: u64,
        fallback_confidence: f64,
    ) -> Vec<BeatGridPoint> {
        let bar_beats = (beats_per_bar.max(2) as i32).min(8);
        if tempo_map.is_empty() || duration_ms == 0 {
            return Vec::new();
        }

        let mut grid = Vec::new();
        let mut cursor_ms = downbeat_offset_ms.max(0) as f64;
        let mut beat_idx: i32 = 0;

        while (cursor_ms as u64) < duration_ms && beat_idx < 4000 {
            let cur_i64 = cursor_ms.round() as i64;
            let seg = tempo_map
                .iter()
                .find(|s| cur_i64 >= s.start_ms && cur_i64 < s.end_ms)
                .or_else(|| tempo_map.last());
            let local_bpm = seg.map(|s| s.bpm).unwrap_or(120.0).clamp(60.0, 200.0);
            let local_conf = seg
                .map(|s| s.confidence)
                .unwrap_or(fallback_confidence)
                .clamp(0.15, 0.98);

            grid.push(BeatGridPoint {
                beat_number: beat_idx + 1,
                time_offset_ms: cur_i64,
                is_downbeat: (beat_idx % bar_beats) == 0,
                confidence: (local_conf * 100.0).round() / 100.0,
            });

            beat_idx += 1;
            cursor_ms += (60.0 / local_bpm) * 1000.0;
        }

        grid
    }
}

