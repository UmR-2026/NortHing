//! MiniApp storage - persist and load MiniApp data under user data dir.
//!
//! After R38c this file holds only the foundational types (error enums,
//! service struct, layout/path accessors) and a thin set of cross-sibling
//! helpers (`delete`, customization metadata, drafts helpers). The port
//! adapter lives in `storage_port.rs`, import-bundle IO lives in
//! `storage_imports_io.rs`, app IO in `storage_app_io.rs`, draft IO in
//! `storage_drafts.rs`, and the unit tests in `storage_tests.rs`.

pub use northhing_product_domains::miniapp::customization::MiniAppCustomizationMetadata;
use northhing_product_domains::miniapp::storage::{MiniAppStorageLayout, DRAFTS_CLEANUP_MARKER};
pub use northhing_product_domains::miniapp::types::{MiniApp, MiniAppMeta, MiniAppSource, NpmDep};
use serde_json;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniAppStorageErrorKind {
    NotFound,
    Validation,
    Deserialization,
    Io,
    Backend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniAppStorageError {
    kind: MiniAppStorageErrorKind,
    message: String,
}

impl MiniAppStorageError {
    pub fn new(kind: MiniAppStorageErrorKind, message: impl ToString) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }

    pub fn not_found(message: impl ToString) -> Self {
        Self::new(MiniAppStorageErrorKind::NotFound, message)
    }

    pub fn validation(message: impl ToString) -> Self {
        Self::new(MiniAppStorageErrorKind::Validation, message)
    }

    pub fn parse(message: impl ToString) -> Self {
        Self::new(MiniAppStorageErrorKind::Deserialization, message)
    }

    pub fn io(message: impl ToString) -> Self {
        Self::new(MiniAppStorageErrorKind::Io, message)
    }

    pub fn kind(&self) -> MiniAppStorageErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for MiniAppStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MiniAppStorageError {}

pub type MiniAppStorageResult<T> = Result<T, MiniAppStorageError>;

/// MiniApp storage service (file-based under the MiniApp data directory).
pub struct MiniAppStorage {
    miniapps_dir: PathBuf,
}

impl MiniAppStorage {
    pub fn new(miniapps_dir: PathBuf) -> Self {
        Self { miniapps_dir }
    }

    /// Cross-sibling accessor for the private root directory (R37d facade pattern).
    pub(super) fn miniapps_dir_handle(&self) -> &PathBuf {
        &self.miniapps_dir
    }

    pub(super) fn layout(&self, app_id: &str) -> MiniAppStorageLayout {
        MiniAppStorageLayout::new(&self.miniapps_dir, app_id)
    }

    pub(super) fn app_dir(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).app_dir()
    }

    pub(super) fn meta_path(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).meta_path()
    }

    pub(super) fn source_dir(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).source_dir()
    }

    pub(super) fn compiled_path(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).compiled_path()
    }

    pub(super) fn storage_path(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).storage_path()
    }

    pub(super) fn version_path(&self, app_id: &str, version: u32) -> PathBuf {
        self.layout(app_id).version_path(version)
    }

    pub fn drafts_root(&self) -> PathBuf {
        MiniAppStorageLayout::drafts_root(&self.miniapps_dir)
    }

    pub fn app_drafts_dir(&self, app_id: &str) -> PathBuf {
        MiniAppStorageLayout::app_drafts_dir(&self.miniapps_dir, app_id)
    }

    pub fn draft_dir(&self, app_id: &str, draft_id: &str) -> PathBuf {
        MiniAppStorageLayout::draft_dir(&self.miniapps_dir, app_id, draft_id)
    }

    pub(super) fn cleanup_drafts_root(&self) -> PathBuf {
        MiniAppStorageLayout::cleanup_drafts_root(&self.miniapps_dir, &uuid::Uuid::new_v4().to_string())
    }

    pub(super) fn cleanup_marker_path(&self, drafts_root: &Path) -> PathBuf {
        drafts_root.join(DRAFTS_CLEANUP_MARKER)
    }

    pub(super) fn draft_not_found(app_id: &str, draft_id: &str) -> MiniAppStorageError {
        MiniAppStorageError::not_found(format!("MiniApp draft not found: {}/{}", app_id, draft_id))
    }

    pub(super) fn ensure_active_drafts_root_readable(&self, app_id: &str, draft_id: &str) -> MiniAppStorageResult<()> {
        if self.cleanup_marker_path(&self.drafts_root()).exists() {
            return Err(Self::draft_not_found(app_id, draft_id));
        }
        Ok(())
    }

    pub(super) fn draft_source_dir(&self, app_id: &str, draft_id: &str) -> PathBuf {
        MiniAppStorageLayout::draft_source_dir(&self.miniapps_dir, app_id, draft_id)
    }

    pub(super) fn customization_path(&self, app_id: &str) -> PathBuf {
        self.layout(app_id).customization_path()
    }

    pub async fn load_customization_metadata(
        &self,
        app_id: &str,
    ) -> MiniAppStorageResult<Option<MiniAppCustomizationMetadata>> {
        let path = self.customization_path(app_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read customization metadata: {}", e)))?;
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| MiniAppStorageError::parse(format!("Invalid customization metadata: {}", e)))
    }

    pub async fn save_customization_metadata(
        &self,
        app_id: &str,
        metadata: &MiniAppCustomizationMetadata,
    ) -> MiniAppStorageResult<()> {
        self.ensure_app_dir(app_id).await?;
        let json = serde_json::to_string_pretty(metadata).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(self.customization_path(app_id), json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write customization metadata: {}", e)))?;
        Ok(())
    }
    pub async fn delete(&self, app_id: &str) -> MiniAppStorageResult<()> {
        let dir = self.app_dir(app_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir)
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to delete miniapp dir: {}", e)))?;
        }
        let drafts_dir = self.app_drafts_dir(app_id);
        if drafts_dir.exists() {
            tokio::fs::remove_dir_all(&drafts_dir)
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to delete miniapp drafts: {}", e)))?;
        }
        Ok(())
    }
}
