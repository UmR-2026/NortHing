pub use northhing_product_domains::miniapp::bridge_builder::{build_bridge_script, build_csp_content};
pub use northhing_product_domains::miniapp::builtin::{
    build_builtin_install_marker, build_builtin_package_json, build_builtin_seed_meta, builtin_content_hash,
    builtin_source_files, legacy_builtin_version_marker_content, parse_builtin_install_marker,
    preserved_builtin_created_at, resolve_builtin_seed_action, resolve_builtin_seed_check,
    serialize_builtin_install_marker, should_seed_builtin_app, BuiltinInstallMarker, BuiltinMiniAppBundle,
    BuiltinSeedAction, BuiltinSeedCheck, BUILTIN_INSTALL_MARKER, BUILTIN_PLACEHOLDER_COMPILED_HTML,
    LEGACY_BUILTIN_VERSION_MARKER,
};
pub use northhing_product_domains::miniapp::compiler::compile;
pub use northhing_product_domains::miniapp::customization::{
    apply_draft_customization_metadata, decline_builtin_update_metadata, declined_builtin_update_needs_local_snapshot,
    is_current_declined_builtin_update, mark_builtin_update_available_metadata, MiniAppCustomizationBaseline,
    MiniAppCustomizationLocalSnapshot, MiniAppCustomizationMetadata, MiniAppCustomizationOrigin,
    MiniAppCustomizationOriginKind, MAX_DECLINED_BUILTIN_UPDATES,
};
pub use northhing_product_domains::miniapp::draft::{
    build_draft_manifest, build_draft_response, MINIAPP_DRAFT_STATUS_APPLIED, MINIAPP_DRAFT_STATUS_DRAFT,
};
pub use northhing_product_domains::miniapp::exporter::{
    build_export_check_result, export_runtime_label, ExportCheckResult, ExportTarget, MISSING_JS_RUNTIME_MESSAGE,
};
pub use northhing_product_domains::miniapp::host_routing::{
    command_basename_allowed, command_basename_for_allowlist, fs_method_access_mode, fs_policy_scopes,
    fs_resolved_path_allowed, host_allowed_by_allowlist, is_host_primitive, plan_fs_host_call,
    plan_fs_legacy_path_check, plan_shell_host_call, shell_exec_cwd, shell_exec_default_env, shell_exec_first_token,
    shell_exec_input_is_empty, shell_exec_timeout_ms, split_host_method, FsAccessMode, MiniAppFsHostCallPlan,
    MiniAppFsHostPathCheck, MiniAppHostPlanErrorKind, MiniAppShellHostCallPlan,
};
pub use northhing_product_domains::miniapp::lifecycle::{
    apply_draft_permission_update_result, apply_draft_source_sync_result, apply_draft_to_active,
    apply_import_runtime_state, apply_recompile_result, apply_sync_from_fs_result, apply_update_patch,
    build_created_app, build_deps_revision, build_runtime_state, build_source_revision, build_worker_revision,
    clear_worker_restart_required_state, ensure_runtime_state, mark_deps_installed_state, prepare_draft_app,
    prepare_rollback_app, workspace_dir_string, MiniAppCreateInput, MiniAppUpdatePatch,
};
pub use northhing_product_domains::miniapp::permission_policy::resolve_policy;
pub use northhing_product_domains::miniapp::ports::{
    MiniAppCompilePort, MiniAppImportFromPathRequest, MiniAppImportPort, MiniAppInstallDepsRequest, MiniAppPortError,
    MiniAppPortErrorKind, MiniAppPortFuture, MiniAppRuntimeFacade, MiniAppRuntimePort, MiniAppStoragePort,
};
pub use northhing_product_domains::miniapp::runtime::{
    candidate_dirs, candidate_executable_path, detect_runtime, runtime_lookup_order, version_manager_roots,
    versioned_executable_candidate, DetectedRuntime, RuntimeKind,
};
pub use northhing_product_domains::miniapp::storage::{
    build_import_bundle_plan, build_import_fallbacks, build_package_json, parse_npm_dependencies,
    MiniAppImportBundlePlanError, MiniAppImportBundleWriteRequest, MiniAppImportLayout, MiniAppStorageLayout,
    COMPILED_HTML, CUSTOMIZATION_JSON, DRAFTS_CLEANUP_MARKER, DRAFTS_CLEANUP_PREFIX, DRAFTS_DIR, DRAFT_JSON,
    EMPTY_ESM_DEPENDENCIES_JSON, EMPTY_STORAGE_JSON, ESM_DEPS_JSON, INDEX_HTML, META_JSON, PACKAGE_JSON,
    PLACEHOLDER_COMPILED_HTML, REQUIRED_SOURCE_FILES, SOURCE_DIR, STORAGE_JSON, STYLE_CSS, UI_JS, VERSIONS_DIR,
    WORKER_JS,
};
pub use northhing_product_domains::miniapp::types::{
    FsPermissions, MiniApp, MiniAppAiContext, MiniAppI18n, MiniAppMeta, MiniAppPermissions, MiniAppRuntimeState,
    MiniAppSource, NetPermissions, NotificationPermissions, NpmDep,
};
pub use northhing_product_domains::miniapp::worker::{
    install_command_for_runtime, plan_install_deps, select_lru_worker, worker_idle_timeout_ms, worker_is_idle,
    worker_pool_at_capacity, InstallDepsPlan, InstallResult,
};
pub use serde_json::json;
pub use std::collections::BTreeMap;
pub use std::future::Future;
pub use std::path::{Path, PathBuf};
pub use std::pin::pin;
pub use std::sync::{Arc, Mutex};
pub use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub struct RuntimePortStub;

