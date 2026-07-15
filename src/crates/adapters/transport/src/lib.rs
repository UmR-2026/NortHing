#![allow(clippy::too_many_arguments)]

pub mod adapters;
pub mod emitter;
pub mod event_bus;
pub mod events;
/// northhing Transport Layer
///
/// Cross-platform communication abstraction layer, supports:
/// - CLI (tokio mpsc)
/// - Tauri (app.emit)
/// - WebSocket/SSE (web server)
pub mod traits;

pub use adapters::{CliEvent, CliTransportAdapter, WebSocketTransportAdapter};
pub use emitter::TransportEmitter;
pub use event_bus::{EventBus, EventPriority};
pub use events::{
    AgenticEventPayload, BackendEventPayload, FileWatchEventPayload, LspEventPayload, ProfileEventPayload,
    SnapshotEventPayload, UnifiedEvent,
};
pub use traits::{StreamEvent, TextChunk, ToolEventPayload, ToolEventType, TransportAdapter};

#[cfg(feature = "tauri-adapter")]
pub use adapters::TauriTransportAdapter;

#[cfg(feature = "slint-adapter")]
pub use adapters::{SlintEvent, SlintTransportAdapter};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
