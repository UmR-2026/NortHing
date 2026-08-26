//! OS keyring abstraction for secure API key storage (C3, P1-2).
//!
//! ## Architecture
//!
//! - [`KeyringBackend`] trait: `store` / `get` / `delete` operations.
//! - [`ProductionKeyring`]: wraps the `keyring` crate (real OS keyring).
//! - [`MockKeyring`]: `HashMap`-based, used in tests (available in all builds
//!   so tests don't need `cfg(test)` conditionals on the trait itself).
//!
//! ## Sentinel
//!
//! When a provider's API key is migrated to the OS keyring, the
//! `ProviderConfig.api_key` field is replaced with [`API_KEY_SENTINEL`] before
//! serialization to disk. At load time, code that needs the actual key calls
//! [`resolve_api_key`]; at save time, [`store_api_key`] handles the keyring
//! write and returns the sentinel for the in-memory field.
//!
//! ## Fail-closed
//!
//! Any keyring operation that fails (OS keychain unavailable, credential not
//! found, etc.) propagates an `Err` — the caller must never silently fall back
//! to plaintext storage.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Mutex;

// ===== Keyring service identity =====

// Single source of truth is `northhing_core::infrastructure::keyring::KEYRING_SERVICE`
// so the desktop and the CLI address the same OS keychain entries.
use northhing_core::infrastructure::keyring::KEYRING_SERVICE;

// ===== Sentinel =====

/// Sentinel value written to `ProviderConfig.api_key` on disk when the real
/// API key has been moved to the OS keyring.
///
/// ## Why `"__kr__"`?
///
/// - **Short**: 6 chars — no measurable serialization overhead.
/// - **Unambiguous**: No real API key starts with `__kr__` (API keys from
///   Anthropic / OpenAI / Gemini use `sk-`, `api-`, `AIza` etc.). Even if one
///   did, the load-time migration checks for exact equality, not prefix match.
/// - **ASCII-only**: Safe across all filesystems and encodings.
/// - **Readable**: A human reading `app.json` can immediately tell the key is
///   stored elsewhere.
///
/// Alternatives considered:
/// - Empty string `""` — indistinguishable from "no key configured".
/// - `null` — would require changing the field to `Option<String>` (breaking
///   schema compat).
/// - A UUID — adds 36 chars for no benefit.
/// - Opaque base64 — hurts debugging for no security gain (it's a sentinel,
///   not a secret).
#[allow(dead_code)]
pub const API_KEY_SENTINEL: &str = "__kr__";

/// Sentinel value written to `MCPServerConfig.env` on disk when the real
/// environment variables have been moved to the OS keyring (P1c, P1-8).
#[allow(dead_code)]
pub const MCP_ENV_SENTINEL: &str = "__kr_env__";

/// Returns `true` when `s` is the keyring sentinel value.
#[allow(dead_code)]
pub fn is_keyring_sentinel(s: &str) -> bool {
    s == API_KEY_SENTINEL
}

/// Returns `true` when `s` is the MCP env keyring sentinel value.
#[allow(dead_code)]
pub fn is_mcp_env_sentinel(s: &str) -> bool {
    s == MCP_ENV_SENTINEL
}

/// Returns `true` when `env` map represents a keyring sentinel placeholder.
#[allow(dead_code)]
pub fn is_env_sentinel(env: &HashMap<String, String>) -> bool {
    env.contains_key(MCP_ENV_SENTINEL)
}

/// Helper to create a sentinel env map for on-disk persistence.
#[allow(dead_code)]
pub fn make_env_sentinel() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(MCP_ENV_SENTINEL.to_string(), "true".to_string());
    map
}

// ===== Backend trait =====

/// Abstraction over OS keyring operations.
///
/// Production impl wraps the `keyring` crate; mock impl stores in a
/// `HashMap` for testing. Both are available in all builds — tests
/// construct [`MockKeyring`] directly without conditional compilation.
pub trait KeyringBackend: Send + Sync + std::fmt::Debug {
    /// Store `secret` under `account` in the keyring.
    fn store(&self, account: &str, secret: &str) -> Result<()>;
    /// Retrieve the secret stored under `account`.
    fn get(&self, account: &str) -> Result<String>;
    /// Delete the credential stored under `account`.
    fn delete(&self, account: &str) -> Result<()>;
}

// ===== Production impl =====

/// Production keyring backend wrapping the `keyring` crate.
///
/// All operations use `KEYRING_SERVICE` as the service name and the
/// provider UUID as the account name.
#[derive(Debug)]
pub struct ProductionKeyring;

