//! Prompt-visible tool description for the Git tool.
//!
//! # Why the full `impl Tool for GitTool` lives here
//!
//! Rust does not allow splitting one `impl Trait for Type` across multiple
//! blocks in the same crate (E0119 "conflicting implementations"). A 2026-07-12
//! attempt to put only `description()` + `description_with_context()` here as
//! a second `impl Tool for GitTool` block, with the remaining 9 trait methods
//! kept in `mod.rs`, failed at `cargo check` with E0119 even though the
//! method names did not collide.
//!
//! Therefore the entire `impl Tool for GitTool` block lives in this file
//! (not in `mod.rs` as the original R73-4 spec described). `mod.rs` is a
//! thin facade: 5 sibling `mod` decls + `GitTool` struct + `Default` impl.
//!
//! File layout after R73-4 split:
//! - `mod.rs`           (~25 lines): facade - mod decls + `GitTool` + `Default`
//! - `description.rs`   (~401 lines): this file - full `impl Tool for GitTool`
//! - `tests.rs`         (~238 lines): 14 unit tests (moved from pre-split `mod.rs`)

use super::git_branch::{execute_branch, execute_checkout};
use super::git_commit::execute_commit;
use super::git_query::{execute_add, execute_diff, execute_generic, execute_log, execute_status, get_repo_path};
use super::git_remote::{execute_pull, execute_push, execute_remote_git_cli};
use super::git_types::{
    git_operation_needs_light_checkpoint, is_dangerous_operation, normalize_git_input, ParsedDiffArgs,
    ALLOWED_OPERATIONS,
};
use super::GitTool;
use crate::agentic::tools::framework::{
    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "Git"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(r#"Executes Git commands for version control operations.

This tool provides a safe and convenient way to execute Git commands. It supports common Git operations like status, diff, log, add, commit, branch, checkout, pull, push, and more.

If this tool was collapsed earlier in the conversation, only call it after `GetToolSpec` has returned this definition. A failed direct call that says "Tool 'Git' is collapsed" means the next tool call should be `GetToolSpec` with `{"tool_name":"Git"}`; after that, retry `Git` with the schema below.

## Supported Operations

- **status**: Show working tree status
- **diff**: Show changes between commits, commit and working tree, etc.
- **log**: Show commit logs
- **add**: Add file contents to the index
- **commit**: Record changes to the repository
- **branch**: List, create, or delete branches
- **checkout/switch**: Switch branches or restore working tree files
- **pull**: Fetch from and integrate with another repository or a local branch
- **push**: Update remote refs along with associated objects
- **fetch**: Download objects and refs from another repository
- **merge**: Join two or more development histories together
- **rebase**: Reapply commits on top of another base tip
- **stash**: Stash the changes in a dirty working directory away
- **reset**: Reset current HEAD to the specified state
- **restore**: Restore working tree files
- **show**: Show various types of objects
- **tag**: Create, list, delete or verify a tag object
- **remote**: Manage set of tracked repositories
- **clone**: Clone a repository into a new directory
- **init**: Create an empty Git repository
- **blame**: Show what revision and author last modified each line
- **cherry-pick**: Apply the changes introduced by some existing commits

## Usage Examples

1. Check status:
    ```json
    {"operation": "status"}
    ```

2. View diff of staged changes:
    ```json
    {"operation": "diff", "args": "--staged"}
    ```

3. View recent commits:
    ```json
    {"operation": "log", "args": "--oneline -10"}
    ```

4. Add files:
    ```json
    {"operation": "add", "args": "."}
    ```

5. Commit with message:
    ```json
    {"operation": "commit", "args": "-m \"Your commit message\""}
    ```

6. Create a new branch:
    ```json
    {"operation": "branch", "args": "feature/new-feature"}
    ```

7. Switch to a branch:
    ```json
    {"operation": "switch", "args": "main"}
    ```

## Important: Input Shape

- **Preferred format:** always send a JSON object with top-level `operation` plus optional `args`.
- `operation` is the bare Git subcommand (`status`, `diff`, `log`, `add`, `commit`, ...).
- `args` contains only flags, refs, paths, or commit-message text for that subcommand.
- **Do NOT repeat the subcommand in `args`.** Example: `{"operation": "diff", "args": "HEAD~2..HEAD --stat"}` — not `{"operation": "diff", "args": "diff HEAD~2..HEAD --stat"}`.
- Prefer this tool over Bash for Git subcommands when `Git` is available. Bash is still fine for shell pipelines, hooks, or commands that combine Git with other tools.
- Common shell-style mistakes (`"git status"`, `{"command": "git status"}`, or `{"args": "log --oneline -10"}`) are auto-normalized when possible, but the canonical `{operation, args?}` shape above is more reliable.

## Safety Notes

- This tool validates operations to ensure only allowed Git commands are executed
- Dangerous operations (like `push --force`, `reset --hard`) will show warnings
- Never run `git config` to modify user settings
- Always verify changes before committing
  - Use `--dry-run` for push/pull operations when unsure

## Remote SSH

When the workspace is opened over Remote SSH, Git runs on the **server** (see tool description context at runtime).

## Commit Message Guidelines

When creating commits, use this format for the commit message:
- Start with a concise summary, preferably 50 characters or less
- Leave a blank line after the summary when adding a body
- Add a body only when it helps explain the rationale, scope, or verification
- Do not add generated-by or co-author footers unless the user or repository convention asks for them"#.to_string())
    }

    async fn description_with_context(&self, context: Option<&ToolUseContext>) -> NortHingResult<String> {
        let mut base = self.description().await?;
        if context.map(|c| c.is_remote()).unwrap_or(false) {
            base.push_str(
                "\n\n**Remote workspace:** Commands execute on the **SSH host** via `git -C <repo> …`, using the same repository and Git install as a native terminal on that server (equivalent to Claude Code / CLI on the remote machine). Paths are POSIX paths on the server.",
            );
        }
        Ok(base)
    }

    fn short_description(&self) -> String {
        "Inspect and operate on the Git repository; load with GetToolSpec before first use when collapsed.".to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Git subcommand to run. Use the bare subcommand only, such as \"status\", \"diff\", \"log\", \"add\", or \"commit\". Do not prefix with \"git\" and do not put the subcommand in args.",
                    "enum": ALLOWED_OPERATIONS
                },
                "args": {
                    "type": "string",
                    "description": "Optional extra arguments for the selected operation: flags, refs, commit messages, or file paths. Examples: \"--staged\", \"--oneline -10\", \"-m \\\"message\\\"\", or \"-- src/file.rs\". Do not include \"git\" or repeat the operation/subcommand here."
                },
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory to run the Git command in. Omit to use the current workspace. If provided, use a workspace-relative path or an absolute path inside the current workspace; never use placeholder paths such as /workspace."
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn needs_permissions(&self, input: Option<&Value>) -> bool {
        if let Some(input) = input {
            if let Some(operation) = input.get("operation").and_then(|v| v.as_str()) {
                let readonly_ops = [
                    "status",
                    "diff",
                    "log",
                    "show",
                    "branch",
                    "remote",
                    "tag",
                    "blame",
                    "describe",
                    "shortlog",
                    "rev-parse",
                ];
                if operation == "branch" {
                    if let Some(args) = input.get("args").and_then(|v| v.as_str()) {
                        if !args.is_empty()
                            && !args.contains("-l")
                            && !args.contains("--list")
                            && !args.contains("-a")
                            && !args.contains("-r")
                        {
                            return true;
                        }
                    }
                    return false;
                }
                return !readonly_ops.contains(&operation);
            }
        }
        true
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        let input = &normalize_git_input(input.clone());

        let operation = match input.get("operation").and_then(|v| v.as_str()) {
            Some(op) => op,
            None => {
                return ValidationResult {
                    result: false,
                    message: Some(
                        "Could not determine Git operation. Send {\"operation\":\"status\"} (preferred) or a repairable shell-style payload such as {\"command\":\"git status\"} or {\"args\":\"log --oneline -10\"}."
                            .to_string(),
                    ),
                    error_code: Some(400),
                    meta: None,
                };
            }
        };

        if !ALLOWED_OPERATIONS.contains(&operation) {
            return ValidationResult {
                result: false,
                message: Some(format!(
                    "Operation '{}' is not allowed. Allowed operations: {}",
                    operation,
                    ALLOWED_OPERATIONS.join(", ")
                )),
                error_code: Some(403),
                meta: None,
            };
        }

        let args = input.get("args").and_then(|v| v.as_str()).unwrap_or("");

        if args.contains("-i") || args.contains("--interactive") {
            return ValidationResult {
                result: false,
                message: Some("Interactive mode (-i) is not supported".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        if is_dangerous_operation(operation, args) {
            return ValidationResult {
                result: true,
                message: Some(format!(
                    "Warning: This is a potentially dangerous operation: git {} {}",
                    operation, args
                )),
                error_code: None,
                meta: Some(json!({ "warning": "dangerous_operation" })),
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
        let operation = input.get("operation").and_then(|v| v.as_str()).unwrap_or("unknown");
        let args = input.get("args").and_then(|v| v.as_str()).unwrap_or("");

        if args.is_empty() {
            format!("git {}", operation)
        } else {
            format!("git {} {}", operation, args)
        }
    }

    fn render_result_for_assistant(&self, output: &Value) -> String {
        let stdout = output.get("stdout").and_then(|v| v.as_str()).unwrap_or("").trim();
        let stderr = output.get("stderr").and_then(|v| v.as_str()).unwrap_or("").trim();
        let exit_code = output.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let command = output.get("command").and_then(|v| v.as_str()).unwrap_or("");

        let mut result_parts = Vec::new();

        if !command.is_empty() {
            result_parts.push(format!("$ {}", command));
        }

        if !stdout.is_empty() {
            result_parts.push(stdout.to_string());
        }

        if !stderr.is_empty() {
            result_parts.push(stderr.to_string());
        }

        if exit_code != 0 {
            result_parts.push(format!("\n[Exit code: {} - command failed]", exit_code));
        }

        if result_parts.is_empty() {
            "(no output)".to_string()
        } else {
            result_parts.join("\n")
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let input = &normalize_git_input(input.clone());

        let operation = input
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("operation is required".to_string()))?;

        let args = input.get("args").and_then(|v| v.as_str());

        let args = args.map(|a| {
            let trimmed = a.trim();
            let prefix = format!("{} ", operation);
            if trimmed.starts_with(&prefix) {
                &trimmed[prefix.len()..]
            } else {
                trimmed
            }
        });

        let working_directory = input.get("working_directory").and_then(|v| v.as_str());

        let repo_path = get_repo_path(working_directory, context)?;

        debug!(
            "Git tool executing operation: {} in repository: {}, args: {}",
            operation,
            repo_path,
            args.unwrap_or("")
        );

        if git_operation_needs_light_checkpoint(operation, args) {
            context
                .record_light_checkpoint(
                    "Git",
                    &format!("git {} {}", operation, args.unwrap_or("").trim()),
                    Vec::new(),
                )
                .await;
        }

        let start_time = std::time::Instant::now();

        let result = if context.is_remote() {
            execute_remote_git_cli(&repo_path, operation, args, context).await?
        } else {
            match operation {
                "status" => execute_status(&repo_path).await?,
                "diff" => execute_diff(&repo_path, args).await?,
                "log" => execute_log(&repo_path, args).await?,
                "add" => execute_add(&repo_path, args).await?,
                "commit" => execute_commit(&repo_path, args).await?,
                "push" => execute_push(&repo_path, args).await?,
                "pull" => execute_pull(&repo_path, args).await?,
                "checkout" | "switch" => execute_checkout(&repo_path, args).await?,
                "branch" => execute_branch(&repo_path, args).await?,
                _ => execute_generic(&repo_path, operation, args).await?,
            }
        };

        let duration = start_time.elapsed();
        debug!(
            "Git tool command completed, operation: {}, duration: {}ms",
            operation,
            duration.as_millis()
        );

        let mut result_with_meta = result.clone();
        if let Some(obj) = result_with_meta.as_object_mut() {
            obj.insert("execution_time_ms".to_string(), json!(duration.as_millis() as u64));
            if !context.is_remote() {
                obj.insert(
                    "command".to_string(),
                    json!(format!("git {} {}", operation, args.unwrap_or(""))),
                );
            }
            obj.insert("operation".to_string(), json!(operation));
            obj.insert("working_directory".to_string(), json!(repo_path));
        }

        let result_for_assistant = self.render_result_for_assistant(&result_with_meta);

        Ok(vec![ToolResult::Result {
            data: result_with_meta,
            result_for_assistant: Some(result_for_assistant),
            image_attachments: None,
        }])
    }
}
