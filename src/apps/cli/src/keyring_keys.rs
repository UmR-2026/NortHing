//! Scheme C key bridge for the CLI (parity with the desktop push in
//! `app_state::settings::sync.rs`).
//!
//! Core never persists `api_key` to disk; it lands in core's in-memory model
//! config only when a shell resolves it from the OS keyring and pushes it.
//! The CLI does that once at startup — right after the global config service
//! initializes and before the AI client factory comes up — so every client
//! built later sees the keys. Model add/edit/delete flows keep the keyring in
//! sync so keys entered in the CLI survive restarts.

use anyhow::{Context, Result};
use northhing_core::infrastructure::keyring::KEYRING_SERVICE;

/// Read the stored API key for `model_id` from the OS keyring.
/// `Ok(None)` means "no entry" (treated as no key, not an error).
fn keyring_get(model_id: &str) -> Result<Option<String>> {
    #[cfg(test)]
    if let Some(found) = mock_keyring::get(model_id) {
        return Ok(found);
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, model_id)
        .with_context(|| format!("keyring: failed to open entry for '{model_id}'"))?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(
            "keyring: failed to read credential for '{model_id}': {e}"
        )),
    }
}

/// Store `secret` for `model_id`; an empty secret deletes the entry instead.
pub fn store_model_key(model_id: &str, secret: &str) -> Result<()> {
    #[cfg(test)]
    if mock_keyring::store(model_id, secret) {
        return Ok(());
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, model_id)
        .with_context(|| format!("keyring: failed to open entry for '{model_id}'"))?;
    if secret.is_empty() {
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(
                "keyring: failed to delete credential for '{model_id}': {e}"
            )),
        }
    } else {
        entry
            .set_password(secret)
            .with_context(|| format!("keyring: failed to store credential for '{model_id}"))?;
        Ok(())
    }
}

/// Resolve the effective key for a model form save: a typed key wins; an
/// empty form field inherits the stored keyring entry (desktop
/// `resolve_effective_api_key` parity).
pub fn resolve_effective_model_key(model_id: &str, typed: &str) -> String {
    if typed.trim().is_empty() {
        keyring_get(model_id).ok().flatten().unwrap_or_default()
    } else {
        typed.to_string()
    }
}

/// Push keyring-resolved keys into core's in-memory model configs (startup,
/// best-effort: any failure logs a warning and keeps the model key-less in
/// memory rather than aborting the CLI boot).
pub async fn push_keyring_keys_into_core() {
    let cfg = match northhing_core::service::config::get_global_config_service().await {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!("Scheme C keyring push skipped: config service unavailable: {e}");
            return;
        }
    };
    let models = match cfg.get_ai_models().await {
        Ok(models) => models,
        Err(e) => {
            tracing::warn!("Scheme C keyring push skipped: failed to list models: {e}");
            return;
        }
    };
    let mut pushed = 0usize;
    for mut model in models {
        let key = match keyring_get(&model.id) {
            Ok(Some(key)) if !key.is_empty() => key,
            _ => continue,
        };
        if model.api_key == key {
            continue;
        }
        let model_id = model.id.clone();
        model.api_key = key;
        match cfg.update_ai_model(&model_id, model).await {
            Ok(()) => pushed += 1,
            Err(e) => tracing::warn!("Scheme C keyring push failed for model '{model_id}': {e}"),
        }
    }
    tracing::info!("Scheme C keyring push complete: {pushed} model key(s) resolved into core memory");
}

/// Test-only in-memory keyring so unit tests never consult the OS keyring
/// (red line: `cmdkey /list` output must be identical before and after a
/// test run, and CI must not need a keyring backend). Thread-local + RAII
/// guard, same isolation shape as core's `with_test_memory_db_path`. The
/// `keyring` crate 4.x exposes no runtime-replaceable mock for its v1
/// `Entry` (the platform store is installed once, unconditionally, on the
/// first call), so this module-level seam is the minimal abstraction.
#[cfg(test)]
mod mock_keyring {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static STORE: RefCell<Option<HashMap<String, String>>> = RefCell::new(None);
    }

    pub(super) struct MockKeyringGuard {
        prev: Option<HashMap<String, String>>,
    }

    impl Drop for MockKeyringGuard {
        fn drop(&mut self) {
            STORE.with(|s| *s.borrow_mut() = self.prev.take());
        }
    }

    /// Activate the mock for the calling thread until the guard is dropped.
    pub(super) fn with_test_keyring() -> MockKeyringGuard {
        let prev = STORE.with(|s| s.borrow_mut().replace(HashMap::new()));
        MockKeyringGuard { prev }
    }

    /// `Some(None)` = mock active with no entry; `None` = mock inactive.
    pub(super) fn get(model_id: &str) -> Option<Option<String>> {
        STORE.with(|s| s.borrow().as_ref().map(|store| store.get(model_id).cloned()))
    }

    /// Returns `false` when the mock is inactive (caller falls through to
    /// the real keyring). An empty `secret` deletes the entry, matching the
    /// production contract.
    pub(super) fn store(model_id: &str, secret: &str) -> bool {
        STORE.with(|s| match &mut *s.borrow_mut() {
            Some(store) => {
                if secret.is_empty() {
                    store.remove(model_id);
                } else {
                    store.insert(model_id.to_string(), secret.to_string());
                }
                true
            }
            None => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::mock_keyring::with_test_keyring;
    use super::*;

    #[test]
    fn typed_key_wins_over_keyring() {
        let _kr = with_test_keyring();
        assert_eq!(resolve_effective_model_key("any-model", "sk-typed"), "sk-typed");
        // Whitespace-only input counts as empty (form left blank); the mock
        // keyring holds no entry for "any-model", so it resolves to empty.
        assert_eq!(resolve_effective_model_key("any-model", "  "), "");
    }

    #[test]
    fn missing_keyring_entry_resolves_to_empty() {
        // Mock keyring: no entry stored, no real OS keyring consulted.
        let _kr = with_test_keyring();
        let id = format!("no-such-model-{}", std::process::id());
        assert_eq!(resolve_effective_model_key(&id, ""), "");
    }

    #[test]
    fn chat_edit_path_resolve_contract() {
        // Regression for W11-3: `ChatMode::update_existing_model` (in
        // `modes/chat/model_config.rs`) must route the form's api_key
        // field through this helper so a blank field inherits the
        // stored keyring key rather than wiping it. Locks both arms at
        // the call site against a mock keyring (no real entries touched).
        let _kr = with_test_keyring();
        let id = format!("w11-3-edit-path-{}", std::process::id());
        store_model_key(&id, "sk-stored").unwrap();
        // Arm 1 — blank form field on edit inherits the stored key.
        assert_eq!(resolve_effective_model_key(&id, ""), "sk-stored");
        // Arm 2 — typed key always wins, even alongside a stored entry.
        assert_eq!(resolve_effective_model_key(&id, "sk-typed"), "sk-typed");
        // Storing an empty secret deletes (round-trip via the mock).
        store_model_key(&id, "").unwrap();
        assert_eq!(resolve_effective_model_key(&id, ""), "");
    }
}
