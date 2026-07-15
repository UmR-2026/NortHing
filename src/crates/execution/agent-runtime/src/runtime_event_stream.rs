//! In-memory event stream used by the runtime facade.
//!
//! Split from `runtime.rs` (R39e). `push` is intentionally `pub(super)` so the
//! `AgentRuntime::publish_event` implementation in the facade module can
//! append events without exposing the mutation API to outside crates.

use std::sync::{Arc, Mutex};

use northhing_runtime_ports::RuntimeEventEnvelope;

#[derive(Clone, Default)]
pub struct AgentEventStream {
    events: Arc<Mutex<Vec<RuntimeEventEnvelope>>>,
}

impl std::fmt::Debug for AgentEventStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentEventStream").field("len", &self.len()).finish()
    }
}

impl AgentEventStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn snapshot(&self) -> Vec<RuntimeEventEnvelope> {
        self.events.lock().unwrap().clone()
    }

    pub fn drain(&self) -> Vec<RuntimeEventEnvelope> {
        self.events.lock().unwrap().drain(..).collect()
    }

    pub(super) fn push(&self, event: RuntimeEventEnvelope) {
        self.events.lock().unwrap().push(event);
    }
}
