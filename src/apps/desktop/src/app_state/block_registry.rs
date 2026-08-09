use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Window};
use slint::platform::WindowAdapter;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_APPWINDOW,
    SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, IsIconic
};

use crate::app_state::slint_glue::{AppWindow, InnerWindow, OuterWindow};

pub fn init_block_registry(
    main_weak: slint::Weak<AppWindow>,
    inner_weak: slint::Weak<InnerWindow>,
    outer_weak: slint::Weak<OuterWindow>,
) {
    if let (Some(m), Some(i), Some(o)) = (main_weak.upgrade(), inner_weak.upgrade(), outer_weak.upgrade()) {
        let m_weak_close_i = m.as_weak();
        i.window().on_close_requested(move || {
            if let Some(m) = m_weak_close_i.upgrade() {
                m.set_left_drawer_open(false);
            }
            slint::CloseRequestResponse::HideWindow
        });

        let m_weak_close_o = m.as_weak();
        o.window().on_close_requested(move || {
            if let Some(m) = m_weak_close_o.upgrade() {
                m.set_right_drawer_open(false);
            }
            slint::CloseRequestResponse::HideWindow
        });
    }

    let init_timer = slint::Timer::default();
    let mut initialized = false;
    let i_weak = inner_weak.clone();
    let o_weak = outer_weak.clone();
    init_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
        if !initialized {
            if let (Some(i), Some(o)) = (i_weak.upgrade(), o_weak.upgrade()) {
                if set_tool_window(i.window()) && set_tool_window(o.window()) {
                    initialized = true;
                }
            }
        }
    });
    std::mem::forget(init_timer);

    let timer = slint::Timer::default();
    let mut was_minimized = false;
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        if let (Some(m), Some(i), Some(o)) = (main_weak.upgrade(), inner_weak.upgrade(), outer_weak.upgrade()) {
            let mut is_minimized = false;
            if let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(&m.window().window_handle()) {
                if let RawWindowHandle::Win32(h) = handle.as_raw() {
                    let hwnd = HWND(h.hwnd.get() as _);
                    unsafe {
                        is_minimized = IsIconic(hwnd).as_bool();
                    }
                }
            }

            let left_open = m.get_left_drawer_open();
            let right_open = m.get_right_drawer_open();

            if is_minimized {
                if !was_minimized {
                    let _ = i.hide();
                    let _ = o.hide();
                    was_minimized = true;
                }
            } else {
                if was_minimized {
                    was_minimized = false;
                }
                
                if left_open && !i.window().is_visible() { let _ = i.show(); }
                if !left_open && i.window().is_visible() { let _ = i.hide(); }
                
                if right_open && !o.window().is_visible() { let _ = o.show(); }
                if !right_open && o.window().is_visible() { let _ = o.hide(); }
                
                let m_pos = m.window().position();
                let m_size = m.window().size();
                let i_size = i.window().size();
                
                if i.window().is_visible() {
                    let target_x = m_pos.x - i_size.width as i32 - 16;
                    let target_y = m_pos.y;
                    i.window().set_position(slint::PhysicalPosition::new(target_x, target_y));
                }
                if o.window().is_visible() {
                    let target_x = m_pos.x + m_size.width as i32 + 16;
                    let target_y = m_pos.y;
                    o.window().set_position(slint::PhysicalPosition::new(target_x, target_y));
                }
            }
        }
    });
    std::mem::forget(timer);
}

fn set_tool_window(window: &slint::Window) -> bool {
    if let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(&window.window_handle()) {
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            let hwnd = HWND(h.hwnd.get() as _);
            unsafe {
                let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                style |= WS_EX_TOOLWINDOW.0 as isize;
                style &= !(WS_EX_APPWINDOW.0 as isize);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
            return true;
        }
    }
    false
}
