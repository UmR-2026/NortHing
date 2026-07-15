//! Round 48b split sibling: scaffold resolution + model-summary retry
//!
//! Moved from execution/compression.rs.
//! Methods are `pub(super)` so sibling modules can call them via `self`.

use super::compression::CompressionRuntimeScaffold;
use super::execution_engine::ExecutionEngine;
use super::types::ExecutionContext;
use crate::agentic::agents::agent_registry;
use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
use crate::agentic::core::Message;
use crate::agentic::tools::resolve_tool_manifest;
use crate::agentic::tools::tool_context_runtime;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::get_global_config_service;
use crate::service::config::types::{ModelCapability, ModelCategory};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::Message as AIMessage;
use crate::util::types::ToolDefinition;
use northhing_ai_adapters::ModelExchangeTraceConfig;
use std::sync::Arc;
use tokio::time;

impl ExecutionEngine {
    pub(super) async fn request_compression_summary_with_retry(
        &self,
        ai_client: Arc<crate::infrastructure::ai::AIClient>,
        request_messages: Vec<AIMessage>,
        tool_definitions: Option<Vec<ToolDefinition>>,
        trace_config: Option<ModelExchangeTraceConfig>,
        max_tries: usize,
    ) -> NortHingResult<String> {
        let mut last_error = None;
        let base_wait_time_ms = 500;

        for attempt in 0..max_tries {
            let result = ai_client
                .send_message_with_trace(request_messages.clone(), tool_definitions.clone(), trace_config.clone())
                .await;

            match result {
                Ok(response) => {
                    if response.tool_calls.is_some() {
                        return Err(NortHingError::AIClient(
                            "Compression request returned tool calls instead of a summary".to_string(),
                        ));
                    }
                    if attempt > 0 {
                        tracing::debug!(
                            "Compression summary generation succeeded (attempt {}/{})",
                            attempt + 1,
                            max_tries
                        );
                    }
                    return Ok(response.text);
                }
                Err(err) => {
                    tracing::warn!(
                        "Compression summary generation failed (attempt {}/{}): {}",
                        attempt + 1,
                        max_tries,
                        err
                    );
                    last_error = Some(err);

                    if attempt < max_tries - 1 {
                        let delay_ms = base_wait_time_ms * (1 << attempt.min(3));
                        tracing::debug!(
                            "Waiting {}ms before compression summary retry {}...",
                            delay_ms,
                            attempt + 2
                        );
                        time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(NortHingError::AIClient(format!(
            "Compression summary generation failed after {} attempts: {}",
            max_tries,
            last_error
                .map(|err| err.to_string())
                .unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    pub(super) async fn resolve_compression_runtime_scaffold(
        &self,
        session: &crate::agentic::core::Session,
        context: &ExecutionContext,
    ) -> NortHingResult<CompressionRuntimeScaffold> {
        let agent_registry = agent_registry();
        if let Some(workspace) = context.workspace.as_ref() {
            agent_registry.load_custom_subagents(workspace.root_path()).await;
        }

        let current_agent = agent_registry
            .get_agent(
                &context.agent_type,
                context.workspace.as_ref().map(|workspace| workspace.root_path()),
            )
            .ok_or_else(|| NortHingError::NotFound(format!("Agent not found: {}", context.agent_type)))?;

        let original_user_input = context.context.get("original_user_input").cloned().unwrap_or_default();
        let model_id = self
            .resolve_model_id_for_turn(
                session,
                &context.agent_type,
                context.workspace.as_ref(),
                &original_user_input,
                context.turn_index,
            )
            .await?;

        let ai_client_factory = get_global_ai_client_factory()
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client factory: {}", e)))?;
        let ai_client = ai_client_factory
            .get_client_resolved(&model_id)
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client (model_id={}): {}", model_id, e)))?;

        let (resolved_primary_model_id, primary_supports_image_understanding) = {
            let config_service = get_global_config_service().await.ok();
            if let Some(service) = config_service {
                let ai_config: crate::service::config::types::AIConfig =
                    service.config(Some("ai")).await.unwrap_or_default();

                let resolved_id = Self::resolve_configured_model_id(&ai_config, &model_id);
                let model_cfg = ai_config
                    .models
                    .iter()
                    .find(|m| m.id == resolved_id)
                    .or_else(|| ai_config.models.iter().find(|m| m.name == resolved_id))
                    .or_else(|| ai_config.models.iter().find(|m| m.model_name == resolved_id))
                    .or_else(|| {
                        ai_config
                            .models
                            .iter()
                            .find(|m| m.model_name == ai_client.config.model && m.provider == ai_client.config.format)
                    });

                let supports = model_cfg.is_some_and(|m| {
                    m.capabilities
                        .iter()
                        .any(|cap| matches!(cap, ModelCapability::ImageUnderstanding))
                        || matches!(m.category, ModelCategory::Multimodal)
                });

                (resolved_id, supports)
            } else {
                tracing::warn!(
                    "Config service unavailable, assuming compression model is text-only for image input gating"
                );
                (model_id.clone(), false)
            }
        };

        let model_capability_profile =
            ModelCapabilityProfile::from_resolved_model(&resolved_primary_model_id, &ai_client.config.model);
        let is_review_subagent = agent_registry
            .get_subagent_is_review(&context.agent_type)
            .unwrap_or(false);
        let context_profile_policy =
            ContextProfilePolicy::for_agent_context(&context.agent_type, is_review_subagent, model_capability_profile);

        let tool_policy = agent_registry
            .get_agent_tool_policy(
                &context.agent_type,
                context.workspace.as_ref().map(|workspace| workspace.root_path()),
            )
            .await;
        let allowed_tools = tool_policy.allowed_tools.clone();
        let enable_tools = context
            .context
            .get("enable_tools")
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or(true);
        let tool_manifest_context_vars = context.context.clone();

        let tool_description_context = tool_context_runtime::build_tool_description_context(
            &context.agent_type,
            context.workspace.as_ref(),
            context.workspace_services.as_ref(),
            primary_supports_image_understanding,
            &tool_manifest_context_vars,
            None, // K.2.3 follow-up: tool manifest doesn't need actor_runtime
        );
        let tool_manifest = if enable_tools {
            Some(
                resolve_tool_manifest(
                    &allowed_tools,
                    &tool_policy.exposure_overrides,
                    &tool_description_context,
                )
                .await,
            )
        } else {
            None
        };
        let tool_listing_sections = if let Some(manifest) = tool_manifest.as_ref() {
            Self::build_tool_listing_sections(manifest, &tool_description_context).await
        } else {
            crate::agentic::agents::ToolListingSections::default()
        };
        let runtime_context_needs = tool_manifest
            .as_ref()
            .map(|manifest| {
                crate::agentic::agents::RuntimeContextNeeds::from_tool_names(manifest.allowed_tool_names.iter())
            })
            .unwrap_or_default();
        // Snapshot prompt-visible tool definitions once for this turn. Do not
        // re-resolve or rewrite them after GetToolSpec unlocks a collapsed tool:
        // the unlocked detail travels in tool results, while mutating the tool
        // definitions would change the request prefix and trigger provider
        // prefix/KV cache misses on subsequent rounds.
        let tool_definitions = tool_manifest.map(|manifest| manifest.tool_definitions);

        let prompt_context = Self::build_prompt_context(
            context,
            &ai_client.config.model,
            primary_supports_image_understanding,
            tool_listing_sections,
            runtime_context_needs,
        )
        .await;
        let prepended_prompt_reminders = self
            .build_cached_prepended_prompt_reminders(
                &context.session_id,
                current_agent.as_ref(),
                prompt_context.as_ref(),
                &context.context,
            )
            .await;
        let system_prompt = self
            .resolve_cached_system_prompt(&context.session_id, current_agent.as_ref(), prompt_context.as_ref())
            .await?;

        Ok(CompressionRuntimeScaffold {
            ai_client,
            tool_definitions,
            system_prompt_message: Message::system(system_prompt),
            prepended_prompt_reminders,
            primary_supports_image_understanding,
            compression_contract_limit: context_profile_policy.compression_contract_limit,
        })
    }
}
