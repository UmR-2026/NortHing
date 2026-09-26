//! Credential store port for MCP and remote services (P1-8, W26-5).
//!
//! Provides an abstraction over OS keyrings and credential stores so that
//! sensitive authentication headers and credentials are not stored in
//! plaintext on disk.

use crate::port_core::{PortError, PortErrorKind, PortResult};

/// Sentinel written to server configurations (e.g. headers["Authorization"])
/// when the real credential is saved in the credential store.
pub const MCP_AUTH_SENTINEL: &str = "__kr_mcp_auth__";

/// Canonical account name function for MCP remote authorization.
/// Single source of truth for the account key in credential stores.
pub fn mcp_remote_authorization_account(server_id: &str) -> String {
    format!("mcp.remote.{server_id}.authorization")
}

/// Asynchronous credential store port.
///
/// Named `McpCredentialStore` to avoid conflict with external transport credential store traits.
#[async_trait::async_trait]
pub trait McpCredentialStore: Send + Sync {
    async fn store(&self, account: &str, secret: &str) -> PortResult<()>;
    async fn get(&self, account: &str) -> PortResult<Option<String>>;
    async fn delete(&self, account: &str) -> PortResult<()>;
}

/// Null implementation of [`McpCredentialStore`].
///
/// Used as a fallback when no credential store is registered.
/// `store` and `delete` return `NotAvailable` error; `get` returns `Ok(None)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullMcpCredentialStore;

#[async_trait::async_trait]
impl McpCredentialStore for NullMcpCredentialStore {
    async fn store(&self, _account: &str, _secret: &str) -> PortResult<()> {
        Err(PortError::new(
            PortErrorKind::NotAvailable,
            "Credential store is not available",
        ))
    }

    async fn get(&self, _account: &str) -> PortResult<Option<String>> {
        Ok(None)
    }

    async fn delete(&self, _account: &str) -> PortResult<()> {
        Err(PortError::new(
            PortErrorKind::NotAvailable,
            "Credential store is not available",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_credential_store_semantics() {
        let store = NullMcpCredentialStore;
        assert_eq!(store.get("account1").await.unwrap(), None);
        let store_res = store.store("account1", "secret").await.unwrap_err();
        assert_eq!(store_res.kind, PortErrorKind::NotAvailable);
        let del_res = store.delete("account1").await.unwrap_err();
        assert_eq!(del_res.kind, PortErrorKind::NotAvailable);
    }

    #[test]
    fn account_key_format() {
        assert_eq!(
            mcp_remote_authorization_account("srv1"),
            "mcp.remote.srv1.authorization"
        );
    }
}
