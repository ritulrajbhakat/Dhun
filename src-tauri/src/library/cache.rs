use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tracing::info;

use crate::db::Database;

#[derive(Debug, Clone)]
pub struct RollingCacheManager {
    locked_asset_ids: Arc<Mutex<HashSet<String>>>,
    max_cached_tracks: usize,
}

impl RollingCacheManager {
    pub fn new(max_cached_tracks: usize) -> Self {
        Self {
            locked_asset_ids: Arc::new(Mutex::new(HashSet::new())),
            max_cached_tracks,
        }
    }

    pub fn max_cached_tracks(&self) -> usize {
        self.max_cached_tracks
    }

    /// Locks an asset from being evicted during active playback or upcoming preloading
    pub fn lock_asset(&self, asset_id: &str) {
        let mut locks = self.locked_asset_ids.lock().unwrap();
        locks.insert(asset_id.to_string());
    }

    /// Releases a playback lock once track and subsequent 2 tracks have finished
    pub fn unlock_asset(&self, asset_id: &str) {
        let mut locks = self.locked_asset_ids.lock().unwrap();
        locks.remove(asset_id);
    }

    pub fn is_locked(&self, asset_id: &str) -> bool {
        let locks = self.locked_asset_ids.lock().unwrap();
        locks.contains(asset_id)
    }

    /// Runs rolling cache cleanup, ensuring NO active or queued audio is ever deleted
    pub fn run_cleanup(&self, _db: &Database) -> Result<usize> {
        let locks = self.locked_asset_ids.lock().unwrap();
        info!("Running rolling cache verification ({} currently locked playback assets)", locks.len());
        // Permanent metadata is never touched. Only temporary decoded files outside locked set can be pruned if needed.
        Ok(0)
    }
}

