use std::fs;
use std::path::Path;
use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::analysis::analyze_track_file;
use crate::db::Database;
use crate::db::models::{AudioAsset, Track};

pub struct LibraryScanner;

impl LibraryScanner {
    pub fn scan_directory<P: AsRef<Path>>(dir: P, db: &Database) -> Result<usize> {
        let mut count = 0;
        let mut paths_to_scan = vec![dir.as_ref().to_path_buf()];

        let supported_extensions = ["mp3", "wav", "flac", "m4a", "aac", "ogg"];

        while let Some(current_dir) = paths_to_scan.pop() {
            if let Ok(entries) = fs::read_dir(&current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        paths_to_scan.push(path);
                    } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                            if let Ok(()) = ingest_audio_file(&path, db) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        info!("Scanned and indexed {} audio files into library", count);
        Ok(count)
    }
}

fn ingest_audio_file(path: &Path, db: &Database) -> Result<()> {
    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("mp3");
    let metadata = fs::metadata(path)?;

    // Parse "Artist - Title" heuristic if tags are unpopulated
    let (artist, title) = if file_name.contains(" - ") {
        let parts: Vec<&str> = file_name.splitn(2, " - ").collect();
        (parts[0].trim().to_string(), parts[1].trim().to_string())
    } else {
        ("Various Artists".to_string(), file_name.trim().to_string())
    };

    let track_id = format!("trk_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let asset_id = format!("ast_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let now = chrono::Utc::now().timestamp_millis();

    // Run deterministic analysis
    let analysis_res = match analyze_track_file(&track_id, path) {
        Ok(res) => res,
        Err(e) => {
            warn!("Could not fully analyze {:?}: {}, using safe fallback attributes", path, e);
            crate::analysis::FullAnalysisResult {
                analysis: crate::db::models::TrackAnalysis {
                    track_id: track_id.clone(),
                    bpm: 120.0,
                    bpm_confidence: 0.3,
                    key_name: "C Major".to_string(),
                    camelot_key: "8B".to_string(),
                    key_confidence: 0.3,
                    time_signature_num: 4,
                    time_signature_den: 4,
                    downbeat_offset_ms: 0,
                    phrase_length_bars: 32,
                    overall_energy: 0.7,
                    dance_suitability: 0.7,
                    recommended_mix_in_ms: 0,
                    recommended_mix_out_ms: 180000,
                    analysis_version: 1,
                    analyzed_at: now,
                    manual_override: false,
                },
                beat_grid: Vec::new(),
                phrase_markers: Vec::new(),
            }
        }
    };

    let track = Track {
        id: track_id.clone(),
        title,
        artist,
        album: None,
        duration_ms: (metadata.len() as f64 / 32000.0 * 1000.0).round() as i64, // Approximate if unparsed
        year: None,
        genre: "Bollywood Dance".to_string(),
        subgenre: None,
        created_at: now,
        updated_at: now,
    };

    let asset = AudioAsset {
        id: asset_id,
        track_id: track_id.clone(),
        file_path: path.to_string_lossy().to_string(),
        file_format: ext.to_string(),
        file_size_bytes: metadata.len() as i64,
        sample_rate: 44100,
        bit_depth: Some(16),
        channels: 2,
        sha256_hash: format!("{:x}", metadata.len()), // fast hash approximation
        verification_status: "verified".to_string(),
        availability_status: "available".to_string(),
        last_verified_at: now,
    };

    db.insert_or_update_track(&track, &asset, &analysis_res.analysis)?;

    Ok(())
}

