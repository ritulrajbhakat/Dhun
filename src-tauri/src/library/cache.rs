use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tracing::{info, warn};

use crate::db::Database;
use crate::db::models::{CacheStats, LocalTempCacheEntry, LocalTempCacheStats};

pub const DEFAULT_MAX_CACHE_BYTES: i64 = 5 * 1024 * 1024 * 1024; // 5 GB
/// Target maximum number of temporary local party audio tracks retained simultaneously (10-15 tracks).
pub const DEFAULT_MAX_TEMP_TRACKS: usize = 15;
/// Fallback expiry TTL for temporary party audio files (24 hours) in case the host crashes before explicit closure.
pub const DEFAULT_TEMP_CACHE_TTL_MS: i64 = 24 * 60 * 60 * 1000;

pub const CACHE_PRIORITY_EVICTABLE: i32 = 0;
pub const CACHE_PRIORITY_UPCOMING: i32 = 1;
pub const CACHE_PRIORITY_NEXT_DECK: i32 = 2;
pub const CACHE_PRIORITY_PLAYING: i32 = 3;

#[derive(Debug, Clone)]
pub struct RollingCacheManager {
    cache_dir: PathBuf,
    max_cache_bytes: i64,
    locked_asset_ids: Arc<Mutex<HashSet<String>>>,
}

