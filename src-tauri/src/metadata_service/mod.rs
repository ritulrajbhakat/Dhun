use std::sync::Arc;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::models::GlobalSongMetadata;
use crate::db::Database;
use crate::library::youtube::parse_youtube_url;
use crate::sync::verify_signature;

/// Maximum payload size for metadata-only sync requests (256 KB).
/// Audio binaries are strictly forbidden on the Global Metadata Catalogue service.
pub const MAX_METADATA_PAYLOAD_BYTES: usize = 256 * 1024;

/// Detects substantive recording variant tags so remixes, live recordings, edits,
/// acoustic sessions, and extended versions remain distinct identities in the global catalogue.
pub fn extract_version_tag(title: &str) -> String {
    let lower = title.to_lowercase();
    if lower.contains("remix") || lower.contains("rmx") || lower.contains("bootleg") {
        return "remix".to_string();
    }
    if lower.contains("mashup") || lower.contains("megamix") {
        return "mashup".to_string();
    }
    if lower.contains("live") || lower.contains("concert") || lower.contains("in session") {
        return "live".to_string();
    }
    if lower.contains("unplugged") || lower.contains("acoustic") {
        return "acoustic".to_string();
    }
    if lower.contains("extended") || lower.contains("club mix") || lower.contains("club edit") {
        return "extended".to_string();
    }
    if lower.contains("radio edit") || lower.contains("short edit") || lower.contains("dj edit") {
        return "edit".to_string();
    }
    if lower.contains("lofi") || lower.contains("lo-fi") || lower.contains("slowed") {
        return "lofi_slowed".to_string();
    }
    if lower.contains("instrumental") || lower.contains("karaoke") {
        return "instrumental".to_string();
    }
    if lower.contains("reprise") {
        return "reprise".to_string();
    }
    "original".to_string()
}

/// Normalizes title/artist strings by stripping upload noise (official video, 4k, lyrical, etc.)
/// while keeping substantive recording identity intact.
pub fn normalize_recording_tokens(title: &str, artist: &str) -> String {
    let mut combined = format!("{} {}", title, artist).to_lowercase();
    for noise in [
        "official music video",
        "official video",
        "official audio",
        "audio video",
        "full video song",
        "full video",
        "full song",
        "lyrical video",
        "lyrics video",
        "with lyrics",
        "4k video",
        "hd video",
        "(official)",
        "[official]",
        "(audio)",
        "[audio]",
        "audio",
        "video",
        "vevo",
    ] {
        combined = combined.replace(noise, " ");
    }

    let mut tokens: Vec<String> = combined
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    tokens.sort();
    tokens.dedup();
    tokens.join(" ")
}

fn normalize_text_core(input: &str) -> String {
    normalize_recording_tokens(input, "")
}

