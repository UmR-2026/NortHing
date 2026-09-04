// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Window OS FFI + shell open/close helpers (W8-4 / W15-1k).
//
// Non-Windows: close_os_window is a no-op; close_module / close_all_modules /
// quit_shell still function (they fall through to Dioxus window().close()).

use dioxus::core::VirtualDom;
use dioxus::desktop::tao::dpi::{LogicalPosition, LogicalSize};
use dioxus::desktop::tao::window::WindowBuilder;
use dioxus::desktop::{window, Config, WindowCloseBehaviour};
use tokio::sync::watch;

use super::entry::{shared_webview_data_directory_for_inner, startup_scale_factor, DOCK_GAP_PX};
use super::registry::{DockSide, ModuleAppProps, ShellWindowManager};
use super::state::{GeometryRxArc, GlobalTheme};

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

// ─── Dynamic module window spawners ────────────────────────────────────────

/// Dynamic module window spawner using `new_window` and `WindowCloseBehaviour::WindowCloses`.
pub fn spawn_module_window(
    id: &'static str,
    manager: &ShellWindowManager,
    geometry_rx: &GeometryRxArc,
    theme: &GlobalTheme,
) {
    let theme_rx = theme.subscribe();
    spawn_module_window_with_theme_rx(id, manager, geometry_rx, theme_rx);
}

/// Dynamic module window spawner accepting a theme receiver.
pub fn spawn_module_window_with_theme_rx(
    id: &'static str,
    manager: &ShellWindowManager,
    geometry_rx: &GeometryRxArc,
    theme_rx: watch::Receiver<bool>,
) {
    let plugin = match manager.registry().get(id) {
        Some(p) => p.clone(),
        None => return,
    };

    let gen = match manager.mark_opening(id) {
        Some(g) => g,
        None => return,
    };

    let data_directory = shared_webview_data_directory_for_inner();

    // I2 审查降级证据（2026-08-22，review-w2 I2 不修的决定依据）：
    // 此处 borrow 到的几何在 gem 可点击前必然已是真实值——两层保证：
    //   1. 通道初值 = 房间创建位（entry.rs initial_geometry 与
    //      with_position 同源常量，非病态占位）；
    //   2. entry.rs tao 事件处理器 pre-mount 接纳（r3p5）：窗口创建
    //      的首个 Moved 事件即发布真实物理几何，早于 webview 渲染。
    // gem 位于 room webview 内，渲染完成才可点击，故「首帧前点击」
    // 时序不可达；残留风险仅 cosmetic。行为不改，避免触碰 W1 取证区。
    let room_geom = *geometry_rx.borrow();
    let scale = startup_scale_factor();
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let room_x_log = room_geom.x as f64 / scale;
    let room_y_log = room_geom.y as f64 / scale;
    let room_w_log = room_geom.width as f64 / scale;
    let room_h_log = room_geom.height as f64 / scale;

    let (initial_x, initial_y, initial_w, initial_h) = match plugin.dock_side {
        DockSide::LeftFull => (
            room_x_log - plugin.initial_width - DOCK_GAP_PX as f64,
            room_y_log,
            plugin.initial_width,
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
        DockSide::RightFull => (
            room_x_log + room_w_log + DOCK_GAP_PX as f64,
            room_y_log,
            plugin.initial_width,
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
        DockSide::Center => (
            room_x_log + (room_w_log - plugin.initial_width) / 2.0,
            room_y_log + 24.0,
            plugin.initial_width,
            plugin.initial_height,
        ),
        DockSide::Fullscreen => (
            room_x_log,
            room_y_log,
            if room_w_log > 0.0 {
                room_w_log
            } else {
                plugin.initial_width
            },
            if room_h_log > 0.0 {
                room_h_log
            } else {
                plugin.initial_height
            },
        ),
    };

    let mut builder = WindowBuilder::new()
        .with_title(plugin.title)
        .with_inner_size(LogicalSize::new(initial_w, initial_h))
        .with_position(LogicalPosition::new(initial_x, initial_y))
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_skip_taskbar(true);
    }

    let cfg = Config::default()
        .with_window(builder)
        .with_close_behaviour(WindowCloseBehaviour::WindowCloses)
        .with_data_directory(data_directory);

    let props = ModuleAppProps {
        plugin_id: id,
        gen,
        rx: geometry_rx.clone(),
        theme_rx,
        manager: manager.clone(),
    };

    let dom = VirtualDom::new_with_props(plugin.component, props);

    // T7 裁定（③-c 接受+注释）：new_window 返回的 PendingDesktopContext 有意丢弃。
    // 影响面 = 放弃经 dioxus DesktopContext API 操控本窗；模块窗生命周期（开/关/析构）
    // 已由 registry + HWND 通道全权负责（W1 racefix，见 registry.rs close_os_window），
    // 且本窗 chrome 只有 收纳/✕，min/max/drag 等 DesktopContext 能力用不上。
    // 若未来确需 dioxus 原生窗控，再透传并 resolve()。
    let _ = window().new_window(dom, cfg);
}
