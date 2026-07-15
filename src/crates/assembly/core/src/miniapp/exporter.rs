//! MiniApp export engine — export to Electron or Tauri standalone app (skeleton).

pub use northhing_product_domains::miniapp::exporter::{
    build_export_check_result, ExportCheckResult, ExportOptions, ExportResult, ExportTarget,
};

use crate::util::errors::{NortHingError, NortHingResult};
use std::path::PathBuf;
use std::sync::Arc;

/// Export engine: check prerequisites and export MiniApp to standalone app.
pub struct MiniAppExporter {
    // reason: path_manager is held for the upcoming path-resolver integration in the export pipeline; today's export derives paths inline
    path_manager: Arc<crate::infrastructure::PathManager>,
    // reason: templates_dir is held for the upcoming template-driven export; today's export reads templates from a hardcoded location
    templates_dir: PathBuf,
}

impl MiniAppExporter {
    pub fn new(path_manager: Arc<crate::infrastructure::PathManager>, templates_dir: PathBuf) -> Self {
        Self {
            path_manager,
            templates_dir,
        }
    }

    /// Check if export is possible (runtime, electron-builder, etc.).
    pub async fn check(&self, _app_id: &str) -> NortHingResult<ExportCheckResult> {
        let runtime = crate::miniapp::runtime_detect::detect_runtime();
        Ok(build_export_check_result(runtime.as_ref().map(|runtime| &runtime.kind)))
    }

    /// Export the MiniApp to a standalone application.
    pub async fn export(&self, _app_id: &str, _options: ExportOptions) -> NortHingResult<ExportResult> {
        Err(NortHingError::validation(
            "Export not yet implemented (skeleton)".to_string(),
        ))
    }
}
