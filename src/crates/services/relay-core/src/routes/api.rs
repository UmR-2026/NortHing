//! REST API routes for the relay server.
//!
//! Provides two HTTP endpoints for mobile clients:
//! - POST /api/rooms/:room_id/pair — initiate pairing
//! - POST /api/rooms/:room_id/command — send encrypted commands
//!
//! Both endpoints bridge the HTTP request to the desktop via WebSocket
//! using correlation-based request-response matching.
//!
//! File-serving and upload endpoints use the `WebAssetStore` trait,
//! so the same handlers work for both disk-backed and memory-backed stores.

use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::relay::RoomManager;
use crate::routes::websocket::OutboundProtocol;
use crate::validated::{ContentHash, ValidatedRelPath, ValidatedRoomId};
use crate::WebAssetStore;

#[derive(Clone)]
pub struct AppState {
    pub room_manager: Arc<RoomManager>,
    pub start_time: std::time::Instant,
    pub asset_store: Arc<dyn WebAssetStore>,
    /// 2026-06-26: shared secret for the `X-API-Key` header. When
    /// `Some(_)`, `pair` and `command` reject requests without a
    /// matching key. When `None`, those endpoints are open (dev only).
    /// Set via `RELAY_API_KEY` env var in `RelayConfig::from_env`.
    pub api_key: Option<String>,
}

// ── Health & Info ──────────────────────────────────────────────────────────

/// 2026-06-26: API key authentication extractor for the `pair` and
/// `command` endpoints. Reads the `X-API-Key` request header. The
/// handler then calls [`AuthExtractor::require`] against
/// `state.api_key` to decide.
///
/// Review: `CODE_REVIEW_2026-06-26.md` §"Relay Server 完全缺乏认证机制".
///
/// `FromRequestParts` is the right trait here (vs `FromRequest`)
/// because we only need headers — the body extractor (`Json<T>`) can
/// still consume the request body afterwards. The 401 path uses
/// `StatusCode::UNAUTHORIZED`; the message is logged but not
/// returned to the client to avoid information leakage about whether
/// the key was missing vs wrong.
pub struct AuthExtractor {
    pub api_key: Option<String>,
}

impl AuthExtractor {
    /// Verify the extracted key against the configured one.
    /// - `None` configured: pass through (dev mode).
    /// - `Some(expected)` and key matches: pass.
    /// - `Some(expected)` and key missing or wrong: 401.
    pub fn require(&self, expected: &Option<String>) -> Result<(), StatusCode> {
        match (expected, &self.api_key) {
            (None, _) => Ok(()),
            (Some(_), None) => Err(StatusCode::UNAUTHORIZED),
            (Some(e), Some(p)) if e == p => Ok(()),
            (Some(_), Some(_)) => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

// Axum 0.8+ uses native async fn in trait (no `#[async_trait]`).
// Rust 1.75+ (we're on 1.95) supports the implicit-lifetime form.
// We pass-through the request headers and grab the `X-API-Key`
// value as an owned `String` (the `HeaderValue` borrows from the
// request, and we need to outlive `parts`).
impl<S> FromRequestParts<S> for AuthExtractor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Ok(AuthExtractor { api_key })
    }
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub rooms: usize,
    pub connections: usize,
}

pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        rooms: state.room_manager.room_count(),
        connections: state.room_manager.connection_count(),
    })
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: u8,
}

pub async fn server_info() -> Json<ServerInfo> {
    Json(ServerInfo {
        name: "northhing Relay Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: 2,
    })
}

// ── Pair & Command (HTTP-to-WS bridge) ────────────────────────────────────

#[derive(Deserialize)]
pub struct PairRequest {
    pub public_key: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Serialize)]
pub struct PairResponse {
    pub encrypted_data: String,
    pub nonce: String,
}

