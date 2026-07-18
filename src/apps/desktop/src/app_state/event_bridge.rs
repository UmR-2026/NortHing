//! Desktop event bridge — subscribes to core events and drives the Slint UI.
//!
//! Bridges the core `EventSubscriber` trait to the desktop UI: streams text
//! chunks into the message list, toggles the streaming flag on turn
//! start/cancel/complete/fail, surfaces turn-failure errors, and tracks the
//! active turn id so the stop button can cancel it.

use super::error_banners::set_session_error;
use super::sessions::build_messages_model;
use super::slint_glue::{AppWindow, MessageItem};
use super::state::AppState;
use northhing_core::agentic::events::router::EventSubscriber;
use northhing_core::util::errors::NortHingResult;
use northhing_events::agentic::ErrorCategory;
use northhing_events::AgenticEvent;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::Arc;

pub struct DesktopEventBridge {
    ui: slint::Weak<AppWindow>,
    app_state: Arc<AppState>,
    draft: std::sync::Mutex<String>,
    last_flush: std::sync::Mutex<std::time::Instant>,
}

impl DesktopEventBridge {
    fn new(ui: slint::Weak<AppWindow>, app_state: Arc<AppState>) -> Self {
        Self {
            ui,
            app_state,
            draft: std::sync::Mutex::new(String::new()),
            last_flush: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    /// Flush the accumulated draft to the UI by fetching the latest messages
    /// from the coordinator and appending a synthetic streaming assistant item.
    fn flush_draft(&self, session_id: &str, draft: String) {
        let ui = self.ui.clone();
        let sid = session_id.to_string();
        // 2026-07-18 (D2j-fix): event bridge subscription runs on a background
        // thread. Fetch messages there, then dispatch only the sync UI set
        // onto the UI thread via invoke_from_event_loop. No nested block_on
        // inside the invoke closure (would panic: "Cannot start a runtime
        // from within a runtime").
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        if let Ok(rt) = rt {
            let _ = rt.block_on(async {
                let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
                    return;
                };
                match coordinator.get_messages(&sid).await {
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

    /// Refresh messages from the coordinator (no synthetic item). Used for
    /// terminal states where the turn file is the source of truth.
    fn refresh_messages(&self, session_id: &str) {
        let ui = self.ui.clone();
        let sid = session_id.to_string();
        // 2026-07-18 (D2j-fix): event bridge subscription runs on a background
        // thread. Fetch messages there, then dispatch only the sync UI set
        // onto the UI thread via invoke_from_event_loop. No nested block_on
        // inside the invoke closure (would panic).
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        if let Ok(rt) = rt {
            let _ = rt.block_on(async {
                if let Some(c) = northhing_core::agentic::coordination::global_coordinator() {
                    if let Ok(msgs) = c.get_messages(&sid).await {
                        let ui_weak = ui.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                let model = build_messages_model(&msgs, None);
                                ui.set_messages(model);
                            }
                        });
                    }
                }
            });
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

/// Map an `ErrorCategory` to a short Chinese suffix for the error banner.
fn error_category_hint(category: &ErrorCategory) -> &'static str {
    match category {
        ErrorCategory::ProviderQuota => "（配额不足，请检查账户余额）",
        ErrorCategory::ProviderBilling => "（套餐已到期或无效）",
        ErrorCategory::Auth => "（鉴权失败，请检查 API Key）",
        ErrorCategory::Permission => "（无权限访问该资源）",
        ErrorCategory::RateLimit => "（请求过于频繁，请稍后重试）",
        ErrorCategory::ProviderUnavailable => "（服务暂时不可用）",
        ErrorCategory::Network => "（网络连接异常）",
        ErrorCategory::Timeout => "（请求超时）",
        ErrorCategory::ContextOverflow => "（上下文超出模型限制）",
        ErrorCategory::InvalidRequest => "（请求格式无效）",
        ErrorCategory::ContentPolicy => "（内容被策略阻止）",
        ErrorCategory::ModelError => "（模型返回错误）",
        ErrorCategory::Unknown => "",
    }
}

#[async_trait::async_trait]
impl EventSubscriber for DesktopEventBridge {
    async fn on_event(&self, event: &AgenticEvent) -> NortHingResult<()> {
        let current_session = self.app_state.get_current_session_id();

        match event {
            AgenticEvent::DialogTurnStarted {
                session_id, turn_id, ..
            } => {
                if session_id != &current_session {
                    return Ok(());
                }
                // Reset draft for the new turn.
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
            AgenticEvent::TextChunk { session_id, text, .. } => {
                if session_id != &current_session {
                    return Ok(());
                }
                // Append to draft, then release the lock immediately.
                {
                    if let Ok(mut d) = self.draft.lock() {
                        d.push_str(text);
                    }
                }
                // Throttle flushes to ≥120ms apart.
                let should_flush = {
                    let Ok(mut last) = self.last_flush.lock() else {
                        return Ok(());
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
                    self.flush_draft(session_id, draft);
                }
            }
            AgenticEvent::DialogTurnCompleted { session_id, .. } => {
                if session_id != &current_session {
                    return Ok(());
                }
                if let Ok(mut d) = self.draft.lock() {
                    d.clear();
                }
                self.app_state.set_active_turn_id(None);
                self.app_state.set_streaming_session(None);

                let ui = self.ui.clone();
                let sid = session_id.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui.upgrade() {
                        ui.set_is_streaming(false);
                    }
                });
                self.refresh_messages(&sid);
            }
            AgenticEvent::DialogTurnCancelled { session_id, .. } => {
                if session_id != &current_session {
                    return Ok(());
                }
                if let Ok(mut d) = self.draft.lock() {
                    d.clear();
                }
                self.app_state.set_active_turn_id(None);
                self.app_state.set_streaming_session(None);

                let ui = self.ui.clone();
                let sid = session_id.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui.upgrade() {
                        ui.set_is_streaming(false);
                    }
                });
                self.refresh_messages(&sid);
            }
            AgenticEvent::DialogTurnFailed {
                session_id,
                error,
                error_category,
                ..
            } => {
                if session_id != &current_session {
                    return Ok(());
                }
                if let Ok(mut d) = self.draft.lock() {
                    d.clear();
                }
                self.app_state.set_active_turn_id(None);
                self.app_state.set_streaming_session(None);

                let mut msg = format!("LLM 调用失败: {error}");
                if let Some(cat) = error_category {
                    let hint = error_category_hint(cat);
                    if !hint.is_empty() {
                        msg.push_str(hint);
                    }
                }

                let ui = self.ui.clone();
                let sid = session_id.clone();
                let msg_clone = msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui.upgrade() {
                        ui.set_is_streaming(false);
                        // 2026-07-18 (D2j): UI thread — pass weak directly; helper upgrades on UI thread.
                        set_session_error(ui.as_weak(), msg_clone);
                    }
                });
                self.refresh_messages(&sid);
            }
            // Other variants are ignored by the desktop bridge.
            _ => {}
        }
        Ok(())
    }
}

/// Construct the bridge and register it with the global coordinator.
///
/// No-ops with a warning log if the coordinator isn't ready yet.
pub(super) fn register_desktop_event_bridge(ui: &AppWindow, app_state: &Arc<AppState>) {
    let bridge = DesktopEventBridge::new(ui.as_weak(), Arc::clone(app_state));
    if let Some(c) = northhing_core::agentic::coordination::global_coordinator() {
        c.subscribe_internal("desktop-ui".to_string(), bridge);
    } else {
        tracing::warn!(
            target: "app_state",
            "global coordinator not available; desktop event bridge not registered"
        );
    }
}
