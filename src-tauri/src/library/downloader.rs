use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::redirect::Policy;
use reqwest::Url;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::DownloadTask;
use crate::library::ingest::{ingest_audio_asset, IngestResult};

pub const DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES: i64 = 500 * 1024 * 1024; // 500 MB max audio file size

/// Validates that an IP address is a safe, publicly routable address (anti-SSRF defense)
pub fn is_restricted_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
                || (ipv4.octets()[0] == 0) // RFC 1122 "this network" (0.0.0.0/8)
                // Carrier-grade NAT (100.64.0.0/10)
                || (ipv4.octets()[0] == 100 && (ipv4.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(ipv6) => {
            if ipv6.is_loopback() || ipv6.is_unspecified() {
                return true;
            }
            // Check IPv4-mapped IPv6 addresses (::ffff:127.0.0.1, etc.)
            let segs = ipv6.segments();
            if segs[0] == 0 && segs[1] == 0 && segs[2] == 0 && segs[3] == 0 && segs[4] == 0 && segs[5] == 0xffff {
                let v4 = std::net::Ipv4Addr::new(
                    (segs[6] >> 8) as u8,
                    (segs[6] & 0xff) as u8,
                    (segs[7] >> 8) as u8,
                    (segs[7] & 0xff) as u8,
                );
                return is_restricted_ip(&IpAddr::V4(v4));
            }
            // Unique local address (fc00::/7)
            ((ipv6.segments()[0] & 0xfe00) == 0xfc00)
                // Link-local unicast (fe80::/10)
                || ((ipv6.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// Strictly validates URL scheme (HTTPS only) and verifies that host does not resolve
/// to loopback, private, or restricted IP addresses (SSRF mitigation), returning the parsed URL
/// and the validated SocketAddr to pin for connection.
pub fn validate_safe_public_url(url_str: &str) -> Result<(Url, std::net::SocketAddr)> {
    let parsed = Url::parse(url_str).map_err(|e| anyhow!("Malformed URL: {}", e))?;

    // 1. Strictly enforce HTTPS protocol
    if parsed.scheme() != "https" {
        return Err(anyhow!(
            "Security violation: Only secure HTTPS endpoints are permitted for downloads (got '{}')",
            parsed.scheme()
        ));
    }

    // 2. Validate host name
    let host = parsed.host_str().ok_or_else(|| anyhow!("URL is missing a valid host"))?;
    let lower_host = host.to_lowercase();
    if lower_host == "localhost" || lower_host.ends_with(".localhost") || lower_host.ends_with(".local") {
        return Err(anyhow!("Security violation: Localhost and local domain targets are blocked: {}", host));
    }

    // 3. Resolve host and block private, loopback, link-local, carrier NAT IPs (fail closed)
    let port = parsed.port_or_known_default().unwrap_or(443);
    let socket_addrs: Vec<std::net::SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| anyhow!("Failed to resolve hostname '{}': {}", host, e))?
        .collect();

    if socket_addrs.is_empty() {
        return Err(anyhow!("Security violation: Host '{}' has no resolvable network addresses", host));
    }

    let mut first_valid = None;
    for addr in &socket_addrs {
        let ip = addr.ip();
        if is_restricted_ip(&ip) {
            return Err(anyhow!(
                "Security violation: Host '{}' resolves to private or restricted network address ({})",
                host,
                ip
            ));
        }
        if first_valid.is_none() {
            first_valid = Some(*addr);
        }
    }

    let pinned = first_valid.ok_or_else(|| anyhow!("No valid address found for host '{}'", host))?;
    Ok((parsed, pinned))
}

/// Sanitizes custom filenames to prevent path traversal and Windows reserved filename issues
pub fn sanitize_custom_filename(input: &str, default_fallback: &str) -> String {
    // 1. Strip any directory separators to prevent traversal
    let path = std::path::Path::new(input);
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or(input);

    // 2. Separate stem and extension
    let (stem, ext) = match file_name.rfind('.') {
        Some(idx) => (&file_name[..idx], Some(&file_name[idx..])),
        None => (file_name, None),
    };

    // Filter characters in stem
    let clean_stem: String = stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .collect();
    let trimmed_stem = clean_stem.trim();

    let clean_ext: String = match ext {
        Some(e) => e.chars().filter(|c| c.is_alphanumeric() || *c == '.').collect(),
        None => String::new(),
    };

    if trimmed_stem.is_empty() {
        return default_fallback.to_string();
    }

    let upper_stem = trimmed_stem.to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let safe_stem = if reserved.contains(&upper_stem.as_str()) {
        format!("safe_{}", trimmed_stem)
    } else {
        trimmed_stem.to_string()
    };

    format!("{}{}", safe_stem, clean_ext)
}

/// Probes a downloaded file using Symphonia to verify it contains a real, decodable audio stream
/// and detects its true canonical format (mp3, wav, flac, m4a, ogg)
pub fn probe_and_verify_audio_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let hint = symphonia::core::probe::Hint::new();
    let format_opts = symphonia::core::formats::FormatOptions::default();
    let meta_opts = symphonia::core::meta::MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &meta_opts)
        .map_err(|e| anyhow!("Invalid media format: unable to probe audio container ({})", e))?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("Invalid media format: file contains no decodable audio stream"))?;

    let ext = match track.codec_params.codec {
        symphonia::core::codecs::CODEC_TYPE_MP3 => "mp3",
        symphonia::core::codecs::CODEC_TYPE_FLAC => "flac",
        symphonia::core::codecs::CODEC_TYPE_AAC => "m4a",
        symphonia::core::codecs::CODEC_TYPE_PCM_S16LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S24LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S32LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_F32LE => "wav",
        symphonia::core::codecs::CODEC_TYPE_VORBIS => "ogg",
        _ => "mp3",
    };

    Ok(ext.to_string())
}

/// RAII cleanup guard to guarantee temporary file removal and cancellation map cleanup on all exits
struct DownloadCleanupGuard {
    part_path: PathBuf,
    dest_path: Option<PathBuf>,
    task_id: String,
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    armed: bool,
}

impl Drop for DownloadCleanupGuard {
    fn drop(&mut self) {
        // Guarantee removal of cancellation handle
        if let Ok(mut map) = self.cancellations.lock() {
            map.remove(&self.task_id);
        }
        // If armed (i.e. task failed or cancelled), remove partial / destination files
        if self.armed {
            if self.part_path.exists() {
                let _ = fs::remove_file(&self.part_path);
            }
            if let Some(dest) = &self.dest_path {
                if dest.exists() {
                    let _ = fs::remove_file(dest);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct AudioDownloader {
    cache_dir: PathBuf,
    client: reqwest::Client,
    max_file_size_bytes: i64,
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl AudioDownloader {
    pub fn new<P: Into<PathBuf>>(cache_dir: P) -> Self {
        let dir = cache_dir.into();
        let _ = fs::create_dir_all(&dir);

        // Configure client with custom redirect policy that validates every redirect target (fail closed)
        let redirect_policy = Policy::custom(|attempt| {
            if attempt.previous().len() >= 3 {
                return attempt.stop();
            }
            let scheme = attempt.url().scheme().to_string();
            if scheme != "https" {
                return attempt.error("Security violation: Redirect to insecure non-HTTPS scheme blocked");
            }
            let host_opt = attempt.url().host_str().map(|h| h.to_string());
            let port = attempt.url().port_or_known_default().unwrap_or(443);

            let Some(host) = host_opt else {
                return attempt.error("Security violation: Redirect URL missing host");
            };
            let lower = host.to_lowercase();
            if lower == "localhost" || lower.ends_with(".localhost") || lower.ends_with(".local") {
                return attempt.error("Security violation: Redirect to localhost/local domain blocked");
            }
            match (host.as_str(), port).to_socket_addrs() {
                Ok(addrs) => {
                    let mut has_addr = false;
                    for addr in addrs {
                        has_addr = true;
                        if is_restricted_ip(&addr.ip()) {
                            return attempt.error("Security violation: Redirect to private or restricted network address blocked");
                        }
                    }
                    if !has_addr {
                        return attempt.error("Security violation: Redirect target has no resolvable IP addresses");
                    }
                }
                Err(e) => {
                    return attempt.error(format!("Security violation: Failed to resolve redirect host '{}': {}", host, e));
                }
            }
            attempt.follow()
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .redirect(redirect_policy)
            .build()
            .unwrap_or_default();

        Self {
            cache_dir: dir,
            client,
            max_file_size_bytes: DEFAULT_MAX_AUDIO_DOWNLOAD_BYTES,
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_max_file_size(mut self, max_bytes: i64) -> Self {
        self.max_file_size_bytes = max_bytes;
        self
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn max_file_size_bytes(&self) -> i64 {
        self.max_file_size_bytes
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

    /// Streams an audio file from an authorized direct URL with security validation,
    /// SSRF prevention, format validation, SHA-256 duplicate detection, and atomic ingestion
    pub async fn download_audio(
        &self,
        url_str: &str,
        expected_sha256: Option<&str>,
        custom_filename: Option<&str>,
        db: &Database,
    ) -> Result<IngestResult> {
        let task_id = format!("dl_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
        let now = chrono::Utc::now().timestamp_millis();

        // 1. Validate URL security (strict HTTPS, anti-SSRF host/IP resolution, address pinning)
        let (validated_url, pinned_addr) = match validate_safe_public_url(url_str) {
            Ok(u) => u,
            Err(e) => {
                let err_msg = format!("URL security validation failed: {}", e);
                // Record failed task in database for audit trail
                let failed_task = DownloadTask {
                    id: task_id.clone(),
                    url: url_str.to_string(),
                    destination_path: String::new(),
                    expected_sha256: expected_sha256.map(|s| s.to_string()),
                    status: "failed".to_string(),
                    bytes_downloaded: 0,
                    total_bytes: None,
                    error_message: Some(err_msg.clone()),
                    created_at: now,
                    updated_at: now,
                };
                let _ = db.create_download_task(&failed_task);
                return Err(anyhow!(err_msg));
            }
        };

        // 2. Compute safe filenames
        let safe_base_name = match custom_filename {
            Some(name) => sanitize_custom_filename(name, &task_id),
            None => task_id.clone(),
        };

        let part_filename = format!("{}.part", task_id);
        let part_path = self.cache_dir.join(&part_filename);

        let task = DownloadTask {
            id: task_id.clone(),
            url: validated_url.to_string(),
            destination_path: part_path.to_string_lossy().to_string(),
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

        // Initialize RAII cleanup guard
        let mut cleanup_guard = DownloadCleanupGuard {
            part_path: part_path.clone(),
            dest_path: None,
            task_id: task_id.clone(),
            cancellations: self.cancellations.clone(),
            armed: true,
        };

        info!("Starting download for task '{}' from URL '{}' (pinned: {})", task_id, validated_url, pinned_addr);

        // 3. Initiate HTTP GET request over DNS-pinned connection to defeat DNS-rebinding attacks
        let host = validated_url.host_str().unwrap_or("");
        let download_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .resolve(host, pinned_addr)
            .build()
            .unwrap_or_else(|_| self.client.clone());

        let resp = match download_client.get(validated_url.clone()).send().await {
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

        // 4. Validate Content-Type header if present
        if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
            let lower_ct = ct.to_lowercase();
            if lower_ct.starts_with("text/html")
                || lower_ct.starts_with("application/json")
                || lower_ct.starts_with("text/plain")
                || lower_ct.starts_with("application/xml")
            {
                let err_msg = format!("Invalid audio format: server returned non-audio Content-Type '{}'", ct);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        }

        // 5. Enforce file size limit via Content-Length header
        if let Some(content_length) = resp.content_length() {
            if content_length as i64 > self.max_file_size_bytes {
                let err_msg = format!(
                    "Remote file size ({} bytes) exceeds maximum allowable limit of {} bytes",
                    content_length, self.max_file_size_bytes
                );
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        }

        let total_bytes = resp.content_length().map(|len| len as i64);
        db.update_download_status(&task_id, "downloading", None)?;
        db.update_download_progress(&task_id, 0, total_bytes)?;

        // 6. Stream chunks to temporary .part file while computing running SHA-256
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
                db.update_download_status(&task_id, "cancelled", Some("Download cancelled by user"))?;
                return Err(anyhow!("Download cancelled"));
            }

            let chunk = match chunk_res {
                Ok(c) => c,
                Err(e) => {
                    drop(file);
                    let err_msg = format!("Network streaming error: {}", e);
                    db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                    return Err(anyhow!(err_msg));
                }
            };

            // Enforce max size limit during streaming chunks
            if downloaded + chunk.len() as i64 > self.max_file_size_bytes {
                drop(file);
                let err_msg = format!(
                    "Download exceeded maximum allowable file size ({} bytes)",
                    self.max_file_size_bytes
                );
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }

            if let Err(e) = file.write_all(&chunk) {
                drop(file);
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

        // 7. Finalize SHA-256 calculation
        let calculated_hash = format!("{:x}", hasher.finalize());
        db.update_download_status(&task_id, "verifying", None)?;

        // Verify cryptographic hash against expected hash if provided
        if let Some(expected) = expected_sha256 {
            if !calculated_hash.eq_ignore_ascii_case(expected.trim()) {
                let err_msg = format!("Hash mismatch: expected '{}', got '{}'", expected, calculated_hash);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        }

        // 8. Probe and verify that the downloaded payload is a genuine, decodable audio file
        let detected_ext = match probe_and_verify_audio_file(&part_path) {
            Ok(ext) => ext,
            Err(e) => {
                let err_msg = format!("Media validation failed: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        };

        // Construct final destination path using the verified codec extension
        let final_filename = format!("{}.{}", safe_base_name, detected_ext);
        let dest_path = self.cache_dir.join(&final_filename);
        cleanup_guard.dest_path = Some(dest_path.clone());

        // Atomically rename .part file to final cache destination
        if let Err(e) = fs::rename(&part_path, &dest_path) {
            let err_msg = format!("Failed to move finished download to destination path: {}", e);
            db.update_download_status(&task_id, "failed", Some(&err_msg))?;
            return Err(anyhow!(err_msg));
        }

        // 9. Automatically ingest and analyze into DHUN library and managed cache
        db.update_download_status(&task_id, "ingesting", None)?;
        let ingest_res = match ingest_audio_asset(&dest_path, "url_download", Some(validated_url.as_str()), true, db) {
            Ok(res) => res,
            Err(e) => {
                let err_msg = format!("Ingestion/analysis failed: {}", e);
                db.update_download_status(&task_id, "failed", Some(&err_msg))?;
                return Err(anyhow!(err_msg));
            }
        };

        // If duplicate detected, clean up redundant download from disk
        if ingest_res.is_duplicate {
            let _ = fs::remove_file(&dest_path);
            info!("Duplicate file removed from disk; reused existing asset '{}'", ingest_res.asset_id);
        }

        // Disarm cleanup guard so successful files are kept
        cleanup_guard.armed = false;

        db.update_download_status(&task_id, "completed", None)?;
        db.update_download_progress(&task_id, downloaded, Some(downloaded))?;

        info!(
            "Successfully completed download task '{}' (Track: '{}', Hash: {}, Duplicate: {})",
            task_id, ingest_res.title, calculated_hash, ingest_res.is_duplicate
        );
        Ok(ingest_res)
    }
}
