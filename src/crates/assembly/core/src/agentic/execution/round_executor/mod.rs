//! Round Executor
//!
//! Executes a single model round: delegates body to 4 sub-handlers in
//! `round_subhandlers/` (Round 8b split, Round 45 facade + 4-sibling
//! sub-domain decomposition).
//!
//! Round 47c sub-domain split: this file holds the facade (struct decl +
//! constructor + accessor + `execute_round` entry point + `has_user_visible_assistant_text`
//! helper + tests). The remaining impl-block methods are split across
//! sibling files by domain:
//!
//! - `rexec_state`: cancellation-token bookkeeping (`register_cancel_token`,
//!   `has_active_dialog_turn`, `cancel_dialog_turn`, `cleanup_dialog_turn`,
//!   `sleep_with_cancellation`, `MAX_STREAM_ATTEMPTS`, `emit_event`).
//! - `rexec_validate`: stream-result validation + transient-error
//!   classification (`is_transient_network_error`, `retry_delay_ms`,
//!   `has_interrupted_invalid_tool_calls`, `is_invalid_tool_only_without_text`).
//! - `rexec_run`: trace-response builders + token-usage / failed-partial
//!   event emission (`emit_token_usage_update`, `emit_failed_partial_tool_calls`,
//!   `complete_model_exchange_trace`, `final_trace_response`, `trace_response`,
//!   and friends).
//! - `rexec_types`: free `token_details_from_usage` helper for token-stats
//!   JSON serialization.

use super::stream_processor::StreamProcessor;
use crate::agentic::events::EventQueue;
use crate::agentic::tools::computer_use_host::ComputerUseHostRef;
use crate::agentic::tools::pipeline::ToolPipeline;
use crate::infrastructure::ai::AIClient;
use crate::util::errors::NortHingResult;
use crate::util::types::Message as AIMessage;
use crate::util::types::ToolDefinition;
use dashmap::DashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

mod rexec_run;
mod rexec_state;
pub(in crate::agentic::execution) mod rexec_types;
mod rexec_validate;

/// Round executor
pub struct RoundExecutor {
    pub(super) stream_processor: Arc<StreamProcessor>,
    pub(super) tool_pipeline: Option<Arc<ToolPipeline>>,
    pub(super) event_queue: Arc<EventQueue>,
    /// Cancellation tokens: use dialog_turn_id as key
    pub(super) cancellation_tokens: Arc<DashMap<String, CancellationToken>>,
}

impl RoundExecutor {
    pub(super) fn has_user_visible_assistant_text(text: &str) -> bool {
        !text.trim().is_empty()
    }

