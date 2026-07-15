//! Public Agent Runtime facade over stable runtime ports.
//!
//! This module is intentionally port-backed. It gives product assembly and
//! future SDK consumers a narrow agent entrypoint without depending on
//! `northhing-core`, app crates, Tauri, or concrete service managers.
//!
//! R39e split: the implementation is organised into sibling files
//! (`runtime_error`, `runtime_event_stream`, `runtime_builder`, `runtime_types`,
//! `tests`) so each file stays under the 800-line god-split budget. The
//! facade keeps the [`AgentRuntime`] struct + impl block and re-exports the
//! sibling types so cross-crate callers continue to use the
//! `northhing_agent_runtime::runtime::*` path unchanged.

use std::sync::Arc;

use northhing_runtime_ports::{
    AgentBackgroundResultRequest, AgentDialogTurnPort, AgentDialogTurnRequest, AgentLifecycleDeliveryPort,
    AgentSessionCreateRequest, AgentSessionCreateResult, AgentSessionDeleteRequest, AgentSessionListRequest,
    AgentSessionManagementPort, AgentSessionSummary, AgentSessionWorkspaceRequest, AgentSubmissionPort,
    AgentSubmissionRequest, AgentSubmissionResult, AgentThreadGoalDeliveryRequest, AgentTurnCancellationPort,
    AgentTurnCancellationRequest, AgentTurnCancellationResult, DialogSubmitOutcome, RuntimeEventEnvelope,
};
use northhing_runtime_services::RuntimeServices;

#[path = "runtime_builder.rs"]
mod runtime_builder;
#[path = "runtime_error.rs"]
mod runtime_error;
#[path = "runtime_event_stream.rs"]
mod runtime_event_stream;
#[path = "runtime_types.rs"]
mod runtime_types;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use runtime_builder::AgentRuntimeBuilder;
pub use runtime_error::{RuntimeBuildError, RuntimeError};
pub use runtime_event_stream::AgentEventStream;
pub use runtime_types::{AgentRunHandle, AgentRunRequest, SessionSelector};

/// Constructor exposed to the sibling builder so [`AgentRuntime`]'s private
/// field layout stays encapsulated in this facade module.
pub(super) fn build_agent_runtime(
    submission: Arc<dyn AgentSubmissionPort>,
    session_management: Option<Arc<dyn AgentSessionManagementPort>>,
    dialog_turn: Option<Arc<dyn AgentDialogTurnPort>>,
    lifecycle_delivery: Option<Arc<dyn AgentLifecycleDeliveryPort>>,
    cancellation: Option<Arc<dyn AgentTurnCancellationPort>>,
    services: Option<RuntimeServices>,
    event_stream: Option<AgentEventStream>,
) -> AgentRuntime {
    AgentRuntime {
        submission,
        session_management,
        dialog_turn,
        lifecycle_delivery,
        cancellation,
        services,
        event_stream,
    }
}

#[derive(Clone)]
pub struct AgentRuntime {
    submission: Arc<dyn AgentSubmissionPort>,
    session_management: Option<Arc<dyn AgentSessionManagementPort>>,
    dialog_turn: Option<Arc<dyn AgentDialogTurnPort>>,
    lifecycle_delivery: Option<Arc<dyn AgentLifecycleDeliveryPort>>,
    cancellation: Option<Arc<dyn AgentTurnCancellationPort>>,
    services: Option<RuntimeServices>,
    event_stream: Option<AgentEventStream>,
}

impl std::fmt::Debug for AgentRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntime")
            .field("submission", &"<dyn AgentSubmissionPort>")
            .field(
                "session_management",
                &self
                    .session_management
                    .as_ref()
                    .map(|_| "<dyn AgentSessionManagementPort>"),
            )
            .field(
                "dialog_turn",
                &self.dialog_turn.as_ref().map(|_| "<dyn AgentDialogTurnPort>"),
            )
            .field(
                "lifecycle_delivery",
                &self
                    .lifecycle_delivery
                    .as_ref()
                    .map(|_| "<dyn AgentLifecycleDeliveryPort>"),
            )
            .field(
                "cancellation",
                &self.cancellation.as_ref().map(|_| "<dyn AgentTurnCancellationPort>"),
            )
            .field("services", &self.services.as_ref().map(|_| "<RuntimeServices>"))
            .field(
                "event_stream",
                &self.event_stream.as_ref().map(|_| "<AgentEventStream>"),
            )
            .finish()
    }
}

