//! Local grep search engine — facade that wires together [`grep_types`],
//! [`grep_match`], [`grep_filter`], and [`grep_output`] into the public
//! `grep_search` entry point.
//!
//! Most users only need the public items re-exported at the top of this file;
//! see [`GrepOptions`] for the typical flow. The actual implementation is
//! delegated to the sibling modules in this directory.

use std::time::SystemTime;

use globset::{GlobBuilder, GlobMatcher};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use tracing::{debug, info, warn};

#[path = "grep_filter.rs"]
mod grep_filter;
#[path = "grep_match.rs"]
mod grep_match;
#[path = "grep_output.rs"]
mod grep_output;
#[path = "grep_types.rs"]
mod grep_types;

// Re-exports — keep the public API surface from the pre-split god-file stable.
pub use grep_output::{
    apply_offset_and_limit, build_remote_grep_command, count_remote_grep_matches, relativize_result_text,
    render_remote_grep_result_text,
};
pub use grep_types::{GrepOptions, GrepSearchResult, OutputMode, ProgressCallback, RemoteGrepCommandRequest};

use grep_filter::{apply_offset_limit, is_vcs_path, modified_time, relativize_display_path};
use grep_match::GrepSink;

/// Execute grep search
///
/// # Parameters
/// - `options`: Search options
/// - `progress_callback`: Progress callback (optional)
/// - `progress_interval_millis`: Progress report interval (milliseconds, optional, default 500)
///
/// # Returns
/// - `Ok((file_count, match_count, result_text))`: Number of matching files, number of matches, and result text
/// - `Err(error_message)`: Error message
///
/// # Example
/// ```ignore
/// use tool_runtime::search::{grep_search, GrepOptions, OutputMode};
///
/// let options = GrepOptions::new("pattern", "path/to/search")
///     .case_insensitive(true)
///     .context(2);
///
/// let result = grep_search(options, None, None);
/// ```
pub fn grep_search(
    options: GrepOptions,
    progress_callback: Option<ProgressCallback>,
    progress_interval_millis: Option<u128>,
) -> Result<GrepSearchResult, String> {
    let search_path = &options.path;

    // Validate that search path exists
    let path = std::path::Path::new(search_path);
    if !path.exists() {
        return Err(format!("Search path '{}' does not exist", search_path));
    }

    let before_context = options.before_context.unwrap_or(options.context.unwrap_or(0));
    let after_context = options.after_context.unwrap_or(options.context.unwrap_or(0));
    let pattern = &options.pattern;
    let case_insensitive = options.case_insensitive;
    let multiline = options.multiline;
    let output_mode = options.output_mode;
    let show_line_numbers = options.show_line_numbers;
    let head_limit = options.head_limit;
    let offset = options.offset;
    let file_type = options.file_type.as_deref();
    let display_base = options.display_base.clone();

    // Build regex matcher
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(case_insensitive)
        .multi_line(multiline)
        .dot_matches_new_line(multiline)
        .build(pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;

    // Build searcher
    let mut searcher_builder = SearcherBuilder::new();
    searcher_builder
        .line_number(true)
        .before_context(before_context)
        .after_context(after_context);

    if multiline {
        searcher_builder.multi_line(true);
    }

    let mut searcher = searcher_builder.build();

    // Build walker
    let mut walk_builder = WalkBuilder::new(search_path);
    walk_builder
        .hidden(false) // Include hidden files, closer to Claude's rg --hidden
        .ignore(true) // Use .gitignore
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    // Add file type filter
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults();

    types_builder
        .add("arkts", "*.ets")
        .map_err(|e| format!("Failed to add arkts type: {}", e))?;
    types_builder
        .add("json", "*.json5")
        .map_err(|e| format!("Failed to add json5 type: {}", e))?;

    if let Some(ftype) = file_type {
        // Check if type already exists
        let type_exists = types_builder.definitions().iter().any(|def| def.name() == ftype);

        if !type_exists {
            // Type doesn't exist, automatically add *.{ftype}
            let glob_pattern = format!("*.{}", ftype);
            types_builder
                .add(ftype, &glob_pattern)
                .map_err(|e| format!("Failed to add file type '{}': {}", ftype, e))?;
            debug!("Auto-added file type '{}' with glob '{}'", ftype, glob_pattern);
        }

        // User specified type, use user-specified type
        types_builder.select(ftype);
    } else {
        types_builder.select("all");
    }

    match types_builder.build() {
        Ok(types) => {
            walk_builder.types(types);
        }
        Err(e) => {
            return Err(format!("Invalid file type: {}", e));
        }
    }

    let walker = walk_builder.build();

    // Pre-build glob matcher
    let glob_matchers = options
        .globs
        .iter()
        .map(|glob| {
            GlobBuilder::new(glob)
                .build()
                .map(|compiled| compiled.compile_matcher())
                .map_err(|e| format!("Invalid glob pattern: {}", e))
        })
        .collect::<Result<Vec<GlobMatcher>, String>>()?;

    // Collect all results
    let mut content_lines = Vec::new();
    let mut total_matches = 0;
    let mut file_count = 0;
    let mut file_match_counts: Vec<(String, usize)> = Vec::new();
    let mut matched_files_with_mtime: Vec<(String, SystemTime)> = Vec::new();

    // Progress tracking
    let mut files_processed = 0;
    let mut last_progress_time = std::time::Instant::now();
    let progress_interval_millis = progress_interval_millis.unwrap_or(500);

    // Traverse files and search
    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();

                files_processed += 1;

                if last_progress_time.elapsed().as_millis() >= progress_interval_millis {
                    info!(
                        "Search progress: processed {} files, found {} matching files, total {} matches",
                        files_processed, file_count, total_matches
                    );

                    if let Some(ref callback) = progress_callback {
                        callback(files_processed, file_count, total_matches);
                    }

                    last_progress_time = std::time::Instant::now();
                }

                // Check if it's a file
                if !path.is_file() {
                    continue;
                }

                if is_vcs_path(path) {
                    continue;
                }

                if !glob_matchers.is_empty() && !glob_matchers.iter().any(|matcher| matcher.is_match(path)) {
                    continue;
                }

                let sink = GrepSink::new(
                    output_mode,
                    show_line_numbers,
                    before_context,
                    after_context,
                    None,
                    path.to_path_buf(),
                    display_base.clone(),
                );

                // Execute search
                if let Err(e) = searcher.search_path(&matcher, path, sink.clone()) {
                    warn!("Error searching file {}: {}", path.display(), e);
                    continue;
                }

                let file_matches = sink.get_match_count();
                if file_matches > 0 {
                    file_count += 1;
                    total_matches += file_matches;
                    match output_mode {
                        OutputMode::Content => {
                            let output = sink.output();
                            if !output.is_empty() {
                                content_lines.extend(
                                    output
                                        .lines()
                                        .filter(|line| !line.is_empty())
                                        .map(|line| line.to_string()),
                                );
                            }
                        }
                        OutputMode::FilesWithMatches => {
                            matched_files_with_mtime.push((
                                relativize_display_path(path, display_base.as_deref()),
                                modified_time(path),
                            ));
                        }
                        OutputMode::Count => {
                            file_match_counts
                                .push((relativize_display_path(path, display_base.as_deref()), file_matches));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error walking files: {}", e);
            }
        }
    }

    // Build result
    let result_text = match output_mode {
        OutputMode::Content => {
            let (lines, applied_limit, applied_offset) = apply_offset_limit(content_lines, head_limit, offset);
            if lines.is_empty() {
                format!("No matches found for pattern '{}'", pattern)
            } else {
                return Ok(GrepSearchResult {
                    file_count,
                    total_matches,
                    result_text: lines.join("\n").trim_end_matches('\n').to_string(),
                    applied_limit,
                    applied_offset,
                });
            }
        }
        OutputMode::FilesWithMatches => {
            matched_files_with_mtime.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let sorted_matches = matched_files_with_mtime
                .into_iter()
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            let (matches, applied_limit, applied_offset) = apply_offset_limit(sorted_matches, head_limit, offset);

            if matches.is_empty() {
                format!("No files found matching pattern '{}'", pattern)
            } else {
                return Ok(GrepSearchResult {
                    file_count,
                    total_matches,
                    result_text: matches.join("\n").trim_end_matches('\n').to_string(),
                    applied_limit,
                    applied_offset,
                });
            }
        }
        OutputMode::Count => {
            if file_match_counts.is_empty() {
                format!("No matches found for pattern '{}'", pattern)
            } else {
                let (count_list, applied_limit, applied_offset) =
                    apply_offset_limit(file_match_counts, head_limit, offset);

                let count_lines: Vec<String> = count_list
                    .iter()
                    .map(|(file, count)| format!("{}:{}", file, count))
                    .collect();

                return Ok(GrepSearchResult {
                    file_count,
                    total_matches,
                    result_text: format!(
                        "Total {} matches in {} files:\n{}",
                        total_matches,
                        count_list.len(),
                        count_lines.join("\n")
                    )
                    .trim_end_matches('\n')
                    .to_string(),
                    applied_limit,
                    applied_offset,
                });
            }
        }
    };

    Ok(GrepSearchResult {
        file_count,
        total_matches,
        result_text: result_text.trim_end_matches('\n').to_string(),
        applied_limit: None,
        applied_offset: if offset > 0 { Some(offset) } else { None },
    })
}

#[cfg(test)]
mod tests {
    use super::{grep_search, GrepOptions, OutputMode};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("northhing-grep-search-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn truncates_very_long_output_lines() {
        let root = make_temp_dir("truncate");
        let file_path = root.join("sample.txt");
        let long_line = "a".repeat(600);
        fs::write(&file_path, format!("{long_line}\n")).unwrap();

        let result = grep_search(
            GrepOptions::new("a+", root.to_string_lossy().to_string())
                .output_mode(OutputMode::Content)
                .show_line_numbers(true)
                .head_limit(10),
            None,
            None,
        )
        .unwrap();

        assert!(result.result_text.contains("[truncated]"));

        let _ = fs::remove_dir_all(root);
    }
}