    pub fn new(
        stream_processor: Arc<StreamProcessor>,
        event_queue: Arc<EventQueue>,
        tool_pipeline: Arc<ToolPipeline>,
    ) -> Self {
        Self {
            stream_processor,
            tool_pipeline: Some(tool_pipeline),
            event_queue,
            cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

    pub fn computer_use_host(&self) -> Option<ComputerUseHostRef> {
        self.tool_pipeline.as_ref().and_then(|p| p.computer_use_host())
    }

    /// Execute a single model round.
    ///
    /// Delegates to 4 sub-handlers in `round_subhandlers/`:
    /// `prepare_stream` -> `dispatch_stream` -> `process_result` -> `handle_error`.
    pub async fn execute_round(
        &self,
        ai_client: std::sync::Arc<AIClient>,
        context: super::types::RoundContext,
        ai_messages: Vec<AIMessage>,
        tool_definitions: Option<Vec<ToolDefinition>>,
        context_window: Option<usize>,
    ) -> NortHingResult<super::types::RoundResult> {
        let mut state = super::round_subhandlers::RoundState::new(
            ai_client,
            context,
            ai_messages,
            tool_definitions,
            context_window,
        );
        self.prepare_stream(&mut state).await?;
        let outcome = self.dispatch_stream(&mut state).await?;
        let result = self.process_result(&mut state, outcome).await?;
        self.handle_error(&mut state).await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::stream_processor::{StreamProcessor, StreamResult};
    use super::RoundExecutor;
    use crate::agentic::core::ToolCall;
    use crate::agentic::events::{AgenticEvent, EventQueue, EventQueueConfig};
    use crate::agentic::execution::types::RoundContext;
    use crate::agentic::tools::ToolRuntimeRestrictions;
    use crate::util::errors::NortHingError;
    use crate::util::types::ai::GeminiUsage;
    use dashmap::DashMap;
    use northhing_runtime_ports::DelegationPolicy;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    fn test_round_executor() -> RoundExecutor {
        let event_queue = Arc::new(EventQueue::new(EventQueueConfig::default()));
        RoundExecutor {
            stream_processor: Arc::new(StreamProcessor::new(event_queue.clone())),
            tool_pipeline: None,
            event_queue,
            cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

    fn test_round_context() -> RoundContext {
        RoundContext {
            session_id: "session-1".to_string(),
            subagent_parent_info: None,
            dialog_turn_id: "turn-1".to_string(),
            turn_index: 0,
            round_number: 0,
            workspace: None,
            messages: Vec::new(),
            available_tools: Vec::new(),
            collapsed_tools: Vec::new(),
            unlocked_collapsed_tools: Vec::new(),
            model_name: "model-1".to_string(),
            agent_type: "agentic".to_string(),
            context_vars: HashMap::new(),
            delegation_policy: DelegationPolicy::top_level(),
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            steering_interrupt: None,
            workspace_services: None,
            recover_partial_on_cancel: false,
        }
    }

    #[tokio::test]
    async fn cancel_token_for_dialog_turn_returns_registered_token() {
        let executor = test_round_executor();
        let token = CancellationToken::new();
        executor.register_cancel_token("turn-1", token.clone());

        assert!(executor.cancel_token_for_dialog_turn("turn-1").is_some());
        assert!(executor.cancel_token_for_dialog_turn("missing").is_none());
    }

    #[tokio::test]
    async fn cancel_keeps_token_registered_until_cleanup() {
        let executor = test_round_executor();
        let token = CancellationToken::new();
        executor.register_cancel_token("turn-1", token.clone());

        executor
            .cancel_dialog_turn("turn-1")
            .await
            .expect("cancel should succeed");

        assert!(token.is_cancelled());
        assert!(executor.has_active_dialog_turn("turn-1"));
        assert!(executor.is_dialog_turn_cancelled("turn-1"));

        executor.cleanup_dialog_turn("turn-1").await;
        assert!(!executor.has_active_dialog_turn("turn-1"));
        assert!(!executor.is_dialog_turn_cancelled("turn-1"));
    }

    #[tokio::test]
    async fn emits_token_usage_before_post_stream_cancel_stops_round() {
        let executor = test_round_executor();
        let context = test_round_context();
        let usage = GeminiUsage {
            prompt_token_count: 100,
            candidates_token_count: 20,
            total_token_count: 120,
            reasoning_token_count: None,
            cached_content_token_count: Some(30),
            cache_creation_token_count: None,
        };

        executor
            .emit_token_usage_update(&context, &usage, Some(128_000), false)
            .await;

        let events = executor.event_queue.dequeue_batch(10).await;
        assert!(events.iter().any(|envelope| matches!(
            &envelope.event,
            AgenticEvent::TokenUsageUpdated {
                session_id,
                turn_id,
                model_id,
                input_tokens: 100,
                output_tokens: Some(20),
                total_tokens: 120,
                max_context_tokens: Some(128_000),
                is_subagent: false,
                cached_tokens: Some(30),
                ..
            } if session_id == "session-1" && turn_id == "turn-1" && model_id == "model-1"
        )));
    }

    #[tokio::test]
    async fn cancellable_sleep_returns_cancelled_when_token_fires() {
        let token = CancellationToken::new();
        let token_for_task = token.clone();

        let waiter = tokio::spawn(async move { RoundExecutor::sleep_with_cancellation(5_000, &token_for_task).await });

        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();

        let result = waiter.await.expect("sleep task should join");
        assert!(matches!(result, Err(NortHingError::Cancelled(_))));
    }

    #[tokio::test]
    async fn cancellable_sleep_completes_normally_without_cancel() {
        let token = CancellationToken::new();

        let result = RoundExecutor::sleep_with_cancellation(10, &token).await;

        assert!(result.is_ok());
    }

    #[test]
    fn token_details_emits_both_cache_keys_when_present() {
        use crate::util::types::ai::GeminiUsage;
        let usage = GeminiUsage {
            prompt_token_count: 100,
            candidates_token_count: 20,
            total_token_count: 120,
            reasoning_token_count: None,
            cached_content_token_count: Some(30),
            cache_creation_token_count: Some(20),
        };
        let details = super::rexec_types::token_details_from_usage(&usage).expect("details");
        assert_eq!(
            details.get("cachedContentTokenCount").and_then(|v| v.as_u64()),
            Some(30)
        );
        assert_eq!(
            details.get("cacheCreationTokenCount").and_then(|v| v.as_u64()),
            Some(20)
        );
    }

    #[test]
    fn token_details_emits_only_read_when_creation_absent() {
        use crate::util::types::ai::GeminiUsage;
        let usage = GeminiUsage {
            prompt_token_count: 100,
            candidates_token_count: 20,
            total_token_count: 120,
            reasoning_token_count: None,
            cached_content_token_count: Some(30),
            cache_creation_token_count: None,
        };
        let details = super::rexec_types::token_details_from_usage(&usage).expect("details");
        assert_eq!(
            details.get("cachedContentTokenCount").and_then(|v| v.as_u64()),
            Some(30)
        );
        assert!(details.get("cacheCreationTokenCount").is_none());
    }

    #[test]
    fn token_details_is_none_when_no_cache_info() {
        use crate::util::types::ai::GeminiUsage;
        let usage = GeminiUsage {
            prompt_token_count: 100,
            candidates_token_count: 20,
            total_token_count: 120,
            reasoning_token_count: None,
            cached_content_token_count: None,
            cache_creation_token_count: None,
        };
        assert!(super::rexec_types::token_details_from_usage(&usage).is_none());
    }

    #[test]
    fn error_trace_response_from_stream_result_preserves_structured_context() {
        let stream_result = StreamResult {
            full_thinking: "reasoning".to_string(),
            reasoning_content_present: true,
            thinking_signature: Some("sig".to_string()),
            full_text: String::new(),
            tool_calls: vec![ToolCall {
                tool_id: "tool-1".to_string(),
                tool_name: "Bash".to_string(),
                arguments: json!({}),
                raw_arguments: Some("{\"command\":".to_string()),
                is_error: true,
                recovered_from_truncation: false,
            }],
            usage: Some(GeminiUsage {
                prompt_token_count: 100,
                candidates_token_count: 20,
                total_token_count: 120,
                reasoning_token_count: Some(5),
                cached_content_token_count: Some(30),
                cache_creation_token_count: None,
            }),
            provider_metadata: Some(json!({ "finish_reason": "tool_calls" })),
            has_effective_output: false,
            first_chunk_ms: Some(10),
            first_visible_output_ms: None,
            partial_recovery_reason: Some("tool arguments invalid".to_string()),
        };

        let trace = RoundExecutor::error_trace_response_from_stream_result(
            "error",
            "Provider returned only invalid tool arguments".to_string(),
            &stream_result,
        );

        assert_eq!(trace.kind, "error");
        assert_eq!(
            trace.error.as_deref(),
            Some("Provider returned only invalid tool arguments")
        );
        assert_eq!(trace.assistant_text.as_deref(), Some(""));
        assert_eq!(trace.thinking.as_deref(), Some("reasoning"));
        assert_eq!(trace.partial_recovery_reason.as_deref(), Some("tool arguments invalid"));
        assert_eq!(trace.provider_metadata, Some(json!({ "finish_reason": "tool_calls" })));
        assert_eq!(
            trace.usage,
            Some(json!({
                "promptTokenCount": 100,
                "candidatesTokenCount": 20,
                "totalTokenCount": 120,
                "reasoningTokenCount": 5,
                "cachedContentTokenCount": 30
            }))
        );
        assert_eq!(
            trace.tool_calls,
            Some(json!([{
                "tool_id": "tool-1",
                "tool_name": "Bash",
                "arguments": {},
                "raw_arguments": "{\"command\":",
                "is_error": true
            }]))
        );
    }

    #[test]
    fn error_trace_response_without_stream_result_stays_empty() {
        let trace = RoundExecutor::error_trace_response("error", "request failed".to_string());

        assert_eq!(trace.kind, "error");
        assert!(trace.assistant_text.is_none());
        assert!(trace.thinking.is_none());
        assert!(trace.tool_calls.is_none());
        assert!(trace.usage.is_none());
        assert!(trace.provider_metadata.is_none());
        assert!(trace.partial_recovery_reason.is_none());
        assert_eq!(trace.error.as_deref(), Some("request failed"));
    }

    #[test]
    fn is_transient_error_treats_rate_limit_as_transient() {
        assert!(RoundExecutor::is_transient_network_error(
            "OpenAI Streaming API error 429 Too Many Requests"
        ));
        assert!(RoundExecutor::is_transient_network_error("rate limit exceeded"));
    }

    #[test]
    fn is_transient_error_treats_network_errors_as_transient() {
        assert!(RoundExecutor::is_transient_network_error("connection reset by peer"));
        assert!(RoundExecutor::is_transient_network_error("timeout"));
    }

    #[test]
    fn is_transient_error_treats_context_overflow_as_non_transient() {
        assert!(!RoundExecutor::is_transient_network_error("prompt is too long"));
    }

    #[test]
    fn is_transient_error_treats_budget_exhausted_as_non_transient() {
        // After SSE layer exhausts its retry budget, the round executor must
        // NOT re-enter another round of attempts (would cause 10×10 = 100
        // retries).
        assert!(!RoundExecutor::is_transient_network_error(
            "OpenAI Streaming API failed after 10 attempts: \
             OpenAI Streaming API error 429 Too Many Requests"
        ));
        assert!(!RoundExecutor::is_transient_network_error(
            "Stream retry budget exhausted after 10 attempts: timeout"
        ));
    }

    #[test]
    fn is_transient_error_does_not_misclassify_failed_after_without_attempts() {
        // "failed after " without "attempts:" should NOT be treated as budget
        // exhausted — it may be a legitimately retryable transient error.
        assert!(RoundExecutor::is_transient_network_error(
            "stream failed after connection reset"
        ));
        assert!(RoundExecutor::is_transient_network_error(
            "request failed after timeout"
        ));
    }
}
