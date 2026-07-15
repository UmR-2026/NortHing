use std::path::PathBuf;

pub const MAX_MATCH_CONTEXTS: usize = 5;
pub const CONTEXT_LINES_BEFORE: usize = 2;
pub const CONTEXT_LINES_AFTER: usize = 2;
pub const NOT_FOUND_DIAGNOSTIC_SNIPPETS: usize = 1;
pub const NOT_FOUND_MIN_SUBSTRING_LEN: usize = 8;

/// Edit result, contains line number range information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditResult {
    /// Start line number of old_string/new_string (starts from 1)
    pub start_line: usize,
    /// End line number of old_string (starts from 1)
    pub old_end_line: usize,
    /// End line number of new_string after replacement (starts from 1)
    pub new_end_line: usize,
}

/// Result of applying an edit to in-memory content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyEditResult {
    pub new_content: String,
    pub match_count: usize,
    pub edit_result: EditResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLocalFileRequest {
    pub logical_path: String,
    pub resolved_path: PathBuf,
    pub old_string: String,
    pub new_string: String,
    pub replace_all: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLocalFileWithContentRequest {
    pub logical_path: String,
    pub resolved_path: PathBuf,
    pub current_content: String,
    pub old_string: String,
    pub new_string: String,
    pub replace_all: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLocalFileOutcome {
    pub new_content: String,
    pub match_count: usize,
    pub edit_result: EditResult,
}