impl MiniAppRuntimePort for RuntimePortStub {
    fn detect_runtime(
        &self,
    ) -> MiniAppPortFuture<'_, Option<northhing_product_domains::miniapp::runtime::DetectedRuntime>> {
        Box::pin(async { Ok(None) })
    }

    fn install_deps(&self, _request: MiniAppInstallDepsRequest) -> MiniAppPortFuture<'_, InstallResult> {
        Box::pin(async {
            Ok(InstallResult {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            })
        })
    }
}

#[derive(Clone)]
pub struct ImportPortStub {
    pub storage: StoragePortStub,
    pub meta_json: String,
    pub writes: Arc<Mutex<Vec<MiniAppImportBundleWriteRequest>>>,
}

impl ImportPortStub {
    pub fn new(storage: StoragePortStub, meta_json: String) -> Self {
        Self {
            storage,
            meta_json,
            writes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn writes(&self) -> Vec<MiniAppImportBundleWriteRequest> {
        self.writes.lock().unwrap().clone()
    }
}

impl MiniAppImportPort for ImportPortStub {
    fn read_import_meta_json(&self, _source_path: PathBuf) -> MiniAppPortFuture<'_, String> {
        let meta_json = self.meta_json.clone();
        Box::pin(async move { Ok(meta_json) })
    }

    fn write_import_bundle(&self, request: MiniAppImportBundleWriteRequest) -> MiniAppPortFuture<'_, ()> {
        let storage = self.storage.clone();
        let writes = self.writes.clone();
        Box::pin(async move {
            let meta: MiniAppMeta = serde_json::from_str(&request.meta_json).map_err(|error| {
                MiniAppPortError::new(
                    MiniAppPortErrorKind::Deserialization,
                    format!("Invalid meta.json: {error}"),
                )
            })?;
            let mut app = sample_miniapp_for_lifecycle(MiniAppSource {
                html: "<html><body>imported</body></html>".to_string(),
                ..MiniAppSource::default()
            });
            app.id = request.app_id.clone();
            app.name = meta.name;
            app.description = meta.description;
            app.icon = meta.icon;
            app.category = meta.category;
            app.tags = meta.tags;
            app.compiled_html = request.compiled_html.clone();
            app.updated_at = meta.updated_at;
            app.created_at = meta.created_at;
            storage.state.lock().unwrap().current = app;
            writes.lock().unwrap().push(request);
            Ok(())
        })
    }
}

pub struct CompilePortStub {
    pub calls: Arc<Mutex<Vec<String>>>,
}

