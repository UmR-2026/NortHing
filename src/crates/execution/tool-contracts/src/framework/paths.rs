//! Tool path resolution, runtime-URI contracts, and path policy.
//!
//! R37b sibling: path backend / resolution DTOs, runtime-URI parsing and
//! building, host and posix path normalization, and path operation policy.
//! Split verbatim from `framework.rs`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPathBackend {
    Local,
    RemoteWorkspace,
}

#[derive(Debug, Clone)]
pub struct ToolPathResolution {
    pub requested_path: String,
    pub logical_path: String,
    pub resolved_path: String,
    pub backend: ToolPathBackend,
    pub runtime_scope: Option<String>,
    pub runtime_root: Option<PathBuf>,
}

impl ToolPathResolution {
    pub fn uses_remote_workspace_backend(&self) -> bool {
        matches!(self.backend, ToolPathBackend::RemoteWorkspace)
    }

    pub fn is_runtime_artifact(&self) -> bool {
        self.runtime_scope.is_some()
    }

    pub fn logical_child_path(&self, absolute_child_path: &Path) -> Option<String> {
        let scope = self.runtime_scope.as_deref()?;
        let root = self.runtime_root.as_ref()?;
        let relative = absolute_child_path.strip_prefix(root).ok()?;
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        build_northhing_runtime_uri(scope, &relative_str).ok()
    }
}

pub const NORTHHING_RUNTIME_URI_PREFIX: &str = "northhing://runtime/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNortHingRuntimeUri {
    pub workspace_scope: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPathContractError {
    EmptyRuntimeArtifactPath,
    RuntimeArtifactPathEscapesRoot,
    UnsupportedRuntimeUri { uri: String },
    MissingRuntimeUriWorkspaceScope,
    MissingRuntimeUriArtifactPath,
    EmptyRuntimeWorkspaceScope,
    RuntimeUriScopeMismatch { workspace_scope: String },
    MissingRuntimeRoot,
    EmptyPath,
    MissingWorkspaceRoot { path: String },
}

impl fmt::Display for ToolPathContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRuntimeArtifactPath => {
                write!(formatter, "Runtime artifact path cannot be empty")
            }
            Self::RuntimeArtifactPathEscapesRoot => {
                write!(formatter, "Runtime artifact path cannot escape its root")
            }
            Self::UnsupportedRuntimeUri { uri } => {
                write!(formatter, "Unsupported runtime URI: {uri}")
            }
            Self::MissingRuntimeUriWorkspaceScope => {
                write!(formatter, "Runtime URI is missing workspace scope")
            }
            Self::MissingRuntimeUriArtifactPath => {
                write!(formatter, "Runtime URI is missing artifact path")
            }
            Self::EmptyRuntimeWorkspaceScope => {
                write!(formatter, "Runtime URI workspace scope cannot be empty")
            }
            Self::RuntimeUriScopeMismatch { workspace_scope } => {
                write!(
                    formatter,
                    "Runtime URI scope '{workspace_scope}' does not match the current workspace"
                )
            }
            Self::MissingRuntimeRoot => {
                write!(formatter, "A workspace is required to resolve runtime artifacts")
            }
            Self::EmptyPath => write!(formatter, "path cannot be empty"),
            Self::MissingWorkspaceRoot { path } => {
                write!(
                    formatter,
                    "A workspace path is required to resolve relative path: {path}"
                )
            }
        }
    }
}

impl std::error::Error for ToolPathContractError {}

pub fn is_northhing_runtime_uri(path: &str) -> bool {
    path.trim().starts_with(NORTHHING_RUNTIME_URI_PREFIX)
}

pub fn normalize_host_path(path: &str) -> String {
    let path = Path::new(path);
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            component => components.push(component),
        }
    }
    components.iter().collect::<PathBuf>().to_string_lossy().to_string()
}

pub fn resolve_host_path_with_workspace(
    path: &str,
    workspace_root: Option<&Path>,
) -> Result<String, ToolPathContractError> {
    if Path::new(path).is_absolute() {
        Ok(normalize_host_path(path))
    } else {
        let base_path =
            workspace_root.ok_or_else(|| ToolPathContractError::MissingWorkspaceRoot { path: path.to_string() })?;

        Ok(normalize_host_path(base_path.join(path).to_string_lossy().as_ref()))
    }
}

pub fn resolve_host_path(path: &str) -> Result<String, ToolPathContractError> {
    resolve_host_path_with_workspace(path, None)
}

