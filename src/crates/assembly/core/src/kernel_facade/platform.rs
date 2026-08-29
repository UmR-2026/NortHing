//! KernelPlatformApi implementation.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::platform::{
    AnalysisDto, ArtifactDto, CoreHealthDto, FileTreeEntryDto, ImageContextDto, InspectorDataDto, PanelDto,
    PanelsConfigDto, SkillStatusDto, TerminalConfigDto,
};

use crate::kernel_facade::lifecycle::FACADE_READY;
use crate::service::config::get_global_config_service;
use crate::service::mcp::global_mcp_service;

/// Default cap for `read_workspace_file` when `max_bytes` is `None`.
const DEFAULT_READ_MAX_BYTES: u64 = 256 * 1024;
/// Cap for binary sniff and pre-truncation byte budget.
const BINARY_SNIFF_BYTES: usize = 4096;

/// Path segments disallowed inside user-supplied relative paths. They would
/// either escape the workspace (via `..`) or surprise other OS plumbing
/// (NUL is the canonical shell terminator).
fn has_escape_segment(rel: &Path) -> bool {
    for comp in rel.components() {
        match comp {
            std::path::Component::ParentDir => return true,
            std::path::Component::Prefix(_) => return true,
            std::path::Component::RootDir => return true,
            _ => {}
        }
    }
    rel.to_string_lossy().contains('\0')
}

/// Normalize a user-supplied workspace-relative path and assert it is
/// contained within `workspace_root`. Empty or `.` paths resolve to the
/// workspace root.
fn resolve_within_workspace(workspace_root: &Path, user_path: &str) -> Result<PathBuf, KernelError> {
    if workspace_root.as_os_str().is_empty() {
        return Err(KernelError::Config("workspace root not configured".to_string()));
    }
    let trimmed = user_path.trim();
    if trimmed.is_empty() {
        return Ok(workspace_root.to_path_buf());
    }
    let relative = Path::new(trimmed);
    if relative.is_absolute() {
        return Err(KernelError::Validation(format!(
            "absolute paths are not allowed: {trimmed}"
        )));
    }
    if has_escape_segment(relative) {
        return Err(KernelError::Validation(format!("path escapes workspace: {trimmed}")));
    }
    let joined = workspace_root.join(relative);
    let canonical_root = std::path::absolute(workspace_root)
        .map_err(|e| KernelError::Config(format!("canonicalize workspace root: {e}")))?;
    let canonical_target =
        std::path::absolute(&joined).map_err(|e| KernelError::Validation(format!("canonicalize {trimmed}: {e}")))?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(KernelError::Validation(format!("path escapes workspace: {trimmed}")));
    }
    Ok(canonical_target)
}

/// Containment check using the same canonical-prefix rule as
/// `resolve_within_workspace`, used to re-fence recursive descendants that
/// come from the filesystem rather than the user.
fn is_within(workspace_root: &Path, candidate: &Path) -> bool {
    let Ok(canonical_root) = std::path::absolute(workspace_root) else {
        return false;
    };
    let Ok(canonical_target) = std::path::absolute(candidate) else {
        return false;
    };
    canonical_target.starts_with(canonical_root)
}

/// Path string for the DTO: relative to the workspace root with forward
/// slashes so a Windows path reads the same as a POSIX path in the UI.
fn relative_to_root(workspace_root: &Path, target: &Path) -> String {
    let rel = target.strip_prefix(workspace_root).unwrap_or(target);
    let s = rel.to_string_lossy().to_string();
    s.replace('\\', "/")
}
/// Hard ceiling on individual `read_workspace_file` requests, regardless of the
/// caller-supplied value. Defends the facade against callers claiming
/// astronomically large reads (e.g. `u64::MAX`).
const HARD_READ_CAP_BYTES: u64 = 4 * 1024 * 1024;
/// Hard ceiling on tree depth, regardless of the caller-supplied value. Five
/// levels matches the consult-room preview use case and bounds explosion.
const HARD_TREE_DEPTH: u32 = 5;
/// Per-directory entry cap. Avoids pathological projects where a single
/// `node_modules`/`target` dir holds hundreds of thousands of files.
const TREE_ENTRY_CAP: usize = 1024;

