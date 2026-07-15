//! Runtime-side helpers on [`MiniAppManager`]:
//! compile sources, resolve permission policy, manage user-granted paths,
//! plus the [`MiniAppCompilePort`] trait impl and the worker-revision builder.

use super::mgr_types::map_northhing_error_to_miniapp_port_error;
use super::MiniAppManager;
use crate::miniapp::compiler::{compile_with_request, MiniAppCompileRequest};
use crate::miniapp::permission_policy::{resolve_policy_with_request, MiniAppPermissionPolicyRequest};
use crate::miniapp::types::{MiniAppPermissions, MiniAppSource};
use crate::miniapp::BUILTIN_APPS;
use crate::product_domain_runtime::CoreProductDomainRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_product_domains::miniapp::customization::MiniAppCustomizationBaseline;
use northhing_product_domains::miniapp::lifecycle::build_worker_revision;
use northhing_product_domains::miniapp::ports::{MiniAppCompilePort, MiniAppPortFuture};
use std::path::{Path, PathBuf};

impl MiniAppManager {
    pub(super) fn runtime_facade(&self) -> northhing_product_domains::miniapp::ports::MiniAppRuntimeFacade<'_> {
        CoreProductDomainRuntime::miniapp_runtime_facade(&self.storage)
    }

    pub fn build_worker_revision(&self, app: &crate::miniapp::types::MiniApp, policy_json: &str) -> String {
        build_worker_revision(app, policy_json)
    }

    pub fn compile_source(
        &self,
        app_id: &str,
        source: &MiniAppSource,
        permissions: &MiniAppPermissions,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<String> {
        let app_data_dir = self.path_manager.miniapp_dir(app_id);
        let request = MiniAppCompileRequest::from_paths(app_id, &app_data_dir, workspace_root, theme);
        compile_with_request(source, permissions, &request)
    }

    pub(super) fn compile_source_with_app_data_dir(
        &self,
        app_id: &str,
        app_data_dir: &Path,
        source: &MiniAppSource,
        permissions: &MiniAppPermissions,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<String> {
        let request = MiniAppCompileRequest::from_paths(app_id, app_data_dir, workspace_root, theme);
        compile_with_request(source, permissions, &request)
    }

    /// Resolve permission policy for the given app (for JS Worker startup).
    pub async fn resolve_policy_for_app(
        &self,
        app_id: &str,
        permissions: &MiniAppPermissions,
        workspace_root: Option<&Path>,
    ) -> serde_json::Value {
        let app_data_dir = self.path_manager.miniapp_dir(app_id);
        let gp = self.granted_paths.read().await;
        let granted = gp.get(app_id).map(Vec::as_slice).unwrap_or(&[]);
        let request = MiniAppPermissionPolicyRequest::from_paths(app_id, &app_data_dir, workspace_root, granted);
        resolve_policy_with_request(permissions, &request)
    }

    pub async fn resolve_policy_for_draft(
        &self,
        app_id: &str,
        draft_id: &str,
        permissions: &MiniAppPermissions,
        workspace_root: Option<&Path>,
    ) -> serde_json::Value {
        let app_data_dir = self.storage.draft_dir(app_id, draft_id);
        let gp = self.granted_paths.read().await;
        let granted = gp.get(app_id).map(Vec::as_slice).unwrap_or(&[]);
        let request = MiniAppPermissionPolicyRequest::from_paths(app_id, &app_data_dir, workspace_root, granted);
        resolve_policy_with_request(permissions, &request)
    }

    /// Snapshot of user-granted extra paths for an app (used by the host-side dispatch
    /// to mirror what `resolve_policy_for_app` would inject into the worker policy).
    pub async fn granted_paths_for_app(&self, app_id: &str) -> Vec<PathBuf> {
        let gp = self.granted_paths.read().await;
        gp.get(app_id).cloned().unwrap_or_default()
    }

    /// Grant workspace access for an app (no-op; workspace context is supplied by caller).
    pub async fn grant_workspace(&self, _app_id: &str) {}

    /// Grant path (user-selected) for an app.
    pub async fn grant_path(&self, app_id: &str, path: PathBuf) {
        let mut guard = self.granted_paths.write().await;
        let list = guard.entry(app_id.to_string()).or_default();
        if !list.contains(&path) {
            list.push(path);
        }
    }

    pub(super) fn customization_baseline(&self, app_id: &str) -> MiniAppCustomizationBaseline {
        if let Some(builtin) = BUILTIN_APPS.iter().find(|builtin| builtin.id == app_id) {
            MiniAppCustomizationBaseline::Builtin {
                builtin_id: builtin.id.to_string(),
                builtin_version: builtin.version,
            }
        } else {
            MiniAppCustomizationBaseline::UserCreated
        }
    }
}

impl MiniAppCompilePort for MiniAppManager {
    fn compile_app(
        &self,
        app_id: String,
        source: MiniAppSource,
        permissions: MiniAppPermissions,
        theme: String,
        workspace_root: Option<PathBuf>,
    ) -> MiniAppPortFuture<'_, String> {
        Box::pin(async move {
            self.compile_source(&app_id, &source, &permissions, &theme, workspace_root.as_deref())
                .map_err(|error: NortHingError| map_northhing_error_to_miniapp_port_error(error))
        })
    }
}
