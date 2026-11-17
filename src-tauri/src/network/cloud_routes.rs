use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    extract::{
        ConnectInfo, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::*;
use crate::library::sources::{
    fetch_youtube_metadata, parse_youtube_url, MusicSourceProvider, SourcePlayabilityStatus,
};
use crate::network::qr::generate_party_qr_svg;
use crate::network::rate_limit::RateLimiter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyEventMessage {
    pub party_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct CloudServerState {
    pub db: Database,
    pub public_guest_url: String,
    pub cors_origin: String,
    pub reqwest_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
    pub ws_broadcaster: broadcast::Sender<PartyEventMessage>,
}

impl CloudServerState {
    pub fn new(db: Database, public_guest_url: String, cors_origin: String) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            db,
            public_guest_url,
            cors_origin,
            reqwest_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            rate_limiter: RateLimiter::new(10, 60), // 10 requests/min per IP/guest
            ws_broadcaster: tx,
        }
    }

    pub fn broadcast(&self, party_id: &str, event_type: &str, payload: serde_json::Value) {
        let now = chrono::Utc::now().timestamp_millis();
        let _ = self.ws_broadcaster.send(PartyEventMessage {
            party_id: party_id.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: now,
        });
    }
}

// ----------------------------------------------------------------------------
// Authentication helpers
// ----------------------------------------------------------------------------

fn extract_bearer_token(headers: &HeaderMap, custom_header: &str) -> Option<String> {
    if let Some(auth) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
        let trimmed = auth.trim();
        if trimmed.to_lowercase().starts_with("bearer ") && trimmed.len() > 7 {
            return Some(trimmed[7..].trim().to_string());
        }
    }
    if let Some(token) = headers.get(custom_header).and_then(|h| h.to_str().ok()) {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn verify_host_auth(headers: &HeaderMap, party: &CloudParty) -> Result<(), (StatusCode, String)> {
    let raw_token = extract_bearer_token(headers, "x-host-token").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Host authentication required (Bearer token or X-Host-Token)".to_string(),
        )
    })?;
    let hash = hash_token(&raw_token);
    if hash != party.host_token_hash {
        return Err((StatusCode::FORBIDDEN, "Invalid host token for this party".to_string()));
    }
    Ok(())
}

fn verify_guest_auth(
    headers: &HeaderMap,
    party_id: &str,
    db: &Database,
) -> Result<PartyGuest, (StatusCode, String)> {
    let raw_token = extract_bearer_token(headers, "x-guest-token").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Guest authentication required (Bearer token or X-Guest-Token)".to_string(),
        )
    })?;
    let hash = hash_token(&raw_token);
    let guest = db
        .lookup_party_guest_by_token(party_id, &hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid or expired guest session token".to_string()))?;

    if guest.is_blocked {
        return Err((StatusCode::FORBIDDEN, "This guest has been blocked by host".to_string()));
    }
    Ok(guest)
}