impl CompilePortStub {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl MiniAppCompilePort for CompilePortStub {
    fn compile_app(
        &self,
        app_id: String,
        source: MiniAppSource,
        _permissions: MiniAppPermissions,
        theme: String,
        workspace_root: Option<PathBuf>,
    ) -> MiniAppPortFuture<'_, String> {
        let calls = self.calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(format!(
                "{}|{}|{}|{}",
                app_id,
                source.html,
                theme,
                workspace_root
                    .as_deref()
                    .map(Path::to_string_lossy)
                    .unwrap_or_else(|| "".into())
            ));
            Ok(format!("<html>{app_id}:{theme}</html>"))
        })
    }
}

#[derive(Clone)]
pub struct StoragePortStub {
    pub state: Arc<Mutex<StoragePortStubState>>,
}

pub struct StoragePortStubState {
    pub current: MiniApp,
    pub versions: BTreeMap<u32, MiniApp>,
    pub drafts: BTreeMap<(String, String), (MiniApp, serde_json::Value)>,
    pub customization: BTreeMap<String, MiniAppCustomizationMetadata>,
    pub save_count: usize,
    pub saved_version_numbers: Vec<u32>,
    pub deleted_drafts: Vec<(String, String)>,
}

impl StoragePortStub {
    pub fn new(current: MiniApp) -> Self {
        Self {
            state: Arc::new(Mutex::new(StoragePortStubState {
                current,
                versions: BTreeMap::new(),
                drafts: BTreeMap::new(),
                customization: BTreeMap::new(),
                save_count: 0,
                saved_version_numbers: Vec::new(),
                deleted_drafts: Vec::new(),
            })),
        }
    }

    pub fn current(&self) -> MiniApp {
        self.state.lock().unwrap().current.clone()
    }

    pub fn save_count(&self) -> usize {
        self.state.lock().unwrap().save_count
    }

    pub fn saved_version_numbers(&self) -> Vec<u32> {
        self.state.lock().unwrap().saved_version_numbers.clone()
    }

    pub fn customization_metadata(&self, app_id: &str) -> Option<MiniAppCustomizationMetadata> {
        self.state.lock().unwrap().customization.get(app_id).cloned()
    }

    pub fn deleted_drafts(&self) -> Vec<(String, String)> {
        self.state.lock().unwrap().deleted_drafts.clone()
    }
}

