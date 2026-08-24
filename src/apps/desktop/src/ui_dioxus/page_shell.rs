// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Gap audit refactor (2026-08-25) - reusable scaffolding for module windows.
//
// Every Dioxus module window (`archive`, `space`, `settings`, `onboarding`,
// `self`, `facility`, `work`) repeats the same lifecycle boilerplate:
//
//   1. `WindowDropGuard::new` via `use_hook` (notifies the manager when
//      the window unmounts so a stale generation cannot re-open).
//   2. `register_window_with_hwnd` via `use_effect` (claims the OS window
//      handle for this generation; stale generations self-close).
//   3. `theme_rx.changed()` via `use_future` (mirrors the global theme
//      into a local `Signal<bool>` so render code can read it).
//   4. A standard "✕" close button (hide HWND + post WM_CLOSE on Windows,
//      `window().close()` everywhere; the onmousedown stops drag-bubble
//      from the parent chrome).
//
// Before this refactor those four blocks appeared ~6x across the module
// windows (and ~3x more in `windows.rs` for self/facility/work which the
// brief explicitly excluded — see note below). The inline duplication was
// a regression risk: any change to the lifecycle (e.g. adding a third
// generation tag) had to be made in 6 places.
//
// # Why `windows.rs` is not refactored
// The brief (§P3-A) says: "refactor pages_archive.rs and pages_space.rs
// to use it (the two most complex pages). Keep pages_settings.rs and
// pages_onboarding.rs untouched if they have too many custom states".
// The brief did not call out `windows.rs` at all, and the three window
// components there (`self_app_root`, `facility_app_root`,
// `work_app_root`) need a sibling geometry-follow thread that this
// shell does not own. Migrating them is a follow-up.

use dioxus::desktop::window;
use dioxus::prelude::*;
use std::rc::Rc;

use super::i18n::{keys, LocalePack};
use super::registry::ModuleAppProps;
use super::windows::WindowDropGuard;

#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use super::windows::win::hide_and_close_hwnd;

/// Wire the four lifecycle pieces every module window needs.
///
/// Reads `props.plugin_id`, `props.gen`, `props.manager`, and
/// `props.theme_rx`. Returns a `Signal<bool>` mirroring the global
/// theme: read it for the body `data-theme` attribute and to drive
/// the theme toggle SVG.
///
/// Calls into Dioxus hooks (`use_hook`, `use_effect`, `use_signal`,
/// `use_future`), so call this exactly once at the top of your
/// `*_app_root` function before any other hooks.
///
/// `# ponytail: one helper, four jobs. Splitting per concern would
/// add parameter noise without making the call sites clearer — the
/// lifecycle stages are coupled (drop guard + register effect both
/// need `plugin_id`/`gen`/`manager`; theme future needs the same
/// `theme_rx`).
pub fn use_page_shell(props: &ModuleAppProps) -> Signal<bool> {
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
                hide_and_close_hwnd(hwnd as isize);
                window().close();
            }
        });
    }

    let theme_rx = props.theme_rx.clone();
    let theme_dark = use_signal(|| *theme_rx.borrow());

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

    theme_dark
}

/// Render the standard "✕" close button. `locale` is the same
/// `LocalePack` the rest of the page uses; the button's title and
/// aria-label come from `keys::WINDOW_CLOSE_BTN`. Stops mousedown
/// propagation so the parent's `onmousedown: window().drag()` (the
/// frameless chrome drag) does not fire when the user clicks ✕.
///
/// `# ponytail: the visible button text "✕" is a glyph literal — no
/// reason to thread it through i18n; the underlying meaning (close)
/// is what's translated for accessibility.
pub fn render_close_button(locale: &LocalePack) -> Element {
    rsx! {
        button {
            class: "close-btn",
            title: "{locale.t(keys::WINDOW_CLOSE_BTN)}",
            "aria-label": "{locale.t(keys::WINDOW_CLOSE_BTN)}",
            onmousedown: move |e| { e.stop_propagation(); },
            onclick: move |_| {
                #[cfg(target_os = "windows")]
                hide_and_close_hwnd(window().hwnd() as isize);
                window().close();
            },
            "✕"
        }
    }
}
