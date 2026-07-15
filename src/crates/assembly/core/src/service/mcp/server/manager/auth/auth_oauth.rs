use super::super::MCPServerManager;
use super::auth_types::OAuthCallbackAppState;
use crate::service::config::app_language::get_app_language_code;
use crate::service::mcp::auth::{
    map_auth_error, prepare_remote_oauth_authorization, MCPRemoteOAuthSessionSnapshot, MCPRemoteOAuthStatus,
};
use crate::service::mcp::server::MCPServerType;
use crate::util::errors::{NortHingError, NortHingResult};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{timeout, Duration};
use tracing::warn;

const OAUTH_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

// ---------------------------------------------------------------------------
// Public OAuth flow entry points
// ---------------------------------------------------------------------------

impl MCPServerManager {
    /// Initiates a remote OAuth authorization flow for `server_id`.
    ///
    /// Spawns a local callback listener and waits for the provider to redirect
    /// back (or for the user to cancel / timeout).
    pub async fn start_remote_oauth_authorization(
        &self,
        server_id: &str,
    ) -> NortHingResult<MCPRemoteOAuthSessionSnapshot> {
        let config = self
            .config_service
            .get_server_config(server_id)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("MCP server config not found: {}", server_id)))?;

        if config.server_type != MCPServerType::Remote {
            return Err(NortHingError::Validation(format!(
                "MCP server '{}' is not a remote server",
                server_id
            )));
        }

        if let Some(existing) = self.oauth_sessions.write().await.remove(server_id) {
            MCPServerManager::shutdown_oauth_session(&existing).await;
        }

        let prepared = prepare_remote_oauth_authorization(&config).await?;
        let callback_path = reqwest::Url::parse(&prepared.redirect_uri)
            .map_err(|error| {
                NortHingError::MCPError(format!(
                    "Invalid OAuth redirect URI for server '{}': {}",
                    server_id, error
                ))
            })?
            .path()
            .to_string();

        let initial_snapshot = MCPRemoteOAuthSessionSnapshot::new(
            server_id.to_string(),
            MCPRemoteOAuthStatus::AwaitingBrowser,
            Some(prepared.authorization_url.clone()),
            Some(prepared.redirect_uri.clone()),
            Some("Open the authorization URL to continue OAuth sign-in.".to_string()),
        );

        let (callback_tx, callback_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let session = Arc::new(super::ActiveRemoteOAuthSession {
            snapshot: Arc::new(tokio::sync::RwLock::new(initial_snapshot.clone())),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        });

        if let Some(previous) = self.insert_oauth_session(server_id, session.clone()).await {
            MCPServerManager::shutdown_oauth_session(&previous).await;
        }

        let callback_state = OAuthCallbackAppState {
            callback_tx: Arc::new(Mutex::new(Some(callback_tx))),
            preferred_language: get_app_language_code().await,
        };
        let router = axum::Router::new()
            .route(&callback_path, axum::routing::get(handle_oauth_callback))
            .with_state(callback_state);
        let callback_server_session = session.clone();
        let callback_server_id = server_id.to_string();
        tokio::spawn(async move {
            let server = axum::serve(prepared.listener, router).with_graceful_shutdown(async move {
                if let Err(e) = shutdown_rx.await {
                    warn!("OAuth callback server shutdown receiver already closed: {e}");
                }
            });

            if let Err(error) = server.await {
                let _ = MCPServerManager::update_oauth_snapshot(&callback_server_session, |snapshot| {
                    if matches!(
                        snapshot.status,
                        MCPRemoteOAuthStatus::Authorized | MCPRemoteOAuthStatus::Cancelled
                    ) {
                        return;
                    }
                    snapshot.status = MCPRemoteOAuthStatus::Failed;
                    snapshot.message = Some(format!(
                        "OAuth callback listener failed for server '{}': {}",
                        callback_server_id, error
                    ));
                })
                .await;
            }
        });

        let manager = self.clone();
        let callback_session = session.clone();
        let callback_server_id = server_id.to_string();
        let authorization_url = prepared.authorization_url.clone();
        let redirect_uri = prepared.redirect_uri.clone();
        let mut oauth_state = prepared.state;
        tokio::spawn(async move {
            let _ = MCPServerManager::update_oauth_snapshot(&callback_session, |snapshot| {
                snapshot.status = MCPRemoteOAuthStatus::AwaitingCallback;
                snapshot.message = Some("Waiting for the OAuth provider to redirect back to northhing.".to_string());
            })
            .await;

            let callback = match timeout(OAUTH_CALLBACK_TIMEOUT, callback_rx).await {
                Ok(Ok(callback)) => callback,
                Ok(Err(_)) => {
                    let _ = MCPServerManager::update_oauth_snapshot(&callback_session, |snapshot| {
                        snapshot.status = MCPRemoteOAuthStatus::Cancelled;
                        snapshot.message = Some("OAuth authorization was cancelled.".to_string());
                    })
                    .await;
                    MCPServerManager::shutdown_oauth_session(&callback_session).await;
                    return;
                }
                Err(_) => {
                    let _ = MCPServerManager::fail_oauth_session(
                        &callback_session,
                        "OAuth authorization timed out before the provider redirected back.".to_string(),
                    )
                    .await;
                    return;
                }
            };

            if let Some(error) = callback.error {
                let description = callback
                    .error_description
                    .map(|value| format!(": {}", value))
                    .unwrap_or_default();
                let _ = MCPServerManager::fail_oauth_session(
                    &callback_session,
                    format!("OAuth provider returned '{}{}'", error, description),
                )
                .await;
                return;
            }

            let code = match callback.code {
                Some(code) => code,
                None => {
                    let _ = MCPServerManager::fail_oauth_session(
                        &callback_session,
                        "OAuth callback did not include an authorization code.".to_string(),
                    )
                    .await;
                    return;
                }
            };

            let state = match callback.state {
                Some(state) => state,
                None => {
                    let _ = MCPServerManager::fail_oauth_session(
                        &callback_session,
                        "OAuth callback did not include a state token.".to_string(),
                    )
                    .await;
                    return;
                }
            };

            let _ = MCPServerManager::update_oauth_snapshot(&callback_session, |snapshot| {
                snapshot.status = MCPRemoteOAuthStatus::ExchangingToken;
                snapshot.message = Some("Exchanging the authorization code for an access token.".to_string());
            })
            .await;

            match oauth_state.handle_callback(&code, &state).await {
                Ok(_) => {
                    let _ = MCPServerManager::set_oauth_snapshot(
                        &callback_session,
                        MCPRemoteOAuthSessionSnapshot::new(
                            callback_server_id.clone(),
                            MCPRemoteOAuthStatus::Authorized,
                            Some(authorization_url.clone()),
                            Some(redirect_uri.clone()),
                            Some("OAuth authorization completed. Reconnecting MCP server.".to_string()),
                        ),
                    )
                    .await;

                    if let Some(shutdown_tx) = callback_session.shutdown_tx.lock().await.take() {
                        if let Err(e) = shutdown_tx.send(()) {
                            warn!("Failed to send shutdown signal after OAuth success: {e:?}");
                        }
                    }

                    manager.clear_reconnect_state(&callback_server_id).await;
                    if let Err(e) = manager.stop_server(&callback_server_id).await {
                        warn!("Failed to stop server before OAuth reconnect: {e}");
                    }
                    if let Err(error) = manager.start_server(&callback_server_id).await {
                        let _ = MCPServerManager::update_oauth_snapshot(&callback_session, |snapshot| {
                            snapshot.message = Some(format!("OAuth token saved, but reconnect failed: {}", error));
                        })
                        .await;
                    }
                }
                Err(error) => {
                    let _ = MCPServerManager::fail_oauth_session(&callback_session, map_auth_error(error).to_string())
                        .await;
                }
            }
        });

        Ok(initial_snapshot)
    }

    /// Returns the current OAuth session snapshot for `server_id`, if any.
    pub async fn get_remote_oauth_session(&self, server_id: &str) -> Option<MCPRemoteOAuthSessionSnapshot> {
        let session = self.oauth_sessions.read().await.get(server_id).cloned()?;
        let snapshot = session.snapshot.read().await.clone();
        Some(snapshot)
    }

    /// Cancels any in-progress OAuth authorization for `server_id`.
    pub async fn cancel_remote_oauth_authorization(&self, server_id: &str) -> NortHingResult<()> {
        let session = self.oauth_sessions.write().await.remove(server_id);
        if let Some(session) = session {
            let _ = MCPServerManager::update_oauth_snapshot(&session, |snapshot| {
                snapshot.status = MCPRemoteOAuthStatus::Cancelled;
                snapshot.message = Some("OAuth authorization was cancelled.".to_string());
            })
            .await;
            MCPServerManager::shutdown_oauth_session(&session).await;
        }
        Ok(())
    }

    /// Cancels any in-progress OAuth flow and clears stored credentials.
    pub async fn clear_remote_oauth_credentials(&self, server_id: &str) -> NortHingResult<()> {
        self.cancel_remote_oauth_authorization(server_id).await?;
        crate::service::mcp::auth::clear_stored_oauth_credentials(server_id).await
    }
}

// ---------------------------------------------------------------------------
// Axum callback handler
// ---------------------------------------------------------------------------

async fn handle_oauth_callback(
    State(state): State<OAuthCallbackAppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    use super::auth_types::{render_oauth_callback_page, resolve_oauth_callback_locale, OAuthCallbackPayload};

    let payload = OAuthCallbackPayload {
        code: params.get("code").cloned(),
        state: params.get("state").cloned(),
        error: params.get("error").cloned(),
        error_description: params.get("error_description").cloned(),
    };
    let accept_language = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok());
    let locale = resolve_oauth_callback_locale(Some(state.preferred_language.as_str()), accept_language);
    let page = render_oauth_callback_page(&payload, locale);

    if let Some(callback_tx) = state.callback_tx.lock().await.take() {
        if let Err(e) = callback_tx.send(payload) {
            warn!("Failed to send OAuth callback payload: {e:?}");
        }
    }

    Html(page).into_response()
}