// ----------------------------------------------------------------------------
// Request / Response DTOs
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreatePartyRequest {
    pub party_name: String,
    pub dj_level: Option<String>,
    pub primary_mood_id: Option<String>,
    pub secondary_mood_id: Option<String>,
    pub energy_progression: Option<String>,
    pub allow_guest_requests: Option<bool>,
    pub max_requests_per_guest: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CreatePartyResponse {
    pub party_id: String,
    pub party_name: String,
    pub status: String,
    pub host_token: String,
    pub guest_url: String,
    pub qr_svg: String,
    pub config: PartyConfiguration,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct PartyDetailsResponse {
    pub party: CloudParty,
    pub config: Option<PartyConfiguration>,
    pub guest_url: String,
    pub qr_svg: String,
    pub sources_count: usize,
    pub pending_requests_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub dj_level: Option<String>,
    pub primary_mood_id: Option<String>,
    pub secondary_mood_id: Option<String>,
    pub energy_progression: Option<String>,
    pub allow_guest_requests: Option<bool>,
    pub max_requests_per_guest: Option<i32>,
    pub request_cooldown_mins: Option<i32>,
    pub require_host_approval: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterSourceRequest {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GuestAuthRequest {
    pub nickname: String,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestAuthResponse {
    pub guest_id: String,
    pub token: String,
    pub nickname: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSongRequestPayload {
    pub source_id: String,
    pub custom_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModerationActionPayload {
    pub reason: Option<String>,
}

// ----------------------------------------------------------------------------
// Handlers: Party Lifecycle
// ----------------------------------------------------------------------------

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "dhun-server",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().timestamp_millis()
    }))
}

pub async fn create_party(
    State(state): State<Arc<CloudServerState>>,
    Json(req): Json<CreatePartyRequest>,
) -> Result<Json<CreatePartyResponse>, (StatusCode, String)> {
    let party_name = req.party_name.trim();
    if party_name.is_empty() || party_name.len() > 100 {
        return Err((StatusCode::BAD_REQUEST, "Party name must be between 1 and 100 characters".to_string()));
    }

    let party_id = format!("pty_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let host_token = format!("dhun_host_{}", Uuid::new_v4().to_string().replace('-', ""));
    let host_token_hash = hash_token(&host_token);
    let now = chrono::Utc::now().timestamp_millis();

    let party = CloudParty {
        id: party_id.clone(),
        party_name: party_name.to_string(),
        status: "created".to_string(),
        host_token_hash,
        created_at: now,
        updated_at: now,
        started_at: None,
        ended_at: None,
        last_heartbeat_at: now,
    };

    let config = PartyConfiguration {
        party_id: party_id.clone(),
        dj_level: req.dj_level.unwrap_or_else(|| "balanced".to_string()),
        primary_mood_id: req.primary_mood_id.unwrap_or_else(|| "house_party".to_string()),
        secondary_mood_id: req.secondary_mood_id,
        energy_progression: req.energy_progression.unwrap_or_else(|| "buildup".to_string()),
        allow_guest_requests: req.allow_guest_requests.unwrap_or(true),
        max_requests_per_guest: req.max_requests_per_guest.unwrap_or(3),
        request_cooldown_mins: 10,
        require_host_approval: true,
        updated_at: now,
    };

    state.db.create_cloud_party(&party, &config).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create party: {}", e))
    })?;

    let guest_url = format!("{}/?party={}", state.public_guest_url.trim_end_matches('/'), party_id);
    let qr_svg = generate_party_qr_svg(&guest_url).unwrap_or_default();

    info!("Created cloud party '{}' (ID: {})", party.party_name, party.id);

    Ok(Json(CreatePartyResponse {
        party_id,
        party_name: party.party_name,
        status: party.status,
        host_token,
        guest_url,
        qr_svg,
        config,
        created_at: now,
    }))
}

pub async fn get_party(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
) -> Result<Json<PartyDetailsResponse>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    let config = state.db.get_party_config(&party_id).unwrap_or(None);
    let sources = state.db.get_party_sources(&party_id).unwrap_or_default();
    let requests = state.db.get_party_song_requests(&party_id).unwrap_or_default();
    let pending_count = requests.iter().filter(|r| r.status == "pending").count();

    let guest_url = format!("{}/?party={}", state.public_guest_url.trim_end_matches('/'), party_id);
    let qr_svg = generate_party_qr_svg(&guest_url).unwrap_or_default();

    Ok(Json(PartyDetailsResponse {
        party,
        config,
        guest_url,
        qr_svg,
        sources_count: sources.len(),
        pending_requests_count: pending_count,
    }))
}

pub async fn start_party(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<CloudParty>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    let now = chrono::Utc::now().timestamp_millis();
    state.db.update_cloud_party_status(&party_id, "live", now).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let updated = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.unwrap();

    state.broadcast(&party_id, "party.status_changed", serde_json::json!({
        "status": "live",
        "started_at": updated.started_at
    }));

    let _ = state.db.log_playback_event(&party_id, "party_start", None, None);
    info!("Party '{}' (ID: {}) is now LIVE", party.party_name, party_id);

    Ok(Json(updated))
}

pub async fn end_party(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<CloudParty>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    let now = chrono::Utc::now().timestamp_millis();
    state.db.update_cloud_party_status(&party_id, "ended", now).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let updated = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.unwrap();

    state.broadcast(&party_id, "party.status_changed", serde_json::json!({
        "status": "ended",
        "ended_at": updated.ended_at
    }));

    let _ = state.db.log_playback_event(&party_id, "party_end", None, None);
    info!("Party '{}' (ID: {}) has ENDED", party.party_name, party_id);

    Ok(Json(updated))
}

pub async fn heartbeat_party(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    let now = chrono::Utc::now().timestamp_millis();
    state.db.update_party_heartbeat(&party_id, now).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "last_heartbeat_at": now
    })))
}

