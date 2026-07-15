//! File-walk and path-related helpers shared by the grep engine.
//!
//! This sibling owns pure helpers — no I/O, no state — that operate on
//! filesystem paths and in-memory slices:
//! - [`is_vcs_path`] — recognize VCS directories to skip during the walk.
//! - [`modified_time`] — read a file's mtime, defaulting to UNIX_EPOCH on error.
//! - [`normalize_display_base`] / [`relativize_display_path`] — render paths
//!   relative to a chosen display base.
//! - [`apply_offset_limit`] — generic offset/limit slicing that also reports
//!   whether the offset and limit were actually applied.

use std::path::{Component, Path};
use std::time::SystemTime;

pub(super) const VCS_DIRECTORIES_TO_EXCLUDE: &[&str] = &[".git", ".svn", ".hg", ".bzr", ".jj", ".sl"];

pub(super) fn is_vcs_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if VCS_DIRECTORIES_TO_EXCLUDE
                    .iter()
                    .any(|excluded| name.to_string_lossy() == *excluded)
        )
    })
}

pub(super) fn modified_time(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

pub(super) fn normalize_display_base(base: &str) -> String {
    base.replace('\\', "/").trim_end_matches('/').to_string()
}

pub(super) fn relativize_display_path(path: &Path, display_base: Option<&str>) -> String {
    let normalized = path.display().to_string().replace('\\', "/");
    let Some(base) = display_base else {
        return normalized;
    };

    let normalized_base = normalize_display_base(base);
    if normalized == normalized_base {
        return ".".to_string();
    }

    if let Some(rest) = normalized.strip_prefix(&(normalized_base + "/")) {
        return rest.to_string();
    }

    normalized
}

pub(super) fn apply_offset_limit<T>(
    items: Vec<T>,
    limit: Option<usize>,
    offset: usize,
) -> (Vec<T>, Option<usize>, Option<usize>)
where
    T: Clone,
{
    let total_len = items.len();
    let sliced = match limit {
        Some(limit) => items.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
        None => items.into_iter().skip(offset).collect::<Vec<_>>(),
    };

    let applied_limit = match limit {
        Some(limit) if total_len.saturating_sub(offset) > limit => Some(limit),
        _ => None,
    };
    let applied_offset = if offset > 0 { Some(offset) } else { None };

    (sliced, applied_limit, applied_offset)
}
