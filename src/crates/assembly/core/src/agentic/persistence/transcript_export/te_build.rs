//! Build pipeline: validate options, select turns, build sections, render body.

use super::te_types::TranscriptSectionData;
use crate::agentic::persistence::manager::PersistenceManager;
use crate::agentic::persistence::transcript_fingerprint::ParsedTranscriptTurnSelector;
use crate::service::session::{
    DialogTurnData, SessionTranscriptExportOptions, SessionTranscriptIndexEntry, TranscriptLineRange,
};
use crate::util::errors::NortHingResult;

impl PersistenceManager {
    /// Resolve and normalize the export options from raw user input.
    /// - parses turn selectors via the fingerprint sibling helper
    /// - normalizes them into the round-trippable form stored on the export
    pub(super) fn prepare_export_options(
        options: &SessionTranscriptExportOptions,
    ) -> NortHingResult<(
        SessionTranscriptExportOptions,
        Option<Vec<ParsedTranscriptTurnSelector>>,
    )> {
        let parsed_turn_selectors = options
            .turns
            .as_ref()
            .map(|selectors| Self::parse_transcript_turn_selectors(selectors))
            .transpose()?;
        let normalized_options = SessionTranscriptExportOptions {
            tools: options.tools,
            tool_inputs: options.tool_inputs,
            thinking: options.thinking,
            turns: parsed_turn_selectors
                .as_ref()
                .map(|selectors| selectors.iter().map(|selector| selector.normalized.clone()).collect()),
        };
        Ok((normalized_options, parsed_turn_selectors))
    }

    /// Compute the set of turn indices that match the parsed turn selectors.
    /// When no selectors are present, selects every persisted turn in order.
    pub(super) fn select_export_turn_indices(
        total: usize,
        parsed_turn_selectors: &Option<Vec<ParsedTranscriptTurnSelector>>,
    ) -> Vec<usize> {
        parsed_turn_selectors
            .as_ref()
            .map(|selectors| Self::transcript_select_turn_indices(total, selectors))
            .unwrap_or_else(|| (0..total).collect::<Vec<_>>())
    }

    /// Build the per-turn sections in source order. Returns `(source_index, section)`
    /// pairs so callers can preserve the mapping between selected turn indices and
    /// their original positions in the persisted turn list.
    pub(crate) fn build_export_sections(
        all_turns: &[DialogTurnData],
        selected_indices: &[usize],
        normalized_options: &SessionTranscriptExportOptions,
    ) -> Vec<(usize, TranscriptSectionData)> {
        selected_indices
            .iter()
            .map(|&index| {
                (
                    index,
                    Self::build_transcript_section(&all_turns[index], normalized_options),
                )
            })
            .collect()
    }

    /// Render the markdown body that combines the index header and the per-turn
    /// sections (with omitted-range placeholders between non-contiguous sections).
    /// Returns `(lines, index_entries, index_range)` ready to be written to disk.
    pub(crate) fn render_transcript_body(
        all_turns: &[DialogTurnData],
        sections: &[(usize, TranscriptSectionData)],
    ) -> (Vec<String>, Vec<SessionTranscriptIndexEntry>, TranscriptLineRange) {
        let mut lines = vec!["## Index".to_string()];

        let mut index = Vec::with_capacity(sections.len());
        if sections.is_empty() {
            lines.push(if all_turns.is_empty() {
                "(no persisted turns)".to_string()
            } else {
                "(no matching turns)".to_string()
            });
        } else {
            let index_offset = lines.len() + sections.len() + 1;
            let mut body_lines = Vec::new();

            for (position, (source_index, section)) in sections.iter().enumerate() {
                let omitted_range = if position == 0 {
                    (*source_index > 0).then(|| (0, *source_index - 1))
                } else {
                    let previous_index = sections[position - 1].0;
                    (*source_index > previous_index + 1).then(|| (previous_index + 1, *source_index - 1))
                };

                if let Some((start, end)) = omitted_range {
                    if !body_lines.is_empty() {
                        body_lines.push(String::new());
                    }
                    body_lines.push(Self::transcript_omitted_turns_label(all_turns, start, end));
                    body_lines.push(String::new());
                } else if !body_lines.is_empty() {
                    body_lines.push(String::new());
                }

                let section_offset = index_offset + body_lines.len();
                let turn_range = Self::offset_range(&section.turn_range, section_offset);
                let user_range = Self::offset_range(&section.user_range, section_offset);

                let index_line = format!(
                    "- turn={} range={} preview=\"{}\"",
                    section.turn_index,
                    Self::format_range(&turn_range),
                    section.preview.replace('"', "'")
                );
                lines.push(index_line);

                index.push(SessionTranscriptIndexEntry {
                    turn_index: section.turn_index,
                    preview: section.preview.clone(),
                    turn_range,
                    user_range,
                });

                body_lines.extend(section.lines.iter().cloned());
            }

            if let Some((last_index, _)) = sections.last() {
                if *last_index + 1 < all_turns.len() {
                    body_lines.push(String::new());
                    body_lines.push(Self::transcript_omitted_turns_label(
                        all_turns,
                        *last_index + 1,
                        all_turns.len() - 1,
                    ));
                }
            }

            lines.push(String::new());
            lines.extend(body_lines);
        }

        let index_range = TranscriptLineRange {
            start_line: 1,
            end_line: lines.iter().position(|line| line.is_empty()).unwrap_or(lines.len()),
        };
        (lines, index, index_range)
    }
}
