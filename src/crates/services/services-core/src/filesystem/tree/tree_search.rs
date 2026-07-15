//! Search methods on [`FileTreeService`].
//!
//! Sibling to [`super::tree_build`] (which owns build/listing) and
//! [`super::tree_progress`] (which owns progress reporting).
//!
//! The walker uses the `ignore` crate for parallel tree traversal and the
//! `regex` crate for both literal and regex matching. File-result mutators
//! live in file-private helpers at the top of this module; the impl block
//! below contains the public entry points plus their immediate walker
//! glue.

use super::super::error::{FileSystemError, FileSystemResult};
use super::tree_progress::FileSearchProgressSink;
use super::tree_types::{
    FileContentSearchOptions, FileNameSearchOptions, FileSearchOutcome, FileSearchResult, FileSearchResultGroup,
    SearchMatchType,
};
use super::FileTreeService;
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::warn;

fn lock_search_results(
    results: &Arc<Mutex<Vec<FileSearchResult>>>,
) -> std::sync::MutexGuard<'_, Vec<FileSearchResult>> {
    match results.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("File search results mutex was poisoned, recovering lock");
            poisoned.into_inner()
        }
    }
}

fn cancellation_requested(cancel_flag: Option<&Arc<AtomicBool>>) -> bool {
    cancel_flag.map(|flag| flag.load(Ordering::Relaxed)).unwrap_or(false)
}

impl FileTreeService {
    pub async fn search_files(
        &self,
        root_path: &str,
        pattern: &str,
        search_content: bool,
    ) -> FileSystemResult<Vec<FileSearchResult>> {
        self.search_files_with_options(root_path, pattern, search_content, false, false, false)
            .await
    }

    pub async fn search_files_with_options(
        &self,
        root_path: &str,
        pattern: &str,
        search_content: bool,
        case_sensitive: bool,
        use_regex: bool,
        whole_word: bool,
    ) -> FileSystemResult<Vec<FileSearchResult>> {
        let filename_outcome = self
            .search_file_names(
                root_path,
                pattern,
                FileNameSearchOptions {
                    case_sensitive,
                    use_regex,
                    whole_word,
                    max_results: 10_000,
                    include_directories: true,
                    cancel_flag: None,
                },
            )
            .await?;
        let mut results = filename_outcome.results;

        if search_content && !filename_outcome.truncated && results.len() < 10_000 {
            let remaining = 10_000 - results.len();
            let mut content_outcome = self
                .search_file_contents(
                    root_path,
                    pattern,
                    FileContentSearchOptions {
                        case_sensitive,
                        use_regex,
                        whole_word,
                        max_results: remaining,
                        max_file_size_bytes: 10 * 1024 * 1024,
                        cancel_flag: None,
                    },
                )
                .await?;
            results.append(&mut content_outcome.results);
        }

        Ok(results)
    }

    pub async fn search_file_names(
        &self,
        root_path: &str,
        pattern: &str,
        options: FileNameSearchOptions,
    ) -> FileSystemResult<FileSearchOutcome> {
        self.search_file_names_with_progress(root_path, pattern, options, None)
            .await
    }

