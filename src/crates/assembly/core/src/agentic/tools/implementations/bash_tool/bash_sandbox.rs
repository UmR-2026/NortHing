use super::bash_types::ResolvedShell;
use crate::agentic::tools::framework::ToolUseContext;
use crate::agentic::tools::implementations::shell_safety;
use crate::service::config::global::get_global_config_service;
use crate::util::errors::{NortHingError, NortHingResult};
use terminal_core::shell::{ShellDetector, ShellType};
use tool_runtime::shell::{bash_noninteractive_env, detect_osascript_im_app, detect_osascript_keystroke_non_ascii};
use tracing::{debug, error};

pub(crate) fn noninteractive_env() -> std::collections::HashMap<String, String> {
    bash_noninteractive_env()
}

pub(crate) async fn resolve_shell() -> ResolvedShell {
    try_configured_shell().await.unwrap_or_else(system_default_shell)
}

async fn try_configured_shell() -> Option<ResolvedShell> {
    let config_service = get_global_config_service().await.ok()?;
    let shell_str: String = config_service
        .config::<String>(Some("terminal.default_shell"))
        .await
        .ok()
        .filter(|s| !s.is_empty())?;

    let parsed = ShellType::from_executable(&shell_str);
    if parsed.supports_integration() {
        Some(ResolvedShell {
            shell_type: Some(parsed.clone()),
            display_name: parsed.name().to_string(),
        })
    } else {
        debug!(
            "Configured shell '{}' does not support integration, using system default",
            shell_str
        );
        None
    }
}

fn system_default_shell() -> ResolvedShell {
    let detected = ShellDetector::default_shell();
    ResolvedShell {
        shell_type: None,
        display_name: detected.display_name,
    }
}

pub(crate) fn cancellation_requested(context: &ToolUseContext) -> bool {
    context.cancellation_token().is_some_and(|token| token.is_cancelled())
}

pub(crate) fn cancellation_error(stage: &str) -> NortHingError {
    NortHingError::cancelled(format!("Bash tool execution cancelled {}", stage))
}

pub(crate) fn command_needs_light_checkpoint(command: &str) -> bool {
    let command = command.trim().to_ascii_lowercase();
    let mutating_prefixes = [
        "rm ",
        "rmdir ",
        "del ",
        "erase ",
        "move ",
        "mv ",
        "cp ",
        "git reset",
        "git clean",
        "git checkout",
        "git switch",
        "git merge",
        "git rebase",
        "git pull",
        "git stash",
        "git commit",
        "cargo fmt",
        "cargo fix",
        "rustfmt",
        "prettier --write",
    ];

    mutating_prefixes.iter().any(|prefix| command.starts_with(prefix))
        || command.contains(" --fix")
        || command.contains(" > ")
        || command.contains(" >> ")
}
