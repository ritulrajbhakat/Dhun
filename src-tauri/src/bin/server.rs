use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use dhun_lib::db::Database;
use dhun_lib::metadata_service::{create_global_catalogue_router, GlobalCatalogueState};

/// DHUN Global Shared Song Metadata Catalogue Service (`dhun-server`)
///
/// Architectural Boundary:
/// - Stores ONLY reusable song identity and musical analysis metadata.
/// - Accessible ONLY by the DHUN Automated DJ Engine via HMAC-SHA256 service auth.
/// - Does NOT host party rooms, playlists, guest requests, party sessions, or audio files.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("=================================================================");
    info!("DHUN धुन — Global Shared Song Metadata Catalogue Service");
    info!("Engine-Only Metadata Intelligence (Zero Party / Zero Audio Files)");
    info!("=================================================================");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let db_path_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "dhun_global_catalogue.db".to_string());
    let db_path = PathBuf::from(db_path_str);

    // Strictly isolated schema containing ONLY global song metadata tables (no party/guest tables)
    info!("Connecting Global Metadata Catalogue SQLite DB at: {:?}", db_path);
    let db = Database::new_global_catalogue_only(&db_path)?;

    let service_api_key = std::env::var("DHUN_CATALOGUE_SERVICE_KEY")
        .or_else(|_| std::env::var("DHUN_API_SECRET"))
        .unwrap_or_default();

    if service_api_key.trim().is_empty() {
        warn!("DHUN_CATALOGUE_SERVICE_KEY is not set! All authenticated catalogue endpoints will return 503 until configured.");
    }

    let state = Arc::new(GlobalCatalogueState::new(db, service_api_key));

    // Notice: NO permissive browser CORS layer is mounted here because browser clients
    // must NEVER directly browse, search, or edit the global metadata catalogue.
    let app = create_global_catalogue_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Global Metadata Catalogue Service listening on 0.0.0.0:{}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Global Metadata Catalogue Service stopped gracefully.");
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
