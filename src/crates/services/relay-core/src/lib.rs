//! northhing Relay Core
//!
//! Shared relay logic extracted from the standalone relay-server.
//! Used by both the standalone relay-server binary and the embedded relay
//! running inside the desktop process.

pub mod relay;
pub mod routes;
pub mod validated;

pub use relay::room::{CreateRoomOutcome, ResponsePayload, RoomManager};
pub use routes::api::AppState;
pub use routes::websocket::OutboundProtocol;
pub use validated::{ContentHash, ValidatedRelPath, ValidatedRoomId};

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

// ── WebAssetStore trait ───────────────────────────────────────────────

/// Abstract storage for per-room mobile-web static assets.
///
/// The standalone relay uses `DiskAssetStore` (filesystem-backed), while
/// the embedded relay uses `MemoryAssetStore` (in-memory DashMap-backed).
pub trait WebAssetStore: Send + Sync + 'static {
    /// Check if content with this SHA-256 hash exists in the store.
    fn has_content(&self, hash: &ContentHash) -> bool;

    /// Store content by its SHA-256 hash. No-op if already present.
    fn store_content(&self, hash: &ContentHash, data: Vec<u8>) -> Result<(), String>;

    /// Associate a relative file path within a room to a stored content hash.
    fn map_to_room(
        &self,
        room_id: &ValidatedRoomId,
        rel_path: &ValidatedRelPath,
        hash: &ContentHash,
    ) -> Result<(), String>;

    /// Retrieve file content for serving. Falls back to `index.html` if the
    /// requested path doesn't exist (SPA routing).
    fn get_file(&self, room_id: &ValidatedRoomId, path: &ValidatedRelPath) -> Option<Vec<u8>>;

    /// Check if any web files have been uploaded for this room.
    fn has_room_files(&self, room_id: &ValidatedRoomId) -> bool;

    /// Remove all uploaded web files for a room.
    fn cleanup_room(&self, room_id: &ValidatedRoomId);
}

// ── MemoryAssetStore ──────────────────────────────────────────────────

/// In-memory asset store backed by DashMap. Used by the embedded relay.
pub struct MemoryAssetStore {
    content_store: DashMap<String, Arc<Vec<u8>>>,
    room_manifests: DashMap<String, HashMap<String, String>>,
}

impl MemoryAssetStore {
    pub fn new() -> Self {
        Self {
            content_store: DashMap::new(),
            room_manifests: DashMap::new(),
        }
    }
}

impl Default for MemoryAssetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WebAssetStore for MemoryAssetStore {
    fn has_content(&self, hash: &ContentHash) -> bool {
        self.content_store.contains_key(hash.as_str())
    }

    fn store_content(&self, hash: &ContentHash, data: Vec<u8>) -> Result<(), String> {
        self.content_store
            .entry(hash.to_string())
            .or_insert_with(|| Arc::new(data));
        Ok(())
    }

    fn map_to_room(
        &self,
        room_id: &ValidatedRoomId,
        rel_path: &ValidatedRelPath,
        hash: &ContentHash,
    ) -> Result<(), String> {
        self.room_manifests
            .entry(room_id.as_str().to_string())
            .or_default()
            .insert(rel_path.as_str().to_string(), hash.to_string());
        Ok(())
    }

    fn get_file(&self, room_id: &ValidatedRoomId, path: &ValidatedRelPath) -> Option<Vec<u8>> {
        let manifest = self.room_manifests.get(room_id.as_str())?;
        // Also try index.html fallback (stored as string key directly)
        let hash = manifest.get(path.as_str()).or_else(|| manifest.get("index.html"))?;
        let content = self.content_store.get(hash)?;
        Some(content.value().as_ref().clone())
    }

    fn has_room_files(&self, room_id: &ValidatedRoomId) -> bool {
        self.room_manifests.contains_key(room_id.as_str())
    }

    fn cleanup_room(&self, room_id: &ValidatedRoomId) {
        self.room_manifests.remove(room_id.as_str());
    }
}

// ── Router builder ────────────────────────────────────────────────────

/// Build the relay router with all API, WebSocket, and static-file routes.
///
/// Both the standalone binary and the embedded relay call this function,
/// passing their own `WebAssetStore` implementation.
///
/// 2026-06-26: `api_key` enables API key auth on `pair` and `command`
/// when `Some(_)`. When `None`, those endpoints stay open (dev mode
/// only). `main.rs` passes `cfg.api_key` here; the embedded relay
/// passes `None` (or its own config).
pub fn build_relay_router(
    room_manager: Arc<RoomManager>,
    asset_store: Arc<dyn WebAssetStore>,
    start_time: std::time::Instant,
    api_key: Option<String>,
) -> Router {
    let state = AppState {
        room_manager,
        start_time,
        asset_store,
        api_key,
        ws_idle_timeout: std::time::Duration::from_secs(90),
    };

    Router::new()
        .route("/health", get(routes::api::health_check))
        .route("/api/info", get(routes::api::server_info))
        .route("/api/rooms/{room_id}/pair", post(routes::api::pair))
        .route(
            "/api/rooms/{room_id}/command",
            post(routes::api::command).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route(
            "/api/rooms/{room_id}/upload-web",
            post(routes::api::upload_web).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route(
            "/api/rooms/{room_id}/check-web-files",
            post(routes::api::check_web_files),
        )
        .route(
            "/api/rooms/{room_id}/upload-web-files",
            post(routes::api::upload_web_files).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/r/{*rest}", get(routes::api::serve_room_web_catchall))
        .route("/ws", get(routes::websocket::websocket_handler))
        // CORS layer is applied per-consumer (standalone relay-server applies
        // config-based CORS; embedded relay applies permissive CORS for LAN/ngrok).
        // Hardcoded CorsLayer::permissive() was removed 2026-08-04 (P1-5).
        .with_state(state)
}