fn _unused_old_norm(input: &str) -> String {
    let mut s = input.to_lowercase();
    for noise in [
        "official music video",
        "official video",
        "official audio",
        "full video song",
        "full song",
        "lyrical video",
        "lyrics video",
        "with lyrics",
        "4k video",
        "hd video",
        "(official)",
        "[official]",
        "vevo",
    ] {
        s = s.replace(noise, " ");
    }
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Computes a deterministic canonical fingerprint distinguishing remixes, live versions,
/// edits, and materially different recording durations (8-second tolerance bucket).
pub fn compute_canonical_fingerprint(
    title: &str,
    artist: &str,
    version_tag: &str,
    duration_ms: i64,
) -> String {
    let tokens = normalize_recording_tokens(title, artist);
    let duration_bucket = if duration_ms > 0 {
        (duration_ms / 8000).to_string()
    } else {
        "unk".to_string()
    };
    let canonical = format!(
        "{}|{}|{}",
        tokens,
        version_tag.trim().to_lowercase(),
        duration_bucket
    );
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Computes the duration-independent recording fingerprint (identifies composition, performer, and version).
/// This enables cross-upload song matching across different YouTube uploads of the same recording before audio is downloaded.
pub fn compute_recording_fingerprint(
    title: &str,
    artist: &str,
    version_tag: &str,
) -> String {
    compute_canonical_fingerprint(title, artist, version_tag, 0)
}

#[derive(Clone)]
pub struct GlobalCatalogueState {
    pub db: Database,
    pub service_api_key: String,
}

impl GlobalCatalogueState {
    pub fn new(db: Database, service_api_key: String) -> Self {
        Self {
            db,
            service_api_key,
        }
    }
}

/// Verifies strict engine-to-service HMAC-SHA256 authentication.
/// Browser clients NEVER possess service credentials and cannot access the catalogue.
fn verify_engine_auth(
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body_str: &str,
    expected_key: &str,
) -> Result<(), (StatusCode, String)> {
    if expected_key.trim().is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Global Metadata Catalogue service key is not configured".to_string(),
        ));
    }

    let sig = headers
        .get("x-dhun-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Automated DJ Engine HMAC authentication required (X-Dhun-Signature missing)".to_string(),
            )
        })?;

    let ts_str = headers
        .get("x-dhun-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Automated DJ Engine authentication timestamp required (X-Dhun-Timestamp missing)".to_string(),
            )
        })?;

    let timestamp: i64 = ts_str.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid X-Dhun-Timestamp header".to_string(),
        )
    })?;

    if !verify_signature(expected_key, method, path, timestamp, body_str, sig) {
        warn!("Rejected unauthorized or tampered Global Catalogue request to {}", path);
        return Err((
            StatusCode::FORBIDDEN,
            "Invalid HMAC-SHA256 signature or expired replay window".to_string(),
        ));
    }

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogueLookupRequest {
    pub source_url: Option<String>,
    pub provider_video_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub version_tag: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogueLookupResponse {
    pub found: bool,
    pub metadata: Option<GlobalSongMetadata>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogueUpsertRequest {
    pub source_url: String,
    pub title: String,
    pub artist: String,
    pub version_tag: Option<String>,
    pub duration_ms: Option<i64>,
    pub original_bpm: Option<f64>,
    pub half_time_bpm: Option<f64>,
    pub double_time_bpm: Option<f64>,
    pub bpm_confidence: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot_key: Option<String>,
    pub key_confidence: Option<f64>,
    pub beat_grid_json: Option<String>,
    pub phrase_markers_json: Option<String>,
    pub overall_energy: Option<f64>,
    pub danceability: Option<f64>,
    pub genre_tags: Option<Vec<String>>,
    pub mood_tags: Option<Vec<String>>,
    pub analysis_version: Option<i32>,
    pub analysis_confidence: Option<f64>,
    pub processing_status: Option<String>,
}

pub async fn catalogue_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "dhun-global-metadata-catalogue",
        "version": "2.0.0",
        "party_management": false,
        "audio_storage": false,
        "engine_only_access": true,
        "timestamp": chrono::Utc::now().timestamp_millis()
    }))
}

