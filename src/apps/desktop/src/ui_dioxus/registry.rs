// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W1 (2026-08-15) — Window plugin registry and shell window manager.
//
// Provides the `WindowPlugin` descriptor and `WindowRegistry` for dynamic
// OS window lifecycle management. Mirrors core "registration only in assembly
// layer" philosophy.

use dioxus::desktop::tao::window::WindowId;
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use super::state::GeometryRxArc;

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Dock position relative to the main room window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSide {
    LeftFull,
    RightFull,
    Center,
    Fullscreen,
}

/// Props for module windows (`self`, `facility`, `work`).
#[derive(Props, Clone)]
pub struct ModuleAppProps {
    pub plugin_id: &'static str,
    pub gen: u64,
    pub rx: GeometryRxArc,
    pub theme_rx: watch::Receiver<bool>,
    pub manager: ShellWindowManager,
}

impl PartialEq for ModuleAppProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Window plugin descriptor carrying its component render function.
#[derive(Clone)]
pub struct WindowPlugin {
    pub id: &'static str,
    pub title: &'static str,
    pub initial_width: f64,
    pub initial_height: f64,
    pub dock_side: DockSide,
    pub component: fn(ModuleAppProps) -> Element,
}

/// Assembly-layer registry holding all window plugin descriptors.
#[derive(Clone, Default)]
pub struct WindowRegistry {
    plugins: HashMap<&'static str, WindowPlugin>,
}

impl WindowRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: WindowPlugin) {
        self.plugins.insert(plugin.id, plugin);
    }

    pub fn get(&self, id: &str) -> Option<&WindowPlugin> {
        self.plugins.get(id)
    }

    /// Assembly layer default registration for consult room module windows.
    pub fn default_registry() -> Self {
        let mut reg = Self::new();
        reg.register(WindowPlugin {
            id: "self",
            // W2.7（2026-08-23）：左列半高对切改为单扇满高「沉积与设施」。
            // facility 插件仍注册（生命周期测试覆盖 mark_opening("facility")），
            // 左宝石不再 spawn 第二扇 OS 窗。
            title: "northhing - 沉积与设施 (dioxus)",
            initial_width: 280.0,
            initial_height: 820.0,
            dock_side: DockSide::LeftFull,
            component: super::windows::self_app_root,
        });
        reg.register(WindowPlugin {
            id: "facility",
            title: "northhing - facility (dioxus)",
            initial_width: 280.0,
            initial_height: 410.0,
            dock_side: DockSide::LeftFull,
            component: super::windows::facility_app_root,
        });
        reg.register(WindowPlugin {
            id: "work",
            title: "northhing - work (dioxus)",
            initial_width: 320.0,
            initial_height: 820.0,
            dock_side: DockSide::RightFull,
            component: super::windows::work_app_root,
        });
        reg.register(WindowPlugin {
            id: "archive",
            title: "northhing - 档案馆 (dioxus)",
            initial_width: 720.0,
            initial_height: 820.0,
            dock_side: DockSide::Center,
            component: super::pages_archive::archive_app_root,
        });
        reg.register(WindowPlugin {
            id: "space",
            title: "northhing - 走廊 (dioxus)",
            initial_width: 760.0,
            initial_height: 820.0,
            dock_side: DockSide::Center,
            component: super::pages_space::space_app_root,
        });
        reg.register(WindowPlugin {
            id: "settings",
            title: "northhing - 全局设置 (dioxus)",
            initial_width: 760.0,
            initial_height: 580.0,
            dock_side: DockSide::Center,
            component: super::pages_settings::settings_app_root,
        });
        reg.register(WindowPlugin {
            id: "onboarding",
            title: "northhing - 房间诞生仪式 (dioxus)",
            initial_width: 1280.0,
            initial_height: 860.0,
            dock_side: DockSide::Fullscreen,
            component: super::pages_onboarding::onboarding_app_root,
        });
        reg
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Opening(u64),
    Open(u64, WindowId, usize),
    Closing,
}

pub struct ShellWindowManagerInner {
    pub registry: Arc<WindowRegistry>,
    active_states: Mutex<HashMap<&'static str, WindowState>>,
    active_tx: watch::Sender<HashSet<&'static str>>,
}

/// Shell window manager tracking active OS window handles and broadcasting
/// window state changes to UI components.
#[derive(Clone)]
pub struct ShellWindowManager {
    inner: Arc<ShellWindowManagerInner>,
}

impl Default for ShellWindowManager {
    fn default() -> Self {
        Self::new(Arc::new(WindowRegistry::default_registry()))
    }
}

