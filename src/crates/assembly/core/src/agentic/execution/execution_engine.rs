//! Execution Engine
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

/// Execution engine configuration
#[derive(Debug, Clone)]
pub struct ExecutionEngineConfig {
    pub max_rounds: usize,
    /// Max consecutive rounds with identical tool-call signatures before loop detection triggers.
    pub max_consecutive_same_tool: usize,
}

impl Default for ExecutionEngineConfig {
    fn default() -> Self {
        Self {
            max_rounds: crate::service::config::types::DEFAULT_MAX_ROUNDS,
            max_consecutive_same_tool: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextCompactionOutcome {
    pub compression_id: String,
    pub compression_count: usize,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub compression_ratio: f64,
    pub duration_ms: u64,
    pub has_summary: bool,
    pub summary_source: String,
    pub applied: bool,
}

/// Execution engine
pub struct ExecutionEngine {
    pub(crate) round_executor: Arc<RoundExecutor>,
    pub(crate) event_queue: Arc<EventQueue>,
    pub(crate) session_manager: Arc<SessionManager>,
    pub(crate) context_compressor: Arc<ContextCompressor>,
    pub(crate) config: ExecutionEngineConfig,
}

impl ExecutionEngine {
    pub(super) const FINALIZE_AFTER_TOOL_USE_REMINDER: &'static str = "Tool execution for this turn has already completed, but the turn is ending at this round boundary. Do not call any more tools. Provide the final response to the user based on the tool results already available.";
    pub(super) const FINALIZE_AFTER_REPEATED_TOOL_FAILURES_REMINDER: &'static str = "Repeated tool attempts have failed and tool use is now disabled for this turn. Provide a concise final response explaining what was completed, what failed, the evidence from the tool results, and the best actionable next step. Do not call any more tools.";
    pub(super) const FINALIZE_AFTER_MAX_ROUNDS_REMINDER: &'static str = "The execution round budget has been reached and tool use is now disabled for this turn. Provide the best final response possible from the work and evidence already collected. Clearly distinguish completed work from unresolved items. Do not call any more tools.";
    pub(super) const FORCE_TEXT_ONLY_REMINDER: &'static str = "STOP. Tool calls are disabled for this final turn. Respond ONLY with a plain-text answer summarizing what you have done and the result for the user. Do not output tool call syntax of any kind.";

    pub fn new(
        round_executor: Arc<RoundExecutor>,
        event_queue: Arc<EventQueue>,
        session_manager: Arc<SessionManager>,
        context_compressor: Arc<ContextCompressor>,
        config: ExecutionEngineConfig,
    ) -> Self {
        Self {
            round_executor,
            event_queue,
            session_manager,
            context_compressor,
            config,
        }
    }

    pub async fn execute_dialog_turn(
        &self,
        agent_type: String,
        initial_messages: Vec<Message>,
        context: ExecutionContext,
    ) -> NortHingResult<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let initial_count = initial_messages.len();

        let dialog_turn_id = context.dialog_turn_id.clone();

        info!("Starting dialog turn: dialog_turn_id={}", dialog_turn_id);

        // Execute actual logic
        let result = self
            .execute_dialog_turn_impl(agent_type, initial_messages, context, start_time, initial_count)
            .await;

        // Cleanup cancellation token
        self.round_executor.cleanup_dialog_turn(&dialog_turn_id).await;
        debug!(
            "Cleaned up cancel token (final cleanup): dialog_turn_id={}",
            dialog_turn_id
        );

        result
    }

    pub async fn cancel_dialog_turn(&self, dialog_turn_id: &str) -> NortHingResult<()> {
        self.cancel_dialog_turn_impl(dialog_turn_id).await
    }

    pub fn has_active_turn(&self, dialog_turn_id: &str) -> bool {
        self.has_active_turn_impl(dialog_turn_id)
    }

    pub fn register_cancel_token(&self, dialog_turn_id: &str, token: CancellationToken) {
        self.register_cancel_token_impl(dialog_turn_id, token)
    }

    pub fn cancel_token_for_dialog_turn(&self, dialog_turn_id: &str) -> Option<CancellationToken> {
        self.cancel_token_for_dialog_turn_impl(dialog_turn_id)
    }

    pub async fn cleanup_cancel_token(&self, dialog_turn_id: &str) {
        self.cleanup_cancel_token_impl(dialog_turn_id).await
    }

    pub async fn init_turn(
        &self,
        agent_type: String,
        initial_messages: Vec<Message>,
        context: &ExecutionContext,
    ) -> NortHingResult<ExecutionTurnState> {
        self.init_turn_impl(agent_type, initial_messages, context).await
    }

    pub async fn tick(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> NortHingResult<RoundTickResult> {
        self.tick_impl(context, state).await
    }

    pub async fn finalize_turn(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> NortHingResult<Option<(Message, Option<crate::util::types::ai::GeminiUsage>)>> {
        self.finalize_turn_impl(context, state).await
    }

    pub fn build_result(
        &self,
        state: &ExecutionTurnState,
        start_time: std::time::Instant,
        initial_count: usize,
    ) -> ExecutionResult {
        self.build_result_impl(state, start_time, initial_count)
    }

    pub async fn compress_messages(
        &self,
        session_id: &str,
        dialog_turn_id: &str,
        runtime_messages: Vec<Message>,
        current_tokens: usize,
        context_window: usize,
        ai_client: Arc<crate::infrastructure::ai::AIClient>,
        tool_definitions: &Option<Vec<ToolDefinition>>,
        system_prompt_message: Message,
        prepended_prompt_reminders: &PrependedPromptReminders,
        primary_supports_image_understanding: bool,
        compression_contract_limit: usize,
        workspace: Option<&WorkspaceBinding>,
    ) -> NortHingResult<Option<(usize, Vec<Message>)>> {
        self.compress_messages_impl(
            session_id,
            dialog_turn_id,
            runtime_messages,
            current_tokens,
            context_window,
            ai_client,
            tool_definitions,
            system_prompt_message,
            prepended_prompt_reminders,
            primary_supports_image_understanding,
            compression_contract_limit,
            workspace,
        )
        .await
    }

    pub async fn compact_session_context(
        &self,
        session_id: String,
        dialog_turn_id: String,
        context: ExecutionContext,
        messages: Vec<Message>,
        current_tokens: usize,
        trigger: &str,
    ) -> NortHingResult<ContextCompactionOutcome> {
        self.compact_session_context_impl(session_id, dialog_turn_id, context, messages, current_tokens, trigger)
            .await
    }

    pub async fn resolve_model_id_for_turn(
        &self,
        session: &Session,
        agent_type: &str,
        workspace: Option<&WorkspaceBinding>,
        original_user_input: &str,
        turn_index: usize,
    ) -> NortHingResult<String> {
        self.resolve_model_id_for_turn_impl(session, agent_type, workspace, original_user_input, turn_index)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::super::health_snapshot::ContextHealthSnapshot;
    use super::ExecutionEngine;
    use crate::agentic::core::{Message, MessageRole, ToolCall, ToolResult};
    use crate::service::config::types::AIConfig;
    use crate::service::config::types::AIModelConfig;
    use crate::util::types::ToolDefinition;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn build_model(id: &str, name: &str, model_name: &str) -> AIModelConfig {
        AIModelConfig {
            id: id.to_string(),
            name: name.to_string(),
            model_name: model_name.to_string(),
            provider: "anthropic".to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn resolve_configured_fast_model_falls_back_to_primary_when_fast_is_stale() {
        let mut ai_config = AIConfig::default();
        ai_config.models = vec![build_model("model-primary", "Primary", "claude-sonnet-4.5")];
        ai_config.default_models.primary = Some("model-primary".to_string());
        ai_config.default_models.fast = Some("deleted-fast-model".to_string());

        assert_eq!(
            ExecutionEngine::resolve_configured_model_id(&ai_config, "fast"),
            "model-primary"
        );
    }

    #[test]
    fn auto_compression_pressure_excludes_system_and_tool_overhead() {
        let messages = vec![
            Message::system("system prompt".repeat(10_000)),
            Message::user("hello".to_string()),
        ];
        let tools = vec![ToolDefinition {
            name: "Read".to_string(),
            description: "Read files".repeat(5_000),
            parameters: json!({"type": "object"}),
        }];

        let (total_tokens, conversation_tokens, usage_ratio) =
            ExecutionEngine::estimate_auto_compression_pressure(&messages, Some(&tools), 128_000);

        assert!(total_tokens > conversation_tokens);
        assert!(usage_ratio < total_tokens as f32 / 128_000_f32);
        assert_eq!(messages[1].role, MessageRole::User);
    }

    #[test]
    fn tool_signature_args_summary_truncates_on_utf8_boundary() {
        let args = format!("{}{}", "a".repeat(62), "案".repeat(30));
        let args_hash = hex::encode(Sha256::digest(args.as_bytes()));

        let summary = ExecutionEngine::tool_signature_args_summary(&args);

        assert_eq!(
            summary,
            format!("{}..#{}:sha256={}", "a".repeat(62), args.len(), args_hash)
        );
    }

    #[test]
    fn tool_signature_args_summary_keeps_short_arguments() {
        let args = r#"{"content":"short"}"#;

        let summary = ExecutionEngine::tool_signature_args_summary(args);

        assert_eq!(summary, args);
    }

    #[test]
    fn partial_continuation_allowed_for_stream_stall_reasons() {
        assert!(ExecutionEngine::should_continue_after_partial_response(
            "Stream processor watchdog timeout (no data received for 45 seconds)"
        ));
        assert!(ExecutionEngine::should_continue_after_partial_response(
            "Stream processing error: SSE stream error"
        ));
    }

    #[test]
    fn partial_continuation_skipped_for_user_cancellation() {
        assert!(!ExecutionEngine::should_continue_after_partial_response(
            "Stream processing cancelled after partial output"
        ));
        assert!(!ExecutionEngine::should_continue_after_partial_response(
            "Stream processing cancelled"
        ));
    }

    #[test]
    fn tool_signature_args_summary_distinguishes_same_prefix_and_length() {
        let first = format!("{}{}", "x".repeat(64), "a".repeat(80));
        let second = format!("{}{}", "x".repeat(64), "b".repeat(80));

        let first_summary = ExecutionEngine::tool_signature_args_summary(&first);
        let second_summary = ExecutionEngine::tool_signature_args_summary(&second);

        assert_eq!(first.len(), second.len());
        assert_ne!(first, second);
        assert_ne!(first_summary, second_summary);
    }

    #[test]
    fn failed_tool_round_signature_ignores_successful_repeated_calls() {
        let tool_calls = vec![ToolCall {
            tool_id: "tool-1".to_string(),
            tool_name: "PollStatus".to_string(),
            arguments: json!({ "job_id": "job-1" }),
            raw_arguments: None,
            is_error: false,
            recovered_from_truncation: false,
        }];
        let results = vec![Message::tool_result(ToolResult {
            tool_id: "tool-1".to_string(),
            tool_name: "PollStatus".to_string(),
            result: json!({ "status": "pending", "success": true }),
            result_for_assistant: Some("The job is still pending.".to_string()),
            is_error: false,
            duration_ms: Some(1),
            image_attachments: None,
        })];

        assert!(
            ExecutionEngine::failed_tool_round_signature(&tool_calls, &results).is_none(),
            "successful polling must not be treated as a failed loop"
        );
    }

    #[test]
    fn failed_tool_round_signature_requires_actual_failure_evidence() {
        let tool_calls = vec![ToolCall {
            tool_id: "tool-1".to_string(),
            tool_name: "Read".to_string(),
            arguments: json!({ "path": "missing.txt" }),
            raw_arguments: None,
            is_error: false,
            recovered_from_truncation: false,
        }];
        let results = vec![Message::tool_result(ToolResult {
            tool_id: "tool-1".to_string(),
            tool_name: "Read".to_string(),
            result: json!({ "success": false, "error": "not found" }),
            result_for_assistant: Some("File not found.".to_string()),
            is_error: true,
            duration_ms: Some(1),
            image_attachments: None,
        })];

        assert_eq!(
            ExecutionEngine::failed_tool_round_signature(&tool_calls, &results).as_deref(),
            Some(r#"Read:{"path":"missing.txt"}"#)
        );
    }

    #[test]
    fn periodic_loop_detector_ignores_short_windows() {
        let signatures: Vec<String> = vec!["A".to_string(), "B".to_string(), "A".to_string()];
        assert!(!ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_catches_consecutive_identical_window() {
        let signatures: Vec<String> = std::iter::repeat_n("A".to_string(), 6).collect();
        assert!(ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_catches_alternating_pattern() {
        // A-B-A-B-A-B is a stable period-2 loop with 3 distinct rounds per
        // signature. The strict consecutive check cannot see this because no
        // two adjacent rounds share the same signature.
        let signatures: Vec<String> = ["A", "B", "A", "B", "A", "B"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert!(ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_catches_three_signature_cycle() {
        // A-B-C-A-B-C: window size 6, three distinct signatures, each twice.
        let signatures: Vec<String> = ["A", "B", "C", "A", "B", "C"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert!(ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_skips_genuine_progress() {
        // Six distinct signatures means each tool call is a new exploration
        // step - not a loop, even if the same tool name keeps appearing.
        let signatures: Vec<String> = ["A", "B", "C", "D", "E", "F"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert!(!ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_skips_when_a_signature_appears_only_once() {
        // A-B-A-B-A-C: trailing window has 3 distinct signatures, but C
        // appeared exactly once - the model is still introducing new work.
        let signatures: Vec<String> = ["A", "B", "A", "B", "A", "C"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert!(!ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_only_inspects_trailing_window() {
        // The first 4 rounds were genuine exploration, but the last 6 are a
        // stable A-B alternation. We should still flag the loop.
        let signatures: Vec<String> = ["X1", "X2", "X3", "X4", "A", "B", "A", "B", "A", "B"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        assert!(ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 3));
    }

    #[test]
    fn periodic_loop_detector_treats_threshold_zero_like_one() {
        let signatures: Vec<String> = ["A", "A"].iter().map(|s| (*s).to_string()).collect();
        // A two-round window of identical signatures with threshold 0 should
        // still register as a loop (threshold is clamped to 1, window = 2).
        assert!(ExecutionEngine::is_periodic_tool_signature_loop(&signatures, 0));
    }

    #[test]
    fn context_health_snapshot_scores_repeated_tool_signatures() {
        let signatures = vec![
            r#"Bash:{"command":"cargo test"}"#.to_string(),
            r#"Bash:{"command":"cargo test"}"#.to_string(),
            r#"Bash:{"command":"cargo test"}"#.to_string(),
        ];

        let snapshot = ContextHealthSnapshot::from_runtime_observations(0.82, 1, 0, &signatures, &[]);

        assert!((snapshot.token_usage_ratio - 0.82).abs() < f32::EPSILON);
        assert_eq!(snapshot.full_compression_count, 1);
        assert_eq!(snapshot.compression_failure_count, 0);
        assert_eq!(snapshot.repeated_tool_signature_count, 3);
        assert_eq!(snapshot.consecutive_failed_commands, 0);
    }

    #[test]
    fn context_health_snapshot_counts_consecutive_failed_commands() {
        let messages = vec![
            command_result("Bash", true, Some(0)),
            command_result("Bash", false, Some(1)),
            command_result("Git", false, Some(128)),
        ];

        let snapshot = ContextHealthSnapshot::from_runtime_observations(0.44, 0, 2, &[], &messages);

        assert_eq!(snapshot.repeated_tool_signature_count, 0);
        assert_eq!(snapshot.consecutive_failed_commands, 2);
        assert_eq!(snapshot.compression_failure_count, 2);
    }

    fn command_result(tool_name: &str, success: bool, exit_code: Option<i32>) -> Message {
        Message::tool_result(ToolResult {
            tool_id: format!("{}-tool", tool_name),
            tool_name: tool_name.to_string(),
            result: json!({
                "success": success,
                "exit_code": exit_code,
                "command": format!("{} command", tool_name),
            }),
            result_for_assistant: None,
            is_error: !success,
            duration_ms: Some(1),
            image_attachments: None,
        })
    }

    // ═══════════════════════════════════════════════════════════════════
    // A2: tick API tests
    // ═══════════════════════════════════════════════════════════════════

    /// Verify that `RoundTickResult` variants are what we expect.
    #[test]
    fn round_tick_result_variants_match_semantics() {
        use crate::agentic::execution::RoundTickResult;

        let continue_result = RoundTickResult::Continue;
        let done_result = RoundTickResult::Done;
        let cancelled_result = RoundTickResult::Cancelled;
        let error_result = RoundTickResult::Error {
            error: "test".to_string(),
        };

        // Just verify they construct without panic
        match continue_result {
            RoundTickResult::Continue => {}
            _ => panic!("expected Continue"),
        }
        match done_result {
            RoundTickResult::Done => {}
            _ => panic!("expected Done"),
        }
        match cancelled_result {
            RoundTickResult::Cancelled => {}
            _ => panic!("expected Cancelled"),
        }
        match error_result {
            RoundTickResult::Error { error } => assert_eq!(error, "test"),
            _ => panic!("expected Error"),
        }
    }
}
