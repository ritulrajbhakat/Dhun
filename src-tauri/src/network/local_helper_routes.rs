use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::{
    body::Body,
    extract::{
        ConnectInfo, DefaultBodyLimit, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HOST, HeaderName, HeaderValue},
        HeaderMap, Method, Request, Response, StatusCode,
    },
    middleware::{self, Next},
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

use crate::db::models::{
    GuestOwnSubmissionView, LocalPartyRoom, LocalRoomGuest, LocalRoomSubmission,
    LocalTempCacheStats, SyncStatusOverview,
};
use crate::db::Database;
use crate::dj::levels::DjLevel;
use crate::dj::moods::{get_mood_by_id as get_mood_preset, get_preset_moods as get_all_mood_presets};
use crate::dj::selector::select_next_submission_without_interrupting;
use crate::library::cache::{RollingCacheManager, DEFAULT_MAX_TEMP_TRACKS};
use crate::library::downloader::AudioDownloader;
use crate::library::pipeline::{run_local_song_pipeline, AuthorizedStreamProof};
use crate::network::ip::{
    detect_reachable_lan_ip, is_allowed_local_host_header, is_lan_or_loopback_ip,
};
use crate::network::qr::generate_party_qr_svg;
use crate::network::rate_limit::RateLimiter;
use crate::sync::MetadataSyncManager;

pub const DEFAULT_LOCAL_HELPER_PORT: u16 = 47821;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRoomEvent {
    pub room_id: String,
    pub event_type: String, // 'room.status_changed' | 'room.config_updated' | 'room.closed' | 'dj.submission_received' | 'dj.evaluated'
    /// If Some(guest_id), this event may also be delivered to that specific guest socket;
    /// if None and host_only == true, ONLY host sockets receive it (strict guest privacy).
    pub target_guest_id: Option<String>,
    pub host_only: bool,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct LocalHelperState {
    pub db: Database,
    pub port: u16,
    pub detected_lan_ip: Option<String>,
    pub cache_manager: RollingCacheManager,
    pub downloader: AudioDownloader,
    pub sync_manager: MetadataSyncManager,
    pub http_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
    pub ws_tx: broadcast::Sender<LocalRoomEvent>,
    pub allowed_frontend_origins: Vec<String>,
}

impl LocalHelperState {
    pub fn new(
        db: Database,
        port: u16,
        cache_dir: PathBuf,
        sync_manager: MetadataSyncManager,
        detected_lan_ip_override: Option<Option<String>>,
        allowed_frontend_origins: Vec<String>,
    ) -> Self {
        let (ws_tx, _) = broadcast::channel(1024);
        let cache_manager = RollingCacheManager::new(&cache_dir, crate::library::cache::DEFAULT_MAX_CACHE_BYTES);
        let downloader = AudioDownloader::new(&cache_dir);
        let detected_lan_ip = match detected_lan_ip_override {
            Some(explicit) => explicit,
            None => detect_reachable_lan_ip(),
        };

        Self {
            db,
            port,
            detected_lan_ip,
            cache_manager,
            downloader,
            sync_manager,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(6))
                .build()
                .unwrap_or_default(),
            rate_limiter: RateLimiter::new(15, 60),
            ws_tx,
            allowed_frontend_origins,
        }
    }

    pub fn broadcast_event(
        &self,
        room_id: &str,
        event_type: &str,
        host_only: bool,
        target_guest_id: Option<String>,
        payload: serde_json::Value,
    ) {
        let _ = self.ws_tx.send(LocalRoomEvent {
            room_id: room_id.to_string(),
            event_type: event_type.to_string(),
            target_guest_id,
            host_only,
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });
    }

    /// Builds a real reachable LAN guest URL & QR SVG. Never fabricates a fake IP!
    pub fn build_lan_guest_qr(&self, room_id: &str, room_access_token: Option<&str>) -> (bool, Option<String>, Option<String>) {
        let Some(ref lan_ip) = self.detected_lan_ip else {
            return (false, None, None);
        };
        let url = match room_access_token {
            Some(tok) => format!("http://{}:{}/join/{}?access={}", lan_ip, self.port, room_id, tok),
            None => format!("http://{}:{}/join/{}", lan_ip, self.port, room_id),
        };
        let qr = generate_party_qr_svg(&url).ok();
        (true, Some(url), qr)
    }
}

// ============================================================================
// Security Guards: LAN-Only IP Enforcement, DNS Rebinding Mitigation, & PNA
// ============================================================================

