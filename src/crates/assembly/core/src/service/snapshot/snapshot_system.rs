use crate::service::snapshot::baseline_cache::BaselineCache;
use crate::service::snapshot::types::{
    FileMetadata, FileSnapshot, OptimizedContent, SnapshotError, SnapshotResult, SnapshotType, StorageStats,
};
use crate::service::workspace_runtime::WorkspaceRuntimeContext;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Simplified file snapshot system
///
/// Only stores snapshots of file content; does not manage a change queue.
pub struct FileSnapshotSystem {
    pub(super) snapshot_dir: PathBuf,
    pub(super) snapshot_by_hash_dir: PathBuf,
    pub(super) snapshot_metadata_dir: PathBuf,
    pub(super) hash_to_path: HashMap<String, PathBuf>,
    pub(super) active_snapshots: HashMap<String, FileSnapshot>,
    pub(super) compression_enabled: bool,
    pub(super) dedup_enabled: bool,
    pub(super) baseline_cache: BaselineCache,
}

impl FileSnapshotSystem {
    /// Creates a new file snapshot system.
    pub fn new(runtime_context: WorkspaceRuntimeContext) -> Self {
        let snapshot_dir = runtime_context.snapshots_dir.clone();

        Self {
            snapshot_by_hash_dir: runtime_context.snapshot_by_hash_dir.clone(),
            snapshot_metadata_dir: runtime_context.snapshot_metadata_dir.clone(),
            snapshot_dir,
            hash_to_path: HashMap::new(),
            active_snapshots: HashMap::new(),
            compression_enabled: true,
            dedup_enabled: true,
            baseline_cache: BaselineCache::new(runtime_context.snapshot_baselines_dir.clone()),
        }
    }

    /// Initializes the snapshot system.
    pub async fn initialize(&mut self) -> SnapshotResult<()> {
        let total_started_at = Instant::now();
        info!("Initializing file snapshot system");

        let directories_started_at = Instant::now();
        self.ensure_directories().await?;
        debug!(
            "File snapshot initialize step completed: step=ensure_directories duration_ms={}",
            directories_started_at.elapsed().as_millis()
        );

        let index_started_at = Instant::now();
        self.load_snapshot_index().await?;
        debug!(
            "File snapshot initialize step completed: step=load_snapshot_index duration_ms={}",
            index_started_at.elapsed().as_millis()
        );

        info!(
            "File snapshot system initialized: loaded_snapshots={} duration_ms={}",
            self.active_snapshots.len(),
            total_started_at.elapsed().as_millis()
        );
        Ok(())
    }

    /// Ensures required directories exist.
    async fn ensure_directories(&self) -> SnapshotResult<()> {
        let directories = [
            &self.snapshot_dir,
            &self.snapshot_by_hash_dir,
            &self.snapshot_metadata_dir,
            &self.baseline_cache.baseline_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                return Err(SnapshotError::ConfigError(format!(
                    "Snapshot runtime directory is missing: {}",
                    dir.display()
                )));
            }
        }

