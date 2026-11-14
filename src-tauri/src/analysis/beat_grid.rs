use crate::db::models::BeatGridPoint;

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
}

