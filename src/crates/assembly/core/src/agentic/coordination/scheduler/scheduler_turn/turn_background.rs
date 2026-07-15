use super::super::scheduler_types::DialogScheduler;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use northhing_agent_runtime::scheduler::{
    resolve_background_delivery_action, resolve_background_delivery_injection, BackgroundDeliveryAction,
    BackgroundDeliveryFacts, BackgroundInjectionKind,
};
use northhing_runtime_ports::{
    AgentTurnCancellationPort, AgentTurnCancellationRequest, AgentTurnCancellationResult, PortError, PortErrorKind,
    PortResult,
};

impl DialogScheduler {
    /// Deliver a completed background result back to the parent session.
    /// If the session is currently processing, inject the result into the
    /// running turn at the next model-round boundary. Otherwise, start a new
    /// turn immediately so the result is handled without waiting for an
    /// unrelated future message.
    pub async fn deliver_background_result(
        &self,
        session_id: String,
        agent_type: String,
        workspace_path: Option<String>,
        content: String,
        display_content: Option<String>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let display = display_content.unwrap_or_else(|| content.clone());
        let state = self.session_manager.get_session(&session_id).map(|s| s.state.clone());

        match resolve_background_delivery_action(BackgroundDeliveryFacts {
            session_state: Self::session_state_fact(state.as_ref()),
        }) {
            BackgroundDeliveryAction::InjectIntoRunningTurn => {
                self.round_injection_buffer.push(
                    &session_id,
                    resolve_background_delivery_injection(
                        BackgroundInjectionKind::BackgroundResult,
                        Uuid::new_v4().to_string(),
                        content,
                        Some(display),
                        SystemTime::now(),
                    ),
                );
                Ok(())
            }
            BackgroundDeliveryAction::SubmitAgentSessionFollowUp {
                queue_priority,
                skip_tool_confirmation,
            } => self
                .submit(
                    session_id,
                    content,
                    Some(display),
                    None,
                    agent_type,
                    workspace_path,
                    super::super::scheduler_types::DialogSubmissionPolicy::new(
                        super::super::super::coordinator::DialogTriggerSource::AgentSession,
                        queue_priority,
                        skip_tool_confirmation,
                    ),
                    None,
                    user_message_metadata,
                    None,
                )
                .await
                .map(|_| ()),
        }
    }
}

#[async_trait::async_trait]
impl AgentTurnCancellationPort for DialogScheduler {
    async fn cancel_turn(&self, request: AgentTurnCancellationRequest) -> PortResult<AgentTurnCancellationResult> {
        let session_id = request.session_id;
        let wait_timeout = Duration::from_millis(request.wait_timeout_ms.unwrap_or(1500));

        let cancelled_turn_id = if let Some(turn_id) = request.turn_id {
            self.coordinator
                .cancel_dialog_turn(&session_id, &turn_id)
                .await
                .map_err(|error| PortError::new(PortErrorKind::Backend, error.to_string()))?;
            Some(turn_id)
        } else if let Some(requester_session_id) = request.requester_session_id {
            self.cancel_active_turn_for_session_from_requester(&session_id, &requester_session_id, wait_timeout)
                .await
                .map_err(|error| PortError::new(PortErrorKind::Backend, error.to_string()))?
        } else {
            self.coordinator
                .cancel_active_turn_for_session(&session_id, wait_timeout)
                .await
                .map_err(|error| PortError::new(PortErrorKind::Backend, error.to_string()))?
        };

        Ok(AgentTurnCancellationResult {
            session_id,
            requested: cancelled_turn_id.is_some(),
            turn_id: cancelled_turn_id,
        })
    }
}
