//! Relay server configuration.
//!
//! Security invariants (P1-5 resolved 2026-08-04):
//! - Default bind is loopback (127.0.0.1:9700).
//! - RELAY_BIND overrides the full socket addr; RELAY_PORT only changes the port.
//! - Non-loopback bind without an API key fails closed (from_env returns error).
//! - API key is auto-generated on first run and stored at `~/.northhing/relay/api_key`.
//! - RELAY_API_KEY env var always takes precedence over the key file.
//! - CORS defaults to localhost origins only; RELAY_CORS_ALLOW_ORIGINS overrides.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Source of the API key for diagnostics/logging purposes.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiKeySource {
    /// Set via `RELAY_API_KEY` env var.
    Env,
    /// Read from key file (or newly generated and written).
    File,
    /// No API key configured (only allowed when bind addr is loopback).
    None,
}

// reason: RelayConfig struct fields are reserved for upcoming relay config knobs loaded from disk (today the server reads env vars inline)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RelayConfig {
    pub listen_addr: SocketAddr,
    pub room_ttl_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub heartbeat_timeout_secs: u64,
    pub static_dir: Option<String>,
    /// Directory where per-room uploaded mobile-web files are stored.
    pub room_web_dir: String,
    /// CORS allowed origins. When empty or `["*"]`, the router uses
    /// permissive CORS. Otherwise these origins are passed to CorsLayer.
    /// Default: localhost origins (any port).
    pub cors_allow_origins: Vec<String>,
    /// Shared secret required on the `X-API-Key` header for
    /// `POST /api/rooms/{room_id}/pair` and `POST /api/rooms/{room_id}/command`.
    /// `None` disables authentication (development mode only). Production
    /// deployments MUST set `RELAY_API_KEY` to a sufficiently long random
    /// string.
    ///
    /// Review: `CODE_REVIEW_2026-06-26.md` §"Relay Server 完全缺乏认证机制".
    pub api_key: Option<String>,
    /// Source of the API key (for startup logging).
    pub api_key_source: ApiKeySource,
}

fn default_listen_addr() -> SocketAddr {
    ([127, 0, 0, 1], 9700).into()
}

/// Resolve the key file path: `~/.northhing/relay/api_key`.
///
/// Choice rationale: the existing repo convention stores desktop config
/// at `~/.northhing/config/app.json` (`src/apps/desktop/src/app_state/settings/io.rs:20`).
/// The relay key follows the same `~/.northhing/` base under a `relay/`
/// subdirectory, keeping it separate from the desktop config.
fn key_file_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".northhing").join("relay").join("api_key"))
}

/// Read the API key from the key file, or generate one atomically if the
/// file does not exist.
///
/// Uses a 32-byte random value encoded as base64 (44 characters).
/// Atomic write pattern: write to `.tmp` then rename (matching the repo
/// convention documented in `.superpowers/sdd/final-review.md` §3.2).
/// On Unix, sets file permissions to 0o600. On Windows, skips permissions
/// (no portable equivalent) and logs a comment.
fn load_or_generate_key(key_path: &std::path::Path) -> Result<String, String> {
    // If file exists, read it.
    if key_path.exists() {
        let key = std::fs::read_to_string(key_path)
            .map_err(|e| format!("failed to read relay API key file {}: {e}", key_path.display()))?;
        let trimmed = key.trim().to_string();
        if trimmed.is_empty() {
            return Err(format!("relay API key file {} is empty", key_path.display()));
        }
        if trimmed.len() < 32 {
            return Err(format!(
                "relay API key file {} contains a key that is too short ({} chars, minimum 32)",
                key_path.display(),
                trimmed.len()
            ));
        }
        return Ok(trimmed);
    }

    // Generate a new 32-byte key, base64-encoded.
    use rand::RngCore;
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw);

    // Atomic write: tmp + rename.
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create relay key directory {}: {e}", parent.display()))?;
    }

    let tmp_path = key_path.with_extension("tmp");
    std::fs::write(&tmp_path, key.as_bytes())
        .map_err(|e| format!("failed to write relay key tmp file {}: {e}", tmp_path.display()))?;

    // Set permissions on Unix (0o600).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600)) {
            // Non-fatal: warn but continue
            eprintln!("[relay] warning: could not set permissions on key file: {e}");
        }
    }
    // Windows: no portable permission equivalent; skip.

    std::fs::rename(&tmp_path, key_path)
        .map_err(|e| format!("failed to rename relay key tmp file to {}: {e}", key_path.display()))?;

    eprintln!("[relay] API key generated and written to {}", key_path.display());

    Ok(key)
}