impl RollingCacheManager {
    pub fn new<P: Into<PathBuf>>(cache_dir: P, max_cache_bytes: i64) -> Self {
        let dir = cache_dir.into();
        let _ = fs::create_dir_all(&dir);
        Self {
            cache_dir: dir,
            max_cache_bytes: if max_cache_bytes <= 0 { DEFAULT_MAX_CACHE_BYTES } else { max_cache_bytes },
            locked_asset_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn max_cache_bytes(&self) -> i64 {
        self.max_cache_bytes
    }

    /// Locks an asset from eviction during active playback or upcoming preloading
    pub fn lock_asset(&self, asset_id: &str, db: Option<&Database>) {
        let mut locks = self.locked_asset_ids.lock().unwrap();
        locks.insert(asset_id.to_string());
        if let Some(database) = db {
            let _ = database.set_asset_cache_lock(asset_id, true, Some("active_playback"));
        }
    }

    /// Releases playback lock once track and subsequent queued tracks have finished
    pub fn unlock_asset(&self, asset_id: &str, db: Option<&Database>) {
        let mut locks = self.locked_asset_ids.lock().unwrap();
        locks.remove(asset_id);
        if let Some(database) = db {
            let _ = database.set_asset_cache_lock(asset_id, false, None);
        }
    }

    pub fn is_locked(&self, asset_id: &str) -> bool {
        let locks = self.locked_asset_ids.lock().unwrap();
        locks.contains(asset_id)
    }

    pub fn locked_count(&self) -> usize {
        let locks = self.locked_asset_ids.lock().unwrap();
        locks.len()
    }

    /// Reconciles locks on application startup by clearing stale locks from SQLite
    pub fn reconcile_locks(&self, db: &Database) -> Result<()> {
        let mut locks = self.locked_asset_ids.lock().unwrap();
        locks.clear();
        db.clear_all_asset_cache_locks()?;
        info!("Rolling cache locks successfully reconciled on startup");
        Ok(())
    }

    /// Runs rolling LRU cache eviction when cache exceeds max_cache_bytes.
    /// Strictly guarantees NO active or queued playback asset is ever deleted,
    /// and NEVER deletes local music files stored outside the managed cache directory.
    pub fn run_cleanup(&self, db: &Database) -> Result<usize> {
        // Sync in-memory locks with database manifest before pruning
        {
            let locks = self.locked_asset_ids.lock().unwrap();
            for asset_id in locks.iter() {
                let _ = db.set_asset_cache_lock(asset_id, true, Some("memory_lock"));
            }
        }

        let candidates = db.prune_unlocked_cache_entries(self.max_cache_bytes)?;
        let count = candidates.len();

        for (_asset_id, path_str) in candidates {
            let path = Path::new(&path_str);
            if path.exists() {
                // Safeguard: Only delete files located strictly within cache_dir
                if path.starts_with(&self.cache_dir) {
                    if let Err(e) = fs::remove_file(path) {
                        warn!("Failed to delete pruned cache file {:?}: {}", path, e);
                    } else {
                        info!("Evicted cached file from disk: {:?}", path);
                    }
                } else {
                    info!("Preserving non-cache local file outside cache_dir: {:?}", path);
                }
            }
        }

        info!("Rolling cache cleanup complete: pruned {} unlocked files", count);
        Ok(count)
    }

    /// Forcefully purges all unlocked cached files (useful on user request or disk low warning).
    /// Preserves user files located outside cache_dir.
    pub fn purge_unlocked(&self, db: &Database) -> Result<usize> {
        // Run prune with 0 max_bytes to evict all unlocked entries
        let candidates = db.prune_unlocked_cache_entries(0)?;
        let count = candidates.len();

        for (_asset_id, path_str) in candidates {
            let path = Path::new(&path_str);
            if path.exists() && path.starts_with(&self.cache_dir) {
                let _ = fs::remove_file(path);
            }
        }

        info!("Purged {} unlocked files from cache", count);
        Ok(count)
    }

    pub fn get_stats(&self, db: &Database) -> Result<CacheStats> {
        Ok(db.get_cache_stats(self.max_cache_bytes)?)
    }

    // ========================================================================
    // Temporary Party Track Cache (Target 10-15 Tracks, Priority Lock, Expiry)
    // ========================================================================

    /// Registers a temporary authorized local audio file for an active party room
    /// and immediately enforces the 10-15 track cap by evicting lowest-priority unlocked files.
    pub fn register_temp_track(
        &self,
        room_id: &str,
        submission_id: Option<&str>,
        song_id: &str,
        file_path: &Path,
        priority_tier: i32,
        is_locked: bool,
        ttl_ms: Option<i64>,
        max_tracks: usize,
        db: &Database,
    ) -> Result<LocalTempCacheEntry> {
        let now = chrono::Utc::now().timestamp_millis();
        let size_bytes = fs::metadata(file_path).map(|m| m.len() as i64).unwrap_or(0);
        let id = format!("lcache_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);

        let entry = LocalTempCacheEntry {
            id,
            room_id: room_id.to_string(),
            submission_id: submission_id.map(|s| s.to_string()),
            song_id: song_id.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            size_bytes,
            priority_tier,
            is_locked,
            created_at: now,
            last_accessed_at: now,
            expires_at: now + ttl_ms.unwrap_or(DEFAULT_TEMP_CACHE_TTL_MS),
        };

        db.upsert_local_temp_cache(&entry)?;
        let _ = self.enforce_track_cap(max_tracks, db)?;
        Ok(entry)
    }

    /// Updates priority tier & lock state as a track moves between upcoming, next_deck, playing, and evictable.
    pub fn set_temp_track_priority(
        &self,
        cache_id: &str,
        priority_tier: i32,
        is_locked: bool,
        db: &Database,
    ) -> Result<()> {
        db.update_local_temp_cache_priority(cache_id, priority_tier, is_locked)?;
        Ok(())
    }

    /// Enforces the 10-15 track cap, prioritizing currently playing and upcoming tracks
    /// and removing oldest unlocked tracks strictly within cache_dir.
    pub fn enforce_track_cap(&self, max_tracks: usize, db: &Database) -> Result<usize> {
        let cap = max_tracks.clamp(1, DEFAULT_MAX_TEMP_TRACKS);
        let evicted = db.prune_local_temp_cache_over_cap(cap)?;
        let count = evicted.len();
        for (_id, path_str) in evicted {
            self.safe_delete_within_cache(&path_str);
        }
        if count > 0 {
            info!("Local temp cache cap ({} tracks) enforced: evicted {} unlocked tracks", cap, count);
        }
        Ok(count)
    }

    /// Deletes all temporary cached audio files belonging to a room when the host explicitly closes the party.
    pub fn purge_room_cache(&self, room_id: &str, db: &Database) -> Result<usize> {
        let entries = db.delete_local_temp_cache_for_room(room_id)?;
        let count = entries.len();
        for (_id, path_str) in entries {
            self.safe_delete_within_cache(&path_str);
        }
        info!("Explicit party closure purged {} temporary audio files for room {}", count, room_id);
        Ok(count)
    }

    /// Launch-time sweep: checks and deletes any expired temporary audio files left behind from prior runs.
    pub fn cleanup_expired_on_launch(&self, db: &Database) -> Result<usize> {
        let now = chrono::Utc::now().timestamp_millis();
        let expired = db.delete_expired_local_temp_cache(now)?;
        let count = expired.len();
        for (_id, path_str) in expired {
            self.safe_delete_within_cache(&path_str);
        }
        if count > 0 {
            info!("Startup expiry cleanup deleted {} expired temporary cache files", count);
        }
        Ok(count)
    }

    pub fn get_temp_cache_stats(&self, max_tracks: usize, db: &Database) -> Result<LocalTempCacheStats> {
        Ok(db.get_local_temp_cache_stats(max_tracks)?)
    }

    fn safe_delete_within_cache(&self, path_str: &str) {
        let path = Path::new(path_str);
        if path.exists() && path.starts_with(&self.cache_dir) {
            if let Err(e) = fs::remove_file(path) {
                warn!("Failed deleting temporary cache file {:?}: {}", path, e);
            }
        }
    }
}
