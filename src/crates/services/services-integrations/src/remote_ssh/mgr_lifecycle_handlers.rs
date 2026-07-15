//! SSH session establishment: TCP connect, handshake, auth, server-info probe.
//!
//! Owns the `establish_session` orchestrator and all phase helpers used to
//! open a fresh SSH session from a [`SSHConnectionConfig`] without touching
//! the connection map:
//!
//! - Phase 1 `prepare_session_transport` opens the TCP socket and loads the
//!   private key when relevant (`load_private_key_for_auth`,
//!   `read_private_key_file`).
//! - Phase 2 `perform_session_handshake` builds the russh client config
//!   (`build_session_client_config`), opens the SSH channel, and translates
//!   handshake failures into user-friendly `anyhow` errors
//!   (`map_handshake_error`).
//! - Phase 3 `perform_session_auth` runs password or public-key auth on the
//!   already-handshaken handle.
//! - Phase 4 `resolve_session_server_info` resolves the remote `ServerInfo`
//!   (`get_server_info_internal`, `probe_remote_home_dir`).
//!
//! `interrupt_exec_channel` lives here too because it shares the same
//! `russh::Channel<Msg>` plumbing as the handshake/auth phase (used from the
//! execute pump loop in `mgr_lifecycle_state`).

use crate::remote_ssh::manager::SSHConnectionManager;
use crate::remote_ssh::manager_handler::{HandlerError, SSHHandler};
use crate::remote_ssh::types::{SSHAuthMethod, SSHCommandOptions, SSHConnectionConfig, ServerInfo};
use anyhow::anyhow;
use russh::client::{Handle, Msg};
use russh::Sig;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::Duration;

impl SSHConnectionManager {
    /// Build a fresh SSH session (handshake + auth + server info probe) without
    /// touching the connection map. Reused by both [`Self::connect_with_timeout`]
    /// and the transparent reconnect path in [`Self::ensure_alive_or_reconnect`].
    pub(super) async fn establish_session(
        &self,
        config: &SSHConnectionConfig,
        timeout_secs: u64,
    ) -> anyhow::Result<(Handle<SSHHandler>, Arc<AtomicBool>, Option<ServerInfo>)> {
        let (stream, key_pair) = Self::prepare_session_transport(config, timeout_secs).await?;
        let (mut handle, alive) = self.perform_session_handshake(stream, config, timeout_secs).await?;
        Self::perform_session_auth(&mut handle, config, key_pair.as_ref()).await?;
        let server_info = Self::resolve_session_server_info(&handle).await;
        Ok((handle, alive, server_info))
    }

    /// Phase 1 of `establish_session`: open the TCP socket and load the
    /// private key (no-op for password auth).
    async fn prepare_session_transport(
        config: &SSHConnectionConfig,
        timeout_secs: u64,
    ) -> anyhow::Result<(TcpStream, Option<russh_keys::key::KeyPair>)> {
        let addr = format!("{}:{}", config.host, config.port);

        // Connect to the server with timeout
        let stream = tokio::time::timeout(Duration::from_secs(timeout_secs), TcpStream::connect(&addr))
            .await
            .map_err(|_| anyhow!("Connection timeout after {} seconds", timeout_secs))?
            .map_err(|e| anyhow!("Failed to connect to {}: {}", addr, e))?;

        let key_pair = Self::load_private_key_for_auth(&config.auth)?;
        Ok((stream, key_pair))
    }

    /// Load and decode the private key referenced by `auth`. Returns `None` for
    /// password auth (no key needed).
    fn load_private_key_for_auth(auth: &SSHAuthMethod) -> anyhow::Result<Option<russh_keys::key::KeyPair>> {
        match auth {
            SSHAuthMethod::Password { .. } => Ok(None),
            SSHAuthMethod::PrivateKey { key_path, passphrase } => {
                tracing::info!(
                    "Attempting private key auth with key_path: {}, passphrase provided: {}",
                    key_path,
                    passphrase.is_some()
                );
                let key_content = Self::read_private_key_file(key_path)?;
                tracing::info!("Decoding private key...");
                let key_pair = russh_keys::decode_secret_key(&key_content, passphrase.as_ref().map(|s| s.as_str()))
                    .map_err(|e| anyhow!("Failed to decode private key: {}", e))?;
                tracing::info!("Successfully decoded private key");
                Ok(Some(key_pair))
            }
        }
    }