impl KeyringBackend for ProductionKeyring {
    fn store(&self, account: &str, secret: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, account)
            .with_context(|| format!("keyring: failed to open entry for '{account}'"))?;
        entry
            .set_password(secret)
            .with_context(|| format!("keyring: failed to store credential for '{account}'"))?;
        Ok(())
    }

    fn get(&self, account: &str) -> Result<String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, account)
            .with_context(|| format!("keyring: failed to open entry for '{account}'"))?;
        let secret = entry
            .get_password()
            .with_context(|| format!("keyring: failed to read credential for '{account}'"))?;
        Ok(secret)
    }

    fn delete(&self, account: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, account)
            .with_context(|| format!("keyring: failed to open entry for '{account}'"))?;
        entry
            .delete_credential()
            .with_context(|| format!("keyring: failed to delete credential for '{account}'"))?;
        Ok(())
    }
}

// ===== Mock impl (test use) =====

/// Mock keyring backend backed by a `HashMap<String, String>`.
///
/// Thread-safe via `std::sync::Mutex`. Available in all builds so tests
/// can construct it directly — no `#[cfg(test)]` gates needed on the trait.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct MockKeyring {
    store: Mutex<HashMap<String, String>>,
}

#[allow(dead_code)]
impl MockKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a credential into the mock store (test convenience).
    pub fn seed(&self, account: &str, secret: &str) {
        let mut map = self.store.lock().unwrap();
        map.insert(account.to_string(), secret.to_string());
    }

    /// Assert that a credential exists in the mock store (test convenience).
    pub fn assert_contains(&self, account: &str, expected: &str) {
        let map = self.store.lock().unwrap();
        let actual = map.get(account).expect("credential not found in mock keyring");
        assert_eq!(actual, expected, "mock keyring credential mismatch for '{account}'");
    }

    /// Assert that a credential does NOT exist in the mock store (test
    /// convenience — used to verify deletion / sentinel replacement).
    pub fn assert_not_contains(&self, account: &str) {
        let map = self.store.lock().unwrap();
        assert!(
            !map.contains_key(account),
            "credential '{account}' should not exist in mock keyring"
        );
    }
}

impl KeyringBackend for MockKeyring {
    fn store(&self, account: &str, secret: &str) -> Result<()> {
        let mut map = self.store.lock().unwrap();
        map.insert(account.to_string(), secret.to_string());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<String> {
        let map = self.store.lock().unwrap();
        map.get(account)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("keyring: credential not found for '{account}'"))
    }

    fn delete(&self, account: &str) -> Result<()> {
        let mut map = self.store.lock().unwrap();
        map.remove(account);
        Ok(())
    }
}

// ===== Global production backend =====

use once_cell::sync::Lazy;

/// Global production keyring backend used by the IO and sync paths.
///
/// Tests construct their own [`MockKeyring`] and pass it directly to the
/// test variants of the IO functions — they never touch this global.
pub(crate) static PRODUCTION_KEYRING: Lazy<ProductionKeyring> = Lazy::new(|| ProductionKeyring);

// ===== High-level helpers =====

/// Resolve the actual API key for a provider.
///
/// If the provider's `api_key` field is the sentinel, the real key is
/// fetched from `keyring`; otherwise the field value is returned as-is
/// (empty string means no key configured; non-empty non-sentinel means
/// a plaintext key that hasn't been migrated yet — handled at load time).
#[allow(dead_code)]
pub fn resolve_api_key(keyring: &dyn KeyringBackend, provider_id: &str, api_key_field: &str) -> Result<String> {
    if is_keyring_sentinel(api_key_field) {
        keyring.get(provider_id)
    } else {
        Ok(api_key_field.to_string())
    }
}

/// Store an API key in the keyring and return the sentinel value for
/// in-memory / on-disk storage.
///
/// When `plaintext` is empty or already the sentinel, the keyring is
/// not touched and the input is returned as-is (idempotent).
///
/// ## Errors
///
/// Returns `Err` when the keyring is unavailable (fail-closed) — the
/// caller must abort and not write plaintext to disk.
#[allow(dead_code)]
pub fn store_api_key(keyring: &dyn KeyringBackend, provider_id: &str, plaintext: &str) -> Result<String> {
    if plaintext.is_empty() || is_keyring_sentinel(plaintext) {
        return Ok(plaintext.to_string());
    }
    keyring.store(provider_id, plaintext)?;
    Ok(API_KEY_SENTINEL.to_string())
}

/// Remove a provider's API key from the keyring.
///
/// This is a best-effort cleanup — the provider may not have a keyring
/// entry (e.g. never had a key configured). The function returns `Ok(())`
/// in both cases so callers don't need to handle the "not found" case
/// specially.
pub fn delete_api_key(keyring: &dyn KeyringBackend, provider_id: &str) -> Result<()> {
    match keyring.delete(provider_id) {
        Ok(()) => Ok(()),
        // Credential not found → nothing to clean up, not an error.
        Err(_) => Ok(()),
    }
}