pub fn resolve_workspace_tool_path(
    path: &str,
    workspace_root: Option<&str>,
    workspace_is_remote: bool,
) -> Result<String, ToolPathContractError> {
    if workspace_is_remote {
        posix_resolve_path_with_workspace(path, workspace_root)
    } else {
        resolve_host_path_with_workspace(path, workspace_root.map(Path::new))
    }
}

pub fn resolve_tool_path_with_context(
    path: &str,
    workspace_root: Option<&str>,
    workspace_is_remote: bool,
    workspace_scope: Option<&str>,
    runtime_root: Option<PathBuf>,
) -> Result<ToolPathResolution, ToolPathContractError> {
    if is_northhing_runtime_uri(path) {
        let parsed = parse_northhing_runtime_uri(path)?;
        let scope_matches =
            parsed.workspace_scope == "current" || workspace_scope == Some(parsed.workspace_scope.as_str());
        if !scope_matches {
            return Err(ToolPathContractError::RuntimeUriScopeMismatch {
                workspace_scope: parsed.workspace_scope,
            });
        }

        let runtime_root = runtime_root.ok_or(ToolPathContractError::MissingRuntimeRoot)?;
        let mut resolved_path = runtime_root.clone();
        for segment in parsed.relative_path.split('/') {
            resolved_path.push(segment);
        }

        let effective_scope = workspace_scope
            .map(str::to_string)
            .unwrap_or_else(|| parsed.workspace_scope.clone());
        let logical_path = build_northhing_runtime_uri(&effective_scope, &parsed.relative_path)?;

        return Ok(ToolPathResolution {
            requested_path: path.to_string(),
            logical_path,
            resolved_path: resolved_path.to_string_lossy().to_string(),
            backend: ToolPathBackend::Local,
            runtime_scope: Some(effective_scope),
            runtime_root: Some(runtime_root),
        });
    }

    let resolved_path = resolve_workspace_tool_path(path, workspace_root, workspace_is_remote)?;
    Ok(ToolPathResolution {
        requested_path: path.to_string(),
        logical_path: resolved_path.clone(),
        resolved_path,
        backend: if workspace_is_remote {
            ToolPathBackend::RemoteWorkspace
        } else {
            ToolPathBackend::Local
        },
        runtime_scope: None,
        runtime_root: None,
    })
}

pub fn tool_path_is_effectively_absolute(path: &str, workspace_is_remote: bool) -> bool {
    if is_northhing_runtime_uri(path) {
        return true;
    }

    if workspace_is_remote {
        posix_style_path_is_absolute(path)
    } else {
        Path::new(path).is_absolute()
    }
}

pub fn normalize_runtime_relative_path(path: &str) -> Result<String, ToolPathContractError> {
    let normalized = path.trim().replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        return Err(ToolPathContractError::EmptyRuntimeArtifactPath);
    }

    let mut segments = Vec::new();
    for part in trimmed.split('/') {
        match part {
            "" | "." => continue,
            ".." => return Err(ToolPathContractError::RuntimeArtifactPathEscapesRoot),
            value => segments.push(value.to_string()),
        }
    }

    if segments.is_empty() {
        return Err(ToolPathContractError::EmptyRuntimeArtifactPath);
    }

    Ok(segments.join("/"))
}

pub fn parse_northhing_runtime_uri(path: &str) -> Result<ParsedNortHingRuntimeUri, ToolPathContractError> {
    let trimmed = path.trim();
    let suffix = trimmed
        .strip_prefix(NORTHHING_RUNTIME_URI_PREFIX)
        .ok_or_else(|| ToolPathContractError::UnsupportedRuntimeUri { uri: path.to_string() })?;

    let mut parts = suffix.splitn(2, '/');
    let workspace_scope = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(ToolPathContractError::MissingRuntimeUriWorkspaceScope)?
        .to_string();
    let relative_path = parts
        .next()
        .ok_or(ToolPathContractError::MissingRuntimeUriArtifactPath)?;

    Ok(ParsedNortHingRuntimeUri {
        workspace_scope,
        relative_path: normalize_runtime_relative_path(relative_path)?,
    })
}

pub fn build_northhing_runtime_uri(
    workspace_scope: &str,
    relative_path: &str,
) -> Result<String, ToolPathContractError> {
    let scope = workspace_scope.trim();
    if scope.is_empty() {
        return Err(ToolPathContractError::EmptyRuntimeWorkspaceScope);
    }

    Ok(format!(
        "{}{}/{}",
        NORTHHING_RUNTIME_URI_PREFIX,
        scope,
        normalize_runtime_relative_path(relative_path)?
    ))
}

