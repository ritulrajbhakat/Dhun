use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::models::GlobalSongMetadata;
use crate::db::Database;
use crate::library::cache::RollingCacheManager;
use crate::library::downloader::{
    probe_and_verify_audio_file, sanitize_custom_filename, validate_safe_public_url,
    AudioDownloader, DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES,
};
use crate::library::ingest::ingest_audio_asset;
use crate::library::pipeline::{
    stage_1_validate_and_identify, stage_2_lookup_existing_metadata,
    stage_4_cache_temporary_audio, stage_5_local_audio_analysis, stage_6_sync_metadata_only,
    AcquisitionStatus, AuthorizedStreamProof, PipelineOutcome,
};
use crate::library::youtube::ParsedYouTubeUrl;
use crate::sync::MetadataSyncManager;

/// Optional host-configured HTTPS media resolver endpoint (`DHUN_AUDIO_RESOLVER_URL`).
/// Receives song metadata + source reference via HTTPS POST and returns either:
/// - JSON `{ "direct_https_audio_url": "https://...", "license_or_authorization_note": "...", "expected_sha256": null }`
/// - or direct audio binary bytes (`Content-Type: audio/*`) verified via Symphonia before caching.
#[derive(Debug, Clone)]
pub struct AuthorizedResolverConfig {
    pub endpoint_url: String,
    pub api_key: Option<String>,
    pub allow_loopback: bool,
}

