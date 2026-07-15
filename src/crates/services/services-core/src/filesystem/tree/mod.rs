//! File tree service
//!
//! Provides file tree building, directory scanning, and file search.
//!
//! Implementation is split across sibling modules:
//!
//! - [`tree_types`]: shared DTOs (tree nodes, search options/results).
//! - [`tree_progress`]: the
//!   [`FileSearchProgressSink`](tree_progress::FileSearchProgressSink)
//!   trait and the
//!   [`BatchedFileSearchProgressSink`](tree_progress::BatchedFileSearchProgressSink)
//!   impl.
//! - [`tree_build`]: build/listing methods on [`FileTreeService`].
//! - [`tree_search`]: search methods on [`FileTreeService`].
//!
//! This facade re-exports the public surface so external callers can keep
//! using `filesystem::tree::FileTreeService` and friends unchanged.

mod tree_build;
mod tree_progress;
mod tree_search;
mod tree_types;

pub use tree_progress::{BatchedFileSearchProgressSink, FileSearchProgressSink};
pub use tree_types::{
    FileContentSearchOptions, FileNameSearchOptions, FileSearchOutcome, FileSearchResult, FileSearchResultGroup,
    FileTreeNode, FileTreeOptions, FileTreeStatistics, SearchMatchType,
};

/// File-tree service aggregator.
///
/// Holds build [`FileTreeOptions`](tree_types::FileTreeOptions) and
/// exposes build-time and search-time operations through the [`tree_build`]
/// and [`tree_search`] sibling impl blocks. The constructor and the
/// configuration accessor live in this facade so callers can do
/// `FileTreeService::new(opts)` without knowing the split layout.
pub struct FileTreeService {
    options: FileTreeOptions,
}

impl FileTreeService {
    pub fn new(options: FileTreeOptions) -> Self {
        Self { options }
    }

    /// Read-only accessor for the embedded build options. Sibling impl
    /// blocks in [`tree_build`] and [`tree_search`] use this instead of
    /// reaching into the private `options` field directly, which keeps
    /// the field encapsulated at the facade level.
    pub(super) fn options(&self) -> &FileTreeOptions {
        &self.options
    }
}

impl Default for FileTreeService {
    fn default() -> Self {
        Self::new(FileTreeOptions::default())
    }
}
