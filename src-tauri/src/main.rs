#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_imports, unused_variables)]

mod analysis;
mod audio;
mod commands;
mod db;
mod dj;
mod library;
mod network;
mod recovery;
mod sync;

use std::sync::{Arc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::audio::AudioEngine;
use crate::commands::AppContext;
use crate::db::Database;
use crate::library::cache::{RollingCacheManager, DEFAULT_MAX_CACHE_BYTES};
use crate::library::downloader::AudioDownloader;
use crate::sync::MetadataSyncManager;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Initializing DHUN — Standalone Local-first DJ Party System");

    // Initialize or migrate SQLite database outside of src-tauri
    // so SQLite WAL writes do not trigger Tauri's file watcher recompile loop
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root_dir = if current_dir.ends_with("src-tauri") {
        current_dir.parent().unwrap_or(&current_dir).to_path_buf()
    } else {
        current_dir
    };
    let db_path = root_dir.join("dhun.db");
    info!("SQLite database path: {:?}", db_path);
    let db = Database::new(&db_path).expect("Failed to initialize DHUN SQLite database with migrations");

    // Initialize CPAL Dedicated Audio Engine with graceful recovery (never crash on launch)
    let audio = match AudioEngine::new(None) {
        Ok(engine) => Arc::new(Mutex::new(engine)),
        Err(e) => {
            tracing::warn!("Could not immediately open audio hardware stream ({}). Starting in on-demand mode.", e);
            Arc::new(Mutex::new(AudioEngine::new_dummy()))
        }
    };

    let cache_dir = root_dir.join("cache");
    let cache = Arc::new(RollingCacheManager::new(&cache_dir, DEFAULT_MAX_CACHE_BYTES));
    let _ = cache.reconcile_locks(&db);

    let downloader = Arc::new(AudioDownloader::new(&cache_dir));
    let sync = Arc::new(MetadataSyncManager::new_from_db(&db));
    let _ = sync.recover_in_flight_items(&db);
    sync.clone().start_background_worker(db.clone(), 30);

    let context = AppContext {
        db: db.clone(),
        audio,
        scheduler: Arc::new(Mutex::new(None)),
        server: Arc::new(Mutex::new(None)),
        cache,
        downloader,
        sync,
    };

    tauri::Builder::default()
        .manage(context)
        .invoke_handler(tauri::generate_handler![
            commands::start_party,
            commands::get_party_status,
            commands::get_telemetry,
            commands::emergency_fade,
            commands::deck_play,
            commands::deck_pause,
            commands::deck_seek,
            commands::deck_set_tempo,
            commands::deck_set_gain,
            commands::deck_set_eq,
            commands::deck_set_filter,
            commands::set_crossfader,
            commands::set_master_volume,
            commands::get_mood_profiles,
            commands::get_audio_devices,
            commands::get_tracks,
            commands::scan_library_folder,
            commands::get_guest_requests,
            commands::update_guest_request_status,
            commands::force_next_request,
            commands::set_dj_level,
            commands::set_party_mood,
            commands::download_track,
            commands::cancel_download,
            commands::get_download_tasks,
            commands::get_cache_stats,
            commands::purge_cache,
            commands::verify_audio_asset,
            commands::sync_metadata_correction,
            commands::trigger_sync,
            commands::get_sync_status,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running DHUN application");
}

