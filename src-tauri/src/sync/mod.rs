use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::{SyncOutboxItem, SyncStatusOverview};

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
    client: reqwest::Client,
}

impl MetadataSyncManager {
    pub fn new(remote_endpoint: Option<String>, api_key: Option<String>) -> Self {
        Self {
            remote_endpoint,
            api_key,
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Creates sync manager loading any persisted remote sync endpoint and API key from environment or SQLite
    pub fn new_from_db(db: &Database) -> Self {
        let endpoint = std::env::var("DHUN_SYNC_ENDPOINT")
            .ok()
            .or_else(|| db.get_app_setting("sync_remote_endpoint").unwrap_or(None));
        let api_key = std::env::var("DHUN_SYNC_API_KEY")
            .ok()
            .or_else(|| db.get_app_setting("sync_api_key").unwrap_or(None));
        Self::new(endpoint, api_key)
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
        // If no remote endpoint is configured, keep all items queued locally in pending status
        let Some(_) = &self.remote_endpoint else {
            return Ok(0);
        };

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

    /// Dispatches a single sync item to authenticated remote service with idempotency & request signing
    async fn dispatch_sync_item(&self, item: &SyncOutboxItem) -> Result<()> {
        let Some(endpoint) = &self.remote_endpoint else {
            return Err(anyhow!("No remote sync service configured; item remains in local pending queue."));
        };

        let url = format!("{}/api/v1/sync/{}", endpoint.trim_end_matches('/'), item.entity_type);
        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("X-Dhun-Idempotency-Key", &item.id)
            .header("X-Dhun-Entity-Type", &item.entity_type);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));

            let mut hasher = Sha256::new();
            hasher.update(key.as_bytes());
            hasher.update(b":");
            hasher.update(item.payload_json.as_bytes());
            let sig = format!("{:x}", hasher.finalize());
            req = req.header("X-Dhun-Signature", format!("sha256={}", sig));
        }

        let resp = req
            .body(item.payload_json.clone())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Remote returned status: {}", resp.status()));
        }

        Ok(())
    }

    pub fn get_sync_overview(&self, db: &Database) -> Result<SyncStatusOverview> {
        Ok(db.get_sync_overview()?)
    }
}
