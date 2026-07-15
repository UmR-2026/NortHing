use super::super::agent_selector::AgentItem;
use super::super::model_selector::ModelItem;
use super::super::provider_selector::ProviderSelection;
use super::super::session_selector::SessionItem;
use super::super::skill_selector::{SkillItem, SkillSelectorAction};
use super::super::subagent_selector::{SubagentItem, SubagentSelectorAction};
use super::super::theme::{
    builtin_theme_ids, builtin_theme_json, resolve_appearance, resolve_effective_color_scheme, Appearance,
    EffectiveColorScheme, Theme,
};
use super::super::theme_selector::ThemeItem;

use northhing_core::agentic::agents::{
    agent_registry, AgentInfo, SubAgentSource, SubagentListScope, SubagentQueryContext,
};
use northhing_core::agentic::tools::implementations::skills::{
    mode_overrides::{
        load_project_mode_skills_document_local, save_project_mode_skills_document_local,
        set_mode_skill_disabled_in_document, set_user_mode_skill_state,
    },
    registry::SkillRegistry,
    ModeSkillInfo, SkillInfo,
};
use northhing_core::service::config::GlobalConfigManager;

use super::StartupPage;

impl StartupPage {
    // ======================== Selectors ========================

    /// Push the currently visible popup onto the navigation stack and hide it
    pub(super) fn show_session_selector(&mut self) {
        self.push_current_popup_to_stack();
        let coordinator = self.coordinator.clone();
        let sessions = tokio::task::block_in_place(|| {
            let workspace_path = self.workspace_path_buf();
            tokio::runtime::Handle::current()
                .block_on(async { coordinator.list_sessions(&workspace_path).await.unwrap_or_default() })
        });

        if sessions.is_empty() {
            self.status = Some("No sessions found.".to_string());
            return;
        }

        let session_items: Vec<SessionItem> = sessions
            .into_iter()
            .map(|s| {
                let last_activity = {
                    let elapsed = s.last_activity_at.elapsed().unwrap_or_default();
                    if elapsed.as_secs() < 60 {
                        "just now".to_string()
                    } else if elapsed.as_secs() < 3600 {
                        format!("{}m ago", elapsed.as_secs() / 60)
                    } else if elapsed.as_secs() < 86400 {
                        format!("{}h ago", elapsed.as_secs() / 3600)
                    } else {
                        format!("{}d ago", elapsed.as_secs() / 86400)
                    }
                };
                SessionItem {
                    session_id: s.session_id,
                    session_name: s.session_name,
                    last_activity,
                    workspace: Some(self.workspace_display.clone()),
                }
            })
            .collect();

        self.session_selector.show(session_items, None);
    }

    pub(super) fn handle_session_delete(&mut self, item: &SessionItem) {
        let coordinator = self.coordinator.clone();
        let sid = item.session_id.clone();

        let result = tokio::task::block_in_place(|| {
            let workspace_path = self.workspace_path_buf();
            tokio::runtime::Handle::current()
                .block_on(async { coordinator.delete_session(&workspace_path, &sid).await })
        });

        match result {
            Ok(()) => {
                self.session_selector.remove_item(&item.session_id);
                self.status = Some(format!("Session deleted: {}", item.session_name));
            }
            Err(e) => {
                self.status = Some(format!("Failed to delete session: {}", e));
            }
        }
    }

