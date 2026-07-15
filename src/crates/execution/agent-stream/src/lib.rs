//! Stream Processor
//!
//! Processes AI streaming responses, supports tool pre-detection and parameter streaming

pub mod tool_call_accumulator;
mod unified;

mod sse_log_collector;
mod stream_context;
mod stream_processor;
mod tool_call_repair;
mod tool_call_state;
mod tool_call_types;
mod types;

pub use sse_log_collector::SseLogCollector;
pub use stream_processor::StreamProcessor;
pub use types::{
    SseLogConfig, StreamEventSink, StreamProcessError, StreamProcessOptions, StreamProcessorError, StreamResult,
    ToolCall,
};
pub use unified::{UnifiedResponse, UnifiedTokenUsage, UnifiedToolCall};

#[cfg(test)]
mod tests {
    use super::{StreamEventSink, StreamProcessOptions, StreamProcessor};
    use super::{UnifiedResponse, UnifiedTokenUsage, UnifiedToolCall};
    use futures::StreamExt;
    use northhing_events::{AgenticEvent, AgenticEventPriority as EventPriority};
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_stream::iter;
    use tokio_util::sync::CancellationToken;

    struct NoopEventSink;

    #[async_trait::async_trait]
    impl StreamEventSink for NoopEventSink {
        async fn enqueue(&self, _event: AgenticEvent, _priority: Option<EventPriority>) {}
    }

    fn build_processor() -> StreamProcessor {
        StreamProcessor::new(Arc::new(NoopEventSink))
    }

    #[test]
    fn derives_watchdog_timeout_from_stream_idle_timeout() {
        assert_eq!(StreamProcessor::derive_watchdog_timeout(None), None);
        assert_eq!(
            StreamProcessor::derive_watchdog_timeout(Some(Duration::from_secs(10))),
            Some(Duration::from_secs(12))
        );
    }

    fn sample_usage(total_tokens: u32) -> UnifiedTokenUsage {
        UnifiedTokenUsage {
            prompt_token_count: 1,
            candidates_token_count: total_tokens.saturating_sub(1),
            total_token_count: total_tokens,
            reasoning_token_count: None,
            cached_content_token_count: None,
            cache_creation_token_count: None,
        }
    }