/// `POST /api/rooms/:room_id/pair`
///
/// Mobile sends its public key to initiate pairing. The relay forwards this
/// to the desktop via WebSocket and waits for the encrypted challenge response.
pub async fn pair(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    auth: AuthExtractor,
    Json(body): Json<PairRequest>,
) -> Result<Json<PairResponse>, StatusCode> {
    auth.require(&state.api_key)?;
    if !state.room_manager.has_desktop(&room_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let correlation_id = generate_correlation_id();
    let rx = state.room_manager.register_pending(correlation_id.clone());

    let ws_msg = serde_json::to_string(&OutboundProtocol::PairRequest {
        correlation_id: correlation_id.clone(),
        public_key: body.public_key,
        device_id: body.device_id,
        device_name: body.device_name,
    })
    .unwrap_or_default();

    if !state.room_manager.send_to_desktop(&room_id, &ws_msg) {
        state.room_manager.cancel_pending(&correlation_id);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    match tokio::time::timeout(Duration::from_secs(30), rx).await {
        Ok(Ok(payload)) => Ok(Json(PairResponse {
            encrypted_data: payload.encrypted_data,
            nonce: payload.nonce,
        })),
        _ => {
            state.room_manager.cancel_pending(&correlation_id);
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub encrypted_data: String,
    pub nonce: String,
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub encrypted_data: String,
    pub nonce: String,
}

/// `POST /api/rooms/:room_id/command`
///
/// Mobile sends an encrypted command. The relay forwards it to the desktop
/// via WebSocket, waits for the encrypted response, and returns it.
pub async fn command(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    auth: AuthExtractor,
    Json(body): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    auth.require(&state.api_key)?;
    if !state.room_manager.has_desktop(&room_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let correlation_id = generate_correlation_id();
    let rx = state.room_manager.register_pending(correlation_id.clone());

    let ws_msg = serde_json::to_string(&OutboundProtocol::Command {
        correlation_id: correlation_id.clone(),
        encrypted_data: body.encrypted_data,
        nonce: body.nonce,
    })
    .unwrap_or_default();

    if !state.room_manager.send_to_desktop(&room_id, &ws_msg) {
        state.room_manager.cancel_pending(&correlation_id);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    match tokio::time::timeout(Duration::from_secs(60), rx).await {
        Ok(Ok(payload)) => Ok(Json(CommandResponse {
            encrypted_data: payload.encrypted_data,
            nonce: payload.nonce,
        })),
        _ => {
            state.room_manager.cancel_pending(&correlation_id);
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}

fn generate_correlation_id() -> String {
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ── Per-room mobile-web upload & serving ────────────────────────────────────

#[derive(Deserialize)]
pub struct UploadWebRequest {
    pub files: HashMap<String, String>,
}

/// `POST /api/rooms/:room_id/upload-web`
pub async fn upload_web(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(body): Json<UploadWebRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    let room_id_validated = ValidatedRoomId::try_from(room_id.as_str()).map_err(|_| StatusCode::NOT_FOUND)?;
    if !state.room_manager.room_exists(room_id_validated.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut written = 0usize;
    let mut reused = 0usize;
    for (rel_path, b64_content) in &body.files {
        let rel_path_validated = ValidatedRelPath::try_from(rel_path.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let decoded = B64.decode(b64_content).map_err(|_| StatusCode::BAD_REQUEST)?;
        let hash = ContentHash::from_data(&decoded);

        if !state.asset_store.has_content(&hash) {
            state
                .asset_store
                .store_content(&hash, decoded)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            written += 1;
        } else {
            reused += 1;
        }

        state
            .asset_store
            .map_to_room(&room_id_validated, &rel_path_validated, &hash)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tracing::info!("Room {room_id}: upload-web complete (new={written}, reused={reused})");
    Ok(Json(serde_json::json!({
        "status": "ok",
        "files_written": written,
        "files_reused": reused
    })))
}

// ── Incremental upload protocol ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FileManifestEntry {
    pub path: String,
    pub hash: String,
    // reason: size is deserialized for forward-compat with size-based dedup heuristics; today only hash equality is checked
    #[allow(dead_code)]
    pub size: u64,
}

#[derive(Deserialize)]
pub struct CheckWebFilesRequest {
    pub files: Vec<FileManifestEntry>,
}

#[derive(Serialize)]
pub struct CheckWebFilesResponse {
    pub needed: Vec<String>,
    pub existing_count: usize,
    pub total_count: usize,
}

/// `POST /api/rooms/:room_id/check-web-files`
pub async fn check_web_files(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(body): Json<CheckWebFilesRequest>,
) -> Result<Json<CheckWebFilesResponse>, StatusCode> {
    let room_id_validated = ValidatedRoomId::try_from(room_id.as_str()).map_err(|_| StatusCode::NOT_FOUND)?;
    if !state.room_manager.room_exists(room_id_validated.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut needed = Vec::new();
    let mut existing_count = 0usize;
    let total_count = body.files.len();

    for entry in &body.files {
        let rel_path = match ValidatedRelPath::try_from(entry.path.as_str()) {
            Ok(v) => v,
            Err(_) => {
                // Invalid path — client must re-upload; push to needed.
                tracing::warn!(
                    "Room {}: ignoring malformed path {}",
                    room_id_validated.as_str(),
                    entry.path
                );
                needed.push(entry.path.clone());
                continue;
            }
        };
        let hash = match ContentHash::try_from(entry.hash.as_str()) {
            Ok(h) => h,
            Err(_) => {
                // Invalid hash format — push to needed (client will encounter validation error later).
                needed.push(entry.path.clone());
                continue;
            }
        };
        if state.asset_store.has_content(&hash) {
            if state
                .asset_store
                .map_to_room(&room_id_validated, &rel_path, &hash)
                .is_ok()
            {
                existing_count += 1;
            } else {
                // map_to_room failed (e.g., can't create link): count as needed
                // so client retries.
                needed.push(entry.path.clone());
            }
        } else {
            needed.push(entry.path.clone());
        }
    }

    tracing::info!(
        "Room {room_id}: check-web-files total={total_count}, existing={existing_count}, needed={}",
        needed.len()
    );

    Ok(Json(CheckWebFilesResponse {
        needed,
        existing_count,
        total_count,
    }))
}

#[derive(Deserialize)]
pub struct UploadWebFilesEntry {
    pub content: String,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct UploadWebFilesRequest {
    pub files: HashMap<String, UploadWebFilesEntry>,
}

/// `POST /api/rooms/:room_id/upload-web-files`
pub async fn upload_web_files(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(body): Json<UploadWebFilesRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};

    let room_id_validated = ValidatedRoomId::try_from(room_id.as_str()).map_err(|_| StatusCode::NOT_FOUND)?;
    if !state.room_manager.room_exists(room_id_validated.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut stored = 0usize;
    for (rel_path, entry) in &body.files {
        let rel_path_validated = ValidatedRelPath::try_from(rel_path.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let hash_validated = ContentHash::try_from(entry.hash.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let decoded = B64.decode(&entry.content).map_err(|_| StatusCode::BAD_REQUEST)?;
        let actual_hash = ContentHash::from_data(&decoded);
        if actual_hash != hash_validated {
            tracing::warn!(
                "Room {room_id}: hash mismatch for {} (expected={}, actual={})",
                rel_path,
                entry.hash,
                actual_hash,
            );
            return Err(StatusCode::BAD_REQUEST);
        }

        if !state.asset_store.has_content(&actual_hash) {
            state
                .asset_store
                .store_content(&actual_hash, decoded)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            stored += 1;
        }

        state
            .asset_store
            .map_to_room(&room_id_validated, &rel_path_validated, &actual_hash)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tracing::info!("Room {room_id}: upload-web-files stored {stored} new files");
    Ok(Json(serde_json::json!({ "status": "ok", "files_stored": stored })))
}

/// `GET /r/{*rest}` — serve per-room mobile-web static files.
pub async fn serve_room_web_catchall(
    State(state): State<AppState>,
    Path(rest): Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::body::Body;
    use axum::http::header;
    use axum::response::IntoResponse;

    let rest = rest.trim_start_matches('/');
    let (room_id_str, file_path_str) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx + 1..]),
        None => (rest, ""),
    };

    if room_id_str.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let room_id_validated = ValidatedRoomId::try_from(room_id_str).map_err(|_| StatusCode::NOT_FOUND)?;

    let lookup_path = if file_path_str.is_empty() {
        ValidatedRelPath::try_from("index.html").map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        ValidatedRelPath::try_from(file_path_str).map_err(|_| StatusCode::BAD_REQUEST)?
    };

    let content = state
        .asset_store
        .get_file(&room_id_validated, &lookup_path)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mime = mime_from_path(lookup_path.as_str());
    Ok(([(header::CONTENT_TYPE, mime)], Body::from(content)).into_response())
}

fn mime_from_path(p: &str) -> &'static str {
    match p.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

// ── Handler tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod handler_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    use crate::validated::{ContentHash, ValidatedRelPath, ValidatedRoomId};
    use crate::MemoryAssetStore;
    use crate::WebAssetStore;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;

    fn make_state(store: Arc<dyn WebAssetStore>) -> AppState {
        AppState {
            room_manager: crate::relay::RoomManager::new(),
            start_time: std::time::Instant::now(),
            asset_store: store,
            api_key: None,
        }
    }

    /// Create a populated in-memory store: store the hash+data, then map
    /// `rel_path` inside `room_id` so `has_content` and `map_to_room` both succeed.
    fn populate(store: &MemoryAssetStore, room_id: &str, rel_path: &str, data: &[u8]) {
        let h = ContentHash::from_data(data);
        store.store_content(&h, data.to_vec()).unwrap();
        let rid = ValidatedRoomId::try_from(room_id).unwrap();
        let rp = ValidatedRelPath::try_from(rel_path).unwrap();
        store.map_to_room(&rid, &rp, &h).unwrap();
    }

    /// A minimal room present for `room_exists` to return true.
    fn ensure_room(manager: &crate::relay::RoomManager, room_id: &str) {
        let (tx, _rx) = mpsc::unbounded_channel();
        // conn_id 1 is safe since new() starts next_conn_id at 1 and we never
        // consume a call to next_conn_id().
        manager.create_room(room_id, 0, "device-test", "pk-test", tx);
    }

    // A store that reports content exists but map_to_room always fails at runtime.
    struct FailingMapStore(MemoryAssetStore);

    impl WebAssetStore for FailingMapStore {
        fn has_content(&self, hash: &ContentHash) -> bool {
            self.0.has_content(hash)
        }
        fn store_content(&self, hash: &ContentHash, data: Vec<u8>) -> Result<(), String> {
            self.0.store_content(hash, data)
        }
        fn map_to_room(
            &self,
            _room_id: &ValidatedRoomId,
            _rel_path: &ValidatedRelPath,
            _hash: &ContentHash,
        ) -> Result<(), String> {
            Err("disk mapping failure".to_string())
        }
        fn get_file(&self, _room_id: &ValidatedRoomId, _path: &ValidatedRelPath) -> Option<Vec<u8>> {
            self.0.get_file(_room_id, _path)
        }
        fn has_room_files(&self, room_id: &ValidatedRoomId) -> bool {
            self.0.has_room_files(room_id)
        }
        fn cleanup_room(&self, room_id: &ValidatedRoomId) {
            self.0.cleanup_room(room_id)
        }
    }

    #[tokio::test]
    async fn check_web_files_existing_counts_on_successful_map() {
        let mem = MemoryAssetStore::new();
        populate(&mem, "my-room", "app.js", b"js");
        populate(&mem, "my-room", "index.html", b"<html>");
        let missing_hash = ContentHash::from_data(b"missing content");

        let state = make_state(Arc::new(mem));
        ensure_room(&state.room_manager, "my-room");

        let body = Json(CheckWebFilesRequest {
            files: vec![
                FileManifestEntry {
                    path: "app.js".to_string(),
                    hash: ContentHash::from_data(b"js").as_str().to_string(),
                    size: 2,
                },
                FileManifestEntry {
                    path: "missing.js".to_string(),
                    hash: missing_hash.as_str().to_string(),
                    size: 0,
                },
            ],
        });
        let res = check_web_files(State(state), Path("my-room".to_string()), body)
            .await
            .expect("should succeed");
        let resp = res.0;
        assert_eq!(resp.existing_count, 1, "exactly one entry existed");
        assert_eq!(resp.total_count, 2);
        assert_eq!(resp.needed.len(), 1);
        assert_eq!(resp.needed[0], "missing.js");
    }

    #[tokio::test]
    async fn check_web_files_failing_map_counts_needed_not_existing() {
        let failing = FailingMapStore(MemoryAssetStore::new());
        // Pre-populate the inner store so has_content sees the hash, but our
        // wrapper will reject the map_to_room call, exercising the M-8 path.
        let h = ContentHash::from_data(b"data");
        failing.0.store_content(&h, b"data".to_vec()).unwrap();

        let state = make_state(Arc::new(failing));
        ensure_room(&state.room_manager, "r");

        let body = Json(CheckWebFilesRequest {
            files: vec![FileManifestEntry {
                path: "a.js".to_string(),
                hash: h.as_str().to_string(),
                size: 4,
            }],
        });
        let res = check_web_files(State(state), Path("r".to_string()), body)
            .await
            .expect("should succeed");
        let resp = res.0;
        assert_eq!(resp.existing_count, 0, "map failure must not inflate existing_count");
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.needed.len(), 1, "failed mapping falls to needed so client retries");
        assert_eq!(resp.needed[0], "a.js");
        // Invariant required by M-8: counts cover every entry exactly once.
        assert_eq!(resp.existing_count + resp.needed.len(), resp.total_count);
    }

    #[tokio::test]
    async fn check_web_files_rejects_invalid_room_id() {
        let state = make_state(Arc::new(MemoryAssetStore::new()));
        let body = Json(CheckWebFilesRequest { files: vec![] });
        let res = check_web_files(State(state), Path("..".to_string()), body).await;
        assert!(matches!(res, Err(StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn check_web_files_invalid_path_counts_as_needed() {
        let state = make_state(Arc::new(MemoryAssetStore::new()));
        ensure_room(&state.room_manager, "r");
        let body = Json(CheckWebFilesRequest {
            files: vec![FileManifestEntry {
                path: "../x".to_string(),
                hash: "a".repeat(64),
                size: 0,
            }],
        });
        let res = check_web_files(State(state), Path("r".to_string()), body)
            .await
            .expect("should succeed");
        let resp = res.0;
        assert_eq!(resp.existing_count, 0);
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.needed.len(), 1, "malformed path entry must land in needed");
    }

    #[tokio::test]
    async fn upload_web_rejects_traversal_path() {
        let state = make_state(Arc::new(MemoryAssetStore::new()));
        ensure_room(&state.room_manager, "r");
        let body = Json(UploadWebRequest {
            files: [(
                "../evil".to_string(),
                base64::engine::general_purpose::STANDARD.encode(b"x"),
            )]
            .into_iter()
            .collect(),
        });
        let res = upload_web(State(state), Path("r".to_string()), body).await;
        assert!(matches!(res, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn serve_catchall_rejects_invalid_rel_path() {
        let state = make_state(Arc::new(MemoryAssetStore::new()));
        let res = serve_room_web_catchall(State(state), Path("r/../x".to_string())).await;
        assert!(matches!(res, Err(StatusCode::BAD_REQUEST)));
    }
}
