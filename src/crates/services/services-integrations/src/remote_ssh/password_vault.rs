//! Encrypted file-backed storage for SSH password authentication.
//!
//! A random 32-byte key lives in `data_dir/.ssh_password_vault.key` (0600 on Unix).
//! Ciphertext map is stored in `data_dir/ssh_password_vault.json`.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use northhing_services_core::JsonFileStore;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::sync::Mutex;

const NONCE_LEN: usize = 12;

#[derive(Serialize, Deserialize, Default)]
struct VaultFile {
    entries: HashMap<String, String>,
}

pub struct SSHPasswordVault {
    key_path: PathBuf,
    vault_path: PathBuf,
    lock: Mutex<()>,
}

impl SSHPasswordVault {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            key_path: data_dir.join(".ssh_password_vault.key"),
            vault_path: data_dir.join("ssh_password_vault.json"),
            lock: Mutex::new(()),
        }
    }

    async fn ensure_key(&self) -> Result<[u8; 32]> {
        if self.key_path.exists() {
            let bytes = tokio::fs::read(&self.key_path)
                .await
                .context("read ssh password vault key")?;
            if bytes.len() != 32 {
                anyhow::bail!("invalid ssh password vault key length");
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
        if let Some(p) = self.key_path.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        tokio::fs::write(&self.key_path, key.as_slice())
            .await
            .context("write ssh password vault key")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) = std::fs::set_permissions(&self.key_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::warn!("Failed to set 0600 permissions on {}: {}", self.key_path.display(), error);
            }
        }
        Ok(key)
    }

    fn encrypt_password(key: &[u8; 32], plaintext: &str) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt: {}", e))?;
        let mut blob = Vec::with_capacity(NONCE_LEN + ct.len());
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        Ok(B64.encode(blob))
    }

    fn decrypt_password(key: &[u8; 32], blob_b64: &str) -> Result<String> {
        let blob = B64.decode(blob_b64).context("base64 decode ssh vault entry")?;
        if blob.len() <= NONCE_LEN {
            anyhow::bail!("ssh vault entry too short");
        }
        let (nonce, ct) = blob.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{}", e))?;
        let pt = cipher
            .decrypt(Nonce::from_slice(nonce), ct)
            .map_err(|e| anyhow::anyhow!("decrypt: {}", e))?;
        String::from_utf8(pt).context("utf8 decode ssh vault password")
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
            .context("write ssh password vault atomically")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) = std::fs::set_permissions(&self.vault_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::warn!("Failed to set 0600 permissions on {}: {}", self.vault_path.display(), error);
            }
        }
        Ok(())
    }

    pub async fn store(&self, connection_id: &str, password: &str) -> Result<()> {
        let _g = self.lock.lock().await;
        let key = self.ensure_key().await?;
        let mut file = self.read_vault_file().await.context("refusing to overwrite vault")?;
        let enc = Self::encrypt_password(&key, password)?;
        file.entries.insert(connection_id.to_string(), enc);
        self.write_vault(&file).await
    }

    pub async fn load(&self, connection_id: &str) -> Result<Option<String>> {
        let _g = self.lock.lock().await;
        if !self.vault_path.exists() || !self.key_path.exists() {
            return Ok(None);
        }
        let bytes = tokio::fs::read(&self.key_path).await.context("read ssh vault key")?;
        if bytes.len() != 32 {
            anyhow::bail!("invalid ssh password vault key length");
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);

        let file = self.read_vault_file().await?;
        let Some(entry) = file.entries.get(connection_id) else {
            return Ok(None);
        };
        match Self::decrypt_password(&key, entry) {
            Ok(p) => Ok(Some(p)),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH password vault entry for {}: {}",
                    connection_id,
                    e
                );
                Ok(None)
            }
        }
    }

    pub async fn remove(&self, connection_id: &str) -> Result<()> {
        let _g = self.lock.lock().await;
        if !self.vault_path.exists() {
            return Ok(());
        }
        let mut file = self.read_vault_file().await.context("refusing to overwrite vault")?;
        file.entries.remove(connection_id);
        if file.entries.is_empty() {
            let _ = tokio::fs::remove_file(&self.vault_path).await;
        } else {
            self.write_vault(&file).await?;
        }
        Ok(())
    }

    pub async fn migrate_entry(&self, old_connection_id: &str, new_connection_id: &str) -> Result<()> {
        if old_connection_id == new_connection_id {
            return Ok(());
        }
        let _g = self.lock.lock().await;
        if !self.vault_path.exists() {
            return Ok(());
        }
        let mut file = self.read_vault_file().await.context("refusing to overwrite vault")?;
        let Some(entry) = file.entries.remove(old_connection_id) else {
            return Ok(());
        };
        file.entries.entry(new_connection_id.to_string()).or_insert(entry);
        self.write_vault(&file).await
    }
}