pub async fn update_party_config(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<PartyConfiguration>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    let existing = state.db.get_party_config(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.unwrap_or_else(|| PartyConfiguration {
        party_id: party_id.clone(),
        dj_level: "balanced".to_string(),
        primary_mood_id: "house_party".to_string(),
        secondary_mood_id: None,
        energy_progression: "buildup".to_string(),
        allow_guest_requests: true,
        max_requests_per_guest: 3,
        request_cooldown_mins: 10,
        require_host_approval: true,
        updated_at: chrono::Utc::now().timestamp_millis(),
    });

    let now = chrono::Utc::now().timestamp_millis();
    let updated_config = PartyConfiguration {
        party_id: party_id.clone(),
        dj_level: req.dj_level.unwrap_or(existing.dj_level),
        primary_mood_id: req.primary_mood_id.unwrap_or(existing.primary_mood_id),
        secondary_mood_id: req.secondary_mood_id.or(existing.secondary_mood_id),
        energy_progression: req.energy_progression.unwrap_or(existing.energy_progression),
        allow_guest_requests: req.allow_guest_requests.unwrap_or(existing.allow_guest_requests),
        max_requests_per_guest: req.max_requests_per_guest.unwrap_or(existing.max_requests_per_guest),
        request_cooldown_mins: req.request_cooldown_mins.unwrap_or(existing.request_cooldown_mins),
        require_host_approval: req.require_host_approval.unwrap_or(existing.require_host_approval),
        updated_at: now,
    };

    state.db.update_party_config(&updated_config).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    state.broadcast(&party_id, "party.config_updated", serde_json::to_value(&updated_config).unwrap());

    Ok(Json(updated_config))
}

// ----------------------------------------------------------------------------
// Handlers: Online Music Sources
// ----------------------------------------------------------------------------

pub async fn register_source(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<RegisterSourceRequest>,
) -> Result<Json<PartySourceReference>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    let parsed = parse_youtube_url(&req.url).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Invalid YouTube URL: {}", e))
    })?;

    // Check if track is already registered in this party
    if let Ok(Some(existing)) = state.db.get_party_source_by_video_id(&party_id, &parsed.video_id) {
        return Ok(Json(existing));
    }

    let meta = fetch_youtube_metadata(&parsed, &state.reqwest_client).await;
    let now = chrono::Utc::now().timestamp_millis();
    let source_id = format!("src_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());

    let source = PartySourceReference {
        id: source_id,
        party_id: party_id.clone(),
        provider: parsed.provider.as_str().to_string(),
        original_url: parsed.canonical_url,
        provider_video_id: parsed.video_id,
        title: meta.title,
        artist: meta.artist,
        thumbnail_url: meta.thumbnail_url,
        duration_seconds: meta.duration_seconds,
        verification_status: "verified".to_string(),
        playback_method: meta.playback_method,
        playability_status: meta.playability_status.as_str().to_string(),
        metadata_status: meta.metadata_status,
        error_reason: meta.error_reason,
        registered_at: now,
        updated_at: now,
    };

    state.db.insert_party_source(&source).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register source: {}", e))
    })?;

    state.broadcast(&party_id, "party.source_added", serde_json::to_value(&source).unwrap());
    info!("Registered YouTube source '{}' in party {}", source.title, party_id);

    Ok(Json(source))
}

pub async fn list_sources(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
) -> Result<Json<Vec<PartySourceReference>>, (StatusCode, String)> {
    let sources = state.db.get_party_sources(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(sources))
}

// ----------------------------------------------------------------------------
// Handlers: Guests & Requests
// ----------------------------------------------------------------------------