/// Validates a host-configured loopback resolver URL (`http://127.0.0.1:<port>/...` or `http://localhost:<port>/...`).
/// Still strictly blocks LAN/RFC1918 IPs, cloud metadata, and plain-HTTP remote hosts.
pub fn validate_loopback_resolver_url(url_str: &str) -> Result<(reqwest::Url, std::net::SocketAddr)> {
    let parsed = reqwest::Url::parse(url_str).map_err(|e| anyhow!("Malformed URL: {}", e))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(anyhow!(
            "Loopback resolver must use http or https (got '{}')",
            parsed.scheme()
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Resolver URL missing host"))?
        .to_lowercase();
    if host != "127.0.0.1" && host != "localhost" {
        return Err(anyhow!(
            "Loopback opt-in only permits 127.0.0.1 or localhost (got '{}')",
            host
        ));
    }
    let port = parsed.port_or_known_default().unwrap_or(8000);
    let pinned = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    Ok((parsed, pinned))
}

impl AuthorizedResolverConfig {
    /// Validates that the resolver endpoint is strictly public HTTPS and non-private (anti-SSRF).
    pub fn new(endpoint_url: String, api_key: Option<String>) -> Result<Self> {
        Self::new_with_loopback(endpoint_url, api_key, false)
    }

    /// Validates a resolver endpoint, optionally permitting host-local `http://127.0.0.1:<port>`
    /// when explicitly opted in by the host (`DHUN_ALLOW_LOOPBACK_RESOLVER=1`), eliminating any need for tunnels.
    pub fn new_with_loopback(
        endpoint_url: String,
        api_key: Option<String>,
        allow_loopback: bool,
    ) -> Result<Self> {
        let trimmed = endpoint_url.trim().to_string();
        if trimmed.is_empty() {
            return Err(anyhow!("Resolver URL cannot be empty"));
        }
        let is_loopback_valid = allow_loopback && validate_loopback_resolver_url(&trimmed).is_ok();
        if !is_loopback_valid {
            let _ = validate_safe_public_url(&trimmed)?;
        }
        Ok(Self {
            endpoint_url: trimmed,
            api_key: api_key
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
            allow_loopback: is_loopback_valid,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolverLookupPayload {
    pub source_url: String,
    pub provider_video_id: String,
    pub title: String,
    pub artist: String,
    pub version_tag: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResolverJsonResponse {
    pub direct_https_audio_url: String,
    #[serde(default)]
    pub license_or_authorization_note: Option<String>,
    #[serde(default)]
    pub expected_sha256: Option<String>,
}

/// Downloads an explicitly authorized direct HTTPS audio stream using the correct
/// `(url, expected_sha256, custom_filename, db)` parameter order.
pub async fn download_authorized_proof(
    parsed: &ParsedYouTubeUrl,
    proof: &AuthorizedStreamProof,
    downloader: &AudioDownloader,
    db: &Database,
) -> (AcquisitionStatus, String, Option<PathBuf>) {
    if proof.license_or_authorization_note.trim().is_empty() {
        return (
            AcquisitionStatus::Failed,
            "Explicit authorization/license declaration is required before acquiring audio."
                .to_string(),
            None,
        );
    }

    if let Ok((loopback_file_url, loopback_addr)) =
        validate_loopback_resolver_url(&proof.direct_https_audio_url)
    {
        return fetch_and_ingest_loopback_audio(
            parsed,
            &loopback_file_url,
            loopback_addr,
            None,
            proof.expected_sha256.as_deref(),
            downloader.cache_dir(),
            db,
        )
        .await;
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
                format!(
                    "Authorized stream cached locally (sha256={})",
                    &res.sha256_hash[..12.min(res.sha256_hash.len())]
                ),
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

/// Validates raw audio bytes with Symphonia, writes safely to `cache_dir`, and ingests/deduplicates into SQLite.
pub fn save_and_ingest_audio_bytes_to_cache(
    bytes: &[u8],
    video_id: &str,
    source_url: &str,
    cache_dir: &Path,
    db: &Database,
) -> Result<(PathBuf, String)> {
    if bytes.is_empty() {
        return Err(anyhow!("Resolver returned an empty audio body"));
    }
    if bytes.len() as i64 > DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES {
        return Err(anyhow!(
            "Resolver audio payload exceeds maximum size limit ({} bytes)",
            DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES
        ));
    }

    fs::create_dir_all(cache_dir)?;
    let temp_id = format!("res_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let part_path = cache_dir.join(format!("{}.part", temp_id));

    {
        let mut f = File::create(&part_path)?;
        f.write_all(bytes)?;
        f.flush()?;
    }

    let detected_ext = match probe_and_verify_audio_file(&part_path) {
        Ok(ext) => ext,
        Err(e) => {
            let _ = fs::remove_file(&part_path);
            return Err(anyhow!(
                "Resolver payload is not a valid decodable audio file: {}",
                e
            ));
        }
    };

    let safe_base = sanitize_custom_filename(&format!("resolved_{}", video_id), &temp_id);
    let dest_path = cache_dir.join(format!("{}_{}.{}", safe_base, &temp_id[4..10], detected_ext));
    if let Err(e) = fs::rename(&part_path, &dest_path) {
        let _ = fs::remove_file(&part_path);
        return Err(anyhow!("Failed to finalize cached audio file: {}", e));
    }

    let ingest_res =
        match ingest_audio_asset(&dest_path, "authorized_resolver", Some(source_url), true, db) {
            Ok(r) => r,
            Err(e) => {
                let _ = fs::remove_file(&dest_path);
                return Err(anyhow!("Audio ingestion/analysis failed: {}", e));
            }
        };

    let canonical_path = if ingest_res.is_duplicate {
        let _ = fs::remove_file(&dest_path);
        db.get_asset_by_sha256(&ingest_res.sha256_hash)?
            .map(|a| PathBuf::from(a.file_path))
            .unwrap_or(dest_path)
    } else {
        dest_path
    };

    Ok((canonical_path, ingest_res.sha256_hash))
}

/// Fetches audio bytes from a strictly validated loopback address (127.0.0.1 or localhost),
/// applies streaming size limits, validates the format with Symphonia, and ingests/deduplicates into SQLite.
pub async fn fetch_and_ingest_loopback_audio(
    parsed: &ParsedYouTubeUrl,
    loopback_file_url: &reqwest::Url,
    loopback_addr: std::net::SocketAddr,
    api_key: Option<&str>,
    expected_sha256: Option<&str>,
    cache_dir: &Path,
    db: &Database,
) -> (AcquisitionStatus, String, Option<PathBuf>) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .redirect(Policy::none())
        .resolve("localhost", loopback_addr)
        .resolve("127.0.0.1", loopback_addr)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                AcquisitionStatus::Failed,
                format!("Failed to initialize loopback HTTP client: {}", e),
                None,
            );
        }
    };

    let mut file_req = client.get(loopback_file_url.clone());
    if let Some(key) = api_key {
        file_req = file_req.bearer_auth(key);
    }

    let file_resp = match file_req.send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            return (
                AcquisitionStatus::Failed,
                format!("Loopback resolver audio file returned HTTP {}", r.status()),
                None,
            );
        }
        Err(e) => {
            return (
                AcquisitionStatus::Failed,
                format!("Loopback resolver audio file fetch failed: {}", e),
                None,
            );
        }
    };

    if let Some(len) = file_resp.content_length() {
        if len as i64 > DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES {
            return (
                AcquisitionStatus::Failed,
                format!("Loopback audio size ({} bytes) exceeds 500 MB limit", len),
                None,
            );
        }
    }

    let mut file_buf = Vec::new();
    let mut file_stream = file_resp.bytes_stream();
    while let Some(chunk_res) = file_stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                if (file_buf.len() + chunk.len()) as i64 > DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES {
                    return (
                        AcquisitionStatus::Failed,
                        "Loopback audio stream exceeded 500 MB limit".to_string(),
                        None,
                    );
                }
                file_buf.extend_from_slice(&chunk);
            }
            Err(e) => {
                return (
                    AcquisitionStatus::Failed,
                    format!("Loopback audio stream read failed: {}", e),
                    None,
                );
            }
        }
    }

    match save_and_ingest_audio_bytes_to_cache(
        &file_buf,
        &parsed.video_id,
        loopback_file_url.as_str(),
        cache_dir,
        db,
    ) {
        Ok((path, sha256)) => {
            if let Some(expected) = expected_sha256 {
                if !sha256.eq_ignore_ascii_case(expected.trim()) {
                    return (
                        AcquisitionStatus::Failed,
                        format!(
                            "Loopback audio SHA-256 mismatch: expected '{}', got '{}'",
                            expected, sha256
                        ),
                        None,
                    );
                }
            }
            (
                AcquisitionStatus::CachedAuthorized,
                format!(
                    "Audio fetched from loopback resolver and cached locally (sha256={})",
                    &sha256[..12.min(sha256.len())]
                ),
                Some(path),
            )
        }
        Err(e) => (
            AcquisitionStatus::Failed,
            format!("Loopback audio validation/caching failed: {}", e),
            None,
        ),
    }
}

/// Calls the host-configured HTTPS media resolver (`DHUN_AUDIO_RESOLVER_URL`) with song metadata/URL.
pub async fn acquire_via_authorized_resolver(
    parsed: &ParsedYouTubeUrl,
    metadata: &GlobalSongMetadata,
    resolver: &AuthorizedResolverConfig,
    downloader: &AudioDownloader,
    db: &Database,
) -> (AcquisitionStatus, String, Option<PathBuf>) {
    let validation_res = if resolver.allow_loopback {
        validate_loopback_resolver_url(&resolver.endpoint_url)
            .or_else(|_| validate_safe_public_url(&resolver.endpoint_url))
    } else {
        validate_safe_public_url(&resolver.endpoint_url)
    };
    let (validated_url, pinned_addr) = match validation_res {
        Ok(v) => v,
        Err(e) => {
            warn!("Resolver URL failed security validation: {}", e);
            return (
                AcquisitionStatus::Failed,
                format!("Configured audio resolver rejected by security policy: {}", e),
                None,
            );
        }
    };

    let host = match validated_url.host_str() {
        Some(h) => h,
        None => {
            return (
                AcquisitionStatus::Failed,
                "Configured audio resolver URL is missing a valid host".to_string(),
                None,
            );
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .redirect(Policy::none())
        .resolve(host, pinned_addr)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                AcquisitionStatus::Failed,
                format!("Failed to initialize pinned resolver HTTP client: {}", e),
                None,
            );
        }
    };

    let payload = ResolverLookupPayload {
        source_url: parsed.canonical_url.clone(),
        provider_video_id: parsed.video_id.clone(),
        title: metadata.title.clone(),
        artist: metadata.artist.clone(),
        version_tag: metadata.version_tag.clone(),
    };

    let mut req = client.post(validated_url.clone()).json(&payload);
    if let Some(ref key) = resolver.api_key {
        req = req.bearer_auth(key);
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                AcquisitionStatus::Failed,
                format!("Audio resolver request failed: {}", e),
                None,
            );
        }
    };

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return (
            AcquisitionStatus::RestrictedByProvider,
            "No free/authorized alternative audio found on configured resolver server.".to_string(),
            None,
        );
    }

    if !resp.status().is_success() {
        return (
            AcquisitionStatus::Failed,
            format!("Audio resolver returned HTTP {}", resp.status()),
            None,
        );
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    // Mode A: Resolver returned JSON containing a direct HTTPS audio URL
    if content_type.contains("application/json") {
        let json_body: ResolverJsonResponse = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                return (
                    AcquisitionStatus::Failed,
                    format!("Audio resolver returned invalid JSON response: {}", e),
                    None,
                );
            }
        };

        // If the direct audio URL points to loopback (127.0.0.1 or localhost),
        // fetch it directly via dedicated safe loopback client
        if let Ok((loopback_file_url, loopback_addr)) =
            validate_loopback_resolver_url(&json_body.direct_https_audio_url)
        {
            return fetch_and_ingest_loopback_audio(
                parsed,
                &loopback_file_url,
                loopback_addr,
                resolver.api_key.as_deref(),
                json_body.expected_sha256.as_deref(),
                downloader.cache_dir(),
                db,
            )
            .await;
        }

        let proof = AuthorizedStreamProof {
            direct_https_audio_url: json_body.direct_https_audio_url,
            license_or_authorization_note: json_body
                .license_or_authorization_note
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| {
                    "Resolved via host-configured authorized HTTPS media server".to_string()
                }),
            expected_sha256: json_body.expected_sha256,
        };
        return download_authorized_proof(parsed, &proof, downloader, db).await;
    }

    // Mode B: Resolver streamed audio file bytes directly (`audio/*` or `application/octet-stream`)
    if content_type.starts_with("text/html")
        || content_type.starts_with("text/plain")
        || content_type.starts_with("application/xml")
    {
        return (
            AcquisitionStatus::Failed,
            format!("Audio resolver returned non-audio Content-Type '{}'", content_type),
            None,
        );
    }

    if let Some(len) = resp.content_length() {
        if len as i64 > DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES {
            return (
                AcquisitionStatus::Failed,
                format!("Resolver audio size ({} bytes) exceeds 500 MB limit", len),
                None,
            );
        }
    }

    let mut buf = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                if (buf.len() + chunk.len()) as i64 > DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES {
                    return (
                        AcquisitionStatus::Failed,
                        "Resolver audio stream exceeded 500 MB limit".to_string(),
                        None,
                    );
                }
                buf.extend_from_slice(&chunk);
            }
            Err(e) => {
                return (
                    AcquisitionStatus::Failed,
                    format!("Audio resolver stream read failed: {}", e),
                    None,
                );
            }
        }
    }

    match save_and_ingest_audio_bytes_to_cache(
        &buf,
        &parsed.video_id,
        &resolver.endpoint_url,
        downloader.cache_dir(),
        db,
    ) {
        Ok((path, sha256)) => (
            AcquisitionStatus::CachedAuthorized,
            format!(
                "Audio resolved and cached locally from configured HTTPS server (sha256={})",
                &sha256[..12.min(sha256.len())]
            ),
            Some(path),
        ),
        Err(e) => (
            AcquisitionStatus::Failed,
            format!("Resolver audio validation/caching failed: {}", e),
            None,
        ),
    }
}

