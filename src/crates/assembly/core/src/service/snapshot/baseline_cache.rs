use crate::service::snapshot::types::{FileMetadata, FileSnapshot, SnapshotError, SnapshotResult, SnapshotType};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

/// Baseline snapshot cache
pub struct BaselineCache {
    /// In-memory cache: file_path -> snapshot_id
    /// `None` indicates it has been queried but does not exist.
    pub(super) cache: Arc<RwLock<HashMap<PathBuf, Option<String>>>>,

    /// Baseline metadata directory
    pub(super) baseline_dir: PathBuf,
}

impl BaselineCache {
    /// Creates a new baseline cache.
    pub fn new(baseline_dir: PathBuf) -> Self {
        debug!("BaselineCache initialized: directory={}", baseline_dir.display());

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            baseline_dir,
        }
    }

    /// Gets the baseline snapshot ID for a file.
    ///
    /// Strategy: check the in-memory map first; if missing, check the directory, then cache the result.
    pub async fn get(&self, file_path: &Path) -> Option<String> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(file_path) {
                debug!("Baseline cache hit: file_path={:?}", file_path);
                return cached.clone();
            }
        }

        debug!("Querying baseline directory: file_path={:?}", file_path);
        if let Some(snapshot_id) = self.query_directory(file_path).await {
            debug!(
                "Found baseline snapshot: file_path={:?} snapshot_id={}",
                file_path, snapshot_id
            );

            {
                let mut cache = self.cache.write().await;
                cache.insert(file_path.to_path_buf(), Some(snapshot_id.clone()));
            }

            return Some(snapshot_id);
        }

        debug!("Baseline snapshot not found: file_path={:?}", file_path);
        {
            let mut cache = self.cache.write().await;
            cache.insert(file_path.to_path_buf(), None);
        }
        None
    }

    /// Queries baseline snapshots from the directory.
    async fn query_directory(&self, file_path: &Path) -> Option<String> {
        let entries = fs::read_dir(&self.baseline_dir).ok()?;

        let mut found_snapshots: Vec<(SystemTime, String)> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let content = fs::read_to_string(&path).ok()?;
            let metadata: FileSnapshot = serde_json::from_str(&content).ok()?;

            if metadata.snapshot_type == SnapshotType::Baseline && metadata.file_path == file_path {
                found_snapshots.push((metadata.timestamp, metadata.snapshot_id));
            }
        }

        if found_snapshots.is_empty() {
            return None;
        }

        found_snapshots.sort_by_key(|b| std::cmp::Reverse(b.0));
        let (timestamp, snapshot_id) = &found_snapshots[0];

        debug!(
            "Found {} baseline snapshots, using latest: snapshot_id={} timestamp={:?}",
            found_snapshots.len(),
            snapshot_id,
            timestamp
        );

        Some(snapshot_id.clone())
    }

    /// Creates a baseline from a "before" snapshot.
    pub async fn create_from_snapshot(
        &self,
        file_path: &Path,
        before_snapshot_id: &str,
        active_snapshots: &HashMap<String, FileSnapshot>,
    ) -> SnapshotResult<String> {
        debug!(
            "Creating baseline snapshot: file_path={:?} before_snapshot_id={}",
            file_path, before_snapshot_id
        );

        let before_snapshot = active_snapshots
            .get(before_snapshot_id)
            .ok_or_else(|| SnapshotError::SnapshotNotFound(before_snapshot_id.to_string()))?;

        let baseline_id = format!("baseline_{}", Uuid::new_v4());

        let baseline_metadata = FileSnapshot {
            snapshot_id: baseline_id.clone(),
            file_path: file_path.to_path_buf(),
            content_hash: before_snapshot.content_hash.clone(),
            snapshot_type: SnapshotType::Baseline,
            compressed_content: before_snapshot.compressed_content.clone(),
            timestamp: SystemTime::now(),
            metadata: before_snapshot.metadata.clone(),
        };

        let baseline_meta_path = self.baseline_dir.join(format!("{}.json", baseline_id));
        let metadata_json = serde_json::to_string_pretty(&baseline_metadata)?;
        fs::write(&baseline_meta_path, metadata_json)?;

        debug!(
            "Created baseline snapshot: file_path={:?} baseline_id={} metadata_path={}",
            file_path,
            baseline_id,
            baseline_meta_path.display()
        );

        {
            let mut cache = self.cache.write().await;
            cache.insert(file_path.to_path_buf(), Some(baseline_id.clone()));
        }

        Ok(baseline_id)
    }

    /// Creates an empty baseline for files that are first introduced during the session.
    pub async fn create_empty(
        &self,
        file_path: &Path,
        empty_content_hash: &str,
        content_path: &Path,
    ) -> SnapshotResult<String> {
        let baseline_id = format!("baseline_empty_{}", Uuid::new_v4());

        if !content_path.exists() {
            fs::write(content_path, [])?;
        }

        let baseline_metadata = FileSnapshot {
            snapshot_id: baseline_id.clone(),
            file_path: file_path.to_path_buf(),
            content_hash: empty_content_hash.to_string(),
            snapshot_type: SnapshotType::Baseline,
            compressed_content: Vec::new(),
            timestamp: SystemTime::now(),
            metadata: FileMetadata {
                size: 0,
                permissions: None,
                last_modified: SystemTime::now(),
                encoding: "utf-8".to_string(),
            },
        };

        let baseline_meta_path = self.baseline_dir.join(format!("{}.json", baseline_id));
        let metadata_json = serde_json::to_string_pretty(&baseline_metadata)?;
        fs::write(&baseline_meta_path, metadata_json)?;

        {
            let mut cache = self.cache.write().await;
            cache.insert(file_path.to_path_buf(), Some(baseline_id.clone()));
        }

        Ok(baseline_id)
    }
}
