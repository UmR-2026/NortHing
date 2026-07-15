//! `#[cfg(test)]` unit tests for [`crate::remote_ssh::manager::SSHConnectionManager`].
//!
//! The tests exercise:
//! - Saved-connection pruning when the password vault is missing
//! - Saving password connections requires a non-empty password
//! - Round-tripping a saved profile through `load_connection_config_from_saved`
//! - Pruning orphaned remote-workspace entries
//! - `sftp_mkdir_all_prefixes` POSIX path expansion (absolute + redundant separators)
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::manager_sftp::sftp_mkdir_all_prefixes;
use crate::remote_ssh::types::{RemoteWorkspace, SSHAuthMethod, SSHConnectionConfig, SavedAuthType, SavedConnection};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
fn test_data_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!(
        "northhing-remote-ssh-manager-{}-{}-{}",
        name,
        std::process::id(),
        nanos
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn prunes_password_connection_without_vault_entry() {
        let dir = test_data_dir("missing-vault");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let manager = SSHConnectionManager::new(dir.clone());

        let saved = vec![SavedConnection {
            id: "ssh-root@example.com:22".to_string(),
            name: "root@example.com".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: SavedAuthType::Password,
            default_workspace: None,
            last_connected: Some(1),
        }];
        tokio::fs::write(
            dir.join("ssh_connections.json"),
            serde_json::to_string_pretty(&saved).unwrap(),
        )
        .await
        .unwrap();

        manager.load_saved_connections().await.unwrap();

        assert!(manager.get_saved_connections().await.is_empty());
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn rejects_saving_password_connection_without_password() {
        let dir = test_data_dir("empty-password-save");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let manager = SSHConnectionManager::new(dir.clone());

        let result = manager
            .save_connection(&SSHConnectionConfig {
                id: "ssh-root@example.com:22".to_string(),
                name: "root@example.com".to_string(),
                host: "example.com".to_string(),
                port: 22,
                username: "root".to_string(),
                auth: SSHAuthMethod::Password {
                    password: String::new(),
                },
                default_workspace: None,
            })
            .await;

        assert!(result.is_err());
        assert!(manager.get_saved_connections().await.is_empty());
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn restores_connection_config_from_saved_password_profile() {
        let dir = test_data_dir("restore-password-config");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let manager = SSHConnectionManager::new(dir.clone());

        manager
            .save_connection(&SSHConnectionConfig {
                id: "ssh-root@example.com:22".to_string(),
                name: "root@example.com".to_string(),
                host: "example.com".to_string(),
                port: 22,
                username: "root".to_string(),
                auth: SSHAuthMethod::Password {
                    password: "secret".to_string(),
                },
                default_workspace: Some("/root/project".to_string()),
            })
            .await
            .unwrap();

        let restored = manager
            .load_connection_config_from_saved("ssh-root@example.com:22")
            .await
            .unwrap()
            .expect("expected saved config");

        assert_eq!(restored.host, "example.com");
        assert_eq!(restored.username, "root");
        assert_eq!(restored.default_workspace.as_deref(), Some("/root/project"));
        match restored.auth {
            SSHAuthMethod::Password { password } => assert_eq!(password, "secret"),
            other => panic!("expected password auth, got {:?}", other),
        }

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn prunes_remote_workspaces_without_saved_connection() {
        let dir = test_data_dir("missing-saved");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let manager = SSHConnectionManager::new(dir.clone());

        let workspaces = vec![RemoteWorkspace {
            connection_id: "ssh-root@example.com:22".to_string(),
            remote_path: "/root/project".to_string(),
            connection_name: "root@example.com".to_string(),
            ssh_host: "example.com".to_string(),
        }];
        tokio::fs::write(
            dir.join("remote_workspace.json"),
            serde_json::to_string_pretty(&workspaces).unwrap(),
        )
        .await
        .unwrap();

        manager.load_remote_workspace().await.unwrap();
        let removed = manager
            .prune_remote_workspaces_without_saved_connections()
            .await
            .unwrap();

        assert_eq!(removed.len(), 1);
        assert!(manager.get_remote_workspaces().await.is_empty());
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[test]
    fn mkdir_all_prefixes_expand_absolute_posix_path() {
        assert_eq!(
            sftp_mkdir_all_prefixes("/home/wgq/workspace/bot_detection/.northhing/bin"),
            vec![
                "/home".to_string(),
                "/home/wgq".to_string(),
                "/home/wgq/workspace".to_string(),
                "/home/wgq/workspace/bot_detection".to_string(),
                "/home/wgq/workspace/bot_detection/.northhing".to_string(),
                "/home/wgq/workspace/bot_detection/.northhing/bin".to_string(),
            ]
        );
    }

    #[test]
    fn mkdir_all_prefixes_collapse_redundant_separators() {
        assert_eq!(
            sftp_mkdir_all_prefixes("/home//wgq///project/"),
            vec![
                "/home".to_string(),
                "/home/wgq".to_string(),
                "/home/wgq/project".to_string(),
            ]
        );
    }
}