    #[tokio::test]
    async fn recovers_partial_text_when_cancellation_allows_partial_recovery() {
        let processor = build_processor();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tx.send(Ok(UnifiedResponse {
            text: Some("Partial reviewer evidence.".to_string()),
            ..Default::default()
        }))
        .expect("send partial chunk");
        let _keep_stream_open = tx;
        let cancellation_token = CancellationToken::new();
        let cancel_clone = cancellation_token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancel_clone.cancel();
        });

        let result = processor
            .process_stream_with_options(
                tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed(),
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &cancellation_token,
                StreamProcessOptions {
                    recover_partial_on_cancel: true,
                    ..Default::default()
                },
            )
            .await
            .expect("partial stream result");

        assert_eq!(result.full_text, "Partial reviewer evidence.");
        assert!(result
            .partial_recovery_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("cancelled")));
    }

    #[tokio::test]
    async fn keeps_collecting_tool_args_across_usage_chunks() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: Some("call_1".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: Some("{\"a\":".to_string()),
                    arguments_is_snapshot: false,
                }),
                usage: Some(sample_usage(5)),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: None,
                    name: None,
                    arguments: Some("1}".to_string()),
                    arguments_is_snapshot: false,
                }),
                usage: Some(sample_usage(7)),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_id, "call_1");
        assert_eq!(result.tool_calls[0].tool_name, "tool_a");
        assert_eq!(result.tool_calls[0].arguments, json!({"a": 1}));
        assert_eq!(result.tool_calls[0].raw_arguments.as_deref(), Some("{\"a\":1}"));
        assert!(!result.tool_calls[0].is_error);
        assert_eq!(result.usage.as_ref().map(|u| u.total_token_count), Some(7));
    }

    #[tokio::test]
    async fn marks_token_limit_truncated_text_as_partial_recovery() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                text: Some("{\"slides\": [{\"title\": \"cut off".to_string()),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                finish_reason: Some("length".to_string()),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert!(result
            .partial_recovery_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("output token limit")));
    }

    #[tokio::test]
    async fn natural_stop_finish_reason_is_not_partial_recovery() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                text: Some("complete answer".to_string()),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                finish_reason: Some("stop".to_string()),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert!(result.partial_recovery_reason.is_none());
    }

    #[tokio::test]
    async fn token_limit_with_tool_calls_is_not_partial_recovery() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: Some("call_1".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: Some("{\"a\":1}".to_string()),
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                finish_reason: Some("MAX_TOKENS".to_string()),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        // Tool-call rounds continue through the normal round loop.
        assert!(result.partial_recovery_reason.is_none());
    }

    #[tokio::test]
    async fn whitespace_only_text_is_not_effective_output() {
        let processor = build_processor();
        let stream = iter(vec![Ok(UnifiedResponse {
            text: Some("\n\n ".to_string()),
            ..Default::default()
        })])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.full_text, "\n\n ");
        assert!(!result.has_effective_output);
        assert_eq!(result.first_visible_output_ms, None);
    }

    #[tokio::test]
    async fn finalizes_tool_after_same_chunk_finish_reason() {
        let processor = build_processor();
        let stream = iter(vec![Ok(UnifiedResponse {
            tool_call: Some(UnifiedToolCall {
                tool_call_index: None,
                id: Some("call_1".to_string()),
                name: Some("tool_a".to_string()),
                arguments: Some("{\"a\":1}".to_string()),
                arguments_is_snapshot: false,
            }),
            usage: Some(sample_usage(9)),
            finish_reason: Some("tool_calls".to_string()),
            ..Default::default()
        })])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].arguments, json!({"a": 1}));
        assert_eq!(result.usage.as_ref().map(|u| u.total_token_count), Some(9));
    }

    #[tokio::test]
    async fn skips_duplicate_finalized_tool_call_id_from_tail_chunks() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: Some("call_1".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: Some("{\"a\":1}".to_string()),
                    arguments_is_snapshot: false,
                }),
                finish_reason: Some("tool_calls".to_string()),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: Some("call_1".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: Some("{\"a\":1}".to_string()),
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_id, "call_1");
        assert_eq!(result.tool_calls[0].arguments, json!({"a": 1}));
    }

    #[tokio::test]
    async fn does_not_repair_tool_args_with_one_extra_trailing_right_brace() {
        let processor = build_processor();
        let stream = iter(vec![Ok(UnifiedResponse {
            tool_call: Some(UnifiedToolCall {
                tool_call_index: None,
                id: Some("call_1".to_string()),
                name: Some("tool_a".to_string()),
                arguments: Some("{\"a\":1}}".to_string()),
                arguments_is_snapshot: false,
            }),
            finish_reason: Some("tool_calls".to_string()),
            ..Default::default()
        })])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_id, "call_1");
        assert_eq!(result.tool_calls[0].tool_name, "tool_a");
        assert_eq!(result.tool_calls[0].arguments, json!({}));
        assert_eq!(result.tool_calls[0].raw_arguments.as_deref(), Some("{\"a\":1}}"));
        assert!(result.tool_calls[0].is_error);
    }

    #[tokio::test]
    async fn replaces_tool_args_when_snapshot_chunk_arrives() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: Some("call_1".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: Some("{\"city\":\"Bei".to_string()),
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: None,
                    id: None,
                    name: None,
                    arguments: Some("{\"city\":\"Beijing\"}".to_string()),
                    arguments_is_snapshot: true,
                }),
                finish_reason: Some("tool_calls".to_string()),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_id, "call_1");
        assert_eq!(result.tool_calls[0].tool_name, "tool_a");
        assert_eq!(result.tool_calls[0].arguments, json!({"city": "Beijing"}));
        assert_eq!(
            result.tool_calls[0].raw_arguments.as_deref(),
            Some("{\"city\":\"Beijing\"}")
        );
        assert!(!result.tool_calls[0].is_error);
    }

    #[tokio::test]
    async fn keeps_interleaved_indexed_tool_calls_separate() {
        let processor = build_processor();
        let stream = iter(vec![
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: Some(0),
                    id: Some("call_0".to_string()),
                    name: Some("tool_a".to_string()),
                    arguments: None,
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: Some(1),
                    id: Some("call_1".to_string()),
                    name: Some("tool_b".to_string()),
                    arguments: None,
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: Some(0),
                    id: None,
                    name: None,
                    arguments: Some("{\"a\":1}".to_string()),
                    arguments_is_snapshot: false,
                }),
                ..Default::default()
            }),
            Ok(UnifiedResponse {
                tool_call: Some(UnifiedToolCall {
                    tool_call_index: Some(1),
                    id: None,
                    name: None,
                    arguments: Some("{\"b\":2}".to_string()),
                    arguments_is_snapshot: false,
                }),
                finish_reason: Some("tool_calls".to_string()),
                ..Default::default()
            }),
        ])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].tool_id, "call_0");
        assert_eq!(result.tool_calls[0].tool_name, "tool_a");
        assert_eq!(result.tool_calls[0].arguments, json!({"a": 1}));
        assert_eq!(result.tool_calls[1].tool_id, "call_1");
        assert_eq!(result.tool_calls[1].tool_name, "tool_b");
        assert_eq!(result.tool_calls[1].arguments, json!({"b": 2}));
    }

    #[tokio::test]
    async fn preserves_empty_reasoning_presence_for_replay() {
        let processor = build_processor();
        let stream = iter(vec![Ok(UnifiedResponse {
            reasoning_content: Some(String::new()),
            finish_reason: Some("stop".to_string()),
            ..Default::default()
        })])
        .boxed();

        let result = processor
            .process_stream(
                stream,
                None,
                None,
                "session_1".to_string(),
                "turn_1".to_string(),
                "round_1".to_string(),
                &CancellationToken::new(),
            )
            .await
            .expect("stream result");

        assert!(result.reasoning_content_present);
        assert!(result.full_thinking.is_empty());
        assert!(!result.has_effective_output);
    }
}
