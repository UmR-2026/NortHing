use super::*;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::validated::{ContentHash, ValidatedRelPath, ValidatedRoomId};
use crate::MemoryAssetStore;
use crate::WebAssetStore;
use base64::Engine;

fn make_state(store: Arc<dyn WebAssetStore>) -> AppState {
    AppState {
        room_manager: crate::relay::RoomManager::new(),
        start_time: std::time::Instant::now(),
        asset_store: store,
        api_key: None,
        ws_idle_timeout: std::time::Duration::from_secs(90),
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
    let (tx, _rx) = mpsc::channel(256);
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
    let res = check_web_files(
        State(state),
        Path("my-room".to_string()),
        AuthExtractor { api_key: None },
        body,
    )
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
    let res = check_web_files(
        State(state),
        Path("r".to_string()),
        AuthExtractor { api_key: None },
        body,
    )
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
    let res = check_web_files(
        State(state),
        Path("..".to_string()),
        AuthExtractor { api_key: None },
        body,
    )
    .await;
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
    let res = check_web_files(
        State(state),
        Path("r".to_string()),
        AuthExtractor { api_key: None },
        body,
    )
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
    let res = upload_web(
        State(state),
        Path("r".to_string()),
        AuthExtractor { api_key: None },
        body,
    )
    .await;
    assert!(matches!(res, Err(StatusCode::BAD_REQUEST)));
}

#[tokio::test]
async fn serve_catchall_rejects_invalid_rel_path() {
    let state = make_state(Arc::new(MemoryAssetStore::new()));
    let res = serve_room_web_catchall(State(state), Path("r/../x".to_string())).await;
    assert!(matches!(res, Err(StatusCode::BAD_REQUEST)));
}

// ── Upload-route authentication (C-1) ───────────────────────────────

/// With `api_key` configured, all three upload routes reject requests
/// without a matching key with 401 before touching any state.
#[tokio::test]
async fn upload_routes_reject_missing_api_key_when_configured() {
    let mut state = make_state(Arc::new(MemoryAssetStore::new()));
    state.api_key = Some("secret".to_string());
    ensure_room(&state.room_manager, "r");
    let no_key = AuthExtractor { api_key: None };

    let res = upload_web(
        State(state.clone()),
        Path("r".to_string()),
        no_key.clone(),
        Json(UploadWebRequest { files: HashMap::new() }),
    )
    .await;
    assert!(matches!(res, Err(StatusCode::UNAUTHORIZED)));

    let res = check_web_files(
        State(state.clone()),
        Path("r".to_string()),
        no_key.clone(),
        Json(CheckWebFilesRequest { files: vec![] }),
    )
    .await;
    assert!(matches!(res, Err(StatusCode::UNAUTHORIZED)));

    let res = upload_web_files(
        State(state.clone()),
        Path("r".to_string()),
        no_key,
        Json(UploadWebFilesRequest { files: HashMap::new() }),
    )
    .await;
    assert!(matches!(res, Err(StatusCode::UNAUTHORIZED)));
}

/// With `api_key` configured, a matching key passes; without a key
/// configured (`None`), uploads stay open (embedded relay / dev).
#[tokio::test]
async fn upload_routes_accept_valid_api_key_and_stay_open_when_unset() {
    let mut state = make_state(Arc::new(MemoryAssetStore::new()));
    state.api_key = Some("secret".to_string());
    ensure_room(&state.room_manager, "r");
    let with_key = AuthExtractor {
        api_key: Some("secret".to_string()),
    };

    let res = upload_web(
        State(state.clone()),
        Path("r".to_string()),
        with_key.clone(),
        Json(UploadWebRequest { files: HashMap::new() }),
    )
    .await;
    assert!(res.is_ok(), "matching key must be accepted");

    let res = check_web_files(
        State(state.clone()),
        Path("r".to_string()),
        with_key.clone(),
        Json(CheckWebFilesRequest { files: vec![] }),
    )
    .await;
    assert!(res.is_ok(), "matching key must be accepted");

    let res = upload_web_files(
        State(state.clone()),
        Path("r".to_string()),
        with_key,
        Json(UploadWebFilesRequest { files: HashMap::new() }),
    )
    .await;
    assert!(res.is_ok(), "matching key must be accepted");

    // api_key = None (embedded relay): open, no key needed.
    let open_state = make_state(Arc::new(MemoryAssetStore::new()));
    ensure_room(&open_state.room_manager, "r");
    let res = upload_web(
        State(open_state),
        Path("r".to_string()),
        AuthExtractor { api_key: None },
        Json(UploadWebRequest { files: HashMap::new() }),
    )
    .await;
    assert!(res.is_ok(), "api_key=None must keep uploads open");
}
