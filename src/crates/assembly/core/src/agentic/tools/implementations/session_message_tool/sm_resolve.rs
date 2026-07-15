//! `SessionMessage` tool — workspace, sender, and creator resolution helpers.
//!
//! All non-tool-trait logic that turns tool inputs into concrete runtime
//! handles lives here: workspace-path checks (local + remote host),
//! sending-side session/workspace extraction, session-creator marker
//! composition, and the prepended-reminder envelope for cross-session
//! messages. The `Tool` impl in `tool.rs` and the dispatch flow in
//! `sm_send.rs` both call into these helpers.

use std::path::Path;

use northhing_runtime_ports::AgentDialogPrependedReminder;

use crate::agentic::tools::framework::{ToolUseContext, ValidationResult};
use crate::agentic::tools::workspace_paths::posix_style_path_is_absolute;
use crate::util::errors::{NortHingError, NortHingResult};

use super::super::util::normalize_path;

impl super::tool::SessionMessageTool {
    /// Resolve a workspace string into a `NortHingResult<String>` containing
    /// the canonical absolute path. Local paths must exist as directories;
    /// remote-host workspaces are forwarded through the tool-use context.
    pub(super) fn resolve_workspace(&self, workspace: &str, context: &ToolUseContext) -> NortHingResult<String> {
        let workspace = workspace.trim();
        if workspace.is_empty() {
            return Err(NortHingError::tool(
                "workspace is required and cannot be empty".to_string(),
            ));
        }

        if context.is_remote() {
            if !posix_style_path_is_absolute(workspace) {
                return Err(NortHingError::tool(
                    "workspace must be an absolute POSIX path on the remote host".to_string(),
                ));
            }
            return context.resolve_workspace_tool_path(workspace);
        }

        let path = Path::new(workspace);
        if !path.is_absolute() {
            return Err(NortHingError::tool("workspace must be an absolute path".to_string()));
        }

        let resolved = normalize_path(workspace);
        let path = Path::new(&resolved);
        if !path.exists() {
            return Err(NortHingError::tool(format!("Workspace does not exist: {}", resolved)));
        }
        if !path.is_dir() {
            return Err(NortHingError::tool(format!(
                "Workspace is not a directory: {}",
                resolved
            )));
        }
        Ok(resolved)
    }

    /// Lightweight schema-level workspace validator used from
    /// `validate_input`. Does not touch the filesystem — only checks the
    /// shape — so it can run before any runtime call.
    pub(super) fn validate_workspace_shape(
        &self,
        workspace: &str,
        context: Option<&ToolUseContext>,
    ) -> ValidationResult {
        let workspace = workspace.trim();
        if workspace.is_empty() {
            return ValidationResult {
                result: false,
                message: Some("workspace is required and cannot be empty".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        match context {
            Some(context) => {
                let ws_ok = if context.is_remote() {
                    posix_style_path_is_absolute(workspace)
                } else {
                    Path::new(workspace).is_absolute()
                };
                if !ws_ok {
                    return ValidationResult {
                        result: false,
                        message: Some("workspace must be an absolute path".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }
            }
            None => {
                if !Path::new(workspace).is_absolute() && !posix_style_path_is_absolute(workspace) {
                    return ValidationResult {
                        result: false,
                        message: Some("workspace must be an absolute path".to_string()),
                        error_code: Some(400),
                        meta: None,
                    };
                }
            }
        }

        ValidationResult::default()
    }

    /// Borrowed `&str` view of the sending session id. Used by `call_impl`
    /// when constructing the dialog reply route.
    pub(super) fn sender_session_id<'a>(&self, context: &'a ToolUseContext) -> NortHingResult<&'a str> {
        context
            .session_id
            .as_deref()
            .ok_or_else(|| NortHingError::tool("SessionMessage requires a source session".to_string()))
    }

    /// Owned copy of the sending workspace path. Falls back to the same
    /// error as `sender_session_id` so callers see consistent UX.
    pub(super) fn sender_workspace(&self, context: &ToolUseContext) -> NortHingResult<String> {
        context
            .workspace_root()
            .map(|path| path.to_string_lossy().to_string())
            .ok_or_else(|| NortHingError::tool("SessionMessage requires a source workspace".to_string()))
    }

    /// Compose the `createdBy` marker the runtime records on a freshly
    /// created session. Identifies the source (creator) session so replies
    /// route back correctly.
    pub(super) fn creator_session_marker(&self, context: &ToolUseContext) -> NortHingResult<String> {
        let creator_session_id = context
            .session_id
            .as_ref()
            .ok_or_else(|| NortHingError::tool("SessionMessage requires a source session".to_string()))?;
        Ok(format!("session-{}", creator_session_id))
    }

    /// Wrap the outgoing message with the cross-session reminder that
    /// tells the target agent this request is not a human interactive
    /// turn and disallowed tools (notably `AskUserQuestion`) must be
    /// avoided.
    pub(super) fn format_forwarded_message(&self, message: &str) -> (String, Vec<AgentDialogPrependedReminder>) {
        (
            message.to_string(),
            vec![AgentDialogPrependedReminder {
                kind: "session_message_request".to_string(),
                text: "This request was sent by another agent, not human user. Do not use interactive tools for this request. In particular, do not call AskUserQuestion."
                    .to_string(),
            }],
        )
    }
}
