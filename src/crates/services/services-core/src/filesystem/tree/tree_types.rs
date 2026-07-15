//! File-tree and search DTOs used by [`super::tree`]'s facade and impls.
//!
//! Pure data shapes (structs/enums) plus their trivial constructors and
//! `Default` impls. No I/O, no async. Sibling to [`super::tree_progress`],
//! [`super::tree_build`] and [`super::tree_search`].

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    pub children: Option<Vec<FileTreeNode>>,
    pub size: Option<u64>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    pub extension: Option<String>,

    pub depth: Option<u32>,
    pub is_symlink: Option<bool>,
    pub permissions: Option<String>,
    pub mime_type: Option<String>,
    pub git_status: Option<String>,
}

impl FileTreeNode {
    pub fn new(id: String, name: String, path: String, is_directory: bool) -> Self {
        Self {
            id,
            name,
            path,
            is_directory,
            children: None,
            size: None,
            last_modified: None,
            extension: None,
            depth: None,
            is_symlink: None,
            permissions: None,
            mime_type: None,
            git_status: None,
        }
    }

    pub fn with_metadata(mut self, size: Option<u64>, last_modified: Option<String>) -> Self {
        self.size = size;
        self.last_modified = last_modified;
        self
    }

    pub fn with_extension(mut self, extension: Option<String>) -> Self {
        self.extension = extension;
        self
    }

    pub fn with_children(mut self, children: Vec<FileTreeNode>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn with_enhanced_info(
        mut self,
        is_symlink: bool,
        permissions: Option<String>,
        mime_type: Option<String>,
        git_status: Option<String>,
    ) -> Self {
        self.is_symlink = Some(is_symlink);
        self.permissions = permissions;
        self.mime_type = mime_type;
        self.git_status = git_status;
        self
    }
}

/// File tree build options
#[derive(Debug, Clone)]
pub struct FileTreeOptions {
    pub max_depth: Option<u32>,
    pub include_hidden: bool,
    pub include_git_info: bool,
    pub include_mime_types: bool,
    pub skip_patterns: Vec<String>,
    pub max_file_size_mb: Option<u64>,
    pub follow_symlinks: bool,
}

impl Default for FileTreeOptions {
    fn default() -> Self {
        Self {
            max_depth: Some(50),
            include_hidden: false,
            include_git_info: false,
            include_mime_types: false,
            skip_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".next".to_string(),
                ".nuxt".to_string(),
                ".cache".to_string(),
                "coverage".to_string(),
                "__pycache__".to_string(),
                ".vscode".to_string(),
                ".idea".to_string(),
            ],
            max_file_size_mb: Some(100),
            follow_symlinks: false,
        }
    }
}

/// File tree statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeStatistics {
    pub total_files: usize,
    pub total_directories: usize,
    pub total_size_bytes: u64,
    pub max_depth_reached: u32,
    pub file_type_counts: HashMap<String, usize>,
    pub large_files: Vec<(String, u64)>, // (path, size) for files > 10MB
    pub symlinks_count: usize,
    pub hidden_files_count: usize,
}

#[derive(Debug, Clone)]
pub struct FileNameSearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
    pub max_results: usize,
    pub include_directories: bool,
    pub cancel_flag: Option<Arc<AtomicBool>>,
}

impl Default for FileNameSearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            max_results: 10_000,
            include_directories: true,
            cancel_flag: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileContentSearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
    pub max_results: usize,
    pub max_file_size_bytes: u64,
    pub cancel_flag: Option<Arc<AtomicBool>>,
}

impl Default for FileContentSearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            max_results: 10_000,
            max_file_size_bytes: 10 * 1024 * 1024,
            cancel_flag: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchOutcome {
    pub results: Vec<FileSearchResult>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResultGroup {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub file_name_match: Option<FileSearchResult>,
    pub content_matches: Vec<FileSearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub match_type: SearchMatchType,
    pub line_number: Option<usize>,
    pub matched_content: Option<String>,
    pub preview_before: Option<String>,
    pub preview_inside: Option<String>,
    pub preview_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMatchType {
    FileName,
    Content,
}
