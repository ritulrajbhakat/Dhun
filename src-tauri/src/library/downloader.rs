use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::DownloadTask;
use crate::library::ingest::{ingest_audio_asset, IngestResult};

#[derive(Clone)]
pub struct AudioDownloader {
    cache_dir: PathBuf,
    client: reqwest::Client,
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl AudioDownloader {
    pub fn new<P: Into<PathBuf>>(cache_dir: P) -> Self {
        let dir = cache_dir.into();
        let _ = fs::create_dir_all(&dir);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            cache_dir: dir,
            client,
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Cancels an in-progress download task
    pub fn cancel_download(&self, task_id: &str, db: &Database) -> Result<()> {
        let mut map = self.cancellations.lock().unwrap();
        if let Some(cancel_flag) = map.remove(task_id) {
            cancel_flag.store(true, Ordering::SeqCst);
        }
        db.update_download_status(task_id, "cancelled", Some("Cancelled by user"))?;
        info!("Cancelled download task '{}'", task_id);
        Ok(())
    }

    /// Streams an audio file from a direct URL with hash verification and automatic ingestion
    pub async fn download_audio(
        &self,
        url: &str,
        expected_sha256: Option<&str>,
        custom_filename: Option<&str>,
        db: &Database,
    ) -> Result<IngestResult> {
        let task_id = format!("dl_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();

        // Determine destination file path
        let ext = if url.ends_with(".wav") { "wav" }
            else if url.ends_with(".flac") { "flac" }
            else if url.ends_with(".m4a") { "m4a" }
            else if url.ends_with(".ogg") { "ogg" }
            else { "mp3" };

        let filename = match custom_filename {
            Some(name) => format!("{}.{}", name.trim_end_matches(&['.', ' '][..]), ext),
            None => format!("{}.{}", task_id, ext),
        };

        let dest_path = self.cache_dir.join(&filename);
        let part_path = self.cache_dir.join(format!("{}.part", filename));

        let task = DownloadTask {
            id: task_id.clone(),
            url: url.to_string(),
            destination_path: dest_path.to_string_lossy().to_string(),
            expected_sha256: expected_sha256.map(|s| s.to_string()),
            status: "pending".to_string(),
            bytes_downloaded: 0,
            total_bytes: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        };

        db.create_download_task(&task)?;

        // Register cancellation token
        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut map = self.cancellations.lock().unwrap();
            map.insert(task_id.clone(), cancel_flag.clone());
        }

        info!("Starting download for task '{}' from URL '{}'", task_id, url);

        // Initiate HTTP request
        let resp = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("HTTP connection failed: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        };

        if !resp.status().is_success() {
            let err_msg = format!("HTTP error: status {}", resp.status());
            db.update_download_status(&task_id, "failed", Some(&err_msg))?;
            return Err(anyhow!(err_msg));
        }

        let total_bytes = resp.content_length().map(|len| len as i64);
        db.update_download_status(&task_id, "downloading", None)?;
        db.update_download_progress(&task_id, 0, total_bytes)?;

        // Stream body to part file while calculating SHA-256
        let mut file = match File::create(&part_path) {
            Ok(f) => f,
            Err(e) => {
                let err_msg = format!("Failed to create temporary file: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        };

        let mut hasher = Sha256::new();
        let mut downloaded: i64 = 0;
        let mut stream = resp.bytes_stream();
        let mut last_progress_report = downloaded;

        while let Some(chunk_res) = stream.next().await {
            // Check cancellation flag
            if cancel_flag.load(Ordering::SeqCst) {
                drop(file);
                let _ = fs::remove_file(&part_path);
                db.update_download_status(&task_id, "cancelled", Some("Download cancelled by user"))?;
                return Err(anyhow!("Download cancelled"));
            }

            let chunk = match chunk_res {
                Ok(c) => c,
                Err(e) => {
                    drop(file);
                    let _ = fs::remove_file(&part_path);
                    let err_msg = format!("Network streaming error: {}", e);
                    db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                    return Err(anyhow!(err_msg));
                }
            };

            if let Err(e) = file.write_all(&chunk) {
                drop(file);
                let _ = fs::remove_file(&part_path);
                let err_msg = format!("Disk write error: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }

            hasher.update(&chunk);
            downloaded += chunk.len() as i64;

            // Update database progress every ~256 KB
            if downloaded - last_progress_report >= 262144 {
                let _ = db.update_download_progress(&task_id, downloaded, total_bytes);
                last_progress_report = downloaded;
            }
        }

        let _ = file.flush();
        drop(file);

        // Finalize SHA-256 calculation
        let calculated_hash = format!("{:x}", hasher.finalize());
        db.update_download_status(&task_id, "verifying", None)?;

        // Verify hash if expected hash was provided
        if let Some(expected) = expected_sha256 {
            if !calculated_hash.eq_ignore_ascii_case(expected.trim()) {
                let _ = fs::remove_file(&part_path);
                let err_msg = format!("Hash mismatch: expected '{}', got '{}'", expected, calculated_hash);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        }

        // Rename part file to destination path
        if let Err(e) = fs::rename(&part_path, &dest_path) {
            let _ = fs::remove_file(&part_path);
            let err_msg = format!("Failed to move finished download: {}", e);
            db.update_download_status(&task_id, "failed", Some(&err_msg))?;
            return Err(anyhow!(err_msg));
        }

        // Automatically ingest and analyze downloaded asset into DHUN library and managed cache
        db.update_download_status(&task_id, "ingesting", None)?;
        let ingest_res = match ingest_audio_asset(&dest_path, "url_download", Some(url), true, db) {
            Ok(res) => res,
            Err(e) => {
                let err_msg = format!("Ingestion/analysis failed: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        };

        db.update_download_status(&task_id, "completed", None)?;
        db.update_download_progress(&task_id, downloaded, Some(downloaded))?;

        // Cleanup cancellation handle
        {
            let mut map = self.cancellations.lock().unwrap();
            map.remove(&task_id);
        }

        info!("Successfully completed download task '{}' (Track: '{}', Hash: {})", task_id, ingest_res.title, calculated_hash);
        Ok(ingest_res)
    }
}
