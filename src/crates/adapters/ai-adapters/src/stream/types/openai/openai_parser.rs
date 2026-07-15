use super::openai_types::{Choice, Delta, OpenAISSEData};
use crate::stream::types::openai::OpenAIToolCallArgumentsNormalizer;
use crate::stream::types::unified::{UnifiedResponse, UnifiedToolCall};

impl OpenAISSEData {
    pub fn normalize_tool_call_arguments(&mut self, normalizer: &mut OpenAIToolCallArgumentsNormalizer) {
        if let Some(first_choice) = self.choices.first_mut() {
            normalizer.normalize_choice(first_choice);
        }
    }

    pub fn is_choices_empty(&self) -> bool {
        self.choices.is_empty()
    }

    pub fn first_choice_tool_call_count(&self) -> usize {
        self.choices
            .first()
            .and_then(|choice| choice.delta.tool_calls.as_ref())
            .map(|tool_calls| tool_calls.len())
            .unwrap_or(0)
    }

    pub fn into_unified_responses(self) -> Vec<UnifiedResponse> {
        let mut usage = self.usage.map(|usage| usage.into());

        let Some(first_choice) = self.choices.into_iter().next() else {
            // OpenAI can emit `choices: []` for the final usage chunk.
            return usage
                .map(|usage_data| {
                    vec![UnifiedResponse {
                        usage: Some(usage_data),
                        ..Default::default()
                    }]
                })
                .unwrap_or_default();
        };

        let Choice {
            delta, finish_reason, ..
        } = first_choice;
        let Delta {
            reasoning_content,
            reasoning_details,
            content,
            tool_calls,
            ..
        } = delta;

        // Treat empty strings the same as absent fields for assistant text (MiniMax sends
        // `content: ""` in reasoning-only chunks). Keep empty reasoning content so downstream
        // can replay structurally present thinking blocks when a provider requires it.
        let content = content.filter(|s| !s.is_empty());

        // MiniMax uses `reasoning_details` instead of `reasoning_content`.
        // Collect all "reasoning.text" entries and join them as a fallback.
        let reasoning_content = reasoning_content.or_else(|| {
            reasoning_details.and_then(|details| {
                let text: String = details
                    .into_iter()
                    .filter(|d| d.detail_type.as_deref() == Some("reasoning.text"))
                    .filter_map(|d| d.text)
                    .collect();
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            })
        });

        let mut responses = Vec::new();

        if content.is_some() || reasoning_content.is_some() {
            responses.push(UnifiedResponse {
                text: content,
                reasoning_content,
                thinking_signature: None,
                tool_call: None,
                usage: usage.take(),
                finish_reason: None,
                provider_metadata: None,
            });
        }

        if let Some(tool_calls) = tool_calls {
            for tool_call in tool_calls {
                let is_first_event = responses.is_empty();
                responses.push(UnifiedResponse {
                    text: None,
                    reasoning_content: None,
                    thinking_signature: None,
                    tool_call: Some(UnifiedToolCall::from(tool_call)),
                    usage: if is_first_event { usage.take() } else { None },
                    finish_reason: None,
                    provider_metadata: None,
                });
            }
        }

        if let Some(finish_reason) = finish_reason {
            if let Some(last_response) = responses.last_mut() {
                last_response.finish_reason = Some(finish_reason);
                return responses;
            }

            responses.push(UnifiedResponse {
                text: None,
                reasoning_content: None,
                thinking_signature: None,
                tool_call: None,
                usage,
                finish_reason: Some(finish_reason),
                provider_metadata: None,
            });
            return responses;
        }

        if responses.is_empty() {
            responses.push(UnifiedResponse {
                text: None,
                reasoning_content: None,
                thinking_signature: None,
                tool_call: None,
                usage,
                finish_reason,
                provider_metadata: None,
            });
        }

        responses
    }
}

