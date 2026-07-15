use super::PromptBuilder;
use super::USER_CONTEXT_PROMPT;
use crate::service::agent_memory::{build_workspace_instruction_files_context, build_workspace_memory_files_context};
use crate::service::filesystem::get_formatted_directory_listing;
use crate::util::errors::NortHingResult;
use northhing_agent_runtime::prompt::{UserContextPolicy, UserContextSection};
use std::path::Path;
use tracing::warn;

impl PromptBuilder {
    pub fn workspace_context(&self) -> String {
        let related_paths_section = if self.context.related_paths.is_empty() {
            String::new()
        } else {
            let items = self
                .context
                .related_paths
                .iter()
                .map(|related_path| {
                    let path = related_path.path.replace("\\", "/");
                    match related_path.description.as_deref().map(str::trim) {
                        Some(description) if !description.is_empty() => {
                            format!("  - {} — {}", path, description)
                        }
                        _ => format!("  - {}", path),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "- Related directories (user-specified directories related to this workspace):\n{}",
                items
            )
        };

        if let Some(remote) = &self.context.remote_execution {
            format!(
                r#"## Workspace Context
<workspace_context>
- Workspace root (file tools, Glob, LS, ExecCommand on workspace): {}
{}
- Execution environment: **Remote SSH** — connection "{}".
- Remote host: {} (uname/kernel: {})
</workspace_context>
"#,
                self.context.workspace_path,
                if related_paths_section.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", related_paths_section)
                },
                remote.connection_display_name.replace('"', "'"),
                remote.hostname.replace('"', "'"),
                remote.kernel_name.replace('"', "'"),
            )
        } else {
            format!(
                r#"## Workspace Context
<workspace_context>
- Current Working Directory: {}
{}
</workspace_context>
"#,
                self.context.workspace_path,
                if related_paths_section.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", related_paths_section)
                }
            )
        }
    }

    /// Get workspace file list
    pub fn project_layout(&self) -> String {
        if let Some(remote_layout) = &self.context.remote_project_layout {
            let mut project_layout = "## Workspace Layout\n<project_layout>\n".to_string();
            project_layout
                .push_str("Below is a snapshot of the current workspace's file structure on the **remote** host.\n\n");
            project_layout.push_str(remote_layout);
            project_layout.push_str("\n</project_layout>\n\n");
            return project_layout;
        }

        let formatted_listing =
            get_formatted_directory_listing(&self.context.workspace_path, self.file_tree_max_entries).unwrap_or_else(
                |e| crate::service::filesystem::FormattedDirectoryListing {
                    reached_limit: false,
                    text: format!("Error listing directory: {}", e),
                },
            );
        let mut project_layout = "## Workspace Layout\n<project_layout>\n".to_string();
        if formatted_listing.reached_limit {
            project_layout.push_str(&format!(
                "Below is a snapshot of the current workspace's file structure (showing up to {} entries).\n\n",
                self.file_tree_max_entries
            ));
        } else {
            project_layout.push_str("Below is a snapshot of the current workspace's file structure.\n\n");
        }
        project_layout.push_str(&formatted_listing.text);
        project_layout.push_str("\n</project_layout>\n\n");
        project_layout
    }

    pub async fn build_user_context_reminder(&self, policy: &UserContextPolicy) -> Option<String> {
        let mut additional_sections = Vec::new();

        if policy.includes(UserContextSection::WorkspaceContext) {
            additional_sections.push(self.workspace_context());
        }

        if self.context.remote_execution.is_none() {
            let workspace = Path::new(&self.context.workspace_path);
            if policy.includes(UserContextSection::WorkspaceInstructions) {
                match build_workspace_instruction_files_context(workspace).await {
                    Ok(Some(prompt)) => additional_sections.push(prompt),
                    Ok(None) => {}
                    Err(e) => warn!(
                        "Failed to build workspace instruction context: path={} error={}",
                        workspace.display(),
                        e
                    ),
                }
            }
            if policy.includes(UserContextSection::WorkspaceMemoryFiles) {
                match build_workspace_memory_files_context(workspace).await {
                    Ok(Some(prompt)) => additional_sections.push(prompt),
                    Ok(None) => {}
                    Err(e) => warn!(
                        "Failed to build workspace memory context: path={} error={}",
                        workspace.display(),
                        e
                    ),
                }
            }
        }

        if policy.includes(UserContextSection::ProjectLayout) {
            additional_sections.push(self.project_layout());
        }

        if additional_sections.is_empty() {
            None
        } else {
            Some(format!(
                "# User Context\n{}\n\n{}",
                USER_CONTEXT_PROMPT,
                additional_sections
                    .into_iter()
                    .map(|section| section.trim().to_string())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ))
        }
    }
}
