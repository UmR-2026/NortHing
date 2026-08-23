// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W1 (2026-08-15) — module window entry components.
//
// Defines the three main module windows (`self`, `facility`, `work`).
// Dynamic OS window lifecycle is managed by `ShellWindowManager`.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;
use tokio::sync::watch;

use super::css;
use super::entry::DOCK_GAP_PX;
use super::i18n::{keys, LocalePack};
use super::registry::{ModuleAppProps, ShellWindowManager};
use super::state::Geometry;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
mod win {
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

// 假设链：WebView2 进程/窗口销毁与 Dioxus VirtualDom drop 属于同步过程。
// 代码侧已在 notify_closed 增加 catch_unwind 防护，确保 drop 绝对不引发 panic 异常。
// 运行时 Webview 与 VirtualDom 的实际销毁同步时序保留为「待编排者 GUI 复核」。
impl Drop for WindowDropGuard {
    fn drop(&mut self) {
        let id = self.plugin_id;
        let gen = self.gen;
        // W1 残留竞态取证：drop 时序是 D2（提前 drop）根因的观测点。
        // 日志走 debug.log 结构化通道（同 registry 四态）；drop 上下文
        // 安全——fire-and-forget mpsc，无 panic、无阻塞。
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
        Self { plugin_id, gen, manager }
    }
}

/// Self ("它的自我") module window root component.
pub fn self_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let plugin_id = props.plugin_id;
    let gen = props.gen;
    let manager = props.manager.clone();

    let mgr_guard = manager.clone();
    use_hook(move || Rc::new(WindowDropGuard::new(plugin_id, gen, mgr_guard)));

    {
        let manager = manager.clone();
        use_effect(move || {
            let wid = window().id();
            #[cfg(target_os = "windows")]
            let hwnd = window().hwnd() as usize;
            #[cfg(not(target_os = "windows"))]
            let hwnd = 0usize;

            if !manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd) {
                #[cfg(target_os = "windows")]
                win::hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let theme_dark = use_signal(|| *theme_rx.borrow());

    #[cfg(target_os = "windows")]
    {
        let rx = rx_arc.clone();
        let hwnd_usize = window().hwnd() as usize;
        use_hook(move || {
            std::thread::Builder::new()
                .name("self-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        if rx.has_changed().is_err() {
                            break;
                        }
                        let cur = *rx.borrow_and_update();
                        if cur.x == last.x && cur.y == last.y && cur.width == last.width && cur.height == last.height {
                            continue;
                        }
                        last = cur;
                        let dpi = unsafe { win::GetDpiForWindow(hwnd_ptr) };
                        let scale = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
                        let off_x = ((280.0 + DOCK_GAP_PX as f64) * scale) as i32;
                        let target_x = cur.x.saturating_sub(off_x);
                        let target_y = cur.y;
                        let target_w = (280.0 * scale) as i32;
                        let target_h = (cur.height as f64 / 2.0) as i32;

                        unsafe {
                            if win::IsWindow(hwnd_ptr) == 0 || win::IsWindowVisible(hwnd_ptr) == 0 {
                                break;
                            }
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                target_x,
                                target_y,
                                target_w,
                                target_h,
                                0x0004 | 0x0010,
                            );
                        }
                    }
                })
                .expect("spawn self geometry follow thread");
        });
    }

    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    // W2 视觉解耦（2026-08-21，用户定案 §2.2）：self 窗改名「沉积」，
    // 三子模块各自独立成卡（沉积 SEDIMENT / 知识沉积 RAG ← facility 迁入 /
    // 沉积skill 新建 mock）。滚动语义：窗体永不滚动（OVERLAY_CSS），
    // 卡标题钉住（w2-pin）、列表区内滚（w2-scroll）、卡尾钉住（w2-foot）。
    rsx! {
        body {
            "data-theme": "{class}",
            // T7 裁定（③-b 接受+注释）：self/设施两窗共用 `inner` 系历史命名——
            // 二者同属 inner 语义域（它的自我）；改名须同步 OVERLAY_CSS 选择器，
            // 收益纯语义，接受现状；备选改名留 E/F 顺带。
            "data-window": "inner",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "mind",
                div {
                    class: "station-head w2-head",
                    "data-drag": "true",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::INNER_HEAD_TITLE)}",
                    button {
                        class: "fold-btn",
                        onmousedown: move |e| { e.stop_propagation(); },
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            win::hide_and_close_hwnd(window().hwnd());
                            window().close();
                        },
                        "✕"
                    }
                }
                div { class: "mod w2c-sediment",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_SEDIMENT_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_SEDIMENT_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row", "# 边界不是围墙" button { class: "tag-x", "×" } }
                        div { class: "row", "# 观察先于干预" button { class: "tag-x", "×" } }
                        div { class: "row", "# 允许未完成" button { class: "tag-x", "×" } }
                    }
                    div { class: "w2-foot",
                        div { class: "seg-bar",
                            div { class: "seg on" }
                            div { class: "seg on" }
                            div { class: "seg on" }
                            div { class: "seg" }
                            div { class: "seg" }
                        }
                        div { class: "seg-note",
                            "{locale.t(keys::INNER_SECTION_SEDIMENT_NOTE)}"
                        }
                    }
                }
                div { class: "mod w2c-rag",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_RAG_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_RAG_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active",
                            "@philosophy-core "
                            span {
                                class: "tag-x",
                                style: "color:var(--mind-line);cursor:default",
                                "{locale.t(keys::INNER_RAG_MOUNTED)}"
                            }
                        }
                    }
                }
                div { class: "mod w2c-skill",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_SKILL_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_SKILL_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row",
                            "{locale.t(keys::INNER_SKILL_CAND_1)}"
                            span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_SHAPE)}" }
                        }
                        div { class: "row",
                            "{locale.t(keys::INNER_SKILL_CAND_2)}"
                            span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_SHAPE)}" }
                        }
                        div { class: "row",
                            "{locale.t(keys::INNER_SKILL_CAND_3)}"
                            span { class: "w2-stat", "{locale.t(keys::INNER_SKILL_STAT_WATCH)}" }
                        }
                    }
                }
            }
        }
    }
}

