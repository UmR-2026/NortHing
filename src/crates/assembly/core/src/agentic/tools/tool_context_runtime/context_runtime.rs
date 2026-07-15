use crate::agentic::tools::framework::{
    build_tool_path_policy_denial_message, build_tool_runtime_artifact_reference,
    build_tool_session_runtime_artifact_reference, is_tool_path_allowed_by_resolved_roots,
    resolve_tool_path_with_context, tool_path_is_effectively_absolute, Tool, ToolPathBackend, ToolPathResolution,
};
use crate::agentic::tools::post_call_hooks;
use crate::agentic::tools::restrictions::{
    is_local_path_within_root, is_remote_posix_path_within_root, ToolPathOperation,
};
use crate::agentic::tools::workspace_paths::{
    build_northhing_runtime_uri, is_northhing_runtime_uri, normalize_runtime_relative_path,
};
use crate::infrastructure::path_manager_arc;
use crate::service::remote_ssh::workspace_state::remote_workspace_runtime_root;
use crate::service::{workspace_runtime_service_arc, WorkspaceRuntimeContext};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) async fn call_with_tool_runtime_hooks(
    tool_name: &str,
    input: &Value,
    context: &super::context_init::ToolUseContext,
    call_impl: impl std::future::Future<Output = NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>>>,
) -> NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>> {
    let result = if let Some(cancellation_token) = context.cancellation_token() {
        tokio::select! {
            result = call_impl => {
                result
            }
            _ = cancellation_token.cancelled() => {
                Err(NortHingError::Cancelled("Tool execution cancelled".to_string()))
            }
        }
    } else {
        call_impl.await
    };

    if result.is_ok() {
        post_call_hooks::record_successful_tool_call(tool_name, input, context);
    }

    result
}

pub(crate) async fn call_tool_with_runtime_hooks<T: Tool + ?Sized>(
    tool: &T,
    input: &Value,
    context: &super::context_init::ToolUseContext,
) -> NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>> {
    call_with_tool_runtime_hooks(tool.name(), input, context, tool.call_impl(input, context)).await
}

impl super::context_init::ToolUseContext {
    pub fn ws_fs(&self) -> Option<&dyn crate::agentic::workspace::WorkspaceFileSystem> {
        self.workspace_services().map(|s| s.fs.as_ref())
    }

    pub fn ws_shell(&self) -> Option<&dyn crate::agentic::workspace::WorkspaceShell> {
        self.workspace_services().map(|s| s.shell.as_ref())
    }

    pub fn enforce_tool_runtime_restrictions(&self, tool_name: &str) -> NortHingResult<()> {
        self.runtime_tool_restrictions
            .ensure_tool_allowed(tool_name)
            .map_err(Into::into)
    }

    pub fn enforce_path_operation(
        &self,
        operation: ToolPathOperation,
        resolution: &ToolPathResolution,
    ) -> NortHingResult<()> {
        let allowed_roots = self.runtime_tool_restrictions.path_policy.roots_for(operation);
        if allowed_roots.is_empty() {
            return Ok(());
        }

        let mut resolved_roots = Vec::with_capacity(allowed_roots.len());
        for root in allowed_roots {
            resolved_roots.push(self.resolve_tool_path(root)?);
        }

        let is_allowed = is_tool_path_allowed_by_resolved_roots(
            resolution,
            &resolved_roots,
            |resolution, root| -> NortHingResult<bool> {
                match resolution.backend {
                    ToolPathBackend::Local => {
                        is_local_path_within_root(Path::new(&resolution.resolved_path), Path::new(&root.resolved_path))
                    }
                    ToolPathBackend::RemoteWorkspace => Ok(is_remote_posix_path_within_root(
                        &resolution.resolved_path,
                        &root.resolved_path,
                    )),
                }
            },
        )?;

        if is_allowed {
            return Ok(());
        }

        Err(NortHingError::validation(build_tool_path_policy_denial_message(
            &resolution.logical_path,
            operation,
            allowed_roots,
        )))
    }

    /// Resolve a user or model-supplied path for file/shell tools. Uses POSIX semantics when the
    /// workspace is remote SSH so Windows-hosted clients still resolve `/home/...` correctly.
    pub fn resolve_workspace_tool_path(&self, path: &str) -> NortHingResult<String> {
        let workspace_root_owned = self.workspace.as_ref().map(|w| w.root_path_string()).ok_or_else(|| {
            NortHingError::tool(format!("A workspace path is required to resolve tool path: {}", path))
        })?;
        let resolved_path = crate::agentic::tools::workspace_paths::resolve_workspace_tool_path(
            path,
            Some(workspace_root_owned.as_str()),
            self.is_remote(),
        )?;

        // Remote SSH workspaces stay contained to the opened project tree. Local desktop
        // sessions may use any host path the OS user can access (Bash already has the same
        // reach); optional `path_policy` roots still apply via `enforce_path_operation`.
        if self.is_remote() && !is_remote_posix_path_within_root(&resolved_path, &workspace_root_owned) {
            return Err(NortHingError::tool(format!(
                "Path '{}' resolves outside current workspace '{}': {}",
                path, workspace_root_owned, resolved_path
            )));
        }

        Ok(resolved_path)
    }

