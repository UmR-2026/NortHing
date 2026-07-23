use super::load_app_settings_quiet;
use crate::app_state::error_banners::set_banner_message;
use crate::app_state::settings::{MCPTransport, ModelRef, ProviderType};
use crate::app_state::slint_glue::{AppWindow, MCPItem, ProviderItem, SkillStateItem, WorkspaceItem};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::sync::Arc;
use tokio::time::Duration;

// 2026-07-18 (D2b): refresh all 7 settings-list properties from AppSettings.
// Called once at startup (create_ui) and after every settings mutation
// so the SettingsView sub-panels always reflect the on-disk state.
///
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

    // ProviderItem: map ProviderConfig → UI struct. The `type` string is
    // the inverse of the type→ProviderType parsing in register_upsert_provider_callback.
    let providers: Vec<ProviderItem> = s
        .providers
        .iter()
        .map(|p| {
            let type_str = match p.provider_type {
                ProviderType::Anthropic => "anthropic",
                ProviderType::Openai => "openai",
                ProviderType::Gemini => "gemini",
                ProviderType::CustomOpenaiCompatible => "custom-openai",
                ProviderType::CustomAnthropicCompatible => "custom-anthropic",
            };
            let verified = match p.last_verified_ok {
                None => "",
                Some(true) => "ok",
                Some(false) => "fail",
            };
            ProviderItem {
                id: SharedString::from(p.id.clone()),
                name: SharedString::from(p.name.clone()),
                r#type: SharedString::from(type_str),
                base_url: SharedString::from(p.base_url.clone()),
                model: SharedString::from(p.model.clone()),
                enabled: p.enabled,
                verified: SharedString::from(verified),
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

    // MCPItem: verified reflects last_verified_ok (same tri-state as
    // ProviderItem); tool-count comes from the last successful tools/list.
    let mcp_servers: Vec<MCPItem> = s
        .mcp_servers
        .iter()
        .map(|m| {
            let transport_str = match m.transport {
                MCPTransport::Stdio => "stdio",
                MCPTransport::Sse => "sse",
                MCPTransport::StreamableHttp => "streamable-http",
            };
            let verified = match m.last_verified_ok {
                None => "",
                Some(true) => "ok",
                Some(false) => "fail",
            };
            MCPItem {
                id: SharedString::from(m.id.clone()),
                name: SharedString::from(m.name.clone()),
                transport: SharedString::from(transport_str),
                enabled: m.enabled,
                verified: SharedString::from(verified),
                tool_count: m.last_tools.len() as i32,
            }
        })
        .collect();

    // SkillStateItem: workspace-override is looked up via current_workspace.
    let skills: Vec<SkillStateItem> = s
        .skills_enabled
        .iter()
        .map(|sk| {
            let override_val = s
                .current_workspace
                .as_ref()
                .and_then(|cw| sk.workspace_overrides.get(cw))
                .copied();
            let workspace_override_str = match override_val {
                None => "",
                Some(true) => "on",
                Some(false) => "off",
            };
            let effective = override_val.unwrap_or(sk.global_enabled);
            SkillStateItem {
                id: SharedString::from(sk.name.clone()),
                name: SharedString::from(sk.name.clone()),
                description: SharedString::from(""),
                global_enabled: sk.global_enabled,
                workspace_override: SharedString::from(workspace_override_str),
                effective_enabled: effective,
            }
        })
        .collect();

    // current-workspace-index: position of current_workspace in workspaces, -1 if none.
    let current_workspace_index = s
        .current_workspace
        .as_ref()
        .and_then(|cw| s.workspaces.iter().position(|w| &w.path == cw))
        .map(|i| i as i32)
        .unwrap_or(-1);

    // default-model-provider-id: use the configured value directly (not resolve_default_model).
    let default_model_provider_id = s
        .default_model
        .as_ref()
        .map(|m| m.provider_id.clone())
        .unwrap_or_default();

    // legacy-placeholder-count: providers with id containing "-default" and disabled.
    let legacy_placeholder_count = s
        .providers
        .iter()
        .filter(|p| p.id.contains("-default") && !p.enabled)
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

    // Wrap owned copies in Arc so dispatch (Fn) can be called multiple times.
    let providers_owned: Arc<Vec<ProviderItem>> = Arc::new((*providers).clone());
    let skills_owned: Arc<Vec<SkillStateItem>> = Arc::new((*skills).clone());
    let mcp_servers_owned: Arc<Vec<MCPItem>> = Arc::new((*mcp_servers).clone());
    let workspaces_owned: Arc<Vec<WorkspaceItem>> = Arc::new((*workspaces).clone());
    let current_workspace_index_owned: i32 = *current_workspace_index;
    let default_model_provider_id_owned: String = (*default_model_provider_id).clone();
    let legacy_placeholder_count_owned: i32 = *legacy_placeholder_count;

    let dispatch = || {
        let ui_weak = ui_weak.clone();
        let providers_owned = providers_owned.clone();
        let skills_owned = skills_owned.clone();
        let mcp_servers_owned = mcp_servers_owned.clone();
        let workspaces_owned = workspaces_owned.clone();
        let current_workspace_index_owned = current_workspace_index_owned;
        let default_model_provider_id_owned = default_model_provider_id_owned.clone();
        let legacy_placeholder_count_owned = legacy_placeholder_count_owned;

        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_providers_list(ModelRc::new(VecModel::from((*providers_owned).clone())));
                ui.set_skills_list(ModelRc::new(VecModel::from((*skills_owned).clone())));
                ui.set_mcp_servers_list(ModelRc::new(VecModel::from((*mcp_servers_owned).clone())));
                ui.set_workspaces_list(ModelRc::new(VecModel::from((*workspaces_owned).clone())));
                ui.set_current_workspace_index(current_workspace_index_owned);
                ui.set_default_model_provider_id(SharedString::from(default_model_provider_id_owned.clone()));
                ui.set_legacy_placeholder_count(legacy_placeholder_count_owned);
            }
        }
    };

    if let Err(e) = slint::invoke_from_event_loop(dispatch()) {
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
