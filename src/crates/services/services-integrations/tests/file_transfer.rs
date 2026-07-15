//! File Transfer contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[test]
fn remote_connect_file_transfer_policy_preserves_limits_and_chunk_ranges() {
    assert_eq!(REMOTE_FILE_MAX_READ_BYTES, 30 * 1024 * 1024);
    assert_eq!(REMOTE_FILE_MAX_CHUNK_BYTES, 3 * 1024 * 1024);
    assert_eq!(REMOTE_FILE_MAX_CHUNK_BYTES % 3, 0);

    let range = resolve_remote_file_chunk_range(10_000_000, 5, REMOTE_FILE_MAX_CHUNK_BYTES + 99);
    assert_eq!(range.start, 5);
    assert_eq!(range.end, 5 + REMOTE_FILE_MAX_CHUNK_BYTES as usize);
    assert_eq!(range.chunk_size, REMOTE_FILE_MAX_CHUNK_BYTES);

    let tail = resolve_remote_file_chunk_range(100, 95, 30);
    assert_eq!(tail.start, 95);
    assert_eq!(tail.end, 100);
    assert_eq!(tail.chunk_size, 5);

    let past_end = resolve_remote_file_chunk_range(100, 150, 30);
    assert_eq!(past_end.start, 100);
    assert_eq!(past_end.end, 100);
    assert_eq!(past_end.chunk_size, 0);
}

#[test]
fn remote_connect_file_transfer_policy_preserves_name_fallback() {
    assert_eq!(remote_file_display_name(Some("report.md")), "report.md");
    assert_eq!(remote_file_display_name(None), "file");
    assert_eq!(remote_file_display_name(Some("")), "file");
}

#[test]
fn remote_connect_file_path_resolution_stays_within_workspace_root() {
    let (base, workspace, report) = make_temp_remote_workspace();

    let resolved = resolve_remote_workspace_path("computer://artifacts/report.md", Some(&workspace))
        .expect("workspace-relative file resolves");
    assert_eq!(resolved, report.canonicalize().expect("canonical report"));

    assert!(resolve_remote_workspace_path("../secret.md", Some(&workspace)).is_none());
    assert!(resolve_remote_workspace_path("artifacts/report.md", None).is_none());

    std::fs::remove_dir_all(base).expect("cleanup remote workspace");
}

#[tokio::test]
async fn remote_connect_file_read_helpers_preserve_current_wire_inputs() {
    let (base, workspace, report) = make_temp_remote_workspace();

    let content = read_remote_workspace_file(
        "computer://artifacts/report.md",
        REMOTE_FILE_MAX_READ_BYTES,
        Some(&workspace),
    )
    .await
    .expect("read remote file");

    assert_eq!(content.name, "report.md");
    assert_eq!(content.bytes, b"hello remote file");
    assert_eq!(content.mime_type, "text/markdown");
    assert_eq!(content.size, 17);

    let err = read_remote_workspace_file("computer://artifacts/report.md", 3, Some(&workspace))
        .await
        .expect_err("size limit rejects large file");
    assert!(err.contains("File too large"));
    assert!(err.contains(&report.display().to_string()));

    std::fs::remove_dir_all(base).expect("cleanup remote workspace");
}

#[tokio::test]
async fn remote_connect_file_chunk_and_info_helpers_preserve_response_facts() {
    let (base, workspace, _report) = make_temp_remote_workspace();

    let chunk = read_remote_workspace_file_chunk("computer://artifacts/report.md", Some(&workspace), 6, 99)
        .await
        .expect("read remote file chunk");

    assert_eq!(chunk.name, "report.md");
    assert_eq!(chunk.bytes, b"remote file");
    assert_eq!(chunk.offset, 6);
    assert_eq!(chunk.chunk_size, 11);
    assert_eq!(chunk.total_size, 17);
    assert_eq!(chunk.mime_type, "text/markdown");

    let info = read_remote_workspace_file_info("computer://artifacts/report.md", Some(&workspace))
        .await
        .expect("read remote file info");

    assert_eq!(info.name, "report.md");
    assert_eq!(info.size, 17);
    assert_eq!(info.mime_type, "text/markdown");

    std::fs::remove_dir_all(base).expect("cleanup remote workspace");
}

#[test]
fn remote_connect_file_response_assembly_owns_base64_wire_shape() {
    let content_response = remote_file_content_response(Ok(RemoteWorkspaceFileContent {
        name: "report.md".to_string(),
        bytes: b"hello remote file".to_vec(),
        mime_type: "text/markdown",
        size: 17,
    }));
    let content_json = serde_json::to_value(content_response).expect("serialize file content");

    assert_eq!(content_json["resp"], "file_content");
    assert_eq!(content_json["name"], "report.md");
    assert_eq!(content_json["content_base64"], "aGVsbG8gcmVtb3RlIGZpbGU=");
    assert_eq!(content_json["mime_type"], "text/markdown");
    assert_eq!(content_json["size"], 17);

    let chunk_response = remote_file_chunk_response(Ok(RemoteWorkspaceFileChunk {
        name: "report.md".to_string(),
        bytes: b"remote file".to_vec(),
        offset: 6,
        chunk_size: 11,
        total_size: 17,
        mime_type: "text/markdown",
    }));
    let chunk_json = serde_json::to_value(chunk_response).expect("serialize file chunk");

    assert_eq!(chunk_json["resp"], "file_chunk");
    assert_eq!(chunk_json["chunk_base64"], "cmVtb3RlIGZpbGU=");
    assert_eq!(chunk_json["offset"], 6);
    assert_eq!(chunk_json["chunk_size"], 11);
    assert_eq!(chunk_json["total_size"], 17);

    let info_response = remote_file_info_response(Ok(RemoteWorkspaceFileInfo {
        name: "report.md".to_string(),
        size: 17,
        mime_type: "text/markdown",
    }));
    let info_json = serde_json::to_value(info_response).expect("serialize file info");

    assert_eq!(info_json["resp"], "file_info");
    assert_eq!(info_json["name"], "report.md");
    assert_eq!(info_json["mime_type"], "text/markdown");

    let err_json =
        serde_json::to_value(remote_file_info_response(Err("missing file".to_string()))).expect("serialize file error");
    assert_eq!(err_json["resp"], "error");
    assert_eq!(err_json["message"], "missing file");
}

#[tokio::test]
async fn remote_connect_file_command_handler_owns_owner_flow_and_uses_host_root() {
    let (base, workspace, _report) = make_temp_remote_workspace();
    let host = RecordingFileHost {
        workspace_root: workspace,
        seen_sessions: Mutex::new(Vec::new()),
    };

    let response = handle_remote_workspace_file_command(
        &host,
        &RemoteCommand::ReadFile {
            path: "computer://artifacts/report.md".to_string(),
            session_id: Some("session-1".to_string()),
        },
    )
    .await;
    let json = serde_json::to_value(response).expect("serialize read response");

    assert_eq!(json["resp"], "file_content");
    assert_eq!(json["content_base64"], "aGVsbG8gcmVtb3RlIGZpbGU=");
    assert_eq!(
        host.seen_sessions.lock().unwrap().as_slice(),
        &[Some("session-1".to_string())]
    );

    let error = handle_remote_workspace_file_command(&host, &RemoteCommand::Ping).await;
    assert_eq!(
        error,
        RemoteResponse::Error {
            message: "Unsupported remote workspace file command".to_string()
        }
    );

    std::fs::remove_dir_all(base).expect("cleanup remote workspace");
}
