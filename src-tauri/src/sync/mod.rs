use std::sync::Arc;
use anyhow::{anyhow, Result};
use ring::hmac;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::{SyncOutboxItem, SyncStatusOverview};

pub mod firestore;
pub use firestore::{FirestoreClient, FirestoreConfig};

/// Loads key-value pairs from .env into std::env if not already set, checking common search paths.
pub fn load_dotenv_if_present() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok().map(std::path::PathBuf::from);
    let mut search_paths = vec![
        std::path::PathBuf::from(".env"),
        std::path::PathBuf::from("../.env"),
        std::path::PathBuf::from("../../.env"),
    ];
    if let Some(ref md) = manifest_dir {
        search_paths.push(md.join(".env"));
        if let Some(parent) = md.parent() {
            search_paths.push(parent.join(".env"));
        }
    }
    for path in &search_paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let key = k.trim();
                        let val = v.trim().trim_matches('"').trim_matches('\'');
                        if std::env::var(key).is_err() && !val.is_empty() {
                            std::env::set_var(key, val);
                        }
                    }
                }
            }
            break;
        }
    }
}

/// Computes SHA-256 digest of payload for canonical message binding
pub fn compute_payload_hash(payload_json: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload_json.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Constructs canonical request string: METHOD\nPATH\nTIMESTAMP\nPAYLOAD_SHA256
pub fn create_canonical_string(method: &str, path: &str, timestamp: i64, payload_hash: &str) -> String {
    format!("{}\n{}\n{}\n{}", method.to_uppercase(), path, timestamp, payload_hash)
}

/// Computes true HMAC-SHA256 signature using NIST-standard ring::hmac
pub fn sign_request(api_key: &str, method: &str, path: &str, timestamp: i64, payload_json: &str) -> String {
    let payload_hash = compute_payload_hash(payload_json);
    let canonical = create_canonical_string(method, path, timestamp, &payload_hash);
    let key = hmac::Key::new(hmac::HMAC_SHA256, api_key.as_bytes());
    let tag = hmac::sign(&key, canonical.as_bytes());
    tag.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time verification of HMAC-SHA256 signature with replay window validation (±300 seconds)
pub fn verify_signature(
    api_key: &str,
    method: &str,
    path: &str,
    timestamp: i64,
    payload_json: &str,
    signature_hex: &str,
) -> bool {
    let now = chrono::Utc::now().timestamp();
    // Replay attack prevention: timestamp must be within 300s of current server time (checked subtraction prevents i64 overflow)
    let in_replay_window = now
        .checked_sub(timestamp)
        .map(|diff| (-300..=300).contains(&diff))
        .unwrap_or(false);
    if !in_replay_window {
        return false;
    }

    let clean_sig = signature_hex.strip_prefix("sha256=").unwrap_or(signature_hex);
    if clean_sig.len() != 64 {
        return false;
    }

    let sig_bytes = match (0..clean_sig.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean_sig[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
    {
        Ok(b) => b,
        Err(_) => return false,
    };

    let payload_hash = compute_payload_hash(payload_json);
    let canonical = create_canonical_string(method, path, timestamp, &payload_hash);
    let key = hmac::Key::new(hmac::HMAC_SHA256, api_key.as_bytes());

    hmac::verify(&key, canonical.as_bytes(), &sig_bytes).is_ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadataCorrection {
    pub track_id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub camelot_key: Option<String>,
    pub manual_override: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackSyncEvent {
    pub session_id: String,
    pub track_id: String,
    pub played_at: i64,
    pub duration_played_ms: i64,
    pub was_skipped: bool,
    pub transition_type: String,
}

#[derive(Clone)]
pub struct MetadataSyncManager {
    remote_endpoint: Option<String>,
    api_key: Option<String>,
    firestore: Option<FirestoreClient>,
    client: reqwest::Client,
}

impl MetadataSyncManager {
    pub fn new(remote_endpoint: Option<String>, api_key: Option<String>) -> Result<Self> {
        Self::new_with_firestore(remote_endpoint, api_key, None)
    }

    /// Initializes MetadataSyncManager with optional Cloud Firestore configuration.
    /// Operates fully offline when neither Firestore nor a remote endpoint is provided.
    pub fn new_with_firestore(
        remote_endpoint: Option<String>,
        api_key: Option<String>,
        firestore_config: Option<FirestoreConfig>,
    ) -> Result<Self> {
        if let Some(ref url) = remote_endpoint {
            let lower = url.to_lowercase();
            // Reject insecure HTTP endpoints (only HTTPS is permitted; loopback allowed for unit tests)
            if !lower.starts_with("https://") && !lower.starts_with("http://127.0.0.1") && !lower.starts_with("http://localhost") {
                return Err(anyhow!("Security violation: remote sync endpoint must use HTTPS (got '{}')", url));
            }
            // Enforce mandatory non-empty API key whenever a remote endpoint is configured
            match &api_key {
                Some(key) if !key.trim().is_empty() => {}
                _ => return Err(anyhow!("Authentication required: remote sync endpoint requires a non-empty API key")),
            }
        }

        let firestore = firestore_config.map(FirestoreClient::new);

        Ok(Self {
            remote_endpoint,
            api_key,
            firestore,
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        })
    }

    /// Creates sync manager loading any persisted remote sync endpoint, Firebase credentials, or SQLite settings
    pub fn new_from_db(db: &Database) -> Result<Self> {
        load_dotenv_if_present();
        let endpoint = std::env::var("DHUN_SYNC_ENDPOINT")
            .ok()
            .or_else(|| db.get_app_setting("sync_remote_endpoint").unwrap_or(None));
        let api_key = std::env::var("DHUN_SYNC_API_KEY")
            .ok()
            .or_else(|| db.get_app_setting("sync_api_key").unwrap_or(None));

        let fb_project = std::env::var("DHUN_FIREBASE_PROJECT_ID")
            .ok()
            .or_else(|| std::env::var("VITE_FIREBASE_PROJECT_ID").ok())
            .or_else(|| db.get_app_setting("firebase_project_id").unwrap_or(None));
        let fb_key = std::env::var("DHUN_FIREBASE_API_KEY")
            .ok()
            .or_else(|| std::env::var("VITE_FIREBASE_API_KEY").ok())
            .or_else(|| db.get_app_setting("firebase_api_key").unwrap_or(None));

        let firestore_config = fb_project.filter(|p| !p.trim().is_empty()).map(|project_id| {
            FirestoreConfig {
                project_id,
                api_key: fb_key.filter(|k| !k.trim().is_empty()),
            }
        });

        Self::new_with_firestore(endpoint, api_key, firestore_config)
    }

    pub fn firestore(&self) -> Option<&FirestoreClient> {
        self.firestore.as_ref()
    }

    pub fn remote_endpoint(&self) -> Option<&str> {
        self.remote_endpoint.as_deref()
    }

    /// Recovers any outbox records left in 'in_flight' status from a previous crash/shutdown
    pub fn recover_in_flight_items(&self, db: &Database) -> Result<usize> {
        let count = db.recover_in_flight_sync_items()?;
        if count > 0 {
            info!("Recovered {} in-flight sync outbox records back to pending status", count);
        }
        Ok(count)
    }

    /// Starts an asynchronous background task that periodically flushes pending outbox records
    pub fn start_background_worker(
        self: Arc<Self>,
        db: Database,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Metadata sync background worker started (interval: {}s)", interval_secs);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                if let Err(e) = self.process_pending_outbox(50, &db).await {
                    warn!("Background sync worker error: {}", e);
                }
            }
        })
    }

    /// Enqueues a track metadata correction for online sync
    pub fn enqueue_metadata_correction(
        &self,
        correction: &TrackMetadataCorrection,
        db: &Database,
    ) -> Result<String> {
        let id = format!("sync_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();
        let payload_json = serde_json::to_string(correction)?;

        let item = SyncOutboxItem {
            id: id.clone(),
            entity_type: "track_metadata".to_string(),
            entity_id: correction.track_id.clone(),
            payload_json,
            retry_count: 0,
            status: "pending".to_string(),
            last_attempt_at: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.enqueue_sync_item(&item)?;
        info!("Enqueued metadata sync item '{}' for track '{}'", id, correction.track_id);
        Ok(id)
    }

    /// Enqueues a playback history event for online sync
    pub fn enqueue_playback_event(
        &self,
        event: &PlaybackSyncEvent,
        db: &Database,
    ) -> Result<String> {
        let id = format!("sync_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();
        let payload_json = serde_json::to_string(event)?;

        let item = SyncOutboxItem {
            id: id.clone(),
            entity_type: "playback_event".to_string(),
            entity_id: event.track_id.clone(),
            payload_json,
            retry_count: 0,
            status: "pending".to_string(),
            last_attempt_at: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.enqueue_sync_item(&item)?;
        Ok(id)
    }

    /// Processes pending sync items from the outbox with retry logic & conflict handling
    pub async fn process_pending_outbox(&self, max_batch: usize, db: &Database) -> Result<usize> {
        // If neither Firestore nor a remote endpoint is configured, keep all items queued locally in pending status
        if self.remote_endpoint.is_none() && self.firestore.is_none() {
            return Ok(0);
        }

        let items = db.get_pending_sync_items(max_batch)?;
        if items.is_empty() {
            return Ok(0);
        }

        let mut synced_count = 0;
        let now = chrono::Utc::now().timestamp_millis();

        for item in items {
            // Calculate exponential backoff interval: 2^retry_count * 1000ms
            if let Some(last_attempt) = item.last_attempt_at {
                let backoff_ms = (1i64 << item.retry_count.min(6)) * 1000;
                if now - last_attempt < backoff_ms {
                    continue; // Skip until backoff delay expires
                }
            }

            db.update_sync_item_status(&item.id, "in_flight", None, false)?;

            match self.dispatch_sync_item(&item).await {
                Ok(()) => {
                    db.update_sync_item_status(&item.id, "synced", None, false)?;
                    synced_count += 1;
                    info!("Successfully synced outbox item '{}' ({})", item.id, item.entity_type);
                }
                Err(e) => {
                    let err_msg = format!("Sync dispatch error: {}", e);
                    warn!("Failed sync attempt for '{}': {}", item.id, err_msg);
                    let will_fail_permanently = item.retry_count + 1 >= 5;
                    let next_status = if will_fail_permanently { "failed" } else { "pending" };
                    db.update_sync_item_status(&item.id, next_status, Some(&err_msg), true)?;
                }
            }
        }

        Ok(synced_count)
    }

    /// Dispatches a single sync item to Cloud Firestore or authenticated remote service with idempotency
    async fn dispatch_sync_item(&self, item: &SyncOutboxItem) -> Result<()> {
        // 1. Primary: If Cloud Firestore is configured, publish song metadata directly to Firestore
        if item.entity_type == "global_song_metadata" {
            if let Some(ref firestore) = self.firestore {
                let payload: crate::metadata_service::CatalogueUpsertRequest =
                    serde_json::from_str(&item.payload_json)?;
                let video_id = if !item.entity_id.is_empty() {
                    Some(item.entity_id.as_str())
                } else {
                    None
                };
                firestore.publish_metadata(&payload, video_id).await?;
                return Ok(());
            }
        }

        // 2. Fallback: Custom authenticated HMAC endpoint
        let Some(endpoint) = &self.remote_endpoint else {
            return Err(anyhow!("No remote sync service or Cloud Firestore configured; item remains in local pending queue."));
        };
        let Some(ref key) = self.api_key else {
            return Err(anyhow!("Authentication required: remote sync endpoint configured but API key is missing."));
        };

        let path = if item.entity_type == "global_song_metadata" {
            "/api/v1/catalogue/upsert".to_string()
        } else {
            format!("/api/v1/sync/{}", item.entity_type)
        };
        let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
        let timestamp = chrono::Utc::now().timestamp();
        let sig = sign_request(key, "POST", &path, timestamp, &item.payload_json);

        let resp = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("X-Dhun-Idempotency-Key", &item.id)
            .header("X-Dhun-Entity-Type", &item.entity_type)
            .header("X-Dhun-Timestamp", timestamp.to_string())
            .header("X-Dhun-Signature", format!("sha256={}", sig))
            .header("Authorization", format!("Bearer {}", key))
            .body(item.payload_json.clone())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Remote returned status: {}", resp.status()));
        }

        Ok(())
    }

    /// Enqueues metadata-only song identity & analysis for Global Shared Song Metadata Catalogue sync.
    /// Strictly never transmits raw audio bytes, party IDs, or guest data.
    pub fn enqueue_global_song_metadata(
        &self,
        payload: &crate::metadata_service::CatalogueUpsertRequest,
        video_id: &str,
        db: &Database,
    ) -> Result<String> {
        let id = format!("sync_meta_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
        let now = chrono::Utc::now().timestamp_millis();
        let payload_json = serde_json::to_string(payload)?;

        let item = SyncOutboxItem {
            id: id.clone(),
            entity_type: "global_song_metadata".to_string(),
            entity_id: video_id.to_string(),
            payload_json,
            retry_count: 0,
            status: "pending".to_string(),
            last_attempt_at: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.enqueue_sync_item(&item)?;
        info!("Enqueued metadata-only sync item '{}' for video '{}'", id, video_id);
        Ok(id)
    }

    /// Engine-only lookup against the Global Shared Song Metadata Catalogue (Cloud Firestore or legacy server).
    /// Gracefully returns Ok(None) when offline or when no remote catalogue is configured.
    pub async fn lookup_global_song_metadata(
        &self,
        req: &crate::metadata_service::CatalogueLookupRequest,
    ) -> Result<Option<crate::db::models::GlobalSongMetadata>> {
        // 1. Primary: Cloud Firestore lookup if configured
        if let Some(ref firestore) = self.firestore {
            // A. Lookup by direct video ID
            if let Some(ref vid) = req.provider_video_id {
                match firestore.lookup_by_video_id(vid).await {
                    Ok(Some(hit)) => return Ok(Some(hit)),
                    Ok(None) => {},
                    Err(e) => {
                        debug!("Firestore video ID lookup notice: {}", e);
                    }
                }
            }
            // B. Lookup by exact duration canonical fingerprint (if duration is known)
            if let (Some(ref title), Some(ref artist)) = (&req.title, &req.artist) {
                let vtag = req
                    .version_tag
                    .clone()
                    .unwrap_or_else(|| crate::metadata_service::extract_version_tag(title));

                if let Some(dur) = req.duration_ms {
                    if dur > 0 {
                        let fp = crate::metadata_service::compute_canonical_fingerprint(
                            title,
                            artist,
                            &vtag,
                            dur,
                        );
                        match firestore.lookup_by_fingerprint(&fp).await {
                            Ok(Some(hit)) => return Ok(Some(hit)),
                            Ok(None) => {},
                            Err(e) => {
                                debug!("Firestore exact fingerprint lookup notice: {}", e);
                            }
                        }
                    }
                }

                // C. Cross-upload matching: Lookup by recording identity (duration-independent)
                // This enables a second DHUN installation with a different video upload of the same song
                // to find the existing metadata record published by the first installation!
                let rec_fp = crate::metadata_service::compute_recording_fingerprint(
                    title,
                    artist,
                    &vtag,
                );
                match firestore.lookup_by_recording_id(&rec_fp).await {
                    Ok(Some(hit)) => return Ok(Some(hit)),
                    Ok(None) => {},
                    Err(e) => {
                        debug!("Firestore recording identity lookup notice: {}", e);
                    }
                }

                // D. Also check unk-bucket canonical fingerprint (for legacy/fallback)
                match firestore.lookup_by_fingerprint(&rec_fp).await {
                    Ok(Some(hit)) => return Ok(Some(hit)),
                    Ok(None) => {},
                    Err(e) => {
                        debug!("Firestore fallback unk-fingerprint lookup notice: {}", e);
                    }
                }
            }
            return Ok(None);
        }

        // 2. Fallback: Custom authenticated HMAC endpoint
        let (Some(endpoint), Some(key)) = (&self.remote_endpoint, &self.api_key) else {
            return Ok(None);
        };

        let path = "/api/v1/catalogue/lookup";
        let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
        let payload_json = serde_json::to_string(req)?;
        let timestamp = chrono::Utc::now().timestamp();
        let sig = sign_request(key, "POST", path, timestamp, &payload_json);

        let resp = match self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Dhun-Timestamp", timestamp.to_string())
            .header("X-Dhun-Signature", format!("sha256={}", sig))
            .body(payload_json)
            .timeout(std::time::Duration::from_secs(4))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => return Ok(None),
        };

        let parsed: crate::metadata_service::CatalogueLookupResponse = resp.json().await?;
        Ok(parsed.metadata)
    }

    pub fn get_sync_overview(&self, db: &Database) -> Result<SyncStatusOverview> {
        Ok(db.get_sync_overview()?)
    }
}
