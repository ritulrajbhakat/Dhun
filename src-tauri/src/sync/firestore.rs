use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::models::GlobalSongMetadata;
use crate::metadata_service::{
    compute_canonical_fingerprint, compute_recording_fingerprint, CatalogueUpsertRequest,
};

/// Firestore configuration holding public Firebase project identifiers.
/// No private administrative service account credentials or private keys are ever stored or distributed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirestoreConfig {
    pub project_id: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedAuthToken {
    id_token: String,
    refresh_token: String,
    expires_at_epoch_sec: i64,
}

/// Lightweight client for Cloud Firestore REST API with Firebase Anonymous Auth support.
/// Operates strictly within the Firebase Spark no-cost free tier (reads, writes, no cloud compute).
#[derive(Clone)]
pub struct FirestoreClient {
    config: FirestoreConfig,
    http_client: reqwest::Client,
    cached_auth: Arc<RwLock<Option<CachedAuthToken>>>,
}

impl FirestoreClient {
    pub fn new(config: FirestoreConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(6))
                .build()
                .unwrap_or_default(),
            cached_auth: Arc::new(RwLock::new(None)),
        }
    }

    pub fn project_id(&self) -> &str {
        &self.config.project_id
    }

    /// Explicitly signs in anonymously via Firebase Auth REST API or returns cached token,
    /// returning a detailed error if the configured API key is invalid or rejected by Identity Toolkit.
    pub async fn authenticate_anonymous(&self) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("No Firebase Web API key configured in .env or settings"))?;
        if api_key.trim().is_empty() {
            return Err(anyhow!("Firebase Web API key is empty"));
        }

        let now = chrono::Utc::now().timestamp();
        {
            let read_guard = self.cached_auth.read().await;
            if let Some(ref auth) = *read_guard {
                if now < auth.expires_at_epoch_sec - 120 {
                    return Ok(auth.id_token.clone());
                }
            }
        }

        let mut write_guard = self.cached_auth.write().await;
        if let Some(ref auth) = *write_guard {
            if now < auth.expires_at_epoch_sec - 120 {
                return Ok(auth.id_token.clone());
            }
        }

        let new_token = self.sign_in_anonymously(api_key).await?;
        let id_token = new_token.id_token.clone();
        *write_guard = Some(new_token);
        Ok(id_token)
    }

    /// Verifies that an ID token was issued for the expected Firebase project ID by parsing JWT claims.
    pub fn verify_token_project(&self, id_token: &str) -> Result<bool> {
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid JWT token: expected 3 parts, got {}", parts.len()));
        }
        let payload_bytes = decode_base64_url(parts[1])?;
        let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)?;
        let aud = claims.get("aud").and_then(|v| v.as_str()).unwrap_or("");
        Ok(aud == self.config.project_id)
    }

    /// Obtains a valid Firebase ID token using Anonymous Auth or Token Refresh.
    /// Returns None if no Web API key is configured (allowing unauthenticated access if rules permit).
    pub async fn get_id_token(&self) -> Option<String> {
        let api_key = self.config.api_key.as_ref()?;
        if api_key.trim().is_empty() {
            return None;
        }

        let now = chrono::Utc::now().timestamp();

        // 1. Check existing cached token
        {
            let read_guard = self.cached_auth.read().await;
            if let Some(ref auth) = *read_guard {
                if now < auth.expires_at_epoch_sec - 120 {
                    return Some(auth.id_token.clone());
                }
            }
        }

        // 2. Refresh or acquire anonymous token
        let mut write_guard = self.cached_auth.write().await;
        // Double check in case another task refreshed it
        if let Some(ref auth) = *write_guard {
            if now < auth.expires_at_epoch_sec - 120 {
                return Some(auth.id_token.clone());
            }
        }

        // Try token refresh if refresh_token is available
        if let Some(ref auth) = *write_guard {
            if let Ok(new_token) = self.refresh_token(api_key, &auth.refresh_token).await {
                let id_token = new_token.id_token.clone();
                *write_guard = Some(new_token);
                return Some(id_token);
            }
        }

        // Otherwise acquire a fresh anonymous session via Firebase Auth REST API
        match self.sign_in_anonymously(api_key).await {
            Ok(new_token) => {
                let id_token = new_token.id_token.clone();
                *write_guard = Some(new_token);
                Some(id_token)
            }
            Err(e) => {
                warn!("Firebase anonymous authentication notice: {}. Continuing with unauthenticated request.", e);
                None
            }
        }
    }

    async fn sign_in_anonymously(&self, api_key: &str) -> Result<CachedAuthToken> {
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signUp?key={}",
            api_key
        );
        let resp = self
            .http_client
            .post(&url)
            .json(&json!({ "returnSecureToken": true }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Firebase signUp failed ({}): {}", status, body));
        }

        let data: serde_json::Value = resp.json().await?;
        let id_token = data
            .get("idToken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("idToken missing in response"))?
            .to_string();
        let refresh_token = data
            .get("refreshToken")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let expires_in_sec: i64 = data
            .get("expiresIn")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);

        let now = chrono::Utc::now().timestamp();
        Ok(CachedAuthToken {
            id_token,
            refresh_token,
            expires_at_epoch_sec: now + expires_in_sec,
        })
    }

    async fn refresh_token(&self, api_key: &str, refresh_token: &str) -> Result<CachedAuthToken> {
        let url = format!("https://securetoken.googleapis.com/v1/token?key={}", api_key);
        let resp = self
            .http_client
            .post(&url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Token refresh returned status {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await?;
        let id_token = data
            .get("id_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("id_token missing in refresh response"))?
            .to_string();
        let new_refresh = data
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .unwrap_or(refresh_token)
            .to_string();
        let expires_in_sec: i64 = data
            .get("expires_in")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);

        let now = chrono::Utc::now().timestamp();
        Ok(CachedAuthToken {
            id_token,
            refresh_token: new_refresh,
            expires_at_epoch_sec: now + expires_in_sec,
        })
    }

    /// Fetches a document from Firestore by collection and document ID.
    pub async fn get_document(
        &self,
        collection: &str,
        doc_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let clean_doc_id = sanitize_doc_id(doc_id);
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.config.project_id, collection, clean_doc_id
        );

        let mut req = self.http_client.get(&url);
        if let Some(token) = self.get_id_token().await {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("Firestore GET {} notice: {}", url, e);
                return Err(anyhow!("Firestore network error connecting to {url}: {e}"));
            }
        };

        if resp.status().as_u16() == 404 {
            // True cache miss: document does not exist in Firestore
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 403 || status.as_u16() == 401 {
                return Err(anyhow!("Firestore permission/auth error ({status}): {body}"));
            } else if status.as_u16() == 429 {
                return Err(anyhow!("Firestore Spark quota exceeded (429): {body}"));
            } else {
                return Err(anyhow!("Firestore GET returned error status ({status}): {body}"));
            }
        }

        let doc: serde_json::Value = resp.json().await?;
        Ok(Some(doc))
    }

    /// Lists documents from the Firestore catalogue collection, supporting pagination.
    pub async fn list_catalogue(
        &self,
        page_size: usize,
    ) -> Result<Vec<(String, serde_json::Value)>> {
        let mut results = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!(
                "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/catalogue?pageSize={}",
                self.config.project_id, page_size
            );
            if let Some(ref pt) = page_token {
                url.push_str(&format!("&pageToken={}", pt));
            }

            let mut req = self.http_client.get(&url);
            if let Some(token) = self.get_id_token().await {
                req = req.header("Authorization", format!("Bearer {}", token));
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow!("Firestore catalogue list failed ({status}): {body}"));
            }

            let body: serde_json::Value = resp.json().await?;
            let documents = body
                .get("documents")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default();

            for doc in documents {
                let name = doc.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let doc_id = name.rsplit('/').next().unwrap_or("").to_string();
                results.push((doc_id, doc));
            }

            if let Some(next_pt) = body.get("nextPageToken").and_then(|t| t.as_str()) {
                if !next_pt.trim().is_empty() {
                    page_token = Some(next_pt.to_string());
                    continue;
                }
            }
            break;
        }

        Ok(results)
    }

    /// Deletes a document from Firestore by collection and document ID.
    pub async fn delete_document(
        &self,
        collection: &str,
        doc_id: &str,
    ) -> Result<(reqwest::StatusCode, String)> {
        let clean_doc_id = sanitize_doc_id(doc_id);
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.config.project_id, collection, clean_doc_id
        );

        let mut req = self.http_client.delete(&url);
        if let Some(token) = self.get_id_token().await {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Ok((status, body))
    }

    /// Lists subcollection IDs for a document in the catalogue.
    pub async fn list_subcollections(&self, doc_id: &str) -> Result<Vec<String>> {
        let clean_doc_id = sanitize_doc_id(doc_id);
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/catalogue/{}:listCollectionIds",
            self.config.project_id, clean_doc_id
        );

        let mut req = self.http_client.post(&url).json(&serde_json::json!({}));
        if let Some(token) = self.get_id_token().await {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body: serde_json::Value = resp.json().await?;
        let collection_ids = body
            .get("collectionIds")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(collection_ids)
    }

    /// Upserts a document in Firestore via PATCH.
    pub async fn patch_document(
        &self,
        collection: &str,
        doc_id: &str,
        doc_json: serde_json::Value,
    ) -> Result<()> {
        let clean_doc_id = sanitize_doc_id(doc_id);
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.config.project_id, collection, clean_doc_id
        );

        let mut req = self
            .http_client
            .patch(&url)
            .header("Content-Type", "application/json")
            .json(&doc_json);

        if let Some(token) = self.get_id_token().await {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 403 || status.as_u16() == 401 {
                return Err(anyhow!("Firestore permission/auth error ({status}): {body}"));
            } else if status.as_u16() == 429 {
                return Err(anyhow!("Firestore Spark quota exceeded (429): {body}"));
            } else {
                return Err(anyhow!("Firestore PATCH failed ({status}): {body}"));
            }
        }

        Ok(())
    }

    /// Queries the Firestore catalogue collection by canonical fingerprint.
    pub async fn lookup_by_fingerprint(&self, fp: &str) -> Result<Option<GlobalSongMetadata>> {
        let doc_id = format!("fp_{}", fp);
        match self.get_document("catalogue", &doc_id).await? {
            Some(doc) => {
                if let Some(meta) = from_firestore_document(&doc) {
                    Ok(Some(meta))
                } else {
                    Err(anyhow!("Corrupted document in Firestore: catalogue/{}", doc_id))
                }
            }
            None => Ok(None),
        }
    }

    /// Queries the Firestore catalogue collection by recording identity (duration-independent).
    /// Used for cross-upload song matching across different YouTube uploads of the same song.
    pub async fn lookup_by_recording_id(&self, rec_fp: &str) -> Result<Option<GlobalSongMetadata>> {
        let doc_id = format!("rec_{}", rec_fp);
        match self.get_document("catalogue", &doc_id).await? {
            Some(doc) => {
                if let Some(meta) = from_firestore_document(&doc) {
                    Ok(Some(meta))
                } else {
                    Err(anyhow!("Corrupted document in Firestore: catalogue/{}", doc_id))
                }
            }
            None => Ok(None),
        }
    }

    /// Queries the Firestore catalogue collection by provider video ID alias.
    pub async fn lookup_by_video_id(&self, video_id: &str) -> Result<Option<GlobalSongMetadata>> {
        let doc_id = format!("vid_{}", video_id);
        match self.get_document("catalogue", &doc_id).await? {
            Some(doc) => {
                if let Some(meta) = from_firestore_document(&doc) {
                    Ok(Some(meta))
                } else {
                    Err(anyhow!("Corrupted document in Firestore: catalogue/{}", doc_id))
                }
            }
            None => Ok(None),
        }
    }

    /// Publishes song metadata to Firestore under its canonical fingerprint and video ID alias.
    pub async fn publish_metadata(
        &self,
        payload: &CatalogueUpsertRequest,
        video_id: Option<&str>,
    ) -> Result<()> {
        let vtag = payload
            .version_tag
            .clone()
            .unwrap_or_else(|| "original".to_string());
        let dur = payload.duration_ms.unwrap_or(0);
        let fp = compute_canonical_fingerprint(&payload.title, &payload.artist, &vtag, dur);

        let doc_value = to_firestore_document(payload, &fp, video_id);

        // 1. Primary document indexed by canonical recording fingerprint (exact duration bucket)
        let primary_id = format!("fp_{}", fp);
        self.patch_document("catalogue", &primary_id, doc_value.clone())
            .await?;
        info!("Published catalogue metadata to Firestore: catalogue/{}", primary_id);

        // 2. Fast O(1) direct alias indexed by provider video ID if present
        if let Some(vid) = video_id {
            if !vid.trim().is_empty() {
                let alias_id = format!("vid_{}", vid.trim());
                if let Err(e) = self.patch_document("catalogue", &alias_id, doc_value.clone()).await {
                    debug!("Notice: video ID alias write returned: {}", e);
                } else {
                    info!("Published provider video ID alias: catalogue/{}", alias_id);
                }
            }
        }

        // 3. Duration-independent recording identity index
        // Enables cross-upload matching for any other DHUN installation adding a different video of the same song
        let rec_fp = compute_recording_fingerprint(&payload.title, &payload.artist, &vtag);
        let rec_id = format!("rec_{}", rec_fp);
        if let Err(e) = self.patch_document("catalogue", &rec_id, doc_value).await {
            debug!("Notice: recording identity index write returned: {}", e);
        } else {
            info!("Published recording identity alias: catalogue/{}", rec_id);
        }

        Ok(())
    }
}

