// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room event bridge (W10-1 split).
// Tiered event buffering: TextChunk lossy (bounded), control events guaranteed (unbounded).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

/// Capacity limit for buffered lossy text chunks before dropping under UI lag.
pub const MAX_PENDING_TEXT_CHUNKS: usize = 256;

/// Tiered event receiver bridging kernel events to the Dioxus UI.
///
/// Implements tiered event buffering (F2):
/// - TextChunk events are lossy: bounded to `MAX_PENDING_TEXT_CHUNKS` to avoid unbounded memory
///   growth when the UI consumer lags or re-renders heavily.
/// - Control events (TurnState, ToolCall, TurnPhase, Banner, Error) are guaranteed: delivered via
///   an unbounded channel so critical state machine transitions (e.g. Completed/Failed) and approval
///   cards are never dropped.
/// - Event ordering is strictly preserved in FIFO order across all event types within a single stream.
pub struct EventReceiver {
    rx: UnboundedReceiver<KernelEventDto>,
    pending_text_chunks: Arc<AtomicUsize>,
}

impl EventReceiver {
    /// Receives the next event from the tiered event channel.
    pub async fn recv(&mut self) -> Option<KernelEventDto> {
        let event = self.rx.recv().await?;
        if matches!(event, KernelEventDto::TextChunk { .. }) {
            self.pending_text_chunks.fetch_sub(1, Ordering::Relaxed);
        }
        Some(event)
    }

    /// Returns the current number of pending lossy text chunks in the queue.
    pub fn pending_text_chunks(&self) -> usize {
        self.pending_text_chunks.load(Ordering::Relaxed)
    }
}

/// Creates an isolated tiered event bridge returning a callback and receiver pair.
///
/// Used by `event_channel()` for live kernel subscriptions and by unit tests for deterministic verification.
pub fn create_event_bridge() -> (Box<dyn Fn(KernelEventDto) + Send + 'static>, EventReceiver) {
    let (tx, rx) = unbounded_channel();
    let pending_text_chunks = Arc::new(AtomicUsize::new(0));
    let pending_counter = pending_text_chunks.clone();

    let callback = Box::new(move |dto: KernelEventDto| match dto {
        KernelEventDto::TextChunk { .. } => {
            let mut current = pending_counter.load(Ordering::Relaxed);
            loop {
                if current >= MAX_PENDING_TEXT_CHUNKS {
                    tracing::debug!(
                        pending = current,
                        max = MAX_PENDING_TEXT_CHUNKS,
                        "ui_dioxus::api dropping TextChunk due to capacity limit"
                    );
                    return;
                }
                match pending_counter.compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(actual) => current = actual,
                }
            }
            let _ = tx.send(dto);
        }
        control_dto => {
            // unbounded channel: send only fails if the receiver was dropped (app shutdown); discard is intentional.
            let _ = tx.send(control_dto);
        }
    });

    (
        callback,
        EventReceiver {
            rx,
            pending_text_chunks,
        },
    )
}

/// Creates a subscription to the kernel event stream and returns a tiered event receiver.
///
/// Converts the callback-based `subscribe_events` interface into an async `EventReceiver`.
/// TextChunk events that exceed capacity (256) are dropped under UI lag, while critical control
/// events (TurnState, ToolCall, etc.) are always delivered without loss.
pub fn event_channel() -> EventReceiver {
    let (callback, rx) = create_event_bridge();
    let subscribe_task = async move {
        if let Err(e) = kernel_facade().subscribe_events(callback).await {
            tracing::warn!("ui_dioxus::api::event_channel subscribe failed: {e}");
        }
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(subscribe_task);
    } else {
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                rt.block_on(subscribe_task);
            }
        });
    }

    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::events::{ToolCallDto, ToolCallPhase};
    use northhing_kernel_api::turn::TurnStateKind;

    #[test]
    fn test_event_channel_returns_receiver() {
        let rx = event_channel();
        drop(rx);
    }

    #[tokio::test]
    async fn test_tiered_event_channel_text_chunk_lossy_control_guaranteed() {
        let (callback, mut rx) = create_event_bridge();

        // 1. Schedulers/kernel emit 356 TextChunks (saturating the 256 MAX_PENDING_TEXT_CHUNKS buffer)
        for i in 0..356 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("chunk-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), MAX_PENDING_TEXT_CHUNKS);

        // 2. Emit critical control events while the lossy channel is 100% saturated
        callback(KernelEventDto::TurnState {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            state: TurnStateKind::Completed,
            duration_ms: Some(123),
            error: None,
            error_kind: None,
        });

        callback(KernelEventDto::ToolCall(ToolCallDto {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            call_id: "c1".into(),
            phase: ToolCallPhase::AwaitingConfirmation,
            name: "execute_cmd".into(),
            summary: "run test".to_string(),
            detail: None,
            result_count: None,
        }));

        // 3. Verify exactly 256 TextChunks arrive in FIFO order (chunks 0..256)
        for i in 0..256 {
            match rx.recv().await {
                Some(KernelEventDto::TextChunk { text, .. }) => {
                    assert_eq!(text, format!("chunk-{i}"));
                }
                other => panic!("expected TextChunk {i}, got {other:?}"),
            }
        }

        // 4. Verify TurnState::Completed is not dropped and arrives immediately after the 256 chunks
        match rx.recv().await {
            Some(KernelEventDto::TurnState { state, turn_id, .. }) => {
                assert!(matches!(state, TurnStateKind::Completed));
                assert_eq!(turn_id, "t1");
            }
            other => panic!("expected TurnState::Completed, got {other:?}"),
        }

        // 5. Verify ToolCall is not dropped
        match rx.recv().await {
            Some(KernelEventDto::ToolCall(tc)) => {
                assert_eq!(tc.call_id, "c1");
                assert_eq!(tc.phase, ToolCallPhase::AwaitingConfirmation);
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }

        // Buffer is fully drained
        assert_eq!(rx.pending_text_chunks(), 0);
    }

    #[tokio::test]
    async fn test_tiered_event_channel_drain_refills_budget() {
        use northhing_kernel_api::turn::TurnStateKind;

        let (callback, mut rx) = create_event_bridge();

        // Fill to capacity
        for i in 0..256 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("c-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), 256);

        // One extra chunk dropped
        callback(KernelEventDto::TextChunk {
            session_id: "s1".into(),
            text: "dropped".into(),
        });
        assert_eq!(rx.pending_text_chunks(), 256);

        // Consume 10 chunks
        for _ in 0..10 {
            assert!(rx.recv().await.is_some());
        }
        assert_eq!(rx.pending_text_chunks(), 246);

        // Send 10 new chunks - should be accepted
        for i in 0..10 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("refill-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), 256);

        // Control event accepted at full capacity
        callback(KernelEventDto::TurnState {
            session_id: "s1".into(),
            turn_id: "t2".into(),
            state: TurnStateKind::Failed,
            duration_ms: None,
            error: Some("test error".into()),
            error_kind: None,
        });

        // Drain remaining 246 initial chunks + 10 refill chunks
        for _ in 0..256 {
            assert!(matches!(rx.recv().await, Some(KernelEventDto::TextChunk { .. })));
        }

        // TurnState arrived safely
        match rx.recv().await {
            Some(KernelEventDto::TurnState { state, error, .. }) => {
                assert!(matches!(state, TurnStateKind::Failed));
                assert_eq!(error.as_deref(), Some("test error"));
            }
            other => panic!("expected TurnState::Failed, got {other:?}"),
        }
    }
}