pub fn hash_secret(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_room_code() -> String {
    let raw = Uuid::new_v4().to_string().replace('-', "").to_uppercase();
    format!("DHUN-{}", &raw[..4])
}

fn extract_token(headers: &HeaderMap, header_name: &str) -> Option<String> {
    if let Some(auth) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        let trimmed = auth.trim();
        if trimmed.to_lowercase().starts_with("bearer ") && trimmed.len() > 7 {
            return Some(trimmed[7..].trim().to_string());
        }
    }
    if let Some(val) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

pub fn verify_lan_and_host_headers(
    peer_ip: std::net::IpAddr,
    headers: &HeaderMap,
    detected_lan_ip: Option<&str>,
) -> Result<(), (StatusCode, String)> {
    // 1. Strict LAN / Loopback IP isolation (external guests outside LAN are rejected)
    if !is_lan_or_loopback_ip(peer_ip) {
        warn!("Blocked non-LAN external IP {} attempting to access local DHUN helper", peer_ip);
        return Err((
            StatusCode::FORBIDDEN,
            "Access denied: DHUN local party rooms are restricted to devices on the host's Wi-Fi/LAN.".to_string(),
        ));
    }

    // 2. DNS Rebinding Mitigation via strict Host header verification
    if let Some(host_val) = headers.get(HOST).and_then(|v| v.to_str().ok()) {
        if !is_allowed_local_host_header(host_val, detected_lan_ip) {
            warn!("Blocked suspected DNS rebinding Host header: '{}'", host_val);
            return Err((
                StatusCode::FORBIDDEN,
                format!("Forbidden Host header '{}': DNS rebinding protection active.", host_val),
            ));
        }
    }

    Ok(())
}

fn verify_host_token(headers: &HeaderMap, room: &LocalPartyRoom) -> Result<(), (StatusCode, String)> {
    let raw = extract_token(headers, "x-host-token").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Host authentication required (Bearer host token or X-Host-Token).".to_string(),
        )
    })?;
    if hash_secret(&raw) != room.host_token_hash {
        return Err((
            StatusCode::FORBIDDEN,
            "Host-only operation: invalid host credentials.".to_string(),
        ));
    }
    Ok(())
}

fn verify_guest_token(
    headers: &HeaderMap,
    room_id: &str,
    db: &Database,
) -> Result<LocalRoomGuest, (StatusCode, String)> {
    let raw = extract_token(headers, "x-guest-token").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Guest session token required.".to_string(),
        )
    })?;
    let hash = hash_secret(&raw);
    db.lookup_local_room_guest(room_id, &hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid guest token or cross-room access denied.".to_string(),
            )
        })
}

/// Middleware enforcing LAN IP isolation, DNS rebinding protection, and Private Network Access headers
/// so the HTTPS Vercel host frontend can communicate securely with `http://127.0.0.1:<port>`.
pub async fn local_security_and_cors_middleware(
    State(state): State<Arc<LocalHelperState>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        if let Err((code, msg)) = verify_lan_and_host_headers(
            addr.ip(),
            req.headers(),
            state.detected_lan_ip.as_deref(),
        ) {
            return (code, msg).into_response();
        }
    }

    let origin_header = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let origin_allowed = origin_header.as_deref().map(|orig| {
        let lower = orig.to_lowercase();
        if lower.starts_with("http://localhost")
            || lower.starts_with("http://127.0.0.1")
            || lower.starts_with("https://ritulrajbhakat.github.io")
            || lower.ends_with(".vercel.app")
        {
            return true;
        }
        if let Some(ref lan) = state.detected_lan_ip {
            if lower.starts_with(&format!("http://{}", lan)) {
                return true;
            }
        }
        state.allowed_frontend_origins.iter().any(|allowed| allowed.eq_ignore_ascii_case(orig))
    }).unwrap_or(false);

    // Handle CORS & Chrome Private Network Access (PNA) preflight
    if req.method() == Method::OPTIONS {
        let mut res = StatusCode::NO_CONTENT.into_response();
        let headers = res.headers_mut();
        if let (true, Some(orig)) = (origin_allowed, origin_header.as_deref()) {
            if let Ok(hv) = HeaderValue::from_str(orig) {
                headers.insert("access-control-allow-origin", hv);
            }
        }
        headers.insert(
            "access-control-allow-methods",
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        headers.insert(
            "access-control-allow-headers",
            HeaderValue::from_static("content-type, authorization, accept, x-host-token, x-guest-token"),
        );
        headers.insert(
            HeaderName::from_static("access-control-allow-private-network"),
            HeaderValue::from_static("true"),
        );
        return res;
    }

    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    if let (true, Some(orig)) = (origin_allowed, origin_header.as_deref()) {
        if let Ok(hv) = HeaderValue::from_str(orig) {
            headers.insert("access-control-allow-origin", hv);
        }
    }
    headers.insert(
        HeaderName::from_static("access-control-allow-private-network"),
        HeaderValue::from_static("true"),
    );
    res
}

