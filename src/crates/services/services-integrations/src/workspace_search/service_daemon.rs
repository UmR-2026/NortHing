//! Workspace search daemon binary resolution helpers.
//!
//! Free helpers used by [`super::service::new_with_hooks`] and
//! [`super::service::resolve_workspace_search_daemon_program_path`] to locate
//! the flashgrep daemon binary on disk. The public daemon binary API
//! (`workspace_search_daemon_*`) lives in [`super::service`].

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) fn resolve_daemon_program() -> Option<OsString> {
    super::service::resolve_workspace_search_daemon_program_path().map(PathBuf::into_os_string)
}

pub(super) fn daemon_binary_candidates(
    workspace_root: &Path,
    binary_names: &[&str],
    current_profile: Option<&str>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let mut push_candidate = |path: PathBuf| {
        if seen.insert(path.clone()) {
            candidates.push(path);
        }
    };

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            for binary_name in binary_names {
                push_candidate(parent.join(binary_name));
            }
            push_exe_relative_bundle_candidates(&mut push_candidate, parent, binary_names);
        }
    }

    for profile in current_profile.into_iter().chain(["debug", "release", "release-fast"]) {
        for binary_name in binary_names {
            push_candidate(workspace_root.join("target").join(profile).join(binary_name));
        }
    }

    candidates
}

fn push_exe_relative_bundle_candidates(
    push_candidate: &mut impl FnMut(PathBuf),
    exe_dir: &Path,
    binary_names: &[&str],
) {
    if cfg!(target_os = "macos") {
        for binary_name in binary_names {
            push_candidate(exe_dir.join("../Resources/flashgrep").join(binary_name));
        }
    }

    for binary_name in binary_names {
        push_candidate(exe_dir.join("flashgrep").join(binary_name));
        push_candidate(exe_dir.join("resources/flashgrep").join(binary_name));
    }

    if cfg!(target_os = "linux") {
        for binary_name in binary_names {
            push_candidate(exe_dir.join("../lib/northhing/flashgrep").join(binary_name));
            push_candidate(exe_dir.join("../share/northhing/flashgrep").join(binary_name));
            push_candidate(
                exe_dir
                    .join("../share/com.northhing.desktop/flashgrep")
                    .join(binary_name),
            );
        }
    }
}