// ===== MCP Env Keyring Helpers (P1c, P1-8) =====

/// Store an entire MCP server's env map in the OS keyring under `mcp-env:{server_id}`
/// and return the sentinel value for on-disk persistence.
///
/// If `env` already contains the sentinel, the keyring is not touched and the
/// sentinel is returned (idempotent).
///
/// ## Errors
///
/// Returns `Err` when the keyring is unavailable (fail-closed) or serialization fails.
pub fn store_env(
    keyring: &dyn KeyringBackend,
    server_id: &str,
    env: &HashMap<String, String>,
) -> Result<String> {
    if is_env_sentinel(env) {
        return Ok(MCP_ENV_SENTINEL.to_string());
    }
    let account = format!("mcp-env:{server_id}");
    let json = serde_json::to_string(env).context("failed to serialize MCP env to JSON")?;
    keyring.store(&account, &json)?;
    Ok(MCP_ENV_SENTINEL.to_string())
}

/// Restore an MCP server's env map from the OS keyring under `mcp-env:{server_id}`.
///
/// ## Fail-open
///
/// If the keyring entry is missing or JSON parsing fails, logs a warning and
/// returns `Ok(HashMap::new())` (backward-compatible with old/missing data).
pub fn load_env(keyring: &dyn KeyringBackend, server_id: &str) -> Result<HashMap<String, String>> {
    let account = format!("mcp-env:{server_id}");
    match keyring.get(&account) {
        Ok(secret) => match serde_json::from_str::<HashMap<String, String>>(&secret) {
            Ok(map) => Ok(map),
            Err(e) => {
                tracing::warn!(
                    target: "app_state",
                    "keyring: failed to parse MCP env JSON for '{server_id}': {e}"
                );
                Ok(HashMap::new())
            }
        },
        Err(e) => {
            tracing::warn!(
                target: "app_state",
                "keyring: failed to load MCP env for '{server_id}' (fail-open): {e}"
            );
            Ok(HashMap::new())
        }
    }
}

/// Resolve the actual env map for an MCP server.
///
/// If `env` contains the sentinel, the real env map is fetched from `keyring`
/// via [`load_env`]; otherwise `env` is cloned and returned as-is (plaintext).
#[allow(dead_code)]
pub fn resolve_env(
    keyring: &dyn KeyringBackend,
    server_id: &str,
    env: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    if is_env_sentinel(env) {
        load_env(keyring, server_id)
    } else {
        Ok(env.clone())
    }
}

