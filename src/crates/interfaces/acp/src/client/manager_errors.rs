// R19 split: ACP error mapping, startup-timeout detection, permission option selection.
// File: src/crates/interfaces/acp/src/client/manager_errors.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
//             manager_config.rs
//             manager_install.rs
//             manager_connection.rs
//             manager_transport.rs
//             manager_session.rs
//             manager_prompt.rs
//             manager_cancel.rs
//             manager_permission.rs
//             manager_process.rs
//             manager_process_lifecycle.rs
//             manager_session_helpers.rs
//
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::CLIENT_STARTUP_TIMEOUT_SECS;
use agent_client_protocol::schema::{
    AgentCapabilities, CancelNotification, ClientCapabilities, CloseSessionRequest, Implementation, InitializeRequest,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, ResumeSessionRequest, ResumeSessionResponse, SelectedPermissionOutcome,
    SessionConfigOption, SessionConfigOptionValue, SessionModelState, SetSessionConfigOptionRequest,
    SetSessionModelRequest, StopReason,
};
use northhing_core::util::errors::{NortHingError, NortHingResult};

pub fn protocol_error(error: impl std::fmt::Display) -> NortHingError {
    NortHingError::service(format!("ACP protocol error: {}", error))
}

const STARTUP_TIMEOUT_ERROR_PREFIX: &str = "ACP startup timed out:";

pub fn startup_timeout_error(client_id: &str, phase: &str) -> NortHingError {
    NortHingError::service(startup_timeout_error_message(client_id, phase))
}

pub fn startup_timeout_error_message(client_id: &str, phase: &str) -> String {
    format!(
        "{} client '{}' exceeded {}s during {} and was terminated. Please try again after the client is ready.",
        STARTUP_TIMEOUT_ERROR_PREFIX, client_id, CLIENT_STARTUP_TIMEOUT_SECS, phase
    )
}

pub fn is_startup_timeout_error(error: &NortHingError) -> bool {
    error.to_string().contains(STARTUP_TIMEOUT_ERROR_PREFIX)
}

pub fn select_permission_by_kind(
    request: &RequestPermissionRequest,
    preferred: PermissionOptionKind,
    approve: bool,
) -> RequestPermissionResponse {
    let fallback_kind = if approve {
        PermissionOptionKind::AllowAlways
    } else {
        PermissionOptionKind::RejectAlways
    };
    let option_id = request
        .options
        .iter()
        .find(|option| option.kind == preferred)
        .or_else(|| request.options.iter().find(|option| option.kind == fallback_kind))
        .map(|option| option.option_id.to_string())
        .unwrap_or_else(|| select_permission_option_id(&request.options, approve));
    RequestPermissionResponse::new(RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
        option_id,
    )))
}

pub fn select_permission_option_id(options: &[PermissionOption], approve: bool) -> String {
    let preferred_kinds = if approve {
        [PermissionOptionKind::AllowOnce, PermissionOptionKind::AllowAlways]
    } else {
        [PermissionOptionKind::RejectOnce, PermissionOptionKind::RejectAlways]
    };

    options
        .iter()
        .find(|option| preferred_kinds.contains(&option.kind))
        .or_else(|| options.first())
        .map(|option| option.option_id.to_string())
        .unwrap_or_else(|| {
            if approve {
                "allow_once".to_string()
            } else {
                "reject_once".to_string()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_actual_permission_option_id_for_approval() {
        let options = vec![
            PermissionOption::new("deny", "Deny", PermissionOptionKind::RejectOnce),
            PermissionOption::new("yes-once", "Allow", PermissionOptionKind::AllowOnce),
        ];

        assert_eq!(select_permission_option_id(&options, true), "yes-once");
    }

    #[test]
    fn selects_actual_permission_option_id_for_rejection() {
        let options = vec![
            PermissionOption::new("allow-always", "Allow", PermissionOptionKind::AllowAlways),
            PermissionOption::new("no-once", "Reject", PermissionOptionKind::RejectOnce),
        ];

        assert_eq!(select_permission_option_id(&options, false), "no-once");
    }

    #[test]
    fn formats_startup_timeout_error_message() {
        assert_eq!(
            startup_timeout_error_message("codex", "initialize"),
            "ACP startup timed out: client 'codex' exceeded 60s during initialize and was terminated. Please try again after the client is ready."
        );
    }
}
