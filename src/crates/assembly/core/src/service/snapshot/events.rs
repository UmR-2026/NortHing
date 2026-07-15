//! Snapshot system events
//!
//! Defines all event types for the snapshot/operation history system, for real-time push to the frontend.

use crate::infrastructure::events::EventEmitter;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tracing::{debug, info};

/// Snapshot event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SnapshotEvent {
    /// Session created
    SessionCreated {
        session_id: String,
        agent_type: String,
        timestamp: u64,
    },

    /// Session state changed
    SessionStateChanged {
        session_id: String,
        status: String,
        timestamp: u64,
    },

    /// File modification started
    FileModificationStarted {
        session_id: String,
        operation_id: String,
        file_path: String,
        operation_type: String,
        timestamp: u64,
    },

    /// File modification completed
    FileModificationCompleted {
        session_id: String,
        operation_id: String,
        file_path: String,
        lines_added: usize,
        lines_removed: usize,
        timestamp: u64,
    },

    /// File state updated (for real-time file tree updates)
    FileStateUpdated {
        session_id: String,
        file_path: String,
        status: String, // "created" | "modified" | "deleted"
        lines_added: usize,
        lines_removed: usize,
        timestamp: u64,
    },

    /// Dialog turn completed
    DialogTurnCompleted {
        session_id: String,
        turn_id: String,
        turn_index: usize,
        files_changed: usize,
        lines_added: usize,
        lines_removed: usize,
        timestamp: u64,
    },

    /// Session rolled back
    SessionRolledBack {
        session_id: String,
        target_turn: usize,
        affected_files: Vec<String>,
        timestamp: u64,
    },

    /// Diff state updated
    DiffStateUpdated {
        session_id: String,
        total_files_modified: usize,
        total_lines_added: usize,
        total_lines_removed: usize,
        timestamp: u64,
    },

    /// Error event
    Error {
        session_id: Option<String>,
        error_type: String,
        message: String,
        timestamp: u64,
    },
}

