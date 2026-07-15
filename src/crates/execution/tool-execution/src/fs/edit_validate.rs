use crate::util::string::normalize_string;

use super::edit_types::{CONTEXT_LINES_AFTER, CONTEXT_LINES_BEFORE, MAX_MATCH_CONTEXTS, NOT_FOUND_DIAGNOSTIC_SNIPPETS};

/// Count lines before given byte position (line numbers start from 1)
pub(super) fn count_lines_before(content: &str, byte_pos: usize) -> usize {
    content[..byte_pos].matches('\n').count() + 1
}

/// Count newlines in string
pub(super) fn count_newlines(s: &str) -> usize {
    s.matches('\n').count()
}

pub(super) fn match_contexts(content: &str, old_string: &str, matches: &[(usize, &str)]) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let old_line_count = count_newlines(old_string) + 1;
    let mut contexts = Vec::new();

    for (idx, (byte_pos, _)) in matches.iter().take(MAX_MATCH_CONTEXTS).enumerate() {
        let start_line = count_lines_before(content, *byte_pos);
        let old_end_line = start_line + old_line_count.saturating_sub(1);
        let context_start_line = start_line.saturating_sub(CONTEXT_LINES_BEFORE).max(1);
        let context_end_line = (old_end_line + CONTEXT_LINES_AFTER).min(lines.len().max(1));
        let snippet = lines[(context_start_line - 1)..context_end_line].join("\n");

        contexts.push(format!(
            "[match {} starts at line {}]\n{}",
            idx + 1,
            start_line,
            snippet
        ));
    }

    let omitted = matches.len().saturating_sub(MAX_MATCH_CONTEXTS);
    let omitted_note = if omitted > 0 {
        format!("\n... {omitted} more matches omitted.")
    } else {
        String::new()
    };

    format!(
        "Matched contexts (copy exact text from a snippet and add stable surrounding lines to make `old_string` unique):\n{}{}",
        contexts.join("\n---\n"),
        omitted_note
    )
}

fn snippet_context(lines: &[&str], line_idx: usize) -> String {
    let start = line_idx.saturating_sub(CONTEXT_LINES_BEFORE);
    let end = (line_idx + CONTEXT_LINES_AFTER + 1).min(lines.len());
    lines[start..end].join("\n")
}

pub(super) fn build_not_found_diagnostics(content: &str, old_string: &str) -> String {
    let mut hints = vec![
        "Re-read the target lines with Read (use start_line/limit if needed), then copy the exact text after the tab on each line into old_string without reformatting indentation.".to_string(),
    ];

    if super::edit_preview::contains_read_tool_line_prefixes(old_string) {
        hints.push(
            "Detected Read-tool line-number prefixes inside `old_string`. Copy only the text after the tab on each line.".to_string(),
        );
    }

    if super::edit_preview::contains_read_truncation_marker(old_string) {
        hints.push(
            "Detected a Read-tool `[truncated]` marker inside `old_string`. Re-read with start_line/limit so the target lines are complete.".to_string(),
        );
    }

    let normalized_content = normalize_string(content);
    let lines: Vec<&str> = normalized_content.split('\n').collect();
    let anchor_line = old_string
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(old_string)
        .trim();

    if !anchor_line.is_empty() {
        let mut candidates = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let shared_prefix = super::edit_preview::longest_shared_prefix_len(anchor_line, trimmed);
            let shared_suffix = super::edit_preview::longest_shared_suffix_len(anchor_line, trimmed);
            let score = shared_prefix.max(shared_suffix);

            if anchor_line.contains(trimmed)
                || trimmed.contains(anchor_line)
                || score >= super::edit_types::NOT_FOUND_MIN_SUBSTRING_LEN
            {
                candidates.push((score, idx));
            }
        }

        candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        candidates.dedup_by_key(|candidate| candidate.1);

        let snippets: Vec<String> = candidates
            .into_iter()
            .take(NOT_FOUND_DIAGNOSTIC_SNIPPETS)
            .map(|(_, idx)| {
                format!(
                    "[nearby content around line {}]\n{}",
                    idx + 1,
                    snippet_context(&lines, idx)
                )
            })
            .collect();

        if !snippets.is_empty() {
            hints.push(format!("Closest current file snippet:\n{}", snippets.join("\n---\n")));
        }
    }

    hints.join("\n\n")
}