pub fn build_tool_runtime_artifact_reference(
    relative_path: &str,
    runtime_root: Option<&Path>,
    workspace_scope: Option<&str>,
    emit_runtime_uri: bool,
) -> Result<String, ToolPathContractError> {
    let normalized_relative_path = normalize_runtime_relative_path(relative_path)?;
    if emit_runtime_uri {
        return build_northhing_runtime_uri(workspace_scope.unwrap_or("current"), &normalized_relative_path);
    }

    let runtime_root = runtime_root.ok_or(ToolPathContractError::MissingRuntimeRoot)?;
    let mut resolved_path = runtime_root.to_path_buf();
    for segment in normalized_relative_path.split('/') {
        resolved_path.push(segment);
    }

    Ok(resolved_path.to_string_lossy().to_string())
}

pub fn build_tool_session_runtime_artifact_reference(
    session_id: &str,
    relative_path: &str,
    runtime_root: Option<&Path>,
    workspace_scope: Option<&str>,
    emit_runtime_uri: bool,
) -> Result<String, ToolPathContractError> {
    let normalized_relative_path = normalize_runtime_relative_path(relative_path)?;
    build_tool_runtime_artifact_reference(
        &format!("sessions/{}/{}", session_id, normalized_relative_path),
        runtime_root,
        workspace_scope,
        emit_runtime_uri,
    )
}

pub fn posix_style_path_is_absolute(path: &str) -> bool {
    let path = path.trim().replace('\\', "/");
    path.starts_with('/')
}

pub fn normalize_absolute_posix_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let is_absolute = normalized.starts_with('/');
    let mut segments = Vec::new();

    for segment in normalized.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if !segments.is_empty() {
                    segments.pop();
                }
            }
            value => segments.push(value.to_string()),
        }
    }

    let body = segments.join("/");
    if is_absolute {
        if body.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", body)
        }
    } else {
        body
    }
}

pub fn is_remote_posix_path_within_root(path: &str, root: &str) -> bool {
    let normalized_path = normalize_absolute_posix_path(path);
    let normalized_root = normalize_absolute_posix_path(root);

    if !normalized_path.starts_with('/') || !normalized_root.starts_with('/') {
        return false;
    }

    if normalized_root == "/" {
        return true;
    }

    normalized_path == normalized_root
        || normalized_path
            .strip_prefix(&normalized_root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub fn posix_resolve_path_with_workspace(
    path: &str,
    workspace_root: Option<&str>,
) -> Result<String, ToolPathContractError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(ToolPathContractError::EmptyPath);
    }

    let normalized_input = path.replace('\\', "/");

    let combined = if posix_style_path_is_absolute(&normalized_input) {
        normalized_input
    } else {
        let base = workspace_root
            .ok_or_else(|| ToolPathContractError::MissingWorkspaceRoot { path: path.to_string() })?
            .trim()
            .replace('\\', "/");
        let base = base.trim_end_matches('/');
        format!("{}/{}", base, normalized_input)
    };

    Ok(normalize_absolute_posix_path(&combined))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPathOperation {
    Write,
    Edit,
    Delete,
}

impl ToolPathOperation {
    pub fn verb(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Edit => "edit",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPathPolicy {
    #[serde(default)]
    pub write_roots: Vec<String>,
    #[serde(default)]
    pub edit_roots: Vec<String>,
    #[serde(default)]
    pub delete_roots: Vec<String>,
}

impl ToolPathPolicy {
    pub fn roots_for(&self, operation: ToolPathOperation) -> &[String] {
        match operation {
            ToolPathOperation::Write => &self.write_roots,
            ToolPathOperation::Edit => &self.edit_roots,
            ToolPathOperation::Delete => &self.delete_roots,
        }
    }

    pub fn is_restricted(&self, operation: ToolPathOperation) -> bool {
        !self.roots_for(operation).is_empty()
    }
}

pub fn is_tool_path_allowed_by_resolved_roots<E>(
    resolution: &ToolPathResolution,
    resolved_roots: &[ToolPathResolution],
    mut root_contains_path: impl FnMut(&ToolPathResolution, &ToolPathResolution) -> Result<bool, E>,
) -> Result<bool, E> {
    for root in resolved_roots {
        if root.backend != resolution.backend {
            continue;
        }

        if root_contains_path(resolution, root)? {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn build_tool_path_policy_denial_message(
    logical_path: &str,
    operation: ToolPathOperation,
    allowed_roots: &[String],
) -> String {
    format!(
        "Path '{}' is not allowed for {}. Allowed roots: {}",
        logical_path,
        operation.verb(),
        allowed_roots.join(", ")
    )
}
