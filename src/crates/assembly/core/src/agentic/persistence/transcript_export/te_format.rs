//! Pure formatting helpers for transcript export.

use super::te_types::{
    TranscriptRoundBlock, TranscriptRoundData, TranscriptSectionData, TranscriptTextBlock, TranscriptToolBlock,
    SESSION_TRANSCRIPT_PREVIEW_CHAR_LIMIT,
};
use crate::agentic::core::strip_prompt_markup;
use crate::agentic::persistence::manager::PersistenceManager;
use crate::service::session::{DialogTurnData, SessionTranscriptExportOptions, ToolItemData};
impl PersistenceManager {
    pub(crate) fn transcript_preview(content: &str) -> String {
        let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            return "(empty user message)".to_string();
        }

        let mut preview: String = normalized.chars().take(SESSION_TRANSCRIPT_PREVIEW_CHAR_LIMIT).collect();
        if normalized.chars().count() > SESSION_TRANSCRIPT_PREVIEW_CHAR_LIMIT {
            preview.push_str("...");
        }
        preview
    }

    pub(crate) fn transcript_text_lines(content: &str) -> Vec<String> {
        if content.is_empty() {
            return vec!["(empty)".to_string()];
        }

        let lines = content.lines().map(|line| line.to_string()).collect::<Vec<_>>();
        if lines.is_empty() {
            vec!["(empty)".to_string()]
        } else {
            lines
        }
    }

    pub(crate) fn transcript_value_string(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(text) => text.clone(),
            _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
        }
    }

    pub(crate) fn transcript_tool_input(item: &ToolItemData, tool_inputs: bool) -> Option<String> {
        if !tool_inputs || item.tool_call.input.is_null() {
            return None;
        }

        Some(Self::transcript_value_string(&item.tool_call.input))
    }

    pub(crate) fn transcript_tool_result(item: &ToolItemData) -> Option<String> {
        item.tool_result.as_ref().and_then(|result| {
            result
                .result_for_assistant
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    if result.result.is_null() {
                        None
                    } else {
                        Some(Self::transcript_value_string(&result.result))
                    }
                })
        })
    }

    pub(crate) fn transcript_display_user_content(turn: &DialogTurnData) -> String {
        turn.user_message
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("original_text"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| strip_prompt_markup(&turn.user_message.content))
    }

    pub(crate) fn transcript_assistant_blocks(turn: &DialogTurnData) -> Vec<TranscriptTextBlock> {
        turn.model_rounds
            .iter()
            .filter_map(|round| {
                let content = round
                    .text_items
                    .iter()
                    .filter(|item| !item.is_subagent_item.unwrap_or(false))
                    .map(|item| item.content.trim())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if content.is_empty() {
                    None
                } else {
                    Some(TranscriptTextBlock {
                        round_index: round.round_index,
                        content,
                    })
                }
            })
            .collect()
    }

    pub(crate) fn transcript_thinking_blocks(turn: &DialogTurnData) -> Vec<TranscriptTextBlock> {
        turn.model_rounds
            .iter()
            .filter_map(|round| {
                let content = round
                    .thinking_items
                    .iter()
                    .filter(|item| !item.is_subagent_item.unwrap_or(false))
                    .map(|item| item.content.trim())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if content.is_empty() {
                    None
                } else {
                    Some(TranscriptTextBlock {
                        round_index: round.round_index,
                        content,
                    })
                }
            })
            .collect()
    }

    pub(crate) fn transcript_tool_blocks(turn: &DialogTurnData, tool_inputs: bool) -> Vec<TranscriptToolBlock> {
        turn.model_rounds
            .iter()
            .flat_map(|round| round.tool_items.iter())
            .filter(|item| !item.is_subagent_item.unwrap_or(false))
            .map(|item| TranscriptToolBlock {
                tool_name: item.tool_name.clone(),
                tool_input: Self::transcript_tool_input(item, tool_inputs),
                result: Self::transcript_tool_result(item),
            })
            .collect()
    }

    pub(crate) fn transcript_round_blocks(
        turn: &DialogTurnData,
        options: &SessionTranscriptExportOptions,
    ) -> Vec<TranscriptRoundData> {
        turn.model_rounds
            .iter()
            .filter_map(|round| {
                let thinking_content = if options.thinking {
                    round
                        .thinking_items
                        .iter()
                        .filter(|item| !item.is_subagent_item.unwrap_or(false))
                        .map(|item| item.content.trim())
                        .filter(|value| !value.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n")
                } else {
                    String::new()
                };

                let assistant_content = round
                    .text_items
                    .iter()
                    .filter(|item| !item.is_subagent_item.unwrap_or(false))
                    .map(|item| item.content.trim())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let tool_blocks = if options.tools {
                    round
                        .tool_items
                        .iter()
                        .filter(|item| !item.is_subagent_item.unwrap_or(false))
                        .map(|item| TranscriptToolBlock {
                            tool_name: item.tool_name.clone(),
                            tool_input: Self::transcript_tool_input(item, options.tool_inputs),
                            result: Self::transcript_tool_result(item),
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                if thinking_content.is_empty() && assistant_content.is_empty() && tool_blocks.is_empty() {
                    return None;
                }

                let mut blocks = Vec::new();
                if !thinking_content.is_empty() {
                    blocks.push(TranscriptRoundBlock::Thinking(thinking_content));
                }
                if !assistant_content.is_empty() {
                    blocks.push(TranscriptRoundBlock::Assistant(assistant_content));
                }
                for tool in tool_blocks {
                    blocks.push(TranscriptRoundBlock::Tool(tool));
                }

                Some(TranscriptRoundData {
                    round_index: round.round_index,
                    blocks,
                })
            })
            .collect()
    }

    pub(crate) fn push_transcript_block(
        lines: &mut Vec<String>,
        label: &str,
        body_lines: Vec<String>,
    ) -> crate::service::session::TranscriptLineRange {
        let start_line = lines.len() + 1;
        lines.push(format!("[{}]", label));
        lines.extend(body_lines);
        lines.push(format!("[/{}]", label));
        crate::service::session::TranscriptLineRange {
            start_line,
            end_line: lines.len(),
        }
    }

    pub(crate) fn build_transcript_section(
        turn: &DialogTurnData,
        options: &SessionTranscriptExportOptions,
    ) -> TranscriptSectionData {
        let user_content = Self::transcript_display_user_content(turn);
        let round_blocks = Self::transcript_round_blocks(turn, options);

        let mut lines = Vec::new();
        lines.push(format!("## Turn {}", turn.turn_index));
        lines.push(String::new());

        let user_range = Self::push_transcript_block(&mut lines, "user", Self::transcript_text_lines(&user_content));

        if !round_blocks.is_empty() {
            lines.push(String::new());
            for (round_index, round) in round_blocks.iter().enumerate() {
                lines.push(format!("[assistant_round {}]", round.round_index));
                for (block_index, block) in round.blocks.iter().enumerate() {
                    match block {
                        TranscriptRoundBlock::Thinking(content) => {
                            lines.push("[thinking]".to_string());
                            lines.extend(Self::transcript_text_lines(content));
                            lines.push("[/thinking]".to_string());
                        }
                        TranscriptRoundBlock::Assistant(content) => {
                            lines.push("[text]".to_string());
                            lines.extend(Self::transcript_text_lines(content));
                            lines.push("[/text]".to_string());
                        }
                        TranscriptRoundBlock::Tool(tool) => {
                            lines.push("[tool]".to_string());
                            lines.push(format!("name: {}", tool.tool_name));
                            if let Some(tool_input) = tool.tool_input.as_ref() {
                                lines.push("input:".to_string());
                                lines.extend(Self::transcript_text_lines(tool_input));
                            }
                            if let Some(result) = tool.result.as_ref() {
                                lines.push("result:".to_string());
                                lines.extend(Self::transcript_text_lines(result));
                            }
                            lines.push("[/tool]".to_string());
                        }
                    }

                    if block_index + 1 < round.blocks.len() {
                        lines.push(String::new());
                    }
                }
                lines.push(format!("[/assistant_round {}]", round.round_index));
                if round_index + 1 < round_blocks.len() {
                    lines.push(String::new());
                }
            }
        }

        TranscriptSectionData {
            turn_index: turn.turn_index,
            preview: Self::transcript_preview(&user_content),
            turn_range: crate::service::session::TranscriptLineRange {
                start_line: 1,
                end_line: lines.len(),
            },
            user_range,
            lines,
        }
    }

    pub(crate) fn offset_range(
        range: &crate::service::session::TranscriptLineRange,
        offset: usize,
    ) -> crate::service::session::TranscriptLineRange {
        crate::service::session::TranscriptLineRange {
            start_line: range.start_line + offset,
            end_line: range.end_line + offset,
        }
    }

    pub(crate) fn format_range(range: &crate::service::session::TranscriptLineRange) -> String {
        format!("{}-{}", range.start_line, range.end_line)
    }
}
