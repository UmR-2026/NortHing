//! Sub-domain: remote command execution.
//! Spec step-3.7 — extracted from remote_connect/mod.rs (R55e refactor).

use super::connect::TrustedMobileIdentity;
use super::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(super) async fn send_pairing_error_response(
    relay_arc: &Arc<RwLock<Option<RelayClient>>>,
    correlation_id: &str,
    shared_secret: &[u8; 32],
    message: String,
) {
    let server = RemoteServer::new(*shared_secret);
    if let Ok((enc, nonce)) = server.encrypt_response(&remote_server::RemoteResponse::Error { message }, None) {
        if let Some(ref client) = *relay_arc.read().await {
            let _ = client.send_relay_response(correlation_id, &enc, &nonce).await;
        }
    }
}

pub(super) async fn handle_command_event(
    correlation_id: &str,
    encrypted_data: &str,
    nonce: &str,
    pairing_arc: &Arc<RwLock<PairingProtocol>>,
    relay_arc: &Arc<RwLock<Option<RelayClient>>>,
    server_arc: &Arc<RwLock<Option<RemoteServer>>>,
    trusted_mobile_identity_arc: &Arc<RwLock<Option<TrustedMobileIdentity>>>,
) {
    let mut handled_as_active_command = false;
    {
        let server_guard = server_arc.read().await;
        if let Some(ref server) = *server_guard {
            match server.decrypt_command(encrypted_data, nonce) {
                Ok((cmd, request_id)) => {
                    handled_as_active_command = true;
                    debug!("Remote command: {cmd:?}");
                    let response = server.dispatch(&cmd).await;
                    match server.encrypt_response(&response, request_id.as_deref()) {
                        Ok((enc, resp_nonce)) => {
                            if let Some(ref client) = *relay_arc.read().await {
                                let _ = client.send_relay_response(correlation_id, &enc, &resp_nonce).await;
                            }
                        }
                        Err(e) => {
                            error!("Failed to encrypt response: {e}");
                        }
                    }
                }
                Err(e) => {
                    debug!("Active session could not decrypt command, falling back to pairing verification: {e}");
                }
            }
        }
    }
    if handled_as_active_command {
        return;
    }

    let p = pairing_arc.read().await;
    if let Some(secret) = p.shared_secret() {
        let shared_secret = *secret;
        if let Ok(json) = encryption::decrypt_from_base64(&shared_secret, encrypted_data, nonce) {
            if let Ok(response) = serde_json::from_str::<pairing::PairingResponse>(&json) {
                let submitted_identity = match RemoteConnectService::validate_mobile_identity(
                    trusted_mobile_identity_arc,
                    &response,
                )
                .await
                {
                    Ok(identity) => identity,
                    Err(message) => {
                        drop(p);
                        send_pairing_error_response(relay_arc, correlation_id, &shared_secret, message).await;
                        return;
                    }
                };
                drop(p);
                let mut pw = pairing_arc.write().await;
                match pw.verify_response(&response).await {
                    Ok(true) => {
                        info!("Pairing verified successfully");
                        RemoteConnectService::persist_mobile_identity(
                            trusted_mobile_identity_arc,
                            submitted_identity.clone(),
                        )
                        .await;
                        if let Some(s) = pw.shared_secret() {
                            let server = RemoteServer::new(*s);

                            let initial_sync = server
                                .generate_initial_sync(Some(submitted_identity.user_id.clone()))
                                .await;
                            if let Ok((enc, resp_nonce)) = server.encrypt_response(&initial_sync, None) {
                                if let Some(ref client) = *relay_arc.read().await {
                                    info!("Sending initial sync to mobile after pairing");
                                    let _ = client.send_relay_response(correlation_id, &enc, &resp_nonce).await;
                                }
                            }

                            *server_arc.write().await = Some(server);
                        }
                    }
                    Ok(false) => {
                        error!("Pairing verification failed");
                        send_pairing_error_response(
                            relay_arc,
                            correlation_id,
                            &shared_secret,
                            "Pairing verification failed".to_string(),
                        )
                        .await;
                    }
                    Err(e) => {
                        error!("Pairing verification error: {e}");
                        send_pairing_error_response(
                            relay_arc,
                            correlation_id,
                            &shared_secret,
                            format!("Pairing verification error: {e}"),
                        )
                        .await;
                    }
                }
            }
        }
    }
}
