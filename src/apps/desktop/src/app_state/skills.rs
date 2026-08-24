//! skills module — see mod.rs for the wiring entry point.

use super::slint_glue::AppWindow;
use super::*;
use std::sync::Arc;

/// T4 §10.1: pure desktop-side function deriving a skill's partition
/// `category` from its `SkillInfoDto.id` (= registry `SkillInfo.key` =
/// `{prefix}::{slot}::{dir_name}`). The DTO does not expose `group_key`
/// or `dir_name` directly (the kernel facade folds them out in
/// `kernel_facade/agents.rs`), so the dir_name is recovered as the
/// segment after the last `::` separator in the id.
///
/// Derivation rule (brief §10.1, hit-first order):
/// 1. Built-in skills: the dir_name is looked up in the built-in skill
///    catalog (office/meta/computer-use/gstack) - this mirrors
///    `builtin_skill_group_key` in
///    `assembly/core/.../skills/catalog.rs` without depending on core.
/// 2. User-level skills: the dir_name is prefix-matched against the
///    gamedev skill taxonomy (引擎/玩法/设计/工程) per brief §10.1.
/// 3. Otherwise -> "其他" (other).
///
/// Partition order is enforced by the UI (`SkillsModule`); this fn only
/// returns the category key. SKILL.md frontmatter `category` override
/// is a follow-up (brief §10.1) - not done here.
pub(crate) fn skill_category(id: &str) -> &'static str {
    let dir_name = id.rsplit("::").next().unwrap_or("");

    // 1. Built-in skills: exact dir_name -> group_key.
    if let Some(group) = builtin_skill_group_key(dir_name) {
        return group;
    }

    // 2. User-level prefix inference (hit-first, brief §10.1).
    // 引擎 (engines)
    for prefix in [
        "godot-",
        "unity-",
        "unreal-",
        "bevy-",
        "phaser-",
        "threejs-",
        "roblox-",
        "love2d-",
        "pygame-",
    ] {
        if dir_name.starts_with(prefix) {
            return "引擎";
        }
    }
    // 玩法 (gameplay)
    for prefix in [
        "platformer",
        "roguelike",
        "puzzle",
        "card-game",
        "tower-defense",
        "fps-shooter",
        "rpg",
        "survival-crafting",
        "visual-novel",
    ] {
        if dir_name == prefix || dir_name.starts_with(&format!("{prefix}-")) {
            return "玩法";
        }
    }
    // 设计 (design)
    for prefix in [
        "game-feel",
        "camera-systems",
        "audio-design",
        "shader-programming",
        "level-design",
        "game-ui-ux",
        "physics-tuning",
        "performance-optimization",
        "procedural-gen",
    ] {
        if dir_name == prefix || dir_name.starts_with(&format!("{prefix}-")) {
            return "设计";
        }
    }
    // 工程 (engineering)
    for prefix in ["input-systems", "save-systems", "dialogue-systems"] {
        if dir_name == prefix || dir_name.starts_with(&format!("{prefix}-")) {
            return "工程";
        }
    }

    // 3. Otherwise.
    "其他"
}

/// Mirrors `builtin_skill_group_key` from the core skill catalog
/// (`assembly/core/.../skills/catalog.rs`). Kept here as a static table
/// so the desktop crate can derive built-in skill groups without a
/// core dependency. If the core catalog adds a new built-in skill, this
/// table must be updated in lockstep (brief §10.1: "用 registry
/// SkillInfo.group_key" - the DTO drops group_key, so we reproduce the
/// mapping here).
fn builtin_skill_group_key(dir_name: &str) -> Option<&'static str> {
    // office: docx/pdf/ppt-design/pptx/xlsx
    // meta: find-skills/writing-skills/memory
    // computer-use: agent-browser
    // gstack: gstack-* family
    match dir_name {
        "docx" | "pdf" | "ppt-design" | "pptx" | "xlsx" => Some("office"),
        "find-skills" | "writing-skills" | "memory" => Some("meta"),
        "agent-browser" => Some("computer-use"),
        _ if dir_name.starts_with("gstack-") => Some("gstack"),
        _ => None,
    }
}

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

/// Event emitter that relays core `skills-changed` events to the Slint UI thread.
pub(super) struct DesktopSkillEventEmitter {
    pub(super) ui: slint::Weak<AppWindow>,
}

#[async_trait::async_trait]
impl northhing_events::EventEmitter for DesktopSkillEventEmitter {
    async fn emit(&self, event_name: &str, _payload: serde_json::Value) -> anyhow::Result<()> {
        if event_name == northhing_core::service::skill_watch::SKILLS_CHANGED_EVENT_NAME {
            let ui_weak = self.ui.clone();
            slint::invoke_from_event_loop(move || {
                let ui_weak2 = ui_weak.clone();
                tokio::spawn(async move {
                    crate::app_state::callbacks_settings::refresh_settings_lists(ui_weak2.clone()).await;
                    refresh_skills_ui(ui_weak2).await;
                });
            })
            .ok();
        }
        Ok(())
    }
}

