use crate::agentic::core::{Message, MessageContent, MessageRole};
use crate::agentic::insights::types::BaseStats;
use crate::service::session::DialogTurnData;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const ACTIVITY_GAP_THRESHOLD_SECS: u64 = 30 * 60;

pub(super) fn accumulate_stats(
    base_stats: &mut BaseStats,
    session: &crate::agentic::core::Session,
    messages: &[Message],
) {
    base_stats.total_messages += messages.len() as u32;
    base_stats.total_turns += session.dialog_turn_ids.len() as u32;

    let active_secs = compute_active_duration(messages);
    base_stats.total_duration_minutes += active_secs / 60;

    *base_stats.agent_types.entry(session.agent_type.clone()).or_insert(0) += 1;

    let mut last_assistant_time: Option<SystemTime> = None;
    for msg in messages {
        if msg.role == MessageRole::User {
            if let Ok(dur) = msg.timestamp.duration_since(UNIX_EPOCH) {
                let dt = DateTime::<Utc>::from(UNIX_EPOCH + dur);
                let hour = dt.format("%H").to_string().parse::<u32>().unwrap_or(0);
                *base_stats.hour_counts.entry(hour).or_insert(0) += 1;
            }
        }

        match &msg.content {
            MessageContent::Mixed { tool_calls, .. } => {
                for tc in tool_calls {
                    *base_stats.tool_usage.entry(tc.tool_name.clone()).or_insert(0) += 1;
                }
            }
            MessageContent::ToolResult {
                tool_name, is_error, ..
            } if *is_error => {
                *base_stats.tool_errors.entry(tool_name.clone()).or_insert(0) += 1;
            }
            _ => {}
        }

        match msg.role {
            MessageRole::Assistant => {
                last_assistant_time = Some(msg.timestamp);
            }
            MessageRole::User => {
                if let Some(prev) = last_assistant_time {
                    if let Ok(duration) = msg.timestamp.duration_since(prev) {
                        let secs = duration.as_secs();
                        if (2..=ACTIVITY_GAP_THRESHOLD_SECS).contains(&secs) {
                            base_stats.response_times_raw.push(secs as f64);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Compute active usage duration by summing adjacent message gaps,
/// capping each gap at `ACTIVITY_GAP_THRESHOLD_SECS`.
pub(super) fn compute_active_duration(messages: &[Message]) -> u64 {
    if messages.len() < 2 {
        return 0;
    }
    let mut total_secs: u64 = 0;
    for pair in messages.windows(2) {
        if let Ok(gap) = pair[1].timestamp.duration_since(pair[0].timestamp) {
            let gap_secs = gap.as_secs();
            if gap_secs <= ACTIVITY_GAP_THRESHOLD_SECS {
                total_secs += gap_secs;
            }
        }
    }
    total_secs
}

pub(super) fn bucket_response_times(raw: &[f64]) -> HashMap<String, u32> {
    let buckets: &[(&str, f64, f64)] = &[
        ("2-10s", 2.0, 10.0),
        ("10-30s", 10.0, 30.0),
        ("30s-1m", 30.0, 60.0),
        ("1-2m", 60.0, 120.0),
        ("2-5m", 120.0, 300.0),
        ("5-15m", 300.0, 900.0),
        (">15m", 900.0, f64::MAX),
    ];

    let mut result: HashMap<String, u32> = HashMap::new();
    for &val in raw {
        for &(label, lo, hi) in buckets {
            if val >= lo && val < hi {
                *result.entry(label.to_string()).or_insert(0) += 1;
                break;
            }
        }
    }
    result
}

pub(super) fn compute_response_time_stats(raw: &[f64]) -> (f64, f64) {
    if raw.is_empty() {
        return (0.0, 0.0);
    }

    let avg = raw.iter().sum::<f64>() / raw.len() as f64;

    let mut sorted = raw.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len().is_multiple_of(2) {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    (median, avg)
}

pub(super) fn compute_days_covered(range: &crate::agentic::insights::types::DateRange) -> u32 {
    let parse =
        |s: &str| -> Option<DateTime<Utc>> { DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)) };

    match (parse(&range.start), parse(&range.end)) {
        (Some(start), Some(end)) => {
            let diff = end.signed_duration_since(start);
            let days = diff.num_days().unsigned_abs() as u32;
            days.max(1)
        }
        _ => 1,
    }
}

/// Extract code change statistics from persistent turn data.
///
/// For Edit tool results: uses `old_end_line - start_line + 1` as lines removed
/// and `new_end_line - start_line + 1` as lines added, falling back to counting
/// newlines in `old_string`/`new_string`.
///
/// For Write tool: uses `lines_written`, falling back to counting newlines in
/// the tool input for older persisted sessions.
///
/// Per session, each distinct file path touched by Edit/Write contributes once to `languages_by_files`
/// according to [`language_name_for_path`].
pub(super) fn accumulate_code_stats_from_turns(base_stats: &mut BaseStats, turns: &[DialogTurnData]) {
    let mut modified_files: HashSet<String> = HashSet::new();

    for turn in turns {
        for round in &turn.model_rounds {
            for ti in &round.tool_items {
                let Some(ref result_data) = ti.tool_result else {
                    continue;
                };
                if !result_data.success {
                    continue;
                }

                match ti.tool_name.as_str() {
                    "Edit" => {
                        let result = &result_data.result;

                        if let Some(fp) = result.get("file_path").and_then(|v| v.as_str()) {
                            modified_files.insert(fp.to_string());
                        }

                        let (lines_removed, lines_added) = if let (Some(start), Some(old_end), Some(new_end)) = (
                            result.get("start_line").and_then(|v| v.as_u64()),
                            result.get("old_end_line").and_then(|v| v.as_u64()),
                            result.get("new_end_line").and_then(|v| v.as_u64()),
                        ) {
                            let removed = old_end.saturating_sub(start) + 1;
                            let added = new_end.saturating_sub(start) + 1;
                            (removed as usize, added as usize)
                        } else {
                            let old_lines = result
                                .get("old_string")
                                .and_then(|v| v.as_str())
                                .map(|s| s.lines().count().max(1))
                                .unwrap_or(0);
                            let new_lines = result
                                .get("new_string")
                                .and_then(|v| v.as_str())
                                .map(|s| s.lines().count().max(1))
                                .unwrap_or(0);
                            (old_lines, new_lines)
                        };

                        base_stats.total_lines_removed += lines_removed;
                        base_stats.total_lines_added += lines_added;
                    }
                    "Write" => {
                        let result = &result_data.result;

                        if let Some(fp) = result.get("file_path").and_then(|v| v.as_str()) {
                            modified_files.insert(fp.to_string());
                        }

                        if let Some(lines_written) = result.get("lines_written").and_then(|v| v.as_u64()) {
                            base_stats.total_lines_added += lines_written as usize;
                        } else if let Some(content) = ti.tool_call.input.get("content").and_then(|v| v.as_str()) {
                            base_stats.total_lines_added += content.lines().count().max(1);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    for path in &modified_files {
        if let Some(lang) = language_name_for_path(path) {
            *base_stats.languages_by_files.entry(lang.to_string()).or_insert(0) += 1;
        }
    }

    base_stats.total_files_modified += modified_files.len();
}

/// Infer a language label from a file path (extension or well-known filename).
pub(super) fn language_name_for_path(path: &str) -> Option<&'static str> {
    let p = Path::new(path);
    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
        match name.to_ascii_lowercase().as_str() {
            "dockerfile" | "containerfile" => return Some("Dockerfile"),
            "makefile" | "gnumakefile" => return Some("Makefile"),
            "cargo.toml" | "cargo.lock" => return Some("Rust"),
            _ => {}
        }
    }
    let ext = p.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" | "pyi" | "pyw" => "Python",
        "rs" => "Rust",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "cs" => "C#",
        "cpp" | "cc" | "cxx" | "hpp" => "C/C++",
        "c" | "h" => "C/C++",
        "rb" => "Ruby",
        "php" => "PHP",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "md" | "mdx" => "Markdown",
        "json" | "jsonc" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "html" | "htm" => "HTML",
        "css" | "scss" | "sass" | "less" => "CSS",
        "sh" | "bash" | "zsh" | "fish" => "Shell",
        "ps1" => "PowerShell",
        "sql" => "SQL",
        "gradle" => "Gradle",
        "properties" => "Properties",
        _ => return None,
    })
}