/// Converts a `CatalogueUpsertRequest` into a typed Firestore document JSON payload.
pub fn to_firestore_document(
    song: &CatalogueUpsertRequest,
    canonical_fingerprint: &str,
    video_id: Option<&str>,
) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    fields.insert(
        "canonical_fingerprint".to_string(),
        json!({ "stringValue": canonical_fingerprint }),
    );
    fields.insert("title".to_string(), json!({ "stringValue": song.title }));
    fields.insert("artist".to_string(), json!({ "stringValue": song.artist }));
    fields.insert(
        "version_tag".to_string(),
        json!({ "stringValue": song.version_tag.as_deref().unwrap_or("original") }),
    );
    fields.insert(
        "duration_ms".to_string(),
        json!({ "integerValue": song.duration_ms.unwrap_or(0).to_string() }),
    );

    if let Some(bpm) = song.original_bpm {
        fields.insert("original_bpm".to_string(), json!({ "doubleValue": bpm }));
    }
    if let Some(half) = song.half_time_bpm {
        fields.insert("half_time_bpm".to_string(), json!({ "doubleValue": half }));
    }
    if let Some(double) = song.double_time_bpm {
        fields.insert("double_time_bpm".to_string(), json!({ "doubleValue": double }));
    }
    if let Some(conf) = song.bpm_confidence {
        fields.insert("bpm_confidence".to_string(), json!({ "doubleValue": conf }));
    }
    if let Some(ref key) = song.musical_key {
        fields.insert("musical_key".to_string(), json!({ "stringValue": key }));
    }
    if let Some(ref camelot) = song.camelot_key {
        fields.insert("camelot_key".to_string(), json!({ "stringValue": camelot }));
    }
    if let Some(conf) = song.key_confidence {
        fields.insert("key_confidence".to_string(), json!({ "doubleValue": conf }));
    }
    if let Some(ref grid) = song.beat_grid_json {
        fields.insert("beat_grid_json".to_string(), json!({ "stringValue": grid }));
    }
    if let Some(ref phrases) = song.phrase_markers_json {
        fields.insert(
            "phrase_markers_json".to_string(),
            json!({ "stringValue": phrases }),
        );
    }
    if let Some(energy) = song.overall_energy {
        fields.insert("overall_energy".to_string(), json!({ "doubleValue": energy }));
    }
    if let Some(dance) = song.danceability {
        fields.insert("danceability".to_string(), json!({ "doubleValue": dance }));
    }

    if let Some(ref genres) = song.genre_tags {
        let vals: Vec<_> = genres.iter().map(|g| json!({ "stringValue": g })).collect();
        fields.insert("genre_tags".to_string(), json!({ "arrayValue": { "values": vals } }));
    }
    if let Some(ref moods) = song.mood_tags {
        let vals: Vec<_> = moods.iter().map(|m| json!({ "stringValue": m })).collect();
        fields.insert("mood_tags".to_string(), json!({ "arrayValue": { "values": vals } }));
    }

    fields.insert(
        "analysis_version".to_string(),
        json!({ "integerValue": song.analysis_version.unwrap_or(2).to_string() }),
    );
    fields.insert(
        "analysis_confidence".to_string(),
        json!({ "doubleValue": song.analysis_confidence.unwrap_or(1.0) }),
    );
    fields.insert(
        "processing_status".to_string(),
        json!({ "stringValue": song.processing_status.as_deref().unwrap_or("analyzed") }),
    );

    let mut source_urls_vals = vec![json!({ "stringValue": song.source_url })];
    if let Some(vid) = video_id {
        source_urls_vals.push(json!({ "stringValue": format!("youtube:{}", vid) }));
    }
    fields.insert(
        "source_urls".to_string(),
        json!({ "arrayValue": { "values": source_urls_vals } }),
    );

    fields.insert("provenance".to_string(), json!({ "stringValue": "dhun_local_dsp" }));

    let now_ms = chrono::Utc::now().timestamp_millis();
    fields.insert(
        "created_at".to_string(),
        json!({ "integerValue": now_ms.to_string() }),
    );
    fields.insert(
        "updated_at".to_string(),
        json!({ "integerValue": now_ms.to_string() }),
    );

    json!({ "fields": fields })
}

