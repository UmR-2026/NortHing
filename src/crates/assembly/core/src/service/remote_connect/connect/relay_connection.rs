//! Relay connection lifecycle: LAN, ngrok, northhing Server, Custom Server.
//!
//! Carries the `start` and `stop_relay` methods of `RemoteConnectService`.
//! Bot connections (Feishu / Telegram / Weixin) are dispatched from
//! `bot_connection.rs` instead and run independently of relay state.

use tracing::{error, info};

use super::*;
use crate::service::config::get_app_language_code;

impl super::RemoteConnectService {
    /// Start a remote connection with the given method.
    ///
    /// For relay methods (LAN / ngrok / northhing Server / Custom Server) this
    /// tears down any existing relay and starts a new one.
    /// For bot methods, this starts the bot pairing flow without affecting
    /// any running relay connection.
    pub async fn start(&self, method: ConnectionMethod) -> Result<ConnectionResult> {
        info!("Starting remote connect: {method:?}");

        match &method {
            ConnectionMethod::BotFeishu | ConnectionMethod::BotTelegram | ConnectionMethod::BotWeixin => {
                return self.start_bot_connection(&method).await;
            }
            _ => {}
        }

        // Relay methods: clean up previous relay (but leave bots alone)
        self.stop_relay().await;

        let static_dir = self.config.mobile_web_dir.as_deref();

        let relay_url = match &method {
            ConnectionMethod::Lan => {
                let handle = embedded_relay::start_embedded_relay(self.config.lan_port, static_dir).await?;
                *self.embedded_relay.write().await = Some(handle);
                match lan::build_lan_relay_url(self.config.lan_port) {
                    Ok(url) => url,
                    Err(e) => {
                        if let Some(ref mut relay) = *self.embedded_relay.write().await {
                            relay.stop();
                        }
                        *self.embedded_relay.write().await = None;
                        return Err(e);
                    }
                }
            }
            ConnectionMethod::Ngrok => {
                let handle = embedded_relay::start_embedded_relay(self.config.lan_port, static_dir).await?;
                *self.embedded_relay.write().await = Some(handle);

                let tunnel = match ngrok::start_ngrok_tunnel(self.config.lan_port).await {
                    Ok(tunnel) => tunnel,
                    Err(e) => {
                        if let Some(ref mut relay) = *self.embedded_relay.write().await {
                            relay.stop();
                        }
                        *self.embedded_relay.write().await = None;
                        return Err(e);
                    }
                };
                let url = tunnel.public_url.clone();
                *self.ngrok_tunnel.write().await = Some(tunnel);
                url
            }
            ConnectionMethod::NortHingServer => self.config.northhing_server_url.clone(),
            ConnectionMethod::CustomServer { url } => url.clone(),
            // Bot variants are handled earlier (lines 283-290); this branch is for
            // future variants. Refuse explicitly so a new `ConnectionMethod` added
            // without an explicit relay URL strategy fails the call instead of
            // crashing the remote-connect subsystem.
            other => {
                return Err(anyhow::anyhow!(
                    "ConnectionMethod::{other:?} has no relay URL resolution strategy; \
                     add an explicit arm before this fallback"
                ));
            }
        };

        let mut pairing = self.pairing.write().await;
        pairing.reset().await;
        let qr_payload = pairing.initiate(&relay_url).await?;

        let ws_url = match &method {
            ConnectionMethod::Lan | ConnectionMethod::Ngrok => {
                format!("ws://127.0.0.1:{}/ws", self.config.lan_port)
            }
            _ => {
                format!(
                    "{}/ws",
                    relay_url.replace("https://", "wss://").replace("http://", "ws://")
                )
            }
        };

        let (client, mut event_rx) = RelayClient::new();
        client.connect(&ws_url).await?;
        client
            .create_room(
                &self.device_identity.device_id,
                &qr_payload.public_key,
                Some(&qr_payload.room_id),
            )
            .await?;

        let web_app_url: String = match &method {
            ConnectionMethod::Lan | ConnectionMethod::Ngrok => relay_url.clone(),
            ConnectionMethod::NortHingServer => {
                if let Some(web_dir) = static_dir {
                    match sync::upload_mobile_web(&relay_url, &qr_payload.room_id, web_dir).await {
                        Ok(()) => {
                            let url = format!("{}/r/{}", relay_url.trim_end_matches('/'), qr_payload.room_id);
                            info!("Uploaded mobile-web to relay: {url}");
                            url
                        }
                        Err(e) => {
                            error!("Failed to upload mobile-web to relay: {e}; falling back to server-hosted version");
                            self.config.web_app_url.clone()
                        }
                    }
                } else {
                    info!("No mobile_web_dir configured; using server-hosted mobile web");
                    self.config.web_app_url.clone()
                }
            }
            ConnectionMethod::CustomServer { .. } => {
                if let Some(web_dir) = static_dir {
                    match sync::upload_mobile_web(&relay_url, &qr_payload.room_id, web_dir).await {
                        Ok(()) => {
                            let url = format!("{}/r/{}", relay_url.trim_end_matches('/'), qr_payload.room_id);
                            info!("Uploaded mobile-web to custom relay: {url}");
                            url
                        }
                        Err(e) => {
                            error!(
                                "Failed to upload mobile-web to custom relay: {e}; using custom server URL directly"
                            );
                            relay_url.clone()
                        }
                    }
                } else {
                    info!("No mobile_web_dir configured; using custom server URL directly");
                    relay_url.clone()
                }
            }
            _ => self.config.web_app_url.clone(),
        };

        let client_language = get_app_language_code().await;
        let qr_url = QrGenerator::build_url(&qr_payload, &web_app_url, &client_language);
        let qr_svg = QrGenerator::generate_svg_from_url(&qr_url)?;
        let qr_data = QrGenerator::generate_png_base64_from_url(&qr_url)?;

        *self.active_method.write().await = Some(method.clone());
        *self.relay_client.write().await = Some(client);

        let pairing_arc = self.pairing.clone();
        let relay_arc = self.relay_client.clone();
        let server_arc = self.remote_server.clone();
        let trusted_mobile_identity_arc = self.trusted_mobile_identity.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    relay_client::RelayEvent::PairRequest {
                        correlation_id,
                        public_key,
                        device_id,
                        device_name: _,
                    } => {
                        info!("PairRequest from {device_id}");
                        let mut p = pairing_arc.write().await;
                        match p.on_peer_joined(&public_key).await {
                            Ok(challenge) => {
                                if let Some(secret) = p.shared_secret() {
                                    let challenge_json = serde_json::to_string(&challenge).unwrap_or_default();
                                    if let Ok((enc, nonce)) = encryption::encrypt_to_base64(secret, &challenge_json) {
                                        if let Some(ref client) = *relay_arc.read().await {
                                            let _ = client.send_relay_response(&correlation_id, &enc, &nonce).await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Pairing error on pair_request: {e}");
                            }
                        }
                    }
                    relay_client::RelayEvent::CommandReceived {
                        correlation_id,
                        encrypted_data,
                        nonce,
                    } => {
                        super::command::handle_command_event(
                            &correlation_id,
                            &encrypted_data,
                            &nonce,
                            &pairing_arc,
                            &relay_arc,
                            &server_arc,
                            &trusted_mobile_identity_arc,
                        )
                        .await;
                    }
                    relay_client::RelayEvent::Reconnected => {
                        info!("Relay reconnected - pairing + server preserved for mobile polling");
                    }
                    relay_client::RelayEvent::Disconnected => {
                        info!("Relay disconnected");
                        pairing_arc.write().await.disconnect().await;
                        *server_arc.write().await = None;
                    }
                    relay_client::RelayEvent::Error { message } => {
                        error!("Relay error: {message}");
                        if message.contains("Room not found") {
                            info!("Room expired, disconnecting");
                            pairing_arc.write().await.disconnect().await;
                            *server_arc.write().await = None;
                        }
                    }
                    _ => {}
                }
            }
        });

        let state = pairing.state().await;
        Ok(ConnectionResult {
            method,
            qr_data: Some(qr_data),
            qr_svg: Some(qr_svg),
            qr_url: Some(qr_url),
            bot_pairing_code: None,
            bot_link: None,
            pairing_state: state,
        })
    }

    /// Stop relay connections (LAN / ngrok / northhing Server / Custom Server).
    /// Bot connections are left running.
    pub async fn stop_relay(&self) {
        if let Some(ref client) = *self.relay_client.read().await {
            client.disconnect().await;
        }
        *self.relay_client.write().await = None;
        *self.remote_server.write().await = None;
        *self.active_method.write().await = None;

        if let Some(ref mut tunnel) = *self.ngrok_tunnel.write().await {
            tunnel.stop().await;
        }
        *self.ngrok_tunnel.write().await = None;

        if let Some(ref mut relay) = *self.embedded_relay.write().await {
            relay.stop();
        }
        *self.embedded_relay.write().await = None;

        self.pairing.write().await.reset().await;
        *self.trusted_mobile_identity.write().await = None;
        info!("Relay connections stopped (bots unaffected)");
    }
}
