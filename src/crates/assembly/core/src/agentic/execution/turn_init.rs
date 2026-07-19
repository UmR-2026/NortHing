//! Round 8 split sibling: turn_init
//!
//! Auto-extracted from execution_engine.rs as part of the Round 8 sub-domain split.
//! Methods are declared `pub(super)` so the facade (`execution_engine.rs`) can call them.

use super::compression::CompressionRuntimeScaffold;
use super::execution_engine::ContextCompactionOutcome;
use super::execution_engine::ExecutionEngine;
use super::health_snapshot::ContextHealthSnapshot;

use super::model_exchange_trace::{prepare_model_exchange_trace_for_workspace, ModelExchangeTraceOperation};
use super::round_executor::RoundExecutor;
use super::types::{ExecutionContext, ExecutionResult, ExecutionTurnState, RoundContext, RoundResult, RoundTickResult};
use crate::agentic::agents::{
    agent_registry, build_prompt_context_for_workspace, PartitionedLoader, PrependedPromptReminders, PromptBuilder,
    PromptBuilderContext, RuntimeContextNeeds, ToolListingSections, USE_PARTITIONED_LOADER,
};
use crate::agentic::context_profile::{ContextProfilePolicy, ModelCapabilityProfile};
use crate::agentic::core::{
    render_system_reminder, InternalReminderKind, Message, MessageContent, MessageHelper, MessageRole,
    MessageSemanticKind, RequestReasoningTokenPolicy, Session,
};
use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
use crate::agentic::execution::types::FinishReason;
use crate::agentic::image_analysis::{
    build_multimodal_message_with_images, process_image_contexts_for_provider, ImageContextData, ImageLimits,
};
use crate::agentic::remote_file_delivery::TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY;
use crate::agentic::round_preempt::RoundInjectionKind;
use crate::agentic::session::{CompressionMode, ContextCompressor, SessionManager};
use crate::agentic::skill_agent_snapshot::build_skill_agent_tool_listing_sections_from_snapshot;
use crate::agentic::tools::implementations::{SkillTool, TaskTool};
use crate::agentic::tools::product_runtime::{collect_product_unlocked_collapsed_tools, GetToolSpecTool};
use crate::agentic::tools::{resolve_tool_manifest, tool_context_runtime, ResolvedToolManifest};
use crate::agentic::WorkspaceBinding;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::get_global_config_service;
use crate::service::config::types::{ModelCapability, ModelCategory};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::token_counter::TokenCounter;
use crate::util::types::Message as AIMessage;
use crate::util::types::ToolDefinition;
use crate::util::{elapsed_ms_u64, truncate_at_char_boundary};
use northhing_ai_adapters::ModelExchangeTraceConfig;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

