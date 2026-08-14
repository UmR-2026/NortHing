//! End-to-end relay web-asset tests (Task 3).
//!
//! Spans the full router built by `build_relay_router` (relay-core) wired to
//! the disk-backed `DiskAssetStore` (relay-server) on a loopback listener,
//! then exercises the upload/serve protocol over raw TCP. Raw wire-level
//! requests are used on purpose:
//!
//! - no client-side URI normalization (the bytes on the wire are exactly
//!   what a remote attacker can send, so V-1 traversal variants are tested
//!   verbatim);
//! - the pattern matches the WebSocket handshake tests in
//!   `relay-core/src/routes/websocket.rs` (Task 2).
//!
//! V-1 dynamic qualification: every encoding variant of `..` traversal
//! against the `/r/{*rest}` catch-all must never return the sibling marker
//! file's content, and genuine traversal forms must not be served at all.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use base64::Engine as _;
use northhing_relay_core::relay::room::OutboundMessage;
use northhing_relay_core::validated::ContentHash;
use northhing_relay_server::{build_relay_router, DiskAssetStore, RoomManager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

const API_KEY: &str = "test-key";
const ROOM_ID: &str = "e2e-room";
const INDEX_HTML: &str = "<!doctype html><html><body>e2e-index</body></html>";
const MARKER: &str = "must-not-leak";

// ── Test environment ───────────────────────────────────────────────────

struct TestEnv {
    addr: SocketAddr,
    base: PathBuf,
    marker: PathBuf,
    _room_manager: Arc<RoomManager>,
    _room_rx: mpsc::Receiver<OutboundMessage>,
    _parent: tempfile::TempDir,
    _server: tokio::task::JoinHandle<()>,
}

/// Spin up the full relay router (relay-core) with a real disk store
/// (relay-server) on 127.0.0.1:0, with one pre-created room.
///
/// The marker file `secret.txt` lives OUTSIDE `base` (sibling position):
/// a successful traversal would have to read it across the base boundary.
async fn setup(api_key: Option<String>) -> TestEnv {
    let parent = tempfile::tempdir().expect("tempdir for base + marker siblings");
    let base = parent.path().join("relay-base");
    let marker = parent.path().join("secret.txt");
    std::fs::write(&marker, MARKER).expect("write sibling marker");

    let room_manager = RoomManager::new();
    let conn_id = room_manager.next_conn_id();
    let (tx, room_rx) = mpsc::channel::<OutboundMessage>(256);
    room_manager.create_room(ROOM_ID, conn_id, "device-e2e", "pk-e2e", tx);

    let store = DiskAssetStore::new(base.to_str().expect("base path is utf8"));
    let app = build_relay_router(room_manager.clone(), Arc::new(store), Instant::now(), api_key);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    TestEnv {
        addr,
        base,
        marker,
        _room_manager: room_manager,
        _room_rx: room_rx,
        _parent: parent,
        _server: server,
    }
}

// ── Raw HTTP helpers (wire-level, no client) ──────────────────────────

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

/// Send a raw HTTP/1.1 request head plus body over a fresh connection and
/// read the full response (head + body). `Connection: close` is set so the
/// server ends the response with EOF; chunked bodies are de-chunked.
async fn raw_http(addr: SocketAddr, head: &str, body: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).await.expect("connect to relay");
    let mut wire = format!("{head}Host: {addr}\r\nConnection: close\r\n\r\n").into_bytes();
    wire.extend_from_slice(body.as_bytes());
    stream.write_all(&wire).await.expect("write request");

    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf).await.expect("read response");
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
    }

    let head_end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("response head terminator");
    let head = String::from_utf8_lossy(&response[..head_end]).into_owned();
    let status: u16 = head.splitn(3, ' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    let raw_body = response[head_end + 4..].to_vec();
    let chunked = head.to_ascii_lowercase().contains("transfer-encoding: chunked");
    let body = if chunked { dechunk(&raw_body) } else { raw_body };
    HttpResponse { status, body }
}

/// Strip HTTP/1.1 chunked framing from a body captured up to EOF.
fn dechunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(line_end) = rest.windows(2).position(|w| w == b"\r\n") {
        let size_str = String::from_utf8_lossy(&rest[..line_end]);
        let size = usize::from_str_radix(size_str.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        let start = line_end + 2;
        if start + size > rest.len() {
            break;
        }
        out.extend_from_slice(&rest[start..start + size]);
        rest = &rest[start + size + 2..];
    }
    out
}

