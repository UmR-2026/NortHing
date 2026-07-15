use super::super::types::{RuntimeMigrationRecord, WorkspaceRuntimeContext, WorkspaceRuntimeTarget};
use super::state::WorkspaceRuntimeService;
use super::state::{RuntimeMigrationSpec, RuntimeMigrationStrategy};
use crate::service::remote_ssh::workspace_state::{remote_root_to_mirror_subpath, sanitize_ssh_hostname_for_mirror};
use crate::service::session::{SessionMetadataStore, SessionMetadataStoreError, StoredSessionMetadataFile};
use crate::util::errors::{NortHingError, NortHingResult};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

impl WorkspaceRuntimeService {
    pub(crate) fn remote_workspace_runtime_root(&self, ssh_host: &str, remote_root_norm: &str) -> PathBuf {
        self.path_manager
            .northhing_home_dir()
            .join("remote_ssh")
            .join(sanitize_ssh_hostname_for_mirror(ssh_host))
            .join(remote_root_to_mirror_subpath(remote_root_norm))
    }

    pub(crate) fn migration_specs_for_context(&self, context: &WorkspaceRuntimeContext) -> Vec<RuntimeMigrationSpec> {
        match &context.target {
            WorkspaceRuntimeTarget::LocalWorkspace { workspace_root } => {
                let legacy_project_root = self.path_manager.project_root(workspace_root);
                vec![
                    RuntimeMigrationSpec {
                        source: legacy_project_root.join("sessions"),
                        target: context.sessions_dir.clone(),
                        strategy: RuntimeMigrationStrategy::MoveIfTargetMissing,
                    },
                    RuntimeMigrationSpec {
                        source: legacy_project_root.join("memory"),
                        target: context.memory_dir.clone(),
                        strategy: RuntimeMigrationStrategy::MoveIfTargetMissing,
                    },
                    RuntimeMigrationSpec {
                        source: legacy_project_root.join("plans"),
                        target: context.plans_dir.clone(),
                        strategy: RuntimeMigrationStrategy::MoveIfTargetMissing,
                    },
                    RuntimeMigrationSpec {
                        source: legacy_project_root.join("snapshots"),
                        target: context.snapshots_dir.clone(),
                        strategy: RuntimeMigrationStrategy::MoveIfTargetMissing,
                    },
                ]
            }
            WorkspaceRuntimeTarget::RemoteWorkspaceMirror { ssh_host, remote_root } => {
                let runtime_root = self.remote_workspace_runtime_root(ssh_host, remote_root);
                let legacy_sessions_root = runtime_root.join("sessions").join(".northhing").join("sessions");
                vec![RuntimeMigrationSpec {
                    source: legacy_sessions_root,
                    target: context.sessions_dir.clone(),
                    strategy: RuntimeMigrationStrategy::MergeSessions,
                }]
            }
        }
    }

    pub(crate) async fn apply_migration_specs(
        &self,
        specs: &[RuntimeMigrationSpec],
    ) -> NortHingResult<Vec<RuntimeMigrationRecord>> {
        let mut migrated_entries = Vec::new();

        for spec in specs {
            let migrated = match spec.strategy {
                RuntimeMigrationStrategy::MoveIfTargetMissing => {
                    self.migrate_if_target_missing(&spec.source, &spec.target).await?
                }
                RuntimeMigrationStrategy::MergeSessions => self.merge_session_store(&spec.source, &spec.target).await?,
            };

            if let Some(record) = migrated {
                migrated_entries.push(record);
            }
        }

        Ok(migrated_entries)
    }

    pub(crate) async fn cleanup_legacy_artifacts_for_context(
        &self,
        context: &WorkspaceRuntimeContext,
    ) -> NortHingResult<()> {
        if let WorkspaceRuntimeTarget::RemoteWorkspaceMirror { ssh_host, remote_root } = &context.target {
            let runtime_root = self.remote_workspace_runtime_root(ssh_host, remote_root);
            self.remove_dir_if_empty(&runtime_root.join("sessions").join(".northhing"))
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn migrate_if_target_missing(
        &self,
        source: &Path,
        target: &Path,
    ) -> NortHingResult<Option<RuntimeMigrationRecord>> {
        if !source.exists() || target.exists() {
            return Ok(None);
        }

        self.move_legacy_path(source, target).await.map(Some)
    }

    pub(crate) async fn move_legacy_path(
        &self,
        source: &Path,
        target: &Path,
    ) -> NortHingResult<RuntimeMigrationRecord> {
        if let Some(parent) = target.parent() {
            self.path_manager.ensure_dir(parent).await?;
        }

        match tokio::fs::rename(source, target).await {
            Ok(()) => Ok(RuntimeMigrationRecord {
                source: source.to_path_buf(),
                target: target.to_path_buf(),
                strategy: "rename".to_string(),
            }),
            Err(_) if source.is_dir() => {
                copy_dir_recursive(source, target)?;
                std::fs::remove_dir_all(source).map_err(|e| {
                    NortHingError::service(format!("Failed to remove legacy directory {}: {}", source.display(), e))
                })?;
                Ok(RuntimeMigrationRecord {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                    strategy: "copy_dir".to_string(),
                })
            }
            Err(_) => {
                std::fs::copy(source, target).map_err(|e| {
                    NortHingError::service(format!(
                        "Failed to copy legacy file {} to {}: {}",
                        source.display(),
                        target.display(),
                        e
                    ))
                })?;
                std::fs::remove_file(source).map_err(|e| {
                    NortHingError::service(format!("Failed to remove legacy file {}: {}", source.display(), e))
                })?;
                Ok(RuntimeMigrationRecord {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                    strategy: "copy_file".to_string(),
                })
            }
        }
    }

    pub(crate) async fn merge_session_store(
        &self,
        source: &Path,
        target: &Path,
    ) -> NortHingResult<Option<RuntimeMigrationRecord>> {
        if !source.exists() {
            return Ok(None);
        }

        std::fs::create_dir_all(target).map_err(|e| {
            NortHingError::service(format!(
                "Failed to create target sessions directory {}: {}",
                target.display(),
                e
            ))
        })?;

        for entry in std::fs::read_dir(source).map_err(|e| {
            NortHingError::service(format!(
                "Failed to read legacy sessions directory {}: {}",
                source.display(),
                e
            ))
        })? {
            let entry = entry.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to inspect legacy sessions entry under {}: {}",
                    source.display(),
                    e
                ))
            })?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let file_type = entry.file_type().map_err(|e| {
                NortHingError::service(format!("Failed to read file type for {}: {}", source_path.display(), e))
            })?;

