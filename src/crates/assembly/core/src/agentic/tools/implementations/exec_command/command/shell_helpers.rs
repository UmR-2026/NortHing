//! Free helper functions shared between the local and remote command paths.
//!
//! These functions are pure (no I/O) and do not depend on `ExecCommandTool`'s
//! `&self`. They are kept in a separate sibling so the local + remote
//! callers can `use super::shell_helpers::*` without dragging each other's
//! shell-specific logic.

use std::collections::HashMap;

use super::types::RemoteShell;
use terminal_core::ShellType;

/// Wrap a shell argument in single quotes and escape inner single quotes
/// using the standard POSIX `'\''` idiom. Used for both workdir and command
/// arguments in the remote login shell wrapper.
pub(super) fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Parse the multi-line stdout returned by `resolve_remote_shell`'s probe
/// command and return the first plausible shell path. The probe command
/// emits one path per line; the first line that looks like an absolute,
/// non-control-character path wins.
pub(super) fn parse_remote_shell_probe_output(stdout: &str) -> Option<RemoteShell> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| is_plausible_remote_shell_path(line))
        .map(|path| RemoteShell {
            path: path.to_string(),
            shell_type: ShellType::from_executable(path),
        })
}

/// Reject probe-output lines that obviously are not shell paths: missing
/// leading slash, contains NUL, or contains control characters other than
/// tab.
pub(super) fn is_plausible_remote_shell_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains('\0') && path.chars().all(|ch| !ch.is_control() || ch == '\t')
}

/// Render an environment variable map as a single shell-safe word list
/// (`'K=V' 'K=V' ...`). Keys are sorted to keep the resulting command
/// deterministic for tests and snapshots.
pub(super) fn remote_command_env_words(env: HashMap<String, String>) -> String {
    let mut env: Vec<_> = env.into_iter().collect();
    env.sort_by(|(left, _), (right, _)| left.cmp(right));
    env.into_iter()
        .map(|(key, value)| shell_escape(&format!("{key}={value}")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Login-shell flags passed to the resolved remote shell. Kept as
/// `&["-lc"]` (no interactive startup, run as login shell) so the model can
/// invoke POSIX dotfiles / PATH setup before the user command.
pub(super) fn remote_shell_login_args() -> &'static [&'static str] {
    &["-lc"]
}

/// Current wall-clock seconds since the UNIX epoch, with `0` as the fallback
/// when the system clock is somehow before the epoch. Used for the
/// `BackgroundCommandLifecycleInfo` timestamp field.
pub(super) fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
