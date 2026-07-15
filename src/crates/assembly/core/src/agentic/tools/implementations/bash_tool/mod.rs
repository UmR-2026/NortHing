use crate::agentic::tools::framework::{Tool, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult};
use crate::agentic::tools::implementations::shell_safety;
use crate::agentic::workspace::WorkspaceCommandOptions;
use crate::infrastructure::events::event_system::global_event_system;
use crate::infrastructure::events::event_system::BackendEvent::{ToolExecutionProgress, ToolTerminalReady};
use crate::service::config::global::get_global_config_service;
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::event::{ToolExecutionProgressInfo, ToolTerminalReadyInfo};
use async_trait::async_trait;
use futures::StreamExt;
use northhing_runtime_ports::AgentBackgroundResultRequest;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};
use terminal_core::session::SessionSource;
use terminal_core::shell::{ShellDetector, ShellType};
use terminal_core::{
    CommandCompletionReason, CommandStreamEvent, ExecuteCommandRequest, SignalRequest, TerminalApi,
    TerminalBindingOptions, TerminalSessionBinding,
};
use tokio::io::AsyncWriteExt;
use tool_runtime::shell::{
    banned_shell_command, bash_noninteractive_env, command_for_working_directory, detect_osascript_im_app,
    detect_osascript_keystroke_non_ascii, format_background_command_delivery_text,
    format_background_command_display_text, format_background_command_error_display_text,
    format_background_command_error_text, render_local_shell_result, render_remote_shell_result,
    BackgroundCommandDeliveryTextRequest, BackgroundCommandErrorTextRequest, BackgroundCommandStatusFacts,
    LocalShellResultRenderRequest, RemoteShellResultRenderRequest, BASH_INTERRUPT_OUTPUT_DRAIN_MS,
    BASH_RESULT_MAX_OUTPUT_LENGTH,
};
use tracing::{debug, error, info};

pub mod bash_helpers;
pub mod bash_sandbox;
pub mod bash_tool_impl;
pub mod bash_types;
pub mod execute;