fn get_head(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\n")
}

fn post_head(path: &str, content_length: usize, api_key: Option<&str>) -> String {
    let mut head =
        format!("POST {path} HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\n");
    if let Some(key) = api_key {
        head.push_str(&format!("X-API-Key: {key}\r\n"));
    }
    head
}

fn b64(data: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(data.as_bytes())
}

fn upload_body(files: &[(&str, &str)]) -> String {
    let map: serde_json::Map<String, serde_json::Value> = files
        .iter()
        .map(|(path, content)| ((*path).to_string(), serde_json::Value::String(b64(content))))
        .collect();
    serde_json::json!({ "files": map }).to_string()
}

fn check_body(files: &[(&str, &str)]) -> String {
    let entries: Vec<serde_json::Value> = files
        .iter()
        .map(|(path, content)| {
            let hash = ContentHash::from_data(content.as_bytes());
            serde_json::json!({
                "path": path,
                "hash": hash.as_str(),
                "size": content.len(),
            })
        })
        .collect();
    serde_json::json!({ "files": entries }).to_string()
}

async fn upload(env: &TestEnv, room: &str, files: &[(&str, &str)], key: Option<&str>) -> HttpResponse {
    let body = upload_body(files);
    let head = post_head(&format!("/api/rooms/{room}/upload-web"), body.len(), key);
    raw_http(env.addr, &head, &body).await
}

// ── V-1 helpers ────────────────────────────────────────────────────────

/// Percent-decode a path once, the way axum's `Path` extractor decodes the
/// `{*rest}` wildcard parameter before the handler sees it.
fn percent_decode_once(s: &str) -> String {
    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A variant is a GENUINE traversal attempt if, after one percent-decode
/// (what the handler actually receives) and `\` normalization, its path
/// contains `..`/`.` components, an absolute component, or a drive letter.
/// Variants that decode to plain literal characters (e.g. `%252e%252e` ->
/// `%2e%2e`) are NOT traversal attempts at this layer.
///
/// NOTE (T3 M-1): This helper logic deliberately mirrors the path splitting
/// and component extraction logic in `northhing_relay_core::routes::api::serve_room_web_catchall`
/// (`src/crates/services/relay-core/src/routes/api.rs:467-471`). If the catchall
/// handler's path normalization or segment handling is updated, this test helper
/// must be updated in sync to prevent test-handler logic drift.
fn is_genuine_traversal(path: &str) -> bool {
    fn is_drive(seg: &str) -> bool {
        seg.len() == 2 && seg.ends_with(':') && seg.as_bytes()[0].is_ascii_alphabetic()
    }
    let decoded = percent_decode_once(path).replace('\\', "/");
    let Some(remainder) = decoded.strip_prefix("/r/") else {
        return false;
    };
    let remainder = remainder.trim_start_matches('/');
    let (room_part, file_part) = match remainder.find('/') {
        Some(idx) => (&remainder[..idx], &remainder[idx + 1..]),
        None => (remainder, ""),
    };
    if room_part == ".." || room_part == "." {
        return true;
    }
    if file_part.starts_with('/') {
        return true;
    }
    file_part
        .split('/')
        .any(|seg| seg == ".." || seg == "." || is_drive(seg))
}

/// Attribute the observed status to the layer that rejected the request.
///
/// Mirrors `serve_room_web_catchall`: the wildcard rest (after `/r/`) is
/// trimmed of leading slashes, then split into room (before the first `/`)
/// and file (after it). Attribution is static (handler logic + matchit
/// matching raw segments with no dot-segment normalization).
fn attribution(path: &str, status: u16) -> String {
    let decoded = percent_decode_once(path);
    let Some(rest) = decoded.strip_prefix("/r/") else {
        return "outside /r/ route".to_string();
    };
    let rest = rest.trim_start_matches('/');
    let (room_part, _file_part) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx + 1..]),
        None => (rest, ""),
    };
    let room_syntax_ok = !room_part.is_empty()
        && room_part.len() <= 64
        && room_part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    match status {
        200 => "200: served room's own index.html via SPA fallback (no out-of-bounds read)".to_string(),
        400 => "400: handler-level ValidatedRelPath rejection (after axum percent-decoding)".to_string(),
        404 if !room_syntax_ok => "404: handler-level ValidatedRoomId rejection".to_string(),
        404 => "404: router-level no-match or store get_file miss".to_string(),
        other => format!("{other}: unexpected status"),
    }
}

