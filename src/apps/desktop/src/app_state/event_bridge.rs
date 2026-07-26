//! Desktop event bridge — subscribes to kernel events via the facade and drives the Slint UI.
//!
//! Bridges the kernel facade `subscribe_events` API to the desktop UI: streams text
//! chunks into the message list, toggles the streaming flag on turn
//! start/cancel/complete/fail, surfaces turn-failure errors, and tracks the
//! active turn id so the stop button can cancel it.

use super::error_banners::set_session_error;
use super::sessions::build_messages_model;
use super::slint_glue::{AppWindow, MessageItem};
use super::state::AppState;
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi, SubscriptionId};
use northhing_kernel_api::session::KernelSessionApi;
use northhing_kernel_api::turn::TurnStateKind;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::{Arc, Mutex};

pub struct DesktopEventBridge {
    ui: slint::Weak<AppWindow>,
    app_state: Arc<AppState>,
    draft: Mutex<String>,
    last_flush: Mutex<std::time::Instant>,
    subscription_id: Mutex<Option<SubscriptionId>>,
}

impl DesktopEventBridge {
    fn new(ui: slint::Weak<AppWindow>, app_state: Arc<AppState>) -> Self {
        Self {
            ui,
            app_state,
            draft: Mutex::new(String::new()),
            last_flush: Mutex::new(std::time::Instant::now()),
            subscription_id: Mutex::new(None),
        }
    }

    /// Handle a kernel event (called from the facade subscription callback on the
    /// worker runtime). Performs sync state mutations + UI dispatch inline; spawns
    /// async tasks for the message-fetching terminal states.
    fn handle_event(&self, event: &KernelEventDto) {
        let current_session = self.app_state.get_current_session_id();

        match event {
            KernelEventDto::TurnState {
                session_id,
                turn_id,
                state,
                error,
                ..
            } => {
                if session_id != &current_session {
                    return;
                }
                match state {
                    TurnStateKind::Started => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        self.app_state.set_active_turn_id(Some(turn_id.clone()));
                        self.app_state.set_streaming_session(Some(session_id.clone()));
                        let ui = self.ui.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                ui.set_is_streaming(true);
                            }
                        });
                    }
                    TurnStateKind::Completed | TurnStateKind::Cancelled => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        self.app_state.set_active_turn_id(None);
                        self.app_state.set_streaming_session(None);
                        let ui = self.ui.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                ui.set_is_streaming(false);
                            }
                        });
                        self.spawn_refresh_messages(session_id.clone());
                    }
                    TurnStateKind::Failed => {
                        if let Ok(mut d) = self.draft.lock() {
                            d.clear();
                        }
                        self.app_state.set_active_turn_id(None);
                        self.app_state.set_streaming_session(None);
                        let msg = format!("LLM 调用失败: {}", error.as_deref().unwrap_or("unknown error"));
                        let ui = self.ui.clone();
                        let msg_clone = msg.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui.upgrade() {
                                ui.set_is_streaming(false);
                                set_session_error(ui.as_weak(), msg_clone);
                            }
                        });
                        self.spawn_refresh_messages(session_id.clone());
                    }
                }
            }
            KernelEventDto::TextChunk { session_id, text } => {
                if session_id != &current_session {
                    return;
                }
                {
                    if let Ok(mut d) = self.draft.lock() {
                        d.push_str(text);
                    }
                }
                let should_flush = {
                    let Ok(mut last) = self.last_flush.lock() else {
                        return;
                    };
                    let now = std::time::Instant::now();
                    if now.duration_since(*last).as_millis() >= 120 {
                        *last = now;
                        true
                    } else {
                        false
                    }
                };
                if should_flush {
                    let draft = self.draft.lock().map(|d| d.clone()).unwrap_or_default();
                    self.spawn_flush_draft(session_id.clone(), draft);
                }
            }
            // Other variants (ToolCall, TurnPhase, Banner, Error) are ignored by the
            // desktop bridge — the original EventSubscriber impl ignored them too.
            _ => {}
        }
    }

    /// Spawn an async task to refresh messages from the facade (terminal states).
    fn spawn_refresh_messages(&self, session_id: String) {
        let ui = self.ui.clone();
        tokio::spawn(async move {
            let facade = kernel_facade();
            if let Ok(msgs) = facade.get_messages(&session_id).await {
                let ui_weak = ui.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        let model = build_messages_model(&msgs, None);
                        ui.set_messages(model);
                    }
                });
            }
        });
    }

    /// Spawn an async task to flush the draft to the UI (streaming chunk).
    fn spawn_flush_draft(&self, session_id: String, draft: String) {
        let ui = self.ui.clone();
        tokio::spawn(async move {
            let facade = kernel_facade();
            match facade.get_messages(&session_id).await {
                Ok(msgs) => {
                    let base = build_messages_model(&msgs, None);
                    let mut items: Vec<MessageItem> = base.iter().collect();
                    items.push(slint_streaming_item(draft.clone()));
                    let ui_weak = ui.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_messages(ModelRc::new(VecModel::from(items)));
                        }
                    });
                }
                Err(_) => {
                    let items = vec![slint_streaming_item(draft.clone())];
                    let ui_weak = ui.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_messages(ModelRc::new(VecModel::from(items)));
                        }
                    });
                }
            }
        });
    }
}