    pub async fn search_file_names_with_progress(
        &self,
        root_path: &str,
        pattern: &str,
        options: FileNameSearchOptions,
        progress_sink: Option<Arc<dyn FileSearchProgressSink>>,
    ) -> FileSystemResult<FileSearchOutcome> {
        let root_path_buf = PathBuf::from(root_path);

        if !root_path_buf.exists() {
            return Err(FileSystemError::service("Directory does not exist".to_string()));
        }

        let matcher = Arc::new(Self::compile_search_regex(
            pattern,
            options.case_sensitive,
            options.use_regex,
            options.whole_word,
        )?);
        let results = Arc::new(Mutex::new(Vec::new()));
        let should_stop = Arc::new(AtomicBool::new(false));
        let limit_reached = Arc::new(AtomicBool::new(false));
        let cancel_flag = options.cancel_flag.clone();
        let include_directories = options.include_directories;
        let max_results = options.max_results.max(1);
        let progress_sink_for_walker = progress_sink.clone();

        let walker = Self::build_search_walker(&root_path_buf);

        walker.run(|| {
            let matcher = Arc::clone(&matcher);
            let results = Arc::clone(&results);
            let should_stop = Arc::clone(&should_stop);
            let limit_reached = Arc::clone(&limit_reached);
            let root_path_buf = root_path_buf.clone();
            let cancel_flag = cancel_flag.clone();
            let progress_sink = progress_sink_for_walker.clone();

            Box::new(move |entry| {
                if should_stop.load(Ordering::Relaxed) || cancellation_requested(cancel_flag.as_ref()) {
                    should_stop.store(true, Ordering::Relaxed);
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => return ignore::WalkState::Continue,
                };

                let path = entry.path();
                if path == root_path_buf {
                    return ignore::WalkState::Continue;
                }

                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let file_type = entry.file_type();

                if file_type.map(|kind| kind.is_dir()).unwrap_or(false) {
                    if Self::should_skip_directory_static(&file_name) {
                        return ignore::WalkState::Skip;
                    }

                    if include_directories
                        && matcher.is_match(&file_name)
                        && !Self::push_search_result_group(
                            &results,
                            &should_stop,
                            &limit_reached,
                            max_results,
                            progress_sink.as_ref(),
                            vec![FileSearchResult {
                                path: path.to_string_lossy().to_string(),
                                name: file_name,
                                is_directory: true,
                                match_type: SearchMatchType::FileName,
                                line_number: None,
                                matched_content: None,
                                preview_before: None,
                                preview_inside: None,
                                preview_after: None,
                            }],
                        )
                    {
                        return ignore::WalkState::Quit;
                    }

                    return ignore::WalkState::Continue;
                }

                if !file_type.map(|kind| kind.is_file()).unwrap_or(false) {
                    return ignore::WalkState::Continue;
                }

                if Self::should_skip_file_static(&file_name) || Self::is_binary_file_static(&file_name) {
                    return ignore::WalkState::Continue;
                }

                if matcher.is_match(&file_name)
                    && !Self::push_search_result_group(
                        &results,
                        &should_stop,
                        &limit_reached,
                        max_results,
                        progress_sink.as_ref(),
                        vec![FileSearchResult {
                            path: path.to_string_lossy().to_string(),
                            name: file_name,
                            is_directory: false,
                            match_type: SearchMatchType::FileName,
                            line_number: None,
                            matched_content: None,
                            preview_before: None,
                            preview_inside: None,
                            preview_after: None,
                        }],
                    )
                {
                    return ignore::WalkState::Quit;
                }

                ignore::WalkState::Continue
            })
        });

        if let Some(progress_sink) = progress_sink {
            progress_sink.flush();
        }

        let final_results = lock_search_results(&results).clone();
        Ok(FileSearchOutcome {
            results: final_results,
            truncated: limit_reached.load(Ordering::Relaxed),
        })
    }

    pub async fn search_file_contents(
        &self,
        root_path: &str,
        pattern: &str,
        options: FileContentSearchOptions,
    ) -> FileSystemResult<FileSearchOutcome> {
        self.search_file_contents_with_progress(root_path, pattern, options, None)
            .await
    }

    pub async fn search_file_contents_with_progress(
        &self,
        root_path: &str,
        pattern: &str,
        options: FileContentSearchOptions,
        progress_sink: Option<Arc<dyn FileSearchProgressSink>>,
    ) -> FileSystemResult<FileSearchOutcome> {
        let root_path_buf = PathBuf::from(root_path);

        if !root_path_buf.exists() {
            return Err(FileSystemError::service("Directory does not exist".to_string()));
        }

        let matcher = Arc::new(Self::compile_search_regex(
            pattern,
            options.case_sensitive,
            options.use_regex,
            options.whole_word,
        )?);
        let results = Arc::new(Mutex::new(Vec::new()));
        let should_stop = Arc::new(AtomicBool::new(false));
        let limit_reached = Arc::new(AtomicBool::new(false));
        let cancel_flag = options.cancel_flag.clone();
        let max_results = options.max_results.max(1);
        let max_file_size_bytes = options.max_file_size_bytes;
        let progress_sink_for_walker = progress_sink.clone();

        let walker = Self::build_search_walker(&root_path_buf);

        walker.run(|| {
            let matcher = Arc::clone(&matcher);
            let results = Arc::clone(&results);
            let should_stop = Arc::clone(&should_stop);
            let limit_reached = Arc::clone(&limit_reached);
            let root_path_buf = root_path_buf.clone();
            let cancel_flag = cancel_flag.clone();
            let progress_sink = progress_sink_for_walker.clone();

            Box::new(move |entry| {
                if should_stop.load(Ordering::Relaxed) || cancellation_requested(cancel_flag.as_ref()) {
                    should_stop.store(true, Ordering::Relaxed);
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => return ignore::WalkState::Continue,
                };

                let path = entry.path();
                if path == root_path_buf {
                    return ignore::WalkState::Continue;
                }

                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let file_type = entry.file_type();

                if file_type.map(|kind| kind.is_dir()).unwrap_or(false) {
                    return if Self::should_skip_directory_static(&file_name) {
                        ignore::WalkState::Skip
                    } else {
                        ignore::WalkState::Continue
                    };
                }

                if !file_type.map(|kind| kind.is_file()).unwrap_or(false) {
                    return ignore::WalkState::Continue;
                }

                if Self::should_skip_file_static(&file_name) || Self::is_binary_file_static(&file_name) {
                    return ignore::WalkState::Continue;
                }

                if let Ok(metadata) = path.metadata() {
                    if metadata.len() > max_file_size_bytes {
                        return ignore::WalkState::Continue;
                    }
                }

                if let Err(error) = Self::search_file_content_lines(
                    path,
                    &file_name,
                    matcher.as_ref(),
                    &results,
                    max_results,
                    &should_stop,
                    &limit_reached,
                    cancel_flag.as_ref(),
                    progress_sink.as_ref(),
                ) {
                    warn!("Failed to search file content {}: {}", path.display(), error);
                }

                if should_stop.load(Ordering::Relaxed) {
                    ignore::WalkState::Quit
                } else {
                    ignore::WalkState::Continue
                }
            })
        });

        if let Some(progress_sink) = progress_sink {
            progress_sink.flush();
        }

        let final_results = lock_search_results(&results).clone();
        Ok(FileSearchOutcome {
            results: final_results,
            truncated: limit_reached.load(Ordering::Relaxed),
        })
    }