/// Facility ("设施") module window root component.
pub fn facility_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let plugin_id = props.plugin_id;
    let gen = props.gen;
    let manager = props.manager.clone();

    let mgr_guard = manager.clone();
    use_hook(move || Rc::new(WindowDropGuard::new(plugin_id, gen, mgr_guard)));

    {
        let manager = manager.clone();
        use_effect(move || {
            let wid = window().id();
            #[cfg(target_os = "windows")]
            let hwnd = window().hwnd() as usize;
            #[cfg(not(target_os = "windows"))]
            let hwnd = 0usize;

            if !manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd) {
                #[cfg(target_os = "windows")]
                win::hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let theme_dark = use_signal(|| *theme_rx.borrow());

    #[cfg(target_os = "windows")]
    {
        let rx = rx_arc.clone();
        let hwnd_usize = window().hwnd() as usize;
        use_hook(move || {
            std::thread::Builder::new()
                .name("facility-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        if rx.has_changed().is_err() {
                            break;
                        }
                        let cur = *rx.borrow_and_update();
                        if cur.x == last.x && cur.y == last.y && cur.width == last.width && cur.height == last.height {
                            continue;
                        }
                        last = cur;
                        let dpi = unsafe { win::GetDpiForWindow(hwnd_ptr) };
                        let scale = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
                        let off_x = ((280.0 + DOCK_GAP_PX as f64) * scale) as i32;
                        let half_h = (cur.height as f64 / 2.0) as i32;
                        let target_x = cur.x.saturating_sub(off_x);
                        let target_y = cur.y + half_h;
                        let target_w = (280.0 * scale) as i32;
                        let target_h = cur.height as i32 - half_h;

                        unsafe {
                            if win::IsWindow(hwnd_ptr) == 0 || win::IsWindowVisible(hwnd_ptr) == 0 {
                                break;
                            }
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                target_x,
                                target_y,
                                target_w,
                                target_h,
                                0x0004 | 0x0010,
                            );
                        }
                    }
                })
                .expect("spawn facility geometry follow thread");
        });
    }

    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    // W2 视觉解耦：设施窗 = 两卡。卡1 RUNTIME（模型引擎 + 上下文 +
    // token 消耗/清空 + 全局设置），卡2 AXIOMS 独立浮卡。RAG 已迁入
    // 沉积窗（self_app_root）。token 消耗为 mock 计数，「清空」归零。
    let mut token_used = use_signal(|| 128_437u64);
    let token_text = fmt_tokens(token_used());
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "inner",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "mind",
                div {
                    class: "station-head facility w2-head",
                    "data-drag": "true",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}",
                    button {
                        class: "fold-btn",
                        onmousedown: move |e| { e.stop_propagation(); },
                        "▴ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            win::hide_and_close_hwnd(window().hwnd());
                            window().close();
                        },
                        "✕"
                    }
                }
                div { class: "mod w2c-runtime",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_RUNTIME_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_RUNTIME_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active", span { class: "dot-radio" }, "Claude 3.7 · 主人格" }
                        div { class: "row", span { class: "dot-radio" }, "route.search: Haiku" }
                        div { class: "row active", span { class: "dot-radio" }, "还宽，慢慢来" }
                        div { class: "seg-bar",
                            div { class: "seg on" }
                            div { class: "seg on" }
                            div { class: "seg" }
                            div { class: "seg" }
                            div { class: "seg" }
                        }
                        div { class: "row w2-token",
                            span { class: "w2-token-label", "{locale.t(keys::INNER_RUNTIME_TOKEN_USAGE)}" }
                            span { class: "w2-token-value", "{token_text}" }
                            button {
                                class: "w2-token-clear",
                                disabled: token_used() == 0,
                                onclick: move |_| { token_used.set(0); },
                                "{locale.t(keys::INNER_RUNTIME_TOKEN_CLEAR)}"
                            }
                        }
                    }
                    button { class: "sys-config w2-foot", "≡ {locale.t(keys::INNER_GLOBAL_SETTINGS)}" }
                }
                div { class: "mod w2c-axioms",
                    div { class: "side-title w2-pin",
                        "{locale.t(keys::INNER_SECTION_AXIOMS_TITLE)} "
                        em { "{locale.t(keys::INNER_SECTION_AXIOMS_EM)}" }
                    }
                    div { class: "w2-scroll",
                        div { class: "row active", span { class: "sq-toggle" }, "维护主体边界" }
                        div { class: "row", span { class: "sq-toggle" }, "隐喻性修辞" }
                    }
                }
            }
        }
    }
}