    pub(super) fn show_model_selector(&mut self) {
        self.push_current_popup_to_stack();

        let agent_type = self.agent_type.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = GlobalConfigManager::service().await.ok()?;
                let models: Vec<northhing_core::service::config::AIModelConfig> =
                    config_service.get_ai_models().await.ok()?;
                let global_config: northhing_core::service::config::GlobalConfig =
                    config_service.config(None).await.ok()?;

                let current_model_id = global_config
                    .ai
                    .agent_models
                    .get(&agent_type)
                    .cloned()
                    .or_else(|| global_config.ai.default_models.primary.clone());

                let model_items: Vec<ModelItem> = models
                    .into_iter()
                    .filter(|m| m.enabled)
                    .map(|m| ModelItem {
                        id: m.id,
                        name: m.name,
                        provider: m.provider,
                        model_name: m.model_name,
                    })
                    .collect();

                Some((model_items, current_model_id))
            })
        });

        match result {
            Some((models, current_id)) if !models.is_empty() => {
                self.model_selector.show(models, current_id);
            }
            _ => {
                self.status = Some("No available models found.".to_string());
            }
        }
    }

    pub(super) fn apply_model_selection(&mut self, selected: &ModelItem) {
        let selected_id = selected.id.clone();
        let selected_display_name = format!("{} / {}", selected.model_name, selected.name);
        let modes = self.get_mode_agents();

        let success = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = match GlobalConfigManager::service().await {
                    Ok(s) => s,
                    Err(_) => return false,
                };

                if let Err(e) = config_service
                    .set_config("ai.default_models.primary", &selected_id)
                    .await
                {
                    tracing::error!("Failed to set default primary model: {}", e);
                    return false;
                }

                for mode in &modes {
                    let path = format!("ai.agent_models.{}", mode.id);
                    if let Err(e) = config_service.set_config(&path, &selected_id).await {
                        tracing::error!("Failed to set model for mode '{}': {}", mode.id, e);
                    }
                }

                true
            })
        });

        if success {
            self.model_display_name = selected_display_name.clone();
            self.status = Some(format!("Model switched to: {}", selected_display_name));
        } else {
            self.status = Some("Failed to switch model".to_string());
        }
    }

    /// Handle provider selection result (step 1 → step 2 of add model)
    pub(super) fn handle_provider_selection(&mut self, selection: ProviderSelection) {
        match selection {
            ProviderSelection::Provider(template) => {
                let default_model = template.models.first().cloned().unwrap_or_default();
                self.model_config_form.show_from_provider(
                    &template.name,
                    &template.base_url,
                    &template.format,
                    &default_model,
                );
            }
            ProviderSelection::Custom => {
                self.model_config_form.show_custom();
            }
        }
    }

    /// Save new model to global config
    pub(super) fn save_new_model(&mut self, result: super::super::model_config_form::ModelFormResult) {
        let model_id = format!(
            "model_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let custom_headers: Option<std::collections::HashMap<String, String>> = if result.custom_headers.is_empty() {
            None
        } else {
            serde_json::from_str(&result.custom_headers).ok()
        };

        let custom_request_body: Option<String> = if result.custom_request_body.is_empty() {
            None
        } else {
            Some(result.custom_request_body.clone())
        };

        let model_config = northhing_core::service::config::AIModelConfig {
            id: model_id.clone(),
            name: result.name.clone(),
            provider: result.provider_format.clone(),
            model_name: result.model_name.clone(),
            base_url: result.base_url.clone(),
            api_key: result.api_key.clone(),
            context_window: Some(result.context_window),
            max_tokens: Some(result.max_tokens),
            enabled: true,
            enable_thinking_process: result.enable_thinking || result.support_preserved_thinking,
            skip_ssl_verify: result.skip_ssl_verify,
            custom_headers,
            custom_headers_mode: if result.custom_headers_mode.is_empty() || result.custom_headers_mode == "merge" {
                None
            } else {
                Some(result.custom_headers_mode.clone())
            },
            custom_request_body,
            ..Default::default()
        };

        let result_name = result.name.clone();
        let result_model_display = format!("{} / {}", result.model_name, result.name);

        let success = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = match GlobalConfigManager::service().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to get config service: {}", e);
                        return false;
                    }
                };

                if let Err(e) = config_service.add_ai_model(model_config).await {
                    tracing::error!("Failed to add AI model: {}", e);
                    return false;
                }

                // Auto-set as primary model if no primary model exists
                match config_service
                    .config::<northhing_core::service::config::GlobalConfig>(None)
                    .await
                {
                    Ok(global_config) => {
                        let has_primary = global_config
                            .ai
                            .default_models
                            .primary
                            .as_ref()
                            .map(|p| !p.is_empty())
                            .unwrap_or(false);
                        if !has_primary {
                            if let Err(e) = config_service.set_config("ai.default_models.primary", &model_id).await {
                                tracing::warn!("Failed to auto-set primary model: {}", e);
                            } else {
                                tracing::info!("Auto-set primary model: {}", model_id);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read config for auto-primary: {}", e);
                    }
                }

                true
            })
        });

        if success {
            self.model_display_name = result_model_display;
            self.status = Some(format!("Model added: {}", result_name));
            tracing::info!("Added new AI model: {}", model_id);
            // Reload model name display
            self.load_current_model_name();
        } else {
            self.status = Some("Failed to add model".to_string());
        }
    }

    /// Fetch full model config and open the edit form
    pub(super) fn edit_model(&mut self, selected: &ModelItem) {
        let model_id = selected.id.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = GlobalConfigManager::service().await.ok()?;
                let models: Vec<northhing_core::service::config::AIModelConfig> =
                    config_service.get_ai_models().await.ok()?;
                models.into_iter().find(|m| m.id == model_id)
            })
        });

        match result {
            Some(model) => {
                let form_data = super::super::model_config_form::ModelFormResult {
                    editing_model_id: Some(model.id.clone()),
                    name: model.name,
                    model_name: model.model_name,
                    base_url: model.base_url,
                    api_key: model.api_key,
                    provider_format: model.provider.clone(),
                    context_window: model.context_window.unwrap_or(128000),
                    max_tokens: model.max_tokens.unwrap_or(8192),
                    enable_thinking: model.enable_thinking_process,
                    support_preserved_thinking: model.inline_think_in_text,
                    skip_ssl_verify: model.skip_ssl_verify,
                    custom_headers: model
                        .custom_headers
                        .map(|h| serde_json::to_string(&h).unwrap_or_default())
                        .unwrap_or_default(),
                    custom_headers_mode: model.custom_headers_mode.unwrap_or_else(|| "merge".to_string()),
                    custom_request_body: model.custom_request_body.unwrap_or_default(),
                };
                self.model_config_form.show_for_edit(&model.id, &form_data);
            }
            None => {
                self.status = Some("Failed to load model configuration".to_string());
            }
        }
    }

    /// Update an existing model in global config
    pub(super) fn update_existing_model(&mut self, result: super::super::model_config_form::ModelFormResult) {
        let model_id = match &result.editing_model_id {
            Some(id) => id.clone(),
            None => return,
        };

        let custom_headers: Option<std::collections::HashMap<String, String>> = if result.custom_headers.is_empty() {
            None
        } else {
            serde_json::from_str(&result.custom_headers).ok()
        };

        let custom_request_body: Option<String> = if result.custom_request_body.is_empty() {
            None
        } else {
            Some(result.custom_request_body.clone())
        };

        let model_config = northhing_core::service::config::AIModelConfig {
            id: model_id.clone(),
            name: result.name.clone(),
            provider: result.provider_format.clone(),
            model_name: result.model_name.clone(),
            base_url: result.base_url.clone(),
            api_key: result.api_key.clone(),
            context_window: Some(result.context_window),
            max_tokens: Some(result.max_tokens),
            enabled: true,
            enable_thinking_process: result.enable_thinking || result.support_preserved_thinking,
            skip_ssl_verify: result.skip_ssl_verify,
            custom_headers,
            custom_headers_mode: if result.custom_headers_mode.is_empty() || result.custom_headers_mode == "merge" {
                None
            } else {
                Some(result.custom_headers_mode.clone())
            },
            custom_request_body,
            ..Default::default()
        };

        let result_name = result.name.clone();
        let result_model_display = format!("{} / {}", result.model_name, result.name);

        let success = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = match GlobalConfigManager::service().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to get config service: {}", e);
                        return false;
                    }
                };

                if let Err(e) = config_service.update_ai_model(&model_id, model_config).await {
                    tracing::error!("Failed to update AI model: {}", e);
                    return false;
                }

                true
            })
        });

        if success {
            self.model_display_name = result_model_display;
            self.status = Some(format!("Model updated: {}", result_name));
            tracing::info!("Updated AI model: {}", model_id);
            self.load_current_model_name();
        } else {
            self.status = Some("Failed to update model".to_string());
        }
    }

    pub(super) fn show_agent_selector(&mut self) {
        self.push_current_popup_to_stack();

        let modes = self.get_mode_agents();
        if modes.is_empty() {
            self.status = Some("No mode agents available".to_string());
            return;
        }

        let agent_items: Vec<AgentItem> = modes
            .into_iter()
            .map(|m| AgentItem {
                id: m.id,
                description: m.description,
            })
            .collect();

        self.agent_selector.show(agent_items, Some(self.agent_type.clone()));
    }

    pub(super) fn apply_agent_selection(&mut self, selected: &AgentItem) {
        if selected.id != self.agent_type {
            self.agent_type = selected.id.clone();
            self.status = Some(format!("Agent switched to: {}", selected.id));
            // Reload model name for new agent
            self.load_current_model_name();
        }
    }

    pub(super) fn show_theme_selector(&mut self) {
        let themes = self.list_available_themes();
        if themes.is_empty() {
            self.status = Some("No themes available.".to_string());
            return;
        }

        self.push_current_popup_to_stack();
        self.begin_theme_preview();
        self.theme_selector.show(themes, Some(self.config.ui.theme_id.clone()));
        if let Some(selected) = self.theme_selector.selected_item().cloned() {
            self.preview_theme_selection(&selected);
        }
    }

    fn list_available_themes(&self) -> Vec<ThemeItem> {
        let mut themes: Vec<ThemeItem> = builtin_theme_ids().into_iter().map(|id| ThemeItem { id }).collect();

        themes.sort_by(|a, b| a.id.to_ascii_lowercase().cmp(&b.id.to_ascii_lowercase()));
        themes.dedup_by(|a, b| a.id == b.id);
        themes
    }

    fn current_base_theme(&self) -> (Theme, Appearance, EffectiveColorScheme) {
        let appearance = resolve_appearance(&self.config.ui.theme);
        let scheme = resolve_effective_color_scheme(&self.config.ui.color_scheme);
        let base_is_light = appearance.is_light();
        let base = match (base_is_light, scheme) {
            (_, EffectiveColorScheme::Monochrome) => Theme::monochrome(),
            (true, EffectiveColorScheme::Ansi16) => Theme::light_ansi16(),
            (true, EffectiveColorScheme::Truecolor) => Theme::light(),
            (false, EffectiveColorScheme::Ansi16) => Theme::dark_ansi16(),
            (false, EffectiveColorScheme::Truecolor) => Theme::dark(),
        };

        (base, appearance, scheme)
    }

    fn resolve_theme_by_id(
        &self,
        base: Theme,
        appearance: Appearance,
        scheme: EffectiveColorScheme,
        id: &str,
    ) -> Theme {
        if scheme == EffectiveColorScheme::Monochrome {
            return Theme::monochrome();
        }

        let id = id.trim();
        if id.is_empty() {
            return base;
        }

        if let Some(json) = builtin_theme_json(id) {
            return base
                .apply_opencode_theme_json(json, appearance)
                .unwrap_or(base)
                .with_effective_scheme(scheme);
        }

        base
    }

    fn begin_theme_preview(&mut self) {
        if self.theme_preview_original.is_none() {
            self.theme_preview_original = Some(self.theme.clone());
        }
    }

    pub(super) fn cancel_theme_preview(&mut self) {
        if let Some(original) = self.theme_preview_original.take() {
            self.theme = original;
        }
    }

    pub(super) fn preview_theme_selection(&mut self, theme: &ThemeItem) {
        self.begin_theme_preview();
        let (base, appearance, scheme) = self.current_base_theme();
        self.theme = self.resolve_theme_by_id(base, appearance, scheme, &theme.id);
        self.status = Some(format!("Preview theme: {} (Enter apply, Esc cancel)", theme.id));
    }

    pub(super) fn apply_theme_selection(&mut self, theme: &ThemeItem) {
        let (base, appearance, scheme) = self.current_base_theme();
        self.config.ui.theme_id = theme.id.clone();

        match self.config.save() {
            Ok(()) => {
                self.status = Some(format!("Theme set to: {}", theme.id));
            }
            Err(e) => {
                self.status = Some(format!("Failed to save config: {}", e));
            }
        }

        self.theme = self.resolve_theme_by_id(base, appearance, scheme, &theme.id);
        self.theme_preview_original = None;
    }

    pub(super) fn show_skill_selector(&mut self) {
        self.push_current_popup_to_stack();
        self.skill_selector.show_menu();
    }

    fn show_available_skill_list(&mut self) {
        let skills = tokio::task::block_in_place(|| {
            let workspace = self.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            tokio::runtime::Handle::current().block_on(async {
                let registry = SkillRegistry::global();
                registry
                    .get_resolved_skills_for_workspace(Some(workspace.as_path()), Some(&agent_type))
                    .await
            })
        });

        if skills.is_empty() {
            self.status = Some(format!("No enabled skills found for agent mode '{}'.", self.agent_type));
            return;
        }

        let skill_items: Vec<SkillItem> = skills.into_iter().map(Self::skill_item_from_info).collect();

        if skill_items.is_empty() {
            self.status = Some("No skills found.".to_string());
            return;
        }

        self.skill_selector.show_list(skill_items);
    }

    fn show_skill_config_selector(&mut self) {
        let skills = tokio::task::block_in_place(|| {
            let workspace = self.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            tokio::runtime::Handle::current().block_on(async {
                let registry = SkillRegistry::global();
                registry
                    .get_mode_skill_infos_for_workspace(Some(workspace.as_path()), &agent_type)
                    .await
            })
        });

        let skill_items: Vec<SkillItem> = skills.into_iter().map(Self::skill_item_from_mode_info).collect();

        if skill_items.is_empty() {
            self.status = Some("No skills found.".to_string());
            return;
        }

        self.skill_selector.show_config(skill_items);
    }

    pub(super) fn handle_skill_selector_action(&mut self, action: SkillSelectorAction) {
        match action {
            SkillSelectorAction::ListSkills => self.show_available_skill_list(),
            SkillSelectorAction::ConfigureSkills => self.show_skill_config_selector(),
            SkillSelectorAction::Execute(selected) => {
                self.skill_selector.hide();
                self.set_input(&format!("Execute the {} skill.", selected.name));
            }
            SkillSelectorAction::Toggle(selected) => {
                self.set_skill_enabled(&selected, !selected.enabled);
                self.show_skill_config_selector();
            }
        }
    }

    fn set_skill_enabled(&mut self, selected: &SkillItem, enabled: bool) {
        let workspace = self.workspace_path_buf();
        let mode_id = self.agent_type.clone();
        let skill = selected.clone();

        let result: Result<(), String> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match skill.level.as_str() {
                    "user" => {
                        set_user_mode_skill_state(&mode_id, &skill.key, enabled, skill.default_enabled)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    "project" => {
                        let mut document = load_project_mode_skills_document_local(&workspace)
                            .await
                            .map_err(|error| error.to_string())?;
                        set_mode_skill_disabled_in_document(&mut document, &mode_id, &skill.key, !enabled)
                            .map_err(|error| error.to_string())?;
                        save_project_mode_skills_document_local(&workspace, &document)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    other => {
                        return Err(format!("Unsupported skill level '{}'", other));
                    }
                }

                Ok(())
            })
        });

        self.status = Some(match result {
            Ok(()) => format!(
                "Skill '{}' {} for mode '{}'.",
                selected.name,
                if enabled { "enabled" } else { "disabled" },
                self.agent_type
            ),
            Err(error) => format!("Failed to update skill '{}': {}", selected.name, error),
        });
    }

    fn skill_item_from_info(info: SkillInfo) -> SkillItem {
        SkillItem {
            key: info.key,
            name: info.name,
            description: info.description,
            level: info.level.as_str().to_string(),
            enabled: true,
            selected_for_runtime: true,
            default_enabled: true,
            is_shadowed: info.is_shadowed,
        }
    }

    fn skill_item_from_mode_info(info: ModeSkillInfo) -> SkillItem {
        SkillItem {
            key: info.skill.key,
            name: info.skill.name,
            description: info.skill.description,
            level: info.skill.level.as_str().to_string(),
            enabled: info.effective_enabled,
            selected_for_runtime: info.selected_for_runtime,
            default_enabled: info.default_enabled,
            is_shadowed: info.skill.is_shadowed,
        }
    }

    pub(super) fn show_subagent_selector(&mut self) {
        self.push_current_popup_to_stack();
        self.subagent_selector.show_menu();
    }

    fn show_available_subagent_list(&mut self) {
        let registry = agent_registry();
        let subagents = tokio::task::block_in_place(|| {
            let workspace = self.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            tokio::runtime::Handle::current().block_on(registry.get_subagents_for_query(&SubagentQueryContext {
                parent_agent_type: Some(&agent_type),
                workspace_root: Some(workspace.as_path()),
                list_scope: SubagentListScope::TaskVisible,
                include_disabled: false,
            }))
        });

        if subagents.is_empty() {
            self.status = Some(format!(
                "No enabled subagents found for agent mode '{}'.",
                self.agent_type
            ));
            return;
        }

        let subagent_items: Vec<SubagentItem> = subagents.into_iter().map(Self::subagent_item_from_info).collect();

        if subagent_items.is_empty() {
            self.status = Some("No subagents found.".to_string());
            return;
        }

        self.subagent_selector.show_list(subagent_items);
    }

    fn show_subagent_config_selector(&mut self) {
        let registry = agent_registry();
        let subagents = tokio::task::block_in_place(|| {
            let workspace = self.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            tokio::runtime::Handle::current().block_on(registry.get_subagents_for_query(&SubagentQueryContext {
                parent_agent_type: Some(&agent_type),
                workspace_root: Some(workspace.as_path()),
                list_scope: SubagentListScope::RegistryManagement,
                include_disabled: true,
            }))
        });

        let subagent_items: Vec<SubagentItem> = subagents.into_iter().map(Self::subagent_item_from_info).collect();

        if subagent_items.is_empty() {
            self.status = Some("No subagents found.".to_string());
            return;
        }

        self.subagent_selector.show_config(subagent_items);
    }

    pub(super) fn handle_subagent_selector_action(&mut self, action: SubagentSelectorAction) {
        match action {
            SubagentSelectorAction::ListSubagents => self.show_available_subagent_list(),
            SubagentSelectorAction::ConfigureSubagents => self.show_subagent_config_selector(),
            SubagentSelectorAction::Launch(selected) => {
                self.subagent_selector.hide();
                self.set_input(&format!("Launch subagent {} to finish task: ", selected.name));
            }
            SubagentSelectorAction::Toggle(selected) => {
                self.set_subagent_enabled(&selected, !selected.enabled);
                self.show_subagent_config_selector();
            }
        }
    }

    fn set_subagent_enabled(&mut self, selected: &SubagentItem, enabled: bool) {
        let registry = agent_registry();
        let workspace = self.workspace_path_buf();
        let mode_id = self.agent_type.clone();
        let subagent = selected.clone();

        let result: Result<(), String> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                registry
                    .update_subagent_override(&mode_id, &subagent.id, enabled, Some(workspace.as_path()))
                    .await
                    .map_err(|error| error.to_string())
            })
        });

        self.status = Some(match result {
            Ok(()) => format!(
                "Subagent '{}' {} for mode '{}'.",
                selected.name,
                if enabled { "enabled" } else { "disabled" },
                self.agent_type
            ),
            Err(error) => format!("Failed to update subagent '{}': {}", selected.name, error),
        });
    }

    fn subagent_item_from_info(info: AgentInfo) -> SubagentItem {
        let source = match info.subagent_source {
            Some(SubAgentSource::Builtin) => "builtin",
            Some(SubAgentSource::Project) => "project",
            Some(SubAgentSource::User) => "user",
            None => "builtin",
        }
        .to_string();

        SubagentItem {
            key: info.key,
            id: info.id,
            name: info.name,
            description: info.description,
            source,
            enabled: info.effective_enabled,
            default_enabled: info.default_enabled,
        }
    }

    // ======================== Helpers ========================

    fn get_mode_agents(&self) -> Vec<AgentInfo> {
        let registry = agent_registry();
        let modes =
            tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(registry.get_modes_info()));
        modes
    }

    pub(super) fn cycle_agent(&mut self, offset: isize) {
        let modes = self.get_mode_agents();
        if modes.len() <= 1 {
            return;
        }

        let current_idx = modes.iter().position(|m| m.id == self.agent_type).unwrap_or(0);

        let len = modes.len() as isize;
        let next_idx = ((current_idx as isize + offset) % len + len) % len;
        let next = &modes[next_idx as usize];

        self.agent_type = next.id.clone();
        self.load_current_model_name();
    }

    pub(super) fn load_current_model_name(&mut self) {
        let agent_type = self.agent_type.clone();
        let result: Option<String> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let config_service = GlobalConfigManager::service().await.ok()?;
                let models: Vec<northhing_core::service::config::AIModelConfig> =
                    config_service.get_ai_models().await.ok()?;
                let global_config: northhing_core::service::config::GlobalConfig =
                    config_service.config(None).await.ok()?;

                let model_id = global_config
                    .ai
                    .agent_models
                    .get(&agent_type)
                    .cloned()
                    .or_else(|| global_config.ai.default_models.primary.clone())
                    .unwrap_or_else(|| "primary".to_string());

                fn provider_display_name(model: &northhing_core::service::config::AIModelConfig) -> String {
                    let raw_name = model.name.trim();
                    let model_name = model.model_name.trim();
                    if !raw_name.is_empty() && !model_name.is_empty() {
                        let dashed_suffix = format!(" - {}", model_name);
                        let slash_suffix = format!("/{}", model_name);
                        if let Some(provider) = raw_name.strip_suffix(&dashed_suffix) {
                            return provider.trim().to_string();
                        }
                        if let Some(provider) = raw_name.strip_suffix(&slash_suffix) {
                            return provider.trim().to_string();
                        }
                    }
                    if raw_name.is_empty() {
                        model.provider.clone()
                    } else {
                        raw_name.to_string()
                    }
                }

                fn model_display_name(model: &northhing_core::service::config::AIModelConfig) -> String {
                    format!("{} / {}", model.model_name, provider_display_name(model))
                }

                if model_id == "primary" {
                    let primary_id = global_config.ai.default_models.primary.as_deref()?;
                    models.iter().find(|m| m.id == primary_id).map(model_display_name)
                } else {
                    models.iter().find(|m| m.id == model_id).map(model_display_name)
                }
            })
        });

        self.model_display_name = result.unwrap_or_default();
    }
}
