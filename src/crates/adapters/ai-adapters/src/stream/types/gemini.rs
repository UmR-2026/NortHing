#[path = "gem_response.rs"]
mod gem_response;
#[path = "gem_types.rs"]
mod gem_types;

pub use gem_types::*;

#[cfg(test)]
mod tests {
    use super::GeminiSSEData;

    #[test]
    fn converts_text_thought_and_usage() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "text": "thinking", "thought": true, "thoughtSignature": "sig_1" },
                        { "text": "answer" }
                    ]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 4,
                "thoughtsTokenCount": 2,
                "totalTokenCount": 14
            }
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].reasoning_content.as_deref(), Some("thinking"));
        assert_eq!(responses[0].thinking_signature.as_deref(), Some("sig_1"));
        assert_eq!(
            responses[0]
                .usage
                .as_ref()
                .and_then(|usage| usage.reasoning_token_count),
            Some(2)
        );
        assert_eq!(
            responses[0].usage.as_ref().map(|usage| usage.candidates_token_count),
            Some(6)
        );
        assert_eq!(
            responses[0].usage.as_ref().map(|usage| usage.total_token_count),
            Some(14)
        );
        assert_eq!(responses[1].text.as_deref(), Some("answer"));
    }

    #[test]
    fn keeps_thought_signature_on_function_call_parts() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "thoughtSignature": "sig_tool",
                            "functionCall": {
                                "name": "get_weather",
                                "args": { "city": "Paris" }
                            }
                        }
                    ]
                }
            }]
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].thinking_signature.as_deref(), Some("sig_tool"));
        assert_eq!(
            responses[0]
                .tool_call
                .as_ref()
                .and_then(|tool_call| tool_call.name.as_deref()),
            Some("get_weather")
        );
        assert_eq!(
            responses[0]
                .tool_call
                .as_ref()
                .and_then(|tool_call| tool_call.tool_call_index),
            Some(0)
        );
        assert!(responses[0]
            .tool_call
            .as_ref()
            .is_some_and(|tool_call| tool_call.arguments_is_snapshot));
    }

    #[test]
    fn indexes_parallel_function_call_parts_and_finishes_after_all_tools() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "functionCall": {
                                "name": "read_file",
                                "args": { "path": "a.rs" }
                            }
                        },
                        {
                            "functionCall": {
                                "name": "read_file",
                                "args": { "path": "b.rs" }
                            }
                        }
                    ]
                },
                "finishReason": "STOP"
            }]
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0]
                .tool_call
                .as_ref()
                .and_then(|tool_call| tool_call.tool_call_index),
            Some(0)
        );
        assert_eq!(
            responses[1]
                .tool_call
                .as_ref()
                .and_then(|tool_call| tool_call.tool_call_index),
            Some(1)
        );
        assert!(responses[0].finish_reason.is_none());
        assert_eq!(responses[1].finish_reason.as_deref(), Some("STOP"));
    }

    #[test]
    fn keeps_standalone_thought_signature_parts() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "thoughtSignature": "sig_only" }
                    ]
                }
            }]
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].thinking_signature.as_deref(), Some("sig_only"));
        assert!(responses[0].tool_call.is_none());
        assert!(responses[0].text.is_none());
        assert!(responses[0].reasoning_content.is_none());
    }

    #[test]
    fn converts_code_execution_parts_to_reasoning_chunks() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "executableCode": {
                                "language": "PYTHON",
                                "code": "print(1 + 1)"
                            }
                        },
                        {
                            "codeExecutionResult": {
                                "outcome": "OUTCOME_OK",
                                "output": "2"
                            }
                        }
                    ]
                }
            }]
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert!(responses[0]
            .reasoning_content
            .as_deref()
            .is_some_and(|text| text.contains("print(1 + 1)")));
        assert!(responses[1]
            .reasoning_content
            .as_deref()
            .is_some_and(|text| text.contains("OUTCOME_OK") && text.contains("2")));
    }

    #[test]
    fn emits_grounding_summary_and_provider_metadata() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "text": "answer" }
                    ]
                },
                "groundingMetadata": {
                    "webSearchQueries": ["latest rust release"],
                    "groundingChunks": [
                        {
                            "web": {
                                "uri": "https://www.rust-lang.org",
                                "title": "Rust"
                            }
                        }
                    ]
                }
            }]
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].text.as_deref(), Some("answer"));
        assert!(responses[1]
            .text
            .as_deref()
            .is_some_and(|text| text.contains("Sources:") && text.contains("rust-lang.org")));
        assert!(responses[1]
            .provider_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("groundingMetadata"))
            .is_some());
    }

    #[test]
    fn emits_prompt_feedback_and_safety_summary() {
        let payload = serde_json::json!({
            "candidates": [{
                "content": { "parts": [] },
                "finishReason": "SAFETY",
                "safetyRatings": [
                    {
                        "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                        "probability": "MEDIUM",
                        "blocked": true
                    }
                ]
            }],
            "promptFeedback": {
                "blockReason": "SAFETY",
                "blockReasonMessage": "Blocked by safety system"
            }
        });

        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let responses = data.into_unified_responses();

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].finish_reason.as_deref(), Some("SAFETY"));
        assert!(responses[0]
            .text
            .as_deref()
            .is_some_and(|text| text.contains("Prompt blocked reason: SAFETY")));
        assert!(responses[0]
            .text
            .as_deref()
            .is_some_and(|text| text.contains("HARM_CATEGORY_DANGEROUS_CONTENT")));
        assert!(responses[0]
            .provider_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("promptFeedback"))
            .is_some());
    }

    #[test]
    fn gemini_cache_creation_is_always_none() {
        let payload = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": "answer" }] } }],
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 20,
                "totalTokenCount": 120,
                "cachedContentTokenCount": 35
            }
        });
        let data: GeminiSSEData = serde_json::from_value(payload).expect("gemini payload");
        let usage = data.into_unified_responses()[0].usage.as_ref().expect("usage").clone();
        assert_eq!(usage.cached_content_token_count, Some(35));
        assert_eq!(usage.cache_creation_token_count, None);
    }
}