/// Format a mock token count for the RUNTIME card (`128437 -> "128.4k"`).
/// Values below 1k render verbatim so the post-clear state reads `0`.
fn fmt_tokens(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else {
        format!("{:.1}k", n as f64 / 1000.0)
    }
}

/// Work ("身外之物") module window root component.
pub fn work_app_root(props: ModuleAppProps) -> Element {
    let locale = use_hook(|| Rc::new(LocalePack::load(super::i18n::DEFAULT_LOCALE)));
    let plugin_id = props.plugin_id;
    let gen = props.gen;
    let manager = props.manager.clone();

    let mgr_guard = manager.clone();
    use_hook(move || Rc::new(WindowDropGuard::new(plugin_id, gen, mgr_guard)));

    {
        let manager = manager.clone();
        use_effect(move || {
            let wid = window().id();
            #[cfg(target_os = "windows")]
            let hwnd = window().hwnd() as usize;
            #[cfg(not(target_os = "windows"))]
            let hwnd = 0usize;

            if !manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd) {
                #[cfg(target_os = "windows")]
                win::hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let rx_arc = props.rx.clone();
    let theme_rx = props.theme_rx.clone();
    let theme_dark = use_signal(|| *theme_rx.borrow());

    #[cfg(target_os = "windows")]
    {
        let rx = rx_arc.clone();
        let hwnd_usize = window().hwnd() as usize;
        use_hook(move || {
            std::thread::Builder::new()
                .name("work-geometry-follow".into())
                .spawn(move || {
                    let hwnd_ptr = hwnd_usize as *mut std::ffi::c_void;
                    let mut rx: watch::Receiver<Geometry> = (*rx).clone();
                    let mut last = *rx.borrow();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                        if rx.has_changed().is_err() {
                            break;
                        }
                        let cur = *rx.borrow_and_update();
                        if cur.x == last.x && cur.y == last.y && cur.width == last.width && cur.height == last.height {
                            continue;
                        }
                        last = cur;
                        let dpi = unsafe { win::GetDpiForWindow(hwnd_ptr) };
                        let scale = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
                        let off_x = (DOCK_GAP_PX as f64 * scale) as i32;
                        let target_x = cur.x + cur.width as i32 + off_x;
                        let target_y = cur.y;
                        let target_w = (320.0 * scale) as i32;
                        let target_h = cur.height as i32;

                        unsafe {
                            if win::IsWindow(hwnd_ptr) == 0 || win::IsWindowVisible(hwnd_ptr) == 0 {
                                break;
                            }
                            let _ = win::SetWindowPos(
                                hwnd_ptr,
                                std::ptr::null_mut(),
                                target_x,
                                target_y,
                                target_w,
                                target_h,
                                0x0004 | 0x0010,
                            );
                        }
                    }
                })
                .expect("spawn work geometry follow thread");
        });
    }

    use_future(move || {
        let mut theme_rx = theme_rx.clone();
        let mut theme_dark = theme_dark.clone();
        async move {
            loop {
                if theme_rx.changed().await.is_err() {
                    break;
                }
                theme_dark.set(*theme_rx.borrow());
            }
        }
    });

    let class = if theme_dark() { "dark" } else { "light" };
    rsx! {
        body {
            "data-theme": "{class}",
            "data-window": "outer",
            style { dangerous_inner_html: "{css::truth_css()}" }
            style { dangerous_inner_html: "{css::OVERLAY_CSS}" }
            meta { charset: "UTF-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
            aside {
                id: "work",
                class: "mod",
                "data-drag": "true",
                div {
                    class: "station-head facility",
                    onmousedown: move |_| { window().drag(); },
                    "{locale.t(keys::OUTER_HEAD_TITLE)}",
                    button {
                        class: "fold-btn",
                        id: "work-fold",
                        onmousedown: move |e| { e.stop_propagation(); },
                        "▾ {locale.t(keys::WINDOW_FOLD_BTN)}"
                    }
                    button {
                        class: "close-btn",
                        title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
                        onmousedown: move |e| { e.stop_propagation(); },
                        onclick: move |_| {
                            #[cfg(target_os = "windows")]
                            win::hide_and_close_hwnd(window().hwnd());
                            window().close();
                        },
                        "✕"
                    }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_ROUTING_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_ROUTING_EM)}" }
                    }
                    div { class: "row active",
                        span { class: "dot-radio" },
                        "架构师",
                        span {
                            style: "margin-left:auto;color:var(--mind-line);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_ROUTING_INTERVENING)}"
                        }
                    }
                    div { class: "row",
                        span { class: "dot-radio" },
                        "search · Haiku",
                        span {
                            style: "margin-left:auto;color:var(--faint);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_ROUTING_STANDBY)}"
                        }
                    }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_PLANNER_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_PLANNER_EM)}" }
                    }
                    div { class: "row active",
                        span {
                            class: "plan-check",
                            style: "border-color:var(--accent-solid);background:var(--accent-solid)"
                        },
                        "重新定义对齐 ",
                        span {
                            style: "margin-left:auto;color:var(--mind-line);font-size:10px",
                            "{locale.t(keys::OUTER_SECTION_PLANNER_INPROGRESS)}"
                        }
                    }
                    div { class: "sub-step", "├ 读取沉积记忆" }
                    div { class: "sub-step", "└ 写入行动准则" }
                    div { class: "row done",
                        span { class: "plan-check" },
                        "建立隔离沙盒"
                    }
                }
                div { class: "side-section",
                    div { class: "side-title",
                        "{locale.t(keys::OUTER_SECTION_DIFF_TITLE)} "
                        em { "{locale.t(keys::OUTER_SECTION_DIFF_EM)}" }
                    }
                    div { class: "row",
                        span { class: "fname", "alignment.md" },
                        span { class: "diff-add", "+18" },
                        span { class: "diff-del", "-06" }
                    }
                    div { class: "btn-undo", "{locale.t(keys::OUTER_DIFF_REVERTED)}" }
                }
                div { class: "term-well",
                    "$ northing inspect --boundary\n> 3 observers / clean\n> "
                    span { class: "preview-row", "preview: localhost:4173" }
                    "\n> _"
                }
            }
        }
    }
}
