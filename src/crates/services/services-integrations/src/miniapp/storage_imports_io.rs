//! MiniApp storage import-bundle IO methods (split from storage.rs in R38c).
//!
//! Owns the three import-bundle IO entry points:
//! - `validate_import_layout` — gate that checks a candidate source dir
//!   carries the required files before any other IO runs.
//! - `read_import_meta_json` — read `meta.json` from the source dir, gated
//!   by `validate_import_layout`.
//! - `write_import_bundle` — copy/merge the bundle into a newly-created app
//!   directory, falling back to caller-provided JSON when source files are
//!   absent.
//!
//! These methods stay in a sibling of `storage.rs` because they have no
//! internal sibling split yet, are kept near the customization helpers that
//! complement them, and the boundary `required-rules` does not pull them
//! into either `storage_app_io.rs` or `storage_drafts.rs`.

use northhing_product_domains::miniapp::storage::{
    MiniAppImportBundleWriteRequest, MiniAppImportLayout, COMPILED_HTML, ESM_DEPS_JSON, META_JSON, PACKAGE_JSON,
    REQUIRED_SOURCE_FILES, STORAGE_JSON,
};

use super::storage::{MiniAppStorageError, MiniAppStorageResult};
use std::path::Path;

impl super::storage::MiniAppStorage {
    pub(super) fn validate_import_layout(
        source_path: &Path,
        import_layout: &MiniAppImportLayout,
    ) -> MiniAppStorageResult<()> {
        if !source_path.is_dir() {
            return Err(MiniAppStorageError::validation(format!(
                "Not a directory: {}",
                source_path.display()
            )));
        }

        let meta_path = import_layout.meta_path();
        let source_dir = import_layout.source_dir();
        if !meta_path.exists() {
            return Err(MiniAppStorageError::validation(format!(
                "Missing meta.json in {}",
                source_path.display()
            )));
        }
        if !source_dir.is_dir() {
            return Err(MiniAppStorageError::validation(format!(
                "Missing source/ directory in {}",
                source_path.display()
            )));
        }
        for (required, path) in import_layout.required_source_file_paths() {
            if !path.exists() {
                return Err(MiniAppStorageError::validation(format!(
                    "Missing source/{} in {}",
                    required,
                    source_path.display()
                )));
            }
        }
        Ok(())
    }
    pub async fn read_import_meta_json(&self, source_path: impl AsRef<Path>) -> MiniAppStorageResult<String> {
        let source_path = source_path.as_ref();
        let import_layout = MiniAppImportLayout::new(source_path);
        Self::validate_import_layout(source_path, &import_layout)?;
        tokio::fs::read_to_string(import_layout.meta_path())
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read meta.json: {}", e)))
    }
    pub async fn write_import_bundle(&self, request: MiniAppImportBundleWriteRequest) -> MiniAppStorageResult<()> {
        let import_layout = MiniAppImportLayout::new(&request.source_path);
        Self::validate_import_layout(&request.source_path, &import_layout)?;

        let dest_dir = self.app_dir(&request.app_id);
        let dest_source = self.source_dir(&request.app_id);
        tokio::fs::create_dir_all(&dest_source)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to create app dir: {}", e)))?;

        tokio::fs::write(dest_dir.join(META_JSON), request.meta_json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write meta.json: {}", e)))?;

        let source_dir = import_layout.source_dir();
        for name in REQUIRED_SOURCE_FILES {
            let from = source_dir.join(name);
            let to = dest_source.join(name);
            if from.exists() {
                tokio::fs::copy(&from, &to)
                    .await
                    .map_err(|e| MiniAppStorageError::io(format!("Failed to copy {}: {}", name, e)))?;
            }
        }

        let esm_path = import_layout.esm_dependencies_path();
        if esm_path.exists() {
            tokio::fs::copy(&esm_path, dest_source.join(ESM_DEPS_JSON))
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to copy esm_dependencies.json: {}", e)))?;
        } else {
            tokio::fs::write(dest_source.join(ESM_DEPS_JSON), request.esm_dependencies_json)
                .await
                .map_err(|_| MiniAppStorageError::io("Failed to write esm_dependencies.json"))?;
        }

        let pkg_src = import_layout.package_json_path();
        if pkg_src.exists() {
            tokio::fs::copy(&pkg_src, dest_dir.join(PACKAGE_JSON))
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to copy package.json: {}", e)))?;
        } else {
            tokio::fs::write(dest_dir.join(PACKAGE_JSON), request.package_json)
                .await
                .map_err(|_| MiniAppStorageError::io("Failed to write package.json"))?;
        }

        let storage_src = import_layout.storage_json_path();
        if storage_src.exists() {
            tokio::fs::copy(&storage_src, dest_dir.join(STORAGE_JSON))
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to copy storage.json: {}", e)))?;
        } else {
            tokio::fs::write(dest_dir.join(STORAGE_JSON), request.storage_json)
                .await
                .map_err(|_| MiniAppStorageError::io("Failed to write storage.json"))?;
        }

        tokio::fs::write(dest_dir.join(COMPILED_HTML), request.compiled_html)
            .await
            .map_err(|_| MiniAppStorageError::io("Failed to write placeholder compiled.html"))?;
        Ok(())
    }
}
