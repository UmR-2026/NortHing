// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W10-2 — windows module split (thin shell).
//
// Re-exports the three root components and shared helpers from sibling
// files. All behavior lives in `self.rs`, `facility.rs`, `work.rs`.

mod self_app;
mod facility;
mod work;

pub use self::self_app::self_app_root;
pub use self::facility::facility_app_root;
pub use self::work::work_app_root;

use dioxus::prelude::*;
use std::rc::Rc;
use tokio::sync::watch;

use super::css;
use super::entry::DOCK_GAP_PX;
use super::i18n::{keys, LocalePack};
use super::registry::ShellWindowManager;
use super::state::Geometry;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
pub(crate) mod win {
    use std::ffi::c_void;

    unsafe extern "system" {
        pub fn SetWindowPos(
            h_wnd: *mut c_void,
            h_wnd_insert_after: *mut c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;

        pub fn IsWindow(h_wnd: *mut c_void) -> i32;

        pub fn IsWindowVisible(h_wnd: *mut c_void) -> i32;

        pub fn ShowWindow(h_wnd: *mut c_void, n_cmd_show: i32) -> i32;

        pub fn PostMessageW(h_wnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> i32;

        pub fn GetDpiForWindow(h_wnd: *mut c_void) -> u32;
    }

    pub const WM_CLOSE: u32 = 0x0010;
    pub const SW_HIDE: i32 = 0;

    #[inline]
    pub fn hide_and_close_hwnd(hwnd: isize) {
        if hwnd == 0 {
            return;
        }
        let hwnd_ptr = hwnd as *mut c_void;
        unsafe {
            ShowWindow(hwnd_ptr, SW_HIDE);
            PostMessageW(hwnd_ptr, WM_CLOSE, 0, 0);
        }
    }
}

/// Drop guard to notify the shell window manager when a module window unmounts.
pub struct WindowDropGuard {
    plugin_id: &'static str,
    gen: u64,
    manager: ShellWindowManager,
}

impl Drop for WindowDropGuard {
    fn drop(&mut self) {
        let id = self.plugin_id;
        let gen = self.gen;
        crate::app_state::log::log_debug_event(
            northhing_debug_log::COMP_UI_DIOXUS_WIN,
            "drop_guard",
            id,
            &format!("gen={gen}"),
            None,
        );
        let mgr = self.manager.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            mgr.notify_closed_with_gen(id, gen);
        }));
    }
}

impl WindowDropGuard {
    pub fn new(plugin_id: &'static str, gen: u64, manager: ShellWindowManager) -> Self {
        Self {
            plugin_id,
            gen,
            manager,
        }
    }
}

/// Format a mock token count for the RUNTIME card (`128437 -> "128.4k"`).
/// Values below 1k render verbatim so the post-clear state reads `0`.
pub(crate) fn fmt_tokens(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else {
        format!("{:.1}k", n as f64 / 1000.0)
    }
}