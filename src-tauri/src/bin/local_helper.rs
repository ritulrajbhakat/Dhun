use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use dhun_lib::db::Database;
use dhun_lib::network::local_helper_routes::{
    create_local_helper_router, LocalHelperState, DEFAULT_LOCAL_HELPER_PORT,
};
use dhun_lib::sync::MetadataSyncManager;

/// DHUN Local Helper Service (`dhun-local-helper`)
///
/// Runs on the host laptop while the party is active:
/// - Hosts the private local party room over host loopback + same-LAN Wi-Fi ONLY.
/// - Generates real LAN QR codes with room_id + cryptographic room_access_token.
/// - Manages temporary local audio cache (10-15 tracks max) with post-party & expiry cleanup.
/// - Runs local song analysis & non-interrupting automated DJ candidate evaluation.
/// - Syncs ONLY reusable song metadata (never audio or party state) to the Global Metadata Catalogue.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("=================================================================");
    info!("DHUN धुन — Local Party Helper Service");
    info!("Private LAN Party Room • Local Temp Cache • Automated DJ Engine");
    info!("=================================================================");

    let port: u16 = std::env::var("DHUN_LOCAL_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_LOCAL_HELPER_PORT);

    let db_path = PathBuf::from(
        std::env::var("DHUN_LOCAL_DB_PATH").unwrap_or_else(|_| "dhun_local_helper.db".to_string()),
    );
    let cache_dir = PathBuf::from(
        std::env::var("DHUN_CACHE_DIR").unwrap_or_else(|_| "dhun_temp_cache".to_string()),
    );

    info!("Local SQLite DB: {:?}", db_path);
    info!("Temporary Audio Cache Dir: {:?}", cache_dir);

    let db = Database::new(&db_path)?;

    // Configure engine-only Global Shared Song Metadata Catalogue sync client (optional — works 100% offline)
    let catalogue_url = std::env::var("DHUN_GLOBAL_CATALOGUE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let catalogue_key = std::env::var("DHUN_CATALOGUE_SERVICE_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let sync_manager = match MetadataSyncManager::new(catalogue_url.clone(), catalogue_key) {
        Ok(mgr) => {
            if catalogue_url.is_some() {
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

    let state = Arc::new(LocalHelperState::new(
        db.clone(),
        port,
        cache_dir,
        sync_manager,
        None,
        allowed_origins,
    ));

    // Launch-time cache maintenance: reconcile stale locks and purge expired temp audio files
    let _ = state.cache_manager.reconcile_locks(&db);
    let purged_expired = state.cache_manager.cleanup_expired_on_launch(&db).unwrap_or(0);
    if purged_expired > 0 {
        info!("Cleaned up {} expired temporary cache file(s) on launch.", purged_expired);
    }

    info!("Host Loopback API:  http://127.0.0.1:{}", port);
    match &state.detected_lan_ip {
        Some(lan) => info!("LAN Guest Endpoint: http://{}:{} (same Wi-Fi/LAN only)", lan, port),
        None => warn!("No reachable RFC1918 LAN IP detected. Connect to Wi-Fi/LAN for guest smartphone access."),
    }

    let app = create_local_helper_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("DHUN Local Helper bound on 0.0.0.0:{}", port);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

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
