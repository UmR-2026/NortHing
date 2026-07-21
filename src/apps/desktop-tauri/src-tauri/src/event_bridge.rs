//! Event bridge: subscribes to core AgenticEvents via kernel facade and re-emits them as
//! Tauri frontend events. W4b discipline: emit is sync, no block_on,
//! no new runtime, no get_messages in the subscriber.

use northhing_core::kernel_facade::{kernel_facade, KernelFacade};
use northhing_kernel_api::events::{KernelEventDto, ToolCallDto, ToolCallPhase, TurnPhaseKind};
use northhing_kernel_api::{KernelEventsApi, KernelBootstrapApi};
use tauri::AppHandle;
use tauri::Emitter;

pub struct TauriEventBridge {
    app: AppHandle,
}

impl TauriEventBridge {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn emit_chat_chunk(&self, session_id: &str, text: &str) {
        let r = self.app.emit(
            "chat-chunk",
            serde_json::json!({ "session_id": session_id, "text": text }),
        );
        if let Err(e) = r {
            tracing::warn!("chat-chunk emit failed: {e}");
        } else {
            tracing::debug!("chat-chunk emitted session={} len={}", session_id, text.len());
        }
    }

    fn emit_chat_turn_state(&self, session_id: &str, state: &str, turn_id: Option<&str>, duration_ms: Option<u64>, error: Option<&str>, error_kind: Option<&str>) {
        let mut payload = serde_json::json!({
            "session_id": session_id,
            "state": state,
        });
        if let Some(tid) = turn_id {
            payload["turn_id"] = serde_json::json!(tid);
        }
        if let Some(dm) = duration_ms {
            payload["duration_ms"] = serde_json::json!(dm);
        }
        if let Some(err) = error {
            payload["error"] = serde_json::json!(err);
        }
        if let Some(kind) = error_kind {
            payload["error_kind"] = serde_json::json!(kind);
        }
        let r = self.app.emit("chat-turn-state", payload);
        tracing::info!("chat-turn-state emit result={:?}", r);
    }

    fn emit_chat_tool(&self, tool_call: &ToolCallDto) {
        let phase_str = match tool_call.phase {
            ToolCallPhase::Started => "started",
            ToolCallPhase::Completed => "completed",
        };
        let mut payload = serde_json::json!({
            "session_id": tool_call.session_id,
            "turn_id": tool_call.turn_id,
            "call_id": tool_call.call_id,
            "phase": phase_str,
            "name": tool_call.name,
            "summary": tool_call.summary,
        });
        if let Some(ref detail) = tool_call.detail {
            payload["detail"] = serde_json::json!(detail);
        }
        let r = self.app.emit("chat-tool", payload);
        if let Err(e) = r {
            tracing::warn!("chat-tool emit failed: {e}");
        } else {
            tracing::debug!("chat-tool emitted call_id={}", tool_call.call_id);
        }
    }

    fn emit_chat_turn_phase(&self, session_id: &str, turn_id: &str, phase: TurnPhaseKind, tool_name: Option<&str>) {
        let phase_str = match phase {
            TurnPhaseKind::Thinking => "thinking",
            TurnPhaseKind::Generating => "generating",
            TurnPhaseKind::ToolUse => "tool_use",
        };
        let mut payload = serde_json::json!({
            "session_id": session_id,
            "turn_id": turn_id,
            "phase": phase_str,
        });
        if let Some(tn) = tool_name {
            payload["tool_name"] = serde_json::json!(tn);
        }
        let r = self.app.emit("chat-turn-phase", payload);
        if let Err(e) = r {
            tracing::warn!("chat-turn-phase emit failed: {e}");
        } else {
            tracing::debug!("chat-turn-phase emitted session={} turn={} phase={}", session_id, turn_id, phase_str);
        }
    }
}

impl TauriEventBridge {
    fn on_kernel_event(&self, event: KernelEventDto) {
        match event {
            KernelEventDto::TextChunk { session_id, text } => {
                self.emit_chat_chunk(&session_id, &text);
            }
            KernelEventDto::TurnState { session_id, turn_id, state, duration_ms, error, error_kind } => {
                let state_str = match state {
                    northhing_kernel_api::turn::TurnStateKind::Started => "started",
                    northhing_kernel_api::turn::TurnStateKind::Completed => "completed",
                    northhing_kernel_api::turn::TurnStateKind::Failed => "failed",
                    northhing_kernel_api::turn::TurnStateKind::Cancelled => "cancelled",
                };
                let error_kind_str = error_kind.map(|k| match k {
                    northhing_kernel_api::events::TurnErrorKind::Recoverable => "recoverable",
                    northhing_kernel_api::events::TurnErrorKind::Fatal => "fatal",
                });
                self.emit_chat_turn_state(&session_id, state_str, Some(&turn_id), duration_ms, error.as_deref(), error_kind_str);
            }
            KernelEventDto::ToolCall(tool_call) => {
                self.emit_chat_tool(&tool_call);
            }
            KernelEventDto::TurnPhase { session_id, turn_id, phase, tool_name } => {
                self.emit_chat_turn_phase(&session_id, &turn_id, phase, tool_name.as_deref());
            }
            KernelEventDto::Banner { level, message } => {
                tracing::debug!("banner event (ignored for Tauri): {:?} - {}", level, message);
            }
            KernelEventDto::Error { message } => {
                tracing::debug!("error event (ignored for Tauri): {}", message);
            }
        }
    }
}

/// Register the bridge with the kernel facade. Spawns a long-lived task on
/// core_rt that awaits subscribe_events properly. If the facade isn't ready yet,
/// retries every 500ms until core_ready() is true (initialization race).
pub fn register(app: &AppHandle) {
    let bridge = TauriEventBridge::new(app.clone());
    crate::core_rt::core_rt().spawn(async move {
        // Wait for facade to be ready if needed
        if !kernel_facade().core_ready() {
            tracing::info!("desktop-tauri bridge: facade not ready, retry loop spawned");
            for _attempt in 1..=120 {
                if kernel_facade().core_ready() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            if !kernel_facade().core_ready() {
                tracing::error!("desktop-tauri bridge: facade never became ready; giving up");
                return;
            }
        }
        let callback = Box::new(move |event: KernelEventDto| {
            bridge.on_kernel_event(event);
        });
        let _subscription_id = match kernel_facade().subscribe_events(callback).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("desktop-tauri bridge: subscribe_events failed: {e}");
                return;
            }
        };
        tracing::info!("desktop-tauri bridge subscribed (direct)");
    });
}
