use crate::remote_ssh::RemoteWorkspaceEntry;
use crate::workspace_search::flashgrep::{
    drain_content_length_messages, log_flashgrep_stderr_line_with_context, ProtocolClient,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct RemoteCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Clone)]
pub struct RemoteWorkspaceSearchStdioProtocol {
    protocol: ProtocolClient,
}

impl RemoteWorkspaceSearchStdioProtocol {
    pub(super) fn new(protocol: ProtocolClient) -> Self {
        Self { protocol }
    }

    pub async fn handle_stdout_chunk(&self, read_buffer: &mut Vec<u8>, data: &[u8]) -> Result<(), String> {
        read_buffer.extend_from_slice(data);
        let messages = drain_content_length_messages(read_buffer).map_err(|error| error.to_string())?;
        for message in messages {
            self.protocol.handle_server_message(message).await;
        }
        Ok(())
    }

    pub fn log_stderr_line_with_context(&self, context: Option<&str>, line: &str) {
        log_flashgrep_stderr_line_with_context(context, line);
    }

    pub async fn close_with_message(&self, message: impl Into<String>) {
        self.protocol.close_with_message(message).await;
    }
}

#[async_trait]
pub trait RemoteWorkspaceSearchProvider: Send + Sync {
    async fn resolve_workspace_entry(
        &self,
        root_path: &str,
        preferred_connection_id: Option<&str>,
    ) -> Result<RemoteWorkspaceEntry, String>;

    async fn cached_server_os_type(&self, connection_id: &str) -> Option<String>;

    async fn execute_command(&self, connection_id: &str, command: &str) -> Result<RemoteCommandOutput, String>;

    async fn create_dir_all(&self, connection_id: &str, path: &str) -> Result<(), String>;

    async fn write_file(&self, connection_id: &str, path: &str, contents: &[u8]) -> Result<(), String>;

    async fn repo_max_file_size(&self) -> u64;

    async fn spawn_stdio_daemon(
        &self,
        connection_id: &str,
        command: &str,
        write_rx: mpsc::Receiver<Vec<u8>>,
        protocol: RemoteWorkspaceSearchStdioProtocol,
    ) -> Result<(), String>;
}
