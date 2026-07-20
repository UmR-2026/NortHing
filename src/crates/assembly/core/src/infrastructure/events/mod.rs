//! Event system module

pub mod emitter;
pub mod event_system;

pub use emitter::EventEmitter;
pub use event_system::BackendEventSystem as BackendEventManager;
pub use event_system::{emit_global_event, global_event_system, BackendEvent, BackendEventSystem};
