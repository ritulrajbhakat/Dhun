use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, RwLock};
use axum::{
    body::Body,
    extract::{
        ConnectInfo, DefaultBodyLimit, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        header::{
            ACCEPT_RANGES, AUTHORIZATION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, HOST,
            RANGE, HeaderName, HeaderValue,
        },
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
    LocalTempCacheStats, PhraseMarker, SubmissionIntakeJob, SyncStatusOverview, TrackTimingProfile,
};
use crate::db::Database;
use crate::dj::decision_engine::{
    plan_dj_transition, PartyDecisionContext, TempoQualityPolicy, TrackTransitionInput,
    TransitionDecisionPlan,
};
use crate::dj::levels::{DjLevel, DjSkillProfile};
use crate::dj::moods::{get_mood_by_id as get_mood_preset, get_preset_moods as get_all_mood_presets};
use crate::dj::selector::{
    build_dj_recommendations, choose_technique_for_pair, compute_mood_energy_target,
    select_next_submission_without_interrupting, DjRecommendation,
};
use crate::library::cache::{
    RollingCacheManager, CACHE_PRIORITY_EVICTABLE, CACHE_PRIORITY_NEXT_DECK,
    CACHE_PRIORITY_PLAYING, DEFAULT_MAX_TEMP_TRACKS,
};
use crate::library::downloader::AudioDownloader;
use crate::library::pipeline::{
    stage_1_validate_and_identify, stage_2_lookup_existing_metadata,
    stage_4_cache_temporary_audio, stage_5_local_audio_analysis, stage_6_sync_metadata_only,
    AcquisitionStatus, AuthorizedStreamProof,
};
use crate::library::resolver::{
    acquire_via_authorized_resolver, download_authorized_proof, AuthorizedResolverConfig,
};
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
    pub event_type: String,
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
    detected_lan_ip: Arc<RwLock<Option<String>>>,
    lan_ip_pinned: bool,
    pub cache_manager: RollingCacheManager,
    pub downloader: AudioDownloader,
    pub sync_manager: MetadataSyncManager,
    pub http_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
    pub ws_tx: broadcast::Sender<LocalRoomEvent>,
    pub allowed_frontend_origins: Vec<String>,
    pub audio_resolver: Option<AuthorizedResolverConfig>,
    pub intake_notify: Arc<tokio::sync::Notify>,
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
        let (initial_lan_ip, lan_ip_pinned) = match detected_lan_ip_override {
            Some(explicit) => (explicit, true),
            None => (detect_reachable_lan_ip(), false),
        };

        Self {
            db,
            port,
            detected_lan_ip: Arc::new(RwLock::new(initial_lan_ip)),
            lan_ip_pinned,
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
            audio_resolver: None,
            intake_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn with_audio_resolver(mut self, resolver: Option<AuthorizedResolverConfig>) -> Self {
        self.audio_resolver = resolver;
        self
    }

    /// Reads the current reachable RFC1918 LAN IP (re-detecting automatically when not pinned in tests
    /// so host Wi-Fi/LAN IP changes are picked up gracefully without restarting the helper).
    pub fn current_lan_ip(&self) -> Option<String> {
        if !self.lan_ip_pinned {
            let fresh = detect_reachable_lan_ip();
            if let Ok(mut guard) = self.detected_lan_ip.write() {
                *guard = fresh.clone();
            }
            return fresh;
        }
        self.detected_lan_ip.read().ok().and_then(|g| g.clone())
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
        let Some(lan_ip) = self.current_lan_ip() else {
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
    let lan_ip_now = state.current_lan_ip();
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        if let Err((code, msg)) = verify_lan_and_host_headers(
            addr.ip(),
            req.headers(),
            lan_ip_now.as_deref(),
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
        if let Some(ref lan) = lan_ip_now {
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
            HeaderValue::from_static("content-type, authorization, accept, range, x-host-token, x-guest-token"),
        );
        headers.insert(
            "access-control-expose-headers",
            HeaderValue::from_static("content-length, content-range, accept-ranges, content-type"),
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
        "access-control-expose-headers",
        HeaderValue::from_static("content-length, content-range, accept-ranges, content-type"),
    );
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
pub struct PlayableCandidatePointer {
    pub submission_id: String,
    pub cache_id: String,
    pub stream_url: String,
    pub title: String,
    pub artist: String,
    pub bpm: Option<f64>,
    pub camelot_key: Option<String>,
    pub duration_ms: Option<i64>,
    pub overall_energy: Option<f64>,
    pub genre_tags: Vec<String>,
    pub sections: Vec<PhraseMarker>,
    pub timing_profile: Option<TrackTimingProfile>,
}

#[derive(Debug, Serialize)]
pub struct HostRoomDashboardResponse {
    pub room: LocalPartyRoom,
    pub dj_level_name: String,
    pub dj_skill_profile: DjSkillProfile,
    pub exposed_decks: usize,
    pub internal_deck_capacity: usize,
    pub auto_dj_enabled: bool,
    pub lan_reachable: bool,
    pub detected_lan_ip: Option<String>,
    pub submissions: Vec<LocalRoomSubmission>,
    pub recommendations: Vec<DjRecommendation>,
    pub pending_intake_jobs: usize,
    pub next_playable_candidate: Option<PlayableCandidatePointer>,
    pub transition_plan: Option<TransitionDecisionPlan>,
    pub transition_style: String,
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
pub struct CloseLocalRoomRequest {
    /// Set to true once the host audio engine has stopped playback and released all decks.
    pub force_release_audio: Option<bool>,
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
    pub status: String,
    pub metadata_source: String,
    pub acquisition_status: String,
    pub acquisition_note: String,
    pub cache_id: Option<String>,
    pub dj_score: f64,
    pub interrupted_current_track: bool,
}

#[derive(Debug, Serialize)]
pub struct DjEvaluationResponse {
    pub evaluated_count: usize,
    pub prepared_next_submission_id: Option<String>,
    pub next_playable_candidate: Option<PlayableCandidatePointer>,
    pub recommendations: Vec<DjRecommendation>,
    pub interrupted_current_track: bool,
    pub transition_style: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackEventRequest {
    pub submission_id: String,
    pub event: String, // 'prepared' | 'playing' | 'played' | 'decode_failed'
    pub deck_id: Option<usize>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaybackEventResponse {
    pub submission_id: String,
    pub dj_status: String,
    pub cache_locked: bool,
    pub next_playable_candidate: Option<PlayableCandidatePointer>,
    pub recommendations: Vec<DjRecommendation>,
    pub transition_style: String,
}

// ============================================================================
// Helper: Build Playable Pointer & Score Room
// ============================================================================

fn build_playable_pointer(
    room_id: &str,
    sub: &LocalRoomSubmission,
    db: &Database,
) -> Option<PlayableCandidatePointer> {
    if sub.acquisition_status != "cached_authorized" {
        return None;
    }
    let cache_id = sub.cache_id.as_ref()?;
    let sections = sub
        .song_id
        .as_deref()
        .and_then(|sid| db.get_phrase_markers_for_song(sid).ok())
        .unwrap_or_default();
    let timing_profile = sub
        .song_id
        .as_deref()
        .and_then(|sid| db.get_timing_profile_for_song(sid).ok())
        .flatten();
    let genre_tags: Vec<String> = serde_json::from_str(&sub.genre_tags_json).unwrap_or_default();
    Some(PlayableCandidatePointer {
        submission_id: sub.id.clone(),
        cache_id: cache_id.clone(),
        stream_url: format!("/api/local/rooms/{}/cache/{}/stream", room_id, cache_id),
        title: sub.title.clone(),
        artist: sub.artist.clone(),
        bpm: sub.bpm,
        camelot_key: sub.camelot_key.clone(),
        duration_ms: sub.duration_ms,
        overall_energy: sub.overall_energy,
        genre_tags,
        sections,
        timing_profile,
    })
}

fn build_track_transition_input(
    sub: &LocalRoomSubmission,
    deck_id: usize,
    db: &Database,
) -> TrackTransitionInput {
    let sections = sub
        .song_id
        .as_deref()
        .and_then(|sid| db.get_phrase_markers_for_song(sid).ok())
        .unwrap_or_default();
    let timing_profile = sub
        .song_id
        .as_deref()
        .and_then(|sid| db.get_timing_profile_for_song(sid).ok())
        .flatten();
    let genre_tags: Vec<String> = serde_json::from_str(&sub.genre_tags_json).unwrap_or_default();
    let profile = crate::dj::selector::extract_musical_profile(
        sub,
        sub.bpm.unwrap_or(124.0),
        sub.overall_energy.unwrap_or(0.70),
    );
    TrackTransitionInput {
        track_id: sub.id.clone(),
        deck_id,
        title: sub.title.clone(),
        artist: sub.artist.clone(),
        duration_sec: (sub.duration_ms.unwrap_or(180_000) as f64 / 1000.0).max(10.0),
        current_pos_sec: 0.0,
        bpm: sub.bpm,
        camelot_key: sub.camelot_key.clone(),
        overall_energy: sub.overall_energy,
        genre_tags,
        sections,
        timing_profile,
        vocal_density: Some(profile.vocal_density),
        is_electronic_dance: profile.is_electronic_dance,
        is_acoustic_ballad: profile.is_acoustic,
        is_percussion_heavy: profile.is_percussion_heavy,
        predicted_energy_direction: None,
        vocal_style: if profile.vocal_density >= 0.70 {
            Some("dense_vocal".to_string())
        } else if profile.vocal_density <= 0.25 {
            Some("instrumental".to_string())
        } else {
            None
        },
    }
}

fn evaluate_and_find_next_playable(
    room: &LocalPartyRoom,
    db: &Database,
) -> (
    Vec<(String, f64)>,
    Option<PlayableCandidatePointer>,
    String,
    Vec<DjRecommendation>,
) {
    let primary_mood = get_mood_preset(&room.primary_mood_id)
        .unwrap_or_else(|| get_all_mood_presets().into_iter().next().unwrap());
    let secondary_mood = room
        .secondary_mood_id
        .as_deref()
        .and_then(get_mood_preset);
    let dj_level = DjLevel::from_i32(room.dj_level);

    let elapsed_mins = room
        .started_at
        .map(|s| ((chrono::Utc::now().timestamp_millis() - s).max(0) as f64) / 60_000.0)
        .unwrap_or(0.0);
    let target_energy = compute_mood_energy_target(&primary_mood, elapsed_mins);

    let submissions = db.get_local_room_submissions(&room.room_id).unwrap_or_default();
    let (scored, best_playable_id) = select_next_submission_without_interrupting(
        &submissions,
        &primary_mood,
        secondary_mood.as_ref(),
        target_energy,
    );

    for (sid, sc) in &scored {
        let cur_status = submissions
            .iter()
            .find(|s| &s.id == sid)
            .map(|s| s.dj_status.as_str())
            .unwrap_or("queued_for_dj");
        // Strictly preserve any playing / played / skipped_unplayable tracks
        if cur_status != "playing" && cur_status != "played" && cur_status != "skipped_unplayable" {
            let next_st = if best_playable_id.as_deref() == Some(sid.as_str()) {
                "prepared"
            } else {
                "queued_for_dj"
            };
            let _ = db.update_submission_dj_state(sid, next_st, *sc);
        }
    }

    let recommendations = build_dj_recommendations(
        &submissions,
        &primary_mood,
        secondary_mood.as_ref(),
        target_energy,
        dj_level,
    );

    let currently_playing = submissions.iter().find(|s| s.dj_status == "playing");
    let next_sub = best_playable_id
        .as_deref()
        .and_then(|id| submissions.iter().find(|s| s.id == id));

    let transition_style = if let Some(cand) = next_sub {
        let conf = recommendations
            .iter()
            .find(|r| r.submission_id == cand.id)
            .map(|r| r.confidence)
            .unwrap_or(0.5);
        choose_technique_for_pair(dj_level, currently_playing, cand, &primary_mood, conf).0
    } else if let Some(top_rec) = recommendations.first() {
        top_rec.suggested_technique.clone()
    } else {
        primary_mood.transition_style.clone()
    };

    let next_ptr = next_sub.and_then(|s| build_playable_pointer(&room.room_id, s, db));

    (scored, next_ptr, transition_style, recommendations)
}

/// Processes a single persistent intake job through truthful stages:
/// `metadata_processing` -> `awaiting_authorized_audio` -> `analysing` -> `cached_authorized` / `restricted_by_provider` / `failed`.
pub async fn process_single_intake_job(state: &LocalHelperState, job: &SubmissionIntakeJob) -> bool {
    let parsed = match stage_1_validate_and_identify(&job.raw_url) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Invalid URL: {}", e);
            let _ = state.db.update_submission_processing_stage(&job.submission_id, "failed", Some(&msg));
            let _ = state.db.update_submission_intake_job_status(&job.id, "failed", Some(&msg));
            return false;
        }
    };

    // Stage 2: Metadata lookup (local cache -> global catalogue -> official oEmbed)
    let _ = state.db.update_submission_processing_stage(
        &job.submission_id,
        "metadata_processing",
        Some("Resolving song title, artist, and existing catalogue metadata..."),
    );
    state.broadcast_event(
        &job.room_id,
        "dj.submission_updated",
        true,
        None,
        serde_json::json!({
            "submission_id": job.submission_id,
            "stage": "metadata_processing"
        }),
    );

    let (mut metadata, mut metadata_source) = stage_2_lookup_existing_metadata(
        &parsed,
        &state.db,
        &state.sync_manager,
        &state.http_client,
    )
    .await;

    // Immediately persist resolved title/artist so the UI shows real track identity before audio finishes
    let mut current_sub = state
        .db
        .get_local_room_submissions(&job.room_id)
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.id == job.submission_id)
        .unwrap_or_else(|| LocalRoomSubmission {
            id: job.submission_id.clone(),
            room_id: job.room_id.clone(),
            submitted_by_type: "host".to_string(),
            submitted_by_id: "host".to_string(),
            submitted_by_name: "Host".to_string(),
            provider: parsed.provider.as_str().to_string(),
            source_url: parsed.canonical_url.clone(),
            provider_video_id: parsed.video_id.clone(),
            song_id: Some(metadata.song_id.clone()),
            title: metadata.title.clone(),
            artist: metadata.artist.clone(),
            version_tag: metadata.version_tag.clone(),
            thumbnail_url: Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", parsed.video_id)),
            duration_ms: None,
            bpm: metadata.original_bpm,
            half_time_bpm: metadata.half_time_bpm,
            double_time_bpm: metadata.double_time_bpm,
            musical_key: metadata.musical_key.clone(),
            camelot_key: metadata.camelot_key.clone(),
            overall_energy: metadata.overall_energy,
            danceability: metadata.danceability,
            genre_tags_json: "[]".to_string(),
            mood_tags_json: "[]".to_string(),
            metadata_source: metadata_source.clone(),
            acquisition_status: "awaiting_authorized_audio".to_string(),
            acquisition_note: None,
            dj_status: "queued_for_dj".to_string(),
            dj_score: 0.0,
            local_cache_path: None,
            cache_id: None,
            created_at: job.created_at,
            updated_at: chrono::Utc::now().timestamp_millis(),
        });

    current_sub.song_id = Some(metadata.song_id.clone());
    current_sub.title = metadata.title.clone();
    current_sub.artist = metadata.artist.clone();
    current_sub.version_tag = metadata.version_tag.clone();
    current_sub.bpm = metadata.original_bpm;
    current_sub.half_time_bpm = metadata.half_time_bpm;
    current_sub.double_time_bpm = metadata.double_time_bpm;
    current_sub.musical_key = metadata.musical_key.clone();
    current_sub.camelot_key = metadata.camelot_key.clone();
    current_sub.overall_energy = metadata.overall_energy;
    current_sub.danceability = metadata.danceability;
    current_sub.genre_tags_json =
        serde_json::to_string(&metadata.genre_tags).unwrap_or_else(|_| "[]".to_string());
    current_sub.mood_tags_json =
        serde_json::to_string(&metadata.mood_tags).unwrap_or_else(|_| "[]".to_string());
    current_sub.metadata_source = metadata_source.clone();
    current_sub.acquisition_status = "awaiting_authorized_audio".to_string();
    current_sub.acquisition_note = Some("Metadata resolved; checking authorized audio source...".to_string());
    if let Err(e) = state.db.update_submission_pipeline_result(&current_sub) {
        warn!("Failed to persist async pipeline result for submission {}: {}", current_sub.id, e);
    }
    let _ = state
        .db
        .update_submission_intake_job_status(&job.id, "awaiting_authorized_audio", None);

    // Stage 3: Authorised audio acquisition (explicit proof or host-configured resolver)
    let auth_proof: Option<AuthorizedStreamProof> = job
        .authorized_stream_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok());

    let (acq_status, acq_note, acquired_path) = if let Some(ref proof) = auth_proof {
        download_authorized_proof(&parsed, proof, &state.downloader, &state.db).await
    } else if let Some(ref resolver) = state.audio_resolver {
        acquire_via_authorized_resolver(&parsed, &metadata, resolver, &state.downloader, &state.db)
            .await
    } else {
        (
            AcquisitionStatus::RestrictedByProvider,
            "YouTube/YouTube Music URLs provide legal oEmbed metadata only; no authorized audio source is configured. Track remains in recommendations as metadata-only.".to_string(),
            None,
        )
    };

    // Stage 4 & 5: Temporary caching + Local Symphonia DSP analysis (only when audio is cached)
    let mut cached_entry = None;
    if acq_status == AcquisitionStatus::CachedAuthorized {
        if let Some(ref audio_path) = acquired_path {
            let _ = state.db.update_submission_processing_stage(
                &job.submission_id,
                "analysing",
                Some("Audio cached; running local Symphonia BPM, key, beat-grid & energy analysis..."),
            );
            let _ = state
                .db
                .update_submission_intake_job_status(&job.id, "analysing", None);
            state.broadcast_event(
                &job.room_id,
                "dj.submission_updated",
                true,
                None,
                serde_json::json!({
                    "submission_id": job.submission_id,
                    "title": metadata.title,
                    "stage": "analysing"
                }),
            );

            if let Ok(entry) = stage_4_cache_temporary_audio(
                &state.cache_manager,
                &job.room_id,
                Some(&job.submission_id),
                &metadata.song_id,
                audio_path,
                &state.db,
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

                let _ = state.db.upsert_global_song_metadata(
                    &metadata,
                    Some((
                        &parsed.canonical_url,
                        parsed.provider.as_str(),
                        &parsed.video_id,
                    )),
                );
            }
        }
    }

    let is_transient_failure = acq_status == AcquisitionStatus::Failed
        && !acq_note.contains("Security violation")
        && !acq_note.contains("license declaration")
        && !acq_note.contains("SHA-256")
        && !acq_note.contains("Invalid URL");

    let will_retry = is_transient_failure && job.attempts < 3;

    // Finalize submission record (or mark awaiting retry if transient failure and retries remain)
    current_sub.duration_ms = if metadata.duration_ms > 0 {
        Some(metadata.duration_ms)
    } else {
        None
    };
    current_sub.bpm = metadata.original_bpm;
    current_sub.half_time_bpm = metadata.half_time_bpm;
    current_sub.double_time_bpm = metadata.double_time_bpm;
    current_sub.musical_key = metadata.musical_key.clone();
    current_sub.camelot_key = metadata.camelot_key.clone();
    current_sub.overall_energy = metadata.overall_energy;
    current_sub.danceability = metadata.danceability;
    current_sub.metadata_source = metadata_source;
    if will_retry {
        current_sub.acquisition_status = "awaiting_authorized_audio".to_string();
        current_sub.acquisition_note = Some(format!(
            "Transient network/resolver failure (attempt {}/3); queued for automatic retry: {}",
            job.attempts, acq_note
        ));
    } else {
        current_sub.acquisition_status = acq_status.as_str().to_string();
        current_sub.acquisition_note = Some(acq_note.clone());
    }
    current_sub.local_cache_path = cached_entry.as_ref().map(|c| c.file_path.clone());
    current_sub.cache_id = cached_entry.as_ref().map(|c| c.id.clone());

    if let Err(e) = state.db.update_submission_pipeline_result(&current_sub) {
        warn!("Failed to persist async pipeline result for submission {}: {}", current_sub.id, e);
    }

    // Stage 6: Metadata-only sync (offline-safe outbox, zero audio bytes)
    let _ = stage_6_sync_metadata_only(&parsed, &metadata, &state.sync_manager, &state.db).await;

    let final_job_status = if will_retry {
        "received"
    } else {
        match acq_status {
            AcquisitionStatus::CachedAuthorized => "ready",
            AcquisitionStatus::RestrictedByProvider => "restricted_by_provider",
            AcquisitionStatus::Failed => "failed",
        }
    };
    let final_err = if acq_status == AcquisitionStatus::Failed {
        Some(acq_note.as_str())
    } else {
        None
    };
    let _ = state
        .db
        .update_submission_intake_job_status(&job.id, final_job_status, final_err);

    // Re-evaluate DJ recommendations and push real-time WebSocket notification reflecting DB truth
    if let Ok(Some(room)) = state.db.get_local_party_room(&job.room_id) {
        if room.status != "closed" {
            let _ = evaluate_and_find_next_playable(&room, &state.db);
        }
    }

    let persisted_sub = state
        .db
        .get_local_room_submissions(&job.room_id)
        .ok()
        .and_then(|subs| subs.into_iter().find(|s| s.id == job.submission_id))
        .unwrap_or(current_sub);

    let ws_event_name = if will_retry {
        "dj.submission_updated"
    } else {
        "dj.submission_ready"
    };

    state.broadcast_event(
        &job.room_id,
        ws_event_name,
        true,
        None,
        serde_json::json!({
            "submission_id": persisted_sub.id,
            "title": persisted_sub.title,
            "artist": persisted_sub.artist,
            "acquisition_status": persisted_sub.acquisition_status,
            "cache_id": persisted_sub.cache_id,
            "dj_status": persisted_sub.dj_status,
            "dj_score": persisted_sub.dj_score,
            "bpm": persisted_sub.bpm,
            "camelot_key": persisted_sub.camelot_key,
            "attempts": job.attempts,
            "will_retry": will_retry
        }),
    );

    true
}

/// Spawns the persistent background intake queue worker that survives helper restarts
/// and drains queued song submissions without blocking HTTP request handlers.
pub fn start_intake_queue_worker(state: Arc<LocalHelperState>) -> tokio::task::JoinHandle<()> {
    let recovered = state.db.recover_interrupted_intake_jobs().unwrap_or(0);
    if recovered > 0 {
        info!("Recovered {} interrupted submission intake job(s) on startup.", recovered);
    }

    tokio::spawn(async move {
        info!("Persistent async submission intake worker started.");
        let mut tick_count: u64 = 0;
        loop {
            tick_count = tick_count.wrapping_add(1);
            if tick_count % 15 == 0 {
                let stale = state.db.recover_stale_intake_jobs(120_000).unwrap_or(0);
                if stale > 0 {
                    warn!("Recovered {} stale in-flight submission intake job(s).", stale);
                }
            }

            let mut processed_any = false;
            while let Ok(Some(job)) = state.db.claim_next_submission_intake_job() {
                processed_any = true;
                process_single_intake_job(&state, &job).await;
            }
            if !processed_any {
                tokio::select! {
                    _ = state.intake_notify.notified() => {},
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {},
                }
            }
        }
    })
}

// ============================================================================
// Route Handlers
// ============================================================================

pub async fn local_diagnostic_landing(
    State(state): State<Arc<LocalHelperState>>,
) -> Html<String> {
    let lan = state.current_lan_ip().unwrap_or_else(|| "Not connected to Wi-Fi/LAN".to_string());
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>DHUN Local Helper Status</title>
<style>
  body {{ background:#090a0f; color:#f8fafc; font-family:system-ui,sans-serif; max-width:560px; margin:48px auto; padding:20px; }}
  .card {{ background:#111827; border:1px solid #1f2937; border-radius:16px; padding:22px; }}
  h1 {{ color:#f59e0b; margin:0 0 8px; }}
  code {{ background:#030712; padding:3px 6px; border-radius:6px; color:#5eead4; }}
  p {{ color:#cbd5e1; line-height:1.5; font-size:14px; }}
</style></head>
<body>
  <div class="card">
    <h1>धुन DHUN Local Helper Running</h1>
    <p>Status: <strong>Online (v2.1.0)</strong></p>
    <p>Host Loopback API: <code>http://127.0.0.1:{port}</code></p>
    <p>Detected LAN Wi-Fi IP: <code>{lan}</code></p>
    <p>Open the DHUN public web app in your browser and press <code>M</code> to create and control your private local party room. Guests on the same Wi-Fi join by scanning the QR code generated inside your host console.</p>
  </div>
</body></html>"#,
        port = state.port,
        lan = escape_html(&lan)
    );
    Html(html)
}

pub async fn local_health(
    State(state): State<Arc<LocalHelperState>>,
) -> Json<serde_json::Value> {
    let lan_ip = state.current_lan_ip();
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
        "version": "2.1.0",
        "port": state.port,
        "lan_reachable": lan_ip.is_some(),
        "detected_lan_ip": lan_ip,
        "audio_resolver_configured": state.audio_resolver.is_some(),
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
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

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
        detected_lan_ip: state.current_lan_ip(),
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
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let lvl = DjLevel::from_i32(room.dj_level);
    let (_, next_playable_candidate, transition_style, recommendations) =
        evaluate_and_find_next_playable(&room, &state.db);
    let submissions = state
        .db
        .get_local_room_submissions(&room_id)
        .unwrap_or_default();
    let transition_plan = {
        let cur = submissions.iter().find(|s| s.dj_status == "playing");
        let nxt = next_playable_candidate
            .as_ref()
            .and_then(|p| submissions.iter().find(|s| s.id == p.submission_id));
        match (cur, nxt) {
            (Some(c), Some(n)) => {
                let primary_mood = get_mood_preset(&room.primary_mood_id)
                    .unwrap_or_else(|| get_all_mood_presets().into_iter().next().unwrap());
                let elapsed_mins = room
                    .started_at
                    .map(|s| ((chrono::Utc::now().timestamp_millis() - s).max(0) as f64) / 60_000.0)
                    .unwrap_or(0.0);
                let target_energy = compute_mood_energy_target(&primary_mood, elapsed_mins);
                let out_in = build_track_transition_input(c, 0, &state.db);
                let mut inc_in = build_track_transition_input(n, 1, &state.db);
                if let Some(r) = recommendations.iter().find(|r| r.submission_id == n.id) {
                    inc_in.predicted_energy_direction = Some(r.predicted_energy_direction.clone());
                }
                let recent_techniques: Vec<String> = state
                    .db
                    .get_session_playback_history(&room_id)
                    .map(|h| h.into_iter().rev().take(6).map(|item| item.transition_type).collect())
                    .unwrap_or_default();
                let party_ctx = PartyDecisionContext {
                    dj_level: lvl,
                    mood_id: room.primary_mood_id.clone(),
                    target_energy,
                    recent_techniques,
                    tempo_policy: TempoQualityPolicy::for_dj_level(lvl),
                };
                Some(plan_dj_transition(&out_in, &inc_in, &party_ctx))
            }
            _ => None,
        }
    };
    let pending_intake_jobs = state
        .db
        .get_pending_intake_job_count(&room_id)
        .unwrap_or(0);
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
        dj_skill_profile: lvl.skill_profile(),
        exposed_decks: lvl.max_decks(),
        internal_deck_capacity: lvl.internal_deck_capacity(),
        auto_dj_enabled: lvl.supports_auto_dj(),
        lan_reachable: lan_ip.is_some(),
        detected_lan_ip: lan_ip,
        submissions,
        recommendations,
        pending_intake_jobs,
        next_playable_candidate,
        transition_plan,
        transition_style,
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
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

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
/// Invalidates all guest tokens immediately, deletes unlocked temp cache files immediately,
/// and preserves currently locked audio files with a 60-minute expiry fallback until released
/// (or purges all immediately if `force_release_audio` is true after decks stop).
pub async fn close_local_room(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Option<Json<CloseLocalRoomRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let force_release = payload
        .and_then(|Json(p)| p.force_release_audio)
        .unwrap_or(false);

    let now = chrono::Utc::now().timestamp_millis();
    state
        .db
        .update_local_room_status(&room_id, "closed", now)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let invalidated_guests = state.db.delete_local_room_guests(&room_id).unwrap_or(0);
    let (purged_cache_files, deferred_locked_files) = if force_release {
        (state.cache_manager.purge_room_cache(&room_id, &state.db).unwrap_or(0), 0)
    } else {
        state
            .cache_manager
            .purge_room_cache_safely(&room_id, &state.db)
            .unwrap_or((0, 0))
    };

    state.broadcast_event(
        &room_id,
        "room.closed",
        false,
        None,
        serde_json::json!({ "status": "closed", "closed_at": now }),
    );

    info!(
        "Explicitly closed local room {}: invalidated {} guests, purged {} cache files, deferred {} in-use locked files",
        room_id, invalidated_guests, purged_cache_files, deferred_locked_files
    );

    Ok(Json(serde_json::json!({
        "room_id": room_id,
        "status": "closed",
        "closed_at": now,
        "invalidated_guests": invalidated_guests,
        "purged_cache_files": purged_cache_files,
        "deferred_locked_files": deferred_locked_files
    })))
}

/// Host-only post-playback release endpoint: unlocks and purges any deferred temp cache files
/// once the Web Audio engine has cleanly faded out and stopped all decks.
pub async fn release_room_cache(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let purged = state
        .cache_manager
        .purge_room_cache(&room_id, &state.db)
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "room_id": room_id,
        "purged_cache_files": purged
    })))
}

pub async fn update_local_room_config(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<UpdateLocalRoomConfigRequest>,
) -> Result<Json<LocalPartyRoom>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

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

/// Host-Authenticated Local Temporary Cache Audio Streaming Route:
/// `GET /api/local/rooms/:room_id/cache/:cache_id/stream`
///
/// Security guarantees:
/// - Strictly host-only (guests get 403, unauthenticated get 401).
/// - Validates room ownership (`entry.room_id == room_id`).
/// - Canonicalizes path and strictly rejects any path traversal outside `cache_dir`.
/// - Safely handles missing, expired (410 Gone), or deleted entries.
/// - Supports HTTP Byte-Range requests (`206 Partial Content` + `Accept-Ranges: bytes`).
pub async fn stream_local_cached_audio(
    State(state): State<Arc<LocalHelperState>>,
    Path((room_id, cache_id)): Path<(String, String)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Response<Body>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let entry = state
        .db
        .get_local_temp_cache_entry(&cache_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Cached audio entry not found.".to_string()))?;

    // Strict room ownership check (prevents cross-room cache leaks)
    if entry.room_id != room_id {
        return Err((
            StatusCode::NOT_FOUND,
            "Cached audio entry does not belong to this room.".to_string(),
        ));
    }

    // Expiry check
    let now = chrono::Utc::now().timestamp_millis();
    if entry.expires_at <= now {
        return Err((
            StatusCode::GONE,
            "Cached audio entry has expired and is no longer available.".to_string(),
        ));
    }

    // Strict canonical path traversal check (never serve arbitrary filesystem paths)
    let canonical_cache_dir = std::fs::canonicalize(state.cache_manager.cache_dir()).map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Cache storage directory unavailable.".to_string())
    })?;
    let target_path = FsPath::new(&entry.file_path);
    if !target_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            "Cached audio file is missing from temporary storage.".to_string(),
        ));
    }
    let canonical_file = std::fs::canonicalize(target_path).map_err(|_| {
        (StatusCode::NOT_FOUND, "Cached audio file is inaccessible.".to_string())
    })?;
    if !canonical_file.starts_with(&canonical_cache_dir) {
        warn!("Blocked cache path traversal attempt for cache_id '{}'", cache_id);
        return Err((
            StatusCode::FORBIDDEN,
            "Access denied: cache path escapes managed temporary storage.".to_string(),
        ));
    }

    // Read file bytes (bounded by decoded/downloaded authorized audio limit)
    let file_bytes = tokio::fs::read(&canonical_file).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "Could not read cached audio file.".to_string())
    })?;
    let total_len = file_bytes.len();
    if total_len == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Cached audio file is empty.".to_string(),
        ));
    }

    let content_type = detect_audio_mime_type(&canonical_file, &file_bytes);

    // Optional HTTP Byte-Range support (`Range: bytes=start-end`)
    if let Some(range_val) = headers.get(RANGE).and_then(|v| v.to_str().ok()) {
        if let Some((start, end)) = parse_single_byte_range(range_val, total_len) {
            let slice = file_bytes[start..=end].to_vec();
            let resp = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, content_type)
                .header(ACCEPT_RANGES, "bytes")
                .header(CONTENT_LENGTH, slice.len().to_string())
                .header(
                    CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, total_len),
                )
                .body(Body::from(slice))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(resp);
        } else {
            let resp = Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(CONTENT_RANGE, format!("bytes */{}", total_len))
                .body(Body::empty())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(resp);
        }
    }

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_LENGTH, total_len.to_string())
        .body(Body::from(file_bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(resp)
}

fn detect_audio_mime_type(path: &FsPath, bytes: &[u8]) -> &'static str {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return "audio/wav";
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"fLaC" {
        return "audio/flac";
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        return "audio/ogg";
    }
    if (bytes.len() >= 3 && &bytes[0..3] == b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
    {
        return "audio/mpeg";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("ogg") => "audio/ogg",
        Some("m4a") | Some("aac") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

fn parse_single_byte_range(header: &str, total_len: usize) -> Option<(usize, usize)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    let first = spec.split(',').next()?.trim();
    let (start_s, end_s) = first.split_once('-')?;

    if start_s.is_empty() {
        // Suffix range: bytes=-N
        let suffix: usize = end_s.parse().ok()?;
        if suffix == 0 || total_len == 0 {
            return None;
        }
        let start = total_len.saturating_sub(suffix);
        return Some((start, total_len - 1));
    }

    let start: usize = start_s.parse().ok()?;
    if start >= total_len {
        return None;
    }
    let end = if end_s.is_empty() {
        total_len - 1
    } else {
        let parsed_end: usize = end_s.parse().ok()?;
        parsed_end.min(total_len - 1)
    };

    if start > end {
        return None;
    }
    Some((start, end))
}

/// Host-Only Playback Lifecycle Sync Route:
/// Synchronizes actual Web Audio deck events (`prepared`, `playing`, `played`, `decode_failed`)
/// with cache lock priorities (protecting active audio from eviction/deletion) and advances the DJ selector.
pub async fn record_dj_playback_event(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<PlaybackEventRequest>,
) -> Result<Json<PlaybackEventResponse>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let (next_dj_status, cache_tier, cache_locked) = match req.event.as_str() {
        "prepared" => ("prepared", CACHE_PRIORITY_NEXT_DECK, true),
        "playing" => ("playing", CACHE_PRIORITY_PLAYING, true),
        "played" => ("played", CACHE_PRIORITY_EVICTABLE, false),
        "decode_failed" => ("skipped_unplayable", CACHE_PRIORITY_EVICTABLE, false),
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unknown playback event '{}'", other),
            ));
        }
    };

    let subs = state.db.get_local_room_submissions(&room_id).unwrap_or_default();
    let target_sub = subs
        .iter()
        .find(|s| s.id == req.submission_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Submission not found in room.".to_string()))?;

    state
        .db
        .update_submission_dj_state(&req.submission_id, next_dj_status, target_sub.dj_score)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Ok(Some(cache_entry)) = state
        .db
        .get_temp_cache_entry_by_submission(&room_id, &req.submission_id)
    {
        let _ = state.cache_manager.set_temp_track_priority(
            &cache_entry.id,
            cache_tier,
            cache_locked,
            &state.db,
        );
    }

    // Re-evaluate remaining playable candidates (excluding restricted & already-played tracks)
    let (_, next_playable_candidate, transition_style, recommendations) =
        evaluate_and_find_next_playable(&room, &state.db);

    // Notify host WebSocket (and if this submission belonged to a guest, notify ONLY that guest!)
    let guest_target = if target_sub.submitted_by_type == "guest" {
        Some(target_sub.submitted_by_id.clone())
    } else {
        None
    };
    state.broadcast_event(
        &room_id,
        "dj.playback_updated",
        guest_target.is_none(),
        guest_target,
        serde_json::json!({
            "submission_id": req.submission_id,
            "dj_status": next_dj_status,
            "deck_id": req.deck_id,
            "error_detail": req.error_detail
        }),
    );

    Ok(Json(PlaybackEventResponse {
        submission_id: req.submission_id,
        dj_status: next_dj_status.to_string(),
        cache_locked,
        next_playable_candidate,
        recommendations,
        transition_style,
    }))
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
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

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
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

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

    // Fast synchronous Stage 1 URL validation only (<1ms); heavy metadata/acquisition/analysis
    // runs asynchronously in the persistent SQLite-backed intake worker so users never wait!
    let parsed = stage_1_validate_and_identify(&req.url)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid song submission: {}", e)))?;

    let auth_json = req
        .authorized_stream
        .as_ref()
        .and_then(|p| serde_json::to_string(p).ok());

    // Duplicate URL deduplication & idempotent retry of previously failed submissions (never creates duplicate rows!)
    if let Ok(Some(existing_sub)) = state
        .db
        .find_active_submission_by_video_id(&room_id, &parsed.video_id)
    {
        if existing_sub.acquisition_status == "failed" {
            let retry_note = "Retrying previously failed submission on the same persistent submission ID.".to_string();
            let _ = state.db.update_submission_processing_stage(
                &existing_sub.id,
                "received",
                Some(&retry_note),
            );
            let requeued = state
                .db
                .requeue_failed_submission_intake_job(
                    &existing_sub.id,
                    &parsed.canonical_url,
                    auth_json.as_deref(),
                )
                .unwrap_or(false);
            if !requeued {
                let retry_job = SubmissionIntakeJob {
                    id: format!("job_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]),
                    room_id: room_id.clone(),
                    submission_id: existing_sub.id.clone(),
                    raw_url: parsed.canonical_url.clone(),
                    authorized_stream_json: auth_json,
                    status: "received".to_string(),
                    attempts: 0,
                    last_error: None,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                let _ = state.db.enqueue_submission_intake_job(&retry_job);
            }
            state.intake_notify.notify_one();
            return Ok(Json(UnifiedSongSubmissionResponse {
                id: existing_sub.id,
                submitted_by_type: existing_sub.submitted_by_type,
                title: existing_sub.title,
                artist: existing_sub.artist,
                version_tag: existing_sub.version_tag,
                provider: existing_sub.provider,
                status: "retry_queued".to_string(),
                metadata_source: existing_sub.metadata_source,
                acquisition_status: "received".to_string(),
                acquisition_note: retry_note,
                cache_id: existing_sub.cache_id,
                dj_score: existing_sub.dj_score,
                interrupted_current_track: false,
            }));
        }

        // Already active (queued, processing, prepared, or playing) — return idempotently without duplicating!
        return Ok(Json(UnifiedSongSubmissionResponse {
            id: existing_sub.id,
            submitted_by_type: existing_sub.submitted_by_type,
            title: existing_sub.title,
            artist: existing_sub.artist,
            version_tag: existing_sub.version_tag,
            provider: existing_sub.provider,
            status: "already_queued".to_string(),
            metadata_source: existing_sub.metadata_source,
            acquisition_status: existing_sub.acquisition_status,
            acquisition_note: existing_sub.acquisition_note.unwrap_or_else(|| {
                "Track is already in this party's DJ candidate pool (deduplicated).".to_string()
            }),
            cache_id: existing_sub.cache_id,
            dj_score: existing_sub.dj_score,
            interrupted_current_track: false,
        }));
    }

    let submission_id = format!("sub_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let job_id = format!("job_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let now = chrono::Utc::now().timestamp_millis();

    // Instant local metadata hit if already known in SQLite (still enqueues intake job for audio/refresh)
    let existing_meta = state
        .db
        .lookup_global_song_by_video_id(&parsed.video_id)
        .ok()
        .flatten();

    let initial_title = existing_meta
        .as_ref()
        .map(|m| m.title.clone())
        .unwrap_or_else(|| format!("Resolving YouTube track ({})", parsed.video_id));
    let initial_artist = existing_meta
        .as_ref()
        .map(|m| m.artist.clone())
        .unwrap_or_else(|| "Queued in background intake...".to_string());
    let initial_version = existing_meta
        .as_ref()
        .map(|m| m.version_tag.clone())
        .unwrap_or_else(|| "original".to_string());
    let initial_meta_source = if existing_meta.is_some() {
        "local_cache"
    } else {
        "pending"
    };
    let intake_note = "Accepted into persistent intake queue — resolving metadata & authorized audio in background. You can submit more links immediately.".to_string();

    let submission = LocalRoomSubmission {
        id: submission_id.clone(),
        room_id: room_id.clone(),
        submitted_by_type: submitted_by_type.clone(),
        submitted_by_id: submitted_by_id.clone(),
        submitted_by_name: submitted_by_name.clone(),
        provider: parsed.provider.as_str().to_string(),
        source_url: parsed.canonical_url.clone(),
        provider_video_id: parsed.video_id.clone(),
        song_id: existing_meta.as_ref().map(|m| m.song_id.clone()),
        title: initial_title.clone(),
        artist: initial_artist.clone(),
        version_tag: initial_version.clone(),
        thumbnail_url: Some(format!(
            "https://i.ytimg.com/vi/{}/hqdefault.jpg",
            parsed.video_id
        )),
        duration_ms: existing_meta
            .as_ref()
            .and_then(|m| if m.duration_ms > 0 { Some(m.duration_ms) } else { None }),
        bpm: existing_meta.as_ref().and_then(|m| m.original_bpm),
        half_time_bpm: existing_meta.as_ref().and_then(|m| m.half_time_bpm),
        double_time_bpm: existing_meta.as_ref().and_then(|m| m.double_time_bpm),
        musical_key: existing_meta.as_ref().and_then(|m| m.musical_key.clone()),
        camelot_key: existing_meta.as_ref().and_then(|m| m.camelot_key.clone()),
        overall_energy: existing_meta.as_ref().and_then(|m| m.overall_energy),
        danceability: existing_meta.as_ref().and_then(|m| m.danceability),
        genre_tags_json: existing_meta
            .as_ref()
            .map(|m| serde_json::to_string(&m.genre_tags).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string()),
        mood_tags_json: existing_meta
            .as_ref()
            .map(|m| serde_json::to_string(&m.mood_tags).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string()),
        metadata_source: initial_meta_source.to_string(),
        acquisition_status: "received".to_string(),
        acquisition_note: Some(intake_note.clone()),
        dj_status: "queued_for_dj".to_string(),
        dj_score: 0.0,
        local_cache_path: None,
        cache_id: None,
        created_at: now,
        updated_at: now,
    };

    state
        .db
        .insert_local_room_submission(&submission)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let intake_job = SubmissionIntakeJob {
        id: job_id,
        room_id: room_id.clone(),
        submission_id: submission_id.clone(),
        raw_url: parsed.canonical_url.clone(),
        authorized_stream_json: auth_json,
        status: "received".to_string(),
        attempts: 0,
        last_error: None,
        created_at: now,
        updated_at: now,
    };
    state
        .db
        .enqueue_submission_intake_job(&intake_job)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if submitted_by_type == "guest" {
        let _ = state.db.increment_local_guest_submission_count(&submitted_by_id);
    }

    let (scored, _, _, _) = evaluate_and_find_next_playable(&room, &state.db);
    let this_score = scored
        .iter()
        .find(|(sid, _)| sid == &submission_id)
        .map(|(_, sc)| *sc)
        .unwrap_or(0.0);

    // Wake background intake worker immediately (zero polling delay)
    state.intake_notify.notify_one();

    // Host-only WebSocket broadcast (other guests NEVER see this submission!)
    state.broadcast_event(
        &room_id,
        "dj.submission_received",
        true,
        None,
        serde_json::json!({
            "submission_id": submission_id,
            "submitted_by_type": submitted_by_type,
            "title": initial_title,
            "acquisition_status": "received",
            "dj_score": this_score
        }),
    );

    Ok(Json(UnifiedSongSubmissionResponse {
        id: submission_id,
        submitted_by_type,
        title: initial_title,
        artist: initial_artist,
        version_tag: initial_version,
        provider: parsed.provider.as_str().to_string(),
        status: "received_by_dj".to_string(),
        metadata_source: initial_meta_source.to_string(),
        acquisition_status: "received".to_string(),
        acquisition_note: intake_note,
        cache_id: None,
        dj_score: this_score,
        interrupted_current_track: false,
    }))
}

/// Privacy-Isolated Guest Endpoint: returns ONLY this guest's own submissions.
pub async fn get_guest_own_submissions(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<Vec<GuestOwnSubmissionView>>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let guest = verify_guest_token(&headers, &room_id, &state.db)?;
    let mine = state
        .db
        .get_guest_own_submissions(&room_id, &guest.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(mine))
}

/// Host-only Automated DJ evaluation trigger.
pub async fn evaluate_dj_candidate_pool(
    State(state): State<Arc<LocalHelperState>>,
    Path(room_id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<DjEvaluationResponse>, (StatusCode, String)> {
    let lan_ip = state.current_lan_ip();
    verify_lan_and_host_headers(addr.ip(), &headers, lan_ip.as_deref())?;

    let room = state
        .db
        .get_local_party_room(&room_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Local party room not found.".to_string()))?;

    verify_host_token(&headers, &room)?;

    let (scored, next_playable_candidate, transition_style, recommendations) =
        evaluate_and_find_next_playable(&room, &state.db);
    let prepared_next_submission_id = next_playable_candidate
        .as_ref()
        .map(|p| p.submission_id.clone());

    state.broadcast_event(
        &room_id,
        "dj.evaluated",
        true,
        None,
        serde_json::json!({
            "evaluated_count": scored.len(),
            "prepared_next_submission_id": prepared_next_submission_id,
            "transition_style": transition_style
        }),
    );

    Ok(Json(DjEvaluationResponse {
        evaluated_count: scored.len(),
        prepared_next_submission_id,
        next_playable_candidate,
        recommendations,
        interrupted_current_track: false,
        transition_style,
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

    if room.status == "closed" {
        return (StatusCode::GONE, "Party room is closed").into_response();
    }

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
            let should_close = evt.event_type == "room.closed";
            if let Ok(text) = serde_json::to_string(&evt) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            if should_close {
                let _ = sender.send(Message::Close(None)).await;
                break;
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
// Self-Contained LAN Guest Portal HTML (Same-Origin HTTP on Host Wi-Fi/LAN)
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
    let (party_name, is_closed) = match state.db.get_local_party_room(&room_id) {
        Ok(Some(r)) => (r.party_name, r.status == "closed"),
        _ => ("DHUN Local Party".to_string(), false),
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
      background: #090a0f; color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      min-height: 100vh; display: flex; flex-direction: column; padding: 18px;
      max-width: 480px; margin: 0 auto;
    }}
    header {{ text-align: center; padding: 14px 0 18px; border-bottom: 1px solid #1e293b; }}
    .logo {{ font-size: 28px; font-weight: 900; color: #f59e0b; letter-spacing: 2px; }}
    .subtitle {{ font-size: 13px; color: #94a3b8; margin-top: 4px; }}
    .badge {{
      display: inline-block; margin-top: 8px; font-size: 11px; font-weight: 700;
      color: #fde68a; background: rgba(245, 158, 11, 0.14); border: 1px solid rgba(245, 158, 11, 0.35);
      border-radius: 9999px; padding: 4px 12px;
    }}
    .conn-pill {{ font-size: 10px; color: #5eead4; margin-top: 6px; }}
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
    <div class="subtitle">Local Party Guest Request &bull; गाना मांगें</div>
    <div class="badge">{clean_party}</div>
    <div id="connPill" class="conn-pill"></div>
  </header>

  <div class="card" id="closedCard" style="display:{closed_display}; text-align:center;">
    <p style="font-weight:700;color:#fda4af;">This party room has been closed by the host.</p>
    <p style="font-size:12px;color:#94a3b8;margin-top:6px;">Ask the host for a new QR code if a new party begins.</p>
  </div>

  <div class="card" id="joinCard" style="display:{open_display};">
    <label>Your Name / आपका नाम</label>
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
    const storageKey = 'dhun_local_guest_token_' + roomId;
    const params = new URLSearchParams(window.location.search);
    const qrAccessToken = params.get('access') || '';
    let guestToken = localStorage.getItem(storageKey) || '';
    let ws = null;
    let wsAttempts = 0;

    const closedCard = document.getElementById('closedCard');
    const joinCard = document.getElementById('joinCard');
    const submitCard = document.getElementById('submitCard');
    const mySection = document.getElementById('mySection');
    const codeLabel = document.getElementById('codeLabel');
    const codeInput = document.getElementById('codeInput');
    const connPill = document.getElementById('connPill');

    if (!qrAccessToken) {{
      codeLabel.style.display = 'block';
      codeInput.style.display = 'block';
    }}

    function showMsg(elId, ok, text) {{
      const el = document.getElementById(elId);
      el.className = 'msg ' + (ok ? 'ok' : 'err');
      el.textContent = text;
    }}

    function handleRoomClosed() {{
      localStorage.removeItem(storageKey);
      guestToken = '';
      if (ws) {{ try {{ ws.close(); }} catch (e) {{}} ws = null; }}
      joinCard.style.display = 'none';
      submitCard.style.display = 'none';
      mySection.style.display = 'none';
      closedCard.style.display = 'block';
      connPill.textContent = 'Room Closed';
    }}

    function connectGuestWs() {{
      if (!guestToken) return;
      const wsProto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const url = wsProto + '//' + window.location.host + '/ws/local/rooms/' + roomId + '?role=guest&token=' + encodeURIComponent(guestToken);
      try {{
        ws = new WebSocket(url);
        ws.onopen = () => {{ wsAttempts = 0; connPill.textContent = 'Connected on Local Wi-Fi'; }};
        ws.onmessage = (ev) => {{
          try {{
            const data = JSON.parse(ev.data);
            if (data.event_type === 'room.closed') return handleRoomClosed();
            loadMySubmissions();
          }} catch (e) {{}}
        }};
        ws.onclose = () => {{
          if (!guestToken) return;
          connPill.textContent = 'Reconnecting to host Wi-Fi...';
          const delay = Math.min(1500 * Math.pow(2, wsAttempts++), 12000);
          setTimeout(connectGuestWs, delay);
        }};
      }} catch (e) {{}}
    }}

    function enterPartyUi() {{
      joinCard.style.display = 'none';
      submitCard.style.display = 'block';
      mySection.style.display = 'block';
      loadMySubmissions();
      connectGuestWs();
    }}

    if (guestToken && closedCard.style.display === 'none') {{
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
        if (res.status === 410) return handleRoomClosed();
        if (!res.ok) return showMsg('joinMsg', false, await res.text());
        const data = await res.json();
        guestToken = data.guest_token;
        localStorage.setItem(storageKey, guestToken);
        enterPartyUi();
      }} catch (e) {{
        showMsg('joinMsg', false, 'Could not reach host laptop on local Wi-Fi.');
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
          headers: {{ 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + guestToken }},
          body: JSON.stringify({{ url }})
        }});
        if (res.status === 410) return handleRoomClosed();
        if (res.status === 401) {{
          localStorage.removeItem(storageKey);
          guestToken = '';
          submitCard.style.display = 'none';
          joinCard.style.display = 'block';
          return showMsg('joinMsg', false, 'Session expired. Please re-join with your room code or QR.');
        }}
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
        if (res.status === 401 || res.status === 410) return handleRoomClosed();
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

    setInterval(loadMySubmissions, 8000);
  </script>
</body>
</html>"#,
        clean_party = clean_party,
        clean_room_id = clean_room_id,
        closed_display = if is_closed { "block" } else { "none" },
        open_display = if is_closed { "none" } else { "block" },
    );

    Html(html)
}

// ============================================================================
// Router Builder (Notice: ZERO Approval/Rejection Routes & ZERO Global Catalogue Browse)
// ============================================================================

pub fn create_local_helper_router(state: Arc<LocalHelperState>) -> Router {
    Router::new()
        .route("/", get(local_diagnostic_landing))
        .route("/api/local/health", get(local_health))
        .route("/api/local/rooms", post(create_local_room))
        .route("/api/local/rooms/:room_id", get(get_host_room_dashboard))
        .route("/api/local/rooms/:room_id/start", post(start_local_room))
        .route("/api/local/rooms/:room_id/close", post(close_local_room))
        .route("/api/local/rooms/:room_id/cache/release", post(release_room_cache))
        .route("/api/local/rooms/:room_id/cache/:cache_id/stream", get(stream_local_cached_audio))
        .route("/api/local/rooms/:room_id/config", post(update_local_room_config))
        .route("/api/local/rooms/:room_id/dj/evaluate", post(evaluate_dj_candidate_pool))
        .route("/api/local/rooms/:room_id/dj/playback-event", post(record_dj_playback_event))
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
