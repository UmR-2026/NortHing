//! Remote MCP OAuth runtime support.
//!
//! This module owns the file-backed credential store and OAuth bootstrap logic.
//! `northhing-core` injects the product data directory and maps errors to its
//! compatibility error type.

use crate::mcp::server::{MCPServerConfig, MCPServerOAuthConfig};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use northhing_services_core::JsonFileStore;
use rand::RngCore;
use rmcp::transport::auth::{AuthorizationManager, CredentialStore, OAuthState, StoredCredentials};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MCPRemoteOAuthStatus {
    AwaitingBrowser,
    AwaitingCallback,
    ExchangingToken,
    Authorized,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPRemoteOAuthSessionSnapshot {
    pub server_id: String,
    pub status: MCPRemoteOAuthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl MCPRemoteOAuthSessionSnapshot {
    pub fn new(
        server_id: impl Into<String>,
        status: MCPRemoteOAuthStatus,
        authorization_url: Option<String>,
        redirect_uri: Option<String>,
        message: Option<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            status,
            authorization_url,
            redirect_uri,
            message,
        }
    }
}

pub struct PreparedMCPRemoteOAuthAuthorization {
    pub state: OAuthState,
    pub listener: TcpListener,
    pub authorization_url: String,
    pub redirect_uri: String,
}

#[derive(Serialize, Deserialize, Default)]
struct VaultFile {
    entries: HashMap<String, String>,
}

pub struct MCPRemoteOAuthCredentialVault {
    key_path: PathBuf,
    vault_path: PathBuf,
    lock: Mutex<()>,
}

impl MCPRemoteOAuthCredentialVault {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            key_path: data_dir.join(".mcp_oauth_vault.key"),
            vault_path: data_dir.join("mcp_oauth_vault.json"),
            lock: Mutex::new(()),
        }
    }

    async fn ensure_key(&self) -> Result<[u8; 32]> {
        if self.key_path.exists() {
            let bytes = tokio::fs::read(&self.key_path)
                .await
                .context("read MCP OAuth vault key")?;
            if bytes.len() != 32 {
                anyhow::bail!("invalid MCP OAuth vault key length");
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }

        if let Some(parent) = self.key_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        tokio::fs::write(&self.key_path, key.as_slice())
            .await
            .context("write MCP OAuth vault key")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) = std::fs::set_permissions(&self.key_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::warn!("Failed to set 0600 permissions on {}: {}", self.key_path.display(), error);
            }
        }

        Ok(key)
    }

    fn encrypt_value(key: &[u8; 32], plaintext: &str) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt: {}", e))?;

        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);
        Ok(B64.encode(blob))
    }

    fn decrypt_value(key: &[u8; 32], blob_b64: &str) -> Result<String> {
        let blob = B64.decode(blob_b64).context("base64 decode MCP OAuth vault entry")?;
        if blob.len() <= NONCE_LEN {
            anyhow::bail!("MCP OAuth vault entry too short");
        }

        let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{}", e))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| anyhow::anyhow!("decrypt: {}", e))?;
        String::from_utf8(plaintext).context("utf8 decode MCP OAuth vault entry")
    }

    /// Reads the vault file, treating a missing file as the empty initial
    /// state. Fail-closed: any read or parse error propagates so callers never
    /// overwrite a vault they cannot fully read back.
    async fn read_vault_file(&self) -> Result<VaultFile> {
        match tokio::fs::read_to_string(&self.vault_path).await {
            Ok(body) => {
                serde_json::from_str(&body).with_context(|| format!("vault corrupted: {}", self.vault_path.display()))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(VaultFile::default()),
            Err(error) => Err(error).with_context(|| format!("failed to read vault: {}", self.vault_path.display())),
        }
    }

    /// Backs up the current vault content to `<name>.bak` before an atomic
    /// replace. Failure to back up is warn-only and never blocks the write.
    async fn backup_vault(&self) {
        if !self.vault_path.exists() {
            return;
        }
        if let Err(error) = tokio::fs::copy(&self.vault_path, self.vault_path.with_extension("bak")).await {
            tracing::warn!("Failed to back up vault {}: {}", self.vault_path.display(), error);
        }
    }

    /// Atomically persists the vault via `JsonFileStore::write_atomic`
    /// (tmp + rename with Windows share-handle retries), keeping a `.bak`
    /// copy of the previous content, and restoring 0o600 on Unix.
    async fn write_vault(&self, file: &VaultFile) -> Result<()> {
        self.backup_vault().await;
        JsonFileStore
            .write_atomic(&self.vault_path, file)
            .await
            .context("write MCP OAuth vault atomically")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) = std::fs::set_permissions(&self.vault_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::warn!("Failed to set 0600 permissions on {}: {}", self.vault_path.display(), error);
            }
        }
        Ok(())
    }

    pub async fn load(&self, server_id: &str) -> Result<Option<StoredCredentials>> {
        let _guard = self.lock.lock().await;
        if !self.key_path.exists() || !self.vault_path.exists() {
            return Ok(None);
        }

        let bytes = tokio::fs::read(&self.key_path)
            .await
            .context("read MCP OAuth vault key")?;
        if bytes.len() != 32 {
            anyhow::bail!("invalid MCP OAuth vault key length");
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);

        let file = self.read_vault_file().await?;
        let Some(entry) = file.entries.get(server_id) else {
            return Ok(None);
        };

        let plaintext = match Self::decrypt_value(&key, entry) {
            Ok(plaintext) => plaintext,
            Err(error) => {
                tracing::warn!(
                    "Failed to decrypt MCP OAuth credentials for server {}: {}",
                    server_id,
                    error
                );
                return Ok(None);
            }
        };

        Ok(Some(serde_json::from_str(&plaintext)?))
    }

    pub async fn store(&self, server_id: &str, credentials: &StoredCredentials) -> Result<()> {
        let _guard = self.lock.lock().await;
        let key = self.ensure_key().await?;

        let mut file = self.read_vault_file().await.context("refusing to overwrite vault")?;

        let plaintext = serde_json::to_string(credentials)?;
        let encrypted = Self::encrypt_value(&key, &plaintext)?;
        file.entries.insert(server_id.to_string(), encrypted);

        self.write_vault(&file).await
    }

    pub async fn clear(&self, server_id: &str) -> Result<()> {
        let _guard = self.lock.lock().await;
        if !self.vault_path.exists() {
            return Ok(());
        }

        let mut file = self.read_vault_file().await.context("refusing to overwrite vault")?;
        file.entries.remove(server_id);

        if file.entries.is_empty() {
            let _ = tokio::fs::remove_file(&self.vault_path).await;
        } else {
            self.write_vault(&file).await?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct MCPRemoteOAuthCredentialStore {
    data_dir: PathBuf,
    server_id: String,
}

impl MCPRemoteOAuthCredentialStore {
    pub fn new(data_dir: impl Into<PathBuf>, server_id: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
            server_id: server_id.into(),
        }
    }
}

#[async_trait]
impl CredentialStore for MCPRemoteOAuthCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, rmcp::transport::auth::AuthError> {
        MCPRemoteOAuthCredentialVault::new(self.data_dir.clone())
            .load(&self.server_id)
            .await
            .map_err(|error| rmcp::transport::auth::AuthError::InternalError(error.to_string()))
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), rmcp::transport::auth::AuthError> {
        MCPRemoteOAuthCredentialVault::new(self.data_dir.clone())
            .store(&self.server_id, &credentials)
            .await
            .map_err(|error| rmcp::transport::auth::AuthError::InternalError(error.to_string()))
    }

    async fn clear(&self) -> Result<(), rmcp::transport::auth::AuthError> {
        MCPRemoteOAuthCredentialVault::new(self.data_dir.clone())
            .clear(&self.server_id)
            .await
            .map_err(|error| rmcp::transport::auth::AuthError::InternalError(error.to_string()))
    }
}

