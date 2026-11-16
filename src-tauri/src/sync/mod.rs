use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
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
    client: reqwest::Client,
}

impl MetadataSyncManager {
    pub fn new(remote_endpoint: Option<String>) -> Self {
        Self {
            remote_endpoint,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
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

    /// Dispatches a single sync item to remote endpoint or simulated receiver
    async fn dispatch_sync_item(&self, item: &SyncOutboxItem) -> Result<()> {
        if let Some(endpoint) = &self.remote_endpoint {
            let url = format!("{}/api/v1/sync/{}", endpoint.trim_end_matches('/'), item.entity_type);
            let resp = self.client.post(&url)
                .header("Content-Type", "application/json")
                .body(item.payload_json.clone())
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(anyhow!("Remote returned status: {}", resp.status()));
            }
        } else {
            // Local-first standalone mode: simulated successful sync ack
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    pub fn get_sync_overview(&self, db: &Database) -> Result<SyncStatusOverview> {
        Ok(db.get_sync_overview()?)
    }
}
