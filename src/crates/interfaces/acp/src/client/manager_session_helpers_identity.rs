// R20b split: ACP session identity / key / status helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_identity.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation
//         +67% over QClaw 242 tolerance)
// Sub-domain A: identity primitives (4 free fns: parse_config_value,
//                build_session_key, session_client_connection_id,
//                aggregate_client_status).
// R20b sibling files:
//             manager_session_helpers_session_response.rs (sub-domain B + C)
//             manager_session_helpers_session_state.rs (sub-domain D)
// R19 sibling files (consumers of sub-domain A fns):
//             manager.rs
//             manager_config.rs
//             manager_cancel.rs
//             manager_connection.rs
//             manager_session.rs
// All method bodies are moved verbatim from main. No behavior change.

use super::config::{AcpClientConfigFile, AcpClientStatus};
use northhing_core::util::errors::{NortHingError, NortHingResult};
use serde_json::json;
use std::path::Path;

pub fn parse_config_value(value: serde_json::Value) -> NortHingResult<AcpClientConfigFile> {
    if value.get("acpClients").is_some() {
        serde_json::from_value(value)
            .map_err(|error| NortHingError::config(format!("Invalid ACP client config: {}", error)))
    } else if value.is_object() {
        serde_json::from_value(json!({ "acpClients": value }))
            .map_err(|error| NortHingError::config(format!("Invalid ACP client config map: {}", error)))
    } else {
        Err(NortHingError::config("ACP client config must be an object".to_string()))
    }
}

pub fn build_session_key(northhing_session_id: &str, client_id: &str, cwd: &Path) -> String {
    format!("{}:{}:{}", northhing_session_id, client_id, cwd.to_string_lossy())
}

pub fn session_client_connection_id(client_id: &str, northhing_session_id: &str) -> String {
    format!("{}::session::{}", client_id, northhing_session_id)
}

pub fn aggregate_client_status(statuses: &[AcpClientStatus]) -> AcpClientStatus {
    if statuses.is_empty() {
        return AcpClientStatus::Configured;
    }
    if statuses.iter().any(|status| matches!(status, AcpClientStatus::Running)) {
        return AcpClientStatus::Running;
    }
    if statuses
        .iter()
        .any(|status| matches!(status, AcpClientStatus::Starting))
    {
        return AcpClientStatus::Starting;
    }
    if statuses.iter().any(|status| matches!(status, AcpClientStatus::Failed)) {
        return AcpClientStatus::Failed;
    }
    AcpClientStatus::Stopped
}
