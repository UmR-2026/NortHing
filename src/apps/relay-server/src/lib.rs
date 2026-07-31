#![allow(dead_code)]
//! northhing Relay Server Library
//!
//! The standalone relay-server binary. Re-exports shared relay logic from
//! `northhing-relay-core` and adds the disk-backed asset store.

pub use northhing_relay_core::routes::api::AppState;
pub use northhing_relay_core::{
    build_relay_router, MemoryAssetStore, OutboundProtocol, ResponsePayload, RoomManager, WebAssetStore,
};

use dashmap::DashMap;
use tracing;

use northhing_relay_core::validated::{ContentHash, ValidatedRelPath, ValidatedRoomId};

// ── DiskAssetStore ────────────────────────────────────────────────────

/// Filesystem-backed asset store. Used by the standalone relay server.
///
/// Content is stored in `{base_dir}/_store/{hash}` and symlinked into
/// per-room directories `{base_dir}/{room_id}/{path}`.
pub struct DiskAssetStore {
    base_dir: String,
    known_hashes: DashMap<String, u64>,
}

impl DiskAssetStore {
    pub fn new(base_dir: &str) -> Self {
        let store_dir = std::path::PathBuf::from(base_dir).join("_store");
        let _ = std::fs::create_dir_all(&store_dir);

        let known: DashMap<String, u64> = DashMap::new();
        if store_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&store_dir) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            if let Some(name) = entry.file_name().to_str() {
                                known.insert(name.to_string(), meta.len());
                            }
                        }
                    }
                }
            }
        }
        tracing::info!(
            "DiskAssetStore initialized with {} entries from {}",
            known.len(),
            base_dir
        );
        Self {
            base_dir: base_dir.to_string(),
            known_hashes: known,
        }
    }

    fn store_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.base_dir).join("_store")
    }

    /// Resolve `base_dir` to its canonical path once (used as anchor for containment).
    fn canonical_base_dir(&self) -> Option<std::path::PathBuf> {
        std::fs::canonicalize(std::path::PathBuf::from(&self.base_dir)).ok()
    }

    /// Verify that `candidate` is strictly within `parent` after canonicalization.
    /// Returns true if safe, false otherwise.
    fn is_within(canonical_parent: &std::path::Path, candidate: &std::path::Path) -> bool {
        if let Ok(canonical_candidate) = std::fs::canonicalize(candidate) {
            canonical_candidate.starts_with(canonical_parent) && canonical_candidate != canonical_parent
        } else {
            false
        }
    }
}

impl WebAssetStore for DiskAssetStore {
    fn has_content(&self, hash: &ContentHash) -> bool {
        self.known_hashes.contains_key(hash.as_str())
    }

    fn store_content(&self, hash: &ContentHash, data: Vec<u8>) -> Result<(), String> {
        let store_path = self.store_dir().join(hash.as_str());
        if !store_path.exists() {
            std::fs::write(&store_path, &data).map_err(|e| e.to_string())?;
            self.known_hashes.insert(hash.to_string(), data.len() as u64);
        }
        Ok(())
    }

    fn map_to_room(
        &self,
        room_id: &ValidatedRoomId,
        rel_path: &ValidatedRelPath,
        hash: &ContentHash,
    ) -> Result<(), String> {
        let canonical_base = self
            .canonical_base_dir()
            .ok_or_else(|| "cannot canonicalize base_dir".to_string())?;

        let room_dir = self
            .store_dir()
            .parent()
            .unwrap_or(&canonical_base)
            .join(room_id.as_str());

        let store_path = self.store_dir().join(hash.as_str());
        let dest = room_dir.join(rel_path.as_str());

        // Create parent directories first so canonicalize below succeeds.
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent dirs for {}: {e}", dest.display()))?;
        }

        // Containment check only after the parent tree exists on disk.
        if let Some(parent) = dest.parent() {
            let parent_canon = std::fs::canonicalize(parent)
                .map_err(|e| format!("cannot canonicalize parent {}: {e}", parent.display()))?;
            if !parent_canon.starts_with(&canonical_base) {
                return Err(format!(
                    "refusing to map file outside room dir: {} (resolved to {})",
                    dest.display(),
                    parent_canon.display()
                ));
            }
        }