impl From<OpenAISSEData> for UnifiedResponse {
    fn from(data: OpenAISSEData) -> Self {
        data.into_unified_responses().into_iter().next().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multiple_tool_calls_in_first_choice() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "created": 123,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [
                        {
                            "index": 0,
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "tool_a",
                                "arguments": "{\"a\":1}"
                            }
                        },
                        {
                            "index": 1,
                            "id": "call_2",
                            "type": "function",
                            "function": {
                                "name": "tool_b",
                                "arguments": "{\"b\":2}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
                "prompt_tokens_details": {
                    "cached_tokens": 3
                }
            }
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("valid openai sse data");
        let responses = sse_data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0].tool_call.as_ref().and_then(|tool| tool.tool_call_index),
            Some(0)
        );
        assert_eq!(
            responses[1].tool_call.as_ref().and_then(|tool| tool.tool_call_index),
            Some(1)
        );
        assert_eq!(
            responses[0].tool_call.as_ref().and_then(|tool| tool.id.as_deref()),
            Some("call_1")
        );
        assert_eq!(
            responses[1].tool_call.as_ref().and_then(|tool| tool.id.as_deref()),
            Some("call_2")
        );
        assert!(responses[0].finish_reason.is_none());
        assert_eq!(responses[1].finish_reason.as_deref(), Some("tool_calls"));
        assert!(responses[0].usage.is_some());
        assert!(responses[1].usage.is_none());
    }

    #[test]
    fn preserves_empty_reasoning_content_chunk() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "created": 123,
            "model": "deepseek-test",
            "choices": [{
                "index": 0,
                "delta": {
                    "reasoning_content": ""
                },
                "finish_reason": "stop"
            }]
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("valid openai sse data");
        let responses = sse_data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].reasoning_content.as_deref(), Some(""));
        assert_eq!(responses[0].finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn handles_missing_created_in_openai_compatible_chunk() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "object": "chat.completion.chunk",
            "model": "compatible-model",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "hello"
                },
                "finish_reason": null
            }],
            "usage": {
                "prompt_tokens": 2,
                "completion_tokens": 1,
                "total_tokens": 3
            }
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("compatible openai sse data");
        let responses = sse_data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].text.as_deref(), Some("hello"));
        assert_eq!(
            responses[0].usage.as_ref().map(|usage| usage.total_token_count),
            Some(3)
        );
    }

    #[test]
    fn handles_empty_choices_with_usage_chunk() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "created": 123,
            "model": "gpt-test",
            "choices": [],
            "usage": {
                "prompt_tokens": 7,
                "completion_tokens": 3,
                "total_tokens": 10
            }
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("valid openai sse data");
        let responses = sse_data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert!(responses[0].usage.is_some());
        assert!(responses[0].text.is_none());
        assert!(responses[0].tool_call.is_none());
    }

    #[test]
    fn parses_minimax_final_chunk_with_message_field_instead_of_delta() {
        // MiniMax's last SSE frame uses non-streaming `chat.completion` shape:
        // choice has `message` instead of `delta`, and the real usage lives at
        // the top level. Pre-fix this chunk failed to deserialize (`delta` was
        // a required field), so the real prompt/completion tokens were silently
        // dropped. After the fix, the chunk parses cleanly and usage flows
        // through.
        let raw = r#"{
            "id": "065b58b7a16cf30f1e20c8f1942efeae",
            "created": 1779180983,
            "model": "MiniMax-M2.7-highspeed",
            "object": "chat.completion",
            "choices": [{
                "finish_reason": "stop",
                "index": 0,
                "message": {
                    "content": "hi",
                    "role": "assistant",
                    "name": "MiniMax AI",
                    "reasoning_content": "The user wants hi."
                }
            }],
            "usage": {
                "total_tokens": 92,
                "prompt_tokens": 45,
                "completion_tokens": 47,
                "completion_tokens_details": {"reasoning_tokens": 45}
            }
        }"#;

        let sse_data: OpenAISSEData =
            serde_json::from_str(raw).expect("MiniMax final chunk must deserialize even without delta");
        let responses = sse_data.into_unified_responses();

        // Critical: the usage from this chunk must propagate.
        let usage = responses
            .iter()
            .find_map(|r| r.usage.as_ref())
            .expect("usage from MiniMax final chunk must be preserved");
        assert_eq!(usage.prompt_token_count, 45);
        assert_eq!(usage.candidates_token_count, 47);
        assert_eq!(usage.total_token_count, 92);

        // finish_reason should also be preserved (lives at choice top level).
        assert!(
            responses.iter().any(|r| r.finish_reason.as_deref() == Some("stop")),
            "finish_reason from MiniMax final chunk must be preserved"
        );
    }

    #[test]
    fn handles_empty_choices_without_usage_chunk() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "created": 123,
            "model": "gpt-test",
            "choices": [],
            "usage": null
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("valid openai sse data");
        let responses = sse_data.into_unified_responses();

        assert!(responses.is_empty());
    }

    #[test]
    fn preserves_text_when_tool_calls_exist_in_same_chunk() {
        let raw = r#"{
            "id": "chatcmpl_test",
            "created": 123,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "hello",
                    "tool_calls": [
                        {
                            "index": 0,
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "tool_a",
                                "arguments": "{\"a\":1}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let sse_data: OpenAISSEData = serde_json::from_str(raw).expect("valid openai sse data");
        let responses = sse_data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].text.as_deref(), Some("hello"));
        assert!(responses[0].tool_call.is_none());
        assert!(responses[0].usage.is_some());
        assert!(responses[0].finish_reason.is_none());

        assert!(responses[1].text.is_none());
        assert_eq!(
            responses[1].tool_call.as_ref().and_then(|tool| tool.id.as_deref()),
            Some("call_1")
        );
        assert!(responses[1].usage.is_none());
        assert_eq!(responses[1].finish_reason.as_deref(), Some("tool_calls"));
    }
}
