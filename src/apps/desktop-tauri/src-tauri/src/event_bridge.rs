//! Event bridge: subscribes to core AgenticEvents and re-emits them as
//! Tauri frontend events. W4b discipline: emit is sync, no block_on,
//! no new runtime, no get_messages in the subscriber.

use northhing_core::agentic::events::EventSubscriber;
use northhing_core::util::errors::NortHingResult;
use northhing_events::AgenticEvent;
use tauri::AppHandle;
use tauri::Emitter;

pub struct TauriEventBridge {
    app: AppHandle,
}

impl TauriEventBridge {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl EventSubscriber for TauriEventBridge {
    async fn on_event(&self, event: &AgenticEvent) -> NortHingResult<()> {
        match event {
            AgenticEvent::TextChunk { session_id, text, .. } => {
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
            AgenticEvent::DialogTurnStarted { session_id, turn_id, .. } => {
                let r = self.app.emit(
                    "chat-turn-state",
                    serde_json::json!({ "session_id": session_id, "turn_id": turn_id, "state": "started" }),
                );
                tracing::info!("chat-turn-state started emit result={:?}", r);
            }
            AgenticEvent::DialogTurnCompleted { session_id, .. } => {
                let r = self.app.emit(
                    "chat-turn-state",
                    serde_json::json!({ "session_id": session_id, "state": "completed" }),
                );
                tracing::info!("chat-turn-state completed emit result={:?}", r);
            }
            AgenticEvent::DialogTurnCancelled { session_id, .. } => {
                let _ = self.app.emit(
                    "chat-turn-state",
                    serde_json::json!({ "session_id": session_id, "state": "cancelled" }),
                );
            }
            AgenticEvent::DialogTurnFailed {
                session_id,
                error,
                ..
            } => {
                let _ = self.app.emit(
                    "chat-turn-state",
                    serde_json::json!({ "session_id": session_id, "state": "failed", "error": error }),
                );
            }
            _ => {}
        }
        Ok(())
    }
}

/// Register the bridge with the global coordinator. If the coordinator
/// isn't ready yet, retry every 500ms on the core runtime until it appears
/// (initialization race).
pub fn register(app: &AppHandle) {
    let bridge = TauriEventBridge::new(app.clone());
    let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
        tracing::info!("desktop-tauri bridge: coordinator not ready, retry loop spawned");
        crate::core_rt::core_rt().spawn(async move {
            for _attempt in 1..=120 {
                if let Some(coordinator) =
                    northhing_core::agentic::coordination::global_coordinator()
                {
                    coordinator.subscribe_internal("desktop-tauri".to_string(), bridge);
                    tracing::info!("desktop-tauri bridge subscribed (via retry loop)");
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            tracing::error!("desktop-tauri bridge: coordinator never became ready; giving up");
        });
        return;
    };
    coordinator.subscribe_internal("desktop-tauri".to_string(), bridge);
    tracing::info!("desktop-tauri bridge subscribed (direct)");
}
