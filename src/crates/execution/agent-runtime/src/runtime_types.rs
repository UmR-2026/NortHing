//! Public request/handle/selector DTOs for the [`crate::runtime::AgentRuntime`]
//! facade.
//!
//! Split from `runtime.rs` (R39e). The DTOs stay near each other so cross-crate
//! callers that import them via `northhing_agent_runtime::runtime::*` continue
//! to find the same set of names re-exported by the facade.

use northhing_runtime_ports::{AgentInputAttachment, AgentSubmissionSource};

use super::runtime_event_stream::AgentEventStream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSelector {
    Existing {
        session_id: String,
    },
    Create {
        session_name: String,
        agent_type: String,
        workspace_path: Option<String>,
        metadata: serde_json::Map<String, serde_json::Value>,
    },
}

impl SessionSelector {
    pub fn existing(session_id: impl Into<String>) -> Self {
        Self::Existing {
            session_id: session_id.into(),
        }
    }

    pub fn create(
        session_name: impl Into<String>,
        agent_type: impl Into<String>,
        workspace_path: Option<String>,
    ) -> Self {
        Self::Create {
            session_name: session_name.into(),
            agent_type: agent_type.into(),
            workspace_path,
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Map<String, serde_json::Value>) -> Self {
        if let Self::Create { metadata: existing, .. } = &mut self {
            *existing = metadata;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunRequest {
    pub session: SessionSelector,
    pub message: String,
    pub turn_id: Option<String>,
    pub source: Option<AgentSubmissionSource>,
    pub attachments: Vec<AgentInputAttachment>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl AgentRunRequest {
    pub fn new(session: SessionSelector, message: impl Into<String>) -> Self {
        Self {
            session,
            message: message.into(),
            turn_id: None,
            source: None,
            attachments: Vec::new(),
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_source(mut self, source: AgentSubmissionSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_attachments(mut self, attachments: Vec<AgentInputAttachment>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Map<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone)]
pub struct AgentRunHandle {
    pub session_id: String,
    pub turn_id: String,
    pub agent_type: Option<String>,
    pub accepted: bool,
    pub events: Option<AgentEventStream>,
}