pub async fn guest_auth(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<GuestAuthRequest>,
) -> Result<Json<GuestAuthResponse>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    let clean_nick = req.nickname.trim();
    if clean_nick.is_empty() || clean_nick.len() > 40 {
        return Err((StatusCode::BAD_REQUEST, "Nickname must be between 1 and 40 characters".to_string()));
    }

    let client_ip = addr.ip().to_string();
    let auth_key = format!("auth:{}:{}", client_ip, party_id);
    if !state.rate_limiter.check_and_record(&auth_key) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Please wait before creating another session".to_string()));
    }

    let guest_id = format!("gst_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let token = format!("dhun_guest_{}", Uuid::new_v4().to_string().replace('-', ""));
    let token_hash = hash_token(&token);
    let now = chrono::Utc::now().timestamp_millis();

    let guest = PartyGuest {
        id: guest_id.clone(),
        party_id: party_id.clone(),
        nickname: clean_nick.to_string(),
        fingerprint: req.fingerprint.unwrap_or_else(|| "generic_browser".to_string()),
        token_hash,
        client_ip,
        request_count: 0,
        is_blocked: false,
        created_at: now,
        last_seen_at: now,
    };

    state.db.register_party_guest(&guest).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(GuestAuthResponse {
        guest_id,
        token,
        nickname: clean_nick.to_string(),
    }))
}

