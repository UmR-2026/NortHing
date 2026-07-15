//! Error banner helpers (R37a split from mod.rs)
//!
//! Owns the 4 P0-C / Phase 5 error-banner setters + `schedule_error_clear`
//! + the `ErrorKind` enum that drives the auto-clear branch.
//!
//! Previously all Slint callback failures wrote to stderr via `eprintln!`
//! and the user saw nothing. These helpers route failures to the Slint
//! `session-error` / `input-error` properties which the SidebarView
//! renders as a red banner at the top. After 5s the banner auto-clears;
//! the user can also click × to dismiss immediately (wired to the
//! `clear-session-error` / `clear-input-error` callbacks in create_ui).

use slint::{ComponentHandle, SharedString};

use super::slint_glue::AppWindow;

/// Set the session-level error banner. Auto-clears after 5s.
pub fn set_session_error(ui: &AppWindow, message: impl Into<String>) {
    let msg = message.into();
    tracing::warn!(target: "app_state", "session_error: {msg}");
    ui.set_session_error(SharedString::from(msg));
    schedule_error_clear(ui.as_weak(), ErrorKind::Session);
}

/// Set the input-level error banner (for input-validation failures
/// like "no session selected"). Auto-clears after 5s.
pub fn set_input_error(ui: &AppWindow, message: impl Into<String>) {
    let msg = message.into();
    tracing::warn!(target: "app_state", "input_error: {msg}");
    ui.set_input_error(SharedString::from(msg));
    schedule_error_clear(ui.as_weak(), ErrorKind::Input);
}

/// 2026-06-26 (Phase 5): set the global MaterialBanner (first channel
/// of the Q8=c dual-channel error design). Auto-clears after 5s via
/// `schedule_error_clear`. When `detail` is non-empty, the banner
/// shows a "详情" button that copies the same error to the inline
/// channel (second channel). Use this for transient / summary errors
/// that the user might want to dig into.
pub fn set_banner_message(ui: &AppWindow, message: impl Into<String>, detail: impl Into<String>) {
    let msg = message.into();
    let det = detail.into();
    tracing::warn!(target: "app_state", "banner_message: {msg}");
    ui.set_banner_message(SharedString::from(msg));
    ui.set_banner_detail(SharedString::from(det));
    schedule_error_clear(ui.as_weak(), ErrorKind::Banner);
}

/// 2026-06-26 (Phase 5): set the chat inline error (second channel).
/// This one does NOT auto-clear — the user has to click × (which
/// fires `clear-inline-error` → `set_chat_inline_error` with empty).
/// Use for errors the user needs to acknowledge (e.g. "上次使用的 AI
/// 服务已被移除，已自动切换。", "LLM 调用失败: ...").
pub fn set_inline_error(ui: &AppWindow, message: impl Into<String>) {
    let msg = message.into();
    tracing::warn!(target: "app_state", "inline_error: {msg}");
    ui.set_chat_inline_error(SharedString::from(msg));
}

#[derive(Copy, Clone)]
enum ErrorKind {
    Session,
    Input,
    Banner,
}

fn schedule_error_clear(ui_weak: slint::Weak<AppWindow>, kind: ErrorKind) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        // 2026-06-26 (review follow-up #2): wrap the Slint setter in
        // `invoke_from_event_loop` so the auto-clear runs on the UI
        // thread. Slint 1.16 silently drops property setters called
        // from non-event-loop threads; without this wrap, the 5s
        // auto-clear fails and the error stays on screen. Same root
        // cause as `bff005a`'s model-status / mcp-status / P0-A
        // fixes and the welcome-route fix in `748f628`. Cleanup
        // review's observation #1.
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match kind {
                    ErrorKind::Session => ui.set_session_error(SharedString::from(String::new())),
                    ErrorKind::Input => ui.set_input_error(SharedString::from(String::new())),
                    ErrorKind::Banner => {
                        ui.set_banner_message(SharedString::from(String::new()));
                        ui.set_banner_detail(SharedString::from(String::new()));
                    }
                }
            }
        });
    });
}