impl ExecutionEngine {
    pub(super) async fn init_turn_impl(
        &self,
        agent_type: String,
        initial_messages: Vec<Message>,
        context: &ExecutionContext,
    ) -> NortHingResult<ExecutionTurnState> {
        debug!("Initializing dialog turn: dialog_turn_id={}", context.dialog_turn_id);

        // W4-P: elapsed reference for the diagnostic probes below.
        let w4_start = std::time::Instant::now();

        // 1. Get current agent
        let agent_registry = agent_registry();
        if let Some(workspace) = context.workspace.as_ref() {
            agent_registry.load_custom_subagents(workspace.root_path()).await;
        }
        let current_agent = agent_registry
            .get_agent(
                &agent_type,
                context.workspace.as_ref().map(|workspace| workspace.root_path()),
            )
            .ok_or_else(|| NortHingError::NotFound(format!("Agent not found: {}", agent_type)))?;
        info!("Current Agent: {} ({})", current_agent.name(), current_agent.id());

        let session = self
            .session_manager
            .get_session(&context.session_id)
            .ok_or_else(|| NortHingError::Session(format!("Session not found: {}", context.session_id)))?;

        // 2. Get AI client
        let original_user_input = context.context.get("original_user_input").cloned().unwrap_or_default();
        let model_id = self
            .resolve_model_id_for_turn(
                &session,
                &agent_type,
                context.workspace.as_ref(),
                &original_user_input,
                context.turn_index,
            )
            .await?;
        info!(
            "Agent using model: agent={}, resolved_model_id={}",
            current_agent.name(),
            model_id
        );

        let w4_thread = std::thread::current();
        info!(
            "W4-P: before get_global_ai_client_factory thread={:?} elapsed_ms={}",
            w4_thread.name(),
            w4_start.elapsed().as_millis()
        );
        let ai_client_factory = get_global_ai_client_factory()
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client factory: {}", e)))?;
        info!(
            "W4-P: after get_global_ai_client_factory elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );

        info!(
            "W4-P: before get_client_resolved elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let ai_client = ai_client_factory
            .get_client_resolved(&model_id)
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client (model_id={}): {}", model_id, e)))?;
        info!(
            "W4-P: after get_client_resolved elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );

        // Primary model vision capability
        info!(
            "W4-P: before config_service_block elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
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
                warn!("Config service unavailable, assuming primary model is text-only for image input gating");
                (model_id.clone(), false)
            }
        };
        info!(
            "W4-P: after config_service_block elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );

        let model_context_window = ai_client.config.context_window as usize;
        let session_max_tokens = session.config.max_context_tokens;
        let context_window = model_context_window.min(session_max_tokens);
        if model_context_window != session_max_tokens {
            debug!(
                "Context window: model={}, session_config={}, effective={}",
                model_context_window, session_max_tokens, context_window
            );
        }

        let model_capability_profile =
            ModelCapabilityProfile::from_resolved_model(&resolved_primary_model_id, &ai_client.config.model);
        let is_review_subagent = agent_registry.get_subagent_is_review(&agent_type).unwrap_or(false);
        let context_profile_policy =
            ContextProfilePolicy::for_agent_context(&agent_type, is_review_subagent, model_capability_profile);

        // 3. Get available tools
        info!(
            "W4-P: before get_agent_tool_policy elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let tool_policy = agent_registry
            .get_agent_tool_policy(
                &agent_type,
                context.workspace.as_ref().map(|workspace| workspace.root_path()),
            )
            .await;
        info!(
            "W4-P: after get_agent_tool_policy elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let allowed_tools = tool_policy.allowed_tools.clone();
        let enable_tools = context
            .context
            .get("enable_tools")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);
        let tool_manifest_context_vars = context.context.clone();

        let tool_description_context = tool_context_runtime::build_tool_description_context(
            &agent_type,
            context.workspace.as_ref(),
            context.workspace_services.as_ref(),
            primary_supports_image_understanding,
            &tool_manifest_context_vars,
            None,
        );

        info!(
            "W4-P: before resolve_tool_manifest elapsed_ms={}",
            w4_start.elapsed().as_millis()
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
        info!(
            "W4-P: after resolve_tool_manifest elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let collapsed_tools = tool_manifest
            .as_ref()
            .map(|manifest| manifest.collapsed_tool_names.clone())
            .unwrap_or_default();
        info!(
            "W4-P: before build_tool_listing_sections elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let tool_listing_sections = if let Some(manifest) = tool_manifest.as_ref() {
            Self::build_tool_listing_sections(manifest, &tool_description_context).await
        } else {
            ToolListingSections::default()
        };
        info!(
            "W4-P: after build_tool_listing_sections elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let runtime_context_needs = tool_manifest
            .as_ref()
            .map(|manifest| RuntimeContextNeeds::from_tool_names(manifest.allowed_tool_names.iter()))
            .unwrap_or_default();

        let (available_tools, tool_definitions) = if let Some(manifest) = tool_manifest {
            (manifest.allowed_tool_names, Some(manifest.tool_definitions))
        } else {
            (vec![], None)
        };

        // 4. Get System Prompt
        debug!(
            "Building system prompt from agent: {}, model={}",
            current_agent.name(),
            ai_client.config.model
        );
        info!(
            "W4-P: before build_prompt_context elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let prompt_context = Self::build_prompt_context(
            context,
            &ai_client.config.model,
            primary_supports_image_understanding,
            tool_listing_sections,
            runtime_context_needs,
        )
        .await;
        info!(
            "W4-P: after build_prompt_context elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        info!(
            "W4-P: before build_cached_prepended_prompt_reminders elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let prepended_reminders = self
            .build_cached_prepended_prompt_reminders(
                &context.session_id,
                current_agent.as_ref(),
                prompt_context.as_ref(),
                &context.context,
            )
            .await;
        info!(
            "W4-P: after build_cached_prepended_prompt_reminders elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        info!(
            "W4-P: before resolve_cached_system_prompt elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        let system_prompt = self
            .resolve_cached_system_prompt(&context.session_id, current_agent.as_ref(), prompt_context.as_ref())
            .await?;
        info!(
            "W4-P: after resolve_cached_system_prompt elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        debug!("System prompt built, length: {} bytes", system_prompt.len());

        let system_prompt_message = Message::system(system_prompt.clone());

        let enable_context_compression = session.config.enable_context_compression;
        let compression_threshold = session.config.compression_threshold;

        let mut execution_context_vars = context.context.clone();
        execution_context_vars.insert("primary_model_id".to_string(), resolved_primary_model_id.clone());
        execution_context_vars.insert("primary_model_name".to_string(), ai_client.config.model.clone());
        execution_context_vars.insert("primary_model_provider".to_string(), ai_client.config.format.clone());
        execution_context_vars.insert(
            "primary_model_supports_image_understanding".to_string(),
            primary_supports_image_understanding.to_string(),
        );
        execution_context_vars.insert("turn_index".to_string(), context.turn_index.to_string());

        let setup = super::types::ExecutionTurnSetup {
            session_id: context.session_id.clone(),
            dialog_turn_id: context.dialog_turn_id.clone(),
            workspace: context.workspace.clone(),
            ai_client,
            agent_type,
            model_id,
            resolved_primary_model_id,
            primary_supports_image_understanding,
            context_window,
            available_tools,
            collapsed_tools,
            tool_definitions,
            prepended_reminders,
            context_profile_policy,
            enable_context_compression,
            compression_threshold,
            system_prompt_message,
            initial_messages,
            execution_context_vars,
        };

        info!(
            "W4-P: init_turn_impl return elapsed_ms={}",
            w4_start.elapsed().as_millis()
        );
        Ok(ExecutionTurnState::from_setup(setup))
    }
}
