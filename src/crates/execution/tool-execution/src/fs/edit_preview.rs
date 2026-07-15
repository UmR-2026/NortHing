use crate::util::read_line_prefix::{read_tool_output_to_file_content, strip_read_line_number_prefix};
use crate::util::string::normalize_string;

/// Remove Read-tool cat -n prefixes line-by-line when present.
pub fn sanitize_read_tool_copied_text(text: &str) -> Option<String> {
    let sanitized = read_tool_output_to_file_content(text);
    (sanitized != text).then_some(sanitized)
}

fn normalize_quote_char(ch: char) -> char {
    match ch {
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{201C}' | '\u{201D}' => '"',
        other => other,
    }
}

fn find_actual_string(file_content: &str, search_string: &str) -> Option<String> {
    if file_content.contains(search_string) {
        return Some(search_string.to_string());
    }

    // Normalize line endings so CRLF files can match LF search strings.
    let normalized_file = normalize_string(file_content);
    let normalized_search = normalize_string(search_string);

    if normalized_file.contains(&normalized_search) {
        return Some(search_string.to_string());
    }

    let file_chars: Vec<char> = normalized_file.chars().collect();
    let search_chars: Vec<char> = normalized_search.chars().collect();
    if search_chars.is_empty() || file_chars.len() < search_chars.len() {
        return None;
    }

    let normalized_search_chars: Vec<char> = search_chars.iter().copied().map(normalize_quote_char).collect();

    for start in 0..=file_chars.len() - search_chars.len() {
        let window_matches = file_chars[start..start + search_chars.len()]
            .iter()
            .copied()
            .map(normalize_quote_char)
            .eq(normalized_search_chars.iter().copied());
        if window_matches {
            return Some(file_chars[start..start + search_chars.len()].iter().collect());
        }
    }

    None
}

/// Replace every tab with `tab_width` spaces.
fn convert_tabs_to_spaces(s: &str, tab_width: usize) -> String {
    s.replace('\t', &" ".repeat(tab_width))
}

/// Replace leading spaces on each line with tabs when the space count is a
/// clean multiple of `tab_width`. Lines whose leading whitespace contains
/// tabs or whose space count is not divisible by `tab_width` are left as-is.
fn convert_leading_spaces_to_tabs(s: &str, tab_width: usize) -> String {
    s.lines()
        .map(|line| {
            let trimmed_start = line.len() - line.trim_start().len();
            let leading = &line[..trimmed_start];

            if leading.is_empty() || !leading.chars().all(|c| c == ' ') || leading.len() % tab_width != 0 {
                return line.to_string();
            }

            let tabs = "\t".repeat(leading.len() / tab_width);
            tabs + &line[trimmed_start..]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn contains_read_tool_line_prefixes(text: &str) -> bool {
    text.lines().any(|line| strip_read_line_number_prefix(line) != line)
}

pub(super) fn contains_read_truncation_marker(text: &str) -> bool {
    text.contains(" [truncated]")
}

pub(super) fn longest_shared_prefix_len(left: &str, right: &str) -> usize {
    left.chars().zip(right.chars()).take_while(|(a, b)| a == b).count()
}

pub(super) fn longest_shared_suffix_len(left: &str, right: &str) -> usize {
    longest_shared_prefix_len(
        &left.chars().rev().collect::<String>(),
        &right.chars().rev().collect::<String>(),
    )
}

pub(super) fn edit_string_candidates(content: &str, old_string: &str, new_string: &str) -> Vec<(String, String)> {
    let mut candidates = Vec::new();
    let mut push_candidate = |old: String, new: String| {
        if !candidates
            .iter()
            .any(|(existing_old, existing_new)| existing_old == &old && existing_new == &new)
        {
            candidates.push((old, new));
        }
    };

    push_candidate(old_string.to_string(), new_string.to_string());

    if let Some(sanitized_old) = sanitize_read_tool_copied_text(old_string) {
        let sanitized_new = sanitize_read_tool_copied_text(new_string).unwrap_or_else(|| new_string.to_string());
        push_candidate(sanitized_old, sanitized_new);
    }

    if let Some(actual_old) = find_actual_string(content, old_string) {
        push_candidate(actual_old, new_string.to_string());
    }

    if !old_string.ends_with('\n') {
        let with_newline = format!("{old_string}\n");
        if content.contains(&with_newline) {
            push_candidate(with_newline, format!("{new_string}\n"));
        }
    }

    // Whitespace-normalization fallbacks: when the model copies indentation
    // with tabs instead of spaces (or vice versa), try common conversions.
    // Only the old/new string pair is transformed — file content is never
    // rewritten speculatively.  Each pair must pass exact match inside
    // apply_match_and_replace (after CRLF normalization) before any write.
    for tab_width in [2, 4] {
        let tabs_to_spaces_old = convert_tabs_to_spaces(old_string, tab_width);
        if tabs_to_spaces_old != old_string {
            let tabs_to_spaces_new = convert_tabs_to_spaces(new_string, tab_width);
            push_candidate(tabs_to_spaces_old.clone(), tabs_to_spaces_new.clone());

            // Also try quote-normalized variant (e.g. curly quotes in file
            // after whitespace normalization).
            if let Some(actual_old) = find_actual_string(content, &tabs_to_spaces_old) {
                push_candidate(actual_old, tabs_to_spaces_new);
            }
        }

        let spaces_to_tabs_old = convert_leading_spaces_to_tabs(old_string, tab_width);
        if spaces_to_tabs_old != old_string {
            let spaces_to_tabs_new = convert_leading_spaces_to_tabs(new_string, tab_width);
            push_candidate(spaces_to_tabs_old.clone(), spaces_to_tabs_new.clone());

            if let Some(actual_old) = find_actual_string(content, &spaces_to_tabs_old) {
                push_candidate(actual_old, spaces_to_tabs_new);
            }
        }
    }

    candidates
}