pub async fn create_request(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CreateSongRequestPayload>,
) -> Result<Json<PartySongRequest>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    if party.status != "live" && party.status != "created" {
        return Err((StatusCode::BAD_REQUEST, "Party is not currently accepting requests".to_string()));
    }

    let config = state.db.get_party_config(&party_id).unwrap_or(None).unwrap_or_else(|| PartyConfiguration {
        party_id: party_id.clone(),
        dj_level: "balanced".to_string(),
        primary_mood_id: "house_party".to_string(),
        secondary_mood_id: None,
        energy_progression: "buildup".to_string(),
        allow_guest_requests: true,
        max_requests_per_guest: 3,
        request_cooldown_mins: 10,
        require_host_approval: true,
        updated_at: 0,
    });

    if !config.allow_guest_requests {
        return Err((StatusCode::FORBIDDEN, "Guest requests are disabled for this party".to_string()));
    }

    let guest = verify_guest_auth(&headers, &party_id, &state.db)?;

    if guest.request_count >= config.max_requests_per_guest {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("Maximum song request limit reached ({} songs per guest)", config.max_requests_per_guest),
        ));
    }

    let rate_key = format!("req:{}:{}", guest.id, party_id);
    if !state.rate_limiter.check_and_record(&rate_key) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Please wait a moment before submitting another request".to_string()));
    }

    // Verify source exists
    let sources = state.db.get_party_sources(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    let source = sources
        .into_iter()
        .find(|s| s.id == payload.source_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Selected song is not available in party catalog".to_string()))?;

    // Check duplicate active requests
    let existing_requests = state.db.get_party_song_requests(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    let already_queued = existing_requests.iter().any(|r| {
        r.source_id == payload.source_id && (r.status == "pending" || r.status == "approved")
    });
    if already_queued {
        return Err((StatusCode::CONFLICT, "This song has already been requested and is in the queue!".to_string()));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let request_id = format!("req_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let clean_note = payload.custom_note.map(|n| {
        n.trim().chars().filter(|c| !c.is_control()).take(150).collect::<String>()
    });

    let new_request = PartySongRequest {
        id: request_id,
        party_id: party_id.clone(),
        guest_id: guest.id.clone(),
        guest_name: Some(guest.nickname),
        source_id: source.id,
        source_title: Some(source.title),
        source_artist: Some(source.artist),
        thumbnail_url: source.thumbnail_url,
        custom_note: clean_note,
        status: "pending".to_string(),
        rejection_reason: None,
        priority_score: 100.0,
        created_at: now,
        updated_at: now,
    };

    state.db.insert_party_song_request(&new_request).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let _ = state.db.increment_party_guest_request_count(&guest.id);

    state.broadcast(&party_id, "request.created", serde_json::to_value(&new_request).unwrap());
    info!("Guest submitted song request '{}' in party {}", new_request.source_title.as_deref().unwrap_or(""), party_id);

    Ok(Json(new_request))
}

pub async fn list_requests(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<PartySongRequest>>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    let is_host = verify_host_auth(&headers, &party).is_ok();
    let caller_guest_id = if !is_host {
        verify_guest_auth(&headers, &party_id, &state.db).ok().map(|g| g.id)
    } else {
        None
    };

    let mut requests = state.db.get_party_song_requests(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    if !is_host {
        for r in requests.iter_mut() {
            let is_own = caller_guest_id.as_ref().map(|id| id == &r.guest_id).unwrap_or(false);
            if !is_own {
                r.guest_id = "anonymized".to_string();
                r.guest_name = Some("Party Guest".to_string());
                r.custom_note = None;
            }
        }
    }

    Ok(Json(requests))
}

pub async fn approve_request(
    State(state): State<Arc<CloudServerState>>,
    Path((party_id, request_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    state.db.update_party_request_status(&request_id, &party_id, "approved", None, "host", "host").map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    state.broadcast(&party_id, "request.status_changed", serde_json::json!({
        "request_id": request_id,
        "status": "approved"
    }));

    Ok(Json(serde_json::json!({ "status": "approved", "request_id": request_id })))
}

pub async fn reject_request(
    State(state): State<Arc<CloudServerState>>,
    Path((party_id, request_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<ModerationActionPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let party = state.db.get_cloud_party(&party_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?.ok_or_else(|| (StatusCode::NOT_FOUND, "Party not found".to_string()))?;

    verify_host_auth(&headers, &party)?;

    state.db.update_party_request_status(
        &request_id,
        &party_id,
        "rejected",
        payload.reason.as_deref(),
        "host",
        "host",
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state.broadcast(&party_id, "request.status_changed", serde_json::json!({
        "request_id": request_id,
        "status": "rejected",
        "reason": payload.reason
    }));

    Ok(Json(serde_json::json!({ "status": "rejected", "request_id": request_id })))
}

// ----------------------------------------------------------------------------
// WebSocket Handler
// ----------------------------------------------------------------------------

pub async fn ws_party_handler(
    ws: WebSocketUpgrade,
    Path(party_id): Path<String>,
    State(state): State<Arc<CloudServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_party_socket(socket, party_id, state))
}

async fn handle_party_socket(socket: WebSocket, party_id: String, state: Arc<CloudServerState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_broadcaster.subscribe();

    let target_party_id = party_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg.party_id == target_party_id {
                if let Ok(json_str) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(json_str)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

// ----------------------------------------------------------------------------
// Mobile Guest Portal HTML (Self-Contained Browser Client)
// ----------------------------------------------------------------------------

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn serve_party_guest_portal(
    State(state): State<Arc<CloudServerState>>,
    Path(party_id): Path<String>,
) -> Html<String> {
    let party_name = match state.db.get_cloud_party(&party_id) {
        Ok(Some(p)) => p.party_name,
        _ => "DHUN Party".to_string(),
    };
    let clean_party = escape_html(&party_name);

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>DHUN — {clean_party}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: #090a0f;
      color: #f1f5f9;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      padding: 16px;
      max-width: 480px;
      margin: 0 auto;
    }}
    header {{ text-align: center; padding: 16px 0; border-bottom: 1px solid #1e293b; }}
    .logo {{ font-size: 26px; font-weight: 900; color: #f43f5e; letter-spacing: 2px; }}
    .subtitle {{ font-size: 13px; color: #94a3b8; margin-top: 2px; }}
    .badge {{
      display: inline-block;
      margin-top: 8px;
      font-size: 11px;
      font-weight: 700;
      color: #fbbf24;
      background: rgba(245, 158, 11, 0.15);
      border: 1px solid rgba(245, 158, 11, 0.4);
      border-radius: 9999px;
      padding: 4px 12px;
      text-transform: uppercase;
    }}
    .input-group {{ margin-top: 14px; }}
    input[type="text"] {{
      width: 100%;
      background: #111827;
      border: 1px solid #374151;
      border-radius: 12px;
      padding: 12px 16px;
      color: #fff;
      font-size: 14px;
      outline: none;
    }}
    input[type="text"]:focus {{ border-color: #f43f5e; }}
    #catalogList {{
      margin-top: 14px;
      flex: 1;
      overflow-y: auto;
      max-height: 50vh;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }}
    .track-card {{
      background: #111827;
      border: 1px solid #1f2937;
      border-radius: 12px;
      padding: 12px;
      display: flex;
      align-items: center;
      gap: 12px;
    }}
    .thumb {{
      width: 52px;
      height: 52px;
      border-radius: 8px;
      object-fit: cover;
      background: #1f2937;
      flex-shrink: 0;
    }}
    .info {{ flex: 1; min-width: 0; }}
    .title {{ font-size: 14px; font-weight: 700; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    .artist {{ font-size: 12px; color: #9ca3af; margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    .btn-req {{
      background: #e11d48;
      color: #fff;
      font-size: 12px;
      font-weight: 700;
      border: none;
      border-radius: 8px;
      padding: 8px 14px;
      cursor: pointer;
      flex-shrink: 0;
    }}
    .btn-req:active {{ transform: scale(0.96); }}
    .section-title {{ font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; color: #94a3b8; margin: 16px 0 8px; }}
    .req-item {{
      background: #0f172a;
      border: 1px solid #1e293b;
      border-radius: 8px;
      padding: 8px 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-size: 12px;
    }}
    .status-badge {{
      font-size: 10px;
      font-weight: 700;
      padding: 2px 8px;
      border-radius: 6px;
      text-transform: uppercase;
    }}
    .status-pending {{ background: rgba(148, 163, 184, 0.2); color: #cbd5e1; }}
    .status-approved {{ background: rgba(14, 165, 233, 0.2); color: #38bdf8; border: 1px solid rgba(14, 165, 233, 0.4); }}
    .status-playing {{ background: rgba(16, 185, 129, 0.2); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.4); }}
    footer {{ text-align: center; font-size: 11px; color: #475569; margin-top: auto; padding: 14px 0 4px; }}
  </style>
</head>
<body>
  <header>
    <div class="logo">DHUN धुन</div>
    <div class="subtitle">Party Guest Portal</div>
    <div class="badge">{clean_party}</div>
  </header>

  <div class="input-group">
    <input id="guestName" type="text" placeholder="Your name (e.g. Aman)" />
  </div>

  <div class="section-title">Party Music Catalog</div>
  <div id="catalogList">
    <div style="text-align:center; padding: 30px; color:#64748b;">Loading songs...</div>
  </div>

  <div>
    <div class="section-title">My Song Requests</div>
    <div id="myRequestsList" style="display:flex; flex-direction:column; gap:6px;">
      <div style="color:#64748b; font-style:italic; font-size:12px;">No requests submitted yet.</div>
    </div>
  </div>

  <footer>
    DHUN Cloud Party Engine &bull; Host Audio Stream Protected
  </footer>

  <script>
    const partyId = '{party_id}';
    let guestToken = localStorage.getItem('dhun_guest_token_' + partyId);
    let guestId = localStorage.getItem('dhun_guest_id_' + partyId);
    let nameInput = document.getElementById('guestName');
    nameInput.value = localStorage.getItem('dhun_guest_name') || '';
    nameInput.addEventListener('input', (e) => localStorage.setItem('dhun_guest_name', e.target.value));

    async function ensureAuth() {{
      if (guestToken) return guestToken;
      let name = nameInput.value.trim() || 'Party Guest';
      try {{
        let res = await fetch('/api/parties/' + partyId + '/guest/auth', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ nickname: name }})
        }});
        if (res.ok) {{
          let data = await res.json();
          guestToken = data.token;
          guestId = data.guest_id;
          localStorage.setItem('dhun_guest_token_' + partyId, guestToken);
          localStorage.setItem('dhun_guest_id_' + partyId, guestId);
          return guestToken;
        }}
      }} catch (err) {{
        console.error('Auth error:', err);
      }}
      return null;
    }}

    async function loadSources() {{
      try {{
        let res = await fetch('/api/parties/' + partyId + '/sources');
        let sources = await res.json();
        let container = document.getElementById('catalogList');
        container.innerHTML = '';
        if (!sources || sources.length === 0) {{
          let msg = document.createElement('div');
          msg.style.cssText = 'text-align:center; padding:30px; color:#64748b;';
          msg.textContent = 'No songs registered in this party catalog yet. Ask the host to add tracks!';
          container.appendChild(msg);
          return;
        }}
        sources.forEach(s => {{
          let card = document.createElement('div');
          card.className = 'track-card';

          if (s.thumbnail_url) {{
            let img = document.createElement('img');
            img.src = s.thumbnail_url;
            img.className = 'thumb';
            card.appendChild(img);
          }}

          let info = document.createElement('div');
          info.className = 'info';

          let title = document.createElement('div');
          title.className = 'title';
          title.textContent = s.title;

          let artist = document.createElement('div');
          artist.className = 'artist';
          artist.textContent = s.artist;

          info.appendChild(title);
          info.appendChild(artist);

          let btn = document.createElement('button');
          btn.className = 'btn-req';
          btn.textContent = 'Request';
          btn.addEventListener('click', () => requestSong(s.id, s.title));

          card.appendChild(info);
          card.appendChild(btn);
          container.appendChild(card);
        }});
      }} catch (err) {{
        console.error(err);
      }}
    }}

    async function requestSong(sourceId, title) {{
      let token = await ensureAuth();
      if (!token) {{
        alert('Could not authenticate. Please enter your name.');
        return;
      }}
      try {{
        let res = await fetch('/api/parties/' + partyId + '/requests', {{
          method: 'POST',
          headers: {{
            'Content-Type': 'application/json',
            'Authorization': 'Bearer ' + token
          }},
          body: JSON.stringify({{ source_id: sourceId }})
        }});
        if (res.ok) {{
          alert('Request submitted for "' + title + '"! Host notified.');
          loadRequests();
        }} else {{
          let err = await res.text();
          alert('Could not submit request: ' + err);
        }}
      }} catch (err) {{
        alert('Communication error with party server');
      }}
    }}

    async function loadRequests() {{
      try {{
        let headers = {{}};
        if (guestToken) {{
          headers['Authorization'] = 'Bearer ' + guestToken;
        }}
        let res = await fetch('/api/parties/' + partyId + '/requests', {{ headers }});
        let requests = await res.json();
        let my = requests.filter(r => r.guest_id === guestId || r.guest_id !== 'anonymized');
        let container = document.getElementById('myRequestsList');
        container.innerHTML = '';
        if (!my || my.length === 0) {{
          let msg = document.createElement('div');
          msg.style.cssText = 'color:#64748b; font-style:italic; font-size:12px;';
          msg.textContent = 'No requests submitted yet.';
          container.appendChild(msg);
          return;
        }}
        my.forEach(r => {{
          let item = document.createElement('div');
          item.className = 'req-item';

          let title = document.createElement('span');
          title.style.cssText = 'font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; margin-right:8px;';
          title.textContent = r.source_title || 'Song';

          let badge = document.createElement('span');
          badge.className = 'status-badge status-' + r.status;
          badge.textContent = r.status;

          item.appendChild(title);
          item.appendChild(badge);
          container.appendChild(item);
        }});
      }} catch (err) {{
        console.error(err);
      }}
    }}

    loadSources();
    loadRequests();
    setInterval(loadRequests, 5000);
  </script>
</body>
</html>"#,
        clean_party = clean_party,
        party_id = party_id
    );

    Html(html)
}

// ----------------------------------------------------------------------------
// Create Cloud Axum Router
// ----------------------------------------------------------------------------

pub fn create_cloud_router(state: Arc<CloudServerState>) -> Router {
    Router::new()
        // Health
        .route("/health", get(health_check))
        .route("/api/health", get(health_check))
        // Party Management
        .route("/api/parties", post(create_party))
        .route("/api/parties/:id", get(get_party))
        .route("/api/parties/:id/start", post(start_party))
        .route("/api/parties/:id/end", post(end_party))
        .route("/api/parties/:id/heartbeat", post(heartbeat_party))
        .route("/api/parties/:id/config", post(update_party_config))
        // Sources
        .route("/api/parties/:id/sources", post(register_source).get(list_sources))
        // Guests & Requests
        .route("/api/parties/:id/guest/auth", post(guest_auth))
        .route("/api/parties/:id/requests", post(create_request).get(list_requests))
        .route("/api/parties/:id/requests/:request_id/approve", post(approve_request))
        .route("/api/parties/:id/requests/:request_id/reject", post(reject_request))
        // WebSocket
        .route("/ws/parties/:id", get(ws_party_handler))
        // Guest Portal Web UI
        .route("/party/:id", get(serve_party_guest_portal))
        .route("/guest/:id", get(serve_party_guest_portal))
        .with_state(state)
}