impl AgentRuntime {
    pub async fn create_session(
        &self,
        request: AgentSessionCreateRequest,
    ) -> Result<AgentSessionCreateResult, RuntimeError> {
        self.submission
            .create_session(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn list_sessions(
        &self,
        request: AgentSessionListRequest,
    ) -> Result<Vec<AgentSessionSummary>, RuntimeError> {
        let session_management = self
            .session_management
            .as_ref()
            .ok_or(RuntimeError::MissingSessionManagementPort)?;
        session_management
            .list_sessions(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn delete_session(&self, request: AgentSessionDeleteRequest) -> Result<(), RuntimeError> {
        let session_management = self
            .session_management
            .as_ref()
            .ok_or(RuntimeError::MissingSessionManagementPort)?;
        session_management
            .delete_session(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn resolve_session_workspace_path(
        &self,
        request: AgentSessionWorkspaceRequest,
    ) -> Result<Option<String>, RuntimeError> {
        let session_management = self
            .session_management
            .as_ref()
            .ok_or(RuntimeError::MissingSessionManagementPort)?;
        session_management
            .resolve_session_workspace_path(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn submit_turn(&self, request: AgentSubmissionRequest) -> Result<AgentSubmissionResult, RuntimeError> {
        self.submission
            .submit_message(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn submit_dialog_turn(
        &self,
        request: AgentDialogTurnRequest,
    ) -> Result<DialogSubmitOutcome, RuntimeError> {
        let dialog_turn = self.dialog_turn.as_ref().ok_or(RuntimeError::MissingDialogTurnPort)?;
        dialog_turn
            .submit_dialog_turn(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn deliver_background_result(&self, request: AgentBackgroundResultRequest) -> Result<(), RuntimeError> {
        let lifecycle_delivery = self
            .lifecycle_delivery
            .as_ref()
            .ok_or(RuntimeError::MissingLifecycleDeliveryPort)?;
        lifecycle_delivery
            .deliver_background_result(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn deliver_thread_goal(&self, request: AgentThreadGoalDeliveryRequest) -> Result<(), RuntimeError> {
        let lifecycle_delivery = self
            .lifecycle_delivery
            .as_ref()
            .ok_or(RuntimeError::MissingLifecycleDeliveryPort)?;
        lifecycle_delivery
            .deliver_thread_goal(request)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn resolve_session_agent_type(&self, session_id: &str) -> Result<Option<String>, RuntimeError> {
        self.submission
            .resolve_session_agent_type(session_id)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn cancel_turn(
        &self,
        request: AgentTurnCancellationRequest,
    ) -> Result<AgentTurnCancellationResult, RuntimeError> {
        let cancellation = self
            .cancellation
            .as_ref()
            .ok_or(RuntimeError::MissingCancellationPort)?;
        cancellation.cancel_turn(request).await.map_err(RuntimeError::from)
    }

    pub async fn publish_event(&self, event: RuntimeEventEnvelope) -> Result<(), RuntimeError> {
        if self.services.is_none() && self.event_stream.is_none() {
            return Err(RuntimeError::MissingEventSink);
        }

        if let Some(services) = self.services.as_ref() {
            services
                .events
                .publish_runtime_event(event.clone())
                .await
                .map_err(RuntimeError::from)?;
        }
        if let Some(events) = self.event_stream.as_ref() {
            events.push(event);
        }
        Ok(())
    }

    pub async fn run(&self, request: AgentRunRequest) -> Result<AgentRunHandle, RuntimeError> {
        let (session_id, agent_type) = match request.session {
            SessionSelector::Existing { session_id } => {
                let agent_type = self.resolve_session_agent_type(&session_id).await?;
                (session_id, agent_type)
            }
            SessionSelector::Create {
                session_name,
                agent_type,
                workspace_path,
                metadata,
            } => {
                let created = self
                    .create_session(AgentSessionCreateRequest {
                        session_name,
                        agent_type,
                        workspace_path,
                        metadata,
                    })
                    .await?;
                let agent_type = created.agent_type;
                (created.session_id, Some(agent_type))
            }
        };

        let submitted = self
            .submit_turn(AgentSubmissionRequest {
                session_id: session_id.clone(),
                message: request.message,
                turn_id: request.turn_id,
                source: request.source,
                attachments: request.attachments,
                metadata: request.metadata,
            })
            .await?;

        Ok(AgentRunHandle {
            session_id,
            turn_id: submitted.turn_id,
            agent_type,
            accepted: submitted.accepted,
            events: self.event_stream.clone(),
        })
    }
}
