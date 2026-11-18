use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::analysis::analyze_track_file;
use crate::db::models::{GlobalSongMetadata, LocalTempCacheEntry};
use crate::db::Database;
use crate::library::cache::{RollingCacheManager, CACHE_PRIORITY_UPCOMING, DEFAULT_MAX_TEMP_TRACKS};
use crate::library::downloader::AudioDownloader;
use crate::library::youtube::{fetch_youtube_metadata, parse_youtube_url, ParsedYouTubeUrl};
use crate::metadata_service::{
    compute_canonical_fingerprint, extract_version_tag, CatalogueLookupRequest,
    CatalogueUpsertRequest,
};
use crate::sync::MetadataSyncManager;

/// Explicit representation of Stage 3 (Authorised Audio Acquisition) compliance outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AcquisitionStatus {
    /// Standard YouTube / YouTube Music URL: oEmbed metadata is permitted,
    /// but platform ToS restricts automated audio downloading / extraction.
    RestrictedByProvider,
    /// Authorized media stream acquired or reused from local temporary cache.
    CachedAuthorized,
    /// Acquisition failed due to network, integrity, or security validation error.
    Failed,
}

impl AcquisitionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RestrictedByProvider => "restricted_by_provider",
            Self::CachedAuthorized => "cached_authorized",
            Self::Failed => "failed",
        }
    }
}

