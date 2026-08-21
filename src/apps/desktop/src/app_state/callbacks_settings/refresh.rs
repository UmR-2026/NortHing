use super::load_app_settings_quiet;
use crate::app_state::error_banners::set_banner_message;
use crate::app_state::slint_glue::{AppWindow, MCPItem, ProviderItem, SkillStateItem, WorkspaceItem};
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::settings::MCPServerDto;
use northhing_kernel_api::KernelAgentsApi;
use northhing_kernel_api::KernelSettingsApi;
use slint::{ModelRc, SharedString, VecModel};
use std::sync::Arc;
use tokio::time::Duration;

// 2026-07-18 (D2b): refresh all 7 settings-list properties from AppSettings.
// Called once at startup (create_ui) and after every settings mutation
// so the SettingsView sub-panels always reflect the on-disk state.
//
// 2026-07-27 (K4a R3, Bug C + Bug D): skills and MCP servers are now
// read from the live kernel facade instead of `AppSettings.skills_enabled`
// and `AppSettings.mcp_servers`. The K4a migration moved skill discovery
// and MCP server registration to the core (see `northhing_core::agentic
// ::tools::implementations::skills::skill_registry` and
// `northhing_core::service::mcp::MCPService`), but the Settings panel's
// `refresh_settings_lists` still consumed the now-empty AppSettings
// fields. With `AppSettings::skills_enabled` and
// `AppSettings::mcp_servers` never populated (no code path writes to
// them post-migration; `upsert_mcp` / `remove_mcp` are dead methods),
// the Settings > Skills and Settings > MCP panels rendered empty even
// when core had real entries (e.g. `%APPDATA%\northhing\skills
// \smoke-placeholder\SKILL.md` and the registered `smoke-echo` MCP
// server). The fix reads from `kernel_facade().list_skills()` +
// `load_skill_overrides()` for skills and `kernel_facade()
// .list_mcp_servers()` for MCP servers. Providers + workspaces still
// come from AppSettings — those ARE the user-edited state and the
// upsert/delete callbacks persist there.
//
/// 2026-07-18 (D2j): signature takes `slint::Weak<AppWindow>` so callers on
/// background threads no longer need to `upgrade()` (which returns None on
/// non-UI threads). The upgrade happens inside the invoke closure (UI thread).
pub(crate) async fn refresh_settings_lists(ui_weak: slint::Weak<AppWindow>) {
    let s = match load_app_settings_quiet().await {
        Ok(s) => s,
        Err(e) => {
            set_banner_message(ui_weak, e, "");
            return;
        }
    };

    let facade = kernel_facade();
    let core_models = facade.list_model_configs().await.unwrap_or_default();
    let global_cfg = facade.get_global_config().await.ok();

    // ProviderItem: map AIModelConfigDto → UI struct.
    let providers: Vec<ProviderItem> = core_models
        .iter()
        .map(|p| {
            let type_str = match p.provider_id.as_str() {
                "anthropic" => "anthropic",
                "openai" => "openai",
                "gemini" => "gemini",
                _ => "custom-openai",
            };
            ProviderItem {
                id: SharedString::from(p.id.clone()),
                name: SharedString::from(p.display_name.clone().unwrap_or_else(|| p.id.clone())),
                r#type: SharedString::from(type_str),
                base_url: SharedString::from(p.base_url.clone().unwrap_or_default()),
                model: SharedString::from(p.model.clone()),
                enabled: p.enabled.unwrap_or(true),
                verified: SharedString::from(""),
            }
        })
        .collect();

    // WorkspaceItem: id and path both use the path string; is-current
    // compares against current_workspace.
    let workspaces: Vec<WorkspaceItem> = s
        .workspaces
        .iter()
        .map(|w| {
            let path_str = w.path.to_string_lossy().to_string();
            WorkspaceItem {
                id: SharedString::from(path_str.clone()),
                path: SharedString::from(path_str),
                display_name: SharedString::from(w.display_name.clone()),
                is_current: s.current_workspace.as_deref() == Some(w.path.as_path()),
                identity_md_exists: w.identity_md_path.is_some(),
            }
        })
        .collect();

    let mcp_servers: Vec<MCPItem> = match facade.list_mcp_servers().await {
        Ok(servers) => build_mcp_items(&servers),
        Err(e) => {
            tracing::warn!(
                target: "app_state",
                "refresh_settings_lists: list_mcp_servers failed: {e}"
            );
            Vec::new()
        }
    };

    // 2026-07-27 (K4a R3, Bug C): skills come from the live core skill
    // registry via the kernel facade, not the (always-empty after K4a)
    // AppSettings.skills_enabled. The `enabled` field on `SkillInfoDto` is
    // a placeholder (the comment at `agents.rs:50` notes it is mode-
    // dependent and requires mode context) — we resolve the real per-skill
    // enable state from `load_skill_overrides()` and the per-mode
    // default via `resolve_skill_default_enabled`, mirroring the logic
    // the Inspector uses (`app_state::skills::build_skills_model`).
    //
    // 2026-07-27 (K4a R3, Bug C, fix #5): the per-workspace override
    // column needs `facade.load_project_skills()`. The current
    // implementation is a stub that returns
    // `KernelError::Internal("not yet wired: load_project_skills
    // — workspace_path not available")` — the trait signature
    // doesn't carry `workspace_path`. The user said "不改
    // facade trait 签名" (don't change the trait signature), so we
    // can't ask the facade for the data; instead we try it,
    // observe the `Err`, and pass `workspace_override_supported =
    // false` to the Slint panel. The panel then HONESTLY hides
    // the workspace cycle button — we never write a fake
    // workspace_override value. When a future K4a follow-up
    // wires the per-workspace data source, the panel will
    // unhide the column automatically.
    let mode_id = crate::flags::DEFAULT_MODE_ID;
    let (skills, workspace_override_supported) = match facade.list_skills().await {
        Ok(skills) => {
            let overrides = facade
                .load_skill_overrides()
                .await
                .ok()
                .map(|o| o.overrides)
                .unwrap_or_default();
            let items = build_skill_state_items(&facade, &skills, &overrides, mode_id).await;
            // Probe per-workspace support: `load_project_skills`
            // currently returns a stub `Err` (the trait
            // signature has no `workspace_path` parameter,
            // so it cannot resolve the per-workspace document
            // without one). When the user is on a project
            // workspace AND the facade returns real data,
            // we'd compute per-row workspace overrides here
            // and set `workspace_override_supported = true`.
            let workspace_override_supported = match s.current_workspace.as_ref() {
                Some(_workspace) => {
                    // 2026-07-27 (K4a R3, fix #5): trait
                    // signature is `load_project_skills(&self)
                    // -> ProjectSkillsDto` — no
                    // `workspace_path` arg. The current
                    // implementation is a stub that returns
                    // `KernelError::Internal("not yet wired:
                    // load_project_skills — workspace_path not
                    // available")`. The pre-fix code silently
                    // wrote `workspace_override = ""` for
                    // every row; the panel then rendered the
                    // cycle button which silently no-op'd the
                    // user's clicks (the callback wrote
                    // `set-skill-workspace` to the AppSettings,
                    // but no one ever reads from there in the
                    // data flow). That's a UX trap: the
                    // button LOOKED clickable but the override
                    // never actually took effect.
                    //
                    // The honest path: try the facade probe;
                    // if it returns the unwired-stub error,
                    // tell the panel "we can't honor a
                    // workspace override right now" via
                    // `workspace_override_supported = false`
                    // and let the panel hide the column.
                    let supported = facade
                        .load_project_skills()
                        .await
                        .ok()
                        .map(|doc| !doc.skills.is_empty() || !s.current_workspace.is_none())
                        .unwrap_or(false);
                    supported
                }
                None => false,
            };
            (items, workspace_override_supported)
        }
        Err(e) => {
            tracing::warn!(
                target: "app_state",
                "refresh_settings_lists: list_skills failed: {e}"
            );
            (Vec::new(), false)
        }
    };

    // current-workspace-index: position of current_workspace in workspaces, -1 if none.
    let current_workspace_index = s
        .current_workspace
        .as_ref()
        .and_then(|cw| s.workspaces.iter().position(|w| &w.path == cw))
        .map(|i| i as i32)
        .unwrap_or(-1);

    // default-model-provider-id: from core global config.
    let default_model_provider_id = global_cfg
        .as_ref()
        .and_then(|c| c.default_provider_id.clone())
        .unwrap_or_default();

    // legacy-placeholder-count: providers with id containing "-default" and disabled.
    let legacy_placeholder_count = core_models
        .iter()
        .filter(|p| p.id.contains("-default") && p.enabled == Some(false))
        .count() as i32;

    // All 7 property sets in a single invoke_from_event_loop.
    // Wrap in Arc so retry (after startup-race sleep) can reuse the same data.
    let providers = Arc::new(providers);
    let skills = Arc::new(skills);
    let mcp_servers = Arc::new(mcp_servers);
    let workspaces = Arc::new(workspaces);
    let current_workspace_index = Arc::new(current_workspace_index);
    let default_model_provider_id = Arc::new(default_model_provider_id);
    let legacy_placeholder_count = Arc::new(legacy_placeholder_count);
    // 2026-07-27 (K4a R3, fix #5): the per-workspace override
    // column in the Settings > Skills panel is only enabled when
    // the facade can actually resolve per-workspace data
    // (`load_project_skills` is currently a stub — see comment
    // above). When false, the panel MUST hide the cycle button
    // (so users aren't lured into a click that silently
    // no-ops).
    let workspace_override_supported = Arc::new(workspace_override_supported);

    // Wrap owned copies in Arc so dispatch (Fn) can be called multiple times.
    let providers_owned: Arc<Vec<ProviderItem>> = Arc::new((*providers).clone());
    let skills_owned: Arc<Vec<SkillStateItem>> = Arc::new((*skills).clone());
    let mcp_servers_owned: Arc<Vec<MCPItem>> = Arc::new((*mcp_servers).clone());
    let workspaces_owned: Arc<Vec<WorkspaceItem>> = Arc::new((*workspaces).clone());
    let current_workspace_index_owned: i32 = *current_workspace_index;
    let default_model_provider_id_owned: String = (*default_model_provider_id).clone();
    let legacy_placeholder_count_owned: i32 = *legacy_placeholder_count;
    let workspace_override_supported_owned: bool = *workspace_override_supported;

    let dispatch = || {
        let ui_weak = ui_weak.clone();
        let providers_owned = providers_owned.clone();
        let skills_owned = skills_owned.clone();
        let mcp_servers_owned = mcp_servers_owned.clone();
        let workspaces_owned = workspaces_owned.clone();
        let current_workspace_index_owned = current_workspace_index_owned;
        let default_model_provider_id_owned = default_model_provider_id_owned.clone();
        let legacy_placeholder_count_owned = legacy_placeholder_count_owned;
        let workspace_override_supported_owned = workspace_override_supported_owned;

        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_providers_list(ModelRc::new(VecModel::from((*providers_owned).clone())));
                ui.set_skills_list(ModelRc::new(VecModel::from((*skills_owned).clone())));
                ui.set_mcp_servers_list(ModelRc::new(VecModel::from((*mcp_servers_owned).clone())));
                ui.set_workspaces_list(ModelRc::new(VecModel::from((*workspaces_owned).clone())));
                ui.set_current_workspace_index(current_workspace_index_owned);
                ui.set_default_model_provider_id(SharedString::from(default_model_provider_id_owned.clone()));
                ui.set_legacy_placeholder_count(legacy_placeholder_count_owned);
                ui.set_workspace_override_supported(workspace_override_supported_owned);
            }
        }
    };

    if let Err(_e) = slint::invoke_from_event_loop(dispatch()) {
        // 2026-07-18 (D2h): startup-race retry: the event loop may not be
        // ready yet when this is called early in app init. Wait 500ms and
        // retry with the same data (Arc-wrapped above).
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime for retry dispatch");
        rt.block_on(async {
            tokio::time::sleep(Duration::from_millis(500)).await;
        });
        if let Err(e2) = slint::invoke_from_event_loop(dispatch()) {
            tracing::warn!(
                target: "app_state",
                "refresh_settings_lists: failed to dispatch to UI thread (startup race retry failed): {e2}"
            );
        }
    }
}

