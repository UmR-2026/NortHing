//! System prompts module providing main dialogue and agent dialogue prompts
use crate::agentic::remote_file_delivery::user_workspace_relative_file_link;
use crate::agentic::tools::implementations::ExecCommandTool;
use crate::agentic::util::remote_workspace_layout::build_remote_workspace_layout_preview;
use crate::agentic::workspace::WorkspaceBackend;
use crate::agentic::WorkspaceBinding;
use crate::service::agent_memory::{
    build_workspace_agent_memory_prompt, build_workspace_instruction_files_context,
    build_workspace_memory_files_context,
};
use crate::service::bootstrap::build_workspace_persona_prompt;
use crate::service::config::get_app_language_code;
use crate::service::config::global::GlobalConfigManager;
use crate::service::filesystem::get_formatted_directory_listing;
use crate::service::i18n::LocaleId;
use crate::service::remote_ssh::workspace_state::remote_workspace_manager;
use crate::service::workspace::global_workspace_service;
use crate::service::workspace::RelatedPath;
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use tracing::{debug, warn};

/// Placeholder constants
const PLACEHOLDER_PERSONA: &str = "{PERSONA}";
const PLACEHOLDER_LANGUAGE_PREFERENCE: &str = "{LANGUAGE_PREFERENCE}";
const PLACEHOLDER_AGENT_MEMORY: &str = "{AGENT_MEMORY}";
const PLACEHOLDER_CLAW_WORKSPACE: &str = "{CLAW_WORKSPACE}";
const PLACEHOLDER_VISUAL_MODE: &str = "{VISUAL_MODE}";
const PLACEHOLDER_SESSION_ID: &str = "{SESSION_ID}";
const PLACEHOLDER_DEEP_RESEARCH_REPORT_LINK: &str = "{DEEP_RESEARCH_REPORT_LINK}";
const USER_CONTEXT_PROMPT: &str =
    "As you answer the user's questions, you can use the following context.\nNote: this is a snapshot captured at the start of the conversation and may not reflect real-time changes made afterward.";
/// SSH remote host facts for system prompt (workspace tools run here, not on the local client).
#[derive(Debug, Clone)]
pub struct RemoteExecutionHints {
    pub connection_display_name: String,
    pub kernel_name: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeContextNeeds {
    pub workspace_tools: bool,
    pub exec_command: bool,
    pub exec_control: bool,
    pub computer_use: bool,
}

impl RuntimeContextNeeds {
    pub fn from_tool_names<T, I>(tool_names: I) -> Self
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
    {
        let mut needs = Self::default();
        for tool_name in tool_names {
            let tool_name = tool_name.as_ref();
            match tool_name {
                "Read" | "Write" | "Edit" | "Delete" | "LS" | "Grep" | "Glob" | "ExecCommand" | "WriteStdin"
                | "ExecControl" => {
                    needs.workspace_tools = true;
                    if tool_name == "ExecCommand" {
                        needs.exec_command = true;
                    }
                    if tool_name == "ExecControl" {
                        needs.exec_control = true;
                    }
                }
                "ComputerUse" | "ControlHub" => {
                    needs.computer_use = true;
                }
                _ => {}
            }
        }
        needs
    }

    fn is_empty(self) -> bool {
        !self.workspace_tools && !self.exec_command && !self.exec_control && !self.computer_use
    }
}

#[derive(Debug, Clone)]
pub struct PromptBuilderContext {
    pub workspace_path: String,
    pub related_paths: Vec<RelatedPath>,
    pub session_id: Option<String>,
    pub model_name: Option<String>,
    /// When set, file/shell tools target this remote environment; OS and path instructions follow it.
    pub remote_execution: Option<RemoteExecutionHints>,
    /// Pre-built tree text for `{PROJECT_LAYOUT}` when the workspace is not on the local disk.
    pub remote_project_layout: Option<String>,
    /// When `Some(false)`, system prompt append Computer use text-only guidance (no screenshot tool output).
    pub supports_image_understanding: Option<bool>,
    /// Dynamic tool listings injected outside tool descriptions for cache stability.
    pub tool_listing_sections: ToolListingSections,
    /// Runtime facts needed by the current model-visible tool set.
    pub runtime_context_needs: RuntimeContextNeeds,
    /// Remote mobile/bot turns need `computer://` links for file delivery.
    pub remote_file_delivery_channel: bool,
    /// Context window size from model config (tokens).
    pub context_window: Option<u32>,
    /// Max output tokens from model config.
    pub max_output_tokens: Option<u32>,
}

impl PromptBuilderContext {
    pub fn new(workspace_path: impl Into<String>, session_id: Option<String>, model_name: Option<String>) -> Self {
        Self {
            workspace_path: workspace_path.into().replace("\\", "/"),
            related_paths: Vec::new(),
            session_id,
            model_name,
            remote_execution: None,
            remote_project_layout: None,
            supports_image_understanding: None,
            tool_listing_sections: ToolListingSections::default(),
            runtime_context_needs: RuntimeContextNeeds::default(),
            remote_file_delivery_channel: false,
            context_window: None,
            max_output_tokens: None,
        }
    }