impl SnapshotEvent {
    /// Returns the session ID associated with the event.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionCreated { session_id, .. } => Some(session_id),
            Self::SessionStateChanged { session_id, .. } => Some(session_id),
            Self::FileModificationStarted { session_id, .. } => Some(session_id),
            Self::FileModificationCompleted { session_id, .. } => Some(session_id),
            Self::FileStateUpdated { session_id, .. } => Some(session_id),
            Self::DialogTurnCompleted { session_id, .. } => Some(session_id),
            Self::SessionRolledBack { session_id, .. } => Some(session_id),
            Self::DiffStateUpdated { session_id, .. } => Some(session_id),
            Self::Error { session_id, .. } => session_id.as_deref(),
        }
    }

    /// Returns the event timestamp.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::SessionCreated { timestamp, .. } => *timestamp,
            Self::SessionStateChanged { timestamp, .. } => *timestamp,
            Self::FileModificationStarted { timestamp, .. } => *timestamp,
            Self::FileModificationCompleted { timestamp, .. } => *timestamp,
            Self::FileStateUpdated { timestamp, .. } => *timestamp,
            Self::DialogTurnCompleted { timestamp, .. } => *timestamp,
            Self::SessionRolledBack { timestamp, .. } => *timestamp,
            Self::DiffStateUpdated { timestamp, .. } => *timestamp,
            Self::Error { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the current timestamp (milliseconds).
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Creates a session created event.
    pub fn session_created(session_id: String, agent_type: String) -> Self {
        Self::SessionCreated {
            session_id,
            agent_type,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a file modification started event.
    pub fn file_modification_started(
        session_id: String,
        operation_id: String,
        file_path: PathBuf,
        operation_type: String,
    ) -> Self {
        Self::FileModificationStarted {
            session_id,
            operation_id,
            file_path: file_path.to_string_lossy().to_string(),
            operation_type,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a file modification completed event.
    pub fn file_modification_completed(
        session_id: String,
        operation_id: String,
        file_path: PathBuf,
        lines_added: usize,
        lines_removed: usize,
    ) -> Self {
        Self::FileModificationCompleted {
            session_id,
            operation_id,
            file_path: file_path.to_string_lossy().to_string(),
            lines_added,
            lines_removed,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a file state updated event.
    pub fn file_state_updated(
        session_id: String,
        file_path: PathBuf,
        status: String,
        lines_added: usize,
        lines_removed: usize,
    ) -> Self {
        Self::FileStateUpdated {
            session_id,
            file_path: file_path.to_string_lossy().to_string(),
            status,
            lines_added,
            lines_removed,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a dialog turn completed event.
    pub fn dialog_turn_completed(
        session_id: String,
        turn_id: String,
        turn_index: usize,
        files_changed: usize,
        lines_added: usize,
        lines_removed: usize,
    ) -> Self {
        Self::DialogTurnCompleted {
            session_id,
            turn_id,
            turn_index,
            files_changed,
            lines_added,
            lines_removed,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Creates a diff state updated event.
    pub fn diff_state_updated(
        session_id: String,
        total_files_modified: usize,
        total_lines_added: usize,
        total_lines_removed: usize,
    ) -> Self {
        Self::DiffStateUpdated {
            session_id,
            total_files_modified,
            total_lines_added,
            total_lines_removed,
            timestamp: Self::current_timestamp(),
        }
    }
}

/// Snapshot event emitter trait
#[async_trait::async_trait]
pub trait SnapshotEventEmitter: Send + Sync {
    /// Emits an event.
    async fn emit(&self, event: SnapshotEvent) -> Result<(), String>;

    /// Emits an event to a specific session.
    async fn emit_to_session(&self, session_id: &str, event: SnapshotEvent) -> Result<(), String>;
}

/// Snapshot emitter adapter implementation - uses the generic `EventEmitter`.
pub struct SnapshotEmitterAdapter {
    emitter: Option<Arc<dyn EventEmitter>>,
}

impl SnapshotEmitterAdapter {
    pub fn new(emitter: Option<Arc<dyn EventEmitter>>) -> Self {
        Self { emitter }
    }

    pub fn set_emitter(&mut self, emitter: Arc<dyn EventEmitter>) {
        self.emitter = Some(emitter);
    }
}

#[async_trait::async_trait]
impl SnapshotEventEmitter for SnapshotEmitterAdapter {
    async fn emit(&self, event: SnapshotEvent) -> Result<(), String> {
        if let Some(ref emitter) = self.emitter {
            let event_data = serde_json::to_value(&event).map_err(|e| format!("Failed to serialize event: {}", e))?;

            emitter
                .emit_snapshot("global", event_data)
                .await
                .map_err(|e| format!("Failed to emit event: {}", e))?;

            debug!("Emitted snapshot event: event_type={:?}", event);
        } else {
            debug!("EventEmitter not configured, skipping event emission");
        }
        Ok(())
    }

    async fn emit_to_session(&self, session_id: &str, event: SnapshotEvent) -> Result<(), String> {
        if let Some(ref emitter) = self.emitter {
            let event_data = serde_json::to_value(&event).map_err(|e| format!("Failed to serialize event: {}", e))?;

            emitter
                .emit_snapshot(session_id, event_data.clone())
                .await
                .map_err(|e| format!("Failed to emit event: {}", e))?;

            let session_event_name = format!("snapshot-event:{}", session_id);
            emitter
                .emit(&session_event_name, event_data)
                .await
                .map_err(|e| format!("Failed to emit session event: {}", e))?;

            debug!(
                "Emitted session event: session_id={} event_type={:?}",
                session_id, event
            );
        } else {
            debug!("EventEmitter not configured, skipping event emission");
        }
        Ok(())
    }
}

/// Global event emitter (race-free via `OnceLock`).
static GLOBAL_EVENT_EMITTER: OnceLock<Arc<tokio::sync::RwLock<SnapshotEmitterAdapter>>> = OnceLock::new();

/// Initializes the global event emitter.
///
/// The first call wins. Subsequent calls are ignored — this prevents a torn
/// `Arc` write if two threads race during app initialization. Callers needing
/// re-initialization should clear the runtime state and restart.
pub fn initialize_snapshot_event_emitter(emitter: Arc<dyn EventEmitter>) {
    let arc = Arc::new(tokio::sync::RwLock::new(SnapshotEmitterAdapter::new(Some(emitter))));
    if GLOBAL_EVENT_EMITTER.set(arc).is_err() {
        debug!("Snapshot global event emitter was already initialized; ignoring re-init");
        return;
    }
    info!("Snapshot global event emitter initialized");
}

/// Gets the global event emitter.
pub fn event_emitter() -> Option<Arc<tokio::sync::RwLock<SnapshotEmitterAdapter>>> {
    GLOBAL_EVENT_EMITTER.get().cloned()
}

/// Helper: emits an event.
pub async fn emit_snapshot_event(event: SnapshotEvent) {
    if let Some(emitter) = event_emitter() {
        let e = emitter.read().await;
        let _ = e.emit(event).await;
    }
}

/// Helper: emits a session-scoped event.
pub async fn emit_snapshot_session_event(session_id: &str, event: SnapshotEvent) {
    if let Some(emitter) = event_emitter() {
        let e = emitter.read().await;
        let _ = e.emit_to_session(session_id, event).await;
    }
}
