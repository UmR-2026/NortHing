//! `Grep` tool — facade for the split siblings.
//!
//! Originally a single 1000+ line file, the implementation is now distributed
//! across the following sub-modules:
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`filter`] | Module-private constants (`DEFAULT_HEAD_LIMIT`) and static input-parsing helpers on `GrepTool` (head_limit, offset, glob patterns, display-base resolution). |
//! | [`options`] | `GrepOptions` builder for the local ripgrep execution path (`build_grep_options`). |
//! | [`remote`] | Remote workspace shell-based grep fallback (`call_remote`). |
//! | [`workspace`] | Indexed workspace-search request builder and output renderer (`build_workspace_search_request`, `format_workspace_search_output`) plus the standalone content/result line renderers. |
//! | [`local`] | Local ripgrep execution path with progress callback (`call_local`). |
//! | [`tool`] | `GrepTool` struct + `Default` impl + `Tool` trait implementation + thin `call_impl` dispatcher. |
//! | [`tests`] | Unit tests. |
//!
//! The public surface (`GrepTool`) is preserved exactly; only the internal
//! layout has changed.

mod filter;
mod local;
mod options;
mod remote;
mod tool;
mod workspace;

#[cfg(test)]
mod tests;

pub use tool::GrepTool;
