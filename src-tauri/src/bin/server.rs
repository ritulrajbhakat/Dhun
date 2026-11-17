use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::DefaultBodyLimit;
use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderName},
    Method,
};
use tower_http::cors::CorsLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use dhun_lib::db::Database;
use dhun_lib::network::cloud_routes::{create_cloud_router, CloudServerState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("==================================================");
    info!("DHUN धुन — Cloud Party & Audio Control Server");
    info!("Phase 1: Web Application & Real Cloud Foundation");
    info!("==================================================");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let db_path_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "dhun.db".to_string());
    let db_path = PathBuf::from(db_path_str);
    info!("Connecting SQLite database at: {:?}", db_path);
    let db = Database::new(&db_path)?;

    let public_guest_url = std::env::var("PUBLIC_GUEST_URL")
        .unwrap_or_else(|_| "https://ritulrajbhakat.github.io/Dhun".to_string());

    let cors_origin = std::env::var("CORS_ORIGIN")
        .unwrap_or_else(|_| "*".to_string());

    info!("Configuration:");
    info!("  Port: {}", port);
    info!("  Public Guest URL: {}", public_guest_url);
    info!("  CORS Origin: {}", cors_origin);

    let state = Arc::new(CloudServerState::new(
        db,
        public_guest_url,
        cors_origin.clone(),
    ));

    let x_host_token = HeaderName::from_static("x-host-token");
    let x_guest_token = HeaderName::from_static("x-guest-token");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            ACCEPT,
            x_host_token,
            x_guest_token,
        ])
        .allow_origin(tower_http::cors::Any);

    let app = create_cloud_router(state)
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("DHUN Cloud Server successfully bound on 0.0.0.0:{}", port);
    info!("Ready for host laptop and guest smartphones.");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("DHUN Cloud Server stopped gracefully.");
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
