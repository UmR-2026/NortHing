// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Window OS FFI + shell close helpers extracted from app.rs (W8-4).
//
// Non-Windows: close_os_window is a no-op; close_module / close_all_modules /
// quit_shell still function (they fall through to Dioxus window().close()).

use dioxus::desktop::window;

use super::registry::ShellWindowManager;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

// ─── Platform FFI ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub(crate) mod win_ops {
    use std::ffi::c_void;

    unsafe extern "system" {
        pub fn ShowWindow(h_wnd: *mut c_void, n_cmd_show: i32) -> i32;
        pub fn PostMessageW(h_wnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> i32;
        pub fn IsWindow(h_wnd: *mut c_void) -> i32;
    }

    pub const WM_CLOSE: u32 = 0x0010;
    pub const SW_HIDE: i32 = 0;

    /// Hides and posts WM_CLOSE to an OS window by HWND, with a background watchdog
    /// (std thread, never use_future) to guarantee window destruction.
    pub fn close_os_window(hwnd: usize) {
        if hwnd == 0 {
            return;
        }
        unsafe {
            ShowWindow(hwnd as *mut c_void, SW_HIDE);
            PostMessageW(hwnd as *mut c_void, WM_CLOSE, 0, 0);
        }
        // W8-4 §4: thread spawn failure is best-effort — log and move on.
        // If the OS couldn't spawn a thread, the WM_CLOSE already posted above
        // still closes the window synchronously; the watchdog is a safety net.
        let hwnd_val = hwnd;
        std::thread::Builder::new()
            .name("window-close-watchdog".into())
            .spawn(move || {
                let hwnd_ptr = hwnd_val as *mut c_void;
                for _ in 0..5 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    unsafe {
                        if IsWindow(hwnd_ptr) == 0 {
                            break;
                        }
                        ShowWindow(hwnd_ptr, SW_HIDE);
                        PostMessageW(hwnd_ptr, WM_CLOSE, 0, 0);
                    }
                }
            })
            .map_err(|e| tracing::warn!("window-close-watchdog spawn failed: {e}"))
            .ok();
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) mod win_ops {
    pub fn close_os_window(_hwnd: usize) {}
}

// ─── Shell close helpers ───────────────────────────────────────────────────

pub(crate) fn close_module(id: &'static str, wm: &ShellWindowManager) {
    if let Some((wid, hwnd)) = wm.mark_closing_target(id) {
        window().close_window(wid);
        win_ops::close_os_window(hwnd);
    }
}

pub(crate) fn close_all_modules(wm: &ShellWindowManager) {
    for (_id, wid, hwnd) in wm.mark_all_closing_targets() {
        // ponytail: dual close paths (Dioxus + native) are redundant on tao-managed windows; safe no-op.
        window().close_window(wid);
        win_ops::close_os_window(hwnd);
    }
}

pub(crate) fn quit_shell(wm: &ShellWindowManager) {
    close_all_modules(wm);
    #[cfg(target_os = "windows")]
    win_ops::close_os_window(window().hwnd() as usize);
    window().close();
}
