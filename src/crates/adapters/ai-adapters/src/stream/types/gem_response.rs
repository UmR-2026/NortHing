use super::gem_types::*;
use crate::stream::types::unified::{UnifiedResponse, UnifiedToolCall};
use serde_json::{json, Value};

impl GeminiSSEData {
    fn render_executable_code(executable_code: &GeminiExecutableCode) -> Option<String> {
        let code = executable_code.code.as_deref()?.trim();
        if code.is_empty() {
            return None;
        }

        let language = executable_code
            .language
            .as_deref()
            .map(|language| language.to_ascii_lowercase())
            .unwrap_or_else(|| "text".to_string());

        Some(format!(
            "Gemini code execution generated code:\n```{}\n{}\n```",
            language, code
        ))
    }

    fn render_code_execution_result(result: &GeminiCodeExecutionResult) -> Option<String> {
        let output = result.output.as_deref()?.trim();
        if output.is_empty() {
            return None;
        }

        let outcome = result.outcome.as_deref().unwrap_or("OUTCOME_UNKNOWN");
        Some(format!("Gemini code execution result ({}):\n{}", outcome, output))
    }

    fn grounding_summary(metadata: &Value) -> Option<String> {
        let mut lines = Vec::new();

        let queries = metadata
            .get("webSearchQueries")
            .and_then(Value::as_array)
            .map(|queries| {
                queries
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|query| !query.trim().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !queries.is_empty() {
            lines.push(format!("Search queries: {}", queries.join(" | ")));
        }

        let sources = metadata
            .get("groundingChunks")
            .and_then(Value::as_array)
            .map(|chunks| {
                chunks
                    .iter()
                    .filter_map(|chunk| {
                        let web = chunk.get("web")?;
                        let uri = web.get("uri").and_then(Value::as_str)?.trim();
                        if uri.is_empty() {
                            return None;
                        }
                        let title = web
                            .get("title")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|title| !title.is_empty())
                            .unwrap_or(uri);
                        Some((title.to_string(), uri.to_string()))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !sources.is_empty() {
            lines.push("Sources:".to_string());
            for (index, (title, uri)) in sources.into_iter().enumerate() {
                lines.push(format!("{}. {} - {}", index + 1, title, uri));
            }
        }

        let supports = metadata
            .get("groundingSupports")
            .and_then(Value::as_array)
            .map(|supports| {
                supports
                    .iter()
                    .filter_map(|support| {
                        let segment_text = support
                            .get("segment")
                            .and_then(Value::as_object)
                            .and_then(|segment| segment.get("text"))
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|text| !text.is_empty())?;

                        let chunk_indices = support
                            .get("groundingChunkIndices")
                            .and_then(Value::as_array)
                            .map(|indices| {
                                indices
                                    .iter()
                                    .filter_map(Value::as_u64)
                                    .map(|index| (index + 1).to_string())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        if chunk_indices.is_empty() {
                            None
                        } else {
                            Some((segment_text.to_string(), chunk_indices.join(", ")))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !supports.is_empty() {
            lines.push("Citations:".to_string());
            for (segment, indices) in supports.into_iter().take(5) {
                lines.push(format!("- \"{}\" -> [{}]", segment, indices));
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    fn safety_summary(prompt_feedback: Option<&Value>, safety_ratings: Option<&Value>) -> Option<String> {
        let mut lines = Vec::new();

        if let Some(prompt_feedback) = prompt_feedback {
            if let Some(blocked_reason) = prompt_feedback
                .get("blockReason")
                .and_then(Value::as_str)
                .filter(|reason| !reason.trim().is_empty())
            {
                lines.push(format!("Prompt blocked reason: {}", blocked_reason));
            }

            if let Some(block_reason_message) = prompt_feedback
                .get("blockReasonMessage")
                .and_then(Value::as_str)
                .filter(|message| !message.trim().is_empty())
            {
                lines.push(format!("Prompt block message: {}", block_reason_message));
            }
        }

        let ratings = safety_ratings
            .and_then(Value::as_array)
            .map(|ratings| {
                ratings
                    .iter()
                    .filter_map(|rating| {
                        let category = rating.get("category").and_then(Value::as_str)?;
                        let probability = rating.get("probability").and_then(Value::as_str).unwrap_or("UNKNOWN");
                        let blocked = rating.get("blocked").and_then(Value::as_bool).unwrap_or(false);

                        if blocked || probability != "NEGLIGIBLE" {
                            Some(format!(
                                "{} (probability={}, blocked={})",
                                category, probability, blocked
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !ratings.is_empty() {
            lines.push("Safety ratings:".to_string());
            lines.extend(ratings.into_iter().map(|rating| format!("- {}", rating)));
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    fn provider_metadata_summary(metadata: &Value) -> Option<String> {
        let prompt_feedback = metadata.get("promptFeedback");
        let grounding_metadata = metadata.get("groundingMetadata");
        let safety_ratings = metadata.get("safetyRatings");

        let mut sections = Vec::new();
        if let Some(safety) = Self::safety_summary(prompt_feedback, safety_ratings) {
            sections.push(safety);
        }
        if let Some(grounding) = grounding_metadata.and_then(Self::grounding_summary) {
            sections.push(grounding);
        }

        if sections.is_empty() {
            None
        } else {
            Some(sections.join("\n\n"))
        }
    }

    pub fn into_unified_responses(self) -> Vec<UnifiedResponse> {
        let mut usage = self.usage_metadata.map(Into::into);
        let prompt_feedback = self.prompt_feedback;
        let Some(candidate) = self.candidates.into_iter().next() else {
            return usage
                .take()
                .map(|usage| {
                    vec![UnifiedResponse {
                        usage: Some(usage),
                        ..Default::default()
                    }]
                })
                .unwrap_or_default();
        };

        let mut responses = Vec::new();
        let finish_reason = candidate.finish_reason;
        let grounding_metadata = candidate.grounding_metadata;
        let safety_ratings = candidate.safety_ratings;

        if let Some(content) = candidate.content {
            for (part_index, part) in content.parts.into_iter().enumerate() {
                let has_function_call = part.function_call.is_some();
                let text = part.text.filter(|text| !text.is_empty());
                let is_thought = part.thought.unwrap_or(false);
                let thinking_signature = part.thought_signature.filter(|value| !value.is_empty());

                if let Some(function_call) = part.function_call {
                    let arguments = function_call.args.unwrap_or_else(|| json!({}));
                    responses.push(UnifiedResponse {
                        text: None,
                        reasoning_content: None,
                        thinking_signature,
                        tool_call: Some(UnifiedToolCall {
                            tool_call_index: Some(part_index),
                            id: None,
                            name: function_call.name,
                            arguments: serde_json::to_string(&arguments).ok(),
                            arguments_is_snapshot: true,
                        }),
                        usage: usage.take(),
                        finish_reason: None,
                        provider_metadata: None,
                    });
                    continue;
                }

                if let Some(executable_code) = part.executable_code.as_ref() {
                    if let Some(reasoning_content) = Self::render_executable_code(executable_code) {
                        responses.push(UnifiedResponse {
                            text: None,
                            reasoning_content: Some(reasoning_content),
                            thinking_signature,
                            tool_call: None,
                            usage: usage.take(),
                            finish_reason: None,
                            provider_metadata: None,
                        });
                        continue;
                    }
                }

                if let Some(code_execution_result) = part.code_execution_result.as_ref() {
                    if let Some(reasoning_content) = Self::render_code_execution_result(code_execution_result) {
                        responses.push(UnifiedResponse {
                            text: None,
                            reasoning_content: Some(reasoning_content),
                            thinking_signature,
                            tool_call: None,
                            usage: usage.take(),
                            finish_reason: None,
                            provider_metadata: None,
                        });
                        continue;
                    }
                }

                if let Some(text) = text {
                    responses.push(UnifiedResponse {
                        text: if is_thought { None } else { Some(text.clone()) },
                        reasoning_content: if is_thought { Some(text) } else { None },
                        thinking_signature,
                        tool_call: None,
                        usage: usage.take(),
                        finish_reason: None,
                        provider_metadata: None,
                    });
                    continue;
                }

                if thinking_signature.is_some() && !has_function_call {
                    responses.push(UnifiedResponse {
                        text: None,
                        reasoning_content: None,
                        thinking_signature,
                        tool_call: None,
                        usage: usage.take(),
                        finish_reason: None,
                        provider_metadata: None,
                    });
                }
            }
        }

        let provider_metadata = {
            let mut metadata = serde_json::Map::new();
            if let Some(prompt_feedback) = prompt_feedback {
                metadata.insert("promptFeedback".to_string(), prompt_feedback);
            }
            if let Some(grounding_metadata) = grounding_metadata {
                metadata.insert("groundingMetadata".to_string(), grounding_metadata);
            }
            if let Some(safety_ratings) = safety_ratings {
                metadata.insert("safetyRatings".to_string(), safety_ratings);
            }

            if metadata.is_empty() {
                None
            } else {
                Some(Value::Object(metadata))
            }
        };

        if let Some(provider_metadata) = provider_metadata {
            let summary = Self::provider_metadata_summary(&provider_metadata);
            responses.push(UnifiedResponse {
                text: summary,
                reasoning_content: None,
                thinking_signature: None,
                tool_call: None,
                usage: usage.take(),
                finish_reason: None,
                provider_metadata: Some(provider_metadata),
            });
        }

        if let Some(finish_reason) = finish_reason {
            if let Some(last_response) = responses.last_mut() {
                last_response.finish_reason = Some(finish_reason);
                return responses;
            }

            responses.push(UnifiedResponse {
                usage,
                finish_reason: Some(finish_reason),
                ..Default::default()
            });
            return responses;
        }

        if responses.is_empty() {
            responses.push(UnifiedResponse {
                usage,
                finish_reason,
                ..Default::default()
            });
        }

        responses
    }
}