// ============================================================================
// Request / Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateLocalRoomRequest {
    pub party_name: String,
    pub dj_level: Option<i32>,
    pub primary_mood_id: Option<String>,
    pub secondary_mood_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateLocalRoomResponse {
    pub room: LocalPartyRoom,
    pub host_token: String,
    pub room_access_token: String,
    pub lan_reachable: bool,
    pub detected_lan_ip: Option<String>,
    pub guest_url: Option<String>,
    pub qr_svg: Option<String>,
    pub lan_notice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HostRoomDashboardResponse {
    pub room: LocalPartyRoom,
    pub dj_level_name: String,
    pub exposed_decks: usize,
    pub internal_deck_capacity: usize,
    pub auto_dj_enabled: bool,
    pub lan_reachable: bool,
    pub detected_lan_ip: Option<String>,
    pub submissions: Vec<LocalRoomSubmission>,
    pub cache_stats: LocalTempCacheStats,
    pub sync_overview: SyncStatusOverview,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLocalRoomConfigRequest {
    pub dj_level: Option<i32>,
    pub primary_mood_id: Option<String>,
    pub secondary_mood_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GuestJoinRoomRequest {
    pub nickname: String,
    pub room_access_token: Option<String>,
    pub room_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestJoinRoomResponse {
    pub guest_id: String,
    pub guest_token: String,
    pub nickname: String,
    pub room_id: String,
    pub party_name: String,
    pub mood_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UnifiedSongSubmissionRequest {
    pub url: String,
    pub authorized_stream: Option<AuthorizedStreamProof>,
}

#[derive(Debug, Serialize)]
pub struct UnifiedSongSubmissionResponse {
    pub id: String,
    pub submitted_by_type: String,
    pub title: String,
    pub artist: String,
    pub version_tag: String,
    pub provider: String,
    pub status: String, // 'received_by_dj' for guests; full detail for host
    pub metadata_source: String,
    pub acquisition_status: String,
    pub acquisition_note: String,
    pub dj_score: f64,
    pub interrupted_current_track: bool,
}

#[derive(Debug, Serialize)]
pub struct DjEvaluationResponse {
    pub evaluated_count: usize,
    pub prepared_next_submission_id: Option<String>,
    pub interrupted_current_track: bool,
    pub transition_style: String,
}

// ============================================================================
// Route Handlers
// ============================================================================

pub async fn local_health(
    State(state): State<Arc<LocalHelperState>>,
) -> Json<serde_json::Value> {
    let cache_stats = state
        .cache_manager
        .get_temp_cache_stats(DEFAULT_MAX_TEMP_TRACKS, &state.db)
        .unwrap_or(LocalTempCacheStats {
            track_count: 0,
            max_tracks: DEFAULT_MAX_TEMP_TRACKS,
            total_bytes: 0,
            locked_tracks: 0,
        });
    let sync_overview = state
        .sync_manager
        .get_sync_overview(&state.db)
        .unwrap_or(SyncStatusOverview {
            pending_count: 0,
            in_flight_count: 0,
            synced_count: 0,
            failed_count: 0,
        });

    Json(serde_json::json!({
        "status": "ok",
        "service": "dhun-local-helper",
        "version": "2.0.0",
        "port": state.port,
        "lan_reachable": state.detected_lan_ip.is_some(),
        "detected_lan_ip": state.detected_lan_ip,
        "cache_stats": cache_stats,
        "sync_overview": sync_overview,
        "timestamp": chrono::Utc::now().timestamp_millis()
    }))
}

pub async fn create_local_room(
    State(state): State<Arc<LocalHelperState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateLocalRoomRequest>,
) -> Result<Json<CreateLocalRoomResponse>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let party_name = req.party_name.trim();
    if party_name.is_empty() || party_name.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Party name must be between 1 and 100 characters.".to_string(),
        ));
    }

    let dj_level = req.dj_level.unwrap_or(2).clamp(1, 4);
    let primary_mood_id = req
        .primary_mood_id
        .unwrap_or_else(|| "house_party".to_string());
    if get_mood_preset(&primary_mood_id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported mood preset '{}'.", primary_mood_id),
        ));
    }

    let room_id = format!("rm_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let room_code = generate_room_code();
    let host_token = format!("dhun_host_{}", Uuid::new_v4().to_string().replace('-', ""));
    let room_access_token = format!("dhun_rm_{}", Uuid::new_v4().to_string().replace('-', ""));
    let now = chrono::Utc::now().timestamp_millis();

    let room = LocalPartyRoom {
        room_id: room_id.clone(),
        room_code,
        party_name: party_name.to_string(),
        status: "created".to_string(),
        dj_level,
        primary_mood_id,
        secondary_mood_id: req.secondary_mood_id,
        host_token_hash: hash_secret(&host_token),
        room_access_token_hash: hash_secret(&room_access_token),
        created_at: now,
        started_at: None,
        closed_at: None,
        updated_at: now,
    };

    state.db.create_local_party_room(&room).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create local party room: {}", e),
        )
    })?;

    let (lan_reachable, guest_url, qr_svg) =
        state.build_lan_guest_qr(&room_id, Some(&room_access_token));
    let lan_notice = if !lan_reachable {
        Some("No reachable private LAN Wi-Fi interface detected. Connect this laptop to a Wi-Fi/LAN network to generate a guest-scannable QR code.".to_string())
    } else {
        None
    };

    info!(
        "Created private local party room '{}' (room_id={}, code={}, lan_reachable={})",
        room.party_name, room.room_id, room.room_code, lan_reachable
    );

    Ok(Json(CreateLocalRoomResponse {
        room,
        host_token,
        room_access_token,
        lan_reachable,
        detected_lan_ip: state.detected_lan_ip.clone(),
        guest_url,
        qr_svg,
        lan_notice,
    }))
}

