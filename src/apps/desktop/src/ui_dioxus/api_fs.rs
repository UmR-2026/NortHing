// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room workspace file tree & preview API (W9-6).
// Thin async wrappers over `northhing_core::kernel_facade()`.
//
// The facade enforces the workspace path fence (`..`, absolute paths, and
// symlink escapes are rejected before any IO); the UI can safely pass any
// workspace-relative string it rendered from the tree back to read.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::platform::KernelPlatformApi;

// Re-export the DTO so Dioxus callers can name it through `super::api`.
pub use northhing_kernel_api::platform::FileTreeEntryDto;

/// Lists the immediate children of `dir` (workspace-relative; empty string
/// means the workspace root), with optional bounded recursion.
///
/// `max_depth = Some(0)` is equivalent to `None`: only direct children are
/// returned. `Some(n)` requests recursive expansion capped at `n` levels.
pub async fn list_workspace_tree(dir: &str, max_depth: Option<u32>) -> Result<Vec<FileTreeEntryDto>, KernelError> {
    kernel_facade().list_workspace_tree(dir, max_depth).await
}

/// Reads a workspace-relative text file. The facade enforces a default and
/// a hard ceiling on `max_bytes`; binary and non-UTF-8 files return
/// `KernelError::Validation`. The returned string is intended for the
/// right-drawer preview pane only.
pub async fn read_workspace_file(path: &str, max_bytes: Option<u64>) -> Result<String, KernelError> {
    kernel_facade().read_workspace_file(path, max_bytes).await
}

/// Pure helper used by the UI for placeholder text — kept here so the same
/// formatting lives next to the API wrapper.
pub fn format_size_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    if bytes < KIB {
        format!("{bytes} B")
    } else if bytes < KIB * KIB {
        format!("{:.1} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / (KIB * KIB) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes_units() {
        assert_eq!(format_size_bytes(0), "0 B");
        assert_eq!(format_size_bytes(512), "512 B");
        assert_eq!(format_size_bytes(1024), "1.0 KB");
        assert_eq!(format_size_bytes(1536), "1.5 KB");
        assert_eq!(format_size_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_size_bytes(1024 * 1024 * 3 + 128 * 1024), "3.1 MB");
    }
}
