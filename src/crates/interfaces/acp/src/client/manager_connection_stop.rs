// R20c split: ACP client connection stop lifecycle.
// File: src/crates/interfaces/acp/src/client/manager_connection_stop.rs
// Origin: manager_connection.rs (287 canonical lines, QClaw R20a P2 D-deviation
//        +19% over QClaw 242 tolerance)
// Mavis fix: R20c sub-domain B: connection stop lifecycle (3 pub methods).
// R20c sibling: manager_connection_start.rs (sub-domain A)
// R20c sibling: manager_config_loading.rs + manager_config_requirements.rs
// R19 sibling files (consumers of stop methods):
//             manager_session_lifecycle.rs (stop_connection for release_northhing_session)
//             manager_install.rs (... possibly)
//             start_client_connection in manager_connection_start.rs (cleanup_failed_startup for
//                                                                failed-start cleanup)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::AcpClientStatus;
use super::manager_process_lifecycle::terminate_child_process_tree;
use super::AcpClientService;
use northhing_core::util::errors::NortHingResult;
use std::sync::Arc;
use tracing::{info, warn};

impl AcpClientService {
    pub async fn cleanup_failed_startup(self: &Arc<Self>, connection_id: &str) {
        if let Err(error) = self.stop_connection(connection_id).await {
            warn!(
                "Failed to clean up ACP client after startup failure: connection_id={} error={}",
                connection_id, error
            );
        }
    }

    pub async fn stop_client(self: &Arc<Self>, client_id: &str) -> NortHingResult<()> {
        let connection_ids = self
            .clients
            .iter()
            .filter(|entry| entry.value().client_id == client_id)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for connection_id in connection_ids {
            self.stop_connection(&connection_id).await?;
        }
        Ok(())
    }

    pub async fn stop_connection(self: &Arc<Self>, connection_id: &str) -> NortHingResult<()> {
        let Some(client) = self.clients.get(connection_id).map(|entry| entry.clone()) else {
            return Ok(());
        };

        if let Some(tx) = client.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(child) = client.child.lock().await.take() {
            terminate_child_process_tree(connection_id, child).await;
        }
        *client.connection.write().await = None;
        *client.agent_capabilities.write().await = None;
        client.sessions.clear();
        client.cancel_handles.clear();
        *client.status.write().await = AcpClientStatus::Stopped;
        self.clients.remove(connection_id);
        info!(
            "ACP client stopped: id={} client_id={}",
            connection_id, client.client_id
        );
        Ok(())
    }
}