    fn build_search_walker(root_path: &Path) -> ignore::WalkParallel {
        WalkBuilder::new(root_path)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .threads(
                std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(1)
                    .min(8),
            )
            .build_parallel()
    }

    fn compile_search_regex(
        pattern: &str,
        case_sensitive: bool,
        use_regex: bool,
        whole_word: bool,
    ) -> FileSystemResult<Regex> {
        let search_pattern = if use_regex {
            pattern.to_string()
        } else if whole_word {
            format!(r"\b{}\b", regex::escape(pattern))
        } else {
            regex::escape(pattern)
        };

        RegexBuilder::new(&search_pattern)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|error| FileSystemError::service(format!("Invalid regex pattern: {}", error)))
    }

    fn take_first_chars(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }

        let mut end_index = text.len();
        for (char_count, (byte_index, _)) in text.char_indices().enumerate() {
            if char_count == max_chars {
                end_index = byte_index;
                break;
            }
        }

        text[..end_index].to_string()
    }

    fn left_truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
        let total_chars = text.chars().count();
        if total_chars <= max_chars {
            return text.to_string();
        }

        if max_chars <= 1 {
            return "\u{2026}".to_string();
        }

        let keep_chars = max_chars - 1;
        let start_index = text
            .char_indices()
            .nth(total_chars.saturating_sub(keep_chars))
            .map(|(index, _)| index)
            .unwrap_or(0);

        format!("\u{2026}{}", &text[start_index..])
    }

    fn build_content_match_preview(line: &str, matcher: &Regex) -> (Option<String>, Option<String>, Option<String>) {
        const MAX_PREVIEW_CHARS: usize = 250;
        const MAX_PREVIEW_BEFORE_CHARS: usize = 26;

        let Some(found_match) = matcher.find(line) else {
            return (None, None, None);
        };

        let full_before = &line[..found_match.start()];
        let before = Self::left_truncate_with_ellipsis(full_before, MAX_PREVIEW_BEFORE_CHARS);

        let mut chars_remaining = MAX_PREVIEW_CHARS.saturating_sub(before.chars().count());
        let mut inside = Self::take_first_chars(found_match.as_str(), chars_remaining);
        chars_remaining = chars_remaining.saturating_sub(inside.chars().count());
        let after = Self::take_first_chars(&line[found_match.end()..], chars_remaining);

        if inside.is_empty() {
            inside = found_match.as_str().to_string();
        }

        (Some(before), Some(inside), Some(after))
    }

    fn build_search_result_group(results: Vec<FileSearchResult>) -> Option<FileSearchResultGroup> {
        let first = results.first()?.clone();
        let file_name_match = results
            .iter()
            .find(|result| matches!(result.match_type, SearchMatchType::FileName))
            .cloned();
        let content_matches = results
            .iter()
            .filter(|result| matches!(result.match_type, SearchMatchType::Content))
            .cloned()
            .collect();

        Some(FileSearchResultGroup {
            path: first.path,
            name: first.name,
            is_directory: first.is_directory,
            file_name_match,
            content_matches,
        })
    }

    fn push_search_result_group(
        results: &Arc<Mutex<Vec<FileSearchResult>>>,
        should_stop: &Arc<AtomicBool>,
        limit_reached: &Arc<AtomicBool>,
        max_results: usize,
        progress_sink: Option<&Arc<dyn FileSearchProgressSink>>,
        group_results: Vec<FileSearchResult>,
    ) -> bool {
        if group_results.is_empty() {
            return true;
        }

        let mut results_guard = lock_search_results(results);
        if results_guard.len() >= max_results {
            should_stop.store(true, Ordering::Relaxed);
            limit_reached.store(true, Ordering::Relaxed);
            return false;
        }

        let remaining_capacity = max_results.saturating_sub(results_guard.len());
        if remaining_capacity == 0 {
            should_stop.store(true, Ordering::Relaxed);
            limit_reached.store(true, Ordering::Relaxed);
            return false;
        }

        let accepted_results = if group_results.len() > remaining_capacity {
            limit_reached.store(true, Ordering::Relaxed);
            group_results.into_iter().take(remaining_capacity).collect::<Vec<_>>()
        } else {
            group_results
        };

        results_guard.extend(accepted_results.iter().cloned());
        if results_guard.len() >= max_results {
            should_stop.store(true, Ordering::Relaxed);
            limit_reached.store(true, Ordering::Relaxed);
        }

        drop(results_guard);
        if let (Some(progress_sink), Some(group)) = (progress_sink, Self::build_search_result_group(accepted_results)) {
            progress_sink.report(group);
        }

        !should_stop.load(Ordering::Relaxed)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_file_content_lines(
        path: &Path,
        file_name: &str,
        matcher: &Regex,
        results: &Arc<Mutex<Vec<FileSearchResult>>>,
        max_results: usize,
        should_stop: &Arc<AtomicBool>,
        limit_reached: &Arc<AtomicBool>,
        cancel_flag: Option<&Arc<AtomicBool>>,
        progress_sink: Option<&Arc<dyn FileSearchProgressSink>>,
    ) -> FileSystemResult<()> {
        if should_stop.load(Ordering::Relaxed) || cancellation_requested(cancel_flag) {
            should_stop.store(true, Ordering::Relaxed);
            return Ok(());
        }

        let file =
            File::open(path).map_err(|error| FileSystemError::service(format!("Failed to open file: {}", error)))?;
        let reader = BufReader::new(file);
        let mut matched_results = Vec::new();

        for (index, line_result) in reader.split(b'\n').enumerate() {
            if should_stop.load(Ordering::Relaxed) || cancellation_requested(cancel_flag) {
                should_stop.store(true, Ordering::Relaxed);
                return Ok(());
            }

            let line_bytes =
                line_result.map_err(|error| FileSystemError::service(format!("Failed to read file: {}", error)))?;
            let line = String::from_utf8_lossy(&line_bytes).trim_end_matches('\r').to_string();

            if !matcher.is_match(&line) {
                continue;
            }

            let (preview_before, preview_inside, preview_after) = Self::build_content_match_preview(&line, matcher);

            matched_results.push(FileSearchResult {
                path: path.to_string_lossy().to_string(),
                name: file_name.to_string(),
                is_directory: false,
                match_type: SearchMatchType::Content,
                line_number: Some(index + 1),
                matched_content: Some(line),
                preview_before,
                preview_inside,
                preview_after,
            });

            if matched_results.len() >= max_results {
                break;
            }
        }

        if !Self::push_search_result_group(
            results,
            should_stop,
            limit_reached,
            max_results,
            progress_sink,
            matched_results,
        ) {
            return Ok(());
        }

        Ok(())
    }

    fn should_skip_directory_static(file_name: &str) -> bool {
        matches!(
            file_name,
            "node_modules"
                | ".git"
                | ".svn"
                | ".hg"
                | "target"
                | "build"
                | "dist"
                | "out"
                | ".next"
                | ".nuxt"
                | ".cache"
                | "__pycache__"
                | "coverage"
                | ".idea"
                | ".vscode"
        )
    }

    fn should_skip_file_static(file_name: &str) -> bool {
        matches!(file_name, ".DS_Store" | "Thumbs.db")
    }

    fn is_binary_file_static(file_name: &str) -> bool {
        let binary_extensions = [
            ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".svg", ".webp", ".mp4", ".avi", ".mov", ".wmv", ".flv",
            ".mkv", ".mp3", ".wav", ".flac", ".aac", ".ogg", ".zip", ".tar", ".gz", ".7z", ".rar", ".bz2", ".pdf",
            ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".woff", ".woff2", ".ttf", ".otf", ".eot", ".exe",
            ".dll", ".so", ".dylib", ".bin", ".pyc", ".class", ".o", ".a", ".lib",
        ];

        let lower_name = file_name.to_lowercase();
        binary_extensions.iter().any(|ext| lower_name.ends_with(ext))
    }
}
