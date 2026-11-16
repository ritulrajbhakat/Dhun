use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tracing::{info, warn};

use crate::db::Database;
use crate::db::models::CacheStats;

pub const DEFAULT_MAX_CACHE_BYTES: i64 = 5 * 1024 * 1024 * 1024; // 5 GB

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

    /// Runs rolling LRU cache eviction when cache exceeds max_cache_bytes.
    /// Strictly guarantees NO active or queued playback asset is ever deleted.
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
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to delete pruned cache file {:?}: {}", path, e);
                } else {
                    info!("Evicted cached file from disk: {:?}", path);
                }
            }
        }

        info!("Rolling cache cleanup complete: pruned {} unlocked files", count);
        Ok(count)
    }

    /// Forcefully purges all unlocked cached files (useful on user request or disk low warning)
    pub fn purge_unlocked(&self, db: &Database) -> Result<usize> {
        // Run prune with 0 max_bytes to evict all unlocked entries
        let candidates = db.prune_unlocked_cache_entries(0)?;
        let count = candidates.len();

        for (_asset_id, path_str) in candidates {
            let path = Path::new(&path_str);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }

        info!("Purged {} unlocked files from cache", count);
        Ok(count)
    }

    pub fn get_stats(&self, db: &Database) -> Result<CacheStats> {
        Ok(db.get_cache_stats(self.max_cache_bytes)?)
    }
}