/// Orchestrates all 6 stages cleanly, optionally consulting a host-configured HTTPS media resolver
/// (`DHUN_AUDIO_RESOLVER_URL`) when no explicit `AuthorizedStreamProof` was attached to the request.
#[allow(clippy::too_many_arguments)]
pub async fn run_local_song_pipeline_with_resolver(
    room_id: &str,
    submission_id: &str,
    raw_url: &str,
    authorized_stream: Option<&AuthorizedStreamProof>,
    audio_resolver: Option<&AuthorizedResolverConfig>,
    preexisting_local_cached_audio: Option<&Path>,
    db: &Database,
    cache_mgr: &RollingCacheManager,
    downloader: &AudioDownloader,
    sync_mgr: &MetadataSyncManager,
    http_client: &reqwest::Client,
) -> Result<PipelineOutcome> {
    let parsed = stage_1_validate_and_identify(raw_url)?;
    let (mut metadata, mut metadata_source) =
        stage_2_lookup_existing_metadata(&parsed, db, sync_mgr, http_client).await;

    let (acq_status, acq_note, acquired_path) = if let Some(cached_path) = preexisting_local_cached_audio {
        if cached_path.exists() {
            (
                AcquisitionStatus::CachedAuthorized,
                "Reused locally cached authorized audio file (offline-ready)".to_string(),
                Some(cached_path.to_path_buf()),
            )
        } else if let Some(proof) = authorized_stream {
            download_authorized_proof(&parsed, proof, downloader, db).await
        } else if let Some(resolver) = audio_resolver {
            acquire_via_authorized_resolver(&parsed, &metadata, resolver, downloader, db).await
        } else {
            (
                AcquisitionStatus::RestrictedByProvider,
                "YouTube/YouTube Music URLs provide legal oEmbed metadata only; automated audio extraction is not performed.".to_string(),
                None,
            )
        }
    } else if let Some(proof) = authorized_stream {
        download_authorized_proof(&parsed, proof, downloader, db).await
    } else if let Some(resolver) = audio_resolver {
        acquire_via_authorized_resolver(&parsed, &metadata, resolver, downloader, db).await
    } else {
        (
            AcquisitionStatus::RestrictedByProvider,
            "YouTube/YouTube Music URLs provide legal oEmbed metadata only; automated audio extraction is not performed.".to_string(),
            None,
        )
    };

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
                metadata.analysis_confidence =
                    ((analysis.bpm_confidence + analysis.key_confidence) / 2.0).clamp(0.0, 1.0);
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

    let sync_outbox_id = stage_6_sync_metadata_only(&parsed, &metadata, sync_mgr, db)
        .await
        .ok();

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
