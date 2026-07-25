//! skills module — see mod.rs for the wiring entry point.

use super::slint_glue::AppWindow;
use super::*;

/// Phase C.4: build a Slint ModelRc<SkillItem> from the live skill registry,
/// resolving the per-mode enabled state for each skill.
///
/// `mode_id` selects which mode profile's overrides to read. The desktop
/// shell today only ships a single mode (`DEFAULT_MODE_ID` in
/// `flags.rs`); the parameter is in place so a future multi-mode shell
/// can pass through the active mode here without touching the helper.
///
/// Override precedence (matches the storage model in
/// `mode_overrides::set_user_mode_skill_state`):
///   1. `enabled_skills` from user overrides → `true`
///   2. `disabled_skills` from user overrides → `false`
///   3. Otherwise the policy default (`resolve_skill_default_enabled_for_mode`)
pub(super) async fn build_skills_model(mode_id: &str) -> Vec<SkillItem> {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::agents::SkillOverrideEntry;
    use northhing_kernel_api::KernelAgentsApi;

    let facade = kernel_facade();
    let skills = match facade.list_skills().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(target: "app_state", "list_skills failed: {e}");
            return vec![];
        }
    };
    let overrides = match facade.load_skill_overrides().await {
        Ok(o) => o.overrides,
        Err(_) => vec![],
    };

    let enabled_set: std::collections::HashSet<&str> = overrides
        .iter()
        .filter(|o: &&SkillOverrideEntry| o.value == serde_json::Value::Bool(true))
        .map(|o| o.skill_id.as_str())
        .collect();
    let disabled_set: std::collections::HashSet<&str> = overrides
        .iter()
        .filter(|o: &&SkillOverrideEntry| o.value == serde_json::Value::Bool(false))
        .map(|o| o.skill_id.as_str())
        .collect();

    let mut items = Vec::with_capacity(skills.len());
    for skill in &skills {
        let key = skill.id.as_str();
        let enabled = if enabled_set.contains(key) {
            true
        } else if disabled_set.contains(key) {
            false
        } else {
            facade
                .resolve_skill_default_enabled(&skill.id, mode_id)
                .await
                .unwrap_or(false)
        };
        items.push(SkillItem {
            id: SharedString::from(skill.id.clone()),
            name: SharedString::from(skill.name.clone()),
            description: SharedString::from(skill.description.clone()),
            enabled,
        });
    }
    items
}

/// Phase C.4: refresh the Inspector's `skills` model from the live registry.
/// Called once at init and again after `on_toggle_skill` flips a skill, so
/// the UI badge (●) reflects the new state without a manual reload.
///
/// 2026-07-18 (D2j-fix): signature takes `slint::Weak<AppWindow>` so callers
/// on background threads no longer need to `upgrade()` (which returns None on
/// non-UI threads). Data fetch runs on the caller thread; the ModelRc-based
/// UI set is dispatched onto the UI thread via `invoke_from_event_loop`
/// (synchronous only — no nested block_on).
pub(super) async fn refresh_skills_ui(ui_weak: slint::Weak<AppWindow>) {
    let items = build_skills_model(crate::flags::DEFAULT_MODE_ID).await;
    let ui_weak = ui_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_skills(ModelRc::new(VecModel::from(items)));
        }
    });
}
