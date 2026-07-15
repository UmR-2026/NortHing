/// Slint transport adapter
///
/// Bridges AgenticEvent to Slint UI properties via callback mechanism.
/// Uses tokio::mpsc to send events from async context to Slint UI thread.

#[cfg(feature = "slint-adapter")]
use crate::traits::{TextChunk, ToolEventPayload, TransportAdapter};
#[cfg(feature = "slint-adapter")]
use async_trait::async_trait;
#[cfg(feature = "slint-adapter")]
use northhing_events::AgenticEvent;
#[cfg(feature = "slint-adapter")]
use std::fmt;
#[cfg(feature = "slint-adapter")]
use tokio::sync::mpsc;

/// Slint event type (for UI rendering)
#[cfg(feature = "slint-adapter")]
#[derive(Debug, Clone)]
pub enum SlintEvent {
    TextChunk(TextChunk),
    ToolEvent(ToolEventPayload),
    StreamStart {
        session_id: String,
        turn_id: String,
        round_id: String,
    },
    StreamEnd {
        session_id: String,
        turn_id: String,
        round_id: String,
    },
    DialogTurnStarted {
        session_id: String,
        turn_id: String,
    },
    DialogTurnCompleted {
        session_id: String,
        turn_id: String,
        success: Option<bool>,
        finish_reason: Option<String>,
    },
    SessionCreated {
        session_id: String,
        session_name: String,
        agent_type: String,
    },
    SessionDeleted {
        session_id: String,
    },
    TokenUsageUpdated {
        session_id: String,
        input_tokens: u32,
        output_tokens: u32,
        total_tokens: u32,
    },
    ModelRoundCompleted {
        session_id: String,
        duration_ms: u64,
        model_id: String,
    },
    Generic {
        event_name: String,
        payload: serde_json::Value,
    },
}

/// Slint transport adapter
#[cfg(feature = "slint-adapter")]
#[derive(Clone)]
pub struct SlintTransportAdapter {
    tx: mpsc::UnboundedSender<SlintEvent>,
}

#[cfg(feature = "slint-adapter")]
impl SlintTransportAdapter {
    /// Create a new Slint adapter
    pub fn new(tx: mpsc::UnboundedSender<SlintEvent>) -> Self {
        Self { tx }
    }

    /// Create channel and get receiver (for creating Slint UI event loop)
    pub fn create_channel() -> (Self, mpsc::UnboundedReceiver<SlintEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self::new(tx), rx)
    }
}

#[cfg(feature = "slint-adapter")]
impl fmt::Debug for SlintTransportAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlintTransportAdapter")
            .field("adapter_type", &"slint")
            .finish()
    }
}

#[cfg(feature = "slint-adapter")]
#[async_trait]
impl TransportAdapter for SlintTransportAdapter {
    async fn emit_event(&self, _session_id: &str, event: AgenticEvent) -> anyhow::Result<()> {
        let slint_event = match event {
            AgenticEvent::TextChunk {
                session_id,
                turn_id,
                round_id,
                text,
                ..
            } => SlintEvent::TextChunk(TextChunk {
                session_id,
                turn_id,
                round_id,
                text,
                timestamp: chrono::Utc::now().timestamp_millis(),
            }),
            AgenticEvent::DialogTurnStarted {
                session_id, turn_id, ..
            } => SlintEvent::DialogTurnStarted { session_id, turn_id },
            AgenticEvent::DialogTurnCompleted {
                session_id,
                turn_id,
                success,
                finish_reason,
                ..
            } => SlintEvent::DialogTurnCompleted {
                session_id,
                turn_id,
                success,
                finish_reason,
            },
            AgenticEvent::SessionCreated {
                session_id,
                session_name,
                agent_type,
                ..
            } => SlintEvent::SessionCreated {
                session_id,
                session_name,
                agent_type,
            },
            AgenticEvent::SessionDeleted { session_id } => SlintEvent::SessionDeleted { session_id },
            AgenticEvent::TokenUsageUpdated {
                session_id,
                input_tokens,
                output_tokens,
                total_tokens,
                ..
            } => SlintEvent::TokenUsageUpdated {
                session_id,
                input_tokens: input_tokens as u32,
                output_tokens: output_tokens.unwrap_or(0) as u32,
                total_tokens: total_tokens as u32,
            },
            AgenticEvent::ModelRoundCompleted {
                session_id,
                duration_ms,
                model_id,
                ..
            } => SlintEvent::ModelRoundCompleted {
                session_id,
                duration_ms: duration_ms.unwrap_or(0),
                model_id: model_id.unwrap_or_default(),
            },
            _ => return Ok(()),
        };

        self.tx
            .send(slint_event)
            .map_err(|e| anyhow::anyhow!("Failed to send Slint event: {}", e))?;

        Ok(())
    }

    async fn emit_text_chunk(&self, _session_id: &str, chunk: TextChunk) -> anyhow::Result<()> {
        self.tx
            .send(SlintEvent::TextChunk(chunk))
            .map_err(|e| anyhow::anyhow!("Failed to send text chunk: {}", e))?;
        Ok(())
    }

    async fn emit_tool_event(&self, _session_id: &str, event: ToolEventPayload) -> anyhow::Result<()> {
        self.tx
            .send(SlintEvent::ToolEvent(event))
            .map_err(|e| anyhow::anyhow!("Failed to send tool event: {}", e))?;
        Ok(())
    }

    async fn emit_stream_start(&self, session_id: &str, turn_id: &str, round_id: &str) -> anyhow::Result<()> {
        self.tx
            .send(SlintEvent::StreamStart {
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                round_id: round_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to send stream start: {}", e))?;
        Ok(())
    }

    async fn emit_stream_end(&self, session_id: &str, turn_id: &str, round_id: &str) -> anyhow::Result<()> {
        self.tx
            .send(SlintEvent::StreamEnd {
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
                round_id: round_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to send stream end: {}", e))?;
        Ok(())
    }

    async fn emit_generic(&self, event_name: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        self.tx
            .send(SlintEvent::Generic {
                event_name: event_name.to_string(),
                payload,
            })
            .map_err(|e| anyhow::anyhow!("Failed to send generic event: {}", e))?;
        Ok(())
    }

    fn adapter_type(&self) -> &str {
        "slint"
    }
}