/// Remove an MCP server's env entry from the keyring (best-effort).
#[allow(dead_code)]
pub fn delete_env(keyring: &dyn KeyringBackend, server_id: &str) -> Result<()> {
    let account = format!("mcp-env:{server_id}");
    match keyring.delete(&account) {
        Ok(()) => Ok(()),
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_identity() {
        assert!(is_keyring_sentinel(API_KEY_SENTINEL));
        assert!(!is_keyring_sentinel("sk-real-key-12345"));
        assert!(!is_keyring_sentinel(""));
        assert!(!is_keyring_sentinel("__kr__ ")); // trailing space

        assert!(is_mcp_env_sentinel(MCP_ENV_SENTINEL));
        assert!(!is_mcp_env_sentinel(""));
        assert!(!is_mcp_env_sentinel("__kr__"));

        let mut map = HashMap::new();
        assert!(!is_env_sentinel(&map));
        map.insert("FOO".to_string(), "BAR".to_string());
        assert!(!is_env_sentinel(&map));

        let sentinel_map = make_env_sentinel();
        assert!(is_env_sentinel(&sentinel_map));
    }

    #[test]
    fn mock_keyring_store_get() {
        let kr = MockKeyring::new();
        kr.store("p1", "sk-secret").unwrap();
        assert_eq!(kr.get("p1").unwrap(), "sk-secret");
    }

    #[test]
    fn mock_keyring_get_missing_returns_err() {
        let kr = MockKeyring::new();
        let err = kr.get("nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn mock_keyring_delete_removes_entry() {
        let kr = MockKeyring::new();
        kr.store("p1", "sk-secret").unwrap();
        kr.delete("p1").unwrap();
        assert!(kr.get("p1").is_err());
    }

    #[test]
    fn mock_keyring_delete_missing_does_not_error() {
        let kr = MockKeyring::new();
        kr.delete("nonexistent").unwrap(); // should not panic
    }

    #[test]
    fn resolve_api_key_returns_sentinel_from_keyring() {
        let kr = MockKeyring::new();
        kr.store("p1", "sk-real").unwrap();
        let result = resolve_api_key(&kr, "p1", API_KEY_SENTINEL).unwrap();
        assert_eq!(result, "sk-real");
    }

    #[test]
    fn resolve_api_key_returns_plaintext_directly() {
        let kr = MockKeyring::new();
        let result = resolve_api_key(&kr, "p1", "sk-plain").unwrap();
        assert_eq!(result, "sk-plain");
    }

    #[test]
    fn resolve_api_key_returns_empty_string_as_is() {
        let kr = MockKeyring::new();
        let result = resolve_api_key(&kr, "p1", "").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn resolve_api_key_sentinel_missing_keyring_returns_err() {
        let kr = MockKeyring::new();
        let result = resolve_api_key(&kr, "p1", API_KEY_SENTINEL);
        assert!(result.is_err(), "sentinel without keyring entry must fail");
    }

    #[test]
    fn store_api_key_empty_is_noop() {
        let kr = MockKeyring::new();
        let result = store_api_key(&kr, "p1", "").unwrap();
        assert_eq!(result, "");
        assert!(kr.get("p1").is_err(), "empty key must not be stored");
    }

    #[test]
    fn store_api_key_sentinel_is_noop() {
        let kr = MockKeyring::new();
        let result = store_api_key(&kr, "p1", API_KEY_SENTINEL).unwrap();
        assert_eq!(result, API_KEY_SENTINEL);
        assert!(kr.get("p1").is_err(), "sentinel must not be stored as real key");
    }

    #[test]
    fn store_api_key_returns_sentinel() {
        let kr = MockKeyring::new();
        let result = store_api_key(&kr, "p1", "sk-real").unwrap();
        assert_eq!(result, API_KEY_SENTINEL);
        assert_eq!(kr.get("p1").unwrap(), "sk-real");
    }

    #[test]
    fn delete_api_key_best_effort_missing() {
        let kr = MockKeyring::new();
        delete_api_key(&kr, "nonexistent").unwrap(); // not an error
    }

    #[test]
    fn delete_api_key_removes_existing() {
        let kr = MockKeyring::new();
        kr.store("p1", "sk-real").unwrap();
        delete_api_key(&kr, "p1").unwrap();
        assert!(kr.get("p1").is_err());
    }

    #[test]
    fn mock_seed_and_assert_helpers() {
        let kr = MockKeyring::new();
        kr.seed("p1", "sk-preloaded");
        kr.assert_contains("p1", "sk-preloaded");
    }

    #[test]
    fn mock_keyring_store_load_env_roundtrip() {
        let kr = MockKeyring::new();
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "sk-mcp-secret".to_string());
        env.insert("PORT".to_string(), "9000".to_string());

        let sentinel = store_env(&kr, "srv1", &env).unwrap();
        assert_eq!(sentinel, MCP_ENV_SENTINEL);

        let loaded = load_env(&kr, "srv1").unwrap();
        assert_eq!(loaded, env);
    }

    #[test]
    fn mock_keyring_store_env_sentinel_is_noop() {
        let kr = MockKeyring::new();
        let sentinel_map = make_env_sentinel();
        let result = store_env(&kr, "srv1", &sentinel_map).unwrap();
        assert_eq!(result, MCP_ENV_SENTINEL);
        assert!(kr.get("mcp-env:srv1").is_err(), "sentinel map must not be written to keyring");
    }

    #[test]
    fn mock_keyring_load_env_missing_returns_empty_map_fail_open() {
        let kr = MockKeyring::new();
        let result = load_env(&kr, "nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open() {
        let kr = MockKeyring::new();
        kr.store("mcp-env:corrupt", "not a valid json map").unwrap();
        let result = load_env(&kr, "corrupt");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn mock_keyring_resolve_env_sentinel_and_plaintext() {
        let kr = MockKeyring::new();
        let mut real_env = HashMap::new();
        real_env.insert("KEY".to_string(), "secret".to_string());
        store_env(&kr, "srv1", &real_env).unwrap();

        // Sentinel env resolves from keyring
        let sentinel_map = make_env_sentinel();
        let resolved = resolve_env(&kr, "srv1", &sentinel_map).unwrap();
        assert_eq!(resolved, real_env);

        // Plaintext env returns directly
        let mut plain_env = HashMap::new();
        plain_env.insert("UNSAVED".to_string(), "direct".to_string());
        let resolved_plain = resolve_env(&kr, "srv2", &plain_env).unwrap();
        assert_eq!(resolved_plain, plain_env);
    }

    #[test]
    fn mock_keyring_delete_env_removes_entry() {
        let kr = MockKeyring::new();
        let mut env = HashMap::new();
        env.insert("K".to_string(), "V".to_string());
        store_env(&kr, "srv1", &env).unwrap();
        assert!(kr.get("mcp-env:srv1").is_ok());

        delete_env(&kr, "srv1").unwrap();
        assert!(kr.get("mcp-env:srv1").is_err());

        // Deleting non-existent is best-effort Ok
        delete_env(&kr, "nonexistent").unwrap();
    }
}
