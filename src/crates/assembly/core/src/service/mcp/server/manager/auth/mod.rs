//! MCP server OAuth / session / token / storage management.
//!
//! Split from the original `auth.rs` god-object into focused subdomains:
//!
//! - `auth_types`   – OAuth callback types, locale helpers, HTML renderer
//! - `auth_oauth`   – public OAuth flow entry points + axum callback handler
//! - `auth_session` – session lifecycle helpers (`set_oauth_snapshot`, …)
//! - `auth_token`   – JWT/bearer validation stubs
//! - `auth_storage` – credential persistence helpers

mod auth_oauth;
mod auth_session;
mod auth_storage;
mod auth_token;
mod auth_types;

#[cfg(test)]
mod tests;

use super::{ActiveRemoteOAuthSession, MCPServerManager};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

// ---------------------------------------------------------------------------
// Session lifecycle helpers – pub(super) so sibling submodules can call them
// ---------------------------------------------------------------------------

impl MCPServerManager {
    /// Atomically replaces the snapshot on `session` with `snapshot`.
    pub(super) async fn set_oauth_snapshot(
        session: &Arc<ActiveRemoteOAuthSession>,
        snapshot: crate::service::mcp::auth::MCPRemoteOAuthSessionSnapshot,
    ) {
        *session.snapshot.write().await = snapshot;
    }

    /// Clones the current snapshot, applies `update`, and returns the new value.
    pub(super) async fn update_oauth_snapshot<F>(
        session: &Arc<ActiveRemoteOAuthSession>,
        update: F,
    ) -> crate::service::mcp::auth::MCPRemoteOAuthSessionSnapshot
    where
        F: FnOnce(&mut crate::service::mcp::auth::MCPRemoteOAuthSessionSnapshot),
    {
        let mut snapshot = session.snapshot.write().await;
        update(&mut snapshot);
        snapshot.clone()
    }

    /// Inserts `session` into the live session map keyed by `server_id`.
    pub(super) async fn insert_oauth_session(
        &self,
        server_id: &str,
        session: Arc<ActiveRemoteOAuthSession>,
    ) -> Option<Arc<ActiveRemoteOAuthSession>> {
        self.oauth_sessions.write().await.insert(server_id.to_string(), session)
    }

    /// Sends the shutdown signal for `session`, if not already sent.
    pub(super) async fn shutdown_oauth_session(session: &Arc<ActiveRemoteOAuthSession>) {
        if let Some(shutdown_tx) = session.shutdown_tx.lock().await.take() {
            if let Err(e) = shutdown_tx.send(()) {
                warn!("Failed to send shutdown signal to OAuth session: {e:?}");
            }
        }
    }

    /// Marks `session` as failed, shuts it down, returns the final snapshot.
    pub(super) async fn fail_oauth_session(
        session: &Arc<ActiveRemoteOAuthSession>,
        message: String,
    ) -> crate::service::mcp::auth::MCPRemoteOAuthSessionSnapshot {
        let snapshot = MCPServerManager::update_oauth_snapshot(session, |snapshot| {
            snapshot.status = crate::service::mcp::auth::MCPRemoteOAuthStatus::Failed;
            snapshot.message = Some(message);
        })
        .await;
        Self::shutdown_oauth_session(session).await;
        snapshot
    }
}