/// Converts a Firestore document into a `GlobalSongMetadata` entity.
pub fn from_firestore_document(doc: &serde_json::Value) -> Option<GlobalSongMetadata> {
    let fields = doc.get("fields")?.as_object()?;

    let get_str = |k: &str| -> Option<String> {
        fields.get(k)?.get("stringValue")?.as_str().map(String::from)
    };
    let get_double = |k: &str| -> Option<f64> { fields.get(k)?.get("doubleValue")?.as_f64() };
    let get_int = |k: &str| -> Option<i64> {
        fields
            .get(k)?
            .get("integerValue")?
            .as_str()?
            .parse::<i64>()
            .ok()
    };
    let get_str_array = |k: &str| -> Vec<String> {
        fields
            .get(k)
            .and_then(|v| v.get("arrayValue"))
            .and_then(|a| a.get("values"))
            .and_then(|vals| vals.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("stringValue")?.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };

    let title = get_str("title")?;
    let artist = get_str("artist")?;
    let version_tag = get_str("version_tag").unwrap_or_else(|| "original".to_string());
    let duration_ms = get_int("duration_ms").unwrap_or(0);

    let canonical_fingerprint = get_str("canonical_fingerprint").unwrap_or_else(|| {
        compute_canonical_fingerprint(&title, &artist, &version_tag, duration_ms)
    });

    let song_id = format!("gsng_{}", &Uuid::new_v4().to_string().replace('-', "")[..16]);
    let now = chrono::Utc::now().timestamp_millis();

    Some(GlobalSongMetadata {
        song_id,
        canonical_fingerprint,
        title,
        artist,
        version_tag,
        duration_ms,
        original_bpm: get_double("original_bpm"),
        half_time_bpm: get_double("half_time_bpm"),
        double_time_bpm: get_double("double_time_bpm"),
        bpm_confidence: get_double("bpm_confidence"),
        musical_key: get_str("musical_key"),
        camelot_key: get_str("camelot_key"),
        key_confidence: get_double("key_confidence"),
        beat_grid_json: get_str("beat_grid_json"),
        phrase_markers_json: get_str("phrase_markers_json"),
        overall_energy: get_double("overall_energy"),
        danceability: get_double("danceability"),
        genre_tags: get_str_array("genre_tags"),
        mood_tags: get_str_array("mood_tags"),
        analysis_version: get_int("analysis_version").unwrap_or(2) as i32,
        analysis_confidence: get_double("analysis_confidence").unwrap_or(1.0),
        processing_status: get_str("processing_status").unwrap_or_else(|| "analyzed".to_string()),
        source_urls: get_str_array("source_urls"),
        created_at: get_int("created_at").unwrap_or(now),
        updated_at: get_int("updated_at").unwrap_or(now),
    })
}

fn sanitize_doc_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn decode_base64_url(input: &str) -> Result<Vec<u8>> {
    let mut s = input.replace('-', "+").replace('_', "/");
    while s.len() % 4 != 0 {
        s.push('=');
    }
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = TABLE
            .iter()
            .position(|&x| x == b)
            .ok_or_else(|| anyhow!("Invalid base64 character: {}", b as char))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_and_from_firestore_document() {
        let upsert = CatalogueUpsertRequest {
            source_url: "https://www.youtube.com/watch?v=DuFUtL8zUAk".to_string(),
            title: "Animals".to_string(),
            artist: "Martin Garrix".to_string(),
            version_tag: Some("original".to_string()),
            duration_ms: Some(186000),
            original_bpm: Some(128.0),
            half_time_bpm: Some(64.0),
            double_time_bpm: Some(256.0),
            bpm_confidence: Some(0.99),
            musical_key: Some("F Minor".to_string()),
            camelot_key: Some("4A".to_string()),
            key_confidence: Some(0.98),
            beat_grid_json: Some(r#"{"bpm":128.0}"#.to_string()),
            phrase_markers_json: Some(r#"[{"type":"drop","start_beat":64}]"#.to_string()),
            overall_energy: Some(0.88),
            danceability: Some(0.95),
            genre_tags: Some(vec!["edm".to_string(), "big_room".to_string()]),
            mood_tags: Some(vec!["house_party".to_string()]),
            analysis_version: Some(2),
            analysis_confidence: Some(0.98),
            processing_status: Some("analyzed".to_string()),
        };

        let fp = "a1b2c3d4e5f6";
        let doc = to_firestore_document(&upsert, fp, Some("DuFUtL8zUAk"));

        assert!(doc.get("fields").is_some());
        let meta = from_firestore_document(&doc).expect("Deserialization failed");

        assert_eq!(meta.title, "Animals");
        assert_eq!(meta.artist, "Martin Garrix");
        assert_eq!(meta.canonical_fingerprint, "a1b2c3d4e5f6");
        assert_eq!(meta.original_bpm, Some(128.0));
        assert_eq!(meta.camelot_key.as_deref(), Some("4A"));
        assert_eq!(meta.genre_tags.len(), 2);
        assert_eq!(meta.processing_status, "analyzed");
    }
}
