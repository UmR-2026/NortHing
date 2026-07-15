use super::{PromptBuilder, PromptBuilderContext, PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK, PLACEHOLDER_SESSION_ID};
use crate::agentic::tools::implementations::ExecCommandTool;
use crate::service::agent_memory::build_workspace_agent_memory_prompt;
use crate::service::bootstrap::build_workspace_persona_prompt;
use crate::service::config::get_app_language_code;
use crate::service::config::global::GlobalConfigManager;
use crate::service::i18n::LocaleId;
use crate::util::errors::{NortHingError, NortHingResult};
use std::env;
use tracing::{debug, warn};

impl PromptBuilder {
    pub(super) fn local_exec_shell_runtime_guidance(shell_type: &str) -> &'static [&'static str] {
        match shell_type {
            "powershell" | "pwsh" => &[
                "- For inline Python or other embedded scripts, prefer PowerShell-friendly forms such as `@'\\nprint(\"Hello\")\\n'@ | python -` instead of heavily nested quoting.",
                "- In PowerShell, the escape character is the backtick (`), not backslash. `\\\"` is not a reliable way to escape a double quote for the shell.",
                "- For environment variables, process filtering, and file traversal, prefer native PowerShell cmdlets and syntax over shell-specific Unix patterns.",
                "- Avoid mixing PowerShell with `cmd.exe` or bash in the same command unless cross-shell behavior is explicitly required.",
            ],
            _ => &[],
        }
    }

    fn push_local_exec_shell_runtime_context(
        lines: &mut Vec<String>,
        shell_display_name: &str,
        shell_type: &str,
        shell_invocation: &str,
    ) {
        lines.push(format!(
            "- ExecCommand shell: {shell_display_name} ({shell_type}), invoked as {shell_invocation}."
        ));
        lines.extend(
            Self::local_exec_shell_runtime_guidance(shell_type)
                .iter()
                .map(|line| (*line).to_string()),
        );
    }

    fn push_runtime_context_section(lines: &mut Vec<String>, title: &str, section_lines: Vec<String>) {
        if section_lines.is_empty() {
            return;
        }

        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("## {title}"));
        lines.extend(section_lines);
    }

    pub(super) fn exec_control_runtime_guidance(
        host_os: &str,
        remote_execution: bool,
        exec_control_available: bool,
    ) -> Vec<String> {
        if !exec_control_available || remote_execution || host_os != "windows" {
            return Vec::new();
        }

        vec![
            "- On local Windows ExecCommand sessions, `ExecControl` `interrupt` is effectively the same as `kill` for non-TTY processes.".to_string(),
        ]
    }

    /// Build runtime facts that may change independently from the agent's system prompt.
    pub async fn build_runtime_context_reminder(&self) -> Option<String> {
        let needs = self.context.runtime_context_needs;
        if needs.is_empty() {
            return None;
        }

        let host_os = std::env::consts::OS;
        let host_family = std::env::consts::FAMILY;
        let host_arch = std::env::consts::ARCH;

        let computer_use_keys = match host_os {
            "macos" => "- Computer use / `key_chord`: the local northhing desktop is macOS. Use `command`, `option`, `control`, and `shift` modifier names.",
            "windows" => "- Computer use / `key_chord`: the local northhing desktop is Windows. Use `meta`/`super` for the Windows key, plus `alt`, `control`, and `shift`.",
            "linux" => "- Computer use / `key_chord`: the local northhing desktop is Linux. Use `control`, `alt`, `shift`, and `meta`/`super` as appropriate for the desktop environment.",
            _ => "- Computer use / `key_chord`: match modifier names to the local northhing desktop OS.",
        };

        let mut lines = vec!["# Runtime Context".to_string()];

        if needs.workspace_tools {
            let mut workspace_lines = Vec::new();
            if let Some(remote) = &self.context.remote_execution {
                workspace_lines.push(format!(
                    "- Workspace file and shell tools operate on remote SSH connection \"{}\".",
                    remote.connection_display_name.replace('"', "'")
                ));
                workspace_lines.push(format!(
                    "- Remote host: {} (uname/kernel: {})",
                    remote.hostname.replace('"', "'"),
                    remote.kernel_name.replace('"', "'")
                ));
                workspace_lines.push("- Path conventions for workspace operations: POSIX paths with forward slashes and Unix shell syntax. Do not use PowerShell, `cmd.exe`, or Windows-style paths for remote workspace operations.".to_string());
            } else {
                workspace_lines.push("- Workspace file and shell tools operate on the local filesystem.".to_string());
            }
            Self::push_runtime_context_section(&mut lines, "Workspace Execution", workspace_lines);
        }

        if needs.exec_command {
            let mut exec_command_lines = Vec::new();
            if self.context.remote_execution.is_some() {
                exec_command_lines.push(
                    "- ExecCommand uses the remote user's default POSIX shell, invoked as `<shell> -lc <cmd>`."
                        .to_string(),
                );
            } else {
                let shell = ExecCommandTool::local_shell_prompt_info().await;
                Self::push_local_exec_shell_runtime_context(
                    &mut exec_command_lines,
                    &shell.display_name,
                    &shell.shell_type,
                    &shell.invocation,
                );
            }
            Self::push_runtime_context_section(&mut lines, "ExecCommand Shell", exec_command_lines);
        }

        let exec_control_lines =
            Self::exec_control_runtime_guidance(host_os, self.context.remote_execution.is_some(), needs.exec_control);
        Self::push_runtime_context_section(&mut lines, "ExecControl", exec_control_lines);

        if needs.computer_use {
            let mut local_client_lines = Vec::new();
            if self.context.remote_execution.is_some() && needs.workspace_tools {
                local_client_lines.push(
                    "- Computer use and UI automation operate on the local northhing desktop, even when workspace file and shell tools target a remote host."
                        .to_string(),
                );
            }
            local_client_lines.push(format!("- Local northhing client OS: {host_os} ({host_family})"));
            local_client_lines.push(format!("- Local client architecture: {host_arch}"));
            local_client_lines.push(computer_use_keys.to_string());
            Self::push_runtime_context_section(&mut lines, "Local Client", local_client_lines);
        }

        Some(lines.join("\n"))
    }

    /// Get workspace context that is intentionally injected outside the system prompt cache.
    pub(crate) async fn get_visual_mode_instruction(&self) -> String {
        let enabled = match GlobalConfigManager::service().await {
            Ok(service) => service
                .config::<bool>(Some("app.ai_experience.enable_visual_mode"))
                .await
                .unwrap_or(false),
            Err(e) => {
                debug!("Failed to read visual mode config: {}", e);
                false
            }
        };

        if enabled {
            r"# Visualizing complex logic as you explain
Use Mermaid diagrams to visualize complex logic, workflows, architectures, and data flows whenever it helps clarify the explanation.
Output Mermaid in fenced code blocks (```mermaid) so the UI can render them.
".to_string()
        } else {
            String::new()
        }
    }

    /// Get user language preference instruction
    ///
    /// Read app.language from global config, generate simple language instruction
    /// Returns empty string if config cannot be read
    /// Returns error if language code is unsupported
    pub async fn get_language_preference(&self) -> NortHingResult<String> {
        let language_code = get_app_language_code().await;
        Self::format_language_instruction(&language_code)
    }

    /// Format language instruction based on language code
    fn format_language_instruction(lang_code: &str) -> NortHingResult<String> {
        let Some(locale) = LocaleId::from_str(lang_code) else {
            return Err(NortHingError::config(format!("Unknown language code: {}", lang_code)));
        };
        let language = format!("**{}**", locale.model_language_name());
        Ok(format!("# Language Preference\nYou MUST respond in {} regardless of the user's input language. This is the system language setting and should be followed unless the user explicitly specifies a different language. This is crucial for smooth communication and user experience\n", language))
    }

    /// Get Claw-specific workspace boundary instruction
    pub(crate) fn get_claw_workspace_instruction(&self) -> String {
        "# Workspace
Your dedicated operating space is the workspace root shown in the current user context.
Prefer doing work inside this workspace and keep it well organized with clear structure, sensible filenames, and minimal clutter.
Do not read from, modify, create, move, or delete files outside this workspace unless the user has explicitly granted permission for that external action.
"
        .to_string()
    }
}