pub async fn get_host_room_dashboard(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<HostRoomDashboardResponse>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let lvl = DjLevel::from_i32(room.dj_level);
    let submissions = state
        .db
        .get_local_room_submissions(&room_id)
        .unwrap_or_default();
    let cache_stats = state
        .cache_manager
        .get_temp_cache_stats(DEFAULT_MAX_TEMP_TRACKS, &state.db)
        .unwrap_or(LocalTempCacheStats {
            track_count: 0,
            max_tracks: DEFAULT_MAX_TEMP_TRACKS,
            total_bytes: 0,
            locked_tracks: 0,
        });
    let sync_overview = state
        .sync_manager
        .get_sync_overview(&state.db)
        .unwrap_or(SyncStatusOverview {
            pending_count: 0,
            in_flight_count: 0,
            synced_count: 0,
            failed_count: 0,
        });

    Ok(Json(HostRoomDashboardResponse {
        room,
        dj_level_name: lvl.display_name().to_string(),
        exposed_decks: lvl.max_decks(),
        internal_deck_capacity: lvl.internal_deck_capacity(),
        auto_dj_enabled: lvl.supports_auto_dj(),
        lan_reachable: state.detected_lan_ip.is_some(),
        detected_lan_ip: state.detected_lan_ip.clone(),
        submissions,
        cache_stats,
        sync_overview,
    }))
}

pub async fn start_local_room(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<LocalPartyRoom>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    if room.status == "closed" {
        return Err((
            StatusCode::BAD_REQUEST,
            "This party room has already been closed. Create a new local party room.".to_string(),
        ));
    }

    let now = chrono::Utc::now().timestamp_millis();
    state
        .db
        .update_local_room_status(&room_id, "live", now)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated = state.db.get_local_party_room(&room_id).unwrap().unwrap();
    state.broadcast_event(
        &room_id,
        "room.status_changed",
        false,
        None,
        serde_json::json!({ "status": "live", "started_at": updated.started_at }),
    );
    info!("Local party room '{}' ({}) is now LIVE", updated.party_name, room_id);

    Ok(Json(updated))
}

/// Explicit host closure ONLY. Never auto-expired on inactivity.
/// Invalidates all guest tokens for this room and deletes temporary cached audio files.
pub async fn close_local_room(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let now = chrono::Utc::now().timestamp_millis();
    state
        .db
        .update_local_room_status(&room_id, "closed", now)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let invalidated_guests = state.db.delete_local_room_guests(&room_id).unwrap_or(0);
    let purged_cache_files = state
        .cache_manager
        .purge_room_cache(&room_id, &state.db)
        .unwrap_or(0);

    state.broadcast_event(
        &room_id,
        "room.closed",
        false,
        None,
        serde_json::json!({ "status": "closed", "closed_at": now }),
    );

    info!(
        "Explicitly closed local room {}: invalidated {} guest sessions, purged {} temporary audio cache files",
        room_id, invalidated_guests, purged_cache_files
    );

    Ok(Json(serde_json::json!({
        "room_id": room_id,
        "status": "closed",
        "closed_at": now,
        "invalidated_guests": invalidated_guests,
        "purged_cache_files": purged_cache_files
    })))
}

pub async fn update_local_room_config(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<UpdateLocalRoomConfigRequest>,
) -> Result<Json<LocalPartyRoom>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let new_level = req.dj_level.unwrap_or(room.dj_level).clamp(1, 4);
    let new_mood = req.primary_mood_id.unwrap_or(room.primary_mood_id);
    if get_mood_preset(&new_mood).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported mood preset '{}'.", new_mood),
        ));
    }
    let new_secondary = req.secondary_mood_id.or(room.secondary_mood_id);

    state
        .db
        .update_local_room_config(&room_id, new_level, &new_mood, new_secondary.as_deref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated = state.db.get_local_party_room(&room_id).unwrap().unwrap();
    state.broadcast_event(
        &room_id,
        "room.config_updated",
        true,
        None,
        serde_json::json!({
            "dj_level": updated.dj_level,
            "primary_mood_id": updated.primary_mood_id,
            "secondary_mood_id": updated.secondary_mood_id
        }),
    );

    Ok(Json(updated))
}

/// LAN Guest Authentication.
/// Strictly rejects a bare room_id without a valid room_access_token (QR) or room_code.
pub async fn guest_join_local_room(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<GuestJoinRoomRequest>,
) -> Result<Json<GuestJoinRoomResponse>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    if room.status == "closed" {
        return Err((
            StatusCode::GONE,
            "This party has ended and the room is closed.".to_string(),
        ));
    }

    // Room ID alone is NEVER trusted as authentication!
    let token_valid = req
        .room_access_token
        .as_deref()
        .map(|t| !t.trim().is_empty() && hash_secret(t.trim()) == room.room_access_token_hash)
        .unwrap_or(false);
    let code_valid = req
        .room_code
        .as_deref()
        .map(|c| !c.trim().is_empty() && c.trim().eq_ignore_ascii_case(&room.room_code))
        .unwrap_or(false);

    if !token_valid && !code_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Room access token (QR code) or valid room code is required to join this party.".to_string(),
        ));
    }

    let clean_nick = req.nickname.trim();
    if clean_nick.is_empty() || clean_nick.len() > 40 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Nickname must be between 1 and 40 characters.".to_string(),
        ));
    }

    let client_ip = addr.ip().to_string();
    let rate_key = format!("join:{}:{}", client_ip, room_id);
    if !state.rate_limiter.check_and_record(&rate_key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Too many join attempts. Please wait a moment.".to_string(),
        ));
    }

    let guest_id = format!("lgst_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let guest_token = format!("dhun_guest_{}", Uuid::new_v4().to_string().replace('-', ""));
    let now = chrono::Utc::now().timestamp_millis();

    let guest = LocalRoomGuest {
        id: guest_id.clone(),
        room_id: room_id.clone(),
        nickname: clean_nick.to_string(),
        token_hash: hash_secret(&guest_token),
        client_ip,
        submission_count: 0,
        created_at: now,
        last_seen_at: now,
    };

    state
        .db
        .register_local_room_guest(&guest)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mood_name = get_mood_preset(&room.primary_mood_id)
        .map(|m| m.display_name)
        .unwrap_or_else(|| room.primary_mood_id.clone());

    Ok(Json(GuestJoinRoomResponse {
        guest_id,
        guest_token,
        nickname: clean_nick.to_string(),
        room_id: room.room_id,
        party_name: room.party_name,
        mood_name,
    }))
}

