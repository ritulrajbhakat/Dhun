use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use dhun_lib::db::Database;
use dhun_lib::library::resolver::{validate_loopback_resolver_url, AuthorizedResolverConfig};
use dhun_lib::library::resolver_process::ResolverSupervisor;
use dhun_lib::network::local_helper_routes::{
    create_local_helper_router, start_intake_queue_worker, LocalHelperState,
    DEFAULT_LOCAL_HELPER_PORT,
};
use dhun_lib::sync::MetadataSyncManager;

/// Resolves safe OS-native storage directory for the local helper database and temporary cache:
/// - Windows: `%LOCALAPPDATA%\DHUN\helper`
/// - macOS:   `$HOME/Library/Application Support/DHUN/helper`
/// - Linux:   `$XDG_DATA_HOME/dhun/helper` or `$HOME/.local/share/dhun/helper`
fn resolve_default_helper_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("DHUN").join("helper");
            if fs::create_dir_all(&p).is_ok() {
                return p;
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("DHUN")
                .join("helper");
            if fs::create_dir_all(&p).is_ok() {
                return p;
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.trim().is_empty() {
                let p = PathBuf::from(xdg).join("dhun").join("helper");
                if fs::create_dir_all(&p).is_ok() {
                    return p;
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".local").join("share").join("dhun").join("helper");
            if fs::create_dir_all(&p).is_ok() {
                return p;
            }
        }
    }
    let fallback = PathBuf::from(".").join("dhun_helper_data");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// DHUN Local Helper Service (`dhun-local-helper`)
///
/// Runs on the host laptop while the party is active:
/// - Hosts the private local party room over host loopback + same-LAN Wi-Fi ONLY.
/// - Generates real LAN QR codes with room_id + cryptographic room_access_token.
/// - Streams host-authenticated locally cached authorized audio to the 4-deck Web Audio engine.
/// - Manages temporary local audio cache (10-15 tracks max) with post-party & 60m expiry cleanup.
/// - Runs local song analysis & non-interrupting automated DJ candidate evaluation.
/// - Syncs ONLY reusable song metadata (never audio or party state) to the Global Metadata Catalogue.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("=================================================================");
    info!("DHUN धुन — Local Party Helper Service (v2.1.0)");
    info!("Private LAN Party Room • Local Temp Cache • Automated DJ Engine");
    info!("=================================================================");

    let port: u16 = std::env::var("DHUN_LOCAL_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_LOCAL_HELPER_PORT);

    let default_data_dir = resolve_default_helper_data_dir();
    let _ = fs::create_dir_all(&default_data_dir);

    let db_path = std::env::var("DHUN_LOCAL_DB_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_data_dir.join("dhun_local_helper.db"));

    let cache_dir = std::env::var("DHUN_CACHE_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_data_dir.join("temp_cache"));

    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::create_dir_all(&cache_dir);

    info!("Local SQLite DB:          {:?}", db_path);
    info!("Temporary Cache Dir:      {:?}", cache_dir);

    let db = Database::new(&db_path)?;

    // Engine-only Global Shared Song Metadata Catalogue sync client (optional — works 100% offline)
    let catalogue_url = std::env::var("DHUN_GLOBAL_CATALOGUE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let catalogue_key = std::env::var("DHUN_CATALOGUE_SERVICE_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let fb_project = std::env::var("DHUN_FIREBASE_PROJECT_ID")
        .ok()
        .or_else(|| std::env::var("VITE_FIREBASE_PROJECT_ID").ok())
        .or_else(|| db.get_app_setting("firebase_project_id").unwrap_or(None));
    let fb_key = std::env::var("DHUN_FIREBASE_API_KEY")
        .ok()
        .or_else(|| std::env::var("VITE_FIREBASE_API_KEY").ok())
        .or_else(|| db.get_app_setting("firebase_api_key").unwrap_or(None));

    let firestore_config = fb_project.filter(|p| !p.trim().is_empty()).map(|project_id| {
        dhun_lib::sync::FirestoreConfig {
            project_id,
            api_key: fb_key.filter(|k| !k.trim().is_empty()),
        }
    });

    let sync_manager = match MetadataSyncManager::new_with_firestore(catalogue_url.clone(), catalogue_key, firestore_config.clone()) {
        Ok(mgr) => {
            if let Some(ref fc) = firestore_config {
                info!("Cloud Firestore Global Shared Catalogue configured for project '{}' (zero-audio metadata sync).", fc.project_id);
            } else if catalogue_url.is_some() {
                info!("Global Metadata Catalogue sync configured (metadata-only, HMAC-SHA256 signed).");
            } else {
                info!("Running in offline-first mode (no remote Global Metadata Catalogue configured; sync outbox will queue locally).");
            }
            mgr
        }
        Err(e) => {
            warn!("Invalid Global Metadata Catalogue config ({}). Continuing offline-only.", e);
            MetadataSyncManager::new(None, None)?
        }
    };

    // Recover any interrupted outbox items & launch background metadata-only sync retry worker
    let _ = sync_manager.recover_in_flight_items(&db);
    let _sync_worker = Arc::new(sync_manager.clone()).start_background_worker(db.clone(), 30);

    // Allowed public frontend origins (e.g. Vercel deployment)
    let allowed_origins: Vec<String> = std::env::var("DHUN_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Optional host-configured media resolver (public HTTPS, or local 127.0.0.1 when DHUN_ALLOW_LOOPBACK_RESOLVER=1)
    let resolver_url = std::env::var("DHUN_AUDIO_RESOLVER_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let resolver_key = std::env::var("DHUN_AUDIO_RESOLVER_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let allow_loopback_resolver = std::env::var("DHUN_ALLOW_LOOPBACK_RESOLVER")
        .ok()
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    // Automatically supervise and start the Python audio resolver service if not explicitly disabled
    let auto_resolver_disabled = std::env::var("DHUN_DISABLE_AUTO_RESOLVER")
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    let resolver_supervisor = if !auto_resolver_disabled && resolver_url.is_none() {
        match ResolverSupervisor::start_default(&default_data_dir).await {
            Ok(sup) => {
                info!("Auto-started/verified Audio Resolver: {}", sup.endpoint_url());
                Some(Arc::new(sup))
            }
            Err(e) => {
                warn!("Audio Resolver auto-startup notice: {}. Continuing in offline/metadata mode.", e);
                None
            }
        }
    } else {
        None
    };

    // If resolver_url was not explicitly configured in environment, use the auto-supervised resolver
    let auto_endpoint = resolver_supervisor.as_ref().map(|s| s.endpoint_url());
    let effective_resolver_url = resolver_url.or(auto_endpoint);
    let effective_allow_loopback = allow_loopback_resolver || resolver_supervisor.is_some();

    let audio_resolver = match effective_resolver_url {
        Some(url) => {
            let is_loopback = validate_loopback_resolver_url(&url).is_ok();
            match AuthorizedResolverConfig::new_with_loopback(
                url.clone(),
                resolver_key,
                effective_allow_loopback || is_loopback,
            ) {
                Ok(cfg) => {
                    info!(
                        "Authorized Audio Resolver configured: {} (loopback={})",
                        cfg.endpoint_url, cfg.allow_loopback
                    );
                    Some(cfg)
                }
                Err(e) => {
                    warn!("Ignored invalid audio resolver URL '{}': {}", url, e);
                    None
                }
            }
        }
        None => None,
    };

    let state = Arc::new(
        LocalHelperState::new(
            db.clone(),
            port,
            cache_dir,
            sync_manager,
            None,
            allowed_origins,
        )
        .with_audio_resolver(audio_resolver),
    );

    // Launch-time cache maintenance: reconcile stale locks and purge expired temp audio files
    let _ = state.cache_manager.reconcile_locks(&db);
    let purged_expired = state.cache_manager.cleanup_expired_on_launch(&db).unwrap_or(0);
    if purged_expired > 0 {
        info!("Cleaned up {} expired temporary cache file(s) on launch.", purged_expired);
    }

    // Start persistent asynchronous submission intake worker (recovers interrupted jobs automatically)
    let _intake_worker = start_intake_queue_worker(state.clone());

    info!("Host Loopback API:        http://127.0.0.1:{}", port);
    match state.current_lan_ip() {
        Some(lan) => info!("LAN Guest Endpoint:       http://{}:{} (same Wi-Fi/LAN only)", lan, port),
        None => warn!("No reachable RFC1918 LAN IP detected. Connect to Wi-Fi/LAN for guest smartphone access."),
    }

    let app = create_local_helper_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            info!(
                "DHUN Local Helper is already running on port {} (e.g. triggered via dhun://start). Exiting duplicate instance cleanly.",
                port
            );
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    info!("DHUN Local Helper bound on 0.0.0.0:{}", port);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    if let Some(ref sup) = resolver_supervisor {
        sup.shutdown();
    }

    info!("DHUN Local Helper stopped gracefully.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
