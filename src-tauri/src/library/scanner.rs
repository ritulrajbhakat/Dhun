use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use anyhow::Result;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::analysis::analyze_track_file;
use crate::db::Database;
use crate::db::models::{AudioAsset, Track, TrackAnalysis};

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

fn compute_sha256<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64 KB streaming buffer

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn infer_genre_from_metadata(file_name: &str, bpm: f64) -> String {
    let lower = file_name.to_lowercase();
    if lower.contains("dhol") || lower.contains("bhangra") || lower.contains("punjabi") {
        "Punjabi Pop".to_string()
    } else if lower.contains("garba") || lower.contains("dandiya") || lower.contains("chogada") {
        "Garba Folk".to_string()
    } else if lower.contains("remix") || lower.contains("club") || lower.contains("mix") {
        "Desi Club".to_string()
    } else if lower.contains("retro") || lower.contains("disco") {
        "Retro Bollywood".to_string()
    } else if bpm >= 126.0 {
        "High-Energy Celebration".to_string()
    } else if bpm >= 110.0 {
        "Bollywood Dance".to_string()
    } else {
        "Melodic Celebration".to_string()
    }
}

fn ingest_audio_file(path: &Path, db: &Database) -> Result<()> {
    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("mp3");
    let metadata = fs::metadata(path)?;

    // Parse "Artist - Title" format if available
    let (artist, title) = if file_name.contains(" - ") {
        let parts: Vec<&str> = file_name.splitn(2, " - ").collect();
        (parts[0].trim().to_string(), parts[1].trim().to_string())
    } else {
        ("Various Artists".to_string(), file_name.trim().to_string())
    };

    let file_path_str = path.to_string_lossy().to_string();
    let track_id = if let Ok(Some(existing_id)) = db.get_track_id_by_path(&file_path_str) {
        existing_id
    } else {
        format!("trk_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string())
    };
    let asset_id = format!("ast_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let now = chrono::Utc::now().timestamp_millis();

    // 1. Compute real cryptographic SHA-256 hash
    let sha256_hash = match compute_sha256(path) {
        Ok(h) => h,
        Err(e) => {
            warn!("Failed to compute SHA-256 for {:?}: {}", path, e);
            format!("sha_{:x}_{:x}", metadata.len(), now)
        }
    };

    // 2. Run deterministic analysis with real duration, sample rate, channels, and verification states
    let (analysis, duration_ms, is_verified, sample_rate, channels) = match analyze_track_file(&track_id, path) {
        Ok(res) => {
            let dur = res.duration_ms as i64;
            (res.analysis, dur, true, res.sample_rate as i64, res.channels as i64)
        }
        Err(e) => {
            warn!("Analysis failed for {:?}: {}. Quarantining track.", path, e);
            let unverified_analysis = TrackAnalysis {
                track_id: track_id.clone(),
                bpm: 0.0,
                bpm_confidence: 0.0,
                key_name: "Unknown".to_string(),
                camelot_key: "Unknown".to_string(),
                key_confidence: 0.0,
                time_signature_num: 4,
                time_signature_den: 4,
                downbeat_offset_ms: 0,
                phrase_length_bars: 32,
                overall_energy: 0.0,
                dance_suitability: 0.0,
                recommended_mix_in_ms: 0,
                recommended_mix_out_ms: 0,
                analysis_version: 1,
                analyzed_at: now,
                manual_override: false,
            };
            (unverified_analysis, 0i64, false, 44100i64, 2i64)
        }
    };

    let genre = infer_genre_from_metadata(file_name, analysis.bpm);

    let track = Track {
        id: track_id.clone(),
        title,
        artist,
        album: None,
        duration_ms,
        year: None,
        genre,
        subgenre: None,
        created_at: now,
        updated_at: now,
    };

    // Separate verified vs failed/quarantined states
    let (verification_status, availability_status) = if is_verified {
        ("verified".to_string(), "available".to_string())
    } else {
        ("failed_analysis".to_string(), "quarantined".to_string())
    };

    let asset = AudioAsset {
        id: asset_id,
        track_id: track_id.clone(),
        file_path: file_path_str,
        file_format: ext.to_string(),
        file_size_bytes: metadata.len() as i64,
        sample_rate: sample_rate as i32,
        bit_depth: Some(16),
        channels: channels as i32,
        sha256_hash,
        verification_status,
        availability_status,
        last_verified_at: now,
    };

    db.insert_or_update_track(&track, &asset, &analysis)?;

    Ok(())
}
