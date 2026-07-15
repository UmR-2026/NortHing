//! Public types and options for the local grep search engine.
//!
//! This sibling owns:
//! - [`OutputMode`] — how the search results are rendered (content / count / files-with-matches).
//! - [`ProgressCallback`] — observer hook invoked while the walker drains the tree.
//! - [`GrepOptions`] — user-facing builder for a single search invocation.
//! - [`RemoteGrepCommandRequest`] / [`GrepSearchResult`] — request/response DTOs.

use std::fmt;
use std::sync::Arc;

/// Output mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Content,
    FilesWithMatches,
    Count,
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "content" => Ok(OutputMode::Content),
            "count" => Ok(OutputMode::Count),
            "files_with_matches" => Ok(OutputMode::FilesWithMatches),
            _ => Err(format!("Unknown output mode: {}", s)),
        }
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputMode::Content => write!(f, "content"),
            OutputMode::Count => write!(f, "count"),
            OutputMode::FilesWithMatches => write!(f, "files_with_matches"),
        }
    }
}

/// Progress report callback type
pub type ProgressCallback = Arc<dyn Fn(usize, usize, usize) + Send + Sync>;

/// grep search options
#[derive(Debug, Clone)]
pub struct GrepOptions {
    /// Regular expression pattern
    pub pattern: String,
    /// Search path
    pub path: String,
    /// Whether to ignore case
    pub case_insensitive: bool,
    /// Whether to enable multiline mode
    pub multiline: bool,
    /// Output mode
    pub output_mode: OutputMode,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Context line count (sets both before and after)
    pub context: Option<usize>,
    /// Context lines before match
    pub before_context: Option<usize>,
    /// Context lines after match
    pub after_context: Option<usize>,
    /// Limit output lines/files
    pub head_limit: Option<usize>,
    /// Number of lines/files to skip before limiting output
    pub offset: usize,
    /// Glob pattern filters
    pub globs: Vec<String>,
    /// File type filter
    pub file_type: Option<String>,
    /// Prefer displaying paths relative to this base when possible
    pub display_base: Option<String>,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            path: String::from("."),
            case_insensitive: false,
            multiline: false,
            output_mode: OutputMode::Content,
            show_line_numbers: true,
            context: None,
            before_context: None,
            after_context: None,
            head_limit: None,
            offset: 0,
            globs: Vec::new(),
            file_type: None,
            display_base: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGrepCommandRequest {
    pub pattern: String,
    pub path: String,
    pub case_insensitive: bool,
    pub output_mode: OutputMode,
    pub show_line_numbers: bool,
    pub context: Option<usize>,
    pub before_context: Option<usize>,
    pub after_context: Option<usize>,
    pub glob_patterns: Vec<String>,
    pub file_type: Option<String>,
    pub head_limit: Option<usize>,
    pub offset: usize,
}

impl GrepOptions {
    /// Create a new GrepOptions with required pattern and path
    pub fn new(pattern: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    /// Set whether to ignore case
    pub fn case_insensitive(mut self, value: bool) -> Self {
        self.case_insensitive = value;
        self
    }

    /// Set whether to enable multiline mode
    pub fn multiline(mut self, value: bool) -> Self {
        self.multiline = value;
        self
    }

    /// Set output mode
    pub fn output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    /// Set whether to show line numbers
    pub fn show_line_numbers(mut self, value: bool) -> Self {
        self.show_line_numbers = value;
        self
    }

    /// Set context line count (sets both before and after)
    pub fn context(mut self, lines: usize) -> Self {
        self.context = Some(lines);
        self
    }

    /// Set context lines before match
    pub fn before_context(mut self, lines: usize) -> Self {
        self.before_context = Some(lines);
        self
    }

    /// Set context lines after match
    pub fn after_context(mut self, lines: usize) -> Self {
        self.after_context = Some(lines);
        self
    }

    /// Set output lines/files limit
    pub fn head_limit(mut self, limit: usize) -> Self {
        self.head_limit = Some(limit);
        self
    }

    /// Set glob pattern filter
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn globs(mut self, patterns: Vec<String>) -> Self {
        self.globs = patterns;
        self
    }

    /// Set file type filter
    pub fn file_type(mut self, ftype: impl Into<String>) -> Self {
        self.file_type = Some(ftype.into());
        self
    }

    pub fn display_base(mut self, base: impl Into<String>) -> Self {
        self.display_base = Some(base.into());
        self
    }
}

pub struct GrepSearchResult {
    pub file_count: usize,
    pub total_matches: usize,
    pub result_text: String,
    pub applied_limit: Option<usize>,
    pub applied_offset: Option<usize>,
}