impl ShellWindowManager {
    pub fn new(registry: Arc<WindowRegistry>) -> Self {
        let (tx, _) = watch::channel(HashSet::new());
        Self {
            inner: Arc::new(ShellWindowManagerInner {
                registry,
                active_states: Mutex::new(HashMap::new()),
                active_tx: tx,
            }),
        }
    }

    pub fn registry(&self) -> &Arc<WindowRegistry> {
        &self.inner.registry
    }

    pub fn is_active(&self, id: &str) -> bool {
        let guard = self.inner.active_states.lock().unwrap();
        matches!(guard.get(id), Some(WindowState::Opening(_) | WindowState::Open(..)))
    }

    pub fn is_any_active(&self, ids: &[&str]) -> bool {
        let guard = self.inner.active_states.lock().unwrap();
        ids.iter().any(|id| matches!(guard.get(*id), Some(WindowState::Opening(_) | WindowState::Open(..))))
    }

    pub fn subscribe_active(&self) -> watch::Receiver<HashSet<&'static str>> {
        self.inner.active_tx.subscribe()
    }

    pub fn mark_opening(&self, id: &'static str) -> Option<u64> {
        let mut guard = self.inner.active_states.lock().unwrap();
        if matches!(guard.get(id), Some(WindowState::Opening(_) | WindowState::Open(..))) {
            // W1 残留竞态取证（handoff-20260815 §4 建议顺带）：重复
            // mark_opening 被拒 = 宝石 toggle 粘连的候选现场。
            // 日志通道（2026-08-22，handoff-20260821 §3 选项④）：走
            // debug.log 结构化通道而非 stderr——分离式启动（无控制台）
            // 下 eprintln 丢失，debug.log（T2-7 轮转在位）两种启动方式
            // 均可取证。下同四态。
            crate::app_state::log::log_debug_event(
                northhing_debug_log::COMP_UI_DIOXUS_WIN,
                "mark_opening",
                id,
                &format!("REJECTED (state={:?})", guard.get(id)),
                None,
            );
            return None;
        }
        let gen = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        guard.insert(id, WindowState::Opening(gen));
        let active_set: HashSet<&'static str> = guard
            .iter()
            .filter_map(|(&k, &v)| if matches!(v, WindowState::Opening(_) | WindowState::Open(..)) { Some(k) } else { None })
            .collect();
        let _ = self.inner.active_tx.send(active_set);
        crate::app_state::log::log_debug_event(
            northhing_debug_log::COMP_UI_DIOXUS_WIN,
            "mark_opening",
            id,
            &format!("gen={gen}"),
            None,
        );
        Some(gen)
    }

    #[allow(dead_code)]
    pub fn register_window(&self, id: &'static str, gen: u64, window_id: WindowId) -> bool {
        self.register_window_with_hwnd(id, gen, window_id, 0)
    }

    pub fn register_window_with_hwnd(&self, id: &'static str, gen: u64, window_id: WindowId, hwnd: usize) -> bool {
        let mut guard = self.inner.active_states.lock().unwrap();
        match guard.get(id) {
            Some(WindowState::Opening(g)) if *g == gen => {
                guard.insert(id, WindowState::Open(gen, window_id, hwnd));
                let active_set: HashSet<&'static str> = guard
                    .iter()
                    .filter_map(|(&k, &v)| if matches!(v, WindowState::Opening(_) | WindowState::Open(..)) { Some(k) } else { None })
                    .collect();
                let _ = self.inner.active_tx.send(active_set);
                crate::app_state::log::log_debug_event(
                    northhing_debug_log::COMP_UI_DIOXUS_WIN,
                    "register_window",
                    id,
                    &format!("gen={gen} OPEN"),
                    None,
                );
                true
            }
            other => {
                // 世代不匹配/状态缺位 → 组件侧自杀（window().close()）。
                crate::app_state::log::log_debug_event(
                    northhing_debug_log::COMP_UI_DIOXUS_WIN,
                    "register_window",
                    id,
                    &format!("gen={gen} STALE (state={other:?}) -> self-close"),
                    None,
                );
                false
            }
        }
    }

    pub fn mark_closing_target(&self, id: &'static str) -> Option<(WindowId, usize)> {
        let target = self.get_window_target(id);
        let mut guard = self.inner.active_states.lock().unwrap();
        let prev = guard.get(id).copied();
        if prev.is_none() || prev == Some(WindowState::Closing) {
            crate::app_state::log::log_debug_event(
                northhing_debug_log::COMP_UI_DIOXUS_WIN,
                "mark_closing",
                id,
                &format!("NOOP (state={prev:?})"),
                None,
            );
            return None;
        }
        guard.insert(id, WindowState::Closing);
        let active_set: HashSet<&'static str> = guard
            .iter()
            .filter_map(|(&k, &v)| if matches!(v, WindowState::Opening(_) | WindowState::Open(..)) { Some(k) } else { None })
            .collect();
        let _ = self.inner.active_tx.send(active_set);
        crate::app_state::log::log_debug_event(
            northhing_debug_log::COMP_UI_DIOXUS_WIN,
            "mark_closing",
            id,
            &format!("(from={prev:?})"),
            None,
        );
        target
    }

    #[allow(dead_code)]
    pub fn mark_closing(&self, id: &'static str) -> Option<WindowId> {
        self.mark_closing_target(id).map(|(wid, _)| wid)
    }

    pub fn notify_closed_with_gen(&self, id: &'static str, gen: u64) {
        let mut guard = self.inner.active_states.lock().unwrap();
        let should_remove = match guard.get(id) {
            Some(WindowState::Opening(g)) | Some(WindowState::Open(g, _, _)) => *g == gen,
            Some(WindowState::Closing) => true,
            None => false,
        };
        if should_remove {
            guard.remove(id);
            let active_set: HashSet<&'static str> = guard
                .iter()
                .filter_map(|(&k, &v)| if matches!(v, WindowState::Opening(_) | WindowState::Open(..)) { Some(k) } else { None })
                .collect();
            let _ = self.inner.active_tx.send(active_set);
        }
        crate::app_state::log::log_debug_event(
            northhing_debug_log::COMP_UI_DIOXUS_WIN,
            "notify_closed",
            id,
            &format!("gen={gen} removed={should_remove}"),
            None,
        );
    }

    pub fn get_window_target(&self, id: &str) -> Option<(WindowId, usize)> {
        let guard = self.inner.active_states.lock().unwrap();
        match guard.get(id) {
            Some(WindowState::Open(_, wid, hwnd)) => Some((*wid, *hwnd)),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn get_window_id(&self, id: &str) -> Option<WindowId> {
        self.get_window_target(id).map(|(wid, _)| wid)
    }

    #[allow(dead_code)]
    pub fn get_hwnd(&self, id: &str) -> Option<usize> {
        self.get_window_target(id).map(|(_, hwnd)| hwnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_window_manager_clone_state_sharing() {
        let manager1 = ShellWindowManager::default();
        let manager2 = manager1.clone();

        assert!(!manager2.is_active("self"));
        assert!(!manager2.is_any_active(&["self", "facility"]));

        let gen = manager1.mark_opening("self").expect("mark_opening self");
        assert!(manager2.is_active("self"), "manager2.is_active('self') must be true after manager1.mark_opening");
        assert!(manager2.is_any_active(&["self", "facility"]), "manager2.is_any_active must be true after manager1.mark_opening");

        let mock_wid = unsafe { std::mem::transmute(1usize) };
        assert!(manager1.register_window("self", gen, mock_wid));

        assert!(manager2.is_active("self"));
        assert!(manager2.is_any_active(&["self", "facility"]));
        assert_eq!(manager2.get_window_id("self"), Some(mock_wid));

        let closed_wid = manager1.mark_closing("self");
        assert_eq!(closed_wid, Some(mock_wid));
        assert!(!manager2.is_active("self"));
        assert!(!manager2.is_any_active(&["self", "facility"]));
    }

    #[test]
    fn test_notify_closed_with_gen_matching_vs_stale() {
        let manager = ShellWindowManager::default();
        let gen1 = manager.mark_opening("work").expect("mark_opening work gen1");
        let mock_wid1 = unsafe { std::mem::transmute(1usize) };
        assert!(manager.register_window("work", gen1, mock_wid1));

        let gen2 = 9999u64; // Stale generation
        manager.notify_closed_with_gen("work", gen2);
        assert!(manager.is_active("work"), "Stale notify_closed_with_gen MUST be ignored (work stays active)");

        manager.notify_closed_with_gen("work", gen1);
        assert!(!manager.is_active("work"), "Matching notify_closed_with_gen MUST remove active status");
    }

    #[test]
    fn test_register_window_with_hwnd_and_mark_closing_target() {
        let manager = ShellWindowManager::default();
        let gen = manager.mark_opening("facility").expect("mark_opening facility");
        let mock_wid = unsafe { std::mem::transmute(2usize) };
        let mock_hwnd = 0x1234usize;

        assert!(manager.register_window_with_hwnd("facility", gen, mock_wid, mock_hwnd));
        assert_eq!(manager.get_hwnd("facility"), Some(mock_hwnd));
        assert_eq!(manager.get_window_id("facility"), Some(mock_wid));

        let target = manager.mark_closing_target("facility");
        assert_eq!(target, Some((mock_wid, mock_hwnd)));
        assert!(!manager.is_active("facility"));
    }

    #[test]
    fn test_archive_registration_and_lifecycle() {
        let registry = WindowRegistry::default_registry();
        let plugin = registry.get("archive").expect("archive plugin registered");
        assert_eq!(plugin.id, "archive");
        assert_eq!(plugin.dock_side, DockSide::Center);
        assert_eq!(plugin.initial_width, 720.0);
        assert_eq!(plugin.initial_height, 820.0);

        let manager = ShellWindowManager::new(Arc::new(registry));
        assert!(!manager.is_active("archive"));
        let gen = manager.mark_opening("archive").expect("mark_opening archive");
        assert!(manager.is_active("archive"));
        assert!(manager.mark_opening("archive").is_none(), "singleton: duplicate mark_opening must be rejected");

        let mock_wid = unsafe { std::mem::transmute(3usize) };
        assert!(manager.register_window_with_hwnd("archive", gen, mock_wid, 0x5678));
        assert!(manager.is_active("archive"));

        let target = manager.mark_closing_target("archive");
        assert_eq!(target, Some((mock_wid, 0x5678)));
        assert!(!manager.is_active("archive"));
    }

    #[test]
    fn test_space_registration_and_lifecycle() {
        let registry = WindowRegistry::default_registry();
        let plugin = registry.get("space").expect("space plugin registered");
        assert_eq!(plugin.id, "space");
        assert_eq!(plugin.dock_side, DockSide::Center);
        assert_eq!(plugin.initial_height, 820.0);

        let manager = ShellWindowManager::new(Arc::new(registry));
        assert!(!manager.is_active("space"));
        let gen = manager.mark_opening("space").expect("mark_opening space");
        assert!(manager.is_active("space"));
        assert!(manager.mark_opening("space").is_none(), "singleton: duplicate mark_opening must be rejected");

        let mock_wid = unsafe { std::mem::transmute(4usize) };
        assert!(manager.register_window_with_hwnd("space", gen, mock_wid, 0x9abc));
        assert!(manager.is_active("space"));

        let target = manager.mark_closing_target("space");
        assert_eq!(target, Some((mock_wid, 0x9abc)));
        assert!(!manager.is_active("space"));
    }

    #[test]
    fn test_settings_registration_and_lifecycle() {
        let registry = WindowRegistry::default_registry();
        let plugin = registry.get("settings").expect("settings plugin registered");
        assert_eq!(plugin.id, "settings");
        assert_eq!(plugin.dock_side, DockSide::Center);
        assert_eq!(plugin.initial_width, 760.0);
        assert_eq!(plugin.initial_height, 580.0);

        let manager = ShellWindowManager::new(Arc::new(registry));
        assert!(!manager.is_active("settings"));
        let gen = manager.mark_opening("settings").expect("mark_opening settings");
        assert!(manager.is_active("settings"));
        assert!(manager.mark_opening("settings").is_none(), "singleton: duplicate mark_opening must be rejected");

        let mock_wid = unsafe { std::mem::transmute(5usize) };
        assert!(manager.register_window_with_hwnd("settings", gen, mock_wid, 0xdef0));
        assert!(manager.is_active("settings"));

        let target = manager.mark_closing_target("settings");
        assert_eq!(target, Some((mock_wid, 0xdef0)));
        assert!(!manager.is_active("settings"));
    }

    #[test]
    fn test_onboarding_registration_and_lifecycle() {
        let registry = WindowRegistry::default_registry();
        let plugin = registry.get("onboarding").expect("onboarding plugin registered");
        assert_eq!(plugin.id, "onboarding");
        assert_eq!(plugin.dock_side, DockSide::Fullscreen);
        assert_eq!(plugin.initial_width, 1280.0);
        assert_eq!(plugin.initial_height, 860.0);

        let manager = ShellWindowManager::new(Arc::new(registry));
        assert!(!manager.is_active("onboarding"));
        let gen = manager.mark_opening("onboarding").expect("mark_opening onboarding");
        assert!(manager.is_active("onboarding"));
        assert!(manager.mark_opening("onboarding").is_none(), "singleton: duplicate mark_opening must be rejected");

        let mock_wid = unsafe { std::mem::transmute(6usize) };
        assert!(manager.register_window_with_hwnd("onboarding", gen, mock_wid, 0x1357));
        assert!(manager.is_active("onboarding"));

        let target = manager.mark_closing_target("onboarding");
        assert_eq!(target, Some((mock_wid, 0x1357)));
        assert!(!manager.is_active("onboarding"));
    }
}
