// R20c split: ACP client connection start lifecycle.
// File: src/crates/interfaces/acp/src/client/manager_connection_start.rs
// Origin: manager_connection.rs (287 canonical lines, QClaw R20a P2 D-deviation
//        +19% over QClaw 242 tolerance)
// Mavis fix: R20c split manager_connection.rs into 2 files (this +
//        manager_connection_stop.rs) to close the 242 line cap. Sub-domain A:
//        connection start lifecycle (3 pub methods, includes the
//        147-line start_client_connection QClaw flagged in R20a review).
// R20c sibling: manager_connection_stop.rs (sub-domain B)
// R20c sibling: manager_config_loading.rs + manager_config_requirements.rs
// R19 sibling files (consumers of start methods):
//             manager_session.rs (start_client_for_session for session init)
//             manager_install.rs (initialize_all for fresh install)
//             manager_prompt.rs (... possibly)
//
// All method bodies are moved verbatim from main. No behavior change.

use super::config::AcpClientStatus;
use super::manager::{AcpClientConnection, StartClientConfig, CLIENT_STARTUP_TIMEOUT, CLIENT_STARTUP_TIMEOUT_SECS};
use super::manager_errors::startup_timeout_error;
use super::manager_process_lifecycle::{terminate_child_process_tree, wait_for_client_connection};
use super::manager_session_helpers_identity::session_client_connection_id;
use super::AcpClientService;
use agent_client_protocol::schema::{
    ClientCapabilities, Implementation, InitializeRequest, ProtocolVersion, RequestPermissionRequest,
};
use agent_client_protocol::{Client, ConnectionTo};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{info, warn};

impl AcpClientService {
    pub async fn initialize_all(self: &Arc<Self>) -> NortHingResult<()> {
        let configs = self.load_configs().await?;
        self.register_configured_tools(&configs).await;

        let configured_ids = configs.keys().cloned().collect::<std::collections::HashSet<_>>();
        let running_connections = self
            .clients
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().client_id.clone()))
            .collect::<Vec<_>>();
        for (connection_id, client_id) in running_connections {
            let should_stop = !configured_ids.contains(&client_id)
                || configs.get(&client_id).map(|config| !config.enabled).unwrap_or(true);
            if should_stop {
                let _ = self.stop_connection(&connection_id).await;
            }
        }

        Ok(())
    }

    pub async fn start_client_for_session(
        self: &Arc<Self>,
        client_id: &str,
        northhing_session_id: &str,
        workspace_path: Option<&str>,
        remote_connection_id: Option<&str>,
    ) -> NortHingResult<()> {
        let connection_id = session_client_connection_id(client_id, northhing_session_id);
        self.start_client_connection(&connection_id, client_id, workspace_path, remote_connection_id)
            .await
    }

    pub async fn start_client_connection(
        self: &Arc<Self>,
        connection_id: &str,
        client_id: &str,
        workspace_path: Option<&str>,
        remote_connection_id: Option<&str>,
    ) -> NortHingResult<()> {
        if let Some(existing) = self.clients.get(connection_id).map(|entry| entry.clone()) {
            let status = *existing.status.read().await;
            if matches!(status, AcpClientStatus::Running) {
                return Ok(());
            }
            if matches!(status, AcpClientStatus::Starting) {
                return wait_for_client_connection(existing, connection_id).await;
            }
        }

        let StartClientConfig {
            remote_connection_id,
            config,
        } = self
            .resolve_start_client_config(client_id, workspace_path, remote_connection_id)
            .await?;

        let connection = Arc::new(AcpClientConnection::new(
            connection_id.to_string(),
            client_id.to_string(),
            config,
        ));
        self.clients.insert(connection_id.to_string(), connection.clone());
        *connection.status.write().await = AcpClientStatus::Starting;

        let (transport, child) = match remote_connection_id {
            Some(ref remote_connection_id) => {
                self.open_transport_for_connection(
                    client_id,
                    connection_id,
                    &connection.config,
                    workspace_path,
                    Some(remote_connection_id.as_str()),
                )
                .await
            }
            None => {
                self.open_transport_for_connection(client_id, connection_id, &connection.config, workspace_path, None)
                    .await
            }
        }
        .inspect_err(|_error| {
            self.clients.remove(connection_id);
        })?;
        *connection.child.lock().await = child;
        let service = self.clone();
        let connection_for_task = connection.clone();
        let (cx_tx, cx_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        *connection.shutdown_tx.lock().await = Some(shutdown_tx);

        let connect_task = tokio::spawn(async move {
            let result = Client
                .builder()
                .name("northhing-acp-client")
                .on_receive_request(
                    {
                        let service = service.clone();
                        async move |request: RequestPermissionRequest, responder, cx| {
                            let service = service.clone();
                            cx.spawn(async move {
                                responder.respond_with_result(service.handle_permission_request(request).await)
                            })?;
                            Ok(())
                        }
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .connect_with(transport, async move |cx| {
                    let init = InitializeRequest::new(ProtocolVersion::V1)
                        .client_capabilities(ClientCapabilities::new())
                        .client_info(Implementation::new("northhing-desktop", env!("CARGO_PKG_VERSION")));
                    let initialize_response = cx.send_request(init).block_task().await?;
                    let _ = cx_tx.send((cx, initialize_response.agent_capabilities));
                    let _ = shutdown_rx.await;
                    Ok(())
                })
                .await;

            if let Err(error) = result {
                warn!(
                    "ACP client connection ended with error: id={} error={:?}",
                    connection_for_task.id, error
                );
                *connection_for_task.status.write().await = AcpClientStatus::Failed;
            } else {
                *connection_for_task.status.write().await = AcpClientStatus::Stopped;
            }
            *connection_for_task.connection.write().await = None;
            *connection_for_task.agent_capabilities.write().await = None;
            connection_for_task.sessions.clear();
        });

        let (cx, agent_capabilities) = match tokio::time::timeout(CLIENT_STARTUP_TIMEOUT, cx_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                connect_task.abort();
                self.cleanup_failed_startup(connection_id).await;
                return Err(NortHingError::service(format!(
                    "ACP client '{}' exited before initialization completed",
                    client_id
                )));
            }
            Err(_) => {
                warn!(
                    "ACP client startup timed out during initialize: id={} connection_id={} timeout_secs={}",
                    client_id, connection_id, CLIENT_STARTUP_TIMEOUT_SECS
                );
                connect_task.abort();
                self.cleanup_failed_startup(connection_id).await;
                return Err(startup_timeout_error(client_id, "initialize"));
            }
        };
        *connection.connection.write().await = Some(cx);
        *connection.agent_capabilities.write().await = Some(agent_capabilities);
        *connection.status.write().await = AcpClientStatus::Running;
        info!(
            "ACP client started: id={} remote_connection_id={}",
            client_id,
            remote_connection_id.as_deref().unwrap_or("")
        );
        Ok(())
    }
}
