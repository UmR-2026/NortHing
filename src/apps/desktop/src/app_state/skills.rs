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
    use northhing_core::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode;
    use northhing_core::agentic::tools::implementations::skills::{
        mode_overrides::load_user_mode_skill_overrides, skill_registry,
    };

    let registry = skill_registry();
    let skills = registry.get_all_skills().await;
    let overrides = load_user_mode_skill_overrides(mode_id).await.unwrap_or_default();

    let enabled_set: std::collections::HashSet<&str> = overrides.enabled_skills.iter().map(String::as_str).collect();
    let disabled_set: std::collections::HashSet<&str> = overrides.disabled_skills.iter().map(String::as_str).collect();

    skills
        .into_iter()
        .map(|skill| {
            let key = skill.key.as_str();
            let enabled = if enabled_set.contains(key) {
                true
            } else if disabled_set.contains(key) {
                false
            } else {
                resolve_skill_default_enabled_for_mode(&skill, mode_id)
            };
            SkillItem {
                id: SharedString::from(skill.key.clone()),
                name: SharedString::from(skill.name.clone()),
                description: SharedString::from(skill.description.clone()),
                enabled,
            }
        })
        .collect()
}

/// Phase C.4: refresh the Inspector's `skills` model from the live registry.
/// Called once at init and again after `on_toggle_skill` flips a skill, so
/// the UI badge (●) reflects the new state without a manual reload.
pub(super) async fn refresh_skills_ui(ui: &AppWindow) {
    let items = build_skills_model(crate::flags::DEFAULT_MODE_ID).await;
    ui.set_skills(ModelRc::new(VecModel::from(items)));
}
