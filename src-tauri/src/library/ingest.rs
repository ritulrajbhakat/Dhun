use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::analysis::analyze_track_file;
use crate::db::Database;
use crate::db::models::{AudioAsset, CacheEntry, Track, TrackAnalysis};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub track_id: String,
    pub asset_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: i64,
    pub bpm: f64,
    pub key: String,
    pub sha256_hash: String,
    pub verification_status: String,
    pub is_duplicate: bool,
}

pub fn compute_file_sha256<P: AsRef<Path>>(path: P) -> Result<String> {
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

#[derive(Default)]
pub struct ProbedAudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub bit_depth: Option<i32>,
}

pub fn probe_audio_tags(path: &Path) -> ProbedAudioTags {
    let mut tags = ProbedAudioTags::default();

    let Ok(file) = File::open(path) else { return tags; };
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    if let Ok(mut probed) = symphonia::default::get_probe().format(
        &hint,
        mss,
        &symphonia::core::formats::FormatOptions::default(),
        &symphonia::core::meta::MetadataOptions::default(),
    ) {
        if let Some(track) = probed.format.tracks().iter().find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL) {
            tags.bit_depth = track.codec_params.bits_per_sample.or(track.codec_params.bits_per_coded_sample).map(|b| b as i32);
        }

        if let Some(meta) = probed.format.metadata().current() {
            for tag in meta.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        symphonia::core::meta::StandardTagKey::TrackTitle if tags.title.is_none() => {
                            tags.title = Some(tag.value.to_string());
                        }
                        symphonia::core::meta::StandardTagKey::Artist if tags.artist.is_none() => {
                            tags.artist = Some(tag.value.to_string());
                        }
                        symphonia::core::meta::StandardTagKey::Album if tags.album.is_none() => {
                            tags.album = Some(tag.value.to_string());
                        }
                        symphonia::core::meta::StandardTagKey::Genre if tags.genre.is_none() => {
                            tags.genre = Some(tag.value.to_string());
                        }
                        symphonia::core::meta::StandardTagKey::Date | symphonia::core::meta::StandardTagKey::ReleaseDate if tags.year.is_none() => {
                            let s = tag.value.to_string();
                            if let Ok(y) = s.chars().filter(|c| c.is_ascii_digit()).take(4).collect::<String>().parse::<i32>() {
                                tags.year = Some(y);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    tags
}

pub fn infer_genre_from_metadata(file_name: &str, bpm: f64) -> String {
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

/// Unified ingestion pipeline for local scanner files, manual imports, and downloaded tracks
pub fn ingest_audio_asset(
    path: &Path,
    source_type: &str,
    source_uri: Option<&str>,
    is_cached: bool,
    db: &Database,
) -> Result<IngestResult> {
    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown Track");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("mp3");
    let metadata = fs::metadata(path)?;
    let now = chrono::Utc::now().timestamp_millis();

    let probed_tags = probe_audio_tags(path);

    // Prefer ID3/container tags; fallback gracefully to "Artist - Title" filename parsing
    let title = probed_tags.title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if file_name.contains(" - ") {
                file_name.splitn(2, " - ").nth(1).unwrap_or(file_name).trim().to_string()
            } else {
                file_name.trim().to_string()
            }
        });

    let artist = probed_tags.artist
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if file_name.contains(" - ") {
                file_name.splitn(2, " - ").next().unwrap_or("Various Artists").trim().to_string()
            } else {
                "Various Artists".to_string()
            }
        });

    let album = probed_tags.album;
    let year = probed_tags.year;
    let bit_depth = probed_tags.bit_depth.or(Some(16));

    let file_path_str = path.to_string_lossy().to_string();
    let track_id = if let Ok(Some(existing_id)) = db.get_track_id_by_path(&file_path_str) {
        existing_id
    } else {
        format!("trk_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string())
    };
    let asset_id = format!("ast_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());

    // 1. Compute real cryptographic SHA-256 hash
    let sha256_hash = match compute_file_sha256(path) {
        Ok(h) => h,
        Err(e) => {
            warn!("Failed to compute SHA-256 for {:?}: {}", path, e);
            format!("sha_{:x}_{:x}", metadata.len(), now)
        }
    };

    // Duplicate detection: if an audio asset with this exact SHA-256 already exists, link and return without duplicating
    if let Ok(Some(existing_asset)) = db.get_asset_by_sha256(&sha256_hash) {
        info!("Duplicate audio asset detected with SHA-256 {}. Linking to existing track '{}'", sha256_hash, existing_asset.track_id);
        if let Some(uri) = source_uri {
            let _ = db.insert_source_reference(&existing_asset.track_id, source_type, uri);
        }
        return Ok(IngestResult {
            track_id: existing_asset.track_id,
            asset_id: existing_asset.id,
            title: title.clone(),
            artist: artist.clone(),
            duration_ms: (metadata.len() as f64 / 16000.0) as i64,
            bpm: 128.0,
            key: "8B".to_string(),
            sha256_hash,
            verification_status: existing_asset.verification_status,
            is_duplicate: true,
        });
    }

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

    let genre = probed_tags.genre
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| infer_genre_from_metadata(file_name, analysis.bpm));

    let track = Track {
        id: track_id.clone(),
        title: title.clone(),
        artist: artist.clone(),
        album,
        duration_ms,
        year,
        genre,
        subgenre: None,
        created_at: now,
        updated_at: now,
    };

    let (verification_status, availability_status) = if is_verified {
        ("verified".to_string(), "available".to_string())
    } else {
        ("failed_analysis".to_string(), "quarantined".to_string())
    };

    let asset = AudioAsset {
        id: asset_id.clone(),
        track_id: track_id.clone(),
        file_path: file_path_str.clone(),
        file_format: ext.to_string(),
        file_size_bytes: metadata.len() as i64,
        sample_rate: sample_rate as i32,
        bit_depth,
        channels: channels as i32,
        sha256_hash: sha256_hash.clone(),
        verification_status: verification_status.clone(),
        availability_status,
        last_verified_at: now,
    };

    db.insert_or_update_track(&track, &asset, &analysis)?;

    // 3. Register in cache manifest if this is a managed cached asset
    if is_cached {
        let cache_entry = CacheEntry {
            asset_id: asset_id.clone(),
            cached_file_path: file_path_str,
            size_bytes: metadata.len() as i64,
            is_locked: false,
            lock_reason: None,
            last_accessed_at: now,
            created_at: now,
        };
        let _ = db.upsert_cache_entry(&cache_entry);
    }

    info!("Ingested audio asset: '{}' by '{}' (BPM: {:.1}, Key: {})", title, artist, analysis.bpm, analysis.camelot_key);

    Ok(IngestResult {
        track_id,
        asset_id,
        title,
        artist,
        duration_ms,
        bpm: analysis.bpm,
        key: analysis.camelot_key,
        sha256_hash,
        verification_status,
        is_duplicate: false,
    })
}
