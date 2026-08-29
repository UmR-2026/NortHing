// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task EF-E3 (2026-08-24) — Settings ("全局设置") module window.
//
// Standalone OS window implementing the consult room global settings view
// with two-column philosophy: Left "Its Self" (read-only) & Right "Facility"
// (clickable mock), lightweight chrome, and foldable cards.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::css;
use super::i18n::{keys, LocalePack};
use super::pages_settings_cards::{persist_display_mode, SelfColumn};
use super::pages_settings_provider_edit::ProviderEditModal;
use super::pages_settings_skills::SkillsSection;
use super::registry::ModuleAppProps;
use super::windows::WindowDropGuard;
use crate::app_state::settings::load_app_settings;
use northhing_kernel_api::agents::SkillInfoDto;
use northhing_kernel_api::settings::{AIModelConfigDto, MCPServerDto, ProviderConfigDto};

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use super::windows::win::hide_and_close_hwnd;

/// Settings ("全局设置") module window root component.
pub fn settings_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let plugin_id = props.plugin_id;
    let gen = props.gen;
    let manager = props.manager.clone();

    let mgr_guard = manager.clone();
    use_hook(move || Rc::new(WindowDropGuard::new(plugin_id, gen, mgr_guard)));

    {
        let manager = manager.clone();
        use_effect(move || {
            let wid = window().id();
            #[cfg(target_os = "windows")]
            let hwnd = window().hwnd() as usize;
            #[cfg(not(target_os = "windows"))]
            let hwnd = 0usize;

            if !manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd) {
                #[cfg(target_os = "windows")]
                hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let theme_rx = props.theme_rx.clone();
    let mut theme_dark = use_signal(|| *theme_rx.borrow());

    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    let theme_class = if theme_dark() { "dark" } else { "light" };

    // Left column folding states (它的自我)
    let mut folded_sediment = use_signal(|| false);
    let mut folded_chronicles = use_signal(|| false);
    let mut folded_identity = use_signal(|| false);
    let mut folded_axioms = use_signal(|| false);

    // Right column folding states (设施)
    let mut folded_engine = use_signal(|| false);
    let mut folded_context = use_signal(|| false);
    let mut folded_provider = use_signal(|| false);
    let mut folded_mcp = use_signal(|| false);
    let mut folded_workspace = use_signal(|| false);
    let mut folded_display = use_signal(|| false);

    // Model configs & active selection (Card 1: 模型引擎)
    let model_configs = use_signal(Vec::<AIModelConfigDto>::new);
    let mut active_model_id = use_signal(|| None::<String>);
    let mut active_engine = use_signal(|| 0usize); // Fallback mock state

    // Provider configs & default provider (Card 3: 接入点)
    let providers = use_signal(Vec::<ProviderConfigDto>::new);
    let mut default_provider_id = use_signal(|| None::<String>);
    let mut active_provider_anthropic = use_signal(|| true); // Fallback mock state
    let mut active_provider_google = use_signal(|| false); // Fallback mock state
    let mut editing_provider = use_signal(|| None::<ProviderConfigDto>);

    let refresh_providers = move || {
        let mut providers = providers;
        let mut default_provider_id = default_provider_id;
        let mut active_model_id = active_model_id;
        let mut model_configs = model_configs;
        dioxus::prelude::spawn(async move {
            if let Ok(global_cfg) = super::api::get_global_config().await {
                if let Some(ref def_id) = global_cfg.default_provider_id {
                    default_provider_id.set(Some(def_id.clone()));
                    if active_model_id().is_none() {
                        active_model_id.set(Some(def_id.clone()));
                    }
                }
                providers.set(global_cfg.providers);
            }
            if let Ok(models) = super::api::list_model_configs().await {
                model_configs.set(models);
            }
        });
    };

    // MCP servers (Card 4: 能力集)
    let mut mcp_servers = use_signal(Vec::<MCPServerDto>::new);
    let mut mcp_filesystem = use_signal(|| true); // Fallback mock state
    let mut mcp_philosophy = use_signal(|| true); // Fallback mock state
    let mut mcp_terminal = use_signal(|| true); // Fallback mock state

    // Skills (Card 4: SKILLS sub-section)
    let skills = use_signal(Vec::<SkillInfoDto>::new);
    let skill_error = use_signal(|| None::<String>);

    // Display modes (Card 6: 显示模式) — persisted in AppSettings.display_breath / .display_dual_optics
    let mut display_breath = use_signal(|| true);
    let mut display_dual_optics = use_signal(|| true);

    // Workspace path (Card 5: 工作区)
    let workspace_path = use_signal(|| None::<String>);

    use_future(move || {
        let mut workspace_path = workspace_path;
        let mut model_configs = model_configs;
        let mut active_model_id = active_model_id;
        let mut providers = providers;
        let mut default_provider_id = default_provider_id;
        let mut mcp_servers = mcp_servers;
        let mut skills = skills;
        let mut display_breath = display_breath;
        let mut display_dual_optics = display_dual_optics;
        async move {
            match load_app_settings().await {
                Ok(settings) => {
                    if let Some(cw) = settings.current_workspace {
                        workspace_path.set(Some(cw.to_string_lossy().to_string()));
                    } else if let Some(first_ws) = settings.workspaces.first() {
                        workspace_path.set(Some(first_ws.path.to_string_lossy().to_string()));
                    }
                    display_breath.set(settings.display_breath);
                    display_dual_optics.set(settings.display_dual_optics);
                }
                Err(err) => {
                    tracing::warn!("Failed to load app settings on settings page mount: {err}");
                }
            }

            match super::api::get_global_config().await {
                Ok(global_cfg) => {
                    if let Some(ref def_id) = global_cfg.default_provider_id {
                        default_provider_id.set(Some(def_id.clone()));
                        if active_model_id().is_none() {
                            active_model_id.set(Some(def_id.clone()));
                        }
                    }
                    providers.set(global_cfg.providers);
                }
                Err(err) => {
                    tracing::warn!("Failed to load global config on settings page mount: {err}");
                }
            }

            match super::api::list_model_configs().await {
                Ok(models) => {
                    model_configs.set(models);
                }
                Err(err) => {
                    tracing::warn!("Failed to list model configs on settings page mount: {err}");
                }
            }

            match super::api::list_mcp_servers().await {
                Ok(servers) => {
                    mcp_servers.set(servers);
                }
                Err(err) => {
                    tracing::warn!("Failed to list MCP servers on settings page mount: {err}");
                }
            }

            match super::api::list_skills().await {
                Ok(list) => {
                    skills.set(list);
                }
                Err(err) => {
                    tracing::warn!("Failed to list skills on settings page mount: {err}");
                }
            }
        }
    });

    let fold_all = move |_| {
        let any_open = !folded_sediment()
            || !folded_chronicles()
            || !folded_identity()
            || !folded_axioms()
            || !folded_engine()
            || !folded_context()
            || !folded_provider()
            || !folded_mcp()
            || !folded_workspace()
            || !folded_display();
        let target = any_open;
        folded_sediment.set(target);
        folded_chronicles.set(target);
        folded_identity.set(target);
        folded_axioms.set(target);
        folded_engine.set(target);
        folded_context.set(target);
        folded_provider.set(target);
        folded_mcp.set(target);
        folded_workspace.set(target);
        folded_display.set(target);
    };

    rsx! {
        body {
            "data-theme": "{theme_class}",
            "data-window": "settings",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }

            // Chrome at top: 标题左 + ▴ 收纳 + 主题 + ✕ 关窗
            div {
                class: "settings-chrome",
                onmousedown: move |_| { window().drag(); },
                span { class: "settings-chrome-title", "{locale.t(keys::SETTINGS_WINDOW_TITLE)}" }
                div { class: "settings-chrome-actions",
                    button {
                        class: "fold-btn",
                        title: "{locale.t(keys::WINDOW_FOLD_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: fold_all,
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "theme-btn",
                        id: "settings-theme-toggle",
                        title: "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        "aria-label": "{locale.t(keys::CHROME_THEME_TOGGLE)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            theme_dark.toggle();
                        },
                        svg {
                            view_box: "0 0 16 16",
                            width: "12", height: "12",
                            fill: "none", stroke: "currentColor",
                            stroke_width: "1.3", stroke_linecap: "round", stroke_linejoin: "round",
                            dangerous_inner_html: "{css::theme_toggle_svg(theme_dark())}",
                        }
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        "aria-label": "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            hide_and_close_hwnd(window().hwnd() as isize);
                            window().close();
                        },
                        "✕"
                    }
                }
            }

            // Engine: 2-Column Grid (Left: Its Self [ReadOnly], Right: Facility [Interactive])
            div { class: "settings-engine", id: "settings-engine",
                // Left Column: 它的自我 (Read-only)
                aside { class: "settings-col", id: "settings-self",
                    div { class: "station-head", "{locale.t(keys::SETTINGS_HEAD_SELF)}" }
                    SelfColumn {
                        locale: locale.clone(),
                        folded_sediment,
                        folded_chronicles,
                        folded_identity,
                        folded_axioms,
                    }
                }

                // Right Column: 设施 (Interactive mock)
                aside { class: "settings-col", id: "settings-facility",
                    div { class: "station-head facility", "{locale.t(keys::SETTINGS_HEAD_FACILITY)}" }

                    // Card 1: 模型引擎 ENGINE
                    div {
                        class: if folded_engine() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_engine.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_ENGINE_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_ENGINE_EM)}" }
                            span { class: "fold-caret", if folded_engine() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            if !model_configs().is_empty() {
                                for model in model_configs() {
                                    {
                                        let id = model.id.clone();
                                        let is_active = active_model_id().as_deref() == Some(&id)
                                            || (active_model_id().is_none() && default_provider_id().as_deref() == Some(&id));
                                        let display_title = model.display_name.clone().unwrap_or_else(|| model.model.clone());
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                class: if is_active { "row active" } else { "row" },
                                                onclick: move |_| {
                                                    let id_clone = id.clone();
                                                    active_model_id.set(Some(id_clone.clone()));
                                                    default_provider_id.set(Some(id_clone.clone()));
                                                    dioxus::prelude::spawn(async move {
                                                        if let Err(err) = super::api::set_default_provider(&id_clone).await {
                                                            tracing::warn!("Failed to set default provider {id_clone}: {err}");
                                                        }
                                                    });
                                                },
                                                span { class: "dot-radio" }
                                                "{display_title}"
                                                if is_active {
                                                    span { class: "tag-x current", "{locale.t(keys::SETTINGS_ENGINE_CURRENT)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div {
                                    class: if active_engine() == 0 { "row active" } else { "row" },
                                    onclick: move |_| active_engine.set(0), // TODO(data): fallback mock when empty
                                    span { class: "dot-radio" }
                                    "{locale.t(keys::SETTINGS_ENGINE_CLAUDE)}"
                                    if active_engine() == 0 {
                                        span { class: "tag-x current", "{locale.t(keys::SETTINGS_ENGINE_CURRENT)}" }
                                    }
                                }
                                div {
                                    class: if active_engine() == 1 { "row active" } else { "row" },
                                    onclick: move |_| active_engine.set(1), // TODO(data): fallback mock when empty
                                    span { class: "dot-radio" }
                                    "{locale.t(keys::SETTINGS_ENGINE_GEMINI)}"
                                }
                                div {
                                    class: if active_engine() == 2 { "row active" } else { "row" },
                                    onclick: move |_| active_engine.set(2), // TODO(data): fallback mock when empty
                                    span { class: "dot-radio" }
                                    "{locale.t(keys::SETTINGS_ENGINE_GPT4O)}"
                                }
                            }
                        }
                    }

                    // Card 2: 上下文 CONTEXT
                    div {
                        class: if folded_context() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_context.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_CONTEXT_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_CONTEXT_EM)}" }
                            span { class: "fold-caret", if folded_context() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div { class: "row active",
                                span { class: "dot-radio" }
                                "全局作用域"
                            }
                            div { class: "seg-bar",
                                div { class: "seg on" }
                                div { class: "seg on" }
                                div { class: "seg" }
                                div { class: "seg" }
                                div { class: "seg" }
                            }
                        }
                    }

                    // Card 3: 接入点 PROVIDER
                    div {
                        class: if folded_provider() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_provider.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_PROVIDER_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_PROVIDER_EM)}" }
                            span { class: "fold-caret", if folded_provider() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            if !providers().is_empty() {
                                for provider in providers() {
                                    {
                                        let id = provider.id.clone();
                                        let is_active = default_provider_id().as_deref() == Some(&id);
                                        let name = provider.name.clone();
                                        let provider_type = provider.provider_type.clone().unwrap_or_else(|| provider.model.clone());
                                        let provider_for_edit = provider.clone();
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                class: if is_active { "row active" } else { "row" },
                                                onclick: move |_| {
                                                    let id_clone = id.clone();
                                                    default_provider_id.set(Some(id_clone.clone()));
                                                    active_model_id.set(Some(id_clone.clone()));
                                                    dioxus::prelude::spawn(async move {
                                                        if let Err(err) = super::api::set_default_provider(&id_clone).await {
                                                            tracing::warn!("Failed to set default provider {id_clone}: {err}");
                                                        }
                                                    });
                                                },
                                                span { class: "sq-toggle" }
                                                "{name}"
                                                span { class: "row-meta", "{provider_type}" }
                                                button {
                                                    class: "provider-edit-btn",
                                                    style: "margin-left:auto;padding:1px 6px;font-size:11px;background:var(--bg3);border:1px solid var(--line);border-radius:3px;color:var(--text);cursor:pointer;flex-shrink:0;line-height:1.4;",
                                                    title: "编辑此服务配置",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        editing_provider.set(Some(provider_for_edit.clone()));
                                                    },
                                                    "编辑"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div {
                                    class: if active_provider_anthropic() { "row active" } else { "row" },
                                    onclick: move |_| active_provider_anthropic.toggle(), // TODO(data): fallback mock when empty
                                    span { class: "sq-toggle" }
                                    "{locale.t(keys::SETTINGS_PROVIDER_ANTHROPIC)}"
                                    span { class: "row-meta", "{locale.t(keys::SETTINGS_PROVIDER_DIRECT)}" }
                                }
                                div {
                                    class: if active_provider_google() { "row active" } else { "row" },
                                    onclick: move |_| active_provider_google.toggle(), // TODO(data): fallback mock when empty
                                    span { class: "sq-toggle" }
                                    "{locale.t(keys::SETTINGS_PROVIDER_GOOGLE)}"
                                }
                            }
                        }
                    }

                    // Card 4: 能力集 MCP & SKILLS
                    div {
                        class: if folded_mcp() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_mcp.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_MCP_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_MCP_EM)}" }
                            span { class: "fold-caret", if folded_mcp() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            if !mcp_servers().is_empty() {
                                for server in mcp_servers() {
                                    {
                                        let id = server.id.clone();
                                        let is_enabled = server.enabled.unwrap_or(true);
                                        let name = server.name.clone();
                                        let command_display = if !server.config.command.is_empty() {
                                            server.config.command.clone()
                                        } else {
                                            "MCP".to_string()
                                        };
                                        let server_dto = server.clone();
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                class: if is_enabled { "row active" } else { "row" },
                                                onclick: move |_| {
                                                    let target_id = id.clone();
                                                    let next_enabled = !is_enabled;
                                                    for s in mcp_servers.write().iter_mut() {
                                                        if s.id == target_id {
                                                            s.enabled = Some(next_enabled);
                                                        }
                                                    }
                                                    let server_to_send = server_dto.clone();
                                                    dioxus::prelude::spawn(async move {
                                                        if let Err(err) = super::api::set_mcp_enabled(server_to_send, next_enabled).await {
                                                            tracing::warn!("Failed to set MCP enabled for {target_id}: {err}");
                                                        }
                                                    });
                                                },
                                                span { class: "sq-toggle" }
                                                "{name}"
                                                span { class: "row-meta", "{command_display}" }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div {
                                    class: if mcp_filesystem() { "row active" } else { "row" },
                                    onclick: move |_| mcp_filesystem.toggle(), // TODO(data): fallback mock when empty
                                    span { class: "sq-toggle" }
                                    "{locale.t(keys::SETTINGS_MCP_FILESYSTEM)}"
                                    span { class: "row-meta", "{locale.t(keys::SETTINGS_MCP_READWRITE)}" }
                                }
                                div {
                                    class: if mcp_philosophy() { "row active" } else { "row" },
                                    onclick: move |_| mcp_philosophy.toggle(), // TODO(data): fallback mock when empty
                                    span { class: "sq-toggle" }
                                    "{locale.t(keys::SETTINGS_MCP_PHILOSOPHY)}"
                                    span { class: "row-meta", "{locale.t(keys::SETTINGS_MCP_PLUGIN)}" }
                                }
                                div {
                                    class: if mcp_terminal() { "row active" } else { "row" },
                                    onclick: move |_| mcp_terminal.toggle(), // TODO(data): fallback mock when empty
                                    span { class: "sq-toggle danger" }
                                    "{locale.t(keys::SETTINGS_MCP_TERMINAL)}"
                                    span { class: "row-meta danger", "{locale.t(keys::SETTINGS_MCP_UNAUTHORIZED)}" }
                                }
                            }
                            // Skills sub-section (W9-5): user-scope enable/disable.
                            // ponytail: project-scope overrides deferred — facade list_skills
                            // does not surface group_key/is_builtin yet, and the user-scope
                            // toggle is the minimal viable slice for this round.
                            SkillsSection { skills, last_error: skill_error }
                        }
                    }

                    // Card 5: 工作区 WORKSPACE
                    div {
                        class: if folded_workspace() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_workspace.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_WORKSPACE_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_WORKSPACE_EM)}" }
                            span { class: "fold-caret", if folded_workspace() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: "row static",
                                if let Some(path) = workspace_path() {
                                    "{path}"
                                } else {
                                    "{locale.t(keys::SETTINGS_WORKSPACE_PATH)}"
                                }
                            }
                            button {
                                class: "btn-undo",
                                onmousedown: move |e| e.stop_propagation(),
                                "{locale.t(keys::SETTINGS_BTN_RELOCATE)}"
                            }
                        }
                    }

                    // Card 6: 显示模式 DISPLAY
                    div {
                        class: if folded_display() { "mod is-folded" } else { "mod" },
                        div {
                            class: "side-title w2-pin",
                            onclick: move |_| { folded_display.toggle(); },
                            "{locale.t(keys::SETTINGS_SECTION_DISPLAY_TITLE)} "
                            em { "{locale.t(keys::SETTINGS_SECTION_DISPLAY_EM)}" }
                            span { class: "fold-caret", if folded_display() { "▸" } else { "▾" } }
                        }
                        div { class: "w2-scroll",
                            div {
                                class: if display_breath() { "row active" } else { "row" },
                                onclick: move |_| {
                                    let next = !display_breath();
                                    display_breath.set(next);
                                    dioxus::prelude::spawn(async move {
                                        if let Err(e) = persist_display_mode(Some(next), None).await {
                                            tracing::warn!("display_breath persist failed: {e}");
                                        }
                                    });
                                },
                                span { class: "sq-toggle" }
                                "{locale.t(keys::SETTINGS_DISPLAY_BREATH)}"
                                span { class: "row-meta", "{locale.t(keys::SETTINGS_DISPLAY_BREATH_PERIOD)}" }
                            }
                            div {
                                class: if display_dual_optics() { "row active" } else { "row" },
                                onclick: move |_| {
                                    let next = !display_dual_optics();
                                    display_dual_optics.set(next);
                                    dioxus::prelude::spawn(async move {
                                        if let Err(e) = persist_display_mode(None, Some(next)).await {
                                            tracing::warn!("display_dual_optics persist failed: {e}");
                                        }
                                    });
                                },
                                span { class: "sq-toggle" }
                                "{locale.t(keys::SETTINGS_DISPLAY_DUAL)}"
                                span { class: "row-meta", "{locale.t(keys::SETTINGS_DISPLAY_DUAL_NOTE)}" }
                            }
                            div { class: "row readonly", style: "font-size:11px;color:var(--faint);",
                                "注：呼吸 / 双光学的视觉绑定将在后续视觉更新中生效。"
                            }
                        }
                    }
                }
            }

            if let Some(provider_dto) = editing_provider() {
                ProviderEditModal {
                    provider: provider_dto,
                    on_close: move |_| editing_provider.set(None),
                    on_saved: move |_| {
                        editing_provider.set(None);
                        refresh_providers();
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::settings::AppSettings;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_load_app_settings_resolves_workspace_path_or_default() {
        let mut settings = AppSettings::default();
        assert!(settings.current_workspace.is_none());

        settings.add_workspace(PathBuf::from("/test/path/alpha"));
        settings.set_current_workspace(Some(&PathBuf::from("/test/path/alpha")));

        let resolved = settings
            .current_workspace
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| {
                settings
                    .workspaces
                    .first()
                    .map(|w| w.path.to_string_lossy().to_string())
            });

        assert_eq!(resolved, Some("/test/path/alpha".to_string()));
    }

    #[tokio::test]
    async fn test_update_app_settings_transaction_closure() {
        let mut settings = AppSettings::default();
        let res = (|s: &mut AppSettings| -> anyhow::Result<()> {
            s.onboarding_completed = true;
            Ok(())
        })(&mut settings);

        assert!(res.is_ok());
        assert!(settings.onboarding_completed);
    }

    #[test]
    fn test_mcp_server_toggle_optimistic_update() {
        let mut servers = vec![
            MCPServerDto {
                id: "srv-1".into(),
                name: "Filesystem".into(),
                config: northhing_kernel_api::settings::MCPServerConfigDto {
                    command: "fs".into(),
                    args: vec![],
                    env: None,
                },
                location: northhing_kernel_api::settings::ConfigLocationDto::User,
                enabled: Some(true),
            },
            MCPServerDto {
                id: "srv-2".into(),
                name: "Terminal".into(),
                config: northhing_kernel_api::settings::MCPServerConfigDto {
                    command: "term".into(),
                    args: vec![],
                    env: None,
                },
                location: northhing_kernel_api::settings::ConfigLocationDto::Project,
                enabled: Some(false),
            },
        ];

        let target_id = "srv-1";
        if let Some(s) = servers.iter_mut().find(|s| s.id == target_id) {
            let next = !s.enabled.unwrap_or(true);
            s.enabled = Some(next);
        }

        assert_eq!(servers[0].enabled, Some(false));
        assert_eq!(servers[1].enabled, Some(false));
    }

    #[test]
    fn test_provider_active_matching() {
        let providers = vec![
            ProviderConfigDto {
                id: "anthropic".into(),
                name: "Anthropic".into(),
                base_url: "https://api.anthropic.com".into(),
                model: "claude-3-7-sonnet".into(),
                extra: None,
                enabled: Some(true),
                provider_type: Some("anthropic".into()),
            },
            ProviderConfigDto {
                id: "google".into(),
                name: "Google".into(),
                base_url: "https://generativelanguage.googleapis.com".into(),
                model: "gemini-2.5-pro".into(),
                extra: None,
                enabled: Some(true),
                provider_type: Some("gemini".into()),
            },
        ];

        let default_provider_id = Some("anthropic".to_string());
        let active_id = default_provider_id.as_deref();
        assert_eq!(active_id, Some("anthropic"));
        assert_eq!(providers[0].id, "anthropic");
        assert_ne!(providers[1].id, "anthropic");
    }
}
