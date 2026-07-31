//! WebSocket handler for the relay server.
//!
//! Only desktop clients connect via WebSocket. Mobile clients use HTTP.
//! The relay bridges HTTP requests to the desktop via WebSocket using
//! correlation IDs for request-response matching.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::relay::room::{ConnId, CreateRoomOutcome, OutboundMessage, ResponsePayload, RoomManager};
use crate::routes::api::{AppState, AuthExtractor};

/// Per-message/frame/write-buffer cap. Audit H-2: the previous 64 MiB bound
/// allowed a malicious peer to pin hundreds of MiB per connection.
/// RelayCommand/RelayResponse carry base64 `encrypted_data` payloads far
/// smaller than this; 8 MiB leaves ample headroom (the HTTP command body
/// limit is 10 MiB as reference).
const MAX_WS_FRAME_SIZE: usize = 8 * 1024 * 1024;

/// Per-connection outbound queue capacity. A peer that stops reading fills
/// the queue; the sender then disconnects it instead of blocking the read
/// loop (bounded memory, no back-pressure deadlock).
const OUTBOUND_QUEUE_CAPACITY: usize = 256;

fn truncate_preview(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }

    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Messages received from the desktop via WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMessage {
    CreateRoom {
        room_id: Option<String>,
        device_id: String,
        // reason: device_type is deserialized to accept the W3C-style client kind (e.g. "mobile-web"); not yet routed for differentiated behavior
        #[allow(dead_code)]
        device_type: String,
        public_key: String,
    },
    /// Desktop responds to a bridged HTTP request.
    RelayResponse {
        correlation_id: String,
        encrypted_data: String,
        nonce: String,
    },
    Heartbeat,
}

/// Messages sent to the desktop via WebSocket.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundProtocol {
    RoomCreated {
        room_id: String,
    },
    /// Mobile pairing request forwarded to desktop.
    PairRequest {
        correlation_id: String,
        public_key: String,
        device_id: String,
        device_name: String,
    },
    /// Encrypted command from mobile forwarded to desktop.
    Command {
        correlation_id: String,
        encrypted_data: String,
        nonce: String,
    },
    HeartbeatAck,
    Error {
        message: String,
    },
}

pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>, auth: AuthExtractor) -> Response {
    // C-2: gate the upgrade itself. `api_key = None` (embedded relay / dev)
    // stays open; a configured key must be present and matching.
    if auth.require(&state.api_key).is_err() {
        warn!("Rejected WebSocket upgrade: missing or invalid API key");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // H-2: global connection cap. Admit atomically; the slot is released
    // on teardown (`handle_socket`) or on upgrade failure.
    if !state.room_manager.try_acquire_connection() {
        warn!("Rejected WebSocket upgrade: connection limit reached");
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let manager = state.room_manager.clone();

    ws.max_message_size(MAX_WS_FRAME_SIZE)
        .max_frame_size(MAX_WS_FRAME_SIZE)
        .max_write_buffer_size(MAX_WS_FRAME_SIZE)
        .on_failed_upgrade(move |_| {
            manager.release_connection();
        })
        .on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<OutboundMessage>(OUTBOUND_QUEUE_CAPACITY);

    let conn_id = state.room_manager.next_conn_id();
    info!("WebSocket connected: conn_id={conn_id}");

    let write_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if !msg.text.is_empty() && ws_sender.send(Message::Text(msg.text.into())).await.is_err() {
                break;
            }
        }
    });

    loop {
        // H-2: idle timeout — a connection with no inbound traffic for
        // `state.ws_idle_timeout` is considered dead and closed.
        let frame = tokio::time::timeout(state.ws_idle_timeout, ws_receiver.next()).await;
        match frame {
            Ok(Some(Ok(Message::Text(text)))) => {
                if !handle_text_message(&text, conn_id, &state.room_manager, &out_tx) {
                    warn!("Disconnecting slow consumer conn_id={conn_id}: outbound queue is full or closed");
                    break;
                }
            }
            Ok(Some(Ok(Message::Ping(_)))) => {}
            Ok(Some(Ok(Message::Close(_)))) => {
                info!("WebSocket close from conn_id={conn_id}");
                break;
            }
            Ok(Some(Err(e))) => {
                error!("WebSocket error conn_id={conn_id}: {e}");
                break;
            }
            Ok(None) => break,
            Ok(Some(Ok(_))) => {}
            Err(_) => {
                debug!(
                    "WebSocket idle timeout for conn_id={conn_id} (no message for {:?})",
                    state.ws_idle_timeout
                );
                break;
            }
        }
    }

    state.room_manager.on_disconnect(conn_id);
    state.room_manager.release_connection();
    drop(out_tx);
    // The write task may be stuck flushing frames to a stalled peer; abort
    // it so the connection slot is released promptly instead of awaiting a
    // peer that may never drain.
    write_task.abort();
    let _ = write_task.await;
    info!("WebSocket disconnected: conn_id={conn_id}");
}

