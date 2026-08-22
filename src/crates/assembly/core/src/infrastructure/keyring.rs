//! OS keyring service contract shared by app shells (Scheme C).
//!
//! Core never persists `api_key` to disk; each app shell resolves keys from
//! the OS keyring at startup and pushes them into core's in-memory model
//! config. Desktop and CLI must address the SAME keyring entries, so the
//! service name lives here as the single source of truth.

/// Keyring service name for per-model API keys.
///
/// Account name is the model/provider config id. Introduced by the desktop
/// (P1-C3) and adopted by the CLI; changing it orphans every stored key.
pub const KEYRING_SERVICE: &str = "northhing.desktop.providers";