    pub fn with_supports_image_understanding(mut self, supports: bool) -> Self {
        self.supports_image_understanding = Some(supports);
        self
    }

    pub fn with_tool_listing_sections(mut self, sections: ToolListingSections) -> Self {
        self.tool_listing_sections = sections;
        self
    }

    pub fn with_runtime_context_needs(mut self, needs: RuntimeContextNeeds) -> Self {
        self.runtime_context_needs = needs;
        self
    }

    pub fn with_related_paths(mut self, related_paths: Vec<RelatedPath>) -> Self {
        self.related_paths = related_paths;
        self
    }

    pub fn with_remote_prompt_overlay(
        mut self,
        execution: RemoteExecutionHints,
        project_layout: Option<String>,
    ) -> Self {
        self.remote_execution = Some(execution);
        self.remote_project_layout = project_layout;
        self
    }

    pub fn with_remote_file_delivery_channel(mut self, enabled: bool) -> Self {
        self.remote_file_delivery_channel = enabled;
        self
    }

    pub fn with_context_window(mut self, context_window: u32) -> Self {
        self.context_window = Some(context_window);
        self
    }

    pub fn with_max_output_tokens(mut self, max_output_tokens: u32) -> Self {
        self.max_output_tokens = Some(max_output_tokens);
        self
    }
}

pub async fn build_prompt_context_for_workspace(
    workspace: &WorkspaceBinding,
    workspace_id: Option<&str>,
    session_id: &str,
    model_name: Option<String>,
    supports_image_understanding: Option<bool>,
    tool_listing_sections: ToolListingSections,
    runtime_context_needs: RuntimeContextNeeds,
) -> Option<PromptBuilderContext> {
    let workspace_path = workspace.root_path_string();

    let related_paths = if let Some(workspace_id) = workspace_id {
        if let Some(workspace_service) = global_workspace_service() {
            workspace_service
                .get_workspace(workspace_id)
                .await
                .map(|workspace| workspace.related_paths)
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut base = PromptBuilderContext::new(workspace_path.clone(), Some(session_id.to_string()), model_name)
        .with_related_paths(related_paths)
        .with_tool_listing_sections(tool_listing_sections)
        .with_runtime_context_needs(runtime_context_needs);
    if let Some(supports_image_understanding) = supports_image_understanding {
        base = base.with_supports_image_understanding(supports_image_understanding);
    }

    if !workspace.is_remote() {
        return Some(base);
    }

    let Some(connection_id) = workspace.connection_id() else {
        return Some(base);
    };
    let Some(manager) = remote_workspace_manager() else {
        warn!("Remote workspace active but RemoteWorkspaceStateManager is missing; using client OS hints only");
        return Some(base);
    };

    let ssh_manager = manager.get_ssh_manager().await;
    let file_service = manager.get_file_service().await;
    let (kernel_name, hostname) = if let Some(ref ssh) = ssh_manager {
        if let Some(info) = ssh.get_server_info(connection_id).await {
            (info.os_type, info.hostname)
        } else {
            ("Linux".to_string(), "remote".to_string())
        }
    } else {
        ("Linux".to_string(), "remote".to_string())
    };
    let connection_display_name = match &workspace.backend {
        WorkspaceBackend::Remote { connection_name, .. } => connection_name.clone(),
        _ => connection_id.to_string(),
    };
    let remote_layout = if let Some(ref fs) = file_service {
        match build_remote_workspace_layout_preview(fs, connection_id, &workspace_path, 200).await {
            Ok((_, preview)) => Some(preview),
            Err(e) => {
                warn!("Remote workspace layout for prompt failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    Some(base.with_remote_prompt_overlay(
        RemoteExecutionHints {
            connection_display_name,
            kernel_name,
            hostname,
        },
        remote_layout,
    ))
}

pub struct PromptBuilder {
    pub context: PromptBuilderContext,
    pub file_tree_max_entries: usize,
}

impl PromptBuilder {
    pub fn new(context: PromptBuilderContext) -> Self {
        Self {
            context,
            file_tree_max_entries: 200,
        }
    }
}

mod format;
mod partitioned_loader;
mod system_prompt;
mod tool_prompt;
mod user_context;
mod user_prompt;

pub use northhing_agent_runtime::prompt::{PrependedPromptReminders, ToolListingSections};
pub use partitioned_loader::{AgentPromptCacheIdentity, PartitionedLoader, USE_PARTITIONED_LOADER};
pub use user_context::{UserContextPolicy, UserContextSection};

#[cfg(test)]
mod tests;