/// Unified Song Submission endpoint for BOTH authenticated guests AND the host.
/// ZERO host approval/rejection queue! Enters directly into the private DJ candidate pool
/// and triggers non-interrupting DJ evaluation.
pub async fn submit_song_to_local_room(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<UnifiedSongSubmissionRequest>,
) -> Result<Json<UnifiedSongSubmissionResponse>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    if room.status == "closed" {
        return Err((
            StatusCode::GONE,
            "Party room is closed and no longer accepts song submissions.".to_string(),
        ));
    }

    // Determine caller identity: Host OR authenticated Guest for this specific room
    let (submitted_by_type, submitted_by_id, submitted_by_name) =
        if verify_host_token(&headers, &room).is_ok() {
            ("host".to_string(), "host".to_string(), "Host".to_string())
        } else {
            let guest = verify_guest_token(&headers, &room_id, &state.db)?;
            let rate_key = format!("sub:{}:{}", guest.id, room_id);
            if !state.rate_limiter.check_and_record(&rate_key) {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    "Please wait a moment before submitting another song.".to_string(),
                ));
            }
            ("guest".to_string(), guest.id, guest.nickname)
        };

    let submission_id = format!("sub_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);

    // Execute the 6-stage local processing pipeline
    let outcome = run_local_song_pipeline(
        &room_id,
        &submission_id,
        &req.url,
        req.authorized_stream.as_ref(),
        None,
        &state.db,
        &state.cache_manager,
        &state.downloader,
        &state.sync_manager,
        &state.http_client,
    )
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid song submission: {}", e)))?;

    let now = chrono::Utc::now().timestamp_millis();
    let submission = LocalRoomSubmission {
        id: submission_id.clone(),
        room_id: room_id.clone(),
        submitted_by_type: submitted_by_type.clone(),
        submitted_by_id: submitted_by_id.clone(),
        submitted_by_name: submitted_by_name.clone(),
        provider: outcome.parsed.provider.as_str().to_string(),
        source_url: outcome.parsed.canonical_url.clone(),
        provider_video_id: outcome.parsed.video_id.clone(),
        song_id: Some(outcome.metadata.song_id.clone()),
        title: outcome.metadata.title.clone(),
        artist: outcome.metadata.artist.clone(),
        version_tag: outcome.metadata.version_tag.clone(),
        thumbnail_url: Some(format!(
            "https://i.ytimg.com/vi/{}/hqdefault.jpg",
            outcome.parsed.video_id
        )),
        duration_ms: if outcome.metadata.duration_ms > 0 {
            Some(outcome.metadata.duration_ms)
        } else {
            None
        },
        bpm: outcome.metadata.original_bpm,
        half_time_bpm: outcome.metadata.half_time_bpm,
        double_time_bpm: outcome.metadata.double_time_bpm,
        musical_key: outcome.metadata.musical_key.clone(),
        camelot_key: outcome.metadata.camelot_key.clone(),
        overall_energy: outcome.metadata.overall_energy,
        danceability: outcome.metadata.danceability,
        genre_tags_json: serde_json::to_string(&outcome.metadata.genre_tags).unwrap_or_else(|_| "[]".to_string()),
        mood_tags_json: serde_json::to_string(&outcome.metadata.mood_tags).unwrap_or_else(|_| "[]".to_string()),
        metadata_source: outcome.metadata_source.clone(),
        acquisition_status: outcome.acquisition_status.as_str().to_string(),
        acquisition_note: Some(outcome.acquisition_note.clone()),
        dj_status: "queued_for_dj".to_string(),
        dj_score: 0.0,
        local_cache_path: outcome.cached_entry.as_ref().map(|c| c.file_path.clone()),
        created_at: now,
        updated_at: now,
    };

    state
        .db
        .insert_local_room_submission(&submission)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if submitted_by_type == "guest" {
        let _ = state.db.increment_local_guest_submission_count(&submitted_by_id);
    }

    // Immediately run non-interrupting automated DJ evaluation across the private candidate pool
    let primary_mood = get_mood_preset(&room.primary_mood_id)
        .unwrap_or_else(|| get_all_mood_presets().into_iter().next().unwrap());
    let secondary_mood = room
        .secondary_mood_id
        .as_deref()
        .and_then(get_mood_preset);
    let all_subs = state.db.get_local_room_submissions(&room_id).unwrap_or_default();
    let (scored, _) = select_next_submission_without_interrupting(
        &all_subs,
        &primary_mood,
        secondary_mood.as_ref(),
        primary_mood.target_energy,
    );

    let mut this_score = 0.0;
    for (sid, sc) in &scored {
        if sid == &submission_id {
            this_score = *sc;
        }
        let existing_status = all_subs
            .iter()
            .find(|s| &s.id == sid)
            .map(|s| s.dj_status.as_str())
            .unwrap_or("queued_for_dj");
        // Strictly preserve any already-playing track (never interrupt!)
        if existing_status != "playing" && existing_status != "played" {
            let _ = state.db.update_submission_dj_state(sid, existing_status, *sc);
        }
    }

    // Host-only WebSocket broadcast (other guests NEVER see this submission!)
    state.broadcast_event(
        &room_id,
        "dj.submission_received",
        true,
        None,
        serde_json::json!({
            "submission_id": submission_id,
            "submitted_by_type": submitted_by_type,
            "title": outcome.metadata.title,
            "dj_score": this_score
        }),
    );

    Ok(Json(UnifiedSongSubmissionResponse {
        id: submission_id,
        submitted_by_type,
        title: outcome.metadata.title,
        artist: outcome.metadata.artist,
        version_tag: outcome.metadata.version_tag,
        provider: outcome.parsed.provider.as_str().to_string(),
        status: "received_by_dj".to_string(),
        metadata_source: outcome.metadata_source,
        acquisition_status: outcome.acquisition_status.as_str().to_string(),
        acquisition_note: outcome.acquisition_note,
        dj_score: this_score,
        interrupted_current_track: false,
    }))
}

