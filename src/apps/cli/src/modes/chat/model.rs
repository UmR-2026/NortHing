//! Chat mode model selector — load, show, apply.
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;
use crate::ui::model_selector::ModelItem;

use northhing_core::service::config::GlobalConfigManager;

use super::ChatMode;

impl ChatMode {
    /// Load current model name from global config for display
    pub(crate) fn load_current_model_name(&self, chat_state: &mut ChatState, rt_handle: &tokio::runtime::Handle) {
        let agent_type = self.agent_type.clone();
        let result: Option<String> = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                let config_service = GlobalConfigManager::service().await.ok()?;
                let models: Vec<northhing_core::service::config::AIModelConfig> =
                    config_service.get_ai_models().await.ok()?;
                let global_config: northhing_core::service::config::GlobalConfig =
                    config_service.config(None).await.ok()?;

                // Resolve model ID for the current agent
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

                // Find model name
                let model_name = if model_id == "primary" {
                    // Resolve primary model
                    let primary_id = global_config.ai.default_models.primary.as_deref()?;
                    models.iter().find(|m| m.id == primary_id).map(model_display_name)
                } else {
                    models.iter().find(|m| m.id == model_id).map(model_display_name)
                };

                model_name
            })
        });

        if let Some(name) = result {
            chat_state.current_model_name = name;
        }
    }

    /// Show model selector popup with all available models
    pub(crate) fn show_model_selector(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let agent_type = self.agent_type.clone();
        let result = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                let config_service = match GlobalConfigManager::service().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to get config service: {}", e);
                        return None;
                    }
                };

                let models: Vec<northhing_core::service::config::AIModelConfig> =
                    config_service.get_ai_models().await.ok()?;
                let global_config: northhing_core::service::config::GlobalConfig =
                    config_service.config(None).await.ok()?;

                // Get current model ID
                let current_model_id = global_config
                    .ai
                    .agent_models
                    .get(&agent_type)
                    .cloned()
                    .or_else(|| global_config.ai.default_models.primary.clone());

                // Convert to ModelItem list (only enabled models)
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
                chat_view.show_model_selector(models, current_id);
            }
            _ => {
                chat_state.add_system_message("No available models found. Please configure models first.".to_string());
            }
        }
    }

    /// Apply model selection: update global config and chat state
    pub(crate) fn apply_model_selection(
        &self,
        selected: &ModelItem,
        _chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let selected_id = selected.id.clone();
        let selected_display_name = format!("{} / {}", selected.model_name, selected.name);
        let modes = self.get_mode_agents(rt_handle);

        let success = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                let config_service = match GlobalConfigManager::service().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to get config service: {}", e);
                        return false;
                    }
                };

                // Update default primary model
                if let Err(e) = config_service
                    .set_config("ai.default_models.primary", &selected_id)
                    .await
                {
                    tracing::error!("Failed to set default primary model: {}", e);
                    return false;
                }

                // Update agent_models for all modes
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
            chat_state.current_model_name = selected_display_name.clone();
            tracing::info!("Model switched to: {} ({})", selected_display_name, selected_id);
        } else {
            tracing::error!("Failed to switch model: {} ({})", selected_display_name, selected_id);
        }
    }
}