pub async fn has_stored_oauth_credentials(data_dir: impl Into<PathBuf>, server_id: &str) -> Result<bool> {
    let store = MCPRemoteOAuthCredentialStore::new(data_dir, server_id.to_string());
    let credentials = store.load().await?;
    Ok(credentials.and_then(|entry| entry.token_response).is_some())
}

pub async fn clear_stored_oauth_credentials(data_dir: impl Into<PathBuf>, server_id: &str) -> Result<()> {
    MCPRemoteOAuthCredentialStore::new(data_dir, server_id.to_string())
        .clear()
        .await?;
    Ok(())
}

pub async fn build_authorization_manager(
    data_dir: impl Into<PathBuf>,
    server_id: &str,
    server_url: &str,
) -> Result<(AuthorizationManager, bool)> {
    let mut manager = AuthorizationManager::new(server_url).await?;
    manager.set_credential_store(MCPRemoteOAuthCredentialStore::new(data_dir, server_id.to_string()));
    let initialized = manager.initialize_from_store().await?;
    Ok((manager, initialized))
}

fn normalize_callback_host(config: &MCPServerOAuthConfig) -> String {
    config
        .callback_host
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("127.0.0.1")
        .to_string()
}

fn normalize_callback_path(config: &MCPServerOAuthConfig) -> String {
    let path = config
        .callback_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("/oauth/callback");

    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

fn effective_oauth_config(config: &MCPServerConfig) -> MCPServerOAuthConfig {
    let mut oauth = config.oauth.clone().unwrap_or_default();
    if oauth.client_name.is_none() {
        oauth.client_name = Some(format!("northhing MCP Client ({})", config.name));
    }
    oauth
}

pub async fn prepare_remote_oauth_authorization(
    data_dir: impl Into<PathBuf>,
    config: &MCPServerConfig,
) -> Result<PreparedMCPRemoteOAuthAuthorization> {
    let data_dir = data_dir.into();
    let oauth = effective_oauth_config(config);
    let server_url = config
        .url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Remote MCP server '{}' must have a URL for OAuth", config.id))?;

    let host = normalize_callback_host(&oauth);
    let listener = TcpListener::bind((host.as_str(), oauth.callback_port.unwrap_or(0)))
        .await
        .with_context(|| format!("Failed to bind OAuth callback listener for server '{}'", config.id))?;
    let port = listener
        .local_addr()
        .with_context(|| format!("Failed to resolve OAuth callback listener for server '{}'", config.id))?
        .port();
    let redirect_uri = format!("http://{}:{}{}", host, port, normalize_callback_path(&oauth));

    let scopes = oauth.scopes.iter().map(String::as_str).collect::<Vec<_>>();
    let mut state = OAuthState::new(server_url, None).await?;
    if let OAuthState::Unauthorized(manager) = &mut state {
        manager.set_credential_store(MCPRemoteOAuthCredentialStore::new(data_dir.clone(), config.id.clone()));
    }

    match oauth.client_metadata_url.as_deref() {
        Some(client_metadata_url) => {
            state
                .start_authorization_with_metadata_url(
                    &scopes,
                    &redirect_uri,
                    oauth.client_name.as_deref(),
                    Some(client_metadata_url),
                )
                .await?;
        }
        None => {
            state
                .start_authorization(&scopes, &redirect_uri, oauth.client_name.as_deref())
                .await?;
        }
    }

    let authorization_url = state.get_authorization_url().await?;

    Ok(PreparedMCPRemoteOAuthAuthorization {
        state,
        listener,
        authorization_url,
        redirect_uri,
    })
}

#[cfg(test)]
mod tests {
    use super::MCPRemoteOAuthCredentialVault;
    use northhing_test_support::TestTempDir;
    use rmcp::transport::auth::StoredCredentials;
    use std::path::PathBuf;

    fn test_vault() -> (TestTempDir, MCPRemoteOAuthCredentialVault, PathBuf) {
        let dir = TestTempDir::new("mcp-oauth-vault");
        let vault = MCPRemoteOAuthCredentialVault::new(dir.path().to_path_buf());
        let vault_path = dir.path().join("mcp_oauth_vault.json");
        (dir, vault, vault_path)
    }

    fn credentials(server_id: &str) -> StoredCredentials {
        StoredCredentials::new(format!("client-{server_id}"), None, Vec::new(), None)
    }

    #[tokio::test]
    async fn store_fails_closed_on_corrupted_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let corrupted = b"{ not valid json !!!";
        tokio::fs::write(&vault_path, corrupted).await.unwrap();

        let result = vault.store("server-a", &credentials("server-a")).await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted);
    }

    #[tokio::test]
    async fn clear_fails_closed_on_corrupted_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let corrupted = b"{ not valid json !!!";
        tokio::fs::write(&vault_path, corrupted).await.unwrap();

        let result = vault.clear("server-a").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted);
    }

    #[tokio::test]
    async fn store_fails_closed_on_truncated_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let truncated = br#"{"entries": {"#;
        tokio::fs::write(&vault_path, truncated).await.unwrap();

        let result = vault.store("server-a", &credentials("server-a")).await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), truncated);
    }

    #[tokio::test]
    async fn clear_fails_closed_on_truncated_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let truncated = br#"{"entries": {"#;
        tokio::fs::write(&vault_path, truncated).await.unwrap();

        let result = vault.clear("server-a").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), truncated);
    }

    #[tokio::test]
    async fn load_returns_error_on_corrupted_vault() {
        let (_dir, vault, vault_path) = test_vault();
        vault.store("server-a", &credentials("server-a")).await.unwrap();
        tokio::fs::write(&vault_path, b"garbage").await.unwrap();

        assert!(vault.load("server-a").await.is_err());
    }

    #[tokio::test]
    async fn vault_store_is_atomic_and_keeps_bak_of_previous_content() {
        let (_dir, vault, vault_path) = test_vault();

        vault.store("server-a", &credentials("server-a")).await.unwrap();
        let first = tokio::fs::read(&vault_path).await.unwrap();
        assert!(!vault_path.with_extension("bak").exists());

        vault.store("server-b", &credentials("server-b")).await.unwrap();

        let stored_a = vault.load("server-a").await.unwrap().unwrap();
        assert_eq!(stored_a.client_id, "client-server-a");
        let stored_b = vault.load("server-b").await.unwrap().unwrap();
        assert_eq!(stored_b.client_id, "client-server-b");
        let bak = tokio::fs::read(vault_path.with_extension("bak")).await.unwrap();
        assert_eq!(bak, first);
    }

    #[tokio::test]
    async fn vault_clear_deletes_file_when_last_entry_is_cleared() {
        let (_dir, vault, vault_path) = test_vault();

        vault.store("server-a", &credentials("server-a")).await.unwrap();
        vault.clear("server-a").await.unwrap();

        assert!(!vault_path.exists());
        vault.store("server-a", &credentials("server-a")).await.unwrap();
        let stored = vault.load("server-a").await.unwrap().unwrap();
        assert_eq!(stored.client_id, "client-server-a");
    }
}
