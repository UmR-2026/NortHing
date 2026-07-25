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
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::KernelSettingsApi;
    use std::collections::BTreeSet;

    let facade = kernel_facade();
    let config = match facade.get_global_config().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Phase C.3: failed to read global config: {e}");
            return "Model: Not configured".to_string();
        }
    };

    // Collect unique enabled providers (case-insensitive on the storage side,
    // but we sort lexicographically for stable UI rendering).
    let mut providers: BTreeSet<String> = BTreeSet::new();
    for model in &config.providers {
        if model.enabled != Some(true) {
            continue;
        }
        if let Some(ref pt) = model.provider_type {
            let trimmed = pt.trim();
            if !trimmed.is_empty() {
                providers.insert(trimmed.to_string());
            }
        }
    }

    if providers.is_empty() {
        return "Model: Not configured".to_string();
    }

    format!("Model: {}", providers.into_iter().collect::<Vec<_>>().join(", "))
}