/// Recursively collect every file under `root`.
fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

// ── Functional E2E tests ───────────────────────────────────────────────

/// Unauthenticated upload is rejected; an authenticated upload lands on
/// disk and is served back with matching content; SPA fallback semantics
/// are documented as observed.
#[tokio::test]
async fn upload_requires_key_then_roundtrips_to_disk_and_serve() {
    let env = setup(Some(API_KEY.to_string())).await;
    let room = ROOM_ID;

    // No key -> 401; wrong key -> 401.
    let body = upload_body(&[("index.html", INDEX_HTML)]);
    let head = post_head(&format!("/api/rooms/{room}/upload-web"), body.len(), None);
    let resp = raw_http(env.addr, &head, &body).await;
    assert_eq!(resp.status, 401, "upload without key must be rejected");

    let head = post_head(&format!("/api/rooms/{room}/upload-web"), body.len(), Some("wrong-key"));
    let resp = raw_http(env.addr, &head, &body).await;
    assert_eq!(resp.status, 401, "upload with wrong key must be rejected");

    // Correct key -> 200 and the file is written.
    let resp = upload(&env, room, &[("index.html", INDEX_HTML)], Some(API_KEY)).await;
    assert_eq!(resp.status, 200, "authenticated upload must succeed");
    let json: serde_json::Value = serde_json::from_slice(&resp.body).expect("upload JSON body");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["files_written"], 1);

    // File actually landed on disk inside the room dir.
    let disk_file = env.base.join(room).join("index.html");
    assert!(
        disk_file.is_file(),
        "uploaded file must exist on disk: {}",
        disk_file.display()
    );
    assert_eq!(
        std::fs::read(&disk_file).expect("read disk file"),
        INDEX_HTML.as_bytes()
    );

    // Serve roundtrip: exact content back.
    let resp = raw_http(env.addr, &get_head(&format!("/r/{room}/index.html")), "").await;
    assert_eq!(resp.status, 200);
    assert_eq!(String::from_utf8_lossy(&resp.body), INDEX_HTML);

    // SPA fallback semantics (current behavior): room root and any missing
    // in-room path serve the room's own index.html.
    let resp = raw_http(env.addr, &get_head(&format!("/r/{room}/")), "").await;
    assert_eq!(resp.status, 200, "room root serves index (SPA fallback)");
    assert_eq!(String::from_utf8_lossy(&resp.body), INDEX_HTML);

    let resp = raw_http(env.addr, &get_head(&format!("/r/{room}/missing.js")), "").await;
    assert_eq!(
        resp.status, 200,
        "missing path serves index (SPA fallback, documented current semantics)"
    );
    assert_eq!(String::from_utf8_lossy(&resp.body), INDEX_HTML);
}

/// check-web-files reports already-uploaded hashes as existing and
/// unknown hashes as needed.
#[tokio::test]
async fn check_web_files_counts_uploaded_hashes() {
    let env = setup(Some(API_KEY.to_string())).await;
    let room = ROOM_ID;

    let resp = upload(&env, room, &[("index.html", INDEX_HTML)], Some(API_KEY)).await;
    assert_eq!(resp.status, 200, "seed upload");

    // Uploaded hash -> existing_count=1, needed empty.
    let body = check_body(&[("index.html", INDEX_HTML)]);
    let head = post_head(&format!("/api/rooms/{room}/check-web-files"), body.len(), Some(API_KEY));
    let resp = raw_http(env.addr, &head, &body).await;
    assert_eq!(resp.status, 200);
    let json: serde_json::Value = serde_json::from_slice(&resp.body).expect("check JSON body");
    assert_eq!(json["existing_count"], 1);
    assert_eq!(json["total_count"], 1);
    assert_eq!(
        json["needed"],
        serde_json::json!([]),
        "uploaded hash must not be needed"
    );

    // Unknown hash -> needed.
    let body = check_body(&[("new.js", "brand-new-content-here")]);
    let head = post_head(&format!("/api/rooms/{room}/check-web-files"), body.len(), Some(API_KEY));
    let resp = raw_http(env.addr, &head, &body).await;
    assert_eq!(resp.status, 200);
    let json: serde_json::Value = serde_json::from_slice(&resp.body).expect("check JSON body");
    assert_eq!(json["existing_count"], 0);
    assert_eq!(json["needed"], serde_json::json!(["new.js"]));
}

