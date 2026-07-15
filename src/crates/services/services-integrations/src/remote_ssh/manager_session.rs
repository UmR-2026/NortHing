//! PTY (pseudo-terminal) session for interactive SSH terminals.
//!
//! Split out from `manager.rs` in Round 13 (facade + 3 sub-handlers pattern).
//! `PTYSession` is re-exported by `mod.rs` so cross-crate callers continue to
//! `use northhing_services_integrations::remote_ssh::PTYSession`.

use anyhow::anyhow;
use russh::client::Msg;
use std::sync::Arc;
use tokio::sync::Mutex;

/// PTY session for interactive terminal
#[derive(Clone)]
pub struct PTYSession {
    channel: Arc<Mutex<russh::Channel<Msg>>>,
    connection_id: String,
}

impl PTYSession {
    /// Construct a new PTY session from an open russh channel. Crate-private
    /// so the facade in `manager.rs` can build one without exposing the
    /// internal field layout.
    pub(crate) fn new(channel: russh::Channel<Msg>, connection_id: String) -> Self {
        Self {
            channel: Arc::new(Mutex::new(channel)),
            connection_id,
        }
    }

    /// Extract the inner Channel, consuming the Mutex wrapper.
    /// Only works if this is the sole Arc reference.
    /// Intended for use by RemoteTerminalManager to hand ownership to the owner task.
    pub async fn into_channel(self) -> Option<russh::Channel<Msg>> {
        match Arc::try_unwrap(self.channel) {
            Ok(mutex) => Some(mutex.into_inner()),
            Err(_) => None,
        }
    }
}

impl PTYSession {
    /// Write data to PTY
    pub async fn write(&self, data: &[u8]) -> anyhow::Result<()> {
        let channel = self.channel.lock().await;
        channel
            .data(data)
            .await
            .map_err(|e| anyhow!("Failed to write to PTY: {}", e))?;
        Ok(())
    }

    /// Resize PTY
    pub async fn resize(&self, cols: u32, rows: u32) -> anyhow::Result<()> {
        let channel = self.channel.lock().await;
        // Use default pixel dimensions (80x24 characters)
        channel
            .window_change(cols, rows, 0, 0)
            .await
            .map_err(|e| anyhow!("Failed to resize PTY: {}", e))?;
        Ok(())
    }

    /// Read data from PTY.
    /// Blocks until data is available, PTY closes, or an error occurs.
    /// Returns Ok(Some(bytes)) for data, Ok(None) for clean close, Err for errors.
    pub async fn read(&self) -> anyhow::Result<Option<Vec<u8>>> {
        let mut channel = self.channel.lock().await;
        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Data { data }) => return Ok(Some(data.to_vec())),
                Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                    return Ok(Some(data.to_vec()));
                }
                Some(russh::ChannelMsg::Eof) | Some(russh::ChannelMsg::Close) => return Ok(None),
                Some(russh::ChannelMsg::ExitStatus { .. }) => return Ok(None),
                Some(_) => {
                    // WindowAdjust, Success, RequestSuccess, etc. — skip and keep reading
                    continue;
                }
                None => return Ok(None),
            }
        }
    }

    /// Close PTY session
    pub async fn close(self) -> anyhow::Result<()> {
        let channel = self.channel.lock().await;
        channel.eof().await.map_err(|e| anyhow!("Failed to close PTY: {}", e))?;
        channel
            .close()
            .await
            .map_err(|e| anyhow!("Failed to close channel: {}", e))?;
        Ok(())
    }

    /// Get connection ID
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }
}
