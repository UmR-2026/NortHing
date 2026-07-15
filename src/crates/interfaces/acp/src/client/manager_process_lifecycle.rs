// R19 split: ACP child process tree management (terminate, configure, wait-for-connection).
// File: src/crates/interfaces/acp/src/client/manager_process_lifecycle.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_session_helpers.rs
//             manager_errors.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode, AcpClientRequirementProbe,
    AcpClientStatus, RemoteAcpClientRequirementSnapshot,
};
use super::manager::AcpClientConnection;
use super::manager::CLIENT_STARTUP_TIMEOUT;
use super::manager_errors::startup_timeout_error;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

pub async fn wait_for_client_connection(client: Arc<AcpClientConnection>, connection_id: &str) -> NortHingResult<()> {
    let started_at = Instant::now();
    loop {
        if client.connection.read().await.is_some() {
            return Ok(());
        }

        let status = *client.status.read().await;
        if matches!(status, AcpClientStatus::Failed | AcpClientStatus::Stopped) {
            return Err(NortHingError::service(format!(
                "ACP client '{}' is not running",
                connection_id
            )));
        }

        if started_at.elapsed() >= CLIENT_STARTUP_TIMEOUT {
            return Err(startup_timeout_error(&client.client_id, "initialize"));
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = command;
    }
}

pub async fn terminate_child_process_tree(client_id: &str, mut child: Child) {
    let pid = child.id();

    #[cfg(unix)]
    if let Some(pid) = pid {
        let process_group = format!("-{}", pid);
        match northhing_core::util::process_manager::create_tokio_command("kill")
            .arg("-TERM")
            .arg(&process_group)
            .status()
            .await
        {
            Ok(status) if status.success() => {}
            Ok(status) => {
                warn!(
                    "ACP client process group terminate exited unsuccessfully: id={} pid={} status={}",
                    client_id, pid, status
                );
            }
            Err(error) => {
                warn!(
                    "Failed to terminate ACP client process group: id={} pid={} error={}",
                    client_id, pid, error
                );
            }
        }

        match tokio::time::timeout(Duration::from_millis(750), child.wait()).await {
            Ok(Ok(_)) => return,
            Ok(Err(error)) => {
                warn!(
                    "Failed to wait for ACP client process after terminate: id={} pid={} error={}",
                    client_id, pid, error
                );
            }
            Err(_) => {}
        }

        if let Err(error) = northhing_core::util::process_manager::create_tokio_command("kill")
            .arg("-KILL")
            .arg(&process_group)
            .status()
            .await
        {
            warn!(
                "Failed to kill ACP client process group: id={} pid={} error={}",
                client_id, pid, error
            );
        }
        let _ = child.wait().await;
        return;
    }

    #[cfg(windows)]
    if let Some(pid) = pid {
        match northhing_core::util::process_manager::create_tokio_command("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/T")
            .arg("/F")
            .status()
            .await
        {
            Ok(status) if status.success() => {
                let _ = child.wait().await;
                return;
            }
            Ok(status) => {
                warn!(
                    "ACP client process tree kill exited unsuccessfully: id={} pid={} status={}",
                    client_id, pid, status
                );
            }
            Err(error) => {
                warn!(
                    "Failed to kill ACP client process tree: id={} pid={} error={}",
                    client_id, pid, error
                );
            }
        }
    }

    if let Err(error) = child.start_kill() {
        warn!("Failed to kill ACP client process: id={} error={}", client_id, error);
    }
    let _ = child.wait().await;
}
