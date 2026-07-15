//! Remote-control runtime port for `ConversationCoordinator`.

use super::coordinator::ConversationCoordinator;
use crate::agentic::core::SessionState;
use northhing_runtime_ports::RemoteControlStatePort;
use serde_json::Value;

#[async_trait::async_trait]
impl RemoteControlStatePort for ConversationCoordinator {
    async fn read_remote_control_state(
        &self,
        request: northhing_runtime_ports::RemoteControlStateRequest,
    ) -> northhing_runtime_ports::PortResult<Option<northhing_runtime_ports::RemoteControlStateSnapshot>> {
        let Some(session) = self.session_manager().get_session(&request.session_id) else {
            return Ok(None);
        };

        let mut metadata = serde_json::Map::new();
        let (state, active_turn_id) = match session.state {
            SessionState::Idle => (northhing_runtime_ports::RemoteControlSessionState::Idle, None),
            SessionState::Processing { current_turn_id, phase } => {
                metadata.insert("phase".to_string(), Value::String(format!("{:?}", phase)));
                (
                    northhing_runtime_ports::RemoteControlSessionState::Processing,
                    Some(current_turn_id),
                )
            }
            SessionState::Error { error, recoverable } => {
                metadata.insert("error".to_string(), Value::String(error));
                metadata.insert("recoverable".to_string(), Value::Bool(recoverable));
                (northhing_runtime_ports::RemoteControlSessionState::Error, None)
            }
        };

        Ok(Some(northhing_runtime_ports::RemoteControlStateSnapshot {
            session_id: request.session_id,
            state,
            active_turn_id,
            queue_depth: 0,
            metadata,
        }))
    }
}
