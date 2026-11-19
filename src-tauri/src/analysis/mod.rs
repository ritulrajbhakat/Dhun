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
use crate::db::models::{TrackAnalysis, TrackTimingProfile};

#[allow(dead_code)]
pub struct FullAnalysisResult {
    pub analysis: TrackAnalysis,
    pub beat_grid: Vec<crate::db::models::BeatGridPoint>,
    pub phrase_markers: Vec<crate::db::models::PhraseMarker>,
    pub timing_profile: TrackTimingProfile,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u32,
}

pub fn analyze_track_file<P: AsRef<Path>>(track_id: &str, file_path: P) -> Result<FullAnalysisResult> {
    let decoded = decode_audio_file(file_path)?;
    let (bpm, bpm_confidence, tempo_map, tempo_drift_bpm, is_variable_tempo) =
        BpmDetector::detect_tempo_map(&decoded.samples, decoded.sample_rate, decoded.duration_ms);
    let (key_name, camelot_key, key_confidence) = KeyDetector::detect_key(&decoded.samples, decoded.sample_rate);
    
    let (downbeat_offset_ms, downbeat_confidence) = BpmDetector::detect_downbeat(&decoded.samples, decoded.sample_rate, bpm);
    let meter = BpmDetector::detect_meter(&decoded.samples, decoded.sample_rate, bpm, downbeat_offset_ms);
    let (phrase_markers, overall_energy, dance_suitability, mix_in_ms, mix_out_ms) =
        PhraseDetector::analyze_phrasing_with_timing(
            &decoded.samples,
            decoded.sample_rate,
            bpm,
            downbeat_offset_ms,
            meter.beats_per_bar,
            decoded.duration_ms,
        );

    // Compute separate, distinct beatgrid confidence measuring rhythmic alignment & drift
    let beatgrid_confidence = {
        let drift_penalty = (tempo_drift_bpm / 12.0).clamp(0.0, 0.35);
        ((bpm_confidence * 0.80 + downbeat_confidence * 0.20 - drift_penalty) * 100.0).round() / 100.0
    }.clamp(0.25, 0.98);

    let beat_grid = BeatGridGenerator::generate_variable_grid(
        &tempo_map,
        downbeat_offset_ms,
        meter.beats_per_bar,
        decoded.duration_ms,
        beatgrid_confidence,
    );

    let beat_timestamps_ms: Vec<i64> = beat_grid.iter().map(|b| b.time_offset_ms).collect();
    let downbeat_timestamps_ms: Vec<i64> = beat_grid
        .iter()
        .filter(|b| b.is_downbeat)
        .map(|b| b.time_offset_ms)
        .collect();

    let timing_profile = TrackTimingProfile {
        average_bpm: bpm,
        beatgrid_confidence,
        bpm_confidence,
        downbeat_confidence,
        downbeat_offset_ms,
        is_variable_tempo,
        tempo_drift_bpm,
        meter: meter.clone(),
        tempo_map,
        beat_timestamps_ms,
        downbeat_timestamps_ms,
    };

    let analysis = TrackAnalysis {
        track_id: track_id.to_string(),
        bpm,
        bpm_confidence,
        key_name,
        camelot_key,
        key_confidence,
        time_signature_num: meter.beats_per_bar as i32,
        time_signature_den: if meter.meter_label == "6/8" { 8 } else { 4 },
        downbeat_offset_ms,
        phrase_length_bars: 32,
        overall_energy,
        dance_suitability,
        recommended_mix_in_ms: mix_in_ms,
        recommended_mix_out_ms: mix_out_ms,
        analysis_version: 2,
        analyzed_at: chrono::Utc::now().timestamp_millis(),
        manual_override: false,
    };

    Ok(FullAnalysisResult {
        analysis,
        beat_grid,
        phrase_markers,
        timing_profile,
        duration_ms: decoded.duration_ms,
        sample_rate: decoded.sample_rate,
        channels: decoded.channels,
    })
}