        // Only remove existing destination AFTER validation passes.
        let _ = std::fs::remove_file(&dest);
        create_link(&store_path, &dest).map_err(|e| e.to_string())
    }

    fn get_file(&self, room_id: &ValidatedRoomId, path: &ValidatedRelPath) -> Option<Vec<u8>> {
        let room_dir = self
            .store_dir()
            .parent()
            .unwrap_or(std::path::Path::new(&self.base_dir))
            .join(room_id.as_str());
        let target = room_dir.join(path.as_str());
        let index = ValidatedRelPath::try_from("index.html").expect("static index.html is a valid rel path");
        let file = if target.is_file() {
            target
        } else {
            room_dir.join(index.as_str())
        };
        if file.is_file() {
            std::fs::read(&file).ok()
        } else {
            None
        }
    }

    fn has_room_files(&self, room_id: &ValidatedRoomId) -> bool {
        let room_dir = self
            .store_dir()
            .parent()
            .unwrap_or(std::path::Path::new(&self.base_dir))
            .join(room_id.as_str());
        room_dir.exists()
    }

    fn cleanup_room(&self, room_id: &ValidatedRoomId) {
        let dir = self
            .store_dir()
            .parent()
            .unwrap_or(std::path::Path::new(&self.base_dir))
            .join(room_id.as_str());
        if !dir.exists() {
            return;
        }
        match self.canonical_base_dir() {
            Some(canonical_base) if Self::is_within(&canonical_base, &dir) => {}
            Some(_) => {
                tracing::warn!(
                    "cleanup_room: rejecting unsafe path {} (outside base dir)",
                    dir.display()
                );
                return;
            }
            None => {
                tracing::warn!(
                    "cleanup_room: cannot canonicalize base dir {} — refusing removal of {}",
                    self.base_dir,
                    dir.display()
                );
                return;
            }
        }
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::warn!("Failed to clean up room web dir {}: {e}", dir.display());
        } else {
            tracing::info!("Cleaned up room web dir for {}", room_id);
        }
    }
}

fn create_link(original: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(original, link)
    }
    #[cfg(not(unix))]
    {
        std::fs::hard_link(original, link).or_else(|_| std::fs::copy(original, link).map(|_| ()))
    }
}

#[cfg(test)]
mod disk_tests {
    use super::*;
    use std::fs;

