//! Round 8 split sibling: turn_lifecycle
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
    pub(super) fn resolve_configured_model_id(
        ai_config: &crate::service::config::types::AIConfig,
        model_id: &str,
    ) -> String {
        let trimmed = model_id.trim();
        if trimmed.is_empty() || trimmed == "auto" || trimmed == "default" {
            return "auto".to_string();
        }
        ai_config
            .resolve_model_selection(trimmed)
            .unwrap_or_else(|| "auto".to_string())
    }

    pub(super) async fn build_tool_listing_sections(
        manifest: &ResolvedToolManifest,
        tool_context: &crate::agentic::tools::framework::ToolUseContext,
    ) -> ToolListingSections {
        let has_tool_definition = |tool_name: &str| {
            manifest
                .tool_definitions
                .iter()
                .any(|definition| definition.name == tool_name)
        };

        ToolListingSections {
            skill_listing: if has_tool_definition("Skill") {
                SkillTool::build_available_skills_context_section(Some(tool_context)).await
            } else {
                None
            },
            agent_listing: if has_tool_definition("Task") {
                TaskTool::build_available_agents_context_section(Some(tool_context)).await
            } else {
                None
            },
            collapsed_tool_listing: if has_tool_definition("GetToolSpec") {
                GetToolSpecTool::build_collapsed_tools_context_section(&manifest.collapsed_tool_summaries)
            } else {
                None
            },
        }
    }

    pub(super) async fn build_prompt_context(
        context: &ExecutionContext,
        model_name: &str,
        supports_image_understanding: bool,
        tool_listing_sections: ToolListingSections,
        runtime_context_needs: RuntimeContextNeeds,
    ) -> Option<PromptBuilderContext> {
        let workspace = context.workspace.as_ref()?;
        let remote_file_delivery_channel = context
            .context
            .get(TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY)
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or(false);

        build_prompt_context_for_workspace(
            workspace,
            workspace.workspace_id.as_deref(),
            &context.session_id,
            Some(model_name.to_string()),
            Some(supports_image_understanding),
            tool_listing_sections,
            runtime_context_needs,
        )
        .await
        .map(|prompt_context| prompt_context.with_remote_file_delivery_channel(remote_file_delivery_channel))
    }

    pub(super) async fn build_cached_prepended_prompt_reminders(
        &self,
        session_id: &str,
        current_agent: &dyn crate::agentic::agents::Agent,
        prompt_context: Option<&PromptBuilderContext>,
        _context_vars: &HashMap<String, String>,
    ) -> PrependedPromptReminders {
        let Some(prompt_context) = prompt_context.cloned() else {
            return PrependedPromptReminders::default();
        };

        let prompt_builder = PromptBuilder::new(prompt_context);
        let baseline_snapshot = if let Some(snapshot) = self
            .session_manager
            .skill_agent_baseline_override_snapshot(session_id)
            .await
        {
            Some(snapshot)
        } else {
            self.session_manager.turn_skill_agent_snapshot(session_id, 0).await
        };
        let baseline_tool_sections =
            baseline_snapshot.map(|snapshot| build_skill_agent_tool_listing_sections_from_snapshot(&snapshot));
        if baseline_tool_sections.is_none() {
            warn!(
                "Listing reminder baseline snapshot unavailable while building prepended reminders: session_id={}",
                session_id
            );
        }
        let user_context_identity = current_agent.user_context_cache_identity();
        let user_context = if let Some(cached_user_context) = self
            .session_manager
            .cached_user_context(session_id, &user_context_identity)
            .await
        {
            debug!(
                "User context cache hit: session_id={}, scope_key={}",
                session_id, user_context_identity.scope_key
            );
            Some(cached_user_context)
        } else {
            debug!(
                "User context cache miss: session_id={}, scope_key={}",
                session_id, user_context_identity.scope_key
            );
            let built_user_context = prompt_builder
                .build_user_context_reminder(&current_agent.user_context_policy())
                .await;
            if let Some(ref user_context) = built_user_context {
                self.session_manager
                    .remember_user_context(session_id, user_context_identity.clone(), user_context.clone())
                    .await;
            }
            built_user_context
        };
        let runtime_context = prompt_builder.build_runtime_context_reminder().await;

        PrependedPromptReminders {
            collapsed_tool_listing: prompt_builder.build_collapsed_tool_listing_reminder(),
            skill_listing: baseline_tool_sections
                .as_ref()
                .and_then(|sections| sections.render_skill_listing_reminder()),
            agent_listing: baseline_tool_sections
                .as_ref()
                .and_then(|sections| sections.render_agent_listing_reminder()),
            runtime_context,
            user_context,
        }
    }

    pub(super) async fn resolve_cached_system_prompt(
        &self,
        session_id: &str,
        current_agent: &dyn crate::agentic::agents::Agent,
        prompt_context: Option<&PromptBuilderContext>,
    ) -> NortHingResult<String> {
        let identity = prompt_context
            .map(|context| current_agent.system_prompt_cache_identity(context.model_name.as_deref()))
            .unwrap_or_else(|| current_agent.system_prompt_cache_identity(None));

        // v3 Phase 1: Try PartitionedLoader first (Layer 2/3 in-memory caching)
        if USE_PARTITIONED_LOADER {
            if let Some(context) = prompt_context {
                let template_name = current_agent.prompt_template_name(context.model_name.as_deref());
                let mut loader = PartitionedLoader::new(template_name);
                // Try to build system prompt (uses Layer 2 + Layer 3 caching)
                match loader.build_system_prompt(context, None).await {
                    Ok(partitioned_prompt) => {
                        // Also store in SessionManager for backward compatibility
                        self.session_manager
                            .remember_system_prompt(session_id, identity.clone(), partitioned_prompt.clone())
                            .await;
                        debug!(
                            "System prompt partitioned loader hit: session_id={}, scope_key={}",
                            session_id, identity.scope_key
                        );
                        return Ok(partitioned_prompt);
                    }
                    Err(e) => {
                        warn!("PartitionedLoader failed, falling back to legacy path: {}", e);
                    }
                }
            }
        }

        // Legacy path: SessionManager cache -> Agent::get_system_prompt -> SessionManager::remember
        if let Some(cached_system_prompt) = self.session_manager.cached_system_prompt(session_id, &identity).await {
            debug!(
                "System prompt cache hit: session_id={}, scope_key={}",
                session_id, identity.scope_key
            );
            return Ok(cached_system_prompt);
        }

        debug!(
            "System prompt cache miss: session_id={}, scope_key={}",
            session_id, identity.scope_key
        );
        let system_prompt = current_agent.get_system_prompt(prompt_context).await?;
        self.session_manager
            .remember_system_prompt(session_id, identity, system_prompt.clone())
            .await;
        Ok(system_prompt)
    }

    pub(super) async fn resolve_model_id_for_turn_impl(
        &self,
        session: &Session,
        agent_type: &str,
        workspace: Option<&WorkspaceBinding>,
        original_user_input: &str,
        turn_index: usize,
    ) -> NortHingResult<String> {
        let agent_registry = agent_registry();
        let fallback_model_id = agent_registry
            .get_model_id_for_agent(agent_type, workspace.map(|binding| binding.root_path()))
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get model ID: {}", e)))?;
        let config_service = get_global_config_service().await.map_err(|e| {
            NortHingError::AIClient(format!("Failed to get config service for model resolution: {}", e))
        })?;
        let ai_config: crate::service::config::types::AIConfig =
            config_service.config(Some("ai")).await.unwrap_or_default();
        let configured_model_id = session
            .config
            .model_id
            .as_ref()
            .map(|model_id| model_id.trim())
            .filter(|model_id| !model_id.is_empty())
            .map(str::to_string)
            .unwrap_or(fallback_model_id.clone());
        let resolved_configured_model_id = Self::resolve_configured_model_id(&ai_config, &configured_model_id);

        let model_id = if configured_model_id == "auto"
            || configured_model_id == "default"
            || resolved_configured_model_id == "auto"
        {
            let fallback_model = "primary";
            let resolved_model_id = ai_config.resolve_model_selection(fallback_model);

            if let Some(resolved_model_id) = resolved_model_id {
                info!(
                    "Auto model resolved without locking session: session_id={}, turn_index={}, user_input_chars={}, strategy={}, resolved_model_id={}",
                    session.session_id,
                    turn_index,
                    original_user_input.chars().count(),
                    fallback_model,
                    resolved_model_id
                );

                resolved_model_id
            } else {
                warn!(
                    "Auto model strategy unresolved, keeping symbolic selector: session_id={}, strategy={}",
                    session.session_id, fallback_model
                );
                fallback_model.to_string()
            }
        } else {
            resolved_configured_model_id
        };

        Ok(model_id)
    }
}
