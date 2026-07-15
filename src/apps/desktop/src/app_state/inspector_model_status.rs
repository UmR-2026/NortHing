//! inspector_model_status module — see mod.rs for the wiring entry point.

use super::*;

/// Phase C.3: build the Inspector `model-status` string from the live
/// global config. Returns `"Model: Not configured"` when no providers are
/// configured, otherwise `"Model: <p1>, <p2>, ... (n)"` with the unique
/// enabled provider ids sorted alphabetically for stable rendering.
///
/// The 3 providers today are listed in
/// `.agents/reference/_upstream/northhing-a5-providers.md` (Anthropic,
/// Gemini, OpenAI-compatible). We surface whatever is actually enabled in
/// the user's `GlobalConfig.ai.models` so the displayed set stays honest.
pub(super) async fn build_model_status_string() -> String {
    use std::collections::BTreeSet;

    let config_service = match northhing_core::service::config::get_global_config_service().await {
        Ok(svc) => svc,
        Err(e) => {
            eprintln!("Phase C.3: failed to fetch global config service: {e}");
            return "Model: Not configured".to_string();
        }
    };

    // `None` path == use the user's primary config (no per-workspace override).
    let config: Result<northhing_core::service::config::GlobalConfig, _> = config_service.config(None).await;
    let config = match config {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Phase C.3: failed to read global config: {e}");
            return "Model: Not configured".to_string();
        }
    };

    // Collect unique enabled providers (case-insensitive on the storage side,
    // but we sort lexicographically for stable UI rendering).
    let mut providers: BTreeSet<String> = BTreeSet::new();
    for model in &config.ai.models {
        if !model.enabled {
            continue;
        }
        let trimmed = model.provider.trim();
        if !trimmed.is_empty() {
            providers.insert(trimmed.to_string());
        }
    }

    if providers.is_empty() {
        return "Model: Not configured".to_string();
    }

    format!("Model: {}", providers.into_iter().collect::<Vec<_>>().join(", "))
}