/// 2026-07-27 (K4a R3, Bug D): pure helper that maps the facade's
/// `MCPServerDto` list into the Slint `MCPItem` rows. Extracted so the
/// transport-inference rule (non-empty `command` → "stdio", else
/// "sse") is unit-testable without booting the kernel facade. The
/// `verified` field stays empty — the Inspector's status string
/// already summarizes connection health and per-server probes are
/// deferred to a follow-up.
pub(crate) fn build_mcp_items(servers: &[MCPServerDto]) -> Vec<MCPItem> {
    servers
        .iter()
        .map(|c| {
            let transport_str = if !c.config.command.is_empty() { "stdio" } else { "sse" };
            MCPItem {
                id: SharedString::from(c.id.clone()),
                name: SharedString::from(c.name.clone()),
                transport: SharedString::from(transport_str),
                enabled: c.enabled.unwrap_or(true),
                verified: SharedString::from(""),
                tool_count: 0,
            }
        })
        .collect()
}

/// 2026-07-27 (K4a R3, Bug C + fix #4): pure helper that resolves
/// per-skill enable state from `load_skill_overrides()` and
/// the facade's `resolve_skill_default_enabled`. The override
/// rule mirrors `super::skills::build_skills_model` exactly so
/// the Settings > Skills panel and the Inspector render the
/// same per-skill state for the same `SkillOverrideEntry`
/// list:
///
/// 1. Any boolean-true override for `skill_id` → `true`.
/// 2. Any boolean-false override for `skill_id` → `false`.
/// 3. Otherwise the per-mode default
///    (`resolve_skill_default_enabled_for_mode`).
///
/// The pre-fix helper only honored `key == "user_enabled"`,
/// silently dropping every other boolean entry — so a profile
/// override keyed `"enabled"` or `"user_mode_override"`
/// (which the Inspector does pick up) would show on the
/// Inspector badge but not in the Settings > Skills panel.
/// Extracted so the override-precedence rule is unit-testable
/// without booting the kernel facade. Async because the
/// default lookup itself is async.
pub(crate) async fn build_skill_state_items(
    facade: &northhing_core::kernel_facade::KernelFacade,
    skills: &[northhing_kernel_api::agents::SkillInfoDto],
    overrides: &[northhing_kernel_api::agents::SkillOverrideEntry],
    mode_id: &str,
) -> Vec<SkillStateItem> {
    use northhing_kernel_api::KernelAgentsApi;
    // 2026-07-27 (K4a R3, fix #4): accept ANY boolean override
    // (the Inspector does the same in
    // `super::skills::build_skills_model` — see
    // `apps/desktop/src/app_state/skills.rs:37-46`). Filtering
    // by `key == "user_enabled"` was a K4a-R2 oversight that
    // made the Settings panel's per-skill state disagree with
    // the Inspector's for every override keyed anything other
    // than `"user_enabled"`.
    let mut enabled_set: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut disabled_set: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for entry in overrides.iter() {
        if entry.value == serde_json::Value::Bool(true) {
            enabled_set.insert(entry.skill_id.as_str());
        } else if entry.value == serde_json::Value::Bool(false) {
            disabled_set.insert(entry.skill_id.as_str());
        }
    }

    let mut out = Vec::with_capacity(skills.len());
    for skill in skills.iter() {
        let key = skill.id.as_str();
        let global_enabled = if enabled_set.contains(key) {
            true
        } else if disabled_set.contains(key) {
            false
        } else {
            facade
                .resolve_skill_default_enabled(&skill.id, mode_id)
                .await
                .unwrap_or(false)
        };
        out.push(SkillStateItem {
            id: SharedString::from(skill.id.clone()),
            name: SharedString::from(skill.name.clone()),
            description: SharedString::from(skill.description.clone()),
            global_enabled,
            workspace_override: SharedString::from(""),
            effective_enabled: global_enabled,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::agents::{SkillInfoDto, SkillOverrideEntry, SkillOverridesDto};
    use northhing_kernel_api::settings::{ConfigLocationDto, MCPServerConfigDto, MCPServerDto};
    use serde_json::json;

    fn stdio_server(id: &str, name: &str, command: &str) -> MCPServerDto {
        MCPServerDto {
            id: id.to_string(),
            name: name.to_string(),
            config: MCPServerConfigDto {
                command: command.to_string(),
                args: vec!["/c".to_string(), "ping".to_string()],
                env: None,
            },
            location: ConfigLocationDto::User,
            enabled: Some(true),
        }
    }

    /// 2026-07-27 (K4a R3, Bug D): the registered `smoke-echo` server
    /// (stdio) is rendered as a non-empty MCPItem row. Without the
    /// K4a R3 fix, the Settings > MCP panel showed an empty list
    /// because the data source was the always-empty
    /// `AppSettings.mcp_servers`.
    #[test]
    fn build_mcp_items_renders_stdio_server_from_facade() {
        let servers = vec![stdio_server("smoke-echo", "smoke-echo", "cmd")];
        let items = build_mcp_items(&servers);
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.id.as_str(), "smoke-echo");
        assert_eq!(item.name.as_str(), "smoke-echo");
        assert_eq!(item.transport.as_str(), "stdio");
        assert!(item.enabled);
        // verified intentionally empty until per-server status probes
        // are wired (K4a-T4 deferral). The Inspector's `mcp_status`
        // string carries the connected/failed summary.
        assert_eq!(item.verified.as_str(), "");
    }

    /// 2026-07-27 (K4a R3, Bug D): command-less servers (sse/http
    /// transports) fall back to the "sse" label so the panel still
    /// renders them; the transport field of `MCPServerConfigDto` is
    /// not yet exposed by the facade, so we infer.
    #[test]
    fn build_mcp_items_falls_back_to_sse_when_command_is_empty() {
        let servers = vec![MCPServerDto {
            id: "remote".into(),
            name: "remote".into(),
            config: MCPServerConfigDto {
                command: String::new(),
                args: vec![],
                env: None,
            },
            location: ConfigLocationDto::User,
            enabled: Some(false),
        }];
        let items = build_mcp_items(&servers);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].transport.as_str(), "sse");
        assert!(!items[0].enabled);
    }

    /// 2026-07-27 (K4a R3, Bug D): an empty server list produces no
    /// rows (matches the Settings > MCP panel's "no servers" empty
    /// state) — but the function still returns an empty Vec rather
    /// than None / error, so the UI can render the empty placeholder.
    #[test]
    fn build_mcp_items_empty_input_yields_empty_vec() {
        let items = build_mcp_items(&[]);
        assert!(items.is_empty());
    }

    fn skill(id: &str, name: &str, desc: &str) -> SkillInfoDto {
        SkillInfoDto {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            enabled: false,
            mode: None,
            tags: None,
        }
    }

    fn override_entry(skill_id: &str, value: bool) -> SkillOverrideEntry {
        SkillOverrideEntry {
            skill_id: skill_id.to_string(),
            key: "user_enabled".to_string(),
            value: json!(value),
        }
    }

    /// 2026-07-27 (K4a R3, Bug C): without any overrides, every skill
    /// defaults to `global_enabled=false` (the `resolve_skill_default_
    /// enabled` call returns Err because the global coordinator isn't
    /// initialized in the test — the helper maps that to `false`).
    /// The point of this test is the override-precedence path: a
    /// user who explicitly enabled a skill via `set_user_mode_skill_
    /// state` must see `global_enabled=true` even when the default
    /// would otherwise be false.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn build_skill_state_items_user_enabled_override_wins() {
        let facade = kernel_facade();
        let skills = vec![
            skill("user::home.agents::isaac-ai-game", "isaac-ai-game", "isaac"),
            skill("user::northhing::smoke-placeholder", "smoke-placeholder", "smoke"),
        ];
        let overrides = vec![
            override_entry("user::home.agents::isaac-ai-game", true),
            override_entry("user::northhing::smoke-placeholder", false),
        ];
        let items = build_skill_state_items(&facade, &skills, &overrides, "agentic").await;
        assert_eq!(items.len(), 2);

        let isaac = items
            .iter()
            .find(|s| s.id.as_str() == "user::home.agents::isaac-ai-game")
            .expect("isaac-ai-game row");
        assert!(isaac.global_enabled, "isaac-ai-game user_enabled=true wins");
        assert!(isaac.effective_enabled);

        let smoke = items
            .iter()
            .find(|s| s.id.as_str() == "user::northhing::smoke-placeholder")
            .expect("smoke-placeholder row");
        assert!(!smoke.global_enabled, "smoke-placeholder user_enabled=false wins");
        assert!(!smoke.effective_enabled);
    }

    /// 2026-07-27 (K4a R3, Bug C + fix #4): a SkillOverrideEntry
    /// with a non-`user_enabled` key (the `user_mode` profile
    /// overrides other things) is NOT silently dropped — the
    /// helper picks up any boolean override, matching the
    /// Inspector's `build_skills_model` rule
    /// (`apps/desktop/src/app_state/skills.rs:37-46`). The
    /// pre-fix code only honored `key == "user_enabled"`; this
    /// assertion is the regression guard.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn build_skill_state_items_honors_non_user_enabled_overrides() {
        let facade = kernel_facade();
        let skills = vec![skill("user::home.agents::foo", "foo", "foo")];
        let overrides = vec![SkillOverrideEntry {
            skill_id: "user::home.agents::foo".to_string(),
            key: "user_mode".to_string(),
            value: json!(true),
        }];
        let items = build_skill_state_items(&facade, &skills, &overrides, "agentic").await;
        assert_eq!(items.len(), 1);
        // The non-user_enabled boolean-true override MUST flip
        // the row to enabled — the Inspector's `build_skills_model`
        // does the same. Pre-fix this test would assert `false`
        // and the Settings panel would diverge from the
        // Inspector for any non-`user_enabled` override.
        assert!(
            items[0].global_enabled,
            "non-user_enabled boolean-true override must enable the row (Inspector parity)"
        );
    }

    /// 2026-07-27 (K4a R3, Bug C): the empty-overrides path mirrors
    /// the real-world case where a user has never toggled any skill.
    /// Every skill must end up in the list (the Inspector renders the
    /// same set) and `effective_enabled == global_enabled` for each.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn build_skill_state_items_empty_overrides_keeps_all_rows() {
        let facade = kernel_facade();
        let skills = vec![
            skill("user::home.agents::a", "a", "a"),
            skill("user::home.agents::b", "b", "b"),
        ];
        let items = build_skill_state_items(&facade, &skills, &[], "agentic").await;
        assert_eq!(items.len(), 2);
        for item in &items {
            assert_eq!(item.effective_enabled, item.global_enabled);
            assert_eq!(item.workspace_override.as_str(), "");
        }
    }
}
