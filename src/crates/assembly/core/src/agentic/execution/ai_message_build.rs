//! Round 8 split sibling: ai_message_build
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
    pub(super) async fn run_finalize_round(
        &self,
        ai_client: Arc<crate::infrastructure::ai::AIClient>,
        context: &ExecutionContext,
        agent_type: String,
        round_number: usize,
        execution_context_vars: &HashMap<String, String>,
        primary_supports_image_understanding: bool,
        prepended_reminders: &[&str],
        messages: &[Message],
        reminder_text: &str,
        context_window: usize,
    ) -> NortHingResult<RoundResult> {
        let mut final_ai_messages = Self::build_ai_messages_for_send(
            messages,
            &ai_client.config.format,
            context.workspace.as_ref().map(|workspace| workspace.root_path()),
            &context.dialog_turn_id,
            primary_supports_image_understanding,
            prepended_reminders,
        )
        .await?;
        final_ai_messages.push(AIMessage::user(reminder_text.to_string()));

        let round_context = RoundContext {
            session_id: context.session_id.clone(),
            subagent_parent_info: context.subagent_parent_info.clone(),
            dialog_turn_id: context.dialog_turn_id.clone(),
            turn_index: context.turn_index,
            round_number,
            workspace: context.workspace.clone(),
            messages: messages.to_vec(),
            available_tools: Vec::new(),
            collapsed_tools: Vec::new(),
            unlocked_collapsed_tools: Vec::new(),
            model_name: ai_client.config.model.clone(),
            agent_type,
            context_vars: execution_context_vars.clone(),
            delegation_policy: context.delegation_policy,
            runtime_tool_restrictions: context.runtime_tool_restrictions.clone(),
            steering_interrupt: None,
            cancellation_token: CancellationToken::new(),
            workspace_services: context.workspace_services.clone(),
            recover_partial_on_cancel: context.recover_partial_on_cancel,
        };

        // Tools are disabled here (None) — model must respond in plain text.
        self.round_executor
            .execute_round(ai_client, round_context, final_ai_messages, None, Some(context_window))
            .await
    }

    pub(super) async fn build_ai_messages_for_send(
        messages: &[Message],
        provider: &str,
        workspace_path: Option<&Path>,
        current_turn_id: &str,
        attach_images: bool,
        prepended_reminders: &[&str],
    ) -> NortHingResult<Vec<AIMessage>> {
        /// Only the last this many **messages** that contain images keep their images for the API.
        const MAX_IMAGE_BEARING_MESSAGE_ROUNDS: usize = 2;

        let limits = ImageLimits::for_provider(provider);

        let trimmed_reminders = prepended_reminders
            .iter()
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>();
        let mut result = Vec::with_capacity(messages.len() + trimmed_reminders.len());
        let mut attached_image_count = 0usize;
        let first_non_system_index = messages
            .iter()
            .position(|msg| msg.role != crate::agentic::core::MessageRole::System)
            .unwrap_or(messages.len());
        let mut prepended_reminders_injected = false;

        let keep_image_messages = if attach_images {
            Self::image_bearing_indices_to_keep(messages, MAX_IMAGE_BEARING_MESSAGE_ROUNDS)
        } else {
            HashSet::new()
        };

        for (msg_idx, msg) in messages.iter().enumerate() {
            if !prepended_reminders_injected && msg_idx == first_non_system_index {
                for reminder in &trimmed_reminders {
                    result.push(AIMessage::user(render_system_reminder(reminder)));
                }
                prepended_reminders_injected = true;
            }

            if Self::skip_message_for_model_send(msg) {
                continue;
            }
            let keep_this_message_images = attach_images && keep_image_messages.contains(&msg_idx);
            match &msg.content {
                MessageContent::Multimodal { text, images } => {
                    if !attach_images {
                        // Primary model is text-only (or images are disabled). Convert to text-only
                        // placeholder so providers that don't support image inputs won't error.
                        result.push(AIMessage::from(msg));
                        continue;
                    }

                    let (filtered_images, dropped_count): (Vec<ImageContextData>, usize) = if images.is_empty() {
                        (Vec::new(), 0)
                    } else if keep_this_message_images {
                        (images.clone(), 0)
                    } else {
                        (Vec::new(), images.len())
                    };

                    let prompt = if text.trim().is_empty() {
                        "(image attached)".to_string()
                    } else {
                        text.clone()
                    };
                    let prompt = if dropped_count > 0 {
                        format!(
                            "{}\n\n[{} image(s) from this message omitted: only the latest {} message(s) in the conversation that contain images are sent to the model.]",
                            prompt.trim_end(),
                            dropped_count,
                            MAX_IMAGE_BEARING_MESSAGE_ROUNDS
                        )
                    } else {
                        prompt
                    };

                    match process_image_contexts_for_provider(&filtered_images, provider, workspace_path).await {
                        Ok(processed) => {
                            let next_count = attached_image_count + processed.len();
                            if next_count > limits.max_images_per_request {
                                return Err(NortHingError::validation(format!(
                                    "Too many images in one request: {} > {}",
                                    next_count, limits.max_images_per_request
                                )));
                            }
                            attached_image_count = next_count;

                            let multimodal = build_multimodal_message_with_images(&prompt, &processed, provider)?;
                            result.extend(multimodal);
                        }
                        Err(err) => {
                            if matches!(&err, NortHingError::Validation(msg) if msg.starts_with("Too many images in one request"))
                            {
                                return Err(err);
                            }
                            let is_current_turn_message = msg.metadata.turn_id.as_deref() == Some(current_turn_id);
                            if Self::can_fallback_to_text_only(images, &err, is_current_turn_message) {
                                warn!(
                                    "Failed to rebuild multimodal payload, falling back to text-only message: message_id={}, provider={}, turn_id={:?}, current_turn_id={}, error={}",
                                    msg.id, provider, msg.metadata.turn_id, current_turn_id, err
                                );
                                result.push(AIMessage::from(msg));
                            } else {
                                return Err(err);
                            }
                        }
                    }
                }
                MessageContent::ToolResult { .. } => {
                    if !attach_images {
                        result.push(AIMessage::from(msg));
                        continue;
                    }
                    let mut ai = AIMessage::from(msg.clone());
                    if let Some(atts) = ai.tool_image_attachments.take() {
                        if !atts.is_empty() {
                            if keep_this_message_images {
                                let next_count = attached_image_count + atts.len();
                                if next_count > limits.max_images_per_request {
                                    return Err(NortHingError::validation(format!(
                                        "Too many images in one request: {} > {}",
                                        next_count, limits.max_images_per_request
                                    )));
                                }
                                attached_image_count = next_count;
                                ai.tool_image_attachments = Some(atts);
                            } else {
                                let dropped = atts.len();
                                let content_str = ai.content.as_deref().unwrap_or("");
                                ai.content = Some(format!(
                                    "{}\n\n[{} image(s) from this tool result omitted: only the latest {} message(s) in the conversation that contain images are sent to the model.]",
                                    content_str.trim_end(),
                                    dropped,
                                    MAX_IMAGE_BEARING_MESSAGE_ROUNDS
                                ));
                                ai.tool_image_attachments = None;
                            }
                        }
                    }
                    result.push(ai);
                }
                _ => result.push(AIMessage::from(msg)),
            }
        }

        if !prepended_reminders_injected {
            for reminder in trimmed_reminders {
                result.push(AIMessage::user(render_system_reminder(reminder)));
            }
        }

        Ok(result)
    }

    pub(super) async fn build_compression_request_messages(
        &self,
        runtime_messages: &[Message],
        dialog_turn_id: &str,
        workspace: Option<&WorkspaceBinding>,
        provider: &str,
        attach_images: bool,
        prepended_prompt_reminders: &PrependedPromptReminders,
        contract: Option<&crate::agentic::core::CompressionContract>,
    ) -> NortHingResult<Vec<AIMessage>> {
        let prepended_reminders = prepended_prompt_reminders.ordered_reminders();
        let mut compression_messages = Self::build_ai_messages_for_send(
            runtime_messages,
            provider,
            workspace.map(|workspace| workspace.root_path()),
            dialog_turn_id,
            attach_images,
            &prepended_reminders,
        )
        .await?;
        compression_messages.push(AIMessage::user(self.context_compressor.build_compact_prompt(contract)));
        Ok(compression_messages)
    }
}
