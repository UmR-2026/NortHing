//! Contract tests for workspace re-exports on the runtime-ports facade.
//!
//! R39d sibling: split facade-test bulk from lib.rs (workspace).

use crate::*;

#[test]
fn remote_control_state_snapshot_serializes_active_turn_contract() {
    let snapshot = RemoteControlStateSnapshot {
        session_id: "session_1".to_string(),
        state: RemoteControlSessionState::Processing,
        active_turn_id: Some("turn_1".to_string()),
        queue_depth: 2,
        metadata: serde_json::Map::new(),
    };

    let json = serde_json::to_value(snapshot).expect("serialize state snapshot");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["state"], "processing");
    assert_eq!(json["activeTurnId"], "turn_1");
    assert_eq!(json["queueDepth"], 2);
}

#[test]
fn session_transcript_request_serializes_turn_id_contract() {
    let request = SessionTranscriptRequest {
        session_id: "session_1".to_string(),
        turn_id: Some("turn_1".to_string()),
    };

    let json = serde_json::to_value(request).expect("serialize transcript request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["turnId"], "turn_1");
    assert!(json.get("fromTurnId").is_none());
}

#[derive(Debug)]
struct FakeWorkspaceFileSystem;

#[async_trait::async_trait]
impl WorkspaceFileSystem for FakeWorkspaceFileSystem {
    async fn read_file(&self, _path: &str) -> anyhow::Result<Vec<u8>> {
        Ok(b"hello".to_vec())
    }

    async fn read_file_text(&self, _path: &str) -> anyhow::Result<String> {
        Ok("hello".to_string())
    }

    async fn write_file(&self, _path: &str, _contents: &[u8]) -> anyhow::Result<()> {
        Ok(())
    }

    async fn exists(&self, _path: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn is_file(&self, _path: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn is_dir(&self, _path: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn read_dir(&self, _path: &str) -> anyhow::Result<Vec<WorkspaceDirEntry>> {
        Ok(vec![WorkspaceDirEntry {
            name: "file.txt".to_string(),
            path: "/workspace/file.txt".to_string(),
            is_dir: false,
            is_symlink: false,
        }])
    }
}

#[derive(Debug)]
struct FakeWorkspaceShell;

#[async_trait::async_trait]
impl WorkspaceShell for FakeWorkspaceShell {
    async fn exec_with_options(
        &self,
        _command: &str,
        options: WorkspaceCommandOptions,
    ) -> anyhow::Result<WorkspaceCommandResult> {
        assert_eq!(options.timeout_ms, Some(100));
        assert!(options.cancellation_token.is_none());
        Ok(WorkspaceCommandResult {
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: 0,
            interrupted: false,
            timed_out: false,
        })
    }
}

#[test]
fn workspace_services_contract_is_runtime_port_owned() {
    let services = WorkspaceServices {
        fs: std::sync::Arc::new(FakeWorkspaceFileSystem),
        shell: std::sync::Arc::new(FakeWorkspaceShell),
    };

    let cloned = services.clone();
    assert!(std::sync::Arc::ptr_eq(&services.fs, &cloned.fs));
    assert!(std::sync::Arc::ptr_eq(&services.shell, &cloned.shell));
    assert_eq!(
        format!("{:?}", services),
        "WorkspaceServices { fs: \"<dyn WorkspaceFileSystem>\", shell: \"<dyn WorkspaceShell>\" }"
    );
}
#[test]
fn tool_runtime_handles_keep_workspace_services_and_cancellation_contracts() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let services = WorkspaceServices {
        fs: std::sync::Arc::new(FakeWorkspaceFileSystem),
        shell: std::sync::Arc::new(FakeWorkspaceShell),
    };

    let handles = ToolRuntimeHandles::new(Some(services.clone()), Some(cancellation_token.clone()));

    assert!(handles.cancellation_token().is_some());
    assert!(handles.workspace_services().is_some());
    assert!(std::sync::Arc::ptr_eq(
        &services.fs,
        &handles.workspace_services().expect("workspace services").fs
    ));

    let cloned = handles.clone();
    assert!(cloned.cancellation_token().is_some());
    assert!(std::sync::Arc::ptr_eq(
        &services.shell,
        &cloned.workspace_services().expect("workspace services").shell
    ));
    assert_eq!(
        format!("{:?}", handles),
        "ToolRuntimeHandles { workspace_services: Some(\"<WorkspaceServices>\"), cancellation_token: Some(\"<CancellationToken>\") }"
    );
}