/// Privacy-Isolated Guest Endpoint: returns ONLY this guest's own submissions.
/// Guests NEVER see the party playlist, other guests' requests, or the global catalogue.
pub async fn get_guest_own_submissions(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<Vec<GuestOwnSubmissionView>>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let guest = verify_guest_token(&headers, &room_id, &state.db)?;
    let mine = state
        .db
        .get_guest_own_submissions(&room_id, &guest.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(mine))
}

/// Host-only Automated DJ evaluation trigger: scores all unplayed submissions against current mood,
/// energy curve, BPM, and Camelot key, and stages the best candidate for the next transition
/// WITHOUT interrupting the currently playing deck.
pub async fn evaluate_dj_candidate_pool(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<DjEvaluationResponse>, (StatusCode, String)> {
    verify_lan_and_host_headers(addr.ip(), &headers, state.detected_lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let primary_mood = get_mood_preset(&room.primary_mood_id)
        .unwrap_or_else(|| get_all_mood_presets().into_iter().next().unwrap());
    let secondary_mood = room
        .secondary_mood_id
        .as_deref()
        .and_then(get_mood_preset);
    let submissions = state
        .db
        .get_local_room_submissions(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (scored, best_next) = select_next_submission_without_interrupting(
        &submissions,
        &primary_mood,
        secondary_mood.as_ref(),
        primary_mood.target_energy,
    );

    for (sid, sc) in &scored {
        let next_status = if best_next.as_deref() == Some(sid.as_str()) {
            "prepared"
        } else {
            "queued_for_dj"
        };
        let _ = state.db.update_submission_dj_state(sid, next_status, *sc);
    }

    state.broadcast_event(
        &room_id,
        "dj.evaluated",
        true,
        None,
        serde_json::json!({
            "evaluated_count": scored.len(),
            "prepared_next_submission_id": best_next,
            "transition_style": primary_mood.transition_style
        }),
    );

    Ok(Json(DjEvaluationResponse {
        evaluated_count: scored.len(),
        prepared_next_submission_id: best_next,
        interrupted_current_track: false,
        transition_style: primary_mood.transition_style,
    }))
}

// ============================================================================
// Local WebSocket (Host vs Privacy-Isolated Guest Filtering)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LocalWsQuery {
    pub role: Option<String>, // 'host' | 'guest'
    pub token: Option<String>,
}

pub async fn ws_local_room_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    Query(q): Query<LocalWsQuery>,
    State(state): State<Arc<LocalHelperState>>,
) -> impl IntoResponse {
    let room = match state.db.get_local_party_room(&room_id) {
        Ok(Some(r)) => r,
        _ => return (StatusCode::NOT_FOUND, "Local party room not found").into_response(),
    };

    let raw_token = match q.token.as_deref() {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => return (StatusCode::UNAUTHORIZED, "WebSocket token required").into_response(),
    };

    let hash = hash_secret(&raw_token);
    let is_host = hash == room.host_token_hash;
    let guest_id = if !is_host {
        match state.db.lookup_local_room_guest(&room_id, &hash) {
            Ok(Some(g)) => Some(g.id),
            _ => return (StatusCode::UNAUTHORIZED, "Invalid room WebSocket token").into_response(),
        }
    } else {
        None
    };

    ws.on_upgrade(move |socket| handle_local_room_socket(socket, room_id, is_host, guest_id, state))
}

async fn handle_local_room_socket(
    socket: WebSocket,
    room_id: String,
    is_host: bool,
    guest_id: Option<String>,
    state: Arc<LocalHelperState>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(evt) = rx.recv().await {
            if evt.room_id != room_id {
                continue;
            }
            // Enforce strict privacy: guests NEVER receive host_only events or other guests' targeted events
            if !is_host {
                if evt.host_only {
                    continue;
                }
                if let Some(ref target) = evt.target_guest_id {
                    if guest_id.as_deref() != Some(target.as_str()) {
                        continue;
                    }
                }
            }
            if let Ok(text) = serde_json::to_string(&evt) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
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

// ============================================================================
// Self-Contained LAN Guest Portal HTML (Served Directly Over Host Wi-Fi/LAN)
// ============================================================================

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn serve_lan_guest_portal(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
) -> Html<String> {
    let party_name = match state.db.get_local_party_room(&room_id) {
        Ok(Some(r)) => r.party_name,
        _ => "DHUN Local Party".to_string(),
    };
    let clean_party = escape_html(&party_name);
    let clean_room_id = escape_html(&room_id);

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>DHUN धुन — {clean_party}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: #090a0f;
      color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      padding: 18px;
      max-width: 480px;
      margin: 0 auto;
    }}
    header {{ text-align: center; padding: 14px 0 18px; border-bottom: 1px solid #1e293b; }}
    .logo {{ font-size: 28px; font-weight: 900; color: #f59e0b; letter-spacing: 2px; }}
    .subtitle {{ font-size: 13px; color: #94a3b8; margin-top: 4px; }}
    .badge {{
      display: inline-block; margin-top: 8px; font-size: 11px; font-weight: 700;
      color: #fde68a; background: rgba(245, 158, 11, 0.14); border: 1px solid rgba(245, 158, 11, 0.35);
      border-radius: 9999px; padding: 4px 12px;
    }}
    .card {{ background: #111827; border: 1px solid #1f2937; border-radius: 14px; padding: 14px; margin-top: 16px; }}
    label {{ display: block; font-size: 11px; font-weight: 700; text-transform: uppercase; color: #94a3b8; margin-bottom: 6px; }}
    input {{
      width: 100%; background: #030712; border: 1px solid #374151; border-radius: 10px;
      padding: 11px 14px; color: #fff; font-size: 14px; outline: none; margin-bottom: 10px;
    }}
    input:focus {{ border-color: #f59e0b; }}
    button {{
      width: 100%; background: linear-gradient(90deg, #f59e0b, #e11d48); color: #fff;
      font-size: 13px; font-weight: 800; border: none; border-radius: 10px; padding: 12px; cursor: pointer;
    }}
    button:disabled {{ opacity: 0.45; cursor: not-allowed; }}
    .msg {{ font-size: 12px; margin-top: 10px; padding: 8px 10px; border-radius: 8px; display: none; }}
    .msg.ok {{ display: block; background: rgba(16, 185, 129, 0.15); color: #6ee7b7; border: 1px solid rgba(16, 185, 129, 0.35); }}
    .msg.err {{ display: block; background: rgba(244, 63, 94, 0.15); color: #fda4af; border: 1px solid rgba(244, 63, 94, 0.35); }}
    .section-title {{ font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; color: #94a3b8; margin: 18px 0 8px; }}
    .sub-item {{
      background: #0f172a; border: 1px solid #1e293b; border-radius: 10px; padding: 10px 12px;
      display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; gap: 10px;
    }}
    .sub-title {{ font-size: 13px; font-weight: 700; color: #fff; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    .sub-artist {{ font-size: 11px; color: #94a3b8; }}
    .pill {{ font-size: 10px; font-weight: 700; padding: 3px 8px; border-radius: 999px; background: rgba(45, 212, 191, 0.15); color: #5eead4; text-transform: uppercase; flex-shrink: 0; }}
    footer {{ text-align: center; font-size: 11px; color: #64748b; margin-top: auto; padding-top: 20px; }}
  </style>
</head>
<body>
  <header>
    <div class="logo">धुन DHUN</div>
    <div class="subtitle">Local Party Guest Request</div>
    <div class="badge">{clean_party}</div>
  </header>

  <div class="card" id="joinCard">
    <label>Your Name</label>
    <input id="nickInput" type="text" placeholder="Enter your name (e.g. Riya)" maxlength="40" />
    <label id="codeLabel" style="display:none;">Room Code</label>
    <input id="codeInput" type="text" placeholder="e.g. DHUN-4K9P" style="display:none;" />
    <button id="joinBtn">Join Local Party Room</button>
    <div id="joinMsg" class="msg"></div>
  </div>

  <div class="card" id="submitCard" style="display:none;">
    <label>Paste YouTube or YouTube Music Song Link</label>
    <input id="urlInput" type="url" placeholder="https://www.youtube.com/watch?v=..." />
    <button id="submitBtn">Send Song to Automated DJ</button>
    <div id="submitMsg" class="msg"></div>
    <p style="font-size:11px;color:#94a3b8;margin-top:8px;">
      DHUN&rsquo;s automated DJ evaluates mood, BPM, and harmony to mix your request smoothly without interrupting the current song.
    </p>
  </div>

  <div id="mySection" style="display:none;">
    <div class="section-title">My Submitted Songs</div>
    <div id="myList"></div>
  </div>

  <footer>Private Local Wi-Fi Room &bull; Host Audio Controlled on Laptop</footer>

  <script>
    const roomId = '{clean_room_id}';
    const params = new URLSearchParams(window.location.search);
    const qrAccessToken = params.get('access') || '';
    let guestToken = localStorage.getItem('dhun_local_guest_token_' + roomId) || '';

    const joinCard = document.getElementById('joinCard');
    const submitCard = document.getElementById('submitCard');
    const mySection = document.getElementById('mySection');
    const codeLabel = document.getElementById('codeLabel');
    const codeInput = document.getElementById('codeInput');

    if (!qrAccessToken) {{
      codeLabel.style.display = 'block';
      codeInput.style.display = 'block';
    }}

    function showMsg(elId, ok, text) {{
      const el = document.getElementById(elId);
      el.className = 'msg ' + (ok ? 'ok' : 'err');
      el.textContent = text;
    }}

    function enterPartyUi() {{
      joinCard.style.display = 'none';
      submitCard.style.display = 'block';
      mySection.style.display = 'block';
      loadMySubmissions();
    }}

    if (guestToken) {{
      enterPartyUi();
    }}

    document.getElementById('joinBtn').addEventListener('click', async () => {{
      const nick = document.getElementById('nickInput').value.trim();
      if (!nick) return showMsg('joinMsg', false, 'Please enter your name.');
      try {{
        const res = await fetch('/api/local/rooms/' + roomId + '/guests/join', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{
            nickname: nick,
            room_access_token: qrAccessToken || undefined,
            room_code: codeInput.value.trim() || undefined
          }})
        }});
        if (!res.ok) {{
          return showMsg('joinMsg', false, await res.text());
        }}
        const data = await res.json();
        guestToken = data.guest_token;
        localStorage.setItem('dhun_local_guest_token_' + roomId, guestToken);
        enterPartyUi();
      }} catch (e) {{
        showMsg('joinMsg', false, 'Could not connect to host Wi-Fi room.');
      }}
    }});

    document.getElementById('submitBtn').addEventListener('click', async () => {{
      const url = document.getElementById('urlInput').value.trim();
      if (!url) return showMsg('submitMsg', false, 'Paste a YouTube or YouTube Music URL.');
      const btn = document.getElementById('submitBtn');
      btn.disabled = true;
      try {{
        const res = await fetch('/api/local/rooms/' + roomId + '/submissions', {{
          method: 'POST',
          headers: {{
            'Content-Type': 'application/json',
            'Authorization': 'Bearer ' + guestToken
          }},
          body: JSON.stringify({{ url }})
        }});
        if (!res.ok) {{
          showMsg('submitMsg', false, await res.text());
        }} else {{
          const data = await res.json();
          document.getElementById('urlInput').value = '';
          showMsg('submitMsg', true, 'Sent "' + data.title + '" to the automated DJ!');
          loadMySubmissions();
        }}
      }} catch (e) {{
        showMsg('submitMsg', false, 'Connection error with host laptop.');
      }} finally {{
        btn.disabled = false;
      }}
    }});

    async function loadMySubmissions() {{
      if (!guestToken) return;
      try {{
        const res = await fetch('/api/local/rooms/' + roomId + '/my-submissions', {{
          headers: {{ 'Authorization': 'Bearer ' + guestToken }}
        }});
        if (!res.ok) return;
        const list = await res.json();
        const container = document.getElementById('myList');
        container.innerHTML = '';
        if (!list.length) {{
          const empty = document.createElement('div');
          empty.style.cssText = 'color:#64748b;font-size:12px;';
          empty.textContent = 'No songs submitted yet.';
          container.appendChild(empty);
          return;
        }}
        list.forEach(item => {{
          const row = document.createElement('div');
          row.className = 'sub-item';
          const left = document.createElement('div');
          left.style.minWidth = '0';
          const t = document.createElement('div');
          t.className = 'sub-title';
          t.textContent = item.title;
          const a = document.createElement('div');
          a.className = 'sub-artist';
          a.textContent = item.artist;
          left.appendChild(t);
          left.appendChild(a);
          const pill = document.createElement('span');
          pill.className = 'pill';
          pill.textContent = item.status.replace(/_/g, ' ');
          row.appendChild(left);
          row.appendChild(pill);
          container.appendChild(row);
        }});
      }} catch (e) {{}}
    }}

    setInterval(loadMySubmissions, 6000);
  </script>
</body>
</html>"#,
        clean_party = clean_party,
        clean_room_id = clean_room_id
    );

    Html(html)
}

// ============================================================================
// Router Builder (Notice: ZERO Approval/Rejection Routes & ZERO Global Catalogue Browse)
// ============================================================================

pub fn create_local_helper_router(state: Arc<LocalHelperState>) -> Router {
    Router::new()
        .route("/api/local/health", get(local_health))
        .route("/api/local/rooms", post(create_local_room))
        .route("/api/local/rooms/:room_id", get(get_host_room_dashboard))
        .route("/api/local/rooms/:room_id/start", post(start_local_room))
        .route("/api/local/rooms/:room_id/close", post(close_local_room))
        .route("/api/local/rooms/:room_id/config", post(update_local_room_config))
        .route("/api/local/rooms/:room_id/dj/evaluate", post(evaluate_dj_candidate_pool))
        .route("/api/local/rooms/:room_id/guests/join", post(guest_join_local_room))
        .route("/api/local/rooms/:room_id/submissions", post(submit_song_to_local_room))
        .route("/api/local/rooms/:room_id/my-submissions", get(get_guest_own_submissions))
        .route("/ws/local/rooms/:room_id", get(ws_local_room_handler))
        .route("/join/:room_id", get(serve_lan_guest_portal))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            local_security_and_cors_middleware,
        ))
        .with_state(state)
}