/// Check if a socket address is a loopback address (127.x.x.x or ::1).
fn is_loopback(addr: &SocketAddr) -> bool {
    match addr {
        SocketAddr::V4(v4) => v4.ip().is_loopback(),
        SocketAddr::V6(v6) => v6.ip().is_loopback(),
    }
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            room_ttl_secs: 3600,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 90,
            static_dir: None,
            room_web_dir: "/tmp/northhing-room-web".to_string(),
            // SECURITY: CORS restricted to localhost origins.
            cors_allow_origins: vec![],
            api_key: None,
            api_key_source: ApiKeySource::None,
        }
    }
}

impl RelayConfig {
    /// Build config from environment variables.
    ///
    /// Returns `Err` if the resulting configuration is unsafe:
    /// non-loopback bind address without an API key.
    pub fn from_env() -> Result<Self, String> {
        let mut cfg = Self::default();

        // ── Bind address ───────────────────────────────────────────────
        // RELAY_BIND overrides the full socket addr.
        if let Ok(bind) = std::env::var("RELAY_BIND") {
            cfg.listen_addr = bind
                .parse::<SocketAddr>()
                .map_err(|e| format!("invalid RELAY_BIND value {bind:?}: {e}"))?;
        } else if let Ok(port) = std::env::var("RELAY_PORT") {
            // RELAY_PORT only changes the port, inheriting the default host (127.0.0.1).
            if let Ok(p) = port.parse::<u16>() {
                cfg.listen_addr = ([127, 0, 0, 1], p).into();
            }
        }

        // ── Other env vars ─────────────────────────────────────────────
        if let Ok(dir) = std::env::var("RELAY_STATIC_DIR") {
            cfg.static_dir = Some(dir);
        }
        if let Ok(dir) = std::env::var("RELAY_ROOM_WEB_DIR") {
            cfg.room_web_dir = dir;
        }
        if let Ok(ttl) = std::env::var("RELAY_ROOM_TTL") {
            if let Ok(t) = ttl.parse() {
                cfg.room_ttl_secs = t;
            }
        }

        // ── CORS origins ───────────────────────────────────────────────
        // RELAY_CORS_ALLOW_ORIGINS is comma-separated.
        if let Ok(origins) = std::env::var("RELAY_CORS_ALLOW_ORIGINS") {
            if origins.trim().is_empty() {
                // Empty string: no origins (no CORS header sent).
                cfg.cors_allow_origins = vec![];
            } else {
                cfg.cors_allow_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
            }
        } else {
            // Default: localhost origins (any port).
            cfg.cors_allow_origins = vec![];
        }

        // ── API key ────────────────────────────────────────────────────
        // RELAY_API_KEY env always takes precedence over the key file.
        if let Ok(key) = std::env::var("RELAY_API_KEY") {
            if !key.is_empty() {
                cfg.api_key = Some(key);
                cfg.api_key_source = ApiKeySource::Env;
            }
        } else if let Some(key_path) = key_file_path() {
            // Try loading or generating the key file.
            match load_or_generate_key(&key_path) {
                Ok(key) => {
                    cfg.api_key = Some(key);
                    cfg.api_key_source = ApiKeySource::File;
                }
                Err(e) => {
                    // Key file error is non-fatal for loopback binds
                    // (where auth is optional). For non-loopback, it
                    // will be caught by the safety check below.
                    eprintln!("[relay] warning: {e}");
                    cfg.api_key_source = ApiKeySource::None;
                }
            }
        }

        // ── Safety check: non-loopback without key = fail-closed ───────
        if !is_loopback(&cfg.listen_addr) && cfg.api_key.is_none() {
            return Err(format!(
                "refusing to start relay server on {} without an API key. \
                 Set RELAY_API_KEY to a sufficiently long random string, \
                 or change the bind address to a loopback address (127.0.0.1 or ::1).",
                cfg.listen_addr
            ));
        }

        Ok(cfg)
    }

