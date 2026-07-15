//! [`GrepSink`] implementation — receives matches from `grep_searcher` and
//! formats them according to the requested [`OutputMode`].
//!
//! The sink owns three pieces of shared state:
//! - `output` — accumulated formatted bytes (used for `Content` mode).
//! - `line_count` — number of lines written so far, used for `head_limit`.
//! - `match_count` — number of matches seen, exposed via [`GrepSink::get_match_count`].
//! - `last_line_number` — used to insert `--` separators between discontinuous
//!   context groups.

use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use grep_searcher::{Searcher, Sink, SinkContext, SinkMatch};
use tracing::warn;

use super::grep_filter::relativize_display_path;
use super::grep_types::OutputMode;

pub(super) const MAX_DISPLAY_COLUMNS: usize = 500;

/// Sink implementation for collecting search results
#[derive(Clone)]
pub(super) struct GrepSink {
    pub(super) output_mode: OutputMode,
    pub(super) show_line_numbers: bool,
    pub(super) before_context: usize,
    pub(super) after_context: usize,
    pub(super) head_limit: Option<usize>,
    pub(super) current_file: PathBuf,
    pub(super) display_base: Option<String>,
    pub(super) output: Arc<Mutex<Vec<u8>>>,
    pub(super) line_count: Arc<Mutex<usize>>,
    pub(super) match_count: Arc<Mutex<usize>>,
    /// Last output line number, used to detect discontinuity
    pub(super) last_line_number: Arc<Mutex<Option<u64>>>,
}

fn lock_recover<'a, T>(mutex: &'a Mutex<T>, name: &str) -> std::sync::MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex poisoned in grep search: {}", name);
            poisoned.into_inner()
        }
    }
}

impl GrepSink {
    pub(super) fn new(
        output_mode: OutputMode,
        show_line_numbers: bool,
        before_context: usize,
        after_context: usize,
        head_limit: Option<usize>,
        current_file: PathBuf,
        display_base: Option<String>,
    ) -> Self {
        Self {
            output_mode,
            show_line_numbers,
            before_context,
            after_context,
            head_limit,
            current_file,
            display_base,
            output: Arc::new(Mutex::new(Vec::new())),
            line_count: Arc::new(Mutex::new(0)),
            match_count: Arc::new(Mutex::new(0)),
            last_line_number: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn output(&self) -> String {
        let output = lock_recover(&self.output, "output");
        String::from_utf8_lossy(&output).to_string()
    }

    pub(super) fn get_match_count(&self) -> usize {
        *lock_recover(&self.match_count, "match_count")
    }

    pub(super) fn should_stop(&self) -> bool {
        if let Some(limit) = self.head_limit {
            let count = *lock_recover(&self.line_count, "line_count");
            count >= limit
        } else {
            false
        }
    }

    pub(super) fn increment_line_count(&self) -> bool {
        let mut count = lock_recover(&self.line_count, "line_count");
        *count += 1;
        if let Some(limit) = self.head_limit {
            *count <= limit
        } else {
            true
        }
    }

    pub(super) fn write_line(&self, line: &[u8]) {
        if self.increment_line_count() {
            let mut output = lock_recover(&self.output, "output");
            output.extend_from_slice(line);
            output.push(b'\n');
        }
    }

    /// Check if separator (--) needs to be inserted before current line
    /// Insert when previous line and current line are not continuous (only when context is set)
    pub(super) fn check_and_write_separator(&self, current_line: u64) {
        // Only use separator when context is set (consistent with rg behavior)
        if self.before_context == 0 && self.after_context == 0 {
            return;
        }

        let mut last_line = lock_recover(&self.last_line_number, "last_line_number");
        if let Some(last) = *last_line {
            // If current line number is not continuous with previous line (difference > 1), insert separator
            if current_line > last + 1 {
                let mut output = lock_recover(&self.output, "output");
                output.extend_from_slice(b"--\n");
            }
        }
        *last_line = Some(current_line);
    }

    /// Format output line (rg style: only show line number and content, no path)
    pub(super) fn format_line(&self, line_number: u64, line: &[u8], is_match: bool) -> Vec<u8> {
        let mut line_str = String::from_utf8_lossy(line).trim_end().to_string();
        if line_str.chars().count() > MAX_DISPLAY_COLUMNS {
            line_str = format!(
                "{} [truncated]",
                line_str.chars().take(MAX_DISPLAY_COLUMNS).collect::<String>()
            );
        }
        let separator = if is_match { ":" } else { "-" };
        let path_prefix = relativize_display_path(&self.current_file, self.display_base.as_deref());

        if self.show_line_numbers {
            format!("{}{}{}:{}", path_prefix, separator, line_number, line_str).into_bytes()
        } else {
            format!("{}{}{}", path_prefix, separator, line_str).into_bytes()
        }
    }
}

impl Sink for GrepSink {
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if self.should_stop() {
            return Ok(false);
        }

        *lock_recover(&self.match_count, "match_count") += 1;

        match self.output_mode {
            OutputMode::Content => {
                let line_number = mat.line_number().unwrap_or(0);
                // Check if separator needs to be inserted
                self.check_and_write_separator(line_number);
                let formatted = self.format_line(line_number, mat.bytes(), true);
                self.write_line(&formatted);
            }
            OutputMode::FilesWithMatches => {
                return Ok(false); // Only need first match, then stop
            }
            OutputMode::Count => {
                // Count mode doesn't write here, handled uniformly at the end
            }
        }

        Ok(!self.should_stop())
    }

    fn context(&mut self, _searcher: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.should_stop() {
            return Ok(false);
        }

        // Only output context lines in content mode and when context is set
        if matches!(self.output_mode, OutputMode::Content) && (self.before_context > 0 || self.after_context > 0) {
            let line_number = ctx.line_number().unwrap_or(0);
            // Check if separator needs to be inserted
            self.check_and_write_separator(line_number);
            let formatted = self.format_line(line_number, ctx.bytes(), false);
            self.write_line(&formatted);
        }

        Ok(!self.should_stop())
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        Ok(!self.should_stop())
    }

    fn finish(&mut self, _searcher: &Searcher, _: &grep_searcher::SinkFinish) -> Result<(), Self::Error> {
        Ok(())
    }
}