pub async fn catalogue_lookup(
    State(state): State<Arc<GlobalCatalogueState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<CatalogueLookupResponse>, (StatusCode, String)> {
    let body_str = std::str::from_utf8(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "UTF-8 JSON payload required".to_string()))?;

    verify_engine_auth(
        &headers,
        "POST",
        "/api/v1/catalogue/lookup",
        body_str,
        &state.service_api_key,
    )?;

    let req: CatalogueLookupRequest = serde_json::from_str(body_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid lookup payload: {}", e)))?;

    // 1. Lookup by parsed source URL / video ID
    if let Some(ref raw_url) = req.source_url {
        if let Ok(parsed) = parse_youtube_url(raw_url) {
            if let Ok(Some(song)) = state.db.lookup_global_song_by_video_id(&parsed.video_id) {
                return Ok(Json(CatalogueLookupResponse {
                    found: true,
                    metadata: Some(song),
                }));
            }
        }
        if let Ok(Some(song)) = state.db.lookup_global_song_by_url(raw_url) {
            return Ok(Json(CatalogueLookupResponse {
                found: true,
                metadata: Some(song),
            }));
        }
    }

    // 2. Lookup by direct video ID
    if let Some(ref vid) = req.provider_video_id {
        if let Ok(Some(song)) = state.db.lookup_global_song_by_video_id(vid) {
            return Ok(Json(CatalogueLookupResponse {
                found: true,
                metadata: Some(song),
            }));
        }
    }

    // 3. Lookup by canonical song identity (respecting version tag & duration bucket)
    if let (Some(ref title), Some(ref artist)) = (req.title, req.artist) {
        let vtag = req
            .version_tag
            .clone()
            .unwrap_or_else(|| extract_version_tag(title));
        let fp = compute_canonical_fingerprint(title, artist, &vtag, req.duration_ms.unwrap_or(0));
        if let Ok(Some(song)) = state.db.lookup_global_song_by_fingerprint(&fp) {
            return Ok(Json(CatalogueLookupResponse {
                found: true,
                metadata: Some(song),
            }));
        }
    }

    Ok(Json(CatalogueLookupResponse {
        found: false,
        metadata: None,
    }))
}

pub async fn catalogue_upsert(
    State(state): State<Arc<GlobalCatalogueState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<GlobalSongMetadata>, (StatusCode, String)> {
    let body_str = std::str::from_utf8(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "UTF-8 JSON payload required".to_string()))?;

    verify_engine_auth(
        &headers,
        "POST",
        "/api/v1/catalogue/upsert",
        body_str,
        &state.service_api_key,
    )?;

    // Reject any attempt to upload audio binaries or party/guest state into the global metadata service
    let raw_val: serde_json::Value = serde_json::from_str(body_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;
    if let Some(obj) = raw_val.as_object() {
        for forbidden in [
            "audio_bytes",
            "audio_base64",
            "audio_data",
            "file_bytes",
            "party_id",
            "room_id",
            "guest_id",
            "guest_requests",
            "playlist",
        ] {
            if obj.contains_key(forbidden) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Policy violation: field '{}' is forbidden. Global catalogue stores reusable song metadata only — never audio files or party state.",
                        forbidden
                    ),
                ));
            }
        }
    }

    let req: CatalogueUpsertRequest = serde_json::from_value(raw_val)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid metadata payload: {}", e)))?;

    let parsed = parse_youtube_url(&req.source_url).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Only official YouTube / YouTube Music source URLs are accepted: {}", e),
        )
    })?;

    let clean_title = req.title.trim();
    let clean_artist = req.artist.trim();
    if clean_title.is_empty() || clean_artist.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Title and artist are required".to_string(),
        ));
    }

    let version_tag = req
        .version_tag
        .unwrap_or_else(|| extract_version_tag(clean_title));
    let duration_ms = req.duration_ms.unwrap_or(0);
    let fingerprint = compute_canonical_fingerprint(clean_title, clean_artist, &version_tag, duration_ms);

    let (half_bpm, double_bpm) = match req.original_bpm {
        Some(b) if b > 0.0 => (
            req.half_time_bpm.or(Some((b / 2.0 * 100.0).round() / 100.0)),
            req.double_time_bpm.or(Some((b * 2.0 * 100.0).round() / 100.0)),
        ),
        _ => (req.half_time_bpm, req.double_time_bpm),
    };

    let now = chrono::Utc::now().timestamp_millis();
    let record = GlobalSongMetadata {
        song_id: format!("gsng_{}", &Uuid::new_v4().to_string().replace('-', "")[..16]),
        canonical_fingerprint: fingerprint,
        title: clean_title.to_string(),
        artist: clean_artist.to_string(),
        version_tag,
        duration_ms,
        original_bpm: req.original_bpm,
        half_time_bpm: half_bpm,
        double_time_bpm: double_bpm,
        bpm_confidence: req.bpm_confidence,
        musical_key: req.musical_key,
        camelot_key: req.camelot_key,
        key_confidence: req.key_confidence,
        beat_grid_json: req.beat_grid_json,
        phrase_markers_json: req.phrase_markers_json,
        overall_energy: req.overall_energy,
        danceability: req.danceability,
        genre_tags: req.genre_tags.unwrap_or_default(),
        mood_tags: req.mood_tags.unwrap_or_default(),
        analysis_version: req.analysis_version.unwrap_or(1),
        analysis_confidence: req.analysis_confidence.unwrap_or(0.0),
        processing_status: req
            .processing_status
            .unwrap_or_else(|| "metadata_only".to_string()),
        source_urls: vec![parsed.canonical_url.clone()],
        created_at: now,
        updated_at: now,
    };

    let saved = state
        .db
        .upsert_global_song_metadata(
            &record,
            Some((
                &parsed.canonical_url,
                parsed.provider.as_str(),
                &parsed.video_id,
            )),
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to persist global song metadata: {}", e),
            )
        })?;

    info!(
        "Global Metadata Catalogue upserted song '{}' ({}) [version={}, status={}]",
        saved.title, saved.song_id, saved.version_tag, saved.processing_status
    );

    Ok(Json(saved))
}

/// Strictly engine-only Global Shared Song Metadata Catalogue router.
/// Contains ZERO party room endpoints, ZERO guest endpoints, ZERO public catalogue browsing/search.
pub fn create_global_catalogue_router(state: Arc<GlobalCatalogueState>) -> Router {
    Router::new()
        .route("/health", get(catalogue_health))
        .route("/api/v1/health", get(catalogue_health))
        .route("/api/v1/catalogue/lookup", post(catalogue_lookup))
        .route("/api/v1/catalogue/upsert", post(catalogue_upsert))
        .layer(DefaultBodyLimit::max(MAX_METADATA_PAYLOAD_BYTES))
        .with_state(state)
}