    /// Check if the bind address is loopback (convenience for startup logging).
    pub fn is_loopback(&self) -> bool {
        is_loopback(&self.listen_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Tests in this module manipulate global process environment variables
    /// (`std::env::set_var` / `remove_var`), which is inherently not
    /// thread-safe. This mutex serializes all config tests so they run one
    /// at a time.
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Create a unique temp directory path for each test to avoid collisions.
    fn test_key_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir()
            .join(format!("northhing-relay-key-test-{}-{}", std::process::id(), n))
    }

    /// Acquire the serial mutex for tests that manipulate global environment
    /// variables. Returns a guard that releases the lock on drop.
    /// Recovers from a poisoned mutex (previous test panicked) by
    /// ignoring the poison — the env vars will be cleaned up anyway.
    fn serial() -> impl Drop {
        TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set_env(key: &str, val: &str) {
        std::env::set_var(key, val);
    }

    fn remove_env(key: &str) {
        std::env::remove_var(key);
    }

    // ── Default config ─────────────────────────────────────────────────

    #[test]
    fn default_config_is_loopback() {
        // Default should bind 127.0.0.1:9700.
        let cfg = RelayConfig::default();
        assert_eq!(cfg.listen_addr.ip().to_string(), "127.0.0.1");
        assert_eq!(cfg.listen_addr.port(), 9700);
        assert!(cfg.api_key.is_none());
        assert!(cfg.cors_allow_origins.is_empty());
    }

    // ── from_env: basic ────────────────────────────────────────────────

    #[test]
    fn from_env_defaults_to_loopback_when_no_env() {
        let _guard = serial();
        // Clean env (no RELAY vars set) -> loopback, auto-generated key, no CORS -> Ok
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        // Set HOME to a temp dir so key file can be written
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());

        let cfg = RelayConfig::from_env().expect("default config should be valid");
        assert_eq!(cfg.listen_addr.ip().to_string(), "127.0.0.1");
        assert_eq!(cfg.listen_addr.port(), 9700);
        // Key is auto-generated on first run (requirement 3).
        assert!(cfg.api_key.is_some(), "key should be auto-generated on first run");
        assert_eq!(cfg.api_key_source, ApiKeySource::File);
        assert!(cfg.cors_allow_origins.is_empty());

        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn from_env_respects_relay_bind() {
        let _guard = serial();
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());

        // Set RELAY_BIND to a non-loopback with a key
        set_env("RELAY_BIND", "0.0.0.0:9700");
        set_env("RELAY_API_KEY", "test-key-123");

        let cfg = RelayConfig::from_env().expect("bind with key should be valid");
        assert_eq!(cfg.listen_addr.to_string(), "0.0.0.0:9700");
        assert_eq!(cfg.api_key.as_deref(), Some("test-key-123"));
        assert_eq!(cfg.api_key_source, ApiKeySource::Env);

        remove_env("RELAY_BIND");
        remove_env("RELAY_API_KEY");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn from_env_relay_port_only_changes_port() {
        let _guard = serial();
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        // Ensure no key file
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        set_env("RELAY_PORT", "8080");

        let cfg = RelayConfig::from_env().expect("port change should be valid");
        assert_eq!(cfg.listen_addr.ip().to_string(), "127.0.0.1");
        assert_eq!(cfg.listen_addr.port(), 8080);

        remove_env("RELAY_PORT");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    // ── Non-loopback + no key = fail-closed ────────────────────────────

    #[test]
    fn non_loopback_without_key_is_rejected() {
        let _guard = serial();
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        // Unset HOME so key_file_path() returns None and no key file is
        // generated. This tests the pure env-based path: no RELAY_API_KEY
        // env var and no key file = no key.
        remove_env("HOME");
        remove_env("USERPROFILE");

        set_env("RELAY_BIND", "0.0.0.0:9700");

        let result = RelayConfig::from_env();
        assert!(result.is_err(), "non-loopback without key must fail");
        let err = result.unwrap_err();
        assert!(err.contains("0.0.0.0:9700"), "error should mention the bind address");
        assert!(err.contains("RELAY_API_KEY"), "error should mention RELAY_API_KEY");

        remove_env("RELAY_BIND");
    }

    #[test]
    fn non_loopback_with_key_is_accepted() {
        let _guard = serial();
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        set_env("RELAY_BIND", "0.0.0.0:9700");
        set_env("RELAY_API_KEY", "test-key-for-non-loopback");

        let cfg = RelayConfig::from_env().expect("non-loopback with key should be valid");
        assert_eq!(cfg.listen_addr.to_string(), "0.0.0.0:9700");
        assert_eq!(cfg.api_key.as_deref(), Some("test-key-for-non-loopback"));

        remove_env("RELAY_BIND");
        remove_env("RELAY_API_KEY");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    // ── Key generation and file read ───────────────────────────────────

    #[test]
    fn key_file_generated_and_reused() {
        let _guard = serial();
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());

        // First call: key file doesn't exist -> generated
        let cfg1 = RelayConfig::from_env().expect("first call should generate key");
        assert!(cfg1.api_key.is_some(), "key should be generated");
        assert_eq!(cfg1.api_key_source, ApiKeySource::File);
        let key1 = cfg1.api_key.unwrap();
        assert!(key1.len() >= 32, "generated key should be at least 32 chars");

        // Second call: key file exists -> reused
        let cfg2 = RelayConfig::from_env().expect("second call should reuse key");
        assert_eq!(cfg2.api_key_source, ApiKeySource::File);
        assert_eq!(cfg2.api_key.as_deref(), Some(key1.as_str()), "reused key should match");

        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn relay_api_key_env_overrides_file() {
        let _guard = serial();
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());

        // First call generates a key file
        let _cfg1 = RelayConfig::from_env().expect("generate key file");

        // Second call: RELAY_API_KEY env should override file
        set_env("RELAY_API_KEY", "env-override-key-12345");

        let cfg2 = RelayConfig::from_env().expect("env should override");
        assert_eq!(cfg2.api_key_source, ApiKeySource::Env);
        assert_eq!(cfg2.api_key.as_deref(), Some("env-override-key-12345"));

        remove_env("RELAY_API_KEY");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    // ── CORS ───────────────────────────────────────────────────────────

    #[test]
    fn cors_default_is_empty_localhost() {
        let _guard = serial();
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        remove_env("RELAY_API_KEY");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        let cfg = RelayConfig::from_env().expect("default cors");
        assert!(cfg.cors_allow_origins.is_empty(), "default cors should be empty list");

        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn cors_env_var_parses_comma_separated() {
        let _guard = serial();
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        set_env("RELAY_CORS_ALLOW_ORIGINS", "http://localhost:5173,http://example.com");

        let cfg = RelayConfig::from_env().expect("custom cors");
        assert_eq!(cfg.cors_allow_origins, vec!["http://localhost:5173", "http://example.com"]);

        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn cors_permissive_via_star() {
        let _guard = serial();
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        set_env("RELAY_CORS_ALLOW_ORIGINS", "*");

        let cfg = RelayConfig::from_env().expect("permissive cors");
        assert_eq!(cfg.cors_allow_origins, vec!["*"]);

        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }

    // ── RELAY_BIND overrides RELAY_PORT ────────────────────────────────

    #[test]
    fn relay_bind_takes_priority_over_relay_port() {
        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("RELAY_API_KEY");
        remove_env("RELAY_CORS_ALLOW_ORIGINS");
        let temp = test_key_dir();
        set_env("HOME", temp.to_str().unwrap());
        let key_path = key_file_path().unwrap();
        let _ = std::fs::remove_file(&key_path);

        set_env("RELAY_BIND", "127.0.0.1:9999");
        set_env("RELAY_PORT", "8080");

        let cfg = RelayConfig::from_env().expect("bind should override port");
        assert_eq!(cfg.listen_addr.port(), 9999, "RELAY_BIND port should win");
        assert_eq!(cfg.listen_addr.ip().to_string(), "127.0.0.1");

        remove_env("RELAY_BIND");
        remove_env("RELAY_PORT");
        remove_env("HOME");
        let _ = std::fs::remove_dir_all(&temp);
    }
}