/// WebSocket upgrade auth on the full router: no key / wrong key -> 401,
/// matching key -> 101.
#[tokio::test]
async fn ws_upgrade_requires_api_key_on_full_router() {
    let env = setup(Some(API_KEY.to_string())).await;

    let (status, _stream) = ws_handshake(env.addr, "").await;
    assert!(
        status.starts_with("HTTP/1.1 401"),
        "missing key must 401, got: {status}"
    );

    let (status, _stream) = ws_handshake(env.addr, "X-API-Key: wrong\r\n").await;
    assert!(status.starts_with("HTTP/1.1 401"), "wrong key must 401, got: {status}");

    let (status, _stream) = ws_handshake(env.addr, "X-API-Key: test-key\r\n").await;
    assert!(
        status.starts_with("HTTP/1.1 101"),
        "matching key must upgrade, got: {status}"
    );
}

/// Embedded/open relay (api_key = None): all routes (WebSocket upgrade,
/// upload, check-files, static asset serving) are open without requiring
/// any `x-api-key` auth header. Closes final-review §6 Gap 2 (FR-3).
#[tokio::test]
async fn open_relay_when_api_key_none_accepts_all_routes_without_auth() {
    let env = setup(None).await;
    let room = ROOM_ID;

    // 1. WebSocket upgrade on full router without API key succeeds (101 Switching Protocols).
    let (status, _stream) = ws_handshake(env.addr, "").await;
    assert!(
        status.starts_with("HTTP/1.1 101"),
        "open relay must allow WebSocket upgrade without API key, got: {status}"
    );

    // 2. Upload without API key succeeds (200 OK).
    let resp = upload(&env, room, &[("index.html", INDEX_HTML)], None).await;
    assert_eq!(resp.status, 200, "open relay must accept upload without API key");
    let json: serde_json::Value = serde_json::from_slice(&resp.body).expect("upload JSON body");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["files_written"], 1);

    // 3. Serve static file roundtrip succeeds (200 OK).
    let resp = raw_http(env.addr, &get_head(&format!("/r/{room}/index.html")), "").await;
    assert_eq!(resp.status, 200);
    assert_eq!(String::from_utf8_lossy(&resp.body), INDEX_HTML);

    // 4. check-web-files endpoint without API key succeeds (200 OK).
    let body = check_body(&[("index.html", INDEX_HTML)]);
    let head = post_head(&format!("/api/rooms/{room}/check-web-files"), body.len(), None);
    let resp = raw_http(env.addr, &head, &body).await;
    assert_eq!(resp.status, 200, "open relay must accept check-web-files without API key");
    let json: serde_json::Value = serde_json::from_slice(&resp.body).expect("check JSON body");
    assert_eq!(json["existing_count"], 1);
    assert_eq!(json["total_count"], 1);
    assert_eq!(json["needed"], serde_json::json!([]));
}