pub use bash_helpers::*;
pub use bash_sandbox::*;
pub use bash_tool_impl::BashTool;
pub use bash_types::*;
pub use execute::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_detection_flags_mutating_bash_commands() {
        assert!(command_needs_light_checkpoint("cargo fmt"));
        assert!(command_needs_light_checkpoint("pnpm lint --fix"));
        assert!(command_needs_light_checkpoint("rm -rf target/tmp"));
        assert!(!command_needs_light_checkpoint("cargo test"));
        assert!(!command_needs_light_checkpoint("git status"));
    }

    #[test]
    fn truncate_output_preserving_tail_keeps_end_of_output() {
        let input = "BEGIN-".to_string() + &"x".repeat(120) + "-IMPORTANT-END";

        let truncated = tool_runtime::shell::truncate_output_preserving_tail(&input, 80);

        assert!(truncated.contains("tail preserved"));
        assert!(truncated.ends_with("IMPORTANT-END"));
        assert!(!truncated.contains("BEGIN-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        assert!(truncated.chars().count() <= 80);
    }

    #[test]
    fn detect_osascript_keystroke_non_ascii_flags_cjk_keystroke() {
        let cmd = r#"osascript -e 'tell application "System Events" to keystroke "尉怡青"'"#;
        let hit = detect_osascript_keystroke_non_ascii(cmd).expect("should flag CJK keystroke");
        assert!(hit.contains("尉怡青"));
    }

    #[test]
    fn detect_osascript_keystroke_non_ascii_flags_emoji_keystroke() {
        let cmd = r#"osascript -e 'tell application "System Events" to keystroke "hi 👋"'"#;
        assert!(detect_osascript_keystroke_non_ascii(cmd).is_some());
    }

    #[test]
    fn detect_osascript_keystroke_non_ascii_passes_pure_ascii() {
        let cmd = r#"osascript -e 'tell application "System Events" to keystroke "hello"'"#;
        assert!(detect_osascript_keystroke_non_ascii(cmd).is_none());
    }

    #[test]
    fn detect_osascript_keystroke_non_ascii_passes_non_osascript() {
        let cmd = r#"echo "尉怡青""#;
        assert!(detect_osascript_keystroke_non_ascii(cmd).is_none());
    }

    #[test]
    fn detect_osascript_im_app_flags_wechat() {
        let cmd = r#"osascript -e 'tell application "WeChat" to activate'"#;
        assert_eq!(detect_osascript_im_app(cmd), Some("WeChat"));
    }

    #[test]
    fn detect_osascript_im_app_flags_weixin_chinese() {
        let cmd = r#"osascript -e 'tell application "微信" to activate'"#;
        assert_eq!(detect_osascript_im_app(cmd), Some("微信"));
    }

    #[test]
    fn detect_osascript_im_app_passes_non_im() {
        let cmd = r#"osascript -e 'tell application "Finder" to activate'"#;
        assert!(detect_osascript_im_app(cmd).is_none());
    }

    #[test]
    fn render_result_marks_truncated_output_and_keeps_tail() {
        let long_output = "prefix\n".to_string() + &"y".repeat(BASH_RESULT_MAX_OUTPUT_LENGTH + 100) + "\nfinal-error";

        let rendered = render_local_shell_result(LocalShellResultRenderRequest {
            terminal_session_id: "session-1",
            working_directory: "/repo",
            output_text: &long_output,
            interrupted: false,
            timed_out: false,
            exit_code: 1,
            shell_state: None,
        });

        assert!(rendered.contains("<output truncated=\"true\">"));
        assert!(rendered.contains("tail preserved"));
        assert!(rendered.contains("final-error"));
        assert!(rendered.contains("<exit_code>1</exit_code>"));
    }

    #[test]
    fn render_remote_result_keeps_stdout_and_stderr_separate() {
        let rendered = render_remote_shell_result(RemoteShellResultRenderRequest {
            working_directory: "/repo",
            stdout: "stdout text",
            stderr: "stderr text",
            interrupted: false,
            timed_out: false,
            exit_code: 2,
        });

        assert!(rendered.contains("<remote_ssh>true</remote_ssh>"));
        assert!(rendered.contains("<exit_code>2</exit_code>"));
        assert!(rendered.contains("<stdout>stdout text</stdout>"));
        assert!(rendered.contains("<stderr>stderr text</stderr>"));
        assert!(!rendered.contains("<terminal_session_id>"));
    }

    #[test]
    fn render_remote_result_uses_shared_budget_with_stderr_priority() {
        let long_stdout = "prefix\n".to_string() + &"x".repeat(BASH_RESULT_MAX_OUTPUT_LENGTH + 100) + "\nstdout-tail";
        let long_stderr = "prefix\n".to_string() + &"z".repeat(BASH_RESULT_MAX_OUTPUT_LENGTH / 2) + "\nstderr-tail";

        let rendered = render_remote_shell_result(RemoteShellResultRenderRequest {
            working_directory: "/repo",
            stdout: &long_stdout,
            stderr: &long_stderr,
            interrupted: false,
            timed_out: false,
            exit_code: 1,
        });

        assert!(rendered.contains("<stdout truncated=\"true\">"));
        assert!(rendered.contains("stdout-tail"));
        assert!(!rendered.contains("<stderr truncated=\"true\">"));
        assert!(rendered.contains("stderr-tail"));
    }

    #[test]
    fn render_remote_result_gives_all_budget_to_oversized_stderr() {
        let long_stderr =
            "prefix\n".to_string() + &"z".repeat(BASH_RESULT_MAX_OUTPUT_LENGTH + 100) + "\nremote-final-error";

        let rendered = render_remote_shell_result(RemoteShellResultRenderRequest {
            working_directory: "/repo",
            stdout: "stdout text",
            stderr: &long_stderr,
            interrupted: false,
            timed_out: false,
            exit_code: 1,
        });

        assert!(rendered.contains("<stdout truncated=\"true\">"));
        assert!(rendered.contains("no budget remaining"));
        assert!(rendered.contains("<stderr truncated=\"true\">"));
        assert!(rendered.contains("tail preserved"));
        assert!(rendered.contains("remote-final-error"));
    }

    #[test]
    fn input_schema_accepts_working_directory() {
        let tool = BashTool::new();
        let schema = tool.input_schema();

        assert!(schema["properties"].get("working_directory").is_some());
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn command_is_prefixed_with_quoted_working_directory_when_requested() {
        let command = command_for_working_directory("pnpm install", Some("/Users/example/My Project"));

        assert_eq!(command, "cd '/Users/example/My Project' && pnpm install");
    }

    #[test]
    fn command_prefix_escapes_single_quotes_in_working_directory() {
        let command = command_for_working_directory("pwd", Some("/tmp/it's fine"));

        assert_eq!(command, "cd '/tmp/it'\\''s fine' && pwd");
    }

    #[test]
    fn command_result_includes_working_directory_for_model() {
        let rendered = render_local_shell_result(LocalShellResultRenderRequest {
            terminal_session_id: "session-1",
            working_directory: "/private/tmp",
            output_text: "ERR_PNPM_NO_PKG_MANIFEST No package.json found in /private/tmp",
            interrupted: false,
            timed_out: false,
            exit_code: 1,
            shell_state: None,
        });

        assert!(rendered.contains("<exit_code>1</exit_code>"));
        assert!(rendered.contains("<working_directory>/private/tmp</working_directory>"));
        assert!(rendered.contains("ERR_PNPM_NO_PKG_MANIFEST"));
    }

    #[test]
    fn background_delivery_text_points_to_saved_output_file() {
        let rendered = format_background_command_delivery_text(BackgroundCommandDeliveryTextRequest {
            command: "pnpm test",
            terminal_session_id: "bg-session-1",
            working_directory: "/repo",
            status: BackgroundCommandStatusFacts {
                exit_code: Some(0),
                timed_out: false,
                interrupted: false,
            },
            output_file_reference: "/runtime/sessions/session/tool-results/bash_123.txt",
            output_persist_error: None,
        });

        assert!(rendered.contains("Background Bash command completed successfully."));
        assert!(rendered.contains("status=\"completed\""));
        assert!(rendered.contains("terminal_session_id=\"bg-session-1\""));
        assert!(rendered.contains("Full output was saved to: /runtime/sessions/session/tool-results/bash_123.txt"));
    }

    #[test]
    fn background_display_text_is_concise() {
        assert_eq!(
            format_background_command_display_text(BackgroundCommandStatusFacts {
                exit_code: Some(0),
                timed_out: false,
                interrupted: false,
            }),
            "Background Bash command completed successfully."
        );
        assert_eq!(
            format_background_command_display_text(BackgroundCommandStatusFacts {
                exit_code: Some(1),
                timed_out: false,
                interrupted: false,
            }),
            "Background Bash command completed with a non-zero exit code."
        );
        assert_eq!(
            format_background_command_display_text(BackgroundCommandStatusFacts {
                exit_code: None,
                timed_out: true,
                interrupted: false,
            }),
            "Background Bash command timed out."
        );
        assert_eq!(
            format_background_command_display_text(BackgroundCommandStatusFacts {
                exit_code: Some(130),
                timed_out: false,
                interrupted: true,
            }),
            "Background Bash command was interrupted."
        );
        assert_eq!(
            format_background_command_error_display_text(),
            "Background Bash command failed before producing a final completion result."
        );
    }
}
