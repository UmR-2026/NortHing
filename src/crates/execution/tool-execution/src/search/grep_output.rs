//! Output rendering for the remote grep backend.
//!
//! These helpers translate a [`RemoteGrepCommandRequest`] / raw stdout into
//! the final user-facing text:
//! - [`build_remote_grep_command`] — produce a shell command (`rg` preferred,
//!   `grep` fallback) for the given request.
//! - [`count_remote_grep_matches`] — naive line count for `count` output mode.
//! - [`relativize_result_text`] / [`render_remote_grep_result_text`] — strip
//!   the display-base prefix and substitute a friendly "no matches" message.
//! - [`apply_offset_and_limit`] — in-place [`Vec<String>`] trimmer used by the
//!   remote tooling layer.

use crate::util::string::shell_single_quote;

use super::grep_types::{OutputMode, RemoteGrepCommandRequest};

pub fn build_remote_grep_command(request: &RemoteGrepCommandRequest) -> String {
    let offset_cmd = if request.offset > 0 {
        format!(" | tail -n +{}", request.offset + 1)
    } else {
        String::new()
    };
    let limit_cmd = request
        .head_limit
        .map(|limit| format!(" | head -n {}", limit))
        .unwrap_or_default();

    let mut cmd = "rg --no-heading --hidden --max-columns 500".to_string();
    if request.case_insensitive {
        cmd.push_str(" -i");
    }
    if request.output_mode == OutputMode::FilesWithMatches {
        cmd.push_str(" -l");
    } else if request.output_mode == OutputMode::Count {
        cmd.push_str(" -c");
    } else if request.show_line_numbers {
        cmd.push_str(" --line-number");
    }
    if request.output_mode == OutputMode::Content {
        if let Some(context) = request.context {
            cmd.push_str(&format!(" -C {}", context));
        } else {
            if let Some(before) = request.before_context {
                cmd.push_str(&format!(" -B {}", before));
            }
            if let Some(after) = request.after_context {
                cmd.push_str(&format!(" -A {}", after));
            }
        }
    }
    for glob_pattern in &request.glob_patterns {
        cmd.push_str(&format!(" --glob {}", shell_single_quote(glob_pattern)));
    }
    if let Some(file_type) = &request.file_type {
        cmd.push_str(&format!(" --type {}", shell_single_quote(file_type)));
    }
    cmd.push_str(&format!(
        " -e {} {} 2>/dev/null{}{}",
        shell_single_quote(&request.pattern),
        shell_single_quote(&request.path),
        offset_cmd,
        limit_cmd
    ));

    format!(
        "if command -v rg >/dev/null 2>&1; then {}; else grep -rn{} -e {} {} 2>/dev/null{}{}; fi",
        cmd,
        if request.case_insensitive { "i" } else { "" },
        shell_single_quote(&request.pattern),
        shell_single_quote(&request.path),
        offset_cmd,
        limit_cmd,
    )
}

pub fn count_remote_grep_matches(stdout: &str) -> usize {
    stdout.lines().count()
}

pub fn relativize_result_text(result_text: &str, display_base: Option<&str>) -> String {
    let Some(base) = display_base else {
        return result_text.to_string();
    };

    let normalized_base = base.replace('\\', "/").trim_end_matches('/').to_string();
    if normalized_base.is_empty() {
        return result_text.to_string();
    }

    result_text
        .lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix(&(normalized_base.clone() + "/")) {
                rest.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_remote_grep_result_text(stdout: &str, pattern: &str, display_base: Option<&str>) -> String {
    if stdout.lines().next().is_none() {
        format!("No matches found for pattern '{}'", pattern)
    } else {
        relativize_result_text(stdout, display_base)
    }
}

pub fn apply_offset_and_limit(items: &mut Vec<String>, offset: usize, head_limit: Option<usize>) {
    if offset > 0 {
        if offset >= items.len() {
            items.clear();
        } else {
            *items = items[offset..].to_vec();
        }
    }

    if let Some(limit) = head_limit {
        if items.len() > limit {
            items.truncate(limit);
        }
    }
}