#[cfg(test)]
mod tests {
    use super::SSHPasswordVault;
    use northhing_test_support::TestTempDir;
    use std::path::PathBuf;

    fn test_vault() -> (TestTempDir, SSHPasswordVault, PathBuf) {
        let dir = TestTempDir::new("ssh-vault");
        let vault = SSHPasswordVault::new(dir.path().to_path_buf());
        let vault_path = dir.path().join("ssh_password_vault.json");
        (dir, vault, vault_path)
    }

    #[tokio::test]
    async fn migrate_entry_moves_password_to_new_connection_id() {
        let (dir, vault, _) = test_vault();

        vault.store("ssh-root@example.com:22", "secret").await.unwrap();
        vault
            .migrate_entry("ssh-root@example.com:22", "ssh-root@example.com")
            .await
            .unwrap();

        assert_eq!(
            vault.load("ssh-root@example.com").await.unwrap().as_deref(),
            Some("secret")
        );
        assert!(vault.load("ssh-root@example.com:22").await.unwrap().is_none());

        drop(dir);
    }

    #[tokio::test]
    async fn store_fails_closed_on_corrupted_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let corrupted = b"{ not valid json !!!";
        tokio::fs::write(&vault_path, corrupted).await.unwrap();

        let result = vault.store("ssh-root@example.com:22", "secret").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted);
    }

    #[tokio::test]
    async fn remove_fails_closed_on_corrupted_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let corrupted = b"{ not valid json !!!";
        tokio::fs::write(&vault_path, corrupted).await.unwrap();

        let result = vault.remove("ssh-root@example.com").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted);
    }

    #[tokio::test]
    async fn migrate_fails_closed_on_corrupted_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let corrupted = b"{ not valid json !!!";
        tokio::fs::write(&vault_path, corrupted).await.unwrap();

        let result = vault
            .migrate_entry("ssh-root@example.com:22", "ssh-root@example.com")
            .await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted);
    }

    #[tokio::test]
    async fn store_fails_closed_on_truncated_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let truncated = br#"{"entries": {"#;
        tokio::fs::write(&vault_path, truncated).await.unwrap();

        let result = vault.store("ssh-root@example.com:22", "secret").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), truncated);
    }

    #[tokio::test]
    async fn remove_fails_closed_on_truncated_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let truncated = br#"{"entries": {"#;
        tokio::fs::write(&vault_path, truncated).await.unwrap();

        let result = vault.remove("ssh-root@example.com").await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), truncated);
    }

    #[tokio::test]
    async fn migrate_fails_closed_on_truncated_vault_without_touching_file() {
        let (_dir, vault, vault_path) = test_vault();
        let truncated = br#"{"entries": {"#;
        tokio::fs::write(&vault_path, truncated).await.unwrap();

        let result = vault
            .migrate_entry("ssh-root@example.com:22", "ssh-root@example.com")
            .await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), truncated);
    }

    #[tokio::test]
    async fn load_returns_error_on_corrupted_vault() {
        let (_dir, vault, vault_path) = test_vault();
        vault.store("ssh-root@example.com", "secret").await.unwrap();
        tokio::fs::write(&vault_path, b"garbage").await.unwrap();

        assert!(vault.load("ssh-root@example.com").await.is_err());
    }

    #[tokio::test]
    async fn vault_store_is_atomic_and_keeps_bak_of_previous_content() {
        let (_dir, vault, vault_path) = test_vault();

        vault.store("a", "p1").await.unwrap();
        let first = tokio::fs::read(&vault_path).await.unwrap();
        assert!(!vault_path.with_extension("bak").exists());

        vault.store("b", "p2").await.unwrap();

        assert_eq!(vault.load("a").await.unwrap().as_deref(), Some("p1"));
        assert_eq!(vault.load("b").await.unwrap().as_deref(), Some("p2"));
        let bak = tokio::fs::read(vault_path.with_extension("bak")).await.unwrap();
        assert_eq!(bak, first);
    }

    #[tokio::test]
    async fn vault_remove_deletes_file_when_last_entry_is_removed() {
        let (_dir, vault, vault_path) = test_vault();

        vault.store("a", "p1").await.unwrap();
        vault.remove("a").await.unwrap();

        assert!(!vault_path.exists());
        vault.store("a", "p2").await.unwrap();
        assert_eq!(vault.load("a").await.unwrap().as_deref(), Some("p2"));
    }
}