/// Handle one inbound text message. Returns `false` when the connection
/// must be dropped (outbound queue full / closed — slow consumer).
fn handle_text_message(
    text: &str,
    conn_id: ConnId,
    room_manager: &Arc<RoomManager>,
    out_tx: &mpsc::Sender<OutboundMessage>,
) -> bool {
    debug!("Received from conn_id={conn_id}: {}", truncate_preview(text, 200));
    let msg: InboundMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!("Invalid message from conn_id={conn_id}: {e}");
            return send_json(
                out_tx,
                &OutboundProtocol::Error {
                    message: format!("invalid message format: {e}"),
                },
            );
        }
    };

    match msg {
        InboundMessage::CreateRoom {
            room_id,
            device_id,
            device_type: _,
            public_key,
        } => {
            let room_id = room_id.unwrap_or_else(generate_room_id);
            match room_manager.create_room(&room_id, conn_id, &device_id, &public_key, out_tx.clone()) {
                CreateRoomOutcome::Created => send_json(out_tx, &OutboundProtocol::RoomCreated { room_id }),
                CreateRoomOutcome::Conflict => {
                    warn!("Room {room_id} create conflict for conn_id={conn_id}");
                    send_json(
                        out_tx,
                        &OutboundProtocol::Error {
                            message: "room already exists".to_string(),
                        },
                    )
                }
            }
        }

        InboundMessage::RelayResponse {
            correlation_id,
            encrypted_data,
            nonce,
        } => {
            debug!("RelayResponse from desktop conn_id={conn_id} corr={correlation_id}");
            room_manager.resolve_pending(&correlation_id, ResponsePayload { encrypted_data, nonce });
            true
        }

        InboundMessage::Heartbeat => {
            if room_manager.heartbeat(conn_id) {
                send_json(out_tx, &OutboundProtocol::HeartbeatAck)
            } else {
                send_json(
                    out_tx,
                    &OutboundProtocol::Error {
                        message: "Room not found or expired".into(),
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn make_state(api_key: Option<String>, idle: Duration) -> AppState {
        AppState {
            room_manager: crate::relay::RoomManager::new(),
            start_time: std::time::Instant::now(),
            asset_store: Arc::new(crate::MemoryAssetStore::new()),
            api_key,
            ws_idle_timeout: idle,
        }
    }

    fn make_router(state: AppState) -> Router {
        Router::new().route("/ws", get(websocket_handler)).with_state(state)
    }

    /// Serve the real router on an ephemeral port.
    async fn spawn_server(api_key: Option<String>, idle: Duration) -> (SocketAddr, Arc<RoomManager>) {
        let state = make_state(api_key, idle);
        let room_manager = state.room_manager.clone();
        let app = make_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (addr, room_manager)
    }

    /// Perform a raw HTTP/1.1 WebSocket handshake and return the status
    /// line plus the socket (still open when the upgrade succeeded).
    async fn raw_ws_handshake(addr: SocketAddr, extra_headers: &str) -> (String, tokio::net::TcpStream) {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = format!(
            "GET /ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n{extra_headers}Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        stream.write_all(req.as_bytes()).await.unwrap();

        let mut resp = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            resp.extend_from_slice(&buf[..n]);
            if resp.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let status_line = String::from_utf8_lossy(&resp)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        (status_line, stream)
    }

    #[test]
    fn truncate_preview_respects_utf8_boundaries() {
        let text = format!("{}{}", "a".repeat(199), "你");

        assert_eq!(truncate_preview(&text, 200), "a".repeat(199));
    }

    // ── WebSocket handshake auth (C-2) ─────────────────────────────────

    #[test]
    fn auth_require_gates_only_when_key_configured() {
        let open = AuthExtractor { api_key: None };
        assert!(open.require(&None).is_ok(), "api_key=None stays open");
        assert!(
            open.require(&Some("k".into())).is_err(),
            "missing key rejected when configured"
        );

        let with_key = AuthExtractor {
            api_key: Some("k".to_string()),
        };
        assert!(with_key.require(&Some("k".into())).is_ok());
        assert!(with_key.require(&Some("other".into())).is_err(), "wrong key rejected");
        assert!(with_key.require(&None).is_ok(), "dev mode stays open");
    }

    #[tokio::test]
    async fn websocket_upgrade_requires_api_key_when_configured() {
        let (addr, _rm) = spawn_server(Some("secret".to_string()), Duration::from_secs(90)).await;
        let (status, _stream) = raw_ws_handshake(addr, "").await;
        assert!(
            status.starts_with("HTTP/1.1 401"),
            "missing key must be rejected, got: {status}"
        );
    }

    #[tokio::test]
    async fn websocket_upgrade_rejects_wrong_api_key() {
        let (addr, _rm) = spawn_server(Some("secret".to_string()), Duration::from_secs(90)).await;
        let (status, _stream) = raw_ws_handshake(addr, "X-API-Key: wrong\r\n").await;
        assert!(
            status.starts_with("HTTP/1.1 401"),
            "wrong key must be rejected, got: {status}"
        );
    }

    #[tokio::test]
    async fn websocket_upgrade_allows_configured_api_key() {
        let (addr, _rm) = spawn_server(Some("secret".to_string()), Duration::from_secs(90)).await;
        let (status, _stream) = raw_ws_handshake(addr, "X-API-Key: secret\r\n").await;
        assert!(
            status.starts_with("HTTP/1.1 101"),
            "matching key must upgrade, got: {status}"
        );
    }

    #[tokio::test]
    async fn websocket_upgrade_open_when_api_key_unset() {
        let (addr, _rm) = spawn_server(None, Duration::from_secs(90)).await;
        let (status, _stream) = raw_ws_handshake(addr, "").await;
        assert!(
            status.starts_with("HTTP/1.1 101"),
            "api_key=None must stay open, got: {status}"
        );
    }

    // ── Idle timeout (H-2) ─────────────────────────────────────────────

    /// Real-socket e2e: after the handshake, a silent connection must be
    /// closed by the server once the idle timeout elapses, and the
    /// connection slot must be released afterwards.
    #[tokio::test]
    async fn idle_socket_is_closed_after_timeout_and_slot_released() {
        let (addr, room_manager) = spawn_server(None, Duration::from_millis(200)).await;
        let (status, mut stream) = raw_ws_handshake(addr, "").await;
        assert!(
            status.starts_with("HTTP/1.1 101"),
            "handshake must succeed, got: {status}"
        );

        // Stay silent: the server must close the socket after the timeout.
        let mut buf = [0u8; 1024];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
            .await
            .expect("server should close the idle connection within 5s")
            .unwrap();
        assert_eq!(n, 0, "server must close the idle socket");

        // Teardown releases the connection slot.
        for _ in 0..100 {
            if room_manager.active_connection_count() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            room_manager.active_connection_count(),
            0,
            "slot must be released after close"
        );
    }

    // ── Bounded outbound queue (H-2) ───────────────────────────────────

    /// With the queue full (slow consumer that never reads), a reply that
    /// cannot be enqueued must report failure — the read loop's disconnect
    /// branch — instead of blocking.
    #[test]
    fn slow_consumer_full_queue_signals_disconnect_without_deadlock() {
        let manager = crate::relay::RoomManager::new();
        let (tx, _rx) = mpsc::channel(1);
        // Fill the only slot; every try_send now fails.
        tx.try_send(OutboundMessage { text: "queued".into() }).unwrap();

        let conn_id = manager.next_conn_id();
        let text = serde_json::json!({
            "type": "create_room",
            "room_id": null,
            "device_id": "device-test",
            "device_type": "desktop",
            "public_key": "pk-test"
        })
        .to_string();

        let proceed = handle_text_message(&text, conn_id, &manager, &tx);
        assert!(!proceed, "full outbound queue must signal the disconnect branch");

        // State mutation happened before the reply attempt.
        assert_eq!(
            manager.room_count(),
            1,
            "room creation is not rolled back by a full queue"
        );
        assert_eq!(manager.connection_count(), 1);
    }

    /// A healthy queue delivers replies; `handle_text_message` reports OK.
    #[tokio::test]
    async fn healthy_queue_delivers_replies() {
        let manager = crate::relay::RoomManager::new();
        let conn_id = manager.next_conn_id();
        let (tx, mut rx) = mpsc::channel(256);

        let text = serde_json::json!({
            "type": "create_room",
            "room_id": "room-ok",
            "device_id": "device-test",
            "device_type": "desktop",
            "public_key": "pk-test"
        })
        .to_string();

        let proceed = handle_text_message(&text, conn_id, &manager, &tx);
        assert!(proceed);

        let reply = rx.try_recv().expect("RoomCreated must be queued");
        let parsed: serde_json::Value = serde_json::from_str(&reply.text).unwrap();
        assert_eq!(parsed["type"], "room_created");
        assert_eq!(parsed["room_id"], "room-ok");
    }

    /// A second CreateRoom for the same live room must yield the protocol
    /// `Error { message: "room already exists" }` frame (H-1).
    #[tokio::test]
    async fn duplicate_create_room_sends_room_exists_error() {
        let manager = crate::relay::RoomManager::new();
        let conn_a = manager.next_conn_id();
        let (tx_a, _rx_a) = mpsc::channel(256);
        let first = serde_json::json!({
            "type": "create_room",
            "room_id": "room-dedup",
            "device_id": "device-a",
            "device_type": "desktop",
            "public_key": "pk-a"
        })
        .to_string();
        assert!(handle_text_message(&first, conn_a, &manager, &tx_a));
        drop(tx_a);

        let conn_b = manager.next_conn_id();
        let (tx_b, mut rx_b) = mpsc::channel(256);
        let second = serde_json::json!({
            "type": "create_room",
            "room_id": "room-dedup",
            "device_id": "device-b",
            "device_type": "desktop",
            "public_key": "pk-b"
        })
        .to_string();
        assert!(handle_text_message(&second, conn_b, &manager, &tx_b));

        // The requester receives the protocol Error frame (H-1 semantics).
        let frame = rx_b.try_recv().expect("Error frame must be queued");
        let parsed: serde_json::Value = serde_json::from_str(&frame.text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "room already exists");

        // Original desktop still owns the room; the intruder is not mapped.
        assert_eq!(
            manager.get_desktop_public_key("room-dedup").as_deref(),
            Some("pk-a"),
            "original desktop must keep the room"
        );
        assert_eq!(manager.connection_count(), 1, "intruder conn must not be registered");
    }
}

/// Try to enqueue `msg` on the bounded outbound queue. `false` means the
/// queue is full (slow consumer) or closed — callers should disconnect.
fn send_json<T: Serialize>(tx: &mpsc::Sender<OutboundMessage>, msg: &T) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => tx.try_send(OutboundMessage { text: json }).is_ok(),
        Err(_) => false,
    }
}

fn generate_room_id() -> String {
    // 16 bytes = 128 bits of entropy. The previous 6-byte (48-bit) value
    // was flagged in `CODE_REVIEW_2026-06-26.md` §"Relay Server 的 room_id
    // 生成使用 6 字节随机，熵不足" — at 2^24 attempts you have a 50%
    // birthday-collision chance. 128 bits is the standard cryptographic
    // floor for opaque identifiers.
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
