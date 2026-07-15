use super::openai_types::{Choice, OpenAISSEData, OpenAIToolCall};
use crate::stream::types::openai::OpenAIToolCallArgumentsNormalizer;

impl OpenAIToolCallArgumentsNormalizer {
    pub(super) fn normalize_choice(&mut self, choice: &mut Choice) {
        let has_stop_reason = choice.stop_reason.is_some();
        let Some(tool_calls) = choice.delta.tool_calls.as_mut() else {
            return;
        };

        for tool_call in tool_calls.iter_mut() {
            self.normalize_tool_call(tool_call, has_stop_reason);
        }
    }

    pub(super) fn normalize_tool_call(&mut self, tool_call: &mut OpenAIToolCall, has_stop_reason: bool) {
        let has_id = tool_call.id.as_ref().is_some_and(|value| !value.is_empty());
        let has_name = tool_call
            .function
            .as_ref()
            .and_then(|function| function.name.as_ref())
            .is_some_and(|value| !value.is_empty());

        let Some(function) = tool_call.function.as_mut() else {
            return;
        };
        let Some(arguments) = function.arguments.as_ref() else {
            return;
        };

        if arguments.is_empty() {
            return;
        }

        if has_stop_reason && !has_id && !has_name {
            tool_call.arguments_is_snapshot = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_stop_reason_tool_chunk_as_snapshot() {
        let mut normalizer = OpenAIToolCallArgumentsNormalizer::default();

        let mut first_chunk: OpenAISSEData = serde_json::from_str(
            r#"{
                "id": "chatcmpl_test",
                "created": 123,
                "model": "gpt-test",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "tool_a",
                                "arguments": "{\"city\":\"Bei"
                            }
                        }]
                    },
                    "finish_reason": null
                }]
            }"#,
        )
        .expect("valid first chunk");
        first_chunk.normalize_tool_call_arguments(&mut normalizer);
        let first_responses = first_chunk.into_unified_responses();
        assert_eq!(
            first_responses[0]
                .tool_call
                .as_ref()
                .and_then(|tool| tool.arguments.as_deref()),
            Some("{\"city\":\"Bei")
        );
        assert!(
            !first_responses[0]
                .tool_call
                .as_ref()
                .expect("tool call")
                .arguments_is_snapshot
        );

        let mut snapshot_chunk: OpenAISSEData = serde_json::from_str(
            r#"{
                "id": "chatcmpl_test",
                "created": 123,
                "model": "gpt-test",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "arguments": "{\"city\":\"Beijing\"}"
                            }
                        }]
                    },
                    "stop_reason": "stop"
                }]
            }"#,
        )
        .expect("valid snapshot chunk");
        snapshot_chunk.normalize_tool_call_arguments(&mut normalizer);
        let snapshot_responses = snapshot_chunk.into_unified_responses();
        assert_eq!(
            snapshot_responses[0]
                .tool_call
                .as_ref()
                .and_then(|tool| tool.arguments.as_deref()),
            Some("{\"city\":\"Beijing\"}")
        );
        assert!(
            snapshot_responses[0]
                .tool_call
                .as_ref()
                .expect("tool call")
                .arguments_is_snapshot
        );
        assert!(snapshot_responses[0].finish_reason.is_none());
    }

    #[test]
    fn leaves_normal_tool_delta_chunks_as_non_snapshot() {
        let mut normalizer = OpenAIToolCallArgumentsNormalizer::default();

        let mut chunk: OpenAISSEData = serde_json::from_str(
            r#"{
                "id": "chatcmpl_test",
                "created": 123,
                "model": "gpt-test",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "arguments": "jing"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }]
            }"#,
        )
        .expect("valid chunk");
        chunk.normalize_tool_call_arguments(&mut normalizer);
        let responses = chunk.into_unified_responses();
        assert_eq!(responses.len(), 1);
        assert!(
            !responses[0]
                .tool_call
                .as_ref()
                .expect("tool call")
                .arguments_is_snapshot
        );
    }

    #[test]
    fn parses_numeric_stop_reason_as_string() {
        let data: OpenAISSEData = serde_json::from_str(
            r#"{
                "id": "chatcmpl_test",
                "created": 123,
                "model": "gpt-test",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "arguments": "{\"a\":1}"
                            }
                        }]
                    },
                    "stop_reason": 154829
                }]
            }"#,
        )
        .expect("valid numeric stop_reason payload");

        let mut normalizer = OpenAIToolCallArgumentsNormalizer::default();
        let mut data = data;
        data.normalize_tool_call_arguments(&mut normalizer);
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert!(responses[0].tool_call.is_some());
    }

    #[test]
    fn parses_string_stop_reason_unchanged() {
        let data: OpenAISSEData = serde_json::from_str(
            r#"{
                "id": "chatcmpl_test",
                "created": 123,
                "model": "gpt-test",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "arguments": "{\"a\":1}"
                            }
                        }]
                    },
                    "stop_reason": "154829"
                }]
            }"#,
        )
        .expect("valid string stop_reason payload");

        let mut normalizer = OpenAIToolCallArgumentsNormalizer::default();
        let mut data = data;
        data.normalize_tool_call_arguments(&mut normalizer);
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert!(responses[0].tool_call.is_some());
    }
}