/// Spawns a background task to register the skill watch listener as soon as
/// `SkillWatchService` becomes available, eliminating the startup race with `init_core`.
pub(super) fn register_desktop_skill_watch_listener(ui: slint::Weak<AppWindow>) -> tokio::task::JoinHandle<bool> {
    tokio::spawn(async move {
        for _ in 0..100 {
            if let Some(skill_watch) = northhing_core::service::skill_watch::global_skill_watch_service() {
                let emitter = Arc::new(DesktopSkillEventEmitter { ui });
                if let Err(e) = skill_watch.set_event_emitter(emitter).await {
                    tracing::warn!(target: "app_state", "Failed to set DesktopSkillEventEmitter on SkillWatchService: {e}");
                    return false;
                }
                tracing::info!(target: "app_state", "Registered desktop skill watch listener for live reload");
                return true;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        tracing::warn!(target: "app_state", "Timed out waiting for SkillWatchService during desktop startup");
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_desktop_skill_event_emitter_handles_skills_changed() {
        use northhing_events::EventEmitter;
        let emitter = DesktopSkillEventEmitter {
            ui: slint::Weak::default(),
        };
        let result = emitter
            .emit(
                northhing_core::service::skill_watch::SKILLS_CHANGED_EVENT_NAME,
                serde_json::json!({}),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_register_desktop_skill_watch_listener_mounts_listener() {
        if let Ok(ws) = northhing_core::service::workspace::WorkspaceService::new().await {
            let ws_service = Arc::new(ws);
            let skill_watch = Arc::new(northhing_core::service::skill_watch::SkillWatchService::new(ws_service));
            northhing_core::service::skill_watch::set_global_skill_watch_service(skill_watch);
        }

        let handle = register_desktop_skill_watch_listener(slint::Weak::default());
        let completed = tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await;
        if let Ok(Ok(success)) = completed {
            assert!(success);
        } else {
            panic!("listener registration failed or timed out");
        }
    }

    /// T4 §10.1: built-in skills derive their group from the catalog
    /// (office/meta/computer-use/gstack). The dir_name is the segment
    /// after the last `::` in the id (the facade folds out `dir_name`
    /// and `group_key`, so the desktop re-derives from the id).
    #[test]
    fn skill_category_builtins_map_to_catalog_groups() {
        // office family
        assert_eq!(skill_category("builtin::northhing-system::docx"), "office");
        assert_eq!(skill_category("builtin::northhing-system::pdf"), "office");
        assert_eq!(skill_category("builtin::northhing-system::ppt-design"), "office");
        assert_eq!(skill_category("builtin::northhing-system::pptx"), "office");
        assert_eq!(skill_category("builtin::northhing-system::xlsx"), "office");
        // meta family
        assert_eq!(skill_category("builtin::northhing-system::find-skills"), "meta");
        assert_eq!(skill_category("builtin::northhing-system::writing-skills"), "meta");
        assert_eq!(skill_category("builtin::northhing-system::memory"), "meta");
        // computer-use
        assert_eq!(skill_category("builtin::northhing-system::agent-browser"), "computer-use");
        // gstack family
        assert_eq!(skill_category("builtin::northhing-system::gstack-review"), "gstack");
        assert_eq!(skill_category("builtin::northhing-system::gstack-autoplan"), "gstack");
    }

    /// T4 §10.1: user-level skills are classified by dir-name prefix
    /// against the gamedev taxonomy. Hit-first; the first matching
    /// prefix wins.
    #[test]
    fn skill_category_user_engine_prefixes() {
        assert_eq!(skill_category("user::home.agents::godot-2d-movement"), "引擎");
        assert_eq!(skill_category("user::home.agents::unity-csharp-scripting"), "引擎");
        assert_eq!(skill_category("user::home.agents::bevy-ecs"), "引擎");
        assert_eq!(skill_category("user::home.agents::roblox-luau"), "引擎");
    }

    #[test]
    fn skill_category_user_gameplay_prefixes() {
        assert_eq!(skill_category("user::home.agents::platformer"), "玩法");
        assert_eq!(skill_category("user::home.agents::roguelike-dungeon"), "玩法");
        assert_eq!(skill_category("user::home.agents::tower-defense"), "玩法");
        assert_eq!(skill_category("user::home.agents::visual-novel"), "玩法");
    }

    #[test]
    fn skill_category_user_design_prefixes() {
        assert_eq!(skill_category("user::home.agents::game-feel"), "设计");
        assert_eq!(skill_category("user::home.agents::camera-systems"), "设计");
        assert_eq!(skill_category("user::home.agents::procedural-gen-noise"), "设计");
    }

    #[test]
    fn skill_category_user_engineering_prefixes() {
        assert_eq!(skill_category("user::home.agents::input-systems"), "工程");
        assert_eq!(skill_category("user::home.agents::save-systems-cloud"), "工程");
        assert_eq!(skill_category("user::home.agents::dialogue-systems"), "工程");
    }

    /// T4 §10.1: skills that match no prefix fall back to "其他".
    #[test]
    fn skill_category_unknown_falls_back_to_other() {
        assert_eq!(skill_category("user::northhing::smoke-placeholder"), "其他");
        assert_eq!(skill_category("user::home.agents::custom-helper"), "其他");
        assert_eq!(skill_category("::"), "其他");
    }
}
