//! SSH client handler with host key verification (Russh `Handler` trait impl)
//!
//! Split out from `manager.rs` in Round 13 (facade + 3 sub-handlers pattern).
//! `SSHHandler` is `pub(crate)` so the facade can construct it as a
//! `Handle<SSHHandler>` for `russh::client::connect_stream`, but it is NOT
//! re-exported outside the `services-integrations` crate.

use crate::remote_ssh::manager_known_hosts::KnownHostEntry;
use async_trait::async_trait;
use russh::client::{DisconnectReason, Handler};
use russh_keys::key::PublicKey;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// SSH client handler with host key verification
pub(crate) struct SSHHandler {
    /// Expected host key (if connecting to known host)
    expected_key: Option<(String, u16, PublicKey)>,
    /// Callback for new host key verification
    verify_callback: Option<Box<HostKeyVerifyCallback>>,
    /// Known hosts storage for verification
    known_hosts: Option<Arc<tokio::sync::RwLock<HashMap<String, KnownHostEntry>>>>,
    /// Host info for known hosts lookup
    host: Option<String>,
    port: Option<u16>,
    /// Stores the real disconnect reason so callers get a useful error message.
    /// russh's run() absorbs errors internally; we capture them here and
    /// surface them after connect_stream() returns.
    /// Uses std::sync::Mutex so it can be read from sync map_err closures.
    disconnect_reason: Arc<Mutex<Option<String>>>,
    /// Shared liveness flag, flipped to false on disconnect so the manager
    /// can detect dead sessions and trigger transparent reconnect.
    alive: Arc<AtomicBool>,
}

type HostKeyVerifyCallback = dyn Fn(String, u16, &PublicKey) -> bool + Send + Sync;

impl SSHHandler {
    // reason: SSHHandler::new() is reserved for the upcoming SSH handshake wiring; today the handler is constructed inline at the connection site
    pub(crate) fn new() -> Self {
        Self {
            expected_key: None,
            verify_callback: None,
            known_hosts: None,
            host: None,
            port: None,
            disconnect_reason: Arc::new(Mutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    // reason: SSHHandler::with_expected_key() is reserved for the upcoming host-key verification API; today's handler is constructed without an expected key
    pub(crate) fn with_expected_key(host: String, port: u16, key: PublicKey) -> Self {
        Self {
            expected_key: Some((host, port, key)),
            verify_callback: None,
            known_hosts: None,
            host: None,
            port: None,
            disconnect_reason: Arc::new(Mutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    // reason: SSHHandler::with_verify_callback() is reserved for the upcoming callback-based host-key verification API
    pub(crate) fn with_verify_callback<F>(callback: F) -> Self
    where
        F: Fn(String, u16, &PublicKey) -> bool + Send + Sync + 'static,
    {
        Self {
            expected_key: None,
            verify_callback: Some(Box::new(callback)),
            known_hosts: None,
            host: None,
            port: None,
            disconnect_reason: Arc::new(Mutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn with_known_hosts(
        host: String,
        port: u16,
        known_hosts: Arc<tokio::sync::RwLock<HashMap<String, KnownHostEntry>>>,
    ) -> (Self, Arc<Mutex<Option<String>>>, Arc<AtomicBool>) {
        let disconnect_reason = Arc::new(Mutex::new(None));
        let alive = Arc::new(AtomicBool::new(true));
        let handler = Self {
            expected_key: None,
            verify_callback: None,
            known_hosts: Some(known_hosts),
            host: Some(host),
            port: Some(port),
            disconnect_reason: disconnect_reason.clone(),
            alive: alive.clone(),
        };
        (handler, disconnect_reason, alive)
    }
}

#[derive(Debug)]
pub(crate) struct HandlerError(String);

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for HandlerError {}

impl From<russh::Error> for HandlerError {
    fn from(e: russh::Error) -> Self {
        HandlerError(format!("{:?}", e))
    }
}

impl From<String> for HandlerError {
    fn from(s: String) -> Self {
        HandlerError(s)
    }
}

#[async_trait]
impl Handler for SSHHandler {
    type Error = HandlerError;

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        let server_fingerprint = server_public_key.fingerprint();

        // 1. If we have an expected key, verify it matches
        if let Some((ref host, port, ref expected)) = self.expected_key {
            if expected.fingerprint() == server_fingerprint {
                tracing::debug!("Server key matches expected key for {}:{}", host, port);
                return Ok(true);
            }
            tracing::warn!(
                "Server key mismatch for {}:{}. Expected fingerprint: {}, got: {}",
                host,
                port,
                expected.fingerprint(),
                server_fingerprint
            );
            return Err(HandlerError(format!(
                "Host key mismatch for {}:{}: expected {}, got {}",
                host,
                port,
                expected.fingerprint(),
                server_fingerprint
            )));
        }

        // 2. Check known_hosts for this host
        if let (Some(host), Some(port)) = (self.host.as_ref(), self.port) {
            if let Some(known_hosts) = self.known_hosts.as_ref() {
                let key = format!("{}:{}", host, port);
                let known_guard = known_hosts.read().await;
                if let Some(known) = known_guard.get(&key) {
                    let stored_fingerprint = known.fingerprint.clone();
                    drop(known_guard);

                    if stored_fingerprint == server_fingerprint {
                        tracing::debug!("Server key verified from known_hosts for {}:{}", host, port);
                        return Ok(true);
                    } else {
                        tracing::warn!(
                            "Host key changed for {}:{}. Expected: {}, got: {}",
                            host,
                            port,
                            stored_fingerprint,
                            server_fingerprint
                        );
                        return Err(HandlerError(format!(
                            "Host key changed for {}:{} — stored fingerprint {} does not match server fingerprint {}. \
                             If the server key was legitimately updated, clear the known host entry and reconnect.",
                            host, port, stored_fingerprint, server_fingerprint
                        )));
                    }
                }
            }
        }

        // 3. If we have a verify callback, use it
        if let Some(ref callback) = self.verify_callback {
            let host = self.host.as_deref().unwrap_or("");
            let port = self.port.unwrap_or(22);
            if callback(host.to_string(), port, server_public_key) {
                tracing::debug!("Server key verified via callback for {}:{}", host, port);
                return Ok(true);
            }
            return Err(HandlerError("Host key rejected by verify callback".to_string()));
        }

        // 4. First time connection - accept the key (like standard SSH client's StrictHostKeyChecking=accept-new)
        // This is safe for development and matches user expectations
        tracing::info!(
            "First time connection - accepting server key. Host: {}, Port: {}, Fingerprint: {}",
            self.host.as_deref().unwrap_or("unknown"),
            self.port.unwrap_or(22),
            server_fingerprint
        );
        Ok(true)
    }

    async fn disconnected(&mut self, reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        let msg = match &reason {
            DisconnectReason::ReceivedDisconnect(info) => {
                format!("Server sent disconnect: {:?} — {}", info.reason_code, info.message)
            }
            DisconnectReason::Error(e) => {
                format!("Connection closed with error: {}", e)
            }
        };
        tracing::warn!(
            "SSH disconnected ({}:{}): {}",
            self.host.as_deref().unwrap_or("?"),
            self.port.unwrap_or(22),
            msg
        );
        if let Ok(mut guard) = self.disconnect_reason.lock() {
            *guard = Some(msg);
        }
        // Flip the shared liveness flag so the manager can detect the dead
        // session and trigger transparent reconnect on the next SFTP/exec call.
        self.alive.store(false, Ordering::SeqCst);
        // Propagate errors so russh surfaces them; swallow clean server disconnect.
        match reason {
            DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            DisconnectReason::Error(e) => Err(e),
        }
    }
}
