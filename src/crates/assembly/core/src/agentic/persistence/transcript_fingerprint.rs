//! Transcript Fingerprint & Turn Selector parsing sub-handlers (Round 10b split)
//!
//! Owns `transcript_fingerprint` (which hashes the canonical transcript payload)
//! and the turn-selector parser/normalizer that translates user-facing selectors
//! like `:20`, `-20:`, `10:30`, or `15` into a normalized, sorted, deduplicated
//! set of turn indices.
//!
//! This file owns the transcript-fingerprint-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

use super::manager::PersistenceManager;
use crate::service::session::{DialogTurnData, SessionTranscriptExportOptions};
use crate::util::errors::{NortHingError, NortHingResult};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct TranscriptFingerprintPayload {
    session_id: String,
    tools: bool,
    tool_inputs: bool,
    thinking: bool,
    turn_selectors: Option<Vec<String>>,
    turns: Vec<TranscriptFingerprintTurn>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct TranscriptFingerprintTurn {
    turn_id: String,
    turn_index: usize,
    status: String,
    user: String,
    assistant: Vec<TranscriptFingerprintTextBlock>,
    tools: Vec<TranscriptFingerprintTool>,
    thinking: Vec<TranscriptFingerprintTextBlock>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct TranscriptFingerprintTextBlock {
    round_index: usize,
    content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct TranscriptFingerprintTool {
    tool_name: String,
    tool_input: Option<String>,
    result: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TranscriptTurnSelector {
    Index(isize),
    Slice { start: Option<isize>, end: Option<isize> },
}

#[derive(Debug, Clone)]
pub(super) struct ParsedTranscriptTurnSelector {
    pub(crate) normalized: String,
    selector: TranscriptTurnSelector,
}

impl PersistenceManager {
    pub(super) fn transcript_fingerprint(
        session_id: &str,
        turns: &[DialogTurnData],
        options: &SessionTranscriptExportOptions,
    ) -> NortHingResult<String> {
        let payload = TranscriptFingerprintPayload {
            session_id: session_id.to_string(),
            tools: options.tools,
            tool_inputs: options.tool_inputs,
            thinking: options.thinking,
            turn_selectors: options.turns.clone(),
            turns: turns
                .iter()
                .map(|turn| TranscriptFingerprintTurn {
                    turn_id: turn.turn_id.clone(),
                    turn_index: turn.turn_index,
                    status: Self::turn_status_label(&turn.status).to_string(),
                    user: Self::transcript_display_user_content(turn),
                    assistant: Self::transcript_assistant_blocks(turn)
                        .into_iter()
                        .map(|block| TranscriptFingerprintTextBlock {
                            round_index: block.round_index,
                            content: block.content,
                        })
                        .collect(),
                    tools: if options.tools {
                        Self::transcript_tool_blocks(turn, options.tool_inputs)
                            .into_iter()
                            .map(|tool| TranscriptFingerprintTool {
                                tool_name: tool.tool_name,
                                tool_input: tool.tool_input,
                                result: tool.result,
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    thinking: if options.thinking {
                        Self::transcript_thinking_blocks(turn)
                            .into_iter()
                            .map(|block| TranscriptFingerprintTextBlock {
                                round_index: block.round_index,
                                content: block.content,
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        };

        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NortHingError::serialization(format!("Failed to serialize transcript fingerprint: {}", e)))?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub(super) fn parse_transcript_turn_selectors(
        selectors: &[String],
    ) -> NortHingResult<Vec<ParsedTranscriptTurnSelector>> {
        if selectors.is_empty() {
            return Err(NortHingError::Validation("turns cannot be an empty array".to_string()));
        }

        selectors
            .iter()
            .map(|selector| Self::parse_transcript_turn_selector(selector))
            .collect()
    }

    pub(super) fn parse_transcript_turn_selector(selector: &str) -> NortHingResult<ParsedTranscriptTurnSelector> {
        let normalized = selector.trim();
        if normalized.is_empty() {
            return Err(NortHingError::Validation(
                "turns cannot contain empty selectors".to_string(),
            ));
        }

        if normalized.matches(':').count() > 1 {
            return Err(NortHingError::Validation(format!(
                "Invalid turn selector '{}'. Use forms like ':20', '-20:', '10:30', or '15'.",
                normalized
            )));
        }

        let selector = if let Some((start, end)) = normalized.split_once(':') {
            TranscriptTurnSelector::Slice {
                start: if start.is_empty() {
                    None
                } else {
                    Some(Self::parse_transcript_turn_value(start, normalized)?)
                },
                end: if end.is_empty() {
                    None
                } else {
                    Some(Self::parse_transcript_turn_value(end, normalized)?)
                },
            }
        } else {
            TranscriptTurnSelector::Index(Self::parse_transcript_turn_value(normalized, normalized)?)
        };

        Ok(ParsedTranscriptTurnSelector {
            normalized: normalized.to_string(),
            selector,
        })
    }

    pub(super) fn parse_transcript_turn_value(value: &str, selector: &str) -> NortHingResult<isize> {
        value.parse::<isize>().map_err(|_| {
            NortHingError::Validation(format!(
                "Invalid turn selector '{}'. Use forms like ':20', '-20:', '10:30', or '15'.",
                selector
            ))
        })
    }

    pub(super) fn transcript_normalize_slice_bound(total: usize, bound: Option<isize>, default: usize) -> usize {
        let Some(bound) = bound else {
            return default;
        };

        let total = total as isize;
        let normalized = if bound < 0 { total.saturating_add(bound) } else { bound };
        normalized.clamp(0, total) as usize
    }

    pub(super) fn transcript_normalize_index(total: usize, index: isize) -> Option<usize> {
        let total = total as isize;
        let normalized = if index < 0 { total.saturating_add(index) } else { index };

        if normalized < 0 || normalized >= total {
            None
        } else {
            Some(normalized as usize)
        }
    }

    pub(super) fn transcript_select_turn_indices(
        total: usize,
        selectors: &[ParsedTranscriptTurnSelector],
    ) -> Vec<usize> {
        let mut selected = vec![false; total];

        for selector in selectors {
            match selector.selector {
                TranscriptTurnSelector::Index(index) => {
                    if let Some(index) = Self::transcript_normalize_index(total, index) {
                        selected[index] = true;
                    }
                }
                TranscriptTurnSelector::Slice { start, end } => {
                    let start = Self::transcript_normalize_slice_bound(total, start, 0);
                    let end = Self::transcript_normalize_slice_bound(total, end, total);
                    if start < end {
                        selected[start..end].fill(true);
                    }
                }
            }
        }

        selected
            .into_iter()
            .enumerate()
            .filter_map(|(index, is_selected)| is_selected.then_some(index))
            .collect()
    }

    pub(super) fn transcript_omitted_turns_label(turns: &[DialogTurnData], start: usize, end: usize) -> String {
        let start_turn = turns[start].turn_index;
        let end_turn = turns[end].turn_index;
        if start_turn == end_turn {
            format!("(omitted turn {})", start_turn)
        } else {
            format!("(omitted turns {}-{})", start_turn, end_turn)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PersistenceManager;

    #[test]
    fn transcript_turn_selectors_support_head_and_tail_ranges() {
        let selectors = PersistenceManager::parse_transcript_turn_selectors(&[":1".to_string(), "-3:".to_string()])
            .expect("selectors should parse");

        let selected = PersistenceManager::transcript_select_turn_indices(8, &selectors);

        assert_eq!(selected, vec![0, 5, 6, 7]);
    }

    #[test]
    fn transcript_turn_selectors_deduplicate_and_sort_results() {
        let selectors = PersistenceManager::parse_transcript_turn_selectors(&[
            "4".to_string(),
            "2:5".to_string(),
            "-1".to_string(),
        ])
        .expect("selectors should parse");

        let selected = PersistenceManager::transcript_select_turn_indices(6, &selectors);

        assert_eq!(selected, vec![2, 3, 4, 5]);
    }

    #[test]
    fn transcript_turn_selectors_reject_invalid_syntax() {
        let error = PersistenceManager::parse_transcript_turn_selectors(&["1:2:3".to_string()])
            .expect_err("selector should be rejected");

        assert!(
            error.to_string().contains("Invalid turn selector"),
            "unexpected error: {}",
            error
        );
    }
}
