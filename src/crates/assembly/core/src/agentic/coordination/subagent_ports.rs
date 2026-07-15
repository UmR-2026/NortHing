//! `AgentSubmissionPort` trait implementation for `ConversationCoordinator`.

use super::coordinator::ConversationCoordinator;
use crate::agentic::core::SessionConfig;
use northhing_runtime_ports::AgentSubmissionPort;

pub fn resolve_agent_submission_turn_id(request: &northhing_runtime_ports::AgentSubmissionRequest) -> String {
    request
        .turn_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            request
                .metadata
                .get("turnId")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

pub fn resolve_agent_session_create_created_by(
    metadata: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    metadata
        .get("created_by")
        .or_else(|| metadata.get("createdBy"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn runtime_session_time_ms(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub fn runtime_session_summary(
    session: crate::agentic::core::SessionSummary,
) -> northhing_runtime_ports::AgentSessionSummary {
    northhing_runtime_ports::AgentSessionSummary {
        session_id: session.session_id,
        session_name: session.session_name,
        agent_type: session.agent_type,
        created_at_ms: runtime_session_time_ms(session.created_at),
        last_active_at_ms: runtime_session_time_ms(session.last_activity_at),
    }
}

#[async_trait::async_trait]
impl AgentSubmissionPort for ConversationCoordinator {
    async fn create_session(
        &self,
        request: northhing_runtime_ports::AgentSessionCreateRequest,
    ) -> northhing_runtime_ports::PortResult<northhing_runtime_ports::AgentSessionCreateResult> {
        let workspace_path = request.workspace_path.clone().ok_or_else(|| {
            northhing_runtime_ports::PortError::new(
                northhing_runtime_ports::PortErrorKind::InvalidRequest,
                "workspace_path is required to create an agent session",
            )
        })?;

        let session = self
            .create_session_with_workspace_and_creator(
                None,
                request.session_name,
                request.agent_type,
                SessionConfig {
                    workspace_path: Some(workspace_path.clone()),
                    ..Default::default()
                },
                workspace_path,
                resolve_agent_session_create_created_by(&request.metadata),
            )
            .await
            .map_err(|error| {
                northhing_runtime_ports::PortError::new(
                    northhing_runtime_ports::PortErrorKind::Backend,
                    error.to_string(),
                )
            })?;

        Ok(northhing_runtime_ports::AgentSessionCreateResult {
            session_id: session.session_id,
            session_name: session.session_name,
            agent_type: session.agent_type,
        })
    }

    async fn submit_message(
        &self,
        request: northhing_runtime_ports::AgentSubmissionRequest,
    ) -> northhing_runtime_ports::PortResult<northhing_runtime_ports::AgentSubmissionResult> {
        if !request.attachments.is_empty() {
            return Err(northhing_runtime_ports::PortError::new(
                northhing_runtime_ports::PortErrorKind::InvalidRequest,
                "agent submission port does not yet accept generic attachments",
            ));
        }

        let session = self.session_manager().get_session(&request.session_id).ok_or_else(|| {
            northhing_runtime_ports::PortError::new(
                northhing_runtime_ports::PortErrorKind::NotFound,
                format!("session not found: {}", request.session_id),
            )
        })?;

        let turn_id = resolve_agent_submission_turn_id(&request);

        let trigger_source = request.source.unwrap_or(super::port_types::DialogTriggerSource::Bot);
        let user_message_metadata = if request.metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(request.metadata.clone()))
        };

        self.start_dialog_turn(
            request.session_id,
            request.message.clone(),
            Some(request.message),
            Some(turn_id.clone()),
            session.agent_type.clone(),
            session.config.workspace_path.clone(),
            crate::agentic::coordination::scheduler::DialogSubmissionPolicy::for_source(trigger_source),
            user_message_metadata,
        )
        .await
        .map_err(|error| {
            northhing_runtime_ports::PortError::new(northhing_runtime_ports::PortErrorKind::Backend, error.to_string())
        })?;

        Ok(northhing_runtime_ports::AgentSubmissionResult {
            turn_id,
            accepted: true,
        })
    }

    async fn resolve_session_agent_type(
        &self,
        session_id: &str,
    ) -> northhing_runtime_ports::PortResult<Option<String>> {
        Ok(self
            .session_manager()
            .get_session(session_id)
            .map(|session| session.agent_type.clone()))
    }
}