/// Raw HTTP/1.1 WebSocket handshake; returns the status line.
async fn ws_handshake(addr: SocketAddr, extra_headers: &str) -> (String, TcpStream) {
    let mut stream = TcpStream::connect(addr).await.expect("connect for handshake");
    let req = format!(
        "GET /ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n{extra_headers}Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await.expect("write handshake");

    let mut resp = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = stream.read(&mut buf).await.expect("read handshake");
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

/// GET on nonexistent rooms and invalid room ids: recorded current
/// semantics (404, no SPA fallback without room files).
#[tokio::test]
async fn get_nonexistent_room_and_invalid_room_ids() {
    let env = setup(None).await;

    // Well-formed room that was never created and has no files: 404.
    let resp = raw_http(env.addr, &get_head("/r/ghost-room/x.js"), "").await;
    assert_eq!(resp.status, 404, "no files -> no index fallback -> 404");

    // Empty rest: 404.
    let resp = raw_http(env.addr, &get_head("/r/"), "").await;
    assert_eq!(resp.status, 404, "empty room id must 404");

    // Room id with characters that fail validation after decoding.
    let resp = raw_http(env.addr, &get_head("/r/room%20with%20space/"), "").await;
    assert_eq!(resp.status, 404, "space in room id must 404");

    let resp = raw_http(env.addr, &get_head("/r/.."), "").await;
    assert_eq!(resp.status, 404, "dotdot room id must 404");
}

// ── V-1 traversal qualification (core deliverable) ─────────────────────

/// Every traversal encoding variant against `/r/{*rest}`:
/// - HARD: the response body never contains the sibling marker content;
/// - HARD: genuine traversal forms (decode to `..`, drive, or root) never
///   get a 200;
/// - recorded: actual status + which layer rejected, printed as a table.
/// - disk-level: nothing under the base dir ever holds the marker.
#[tokio::test]
async fn traversal_variants_never_leak_sibling_marker() {
    let env = setup(Some(API_KEY.to_string())).await;
    let room = ROOM_ID;

    let resp = upload(&env, room, &[("index.html", INDEX_HTML)], Some(API_KEY)).await;
    assert_eq!(resp.status, 200, "seed room with index.html");

    let variants: Vec<(&str, String)> = vec![
        ("literal dotdot", format!("/r/{room}/../secret.txt")),
        ("pct dotdot", format!("/r/{room}/%2e%2e/secret.txt")),
        ("double-pct dotdot", format!("/r/{room}/%252e%252e/secret.txt")),
        ("backslash dotdot", format!("/r/{room}/..\\secret.txt")),
        ("pct backslash", format!("/r/{room}/..%5csecret.txt")),
        ("double-slash absolute", format!("/r/{room}//etc/passwd")),
        ("pct drive letter", format!("/r/{room}/%43%3a%5csecret.txt")),
        ("room-side pct slash", "/r/..%2fsecret.txt/...".to_string()),
        ("room-side literal dotdot", "/r/../secret.txt".to_string()),
    ];

    let mut table: Vec<(String, String, u16, String, bool)> = Vec::new();
    for (name, path) in &variants {
        let resp = raw_http(env.addr, &get_head(path), "").await;
        let leaked = resp.body.windows(MARKER.len()).any(|w| w == MARKER.as_bytes());
        let attr = attribution(path, resp.status);

        // HARD security assertion: the marker never reaches the client.
        assert!(
            !leaked,
            "V-1 FAIL: variant {name:?} ({path}) LEAKED the marker (status {}, body {:?})",
            resp.status,
            String::from_utf8_lossy(&resp.body)
        );

        // HARD: genuine traversal forms must never be served with 200.
        if is_genuine_traversal(path) {
            assert_ne!(
                resp.status, 200,
                "V-1 FAIL: traversal variant {name:?} ({path}) was served with 200"
            );
        } else if resp.status == 200 {
            // Non-traversal literal variant: a 200 may only be the room's
            // own public index.html (SPA fallback), never anything else.
            assert_eq!(
                resp.body,
                INDEX_HTML.as_bytes(),
                "V-1 FAIL: variant {name:?} ({path}) returned 200 with unexpected body"
            );
        }

        table.push((name.to_string(), path.clone(), resp.status, attr, leaked));
    }

    // Raw-backslash probe: does hyper accept a raw `\` in the
    // request-target at all? If yes (404 for an unmatched path), the
    // backslash variant's 400 must come from the handler, not the parser.
    let raw_backslash = variants
        .iter()
        .position(|(name, _)| *name == "backslash dotdot")
        .map(|i| table[i].2 == 400);
    if raw_backslash == Some(true) {
        let parser_probe = raw_http(env.addr, &get_head("/\\"), "").await;
        eprintln!(
            "[probe] raw backslash at unmatched path: status {} (404 => hyper/axum accept it)",
            parser_probe.status
        );
        let handler_probe = raw_http(env.addr, &get_head("/r/no-such-room/..\\x"), "").await;
        eprintln!(
            "[probe] raw backslash + dotdot in catch-all shape: status {} (400 => handler validation)",
            handler_probe.status
        );
    }

    eprintln!("=== V-1 traversal variant behavior table ===");
    for (name, path, status, attr, leaked) in &table {
        eprintln!("{name:<26} status={status:<4} leaked={leaked:<5} {attr}");
        eprintln!("    path: {path}");
    }

    // Disk-level containment: nothing under base ever contains the marker,
    // and the sibling marker still exists untouched.
    let mut files = Vec::new();
    collect_files(&env.base, &mut files);
    for file in &files {
        let data = std::fs::read(file).unwrap_or_default();
        assert!(
            !data.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
            "marker content found under base dir: {}",
            file.display()
        );
        assert_ne!(
            file.file_name().map(|n| n.to_string_lossy().into_owned()),
            Some("secret.txt".to_string()),
            "a secret.txt appeared under base dir"
        );
    }
    assert!(env.marker.is_file(), "sibling marker must still exist");
    assert_eq!(
        std::fs::read(&env.marker).expect("read sibling marker"),
        MARKER.as_bytes()
    );
}
