use super::load_app_settings_quiet;
use super::refresh_settings_lists;
use super::save_app_settings_quiet;
use crate::app_state::error_banners::{set_banner_message, set_inline_error};
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use slint::{ComponentHandle, SharedString};
use std::sync::Arc;

pub(crate) fn register_remove_workspace_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- remove-workspace (Q7) ---
    let app_state_arc_rm_ws = std::sync::Arc::clone(&app_state);
    let ui_weak_rm_ws = ui.as_weak();
    ui.on_remove_workspace(move |workspace_path| {
        let wpath = std::path::PathBuf::from(workspace_path.to_string());
        let app_state = Arc::clone(&app_state_arc_rm_ws);
        let ui_weak = ui_weak_rm_ws.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "Phase 5: failed to build runtime for remove-workspace: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak.clone(), e, "");
                        return;
                    }
                };
                let workspace_name = wpath.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                let _ = s.remove_workspace(&wpath);
                if let Err(e) = save_app_settings_quiet(&s).await {
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak.clone(), e, "");
                    return;
                }

                let snapshot = app_state.session_metadata_snapshot();
                let session_ids: Vec<String> = snapshot.iter().map(|(id, _)| id.clone()).collect();
                let provider_lookup = |sid: &str| -> Option<String> {
                    snapshot
                        .iter()
                        .find(|(id, _)| id == sid)
                        .map(|(_, m)| m.provider_id.clone())
                };
                let workspace_lookup = |sid: &str| -> Option<std::path::PathBuf> {
                    snapshot
                        .iter()
                        .find(|(id, _)| id == sid)
                        .map(|(_, m)| m.workspace_path.clone())
                };
                let issues = s.validate_session_integrity(session_ids, &provider_lookup, &workspace_lookup);

                // 2026-07-18 (D2j): pass weak directly; helpers upgrade on UI thread.
                let q7_count = issues.iter().filter(|i| i.kind == "workspace-removed").count();
                let name = if workspace_name.is_empty() {
                    wpath.to_string_lossy().to_string()
                } else {
                    workspace_name
                };
                if q7_count > 0 {
                    let detail = format!("{} 个会话已标记为工作文件夹已移除，无法继续聊天。", q7_count);
                    set_banner_message(ui_weak.clone(), format!("已删除工作文件夹 {}", name), detail);
                    set_inline_error(ui_weak.clone(), "工作文件夹已移除，无法继续聊天。");
                } else {
                    set_banner_message(ui_weak.clone(), format!("已删除工作文件夹 {}", name), "");
                }
                // 2026-07-18 (D2b): refresh settings lists after save.
                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}

// 2026-06-26 (Phase 4 fix): pick-folder handler. The Slint callback runs
// on the UI thread; rfd::FileDialog::pick_folder() is a blocking modal, but
// that is acceptable here since the user explicitly clicked the button and
// the UI is expected to block until they choose. On success we persist the
// new workspace and reflect the chosen path back into the welcome view via
// the bound `welcome-step1-path` property.
pub(crate) fn register_pick_folder_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_pick_folder(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let path = rfd::FileDialog::new().set_title("选择工作文件夹").pick_folder();
        let Some(folder) = path else {
            return;
        };
        let path_str = folder.to_string_lossy().to_string();
        let ui_weak2 = ui.as_weak();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "pick-folder: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak2.clone(), e, "");
                        return;
                    }
                };
                s.add_workspace(folder.clone());
                if let Err(e) = save_app_settings_quiet(&s).await {
                    tracing::warn!(target: "app_state", "pick-folder save failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak2.clone(), e, "");
                    return;
                }
                // 2026-07-18 (D2b): refresh settings lists after save.
                // 2026-07-18 (D2j): pass weak directly; function upgrades on UI thread.
                refresh_settings_lists(ui_weak2.clone()).await;
                let ui_weak3 = ui_weak2.clone();
                if let Err(e) = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak3.upgrade() {
                        ui.set_welcome_step1_path(slint::SharedString::from(path_str.clone()));
                    }
                }) {
                    tracing::warn!(
                        target: "app_state",
                        "pick-folder: failed to dispatch step1-path to UI thread: {e}"
                    );
                }
            });
        });
    });
}

// 2026-06-26 (Phase 4 fix): add-workspace handler (manual path entry).
// Mirrors the pick-folder persistence path but takes an explicit path
// string. `set_current` updates `current_workspace` when true.
pub(crate) fn register_add_workspace_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_add_workspace(move |path, name, set_current| {
        let p = std::path::PathBuf::from(path.to_string());
        let display = name.to_string();
        let set_cur = set_current;
        let ui_weak2 = ui_weak.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "add-workspace: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak2.clone(), e, "");
                        return;
                    }
                };
                s.add_workspace(p.clone());
                if !display.is_empty() {
                    if let Some(w) = s.workspaces.iter_mut().find(|w| w.path == p) {
                        w.display_name = display;
                    }
                }
                if set_cur {
                    s.set_current_workspace(Some(&p));
                }
                if let Err(e) = save_app_settings_quiet(&s).await {
                    tracing::warn!(target: "app_state", "add-workspace save failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak2.clone(), e, "");
                    return;
                }
                // 2026-07-18 (D2b): refresh settings lists after save.
                // 2026-07-18 (D2j): pass weak directly; function upgrades on UI thread.
                refresh_settings_lists(ui_weak2.clone()).await;
            });
        });
    });
}
