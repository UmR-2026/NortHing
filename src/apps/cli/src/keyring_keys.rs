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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_key_wins_over_keyring() {
        assert_eq!(resolve_effective_model_key("any-model", "sk-typed"), "sk-typed");
        // Whitespace-only input counts as empty (form left blank).
        assert_eq!(resolve_effective_model_key("any-model", "  "), "");
    }

    #[test]
    fn missing_keyring_entry_resolves_to_empty() {
        // Uses a random-ish id that no real entry exists for; both
        // "no entry" and "keyring backend unavailable" resolve to empty.
        let id = format!("no-such-model-{}", std::process::id());
        assert_eq!(resolve_effective_model_key(&id, ""), "");
    }
}
