//! northhing Relay Server
//!
//! Standalone binary that runs the relay as a network service.
//! Uses `DiskAssetStore` for filesystem-backed mobile-web file storage.

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

mod config;

use config::RelayConfig;
use northhing_relay_core::validated::ValidatedRoomId;
use northhing_relay_core::{build_relay_router, RoomManager, WebAssetStore};
use northhing_relay_server::DiskAssetStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    let cfg = RelayConfig::from_env()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    info!("northhing Relay Server v{}", env!("CARGO_PKG_VERSION"));

    // ── Auth status logging (no key value printed) ─────────────────────
    if let Some(ref _key) = cfg.api_key {
        match cfg.api_key_source {
            config::ApiKeySource::Env => {
                info!("API key authentication enabled (source: RELAY_API_KEY env var)");
            }
            config::ApiKeySource::File => {
                info!("API key authentication enabled (source: key file)");
            }
            config::ApiKeySource::None => {
                // Should not happen when api_key is Some, but handle gracefully.
                info!("API key authentication enabled");
            }
        }
    } else if cfg.is_loopback() {
        info!("API key authentication disabled (loopback bind only)");
    }

    // ── Bind address info ──────────────────────────────────────────────
    if cfg.is_loopback() {
        info!("Bind address: {} (loopback only)", cfg.listen_addr);
    } else {
        info!(
            "Bind address: {} (non-loopback — ensure RELAY_API_KEY is set)",
            cfg.listen_addr
        );
    }

    // ── Room management ────────────────────────────────────────────────
    let room_manager = RoomManager::new();
    let asset_store = Arc::new(DiskAssetStore::new(&cfg.room_web_dir));

    let cleanup_rm = room_manager.clone();
    let cleanup_ttl = cfg.room_ttl_secs;
    let cleanup_store = asset_store.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let stale_ids = cleanup_rm.cleanup_stale_rooms(cleanup_ttl);
            for room_id_str in &stale_ids {
                match ValidatedRoomId::try_from(room_id_str.as_str()) {
                    Ok(room_id) => cleanup_store.cleanup_room(&room_id),
                    Err(_) => warn!("Skipping cleanup for invalid room id: {room_id_str}"),
                }
            }
        }
    });

    let start_time = std::time::Instant::now();
    let mut app = build_relay_router(room_manager, asset_store, start_time, cfg.api_key.clone());

    // ── CORS layer ─────────────────────────────────────────────────────
    // Config field `cors_allow_origins` was defined in `RelayConfig` but
    // never wired to the axum router (the router used hardcoded
    // `CorsLayer::permissive()` in `build_relay_router`).
    //
    // The brief required verifying this and wiring it if unconnected.
    // Confirmed: `cors_allow_origins` was defined at config.rs:16 but
    // never consumed — the CORS was always `CorsLayer::permissive()` at
    // relay-core/src/lib.rs:168. We now apply a custom CorsLayer here
    // (replacing the permissive default from relay-core) when origins
    // are configured. An empty vec means localhost origins (any port)
    // are allowed by default. A single `*` entry restores the permissive
    // behaviour.
    let cors = if cfg.cors_allow_origins.is_empty() {
        // Default: localhost origins (any port) and 127.0.0.1 (any port).
        // tower-http 0.6 does not support port wildcards, so we enumerate
        // common dev ports. Additionally, we allow any port on localhost
        // by using `AllowOrigin::predicate`.
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::predicate(
                |origin: &axum::http::HeaderValue, _request_parts: &axum::http::request::Parts| {
                    let origin_str = match origin.to_str() {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    // Allow any origin whose host is localhost or 127.0.0.1
                    // (any port/subdomain).
                    origin_str == "http://localhost"
                        || origin_str.starts_with("http://localhost:")
                        || origin_str == "http://127.0.0.1"
                        || origin_str.starts_with("http://127.0.0.1:")
                        || origin_str.starts_with("https://localhost")
                        || origin_str.starts_with("https://127.0.0.1")
                },
            ))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    } else if cfg.cors_allow_origins.len() == 1 && cfg.cors_allow_origins[0] == "*" {
        // Explicit `*` restores permissive behaviour.
        CorsLayer::permissive()
    } else {
        // Specific origins from RELAY_CORS_ALLOW_ORIGINS env var.
        let origins: Vec<axum::http::HeaderValue> = cfg
            .cors_allow_origins
            .iter()
            .filter_map(|o| axum::http::HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::list(origins))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };
    app = app.layer(cors);

    if let Some(static_dir) = &cfg.static_dir {
        info!("Serving static files from: {static_dir}");
        app = app
            .fallback_service(tower_http::services::ServeDir::new(static_dir).append_index_html_on_directories(true));
    }

    info!("Room web upload dir: {}", cfg.room_web_dir);

    let listener = tokio::net::TcpListener::bind(cfg.listen_addr).await?;
    info!("Relay server listening on {}", cfg.listen_addr);
    info!("WebSocket endpoint: ws://{}/ws", cfg.listen_addr);

    axum::serve(listener, app).await?;
    Ok(())
}