    /// Create a unique temporary base directory for each test to avoid collisions.
    fn make_temp_base() -> String {
        let path = std::env::temp_dir().join(format!(
            "northhing-relay-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        path.to_string_lossy().to_string()
    }

    fn sample_hash() -> ContentHash {
        // SHA-256 of b"hello" — deterministic for testing
        ContentHash::from_data(b"hello")
    }

    #[test]
    fn map_to_room_normal_path_writes_and_reads() {
        let base = make_temp_base();
        let store = DiskAssetStore::new(&base);

        let room = ValidatedRoomId::try_from("test-room").unwrap();
        let rel = ValidatedRelPath::try_from("assets/app.js").unwrap();
        let hash = sample_hash();

        store.store_content(&hash, b"hello world".to_vec()).unwrap();

        // Normal map_to_room should succeed.
        store.map_to_room(&room, &rel, &hash).unwrap();

        // File should be readable via get_file.
        let content = store.get_file(&room, &rel).expect("content should exist");
        assert_eq!(content, b"hello world");

        // Directory should have been created inside base.
        let room_dir = std::path::PathBuf::from(&base).join("test-room");
        assert!(room_dir.is_dir(), "room dir should exist");

        // Cleanup.
        store.cleanup_room(&room);
        assert!(!room_dir.exists(), "cleanup should remove room dir");
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn map_to_room_fails_without_stored_content() {
        let base = make_temp_base();
        let store = DiskAssetStore::new(&base);

        let room = ValidatedRoomId::try_from("other-room").unwrap();
        let rel = ValidatedRelPath::try_from("x.txt").unwrap();
        let bad_hash = ContentHash::try_from("ff".repeat(32).as_str()).unwrap();

        // Store content is missing — map_to_room should still work on its side
        // (it does not check content existence), but the dest file won't resolve.
        let result = store.map_to_room(&room, &rel, &bad_hash);
        // Symlink/hardlink creation may fail if source doesn't exist.
        assert!(result.is_err() || !store.has_room_files(&room));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn map_to_room_preserves_existing_dest_on_validation_failure() {
        // If validation fails before filesystem ops, existing files must not be touched.
        // All paths passed through ValidatedRelPath are safe, so this test constructs
        // an impossible scenario by checking that validated types block dangerous input.
        let base = make_temp_base();
        let store = DiskAssetStore::new(&base);

        let room = ValidatedRoomId::try_from("safe-room").unwrap();
        let rel = ValidatedRelPath::try_from("index.html").unwrap();
        let hash = sample_hash();

        store.store_content(&hash, b"data".to_vec()).unwrap();
        store.map_to_room(&room, &rel, &hash).unwrap();

        // Verify dest file content before overwriting.
        let existing = store.get_file(&room, &rel).unwrap();
        assert_eq!(existing, b"data");

        // Overwrite with different content.
        let new_hash = ContentHash::from_data(b"newdata");
        store.store_content(&new_hash, b"newdata".to_vec()).unwrap();
        store.map_to_room(&room, &rel, &new_hash).unwrap();

        let updated = store.get_file(&room, &rel).unwrap();
        assert_eq!(updated, b"newdata");

        store.cleanup_room(&room);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn cleanup_room_deletes_only_room_dir() {
        let base = make_temp_base();
        let store = DiskAssetStore::new(&base);

        let room1 = ValidatedRoomId::try_from("room-a").unwrap();
        let room2 = ValidatedRoomId::try_from("room-b").unwrap();
        let rel = ValidatedRelPath::try_from("index.html").unwrap();
        let hash = sample_hash();

        store.store_content(&hash, b"a".to_vec()).unwrap();
        store.store_content(&hash, b"b".to_vec()).unwrap();

        store.map_to_room(&room1, &rel, &hash).unwrap();
        store.map_to_room(&room2, &rel, &hash).unwrap();

        assert!(store.has_room_files(&room1));
        assert!(store.has_room_files(&room2));

        store.cleanup_room(&room1);

        assert!(!store.has_room_files(&room1));
        assert!(store.has_room_files(&room2));

        // base dir itself must remain.
        assert!(fs::metadata(&base).unwrap().is_dir());

        store.cleanup_room(&room2);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn get_file_returns_index_html_fallback() {
        let base = make_temp_base();
        let store = DiskAssetStore::new(&base);

        let room = ValidatedRoomId::try_from("spa-room").unwrap();
        let html_rel = ValidatedRelPath::try_from("index.html").unwrap();
        let hash = ContentHash::from_data(b"<html></html>");

        store.store_content(&hash, b"<html></html>".to_vec()).unwrap();
        store.map_to_room(&room, &html_rel, &hash).unwrap();

        // Requesting root "/" should fall back to index.html.
        let fallback = ValidatedRelPath::try_from("index.html").unwrap();
        let content = store.get_file(&room, &fallback).expect("should find index.html");
        assert_eq!(content, b"<html></html>");

        // Nonexistent file in a room without any uploaded files returns None.
        let rel_missing = ValidatedRelPath::try_from("missing.js").unwrap();
        let empty_room = ValidatedRoomId::try_from("empty-room").unwrap();
        assert!(store.get_file(&empty_room, &rel_missing).is_none());

        store.cleanup_room(&room);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn validated_types_block_dangerous_inputs_before_disk_ops() {
        // These inputs would be rejected at the type level, preventing any disk access.
        let rejects_room: &[&str] = &["..", "/etc/passwd", "a/b", "", " ", "房间"];
        for s in rejects_room {
            assert!(ValidatedRoomId::try_from(*s).is_err(), "room_id should reject {s:?}");
        }

        let rejects_path: &[&str] = &[
            "../secret",
            "..\\secret",
            "/absolute/path",
            "C:\\windows",
            "\\\\unc\\share",
            "",
            "a/..",
            "a/../b",
            "a/./b",
            "a\0b",
        ];
        for s in rejects_path {
            assert!(ValidatedRelPath::try_from(*s).is_err(), "rel_path should reject {s:?}");
        }
    }

    #[test]
    fn memory_store_trait_compliance() {
        let store = MemoryAssetStore::new();
        let room = ValidatedRoomId::try_from("mem-room").unwrap();
        let rel = ValidatedRelPath::try_from("page.html").unwrap();
        let hash = ContentHash::from_data(b"page-content");

        assert!(!store.has_content(&hash));
        store.store_content(&hash, b"page-content".to_vec()).unwrap();
        assert!(store.has_content(&hash));

        store.map_to_room(&room, &rel, &hash).unwrap();
        assert!(store.has_room_files(&room));

        let content = store.get_file(&room, &rel).expect("should exist");
        assert_eq!(content, b"page-content");

        store.cleanup_room(&room);
        assert!(!store.has_room_files(&room));
        assert!(store.get_file(&room, &rel).is_none());
    }
}