    /// Read the private key from `key_path`, falling back to `~/.ssh/id_rsa`
    /// when the explicit path cannot be read.
    fn read_private_key_file(key_path: &str) -> anyhow::Result<String> {
        let expanded = shellexpand::tilde(key_path);
        tracing::info!("Expanded key path: {}", expanded);
        match std::fs::read_to_string(expanded.as_ref()) {
            Ok(content) => {
                tracing::info!("Successfully read {} bytes from key file", content.len());
                Ok(content)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read private key at '{}': {}, trying default ~/.ssh/id_rsa",
                    expanded,
                    e
                );
                if let Ok(home) = std::env::var("HOME") {
                    let default_key = format!("{}/.ssh/id_rsa", home);
                    tracing::info!("Trying default key at: {}", default_key);
                    std::fs::read_to_string(&default_key).map_err(|e| {
                        anyhow!(
                            "Failed to read private key '{}' and default key '{}': {}",
                            key_path,
                            default_key,
                            e
                        )
                    })
                } else {
                    Err(anyhow!(
                        "Failed to read private key '{}': {}, and could not determine home directory",
                        key_path,
                        e
                    ))
                }
            }
        }
    }

    /// Phase 2 of `establish_session`: build the russh client config, create a
    /// `SSHHandler` (with known-hosts), and run the SSH handshake with timeout.
    /// Returns the live `Handle` and the liveness flag created with the handler.
    async fn perform_session_handshake(
        &self,
        stream: TcpStream,
        config: &SSHConnectionConfig,
        timeout_secs: u64,
    ) -> anyhow::Result<(Handle<SSHHandler>, Arc<AtomicBool>)> {
        let ssh_config = Arc::new(Self::build_session_client_config());
        let (handler, disconnect_reason, alive) =
            SSHHandler::with_known_hosts(config.host.clone(), config.port, self.known_hosts.clone());

        let addr = format!("{}:{}", config.host, config.port);
        tracing::info!("Starting SSH handshake to {}", addr);
        let connect_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            russh::client::connect_stream(ssh_config, stream, handler),
        )
        .await
        .map_err(|_| anyhow!("SSH handshake timeout after {} seconds", timeout_secs))?;

        let handle = connect_result.map_err(|e| Self::map_handshake_error(e, &disconnect_reason, config))?;
        tracing::info!("SSH handshake completed successfully");
        Ok((handle, alive))
    }

    /// Build the russh `Config` used for new sessions: keepalive / inactivity
    /// tunables plus a broad algorithm list for compatibility with legacy servers.
    fn build_session_client_config() -> russh::client::Config {
        russh::client::Config {
            // Tolerate brief network blips (NAT timeouts, Wi-Fi roaming) by
            // widening the inactivity window and allowing more missed keepalives
            // before declaring the session dead. Combined with transparent
            // reconnect, this prevents the user-visible "early eof" cascade
            // while idly browsing the remote file picker.
            inactivity_timeout: Some(Duration::from_secs(180)),
            keepalive_interval: Some(Duration::from_secs(30)),
            keepalive_max: 6,
            // Broad algorithm list for compatibility with both modern and legacy SSH servers.
            // Modern algorithms first (preferred), legacy ones appended as fallback.
            preferred: russh::Preferred {
                // KEX: modern curve25519 first, then older DH groups for legacy servers
                kex: std::borrow::Cow::Owned(vec![
                    russh::kex::CURVE25519,
                    russh::kex::CURVE25519_PRE_RFC_8731,
                    russh::kex::DH_G16_SHA512,
                    russh::kex::DH_G14_SHA256,
                    russh::kex::DH_G14_SHA1, // legacy servers
                    russh::kex::DH_G1_SHA1,  // very old servers
                    russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                    russh::kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT,
                ]),
                // Host key algorithms: include ssh-rsa for older servers
                key: std::borrow::Cow::Owned(vec![
                    russh_keys::key::ED25519,
                    russh_keys::key::ECDSA_SHA2_NISTP256,
                    russh_keys::key::ECDSA_SHA2_NISTP521,
                    russh_keys::key::RSA_SHA2_256,
                    russh_keys::key::RSA_SHA2_512,
                    russh_keys::key::SSH_RSA, // legacy servers that only advertise ssh-rsa
                ]),
                ..russh::Preferred::DEFAULT
            },
            ..Default::default()
        }
    }

    /// Translate a russh handshake error into a user-friendly `anyhow::Error`,
    /// surfacing the captured `disconnect_reason` when available.
    fn map_handshake_error(
        e: HandlerError,
        disconnect_reason: &std::sync::Arc<std::sync::Mutex<Option<String>>>,
        config: &SSHConnectionConfig,
    ) -> anyhow::Error {
        // Try to surface the real disconnect reason captured in the handler.
        // russh's run() absorbs errors; our disconnected() callback stores them.
        let real_reason = disconnect_reason.lock().ok().and_then(|g| g.clone());
        if let Some(reason) = real_reason {
            anyhow!("SSH handshake failed: {}", reason)
        } else {
            // HandlerError("Disconnect") with no stored reason means the server
            // closed the TCP connection before sending any SSH banner.
            // This typically means: sshd is not running, max connections reached,
            // or a firewall/IP ban is in effect.
            let e_dbg = format!("{:?}", e);
            if e_dbg.contains("Disconnect") {
                anyhow!(
                    "SSH connection refused: server {}:{} closed the connection without sending an SSH banner. \
                     Check that sshd is running and accepting connections.",
                    config.host,
                    config.port
                )
            } else {
                anyhow!("Failed to establish SSH connection: {:?}", e)
            }
        }
    }

    /// Phase 3 of `establish_session`: run password or public-key auth on the
    /// already-handshaken `Handle`.
    async fn perform_session_auth(
        handle: &mut Handle<SSHHandler>,
        config: &SSHConnectionConfig,
        key_pair: Option<&russh_keys::key::KeyPair>,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting authentication for user {}", config.username);
        let auth_success: bool = match &config.auth {
            SSHAuthMethod::Password { password } => {
                tracing::debug!("Using password authentication");
                handle
                    .authenticate_password(&config.username, password.clone())
                    .await
                    .map_err(|e| anyhow!("Password authentication failed: {:?}", e))?
            }
            SSHAuthMethod::PrivateKey {
                key_path,
                passphrase: _,
            } => {
                tracing::info!("Using public key authentication with key: {}", key_path);
                if let Some(key) = key_pair {
                    tracing::info!("Attempting to authenticate user '{}' with public key", config.username);
                    let result = handle
                        .authenticate_publickey(&config.username, Arc::new(key.clone()))
                        .await;
                    tracing::info!("Public key auth result: {:?}", result);
                    match result {
                        Ok(true) => {
                            tracing::info!("Public key authentication successful");
                            true
                        }
                        Ok(false) => {
                            tracing::warn!(
                                "Public key authentication rejected by server for user '{}'",
                                config.username
                            );
                            false
                        }
                        Err(e) => {
                            tracing::error!("Public key authentication error: {:?}", e);
                            return Err(anyhow!("Public key authentication failed: {:?}", e));
                        }
                    }
                } else {
                    return Err(anyhow!("Failed to load private key"));
                }
            }
        };

        if !auth_success {
            tracing::warn!("Authentication returned false for user {}", config.username);
            return Err(anyhow!("Authentication failed for user {}", config.username));
        }
        tracing::info!("Authentication successful for user {}", config.username);
        Ok(())
    }

    /// Phase 4 of `establish_session`: resolve `ServerInfo` (os/hostname/home)
    /// via `uname`+`hostname`+`echo $HOME`, then probe the remote home dir
    /// through the candidate shells when the first pass came back empty.
    async fn resolve_session_server_info(handle: &Handle<SSHHandler>) -> Option<ServerInfo> {
        // Resolve remote home to an absolute path (SFTP does not expand `~`; never rely on literal `~` in UI).
        let mut server_info = Self::get_server_info_internal(handle).await;
        if server_info
            .as_ref()
            .map(|s| s.home_dir.trim().is_empty())
            .unwrap_or(true)
        {
            if let Some(home) = Self::probe_remote_home_dir(handle).await {
                match &mut server_info {
                    Some(si) => si.home_dir = home,
                    None => {
                        server_info = Some(ServerInfo {
                            os_type: "unknown".to_string(),
                            hostname: "unknown".to_string(),
                            home_dir: home,
                        });
                    }
                }
            }
        }
        server_info
    }

    /// Get server information (partial lines allowed so we can still fill `home_dir` via [`Self::probe_remote_home_dir`]).
    pub(super) async fn get_server_info_internal(handle: &Handle<SSHHandler>) -> Option<ServerInfo> {
        let result = Self::execute_command_internal(
            handle,
            "uname -s && hostname && echo $HOME",
            SSHCommandOptions::default(),
        )
        .await
        .ok()?;

        if result.exit_code != 0 {
            return None;
        }

        let lines: Vec<&str> = result.stdout.trim().lines().collect();
        if lines.is_empty() {
            return None;
        }

        Some(ServerInfo {
            os_type: lines[0].to_string(),
            hostname: lines.get(1).unwrap_or(&"").to_string(),
            home_dir: lines.get(2).unwrap_or(&"").to_string(),
        })
    }

    /// Resolve remote home directory via SSH `exec` (tilde and `$HOME` are expanded by the remote shell).
    pub(super) async fn probe_remote_home_dir(handle: &Handle<SSHHandler>) -> Option<String> {
        const PROBES: &[&str] = &[
            "sh -c 'echo ~'",
            "echo $HOME",
            "bash -lc 'echo ~'",
            "bash -c 'echo ~'",
            "sh -c 'getent passwd \"$(id -un)\" 2>/dev/null | cut -d: -f6'",
        ];
        for cmd in PROBES {
            let Ok(result) = Self::execute_command_internal(handle, cmd, SSHCommandOptions::default()).await else {
                continue;
            };
            if result.exit_code != 0 {
                continue;
            }
            let first = result.stdout.trim().lines().next().unwrap_or("").trim();
            if first.is_empty() || first == "~" {
                continue;
            }
            return Some(first.to_string());
        }
        None
    }

    /// Send SIGINT (or another signal) to a remote-exec channel and drain.
    pub(super) async fn interrupt_exec_channel(session: &russh::Channel<Msg>, signal: Sig) -> anyhow::Result<()> {
        session.signal(signal).await?;
        let _ = session.eof().await;
        Ok(())
    }
}
