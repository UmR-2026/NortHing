//! SSH config (`~/.ssh/config`) parsing and host lookup.
//!
//! Reads `~/.ssh/config` and exposes the matching `SSHConfigEntry` for a given
//! host alias or hostname (and a list-all entrypoints for the SSH config UI).
//!
//! Split from `manager.rs` in Round 13b.

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::types::{SSHConfigEntry, SSHConfigLookupResult};
#[cfg(feature = "ssh_config")]
use ssh_config::SSHConfig;
use std::collections::HashMap;

/// OpenSSH keyword matching is case-insensitive, but `ssh_config` stores keys as written in the file
/// (e.g. `HostName` vs `Hostname`). Resolve by ASCII case-insensitive compare.
#[cfg(feature = "ssh_config")]
fn ssh_cfg_get<'a>(settings: &HashMap<&'a str, &'a str>, canonical_key: &str) -> Option<&'a str> {
    settings
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(canonical_key))
        .map(|(_, v)| *v)
}

#[cfg(feature = "ssh_config")]
fn ssh_cfg_has(settings: &HashMap<&str, &str>, canonical_key: &str) -> bool {
    settings.keys().any(|k| k.eq_ignore_ascii_case(canonical_key))
}

impl SSHConnectionManager {
    /// Look up SSH config for a given host alias or hostname.
    ///
    /// This parses ~/.ssh/config to find connection parameters for the given host.
    /// The host parameter can be either an alias defined in SSH config or an actual hostname.
    #[cfg(feature = "ssh_config")]
    pub async fn get_ssh_config(&self, host: &str) -> SSHConfigLookupResult {
        let ssh_config_path = dirs::home_dir()
            .map(|p| p.join(".ssh").join("config"))
            .unwrap_or_default();

        if !ssh_config_path.exists() {
            tracing::debug!("SSH config not found at {:?}", ssh_config_path);
            return SSHConfigLookupResult {
                found: false,
                config: None,
            };
        }

        let config_content = match tokio::fs::read_to_string(&ssh_config_path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read SSH config: {:?}", e);
                return SSHConfigLookupResult {
                    found: false,
                    config: None,
                };
            }
        };

        let config = match SSHConfig::parse_str(&config_content) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to parse SSH config: {:?}", e);
                return SSHConfigLookupResult {
                    found: false,
                    config: None,
                };
            }
        };

        // Use query() to get host configuration - this handles Host pattern matching
        let host_settings = config.query(host);

        if host_settings.is_empty() {
            tracing::debug!("No SSH config found for host: {}", host);
            return SSHConfigLookupResult {
                found: false,
                config: None,
            };
        }

        tracing::debug!(
            "Found SSH config for host: {} with {} settings",
            host,
            host_settings.len()
        );

        // Canonical OpenSSH names; lookup is case-insensitive (see ssh_cfg_get).
        let hostname = ssh_cfg_get(&host_settings, "HostName").map(|s| s.to_string());
        let user = ssh_cfg_get(&host_settings, "User").map(|s| s.to_string());
        let port = ssh_cfg_get(&host_settings, "Port").and_then(|s| s.parse::<u16>().ok());
        let identity_file = ssh_cfg_get(&host_settings, "IdentityFile").map(|f| shellexpand::tilde(f).to_string());

        let has_proxy_command = ssh_cfg_has(&host_settings, "ProxyCommand");

        SSHConfigLookupResult {
            found: true,
            config: Some(SSHConfigEntry {
                host: host.to_string(),
                hostname,
                port,
                user,
                identity_file,
                agent: if has_proxy_command { None } else { Some(true) },
            }),
        }
    }

    #[cfg(not(feature = "ssh_config"))]
    pub async fn get_ssh_config(&self, _host: &str) -> SSHConfigLookupResult {
        SSHConfigLookupResult {
            found: false,
            config: None,
        }
    }

    /// List all hosts defined in ~/.ssh/config
    #[cfg(feature = "ssh_config")]
    pub async fn list_ssh_config_hosts(&self) -> Vec<SSHConfigEntry> {
        let ssh_config_path = dirs::home_dir()
            .map(|p| p.join(".ssh").join("config"))
            .unwrap_or_default();

        if !ssh_config_path.exists() {
            tracing::debug!("SSH config not found at {:?}", ssh_config_path);
            return Vec::new();
        }

        let config_content = match tokio::fs::read_to_string(&ssh_config_path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read SSH config: {:?}", e);
                return Vec::new();
            }
        };

        let config = match SSHConfig::parse_str(&config_content) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to parse SSH config: {:?}", e);
                return Vec::new();
            }
        };

        let mut hosts = Vec::new();

        // SSHConfig library doesn't expose listing all hosts, so we parse the raw config
        // to extract Host entries. This is a simple but effective approach.
        for line in config_content.lines() {
            let line = line.trim();
            // Match "Host alias1 alias2 ..." lines (but not "HostName")
            if line.starts_with("Host ") && !line.starts_with("HostName") {
                // Extract everything after "Host "
                let host_part = line.strip_prefix("Host ").unwrap_or("").trim();
                if host_part.is_empty() {
                    continue;
                }
                // Host can be "alias1 alias2 ..." - we want the first one (main alias)
                let aliases: Vec<&str> = host_part.split_whitespace().collect();
                if aliases.is_empty() {
                    continue;
                }

                let alias = aliases[0];
                // Query config for this host to get details
                let settings = config.query(alias);

                let identity_file = ssh_cfg_get(&settings, "IdentityFile").map(|f| shellexpand::tilde(f).to_string());

                let hostname = ssh_cfg_get(&settings, "HostName").map(|s| s.to_string());
                let user = ssh_cfg_get(&settings, "User").map(|s| s.to_string());
                let port = ssh_cfg_get(&settings, "Port").and_then(|s| s.parse::<u16>().ok());

                hosts.push(SSHConfigEntry {
                    host: alias.to_string(),
                    hostname,
                    port,
                    user,
                    identity_file,
                    agent: None, // Can't easily determine agent setting from raw parsing
                });
            }
        }

        tracing::debug!("Found {} hosts in SSH config", hosts.len());
        hosts
    }

    #[cfg(not(feature = "ssh_config"))]
    pub async fn list_ssh_config_hosts(&self) -> Vec<SSHConfigEntry> {
        Vec::new()
    }
}