        Ok(())
    }

    /// Loads the existing snapshot index.
    async fn load_snapshot_index(&mut self) -> SnapshotResult<()> {
        let started_at = Instant::now();
        let metadata_dir = self.snapshot_metadata_dir.clone();

        if !metadata_dir.exists() {
            return Ok(());
        }

        let mut loaded_count = 0;

        for entry in fs::read_dir(&metadata_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_snapshot_metadata(&path).await {
                    Ok(snapshot) => {
                        self.hash_to_path.insert(
                            snapshot.content_hash.clone(),
                            self.get_content_path(&snapshot.content_hash),
                        );
                        self.active_snapshots.insert(snapshot.snapshot_id.clone(), snapshot);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load snapshot metadata: path={} error={}", path.display(), e);
                    }
                }
            }
        }

        debug!(
            "Loaded snapshot metadata files: count={} duration_ms={}",
            loaded_count,
            started_at.elapsed().as_millis()
        );
        Ok(())
    }

    /// Loads snapshot metadata.
    async fn load_snapshot_metadata(&self, path: &Path) -> SnapshotResult<FileSnapshot> {
        let content = fs::read_to_string(path)?;
        let snapshot: FileSnapshot = serde_json::from_str(&content)?;
        Ok(snapshot)
    }

    /// Creates a file snapshot.
    pub async fn create_snapshot(&mut self, file_path: &Path) -> SnapshotResult<String> {
        debug!("Creating snapshot: file_path={}", file_path.display());

        if !file_path.exists() {
            error!("File not found for snapshot: file_path={}", file_path.display());
            return Err(SnapshotError::FileNotFound(file_path.to_path_buf()));
        }

        let content = match fs::read(file_path) {
            Ok(data) => data,
            Err(e) => {
                error!(
                    "Failed to read file for snapshot: file_path={} error={}",
                    file_path.display(),
                    e
                );
                return Err(SnapshotError::Io(e));
            }
        };

        let metadata = self.extract_file_metadata(file_path).await?;

        let content_hash = self.calculate_content_hash(&content);

        if self.dedup_enabled && self.hash_to_path.contains_key(&content_hash) {
            if let Some(snapshot_id) = self.find_snapshot_by_hash(&content_hash) {
                debug!(
                    "Found duplicate content, reusing existing snapshot: content_hash={}",
                    content_hash
                );
                return Ok(snapshot_id);
            }

            debug!(
                "Found reusable content without active snapshot metadata, creating new snapshot metadata: content_hash={}",
                content_hash
            );
        }

        let optimized_content = self.optimize_content(&content);

        let snapshot = FileSnapshot {
            snapshot_id: Uuid::new_v4().to_string(),
            file_path: file_path.to_path_buf(),
            content_hash: content_hash.clone(),
            snapshot_type: SnapshotType::Before,
            compressed_content: match optimized_content {
                OptimizedContent::Raw(data) => data,
                OptimizedContent::Compressed(data) => data,
                OptimizedContent::Reference(_) => Vec::new(),
            },
            timestamp: std::time::SystemTime::now(),
            metadata,
        };

        self.store_snapshot(&snapshot).await?;

        self.hash_to_path
            .insert(content_hash, self.get_content_path(&snapshot.content_hash));
        let snapshot_id = snapshot.snapshot_id.clone();
        self.active_snapshots.insert(snapshot_id.clone(), snapshot);

        debug!("Snapshot created successfully: snapshot_id={}", snapshot_id);
        Ok(snapshot_id)
    }

    /// Extracts file metadata.
    async fn extract_file_metadata(&self, file_path: &Path) -> SnapshotResult<FileMetadata> {
        let metadata = fs::metadata(file_path)?;

        let permissions = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                Some(metadata.permissions().mode())
            }
            #[cfg(not(unix))]
            {
                None
            }
        };

        let encoding = self
            .detect_file_encoding(file_path)
            .await
            .unwrap_or_else(|| "utf-8".to_string());

        Ok(FileMetadata {
            size: metadata.len(),
            permissions,
            last_modified: metadata.modified()?,
            encoding,
        })
    }

    /// Detects file encoding.
    async fn detect_file_encoding(&self, file_path: &Path) -> Option<String> {
        match fs::read(file_path) {
            Ok(bytes) => {
                if bytes.is_ascii() {
                    Some("ascii".to_string())
                } else if String::from_utf8(bytes).is_ok() {
                    Some("utf-8".to_string())
                } else {
                    Some("binary".to_string())
                }
            }
            Err(_) => None,
        }
    }

    /// Stores a snapshot.
    async fn store_snapshot(&self, snapshot: &FileSnapshot) -> SnapshotResult<()> {
        let content_path = self.get_content_path(&snapshot.content_hash);
        if !content_path.exists() {
            fs::write(&content_path, &snapshot.compressed_content)?;
        }

        let metadata_path = self.get_metadata_path(&snapshot.snapshot_id);
        let metadata_json = serde_json::to_string_pretty(snapshot)?;
        fs::write(&metadata_path, metadata_json)?;

        debug!(
            "Snapshot stored: snapshot_id={} content_path={}",
            snapshot.snapshot_id,
            content_path.display()
        );
        Ok(())
    }

    /// Loads snapshot metadata from disk (without using in-memory cache).
    async fn load_snapshot_from_disk(&self, snapshot_id: &str) -> SnapshotResult<FileSnapshot> {
        debug!("Loading snapshot metadata from disk: snapshot_id={}", snapshot_id);
        let metadata_path = self.get_metadata_path(snapshot_id);

        if !metadata_path.exists() {
            return Err(SnapshotError::SnapshotNotFound(snapshot_id.to_string()));
        }

        let snapshot = self.load_snapshot_metadata(&metadata_path).await?;
        debug!("Snapshot metadata loaded successfully: snapshot_id={}", snapshot_id);
        Ok(snapshot)
    }

    /// Recorded logical size (bytes) from snapshot metadata, without loading file contents.
    pub async fn get_snapshot_recorded_size_bytes(&self, snapshot_id: &str) -> SnapshotResult<u64> {
        let snapshot = self.load_snapshot_from_disk(snapshot_id).await?;
        Ok(snapshot.metadata.size)
    }

    /// Gets snapshot content (string), read directly from disk.
    pub async fn get_snapshot_content(&self, snapshot_id: &str) -> SnapshotResult<String> {
        let content_bytes = self.restore_snapshot_content(snapshot_id).await?;
        String::from_utf8(content_bytes)
            .map_err(|e| SnapshotError::ConfigError(format!("Snapshot content is not valid UTF-8: {}", e)))
    }

    /// Restores snapshot content (read directly from disk, without using in-memory cache).
    pub async fn restore_snapshot_content(&self, snapshot_id: &str) -> SnapshotResult<Vec<u8>> {
        let snapshot = self.load_snapshot_from_disk(snapshot_id).await?;

        if !snapshot.compressed_content.is_empty() {
            return self.extract_content_from_snapshot(&snapshot);
        }

        let content_path = self.get_content_path(&snapshot.content_hash);
        if !content_path.exists() {
            return Err(SnapshotError::SnapshotNotFound(format!(
                "content file not found: {}",
                content_path.display()
            )));
        }

        let compressed_content = fs::read(&content_path)?;

        match self.decompress_content(&compressed_content) {
            Ok(decompressed) => Ok(decompressed),
            Err(_) => Ok(compressed_content),
        }
    }

    /// Restores a file to the specified path (reads snapshot directly from disk).
    pub async fn restore_file(&self, snapshot_id: &str, target_path: &Path) -> SnapshotResult<()> {
        info!(
            "Restoring file from snapshot: snapshot_id={} target_path={}",
            snapshot_id,
            target_path.display()
        );

        let snapshot = self.load_snapshot_from_disk(snapshot_id).await?;
        let metadata = snapshot.metadata.clone();

        let content = self.restore_snapshot_content(snapshot_id).await?;

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(target_path, content)?;

        self.restore_file_metadata(target_path, &metadata).await?;

        info!("File restored successfully: target_path={}", target_path.display());
        Ok(())
    }

    /// Restores file metadata.
    async fn restore_file_metadata(&self, file_path: &Path, metadata: &FileMetadata) -> SnapshotResult<()> {
        #[cfg(unix)]
        {
            if let Some(permissions) = metadata.permissions {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(permissions);
                fs::set_permissions(file_path, perms)?;
            }
        }

        let filetime = filetime::FileTime::from_system_time(metadata.last_modified);
        if let Err(e) = filetime::set_file_mtime(file_path, filetime) {
            warn!("Failed to restore file modification time: error={}", e);
        }

        Ok(())
    }

    /// Deletes a snapshot.
    pub async fn delete_snapshot(&mut self, snapshot_id: &str) -> SnapshotResult<()> {
        info!("Deleting snapshot: snapshot_id={}", snapshot_id);

        let snapshot = self
            .active_snapshots
            .remove(snapshot_id)
            .ok_or_else(|| SnapshotError::SnapshotNotFound(snapshot_id.to_string()))?;

        let content_still_used = self
            .active_snapshots
            .values()
            .any(|s| s.content_hash == snapshot.content_hash);

        if !content_still_used {
            let content_path = self.get_content_path(&snapshot.content_hash);
            if content_path.exists() {
                fs::remove_file(&content_path)?;
            }

            self.hash_to_path.remove(&snapshot.content_hash);
        }

        let metadata_path = self.get_metadata_path(snapshot_id);
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)?;
        }

        debug!("Snapshot deleted successfully: snapshot_id={}", snapshot_id);
        Ok(())
    }

    /// Returns storage statistics.
    pub async fn get_storage_stats(&self) -> SnapshotResult<StorageStats> {
        let mut total_size_bytes = 0;
        let mut compressed_size_bytes = 0;

        let total_snapshots = self.active_snapshots.len();

        let content_dir = self.snapshot_by_hash_dir.clone();
        if content_dir.exists() {
            for entry in fs::read_dir(&content_dir)? {
                let entry = entry?;
                if let Ok(metadata) = entry.metadata() {
                    compressed_size_bytes += metadata.len();
                }
            }
        }

        for snapshot in self.active_snapshots.values() {
            total_size_bytes += snapshot.metadata.size;
        }

        let compression_ratio = if total_size_bytes > 0 {
            compressed_size_bytes as f32 / total_size_bytes as f32
        } else {
            1.0
        };

        let dedup_savings_bytes = if self.dedup_enabled {
            let unique_hashes = self.hash_to_path.len() as u64;
            let total_hashes = total_snapshots as u64;
            if total_hashes > unique_hashes {
                (total_size_bytes * (total_hashes - unique_hashes)) / total_hashes
            } else {
                0
            }
        } else {
            0
        };

        Ok(StorageStats {
            total_snapshots,
            total_size_bytes,
            compressed_size_bytes,
            compression_ratio,
            dedup_savings_bytes,
        })
    }

    /// Cleans up orphaned snapshots.
    pub async fn cleanup_orphaned_snapshots(&mut self) -> SnapshotResult<usize> {
        info!("Cleaning up orphaned snapshots");

        let mut cleaned_count = 0;
        let content_dir = self.snapshot_by_hash_dir.clone();

        if !content_dir.exists() {
            return Ok(0);
        }

        let mut content_files = Vec::new();
        for entry in fs::read_dir(&content_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("snap") {
                content_files.push(entry.path());
            }
        }

        for content_file in content_files {
            if let Some(file_stem) = content_file.file_stem().and_then(|s| s.to_str()) {
                let content_hash = file_stem;

                let is_referenced = self
                    .active_snapshots
                    .values()
                    .any(|snapshot| snapshot.content_hash == content_hash);

                if !is_referenced {
                    fs::remove_file(&content_file)?;
                    self.hash_to_path.remove(content_hash);
                    cleaned_count += 1;
                    debug!("Deleted orphaned content file: path={}", content_file.display());
                }
            }
        }

        info!("Cleaned up {} orphaned snapshots", cleaned_count);
        Ok(cleaned_count)
    }

    /// Lists all snapshots.
    pub fn list_snapshots(&self) -> Vec<&FileSnapshot> {
        self.active_snapshots.values().collect()
    }

    /// Returns the snapshot count.
    pub fn snapshot_count(&self) -> usize {
        self.active_snapshots.len()
    }

    /// Gets the baseline snapshot ID for a file.
    ///
    /// Returns: Option<String> - `None` means this file has no baseline
    pub async fn get_baseline_snapshot_id(&self, file_path: &Path) -> Option<String> {
        self.baseline_cache.get(file_path).await
    }

    /// Creates a baseline snapshot.
    ///
    /// Creates a baseline from the specified "before" snapshot.
    /// If a baseline already exists, it will not be created again.
    ///
    /// # Parameters
    /// - file_path: File path
    /// - before_snapshot_id: before snapshot ID
    ///
    /// # Returns
    /// Baseline snapshot ID
    pub async fn create_baseline_from_snapshot(
        &self,
        file_path: &Path,
        before_snapshot_id: &str,
    ) -> SnapshotResult<String> {
        debug!(
            "Creating baseline snapshot: file_path={:?} before_snapshot_id={}",
            file_path, before_snapshot_id
        );

        if let Some(existing_id) = self.get_baseline_snapshot_id(file_path).await {
            debug!("Baseline snapshot already exists: baseline_id={}", existing_id);
            return Ok(existing_id);
        }

        self.baseline_cache
            .create_from_snapshot(file_path, before_snapshot_id, &self.active_snapshots)
            .await
    }

    /// Creates an empty baseline for files that did not exist before the session.
    pub async fn create_empty_baseline(&mut self, file_path: &Path) -> SnapshotResult<String> {
        let empty_content_hash = self.calculate_content_hash(&[]);
        let content_path = self.get_content_path(&empty_content_hash);

        if !self.hash_to_path.contains_key(&empty_content_hash) {
            self.hash_to_path
                .insert(empty_content_hash.clone(), content_path.clone());
        }

        self.baseline_cache
            .create_empty(file_path, &empty_content_hash, &content_path)
            .await
    }

    /// Checks whether the file has a baseline.
    pub async fn has_baseline(&self, file_path: &Path) -> bool {
        self.get_baseline_snapshot_id(file_path).await.is_some()
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::workspace_runtime::{WorkspaceRuntimeContext, WorkspaceRuntimeTarget};

    fn test_runtime_context() -> WorkspaceRuntimeContext {
        let runtime_root = std::env::temp_dir().join(format!("northhing_snapshot_test_{}", Uuid::new_v4()));
        WorkspaceRuntimeContext::new(
            WorkspaceRuntimeTarget::LocalWorkspace {
                workspace_root: runtime_root.join("workspace"),
            },
            runtime_root,
        )
    }

    fn create_runtime_dirs(context: &WorkspaceRuntimeContext) {
        for directory in context.required_directories() {
            fs::create_dir_all(directory).expect("create runtime directory");
        }
    }

    #[tokio::test]
    async fn create_snapshot_reuses_empty_baseline_content_without_panicking() {
        let context = test_runtime_context();
        create_runtime_dirs(&context);

        let file_path = context.runtime_root.join("workspace").join("empty.txt");
        fs::create_dir_all(file_path.parent().expect("file has parent")).expect("create parent");

        let mut snapshot_system = FileSnapshotSystem::new(context.clone());
        snapshot_system.initialize().await.expect("initialize snapshots");
        snapshot_system
            .create_empty_baseline(&file_path)
            .await
            .expect("create empty baseline");

        fs::write(&file_path, []).expect("write empty file");

        let snapshot_id = snapshot_system
            .create_snapshot(&file_path)
            .await
            .expect("create snapshot");
        let restored = snapshot_system
            .restore_snapshot_content(&snapshot_id)
            .await
            .expect("restore snapshot content");

        assert!(restored.is_empty());

        fs::remove_dir_all(&context.runtime_root).expect("cleanup runtime root");
    }
}
