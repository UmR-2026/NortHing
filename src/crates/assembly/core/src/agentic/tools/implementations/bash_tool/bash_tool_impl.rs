use crate::agentic::tools::framework::{Tool, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult};
use crate::agentic::tools::implementations::shell_safety;
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tool_runtime::shell::{
    banned_shell_command, detect_osascript_im_app, detect_osascript_keystroke_non_ascii,
    format_background_command_display_text, format_background_command_error_display_text,
    format_background_command_error_text, render_local_shell_result, render_remote_shell_result,
    BackgroundCommandDeliveryTextRequest, BackgroundCommandErrorTextRequest, BackgroundCommandStatusFacts,
    LocalShellResultRenderRequest, RemoteShellResultRenderRequest, BASH_RESULT_MAX_OUTPUT_LENGTH,
};

use super::bash_sandbox::{
    cancellation_error, cancellation_requested, emit_terminal_ready_event, noninteractive_env, resolve_shell,
};
use super::execute::execute_call;

pub struct BashTool;

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    pub fn noninteractive_env() -> std::collections::HashMap<String, String> {
        noninteractive_env()
    }

    pub(crate) fn resolve_working_directory(input: &Value, context: &ToolUseContext) -> NortHingResult<Option<String>> {
        let Some(raw_dir) = input.get("working_directory").and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        let trimmed = raw_dir.trim();
        if trimmed.is_empty() {
            return Ok(context.workspace.as_ref().map(|w| w.root_path_string()));
        }
        context.resolve_workspace_tool_path(trimmed).map(Some)
    }

    async fn is_existing_workspace_directory(context: &ToolUseContext, resolved_dir: &str) -> NortHingResult<bool> {
        if context.is_remote() {
            let fs = context.ws_fs().ok_or_else(|| {
                NortHingError::tool(
                    "Remote workspace filesystem is unavailable; cannot validate working_directory".to_string(),
                )
            })?;
            fs.is_dir(resolved_dir)
                .await
                .map_err(|e| NortHingError::tool(format!("Failed to validate working_directory: {e}")))
        } else {
            Ok(std::path::Path::new(resolved_dir).is_dir())
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    async fn description(&self) -> NortHingResult<String> {
        let shell_info = resolve_shell().await.display_name;

        Ok(format!(
            r#"Executes a given command in a persistent shell session with optional timeout, ensuring proper handling and security measures.

Shell Environment: {shell_info}

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) - use the specialized tools for this instead.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use `ls` to verify the parent directory exists and is the correct location
   - For example, before running "mkdir foo/bar", first use `ls foo` to check that "foo" exists and is the intended parent directory

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes (e.g., cd "path with spaces/file.txt")
   - Examples of proper quoting:
     - cd "My Documents" (correct)
     - cd My Documents (incorrect - will fail)
     - python "scripts/with spaces/script.py" (correct)
     - python scripts/with spaces/script.py (incorrect - will fail)
   - After ensuring proper quoting, execute the command.
   - Capture the output of the command.

Usage notes:
  - The command argument is required and MUST be a single-line command.
  - DO NOT use multiline commands or HEREDOC syntax (e.g., <<EOF, heredoc with newlines). Only single-line commands are supported.
  - You can specify an optional timeout in milliseconds (up to 600000ms / 10 minutes). If not specified, commands will timeout after 120000ms (2 minutes).
  - It is very helpful if you write a clear, concise description of what this command does. For simple commands, keep it brief (5-10 words). For complex commands (piped commands, obscure flags, or anything hard to understand at a glance), add enough context to clarify what it does.
  - If the output exceeds {BASH_RESULT_MAX_OUTPUT_LENGTH} characters, output will be truncated before being returned to you, with the tail of the output preserved because the ending is usually more important.
  - You can use the `run_in_background` parameter to run the command in a new dedicated background terminal session. The tool returns immediately without waiting for the command to finish. The final completion result will be delivered back to you automatically when it is done, and the full output will be saved to a session runtime file instead of being pasted back into chat. Only use this for long-running processes (e.g., dev servers, watchers) where you do not need the output right away. You do not need to append '&' to the command. NOTE: `timeout_ms` is ignored when `run_in_background` is true.
  - Each result includes a `<terminal_session_id>` tag identifying the terminal session. The persistent shell session ID remains constant throughout the entire conversation; background sessions each have their own unique ID.
  - The output may include the command echo and/or the shell prompt prefix (for example, a printed `PS` or `$` prompt line). Do not treat these as part of the command's actual result.
  - Avoid interactive commands that may block waiting for user input or open a pager/editor. Prefer non-interactive variants and explicit flags. For example, use `git --no-pager diff` instead of `git diff`, and avoid commands that prompt for confirmation unless the User explicitly asks for them.
  
  - Prefer specialized tools for workspace file operations: Glob for file discovery, Grep for content search, Read for reading, Edit for modifying, Write for creating, and Delete for deletion. Prefer the Git tool (after loading it with GetToolSpec when collapsed) for Git subcommands such as status, diff, log, add, commit, branch, checkout, pull, and push. Use Bash for commands that genuinely need a shell, such as build/test/package CLIs, process control, scripts, and environment checks. Never use shell output only to communicate with the user.
  - When issuing multiple commands:
    - If the commands are independent and can run in parallel, make multiple tool calls in a single message. For Git inspection, prefer parallel Git tool calls such as `{{"operation":"status"}}` and `{{"operation":"diff","args":"--stat"}}` instead of Bash.
    - If the commands depend on each other and must run sequentially, use a single Bash call with '&&' to chain them together (e.g., `git add . && git commit -m "message" && git push`). For instance, if one operation must complete before another starts (like mkdir before cp, Write before Bash for git operations, or git add before git commit), run these operations sequentially instead.
    - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail
    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)
  - Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of `cd`. You may use `cd` if the User explicitly asks for it.
    <good-example>
    pytest /foo/bar/tests
    </good-example>
    <bad-example>
    cd /foo/bar && pytest tests
    </bad-example>"#
        ))
    }

    fn short_description(&self) -> String {
        "Run commands in the persistent shell session.".to_string()
    }

    async fn description_with_context(&self, context: Option<&ToolUseContext>) -> NortHingResult<String> {
        let mut base = self.description().await?;
        if context.map(|c| c.is_remote()).unwrap_or(false) {
            base = format!(
                r#"**Remote workspace:** Commands run on the **SSH server** in a shell whose initial working directory is the **remote workspace root** (same as running a terminal on that machine). The shell name shown below may reflect your **local** northhing settings; the actual interpreter on the server is typically `sh`/`bash`. Use **Unix** syntax and POSIX paths — not PowerShell or Windows paths.

{base}"#,
                base = base
            );
        }
        if !context.map(|c| c.is_remote()).unwrap_or(false) {
            base.push_str(
                "\n\n**Desktop automation:** Prefer this tool for actions achievable from the **workspace shell** (build, test, git, scripts, CLIs). On **macOS**, `open -a \"AppName\"` can launch or foreground an app. Use the dedicated `ComputerUse` tool or agent for desktop UI perception/control such as screenshots, OCR, mouse, keyboard, app state, clipboard, and OS-level interactions.",
            );
        }
        Ok(base)
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout_ms": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (default 120000, max 600000). Ignored when run_in_background is true."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "If true, runs the command in a new dedicated background terminal session and returns immediately. The final completion result is delivered back automatically when the command finishes, and the full output is saved to a session runtime file instead of being injected into chat. Useful for long-running processes like dev servers or file watchers. timeout_ms is ignored when this is true."
                },
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory to run the command in. Use a workspace-relative path or an absolute path inside the current workspace. Omit to reuse the persistent terminal's current directory."
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in 5-10 words, in active voice. Examples:\nInput: ls\nOutput: List files in current directory\n\nInput: git status\nOutput: Show working tree status\n\nInput: npm install\nOutput: Install package dependencies\n\nInput: mkdir foo\nOutput: Create directory 'foo'"
                }
            },
            "required": ["command"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        true
    }

    async fn validate_input(&self, input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
        let command = input.get("command").and_then(|v| v.as_str());
        let run_in_background = input
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(cmd) = command {
            if let Some(base_cmd) = banned_shell_command(cmd) {
                return ValidationResult {
                    result: false,
                    message: Some(format!("Command '{}' is not allowed for security reasons", base_cmd)),
                    error_code: Some(403),
                    meta: None,
                };
            }

            const ENABLE_SHELL_DENYLIST: bool = true;
            if ENABLE_SHELL_DENYLIST {
                if let Some(pattern) = shell_safety::check_command_denied(cmd) {
                    return ValidationResult {
                        result: false,
                        message: Some(format!(
                            "Command matched shell denylist (R1 safety filter). Refusing to execute: {}\nMatched pattern: {}",
                            cmd, pattern
                        )),
                        error_code: Some(403),
                        meta: None,
                    };
                }
            }

            if let Some(literal) = detect_osascript_keystroke_non_ascii(cmd) {
                let preview: String = literal.chars().take(40).collect();
                return ValidationResult {
                    result: false,
                    message: Some(format!(
                        "Refused: `osascript ... keystroke \"{}…\"` cannot type non-ASCII text — \
                         AppleScript's `keystroke` sends raw key codes, not Unicode, so CJK / \
                         emoji / accented text comes out as garbage in the target app (e.g. \
                         the WeChat search box receives `AAA…` instead of `{}`). \n\n\
                         Use ControlHub instead:\n\
                         1. `system.open_app {{ app_name: \"<App>\" }}` to focus the app\n\
                         2. (optional) `desktop.key_chord {{ keys: [\"command\",\"f\"] }}` to focus search\n\
                         3. `desktop.paste {{ text: \"<your text>\", submit: true }}` — pastes via \
                            system clipboard, works for ANY language.\n\n\
                         For sending an IM message specifically, run the `im_send_message` \
                         playbook — it's the same 3-step flow pre-packaged.",
                        preview, preview
                    )),
                    error_code: Some(400),
                    meta: None,
                };
            }

            if let Some(app) = detect_osascript_im_app(cmd) {
                return ValidationResult {
                    result: false,
                    message: Some(format!(
                        "Refused: driving {app} via `osascript` / AppleScript GUI scripting is unreliable \
                         (no CJK support in keystroke, no return value, easy to deadlock). \n\n\
                         Use the canonical IM-send recipe instead — same 3 deterministic calls:\n\
                         1. `ControlHub domain:\"system\" action:\"open_app\" {{ app_name:\"{app}\" }}`\n\
                         2. `ControlHub domain:\"desktop\" action:\"key_chord\" {{ keys:[\"command\",\"f\"] }}`\n\
                         3. `ControlHub domain:\"desktop\" action:\"paste\" {{ text:\"<contact>\", submit:true }}`\n\
                         4. `ControlHub domain:\"desktop\" action:\"paste\" {{ text:\"<message>\", submit:true }}`\n\n\
                         Or run the prepackaged `im_send_message` playbook with \
                         `{{ app_name, contact, message }}`. For Slack/Lark where Return inserts \
                         a newline, pass `submit_keys:[\"command\",\"return\"]`."
                    )),
                    error_code: Some(400),
                    meta: None,
                };
            }
        } else {
            return ValidationResult {
                result: false,
                message: Some("command is required".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        let Some(context) = context else {
            return ValidationResult {
                result: false,
                message: Some("tool context is required for Bash tool".to_string()),
                error_code: Some(400),
                meta: None,
            };
        };

        if context.session_id.as_deref().unwrap_or_default().is_empty() {
            return ValidationResult {
                result: false,
                message: Some("session_id is required for Bash tool".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        if context.workspace_root().is_none() {
            return ValidationResult {
                result: false,
                message: Some("workspace_path is required for Bash tool".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        match Self::resolve_working_directory(input, context) {
            Ok(Some(resolved_dir)) => match Self::is_existing_workspace_directory(context, &resolved_dir).await {
                Ok(true) => {}
                Ok(false) => {
                    return ValidationResult {
                        result: false,
                        message: Some(format!(
                            "working_directory must be an existing directory inside the current workspace: {}",
                            resolved_dir
                        )),
                        error_code: Some(400),
                        meta: None,
                    };
                }
                Err(err) => {
                    return ValidationResult {
                        result: false,
                        message: Some(err.to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }
            },
            Ok(None) => {}
            Err(err) => {
                return ValidationResult {
                    result: false,
                    message: Some(err.to_string()),
                    error_code: Some(400),
                    meta: None,
                };
            }
        }

        if run_in_background && input.get("timeout_ms").is_some() {
            return ValidationResult {
                result: true,
                message: Some("Note: timeout_ms is ignored when run_in_background is true".to_string()),
                error_code: None,
                meta: None,
            };
        }

        ValidationResult {
            result: true,
            message: None,
            error_code: None,
            meta: None,
        }
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
            if command.contains("\"$(cat <<'EOF'") {
                if let Some(start) = command.find("\"$(cat <<'EOF'\n") {
                    if let Some(end) = command.find("\nEOF\n)") {
                        let prefix = &command[..start];
                        let content_start = start + "\"$(cat <<'EOF'\n".len();
                        let content = &command[content_start..end];
                        return format!("{} \"{}\"", prefix.trim(), content.trim());
                    }
                }
            }
            command.to_string()
        } else {
            "Executing command".to_string()
        }
    }

    async fn call_impl(&self, _input: &Value, _context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        Err(NortHingError::tool(
            "Bash tool call_impl should not be called".to_string(),
        ))
    }

    async fn call(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        execute_call(input, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::bash_sandbox::command_needs_light_checkpoint;
    use super::*;
    use tool_runtime::shell::{
        command_for_working_directory, format_background_command_delivery_text, BackgroundCommandDeliveryTextRequest,
        BackgroundCommandStatusFacts,
    };

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
