//! Relay server configuration.

use std::net::SocketAddr;

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
    pub cors_allow_origins: Vec<String>,
    /// Shared secret required on the `X-API-Key` header for
    /// `POST /api/rooms/{room_id}/pair` and `POST /api/rooms/{room_id}/command`.
    /// `None` disables authentication (development mode only). Production
    /// deployments MUST set `RELAY_API_KEY` to a sufficiently long random
    /// string.
    ///
    /// Review: `CODE_REVIEW_2026-06-26.md` §"Relay Server 完全缺乏认证机制".
    pub api_key: Option<String>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            listen_addr: ([0, 0, 0, 0], 9700).into(),
            room_ttl_secs: 3600,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 90,
            static_dir: None,
            room_web_dir: "/tmp/northhing-room-web".to_string(),
            // SECURITY: wildcard CORS — acceptable for local dev,
            // must be restricted in production deployment.
            cors_allow_origins: vec!["*".to_string()],
            api_key: None,
        }
    }
}

impl RelayConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(port) = std::env::var("RELAY_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                cfg.listen_addr = ([0, 0, 0, 0], p).into();
            }
        }
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
        // 2026-06-26: `RELAY_API_KEY` enables API key auth on the
        // pair/command endpoints. Empty / unset keeps the server in
        // open mode (dev only). The `pair`/`command` handlers short-
        // circuit to 401 when this is `Some(_)` and the client doesn't
        // send the matching header.
        if let Ok(key) = std::env::var("RELAY_API_KEY") {
            if !key.is_empty() {
                cfg.api_key = Some(key);
            }
        }
        cfg
    }
}