/// Optional explicit authorization envelope for content the host is legally authorized to process
/// (e.g. licensed/royalty-free direct HTTPS audio mirror associated with a YouTube release).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizedStreamProof {
    pub direct_https_audio_url: String,
    pub license_or_authorization_note: String,
    pub expected_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAnalysisResult {
    pub duration_ms: i64,
    pub original_bpm: f64,
    pub half_time_bpm: f64,
    pub double_time_bpm: f64,
    pub bpm_confidence: f64,
    pub musical_key: String,
    pub camelot_key: String,
    pub key_confidence: f64,
    pub beat_grid_json: String,
    pub phrase_markers_json: String,
    pub overall_energy: f64,
    pub danceability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutcome {
    pub parsed: ParsedYouTubeUrl,
    pub metadata: GlobalSongMetadata,
    pub metadata_source: String, // 'local_cache' | 'global_catalogue' | 'oembed' | 'local_analysis'
    pub acquisition_status: AcquisitionStatus,
    pub acquisition_note: String,
    pub cached_entry: Option<LocalTempCacheEntry>,
    pub local_analysis: Option<LocalAnalysisResult>,
    pub sync_outbox_id: Option<String>,
}

/// Stage 1: Strict URL Validation & Source Identification (YouTube / YouTube Music ONLY).
pub fn stage_1_validate_and_identify(raw_url: &str) -> Result<ParsedYouTubeUrl> {
    parse_youtube_url(raw_url)
}

/// Stage 2: Existing Metadata Lookup (Local SQLite Catalogue -> Global Cloud Catalogue -> Official oEmbed).
pub async fn stage_2_lookup_existing_metadata(
    parsed: &ParsedYouTubeUrl,
    db: &Database,
    sync_mgr: &MetadataSyncManager,
    http_client: &reqwest::Client,
) -> (GlobalSongMetadata, String) {
    // 2a. Local SQLite reusable metadata cache (100% offline capable)
    if let Ok(Some(local_hit)) = db.lookup_global_song_by_video_id(&parsed.video_id) {
        return (local_hit, "local_cache".to_string());
    }

    // 2b. Remote Global Shared Song Metadata Catalogue (authenticated engine-only lookup when online)
    let lookup_req = CatalogueLookupRequest {
        source_url: Some(parsed.canonical_url.clone()),
        provider_video_id: Some(parsed.video_id.clone()),
        title: None,
        artist: None,
        version_tag: None,
        duration_ms: None,
    };
    if let Ok(Some(cloud_hit)) = sync_mgr.lookup_global_song_metadata(&lookup_req).await {
        // Persist locally so subsequent lookups work offline
        let _ = db.upsert_global_song_metadata(
            &cloud_hit,
            Some((
                &parsed.canonical_url,
                parsed.provider.as_str(),
                &parsed.video_id,
            )),
        );
        return (cloud_hit, "global_catalogue".to_string());
    }

    // 2c. Official public YouTube oEmbed metadata fallback (title, creator, thumbnail only)
    let oembed = fetch_youtube_metadata(parsed, http_client).await;
    let version_tag = extract_version_tag(&oembed.title);
    let fp = compute_canonical_fingerprint(&oembed.title, &oembed.artist, &version_tag, 0);
    let now = chrono::Utc::now().timestamp_millis();

    let fresh = GlobalSongMetadata {
        song_id: format!("gsng_{}", &Uuid::new_v4().to_string().replace('-', "")[..16]),
        canonical_fingerprint: fp,
        title: oembed.title,
        artist: oembed.artist,
        version_tag,
        duration_ms: 0,
        original_bpm: None,
        half_time_bpm: None,
        double_time_bpm: None,
        bpm_confidence: None,
        musical_key: None,
        camelot_key: None,
        key_confidence: None,
        beat_grid_json: None,
        phrase_markers_json: None,
        overall_energy: None,
        danceability: None,
        genre_tags: Vec::new(),
        mood_tags: Vec::new(),
        analysis_version: 1,
        analysis_confidence: 0.0,
        processing_status: "metadata_only".to_string(),
        source_urls: vec![parsed.canonical_url.clone()],
        created_at: now,
        updated_at: now,
    };

    let stored = db
        .upsert_global_song_metadata(
            &fresh,
            Some((
                &parsed.canonical_url,
                parsed.provider.as_str(),
                &parsed.video_id,
            )),
        )
        .unwrap_or(fresh);

    (stored, "oembed".to_string())
}

/// Stage 3: Authorised Audio Acquisition (strictly compliant — zero YouTube ripping or circumvention).
pub async fn stage_3_authorized_acquisition(
    parsed: &ParsedYouTubeUrl,
    authorized_stream: Option<&AuthorizedStreamProof>,
    downloader: &AudioDownloader,
    db: &Database,
) -> (AcquisitionStatus, String, Option<PathBuf>) {
    let Some(proof) = authorized_stream else {
        return (
            AcquisitionStatus::RestrictedByProvider,
            "YouTube/YouTube Music URLs provide legal oEmbed metadata only; automated audio extraction/downloading is restricted by provider Terms of Service and is not performed. Supply an explicitly authorized stream mirror to cache and analyze audio locally.".to_string(),
            None,
        );
    };

    if proof.license_or_authorization_note.trim().is_empty() {
        return (
            AcquisitionStatus::Failed,
            "Explicit authorization/license declaration is required before acquiring audio.".to_string(),
            None,
        );
    }

    let safe_name = format!("auth_{}", parsed.video_id);
    match downloader
        .download_audio(
            &proof.direct_https_audio_url,
            proof.expected_sha256.as_deref(),
            Some(&safe_name),
            db,
        )
        .await
    {
        Ok(res) => {
            let resolved_path = db
                .get_asset_by_sha256(&res.sha256_hash)
                .ok()
                .flatten()
                .map(|a| PathBuf::from(a.file_path));
            (
                AcquisitionStatus::CachedAuthorized,
                format!("Authorized stream cached locally (sha256={})", &res.sha256_hash[..12.min(res.sha256_hash.len())]),
                resolved_path,
            )
        }
        Err(e) => (
            AcquisitionStatus::Failed,
            format!("Authorized stream acquisition rejected/failed: {}", e),
            None,
        ),
    }
}

/// Stage 4: Local Temporary Audio Caching (10-15 track rolling cap, priority locks, TTL fallback).
pub fn stage_4_cache_temporary_audio(
    cache_mgr: &RollingCacheManager,
    room_id: &str,
    submission_id: Option<&str>,
    song_id: &str,
    file_path: &Path,
    db: &Database,
) -> Result<LocalTempCacheEntry> {
    cache_mgr.register_temp_track(
        room_id,
        submission_id,
        song_id,
        file_path,
        CACHE_PRIORITY_UPCOMING,
        false,
        None,
        DEFAULT_MAX_TEMP_TRACKS,
        db,
    )
}

/// Stage 5: Local Audio Analysis (Symphonia decode + deterministic BPM, half/double-time, Camelot key, beat grid, phrases, energy).
pub fn stage_5_local_audio_analysis(file_path: &Path, song_id: &str) -> Result<LocalAnalysisResult> {
    let full = analyze_track_file(song_id, file_path)?;
    let analysis = full.analysis;

    let orig_bpm = (analysis.bpm * 100.0).round() / 100.0;
    let half_bpm = ((orig_bpm / 2.0) * 100.0).round() / 100.0;
    let double_bpm = ((orig_bpm * 2.0) * 100.0).round() / 100.0;

    Ok(LocalAnalysisResult {
        duration_ms: full.duration_ms as i64,
        original_bpm: orig_bpm,
        half_time_bpm: half_bpm,
        double_time_bpm: double_bpm,
        bpm_confidence: analysis.bpm_confidence,
        musical_key: analysis.key_name,
        camelot_key: analysis.camelot_key,
        key_confidence: analysis.key_confidence,
        beat_grid_json: serde_json::to_string(&full.timing_profile).unwrap_or_else(|_| "{}".to_string()),
        phrase_markers_json: serde_json::to_string(&full.phrase_markers).unwrap_or_else(|_| "[]".to_string()),
        overall_energy: analysis.overall_energy,
        danceability: analysis.dance_suitability,
    })
}

/// Stage 6: Metadata-Only Synchronization with Global Shared Song Metadata Catalogue.
/// Uploads ONLY reusable song metadata when online; queues into `sync_outbox` for retry when offline.
pub async fn stage_6_sync_metadata_only(
    parsed: &ParsedYouTubeUrl,
    metadata: &GlobalSongMetadata,
    sync_mgr: &MetadataSyncManager,
    db: &Database,
) -> Result<String> {
    let upsert_payload = CatalogueUpsertRequest {
        source_url: parsed.canonical_url.clone(),
        title: metadata.title.clone(),
        artist: metadata.artist.clone(),
        version_tag: Some(metadata.version_tag.clone()),
        duration_ms: if metadata.duration_ms > 0 {
            Some(metadata.duration_ms)
        } else {
            None
        },
        original_bpm: metadata.original_bpm,
        half_time_bpm: metadata.half_time_bpm,
        double_time_bpm: metadata.double_time_bpm,
        bpm_confidence: metadata.bpm_confidence,
        musical_key: metadata.musical_key.clone(),
        camelot_key: metadata.camelot_key.clone(),
        key_confidence: metadata.key_confidence,
        beat_grid_json: metadata.beat_grid_json.clone(),
        phrase_markers_json: metadata.phrase_markers_json.clone(),
        overall_energy: metadata.overall_energy,
        danceability: metadata.danceability,
        genre_tags: Some(metadata.genre_tags.clone()),
        mood_tags: Some(metadata.mood_tags.clone()),
        analysis_version: Some(metadata.analysis_version),
        analysis_confidence: Some(metadata.analysis_confidence),
        processing_status: Some(metadata.processing_status.clone()),
    };

    let outbox_id = sync_mgr.enqueue_global_song_metadata(&upsert_payload, &parsed.video_id, db)?;
    // Opportunistic immediate flush when online; silently stays queued if offline
    let _ = sync_mgr.process_pending_outbox(5, db).await;
    Ok(outbox_id)
}

/// Orchestrates all 6 stages cleanly for an incoming YouTube / YouTube Music song submission.
pub async fn run_local_song_pipeline(
    room_id: &str,
    submission_id: &str,
    raw_url: &str,
    authorized_stream: Option<&AuthorizedStreamProof>,
    preexisting_local_cached_audio: Option<&Path>,
    db: &Database,
    cache_mgr: &RollingCacheManager,
    downloader: &AudioDownloader,
    sync_mgr: &MetadataSyncManager,
    http_client: &reqwest::Client,
) -> Result<PipelineOutcome> {
    // Stage 1: Validate URL & identify provider/video ID
    let parsed = stage_1_validate_and_identify(raw_url)?;

    // Stage 2: Lookup existing reusable metadata (local cache -> global catalogue -> official oEmbed)
    let (mut metadata, mut metadata_source) =
        stage_2_lookup_existing_metadata(&parsed, db, sync_mgr, http_client).await;

    // Stage 3: Authorised Audio Acquisition (or reuse pre-verified authorized cache file)
    let (acq_status, acq_note, acquired_path) = if let Some(cached_path) = preexisting_local_cached_audio {
        if cached_path.exists() {
            (
                AcquisitionStatus::CachedAuthorized,
                "Reused locally cached authorized audio file (offline-ready)".to_string(),
                Some(cached_path.to_path_buf()),
            )
        } else {
            stage_3_authorized_acquisition(&parsed, authorized_stream, downloader, db).await
        }
    } else {
        stage_3_authorized_acquisition(&parsed, authorized_stream, downloader, db).await
    };

    // Stage 4 & 5: Temporary caching (10-15 track cap) + Local Audio Analysis
    let mut cached_entry = None;
    let mut local_analysis = None;

    if acq_status == AcquisitionStatus::CachedAuthorized {
        if let Some(ref audio_path) = acquired_path {
            if let Ok(entry) = stage_4_cache_temporary_audio(
                cache_mgr,
                room_id,
                Some(submission_id),
                &metadata.song_id,
                audio_path,
                db,
            ) {
                cached_entry = Some(entry);
            }

            if let Ok(analysis) = stage_5_local_audio_analysis(audio_path, &metadata.song_id) {
                metadata.duration_ms = analysis.duration_ms;
                metadata.original_bpm = Some(analysis.original_bpm);
                metadata.half_time_bpm = Some(analysis.half_time_bpm);
                metadata.double_time_bpm = Some(analysis.double_time_bpm);
                metadata.bpm_confidence = Some(analysis.bpm_confidence);
                metadata.musical_key = Some(analysis.musical_key.clone());
                metadata.camelot_key = Some(analysis.camelot_key.clone());
                metadata.key_confidence = Some(analysis.key_confidence);
                metadata.beat_grid_json = Some(analysis.beat_grid_json.clone());
                metadata.phrase_markers_json = Some(analysis.phrase_markers_json.clone());
                metadata.overall_energy = Some(analysis.overall_energy);
                metadata.danceability = Some(analysis.danceability);
                metadata.analysis_confidence = ((analysis.bpm_confidence + analysis.key_confidence) / 2.0).clamp(0.0, 1.0);
                metadata.processing_status = "analyzed".to_string();
                metadata_source = "local_analysis".to_string();

                let _ = db.upsert_global_song_metadata(
                    &metadata,
                    Some((
                        &parsed.canonical_url,
                        parsed.provider.as_str(),
                        &parsed.video_id,
                    )),
                );

                info!(
                    "Stage 5 local audio analysis complete for '{}': {:.1} BPM ({} / {}), key {}",
                    metadata.title,
                    analysis.original_bpm,
                    analysis.half_time_bpm,
                    analysis.double_time_bpm,
                    analysis.camelot_key
                );
                local_analysis = Some(analysis);
            }
        }
    }

    // Stage 6: Metadata-Only Synchronization (enqueued offline-safe, zero audio bytes)
    let sync_outbox_id = stage_6_sync_metadata_only(&parsed, &metadata, sync_mgr, db).await.ok();

    Ok(PipelineOutcome {
        parsed,
        metadata,
        metadata_source,
        acquisition_status: acq_status,
        acquisition_note: acq_note,
        cached_entry,
        local_analysis,
        sync_outbox_id,
    })
}
