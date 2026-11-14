pub mod beat_grid;
pub mod bpm_detector;
pub mod key_detector;
pub mod phrase_detector;

use std::path::Path;
use anyhow::Result;

use crate::analysis::beat_grid::BeatGridGenerator;
use crate::analysis::bpm_detector::BpmDetector;
use crate::analysis::key_detector::KeyDetector;
use crate::analysis::phrase_detector::PhraseDetector;
use crate::audio::decoder::decode_audio_file;
use crate::db::models::TrackAnalysis;

#[allow(dead_code)]
pub struct FullAnalysisResult {
    pub analysis: TrackAnalysis,
    pub beat_grid: Vec<crate::db::models::BeatGridPoint>,
    pub phrase_markers: Vec<crate::db::models::PhraseMarker>,
}

pub fn analyze_track_file<P: AsRef<Path>>(track_id: &str, file_path: P) -> Result<FullAnalysisResult> {
    let decoded = decode_audio_file(file_path)?;
    let (bpm, bpm_confidence) = BpmDetector::detect_bpm(&decoded.samples, decoded.sample_rate);
    let (key_name, camelot_key, key_confidence) = KeyDetector::detect_key(&decoded.samples, decoded.sample_rate);
    
    let downbeat_offset_ms = 120; // Estimated primary downbeat offset in ms
    let (phrase_markers, overall_energy, dance_suitability, mix_in_ms, mix_out_ms) =
        PhraseDetector::analyze_phrasing_and_energy(&decoded.samples, decoded.sample_rate, bpm, decoded.duration_ms);

    let beat_grid = BeatGridGenerator::generate_grid(bpm, downbeat_offset_ms, decoded.duration_ms);

    let analysis = TrackAnalysis {
        track_id: track_id.to_string(),
        bpm,
        bpm_confidence,
        key_name,
        camelot_key,
        key_confidence,
        time_signature_num: 4,
        time_signature_den: 4,
        downbeat_offset_ms,
        phrase_length_bars: 32,
        overall_energy,
        dance_suitability,
        recommended_mix_in_ms: mix_in_ms,
        recommended_mix_out_ms: mix_out_ms,
        analysis_version: 1,
        analyzed_at: chrono::Utc::now().timestamp_millis(),
        manual_override: false,
    };

    Ok(FullAnalysisResult {
        analysis,
        beat_grid,
        phrase_markers,
    })
}