            if file_name.to_string_lossy().eq_ignore_ascii_case("index.json") {
                remove_path_if_exists(&source_path)?;
                continue;
            }

            if !file_type.is_dir() {
                let target_path = target.join(&file_name);
                if !target_path.exists() {
                    move_path_best_effort(&source_path, &target_path)?;
                } else if files_are_equal(&source_path, &target_path)? {
                    remove_path_if_exists(&source_path)?;
                } else {
                    replace_target_if_source_newer(&source_path, &target_path)?;
                }
                continue;
            }

            let target_path = target.join(&file_name);
            if !target_path.exists() {
                move_path_best_effort(&source_path, &target_path)?;
                continue;
            }

            merge_session_directory(&source_path, &target_path)?;
            remove_path_if_exists(&source_path)?;
        }

        rebuild_session_index(target).await?;
        remove_path_if_exists(&source.join("index.json"))?;
        remove_path_if_exists(source)?;

        Ok(Some(RuntimeMigrationRecord {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            strategy: "merge_sessions".to_string(),
        }))
    }

    pub(crate) async fn remove_dir_if_empty(&self, path: &Path) -> NortHingResult<()> {
        if !path.is_dir() {
            return Ok(());
        }

        let is_empty = match tokio::fs::read_dir(path).await {
            Ok(mut entries) => entries.next_entry().await.map(|entry| entry.is_none()).unwrap_or(false),
            Err(e) => {
                return Err(NortHingError::service(format!(
                    "Failed to inspect directory {}: {}",
                    path.display(),
                    e
                )));
            }
        };

        if is_empty {
            tokio::fs::remove_dir(path).await.map_err(|e| {
                NortHingError::service(format!(
                    "Failed to remove empty legacy directory {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }
}

fn merge_session_directory(source: &Path, target: &Path) -> NortHingResult<()> {
    std::fs::create_dir_all(target).map_err(|e| {
        NortHingError::service(format!(
            "Failed to create target session directory {}: {}",
            target.display(),
            e
        ))
    })?;

    for entry in std::fs::read_dir(source).map_err(|e| {
        NortHingError::service(format!(
            "Failed to read legacy session directory {}: {}",
            source.display(),
            e
        ))
    })? {
        let entry = entry.map_err(|e| {
            NortHingError::service(format!(
                "Failed to inspect legacy session entry under {}: {}",
                source.display(),
                e
            ))
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| {
            NortHingError::service(format!("Failed to read file type for {}: {}", source_path.display(), e))
        })?;

        if file_type.is_dir() {
            if !target_path.exists() {
                move_path_best_effort(&source_path, &target_path)?;
            } else {
                merge_session_directory(&source_path, &target_path)?;
                remove_path_if_exists(&source_path)?;
            }
            continue;
        }

        if file_name_eq(&source_path, "metadata.json") && target_path.exists() {
            merge_session_metadata_file(&source_path, &target_path)?;
            remove_path_if_exists(&source_path)?;
            continue;
        }

        if !target_path.exists() {
            move_path_best_effort(&source_path, &target_path)?;
        } else if files_are_equal(&source_path, &target_path)? {
            remove_path_if_exists(&source_path)?;
        } else {
            replace_target_if_source_newer(&source_path, &target_path)?;
        }
    }

    Ok(())
}

fn merge_session_metadata_file(source: &Path, target: &Path) -> NortHingResult<()> {
    let source_file = read_json_optional_sync::<StoredSessionMetadataFile>(source)?
        .ok_or_else(|| NortHingError::service(format!("Missing readable session metadata in {}", source.display())))?;
    let target_file = read_json_optional_sync::<StoredSessionMetadataFile>(target)?
        .ok_or_else(|| NortHingError::service(format!("Missing readable session metadata in {}", target.display())))?;

    let chosen = if source_file.metadata.last_active_at > target_file.metadata.last_active_at {
        source_file
    } else {
        target_file
    };

    write_json_pretty_sync(target, &chosen)?;
    Ok(())
}

async fn rebuild_session_index(sessions_dir: &Path) -> NortHingResult<()> {
    if !sessions_dir.exists() {
        return Ok(());
    }

    SessionMetadataStore::new(sessions_dir)
        .rebuild_index()
        .await
        .map(|_| ())
        .map_err(session_metadata_store_error)
}

fn session_metadata_store_error(error: SessionMetadataStoreError) -> NortHingError {
    if error.is_deserialization() {
        NortHingError::Deserialization(error.to_string())
    } else if error.is_serialization() {
        NortHingError::serialization(error.to_string())
    } else {
        NortHingError::service(error.to_string())
    }
}

fn replace_target_if_source_newer(source: &Path, target: &Path) -> NortHingResult<()> {
    if source_is_newer(source, target)? {
        remove_path_if_exists(target)?;
        move_path_best_effort(source, target)
    } else {
        remove_path_if_exists(source)
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> NortHingResult<()> {
    std::fs::create_dir_all(target).map_err(|e| {
        NortHingError::service(format!("Failed to create target directory {}: {}", target.display(), e))
    })?;

    for entry in std::fs::read_dir(source)
        .map_err(|e| NortHingError::service(format!("Failed to read legacy directory {}: {}", source.display(), e)))?
    {
        let entry = entry.map_err(|e| {
            NortHingError::service(format!(
                "Failed to inspect legacy directory entry under {}: {}",
                source.display(),
                e
            ))
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| {
            NortHingError::service(format!("Failed to read file type for {}: {}", source_path.display(), e))
        })?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &target_path).map_err(|e| {
                NortHingError::service(format!(
                    "Failed to copy legacy file {} to {}: {}",
                    source_path.display(),
                    target_path.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

fn read_json_optional_sync<T>(path: &Path) -> NortHingResult<Option<T>>
where
    T: DeserializeOwned,
{
    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(path)
        .map_err(|e| NortHingError::service(format!("Failed to read JSON file {}: {}", path.display(), e)))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|e| NortHingError::service(format!("Failed to deserialize JSON file {}: {}", path.display(), e)))?;
    Ok(Some(value))
}

fn write_json_pretty_sync<T>(path: &Path, value: &T) -> NortHingResult<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            NortHingError::service(format!("Failed to create parent directory {}: {}", parent.display(), e))
        })?;
    }

    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| NortHingError::service(format!("Failed to serialize JSON for {}: {}", path.display(), e)))?;
    std::fs::write(path, bytes)
        .map_err(|e| NortHingError::service(format!("Failed to write JSON file {}: {}", path.display(), e)))
}

fn move_path_best_effort(source: &Path, target: &Path) -> NortHingResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            NortHingError::service(format!(
                "Failed to create target parent directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    match std::fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) if source.is_dir() => {
            copy_dir_recursive(source, target)?;
            std::fs::remove_dir_all(source).map_err(|e| {
                NortHingError::service(format!("Failed to remove moved directory {}: {}", source.display(), e))
            })
        }
        Err(_) => {
            std::fs::copy(source, target).map_err(|e| {
                NortHingError::service(format!(
                    "Failed to copy file {} to {}: {}",
                    source.display(),
                    target.display(),
                    e
                ))
            })?;
            std::fs::remove_file(source)
                .map_err(|e| NortHingError::service(format!("Failed to remove moved file {}: {}", source.display(), e)))
        }
    }
}

fn remove_path_if_exists(path: &Path) -> NortHingResult<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| NortHingError::service(format!("Failed to remove directory {}: {}", path.display(), e)))
    } else {
        std::fs::remove_file(path)
            .map_err(|e| NortHingError::service(format!("Failed to remove file {}: {}", path.display(), e)))
    }
}

fn files_are_equal(left: &Path, right: &Path) -> NortHingResult<bool> {
    let left_bytes = std::fs::read(left)
        .map_err(|e| NortHingError::service(format!("Failed to read file {}: {}", left.display(), e)))?;
    let right_bytes = std::fs::read(right)
        .map_err(|e| NortHingError::service(format!("Failed to read file {}: {}", right.display(), e)))?;
    Ok(left_bytes == right_bytes)
}

fn source_is_newer(source: &Path, target: &Path) -> NortHingResult<bool> {
    let source_modified = std::fs::metadata(source)
        .map_err(|e| NortHingError::service(format!("Failed to stat source file {}: {}", source.display(), e)))?
        .modified()
        .ok();
    let target_modified = std::fs::metadata(target)
        .map_err(|e| NortHingError::service(format!("Failed to stat target file {}: {}", target.display(), e)))?
        .modified()
        .ok();

    Ok(match (source_modified, target_modified) {
        (Some(source_time), Some(target_time)) => source_time > target_time,
        (Some(_), None) => true,
        _ => false,
    })
}

fn file_name_eq(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}