impl Drop for DesktopEventBridge {
    fn drop(&mut self) {
        let id = self.subscription_id.lock().ok().and_then(|mut g| g.take());
        let Some(id) = id else {
            return;
        };
        let facade = kernel_facade();
        let unsub = async move {
            let _ = facade.unsubscribe_events(id).await;
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(unsub);
        } else if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
            rt.block_on(unsub);
        }
    }
}

/// Build the synthetic streaming assistant message item.
fn slint_streaming_item(content: String) -> MessageItem {
    MessageItem {
        id: SharedString::from("__streaming__"),
        role: SharedString::from("assistant"),
        content: SharedString::from(content),
        timestamp: SharedString::from(""),
        is_streaming: true,
        tool_calls_count: 0,
        tool_calls_summary: SharedString::from(""),
        tool_calls_json: SharedString::from(""),
    }
}

/// Construct the bridge and subscribe to kernel events via the facade.
///
/// No-ops with a warning log if the facade isn't ready yet.
pub(super) fn register_desktop_event_bridge(ui: &AppWindow, app_state: &Arc<AppState>) {
    let bridge = Arc::new(DesktopEventBridge::new(ui.as_weak(), Arc::clone(app_state)));

    // subscribe_events is async; bridge it without nesting a tokio runtime.
    // The actual subscription work is synchronous (just registers a subscriber),
    // so we use block_in_place when a runtime is already running (e.g. tests),
    // or a throwaway current-thread runtime otherwise.
    let subscription_id = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| {
            handle.block_on(async {
                let bridge_for_callback = Arc::clone(&bridge);
                let callback = Box::new(move |event: KernelEventDto| {
                    bridge_for_callback.handle_event(&event);
                });
                kernel_facade().subscribe_events(callback).await
            })
        })
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime for event bridge subscription")
            .block_on(async {
                let bridge_for_callback = Arc::clone(&bridge);
                let callback = Box::new(move |event: KernelEventDto| {
                    bridge_for_callback.handle_event(&event);
                });
                kernel_facade().subscribe_events(callback).await
            })
    };

    match subscription_id {
        Ok(id) => {
            *bridge.subscription_id.lock().unwrap() = Some(id);
        }
        Err(e) => {
            tracing::warn!(
                target: "app_state",
                "failed to subscribe to kernel events: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::events::SubscriptionId;
    use std::sync::Arc;

    #[test]
    fn started_event_tracks_active_turn_for_stop_path() {
        let app_state = Arc::new(AppState::new());
        app_state.set_current_session_id("session-1".to_string());
        let bridge = DesktopEventBridge {
            ui: slint::Weak::default(),
            app_state: Arc::clone(&app_state),
            draft: Mutex::new(String::new()),
            last_flush: Mutex::new(std::time::Instant::now()),
            subscription_id: Mutex::new(None),
        };

        bridge.handle_event(&KernelEventDto::TurnState {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            state: TurnStateKind::Started,
            duration_ms: None,
            error: None,
            error_kind: None,
        });

        assert_eq!(app_state.get_active_turn_id().as_deref(), Some("turn-1"));
        assert_eq!(app_state.get_streaming_session().as_deref(), Some("session-1"));
    }

    /// Regression test: Drop must take the subscription_id exactly once.
    /// If the id is already taken (e.g. a prior cleanup), Drop must not panic
    /// and must not attempt a second unsubscribe.
    #[test]
    fn drop_takes_subscription_id_idempotently() {
        let bridge = DesktopEventBridge {
            ui: slint::Weak::default(),
            app_state: Arc::new(AppState::new()),
            draft: Mutex::new(String::new()),
            last_flush: Mutex::new(std::time::Instant::now()),
            subscription_id: Mutex::new(Some("999".to_string())),
        };

        // Simulate a first cleanup that takes the id.
        let first = bridge.subscription_id.lock().unwrap().take();
        assert!(first.is_some());

        // Drop the bridge — Drop impl will try to take again, get None, and return early.
        // This must not panic.
        drop(bridge);
    }
}