    pub fn current_workspace_runtime_root(&self) -> NortHingResult<PathBuf> {
        let workspace = self
            .workspace
            .as_ref()
            .ok_or_else(|| NortHingError::tool("A workspace is required to resolve runtime artifacts".to_string()))?;

        if workspace.is_remote() {
            let identity = &workspace.session_identity;
            Ok(remote_workspace_runtime_root(
                &identity.hostname,
                identity.logical_workspace_path(),
            ))
        } else {
            Ok(path_manager_arc().project_runtime_root(workspace.root_path()))
        }
    }

    pub fn current_workspace_scope(&self) -> Option<String> {
        self.workspace
            .as_ref()
            .and_then(|workspace| workspace.workspace_id.clone())
    }

    pub async fn ensure_current_workspace_runtime(&self) -> NortHingResult<WorkspaceRuntimeContext> {
        let workspace = self
            .workspace
            .as_ref()
            .ok_or_else(|| NortHingError::tool("A workspace is required to ensure runtime artifacts".to_string()))?;

        let runtime_service = workspace_runtime_service_arc();
        Ok(runtime_service
            .ensure_runtime_for_workspace_binding(workspace)
            .await?
            .context)
    }

    pub fn should_emit_runtime_uri(&self) -> bool {
        self.is_remote()
    }

    pub fn build_runtime_uri(&self, relative_path: &str) -> NortHingResult<String> {
        let scope = self.current_workspace_scope().unwrap_or_else(|| "current".to_string());
        build_northhing_runtime_uri(&scope, &normalize_runtime_relative_path(relative_path)?)
    }

    pub fn build_runtime_artifact_reference(&self, relative_path: &str) -> NortHingResult<String> {
        let runtime_root = if self.should_emit_runtime_uri() {
            None
        } else {
            Some(self.current_workspace_runtime_root()?)
        };
        build_tool_runtime_artifact_reference(
            relative_path,
            runtime_root.as_deref(),
            self.current_workspace_scope().as_deref(),
            self.should_emit_runtime_uri(),
        )
        .map_err(|error| NortHingError::tool(error.to_string()))
    }

    pub fn build_session_runtime_artifact_reference(
        &self,
        session_id: &str,
        relative_path: &str,
    ) -> NortHingResult<String> {
        let runtime_root = if self.should_emit_runtime_uri() {
            None
        } else {
            Some(self.current_workspace_runtime_root()?)
        };
        build_tool_session_runtime_artifact_reference(
            session_id,
            relative_path,
            runtime_root.as_deref(),
            self.current_workspace_scope().as_deref(),
            self.should_emit_runtime_uri(),
        )
        .map_err(|error| NortHingError::tool(error.to_string()))
    }

    pub fn current_workspace_session_dir(&self, session_id: &str) -> NortHingResult<PathBuf> {
        Ok(self.current_workspace_runtime_root()?.join("sessions").join(session_id))
    }

    pub fn current_workspace_session_tool_results_dir(&self, session_id: &str) -> NortHingResult<PathBuf> {
        Ok(self.current_workspace_session_dir(session_id)?.join("tool-results"))
    }

    pub fn current_workspace_session_tool_result_path(
        &self,
        session_id: &str,
        file_name: &str,
    ) -> NortHingResult<PathBuf> {
        Ok(self
            .current_workspace_session_tool_results_dir(session_id)?
            .join(file_name))
    }

    pub fn resolve_tool_path(&self, path: &str) -> NortHingResult<ToolPathResolution> {
        if is_northhing_runtime_uri(path) {
            let workspace_scope = self.current_workspace_scope();
            let runtime_root = if self.workspace.is_some() {
                Some(self.current_workspace_runtime_root()?)
            } else {
                None
            };
            return resolve_tool_path_with_context(
                path,
                None,
                self.is_remote(),
                workspace_scope.as_deref(),
                runtime_root,
            )
            .map_err(|error| NortHingError::tool(error.to_string()));
        }

        let workspace_root_owned = self.workspace.as_ref().map(|w| w.root_path_string()).ok_or_else(|| {
            NortHingError::tool(format!("A workspace path is required to resolve tool path: {}", path))
        })?;

        resolve_tool_path_with_context(
            path,
            Some(workspace_root_owned.as_str()),
            self.is_remote(),
            self.current_workspace_scope().as_deref(),
            None,
        )
        .map_err(|error| NortHingError::tool(error.to_string()))
    }

    /// Whether `path` is absolute for the active workspace (POSIX `/` for remote SSH).
    pub fn workspace_path_is_effectively_absolute(&self, path: &str) -> bool {
        tool_path_is_effectively_absolute(path, self.is_remote())
    }
}