#[async_trait]
impl northhing_kernel_api::KernelPlatformApi for super::KernelFacade {
    async fn open_terminal(&self, _config: TerminalConfigDto) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: terminal open requires host UI integration.
        Err(KernelError::Internal("not yet wired: open_terminal".to_string()))
    }

    async fn analyze_image(&self, _context: ImageContextDto) -> Result<AnalysisDto, KernelError> {
        Err(KernelError::Internal("not yet wired: analyze_image".to_string()))
    }

    async fn get_core_health(&self) -> Result<CoreHealthDto, KernelError> {
        Ok(CoreHealthDto {
            healthy: FACADE_READY.load(std::sync::atomic::Ordering::SeqCst),
            details: if FACADE_READY.load(std::sync::atomic::Ordering::SeqCst) {
                vec!["core initialized".to_string()]
            } else {
                vec!["core not yet initialized".to_string()]
            },
        })
    }

    async fn read_panels_config(&self) -> Result<PanelsConfigDto, KernelError> {
        // F3: read panels.json from product config directory.
        let config_dir =
            dirs::config_dir().ok_or_else(|| KernelError::Config("cannot find config directory".to_string()))?;
        let panels_path = config_dir.join("northhing").join("config").join("panels.json");
        if !panels_path.exists() {
            return Ok(PanelsConfigDto { panels: vec![] });
        }
        let content = tokio::fs::read_to_string(&panels_path)
            .await
            .map_err(|e| KernelError::Runtime(format!("read panels.json: {e}")))?;
        serde_json::from_str(&content).map_err(|e| KernelError::Runtime(format!("parse panels.json: {e}")))
    }

    async fn is_onboarding_complete(&self) -> Result<bool, KernelError> {
        // NEEDS_CONTEXT: onboarding_completed is desktop UI state, not core GlobalConfig.
        Err(KernelError::Internal(
            "not yet wired: is_onboarding_complete".to_string(),
        ))
    }

    async fn complete_onboarding(&self) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: onboarding_completed is desktop UI state, not core GlobalConfig.
        Err(KernelError::Internal("not yet wired: complete_onboarding".to_string()))
    }

    async fn get_inspector_data(&self) -> Result<InspectorDataDto, KernelError> {
        // Forward to global config for model name, MCP service for MCP status.
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let config: crate::service::config::GlobalConfig = cfg_svc
            .config(None)
            .await
            .map_err(|e| KernelError::Config(format!("get global config: {e}")))?;
        let model_name = config
            .ai
            .default_models
            .primary
            .clone()
            .unwrap_or_else(|| "not configured".to_string());

        // Get MCP status.
        let mcp_status = if let Some(mcp_svc) = global_mcp_service() {
            match mcp_svc.config_service().load_all_configs().await {
                Ok(configs) => {
                    let mut statuses = Vec::new();
                    for config in configs {
                        let probe_status = tokio::time::timeout(
                            Duration::from_millis(30),
                            mcp_svc.server_manager().get_server_status(&config.id),
                        )
                        .await;
                        let kind = crate::kernel_facade::helpers::map_mcp_probe_status(probe_status);
                        statuses.push(northhing_kernel_api::settings::MCPServerStatusDto {
                            id: config.id,
                            status: kind,
                        });
                    }
                    statuses
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        let skills_status = {
            use crate::agentic::tools::implementations::skills::skill_registry;
            let registry = skill_registry();
            let skills = registry.get_all_skills().await;
            skills
                .into_iter()
                .map(|s| SkillStatusDto {
                    skill_id: s.key,
                    name: s.name,
                    enabled: !s.is_shadowed,
                    status: if s.is_shadowed {
                        "shadowed".to_string()
                    } else {
                        "available".to_string()
                    },
                })
                .collect()
        };

        Ok(InspectorDataDto {
            model_name,
            mcp_status,
            skills_status,
        })
    }

    async fn list_artifacts(&self, _session_id: &super::SessionId) -> Result<Vec<ArtifactDto>, KernelError> {
        // NEEDS_CONTEXT: artifact storage not yet wired.
        Err(KernelError::Internal("not yet wired: list_artifacts".to_string()))
    }

    async fn list_workspace_tree(
        &self,
        dir: &str,
        max_depth: Option<u32>,
    ) -> Result<Vec<FileTreeEntryDto>, KernelError> {
        let workspace = crate::kernel_facade::helpers::default_workspace_path();
        let workspace_root = PathBuf::from(&workspace);
        let target = resolve_within_workspace(&workspace_root, dir)?;
        if !target.is_dir() {
            return Err(KernelError::NotFound(format!(
                "directory not found: {}",
                target.display()
            )));
        }
        let depth_limit = max_depth.map(|d| d.min(HARD_TREE_DEPTH)).unwrap_or(0);
        let mut out = Vec::new();
        let mut stack: Vec<(PathBuf, u32)> = vec![(target, 0)];
        while let Some((cur, depth)) = stack.pop() {
            if out.len() >= TREE_ENTRY_CAP {
                break;
            }
            let mut rd = match tokio::fs::read_dir(&cur).await {
                Ok(rd) => rd,
                Err(e) => {
                    return Err(KernelError::Runtime(format!("read_dir {} failed: {e}", cur.display())));
                }
            };
            while let Ok(Some(entry)) = rd.next_entry().await {
                if out.len() >= TREE_ENTRY_CAP {
                    break;
                }
                let p = entry.path();
                // Re-fence each descendant against the workspace root.
                if !is_within(&workspace_root, &p) {
                    return Err(KernelError::Validation(format!(
                        "entry escaped workspace: {}",
                        p.display()
                    )));
                }
                let name = entry.file_name().to_string_lossy().to_string();
                // Pick the canonical metadata so symlinks do not slip past.
                let meta = match tokio::fs::symlink_metadata(&p).await {
                    Ok(m) => m,
                    Err(e) => {
                        return Err(KernelError::Runtime(format!("metadata {} failed: {e}", p.display())));
                    }
                };
                if meta.file_type().is_symlink() {
                    // Skip symlinks — they could resolve outside the workspace.
                    continue;
                }
                let is_dir = meta.is_dir();
                let rel = relative_to_root(&workspace_root, &p);
                let size_bytes = if is_dir { None } else { Some(meta.len()) };
                out.push(FileTreeEntryDto {
                    path: rel,
                    name,
                    is_dir,
                    size_bytes,
                });
                if is_dir && depth < depth_limit {
                    stack.push((p, depth + 1));
                }
            }
        }
        Ok(out)
    }

    async fn read_workspace_file(&self, path: &str, max_bytes: Option<u64>) -> Result<String, KernelError> {
        let workspace = crate::kernel_facade::helpers::default_workspace_path();
        let workspace_root = PathBuf::from(&workspace);
        let target = resolve_within_workspace(&workspace_root, path)?;
        if !target.is_file() {
            return Err(KernelError::NotFound(format!("file not found: {}", target.display())));
        }
        let cap = max_bytes.unwrap_or(DEFAULT_READ_MAX_BYTES).min(HARD_READ_CAP_BYTES);
        let meta = tokio::fs::metadata(&target)
            .await
            .map_err(|e| KernelError::Runtime(format!("metadata {} failed: {e}", target.display())))?;
        if meta.len() > cap {
            return Err(KernelError::Validation(format!(
                "file too large: {} bytes (cap {})",
                meta.len(),
                cap
            )));
        }
        // Read whole file (already size-capped). Binary sniff before UTF-8 decode.
        let bytes = tokio::fs::read(&target)
            .await
            .map_err(|e| KernelError::Runtime(format!("read {} failed: {e}", target.display())))?;
        if bytes.iter().take(BINARY_SNIFF_BYTES).any(|b| *b == 0) {
            return Err(KernelError::Validation("binary file not previewable".to_string()));
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                return Err(KernelError::Validation("non-utf8 file not previewable".to_string()));
            }
        };
        Ok(text)
    }
}
