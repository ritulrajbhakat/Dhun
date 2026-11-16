pub mod ip;
pub mod qr;
pub mod rate_limit;
pub mod routes;

use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::db::Database;
use crate::network::ip::get_local_ip;
use crate::network::qr::generate_party_qr_svg;
use crate::network::rate_limit::RateLimiter;
use crate::network::routes::{get_party_info, get_session_requests, search_catalog, serve_guest_mobile_html, submit_request, AppState};

pub struct LocalPartyServer {
    pub port: u16,
    pub local_ip: String,
    pub qr_svg: String,
    pub guest_url: String,
}

impl LocalPartyServer {
    pub async fn start(
        port: u16,
        session_id: String,
        party_name: String,
        primary_mood: String,
        db: Database,
    ) -> Result<Self> {
        let local_ip = get_local_ip();
        let guest_url = format!("http://{}:{}/", local_ip, port);
        let qr_svg = generate_party_qr_svg(&guest_url)?;

        let state = Arc::new(AppState {
            db,
            session_id,
            party_name,
            primary_mood,
            rate_limiter: RateLimiter::new(4, 300), // Max 4 requests per 5 minutes per guest
        });

        let app = Router::new()
            .route("/", get(serve_guest_mobile_html))
            .route("/request", get(serve_guest_mobile_html))
            .route("/api/party/info", get(get_party_info))
            .route("/api/guest/search", get(search_catalog))
            .route("/api/guest/request", post(submit_request))
            .route("/api/guest/requests", get(get_session_requests))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| anyhow!("Failed to bind local party server on port {}: {}", port, e))?;

        info!("Local DHUN Guest Server listening on {}", guest_url);

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Local party server error: {}", e);
            }
        });

        Ok(Self {
            port,
            local_ip,
            qr_svg,
            guest_url,
        })
    }
}