impl MiniAppStoragePort for StoragePortStub {
    fn list_app_ids(&self) -> MiniAppPortFuture<'_, Vec<String>> {
        let app_id = self.state.lock().unwrap().current.id.clone();
        Box::pin(async move { Ok(vec![app_id]) })
    }

    fn load(&self, app_id: String) -> MiniAppPortFuture<'_, MiniApp> {
        let result = {
            let state = self.state.lock().unwrap();
            if state.current.id == app_id {
                Ok(state.current.clone())
            } else {
                Err(MiniAppPortError::new(
                    MiniAppPortErrorKind::NotFound,
                    format!("App not found: {app_id}"),
                ))
            }
        };
        Box::pin(async move { result })
    }

    fn load_meta(
        &self,
        app_id: String,
    ) -> MiniAppPortFuture<'_, northhing_product_domains::miniapp::types::MiniAppMeta> {
        let result = {
            let state = self.state.lock().unwrap();
            if state.current.id == app_id {
                Ok((&state.current).into())
            } else {
                Err(MiniAppPortError::new(
                    MiniAppPortErrorKind::NotFound,
                    format!("App not found: {app_id}"),
                ))
            }
        };
        Box::pin(async move { result })
    }

    fn load_source(&self, app_id: String) -> MiniAppPortFuture<'_, MiniAppSource> {
        let result = {
            let state = self.state.lock().unwrap();
            if state.current.id == app_id {
                Ok(state.current.source.clone())
            } else {
                Err(MiniAppPortError::new(
                    MiniAppPortErrorKind::NotFound,
                    format!("App not found: {app_id}"),
                ))
            }
        };
        Box::pin(async move { result })
    }

    fn save(&self, app: MiniApp) -> MiniAppPortFuture<'_, ()> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            state.current = app;
            state.save_count += 1;
            Ok(())
        })
    }

    fn save_version(&self, _app_id: String, version: u32, app: MiniApp) -> MiniAppPortFuture<'_, ()> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            state.versions.insert(version, app);
            state.saved_version_numbers.push(version);
            Ok(())
        })
    }

    fn load_app_storage(&self, _app_id: String) -> MiniAppPortFuture<'_, serde_json::Value> {
        Box::pin(async { Ok(serde_json::json!({})) })
    }

    fn save_app_storage(&self, _app_id: String, _key: String, _value: serde_json::Value) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn load_draft_app(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, MiniApp> {
        let result = self
            .state
            .lock()
            .unwrap()
            .drafts
            .get(&(app_id.clone(), draft_id.clone()))
            .map(|(app, _)| app.clone())
            .ok_or_else(|| {
                MiniAppPortError::new(
                    MiniAppPortErrorKind::NotFound,
                    format!("Draft not found: {app_id}/{draft_id}"),
                )
            });
        Box::pin(async move { result })
    }

    fn load_draft_manifest(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, serde_json::Value> {
        let result = self
            .state
            .lock()
            .unwrap()
            .drafts
            .get(&(app_id.clone(), draft_id.clone()))
            .map(|(_, manifest)| manifest.clone())
            .ok_or_else(|| {
                MiniAppPortError::new(
                    MiniAppPortErrorKind::NotFound,
                    format!("Draft not found: {app_id}/{draft_id}"),
                )
            });
        Box::pin(async move { result })
    }

    fn save_draft(
        &self,
        app_id: String,
        draft_id: String,
        app: MiniApp,
        manifest: serde_json::Value,
    ) -> MiniAppPortFuture<'_, ()> {
        let state = self.state.clone();
        Box::pin(async move {
            state.lock().unwrap().drafts.insert((app_id, draft_id), (app, manifest));
            Ok(())
        })
    }

    fn delete_draft(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, ()> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            state.drafts.remove(&(app_id.clone(), draft_id.clone()));
            state.deleted_drafts.push((app_id, draft_id));
            Ok(())
        })
    }

    fn load_customization_metadata(
        &self,
        app_id: String,
    ) -> MiniAppPortFuture<'_, Option<MiniAppCustomizationMetadata>> {
        let metadata = self.state.lock().unwrap().customization.get(&app_id).cloned();
        Box::pin(async move { Ok(metadata) })
    }

    fn save_customization_metadata(
        &self,
        app_id: String,
        metadata: MiniAppCustomizationMetadata,
    ) -> MiniAppPortFuture<'_, ()> {
        let state = self.state.clone();
        Box::pin(async move {
            state.lock().unwrap().customization.insert(app_id, metadata);
            Ok(())
        })
    }

    fn delete(&self, _app_id: String) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn list_versions(&self, _app_id: String) -> MiniAppPortFuture<'_, Vec<u32>> {
        let versions = self.state.lock().unwrap().versions.keys().copied().collect();
        Box::pin(async move { Ok(versions) })
    }

    fn load_version(&self, _app_id: String, version: u32) -> MiniAppPortFuture<'_, MiniApp> {
        let result = self
            .state
            .lock()
            .unwrap()
            .versions
            .get(&version)
            .cloned()
            .ok_or_else(|| {
                MiniAppPortError::new(MiniAppPortErrorKind::NotFound, format!("Version v{version} not found"))
            });
        Box::pin(async move { result })
    }
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

pub fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

pub fn sample_miniapp_for_lifecycle(source: MiniAppSource) -> MiniApp {
    MiniApp {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: "Demo app".to_string(),
        icon: "sparkles".to_string(),
        category: "tools".to_string(),
        tags: Vec::new(),
        version: 3,
        created_at: 1,
        updated_at: 1234,
        source,
        compiled_html: "<html></html>".to_string(),
        permissions: MiniAppPermissions::default(),
        ai_context: None,
        runtime: MiniAppRuntimeState::default(),
        i18n: None,
    }
}
