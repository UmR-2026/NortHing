use crate::service::snapshot::types::{FileSnapshot, OptimizedContent, SnapshotError, SnapshotResult};
use std::path::PathBuf;
use tracing::warn;

use crate::service::snapshot::snapshot_system::FileSnapshotSystem;

impl FileSnapshotSystem {
    /// Computes content hash.
    pub(super) fn calculate_content_hash(&self, content: &[u8]) -> String {
        format!("{:x}", md5::compute(content))
    }

    /// Optimizes content storage.
    pub(super) fn optimize_content(&self, content: &[u8]) -> OptimizedContent {
        if self.dedup_enabled {
            let hash = self.calculate_content_hash(content);
            let content_path = self.get_content_path(&hash);
            if self.hash_to_path.contains_key(&hash) && content_path.exists() {
                return OptimizedContent::Reference(hash);
            }
        }

        if self.compression_enabled && content.len() > 1024 {
            match self.compress_content(content) {
                Ok(compressed) => {
                    if compressed.len() < content.len() * 4 / 5 {
                        return OptimizedContent::Compressed(compressed);
                    }
                }
                Err(e) => {
                    warn!("Content compression failed: error={}", e);
                }
            }
        }

        OptimizedContent::Raw(content.to_vec())
    }

    /// Compresses content.
    pub(super) fn compress_content(&self, content: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        encoder.finish()
    }

    /// Decompresses content.
    pub(super) fn decompress_content(&self, compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// Returns the content file path.
    pub(super) fn get_content_path(&self, content_hash: &str) -> PathBuf {
        self.snapshot_by_hash_dir.join(format!("{}.snap", content_hash))
    }

    /// Returns the metadata file path.
    pub(super) fn get_metadata_path(&self, snapshot_id: &str) -> PathBuf {
        if snapshot_id.starts_with("baseline_") {
            self.baseline_cache.baseline_dir.join(format!("{}.json", snapshot_id))
        } else {
            self.snapshot_metadata_dir.join(format!("{}.json", snapshot_id))
        }
    }

    /// Finds a snapshot ID by hash.
    pub(super) fn find_snapshot_by_hash(&self, content_hash: &str) -> Option<String> {
        for (snapshot_id, snapshot) in &self.active_snapshots {
            if snapshot.content_hash == content_hash {
                return Some(snapshot_id.clone());
            }
        }
        None
    }

    /// Extracts content from a snapshot.
    pub(super) fn extract_content_from_snapshot(&self, snapshot: &FileSnapshot) -> SnapshotResult<Vec<u8>> {
        if snapshot.compressed_content.is_empty() {
            return Err(SnapshotError::SnapshotNotFound("snapshot content is empty".to_string()));
        }

        match self.decompress_content(&snapshot.compressed_content) {
            Ok(decompressed) => Ok(decompressed),
            Err(_) => Ok(snapshot.compressed_content.clone()),
        }
    }
}
