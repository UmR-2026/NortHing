mod builtin_clients;
mod config;
mod manager;
mod manager_cancel;
mod manager_config_loading;
mod manager_config_requirements;
mod manager_connection_start;
mod manager_connection_stop;
mod manager_errors;
mod manager_install;
mod manager_permission;
mod manager_process;
mod manager_process_lifecycle;
mod manager_prompt;
mod manager_session_helpers_identity;
mod manager_session_helpers_session_response;
mod manager_session_helpers_session_state;
mod manager_session_lifecycle;
mod manager_session_read;
mod manager_session_resolve;
mod manager_transport;
mod remote_capability_store;
mod remote_session;
mod remote_shell;
mod requirements;
mod session_options;
mod session_persistence;
mod stream;
mod tool;
mod tool_card_bridge;

pub use config::{
    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode, AcpClientRequirementProbe,
    AcpClientStatus, AcpRequirementProbeItem, RemoteAcpClientRequirementSnapshot,
};
pub use manager::{
    AcpClientPermissionResponse, AcpClientService, CreateAcpFlowSessionRecordResponse, SetAcpSessionModelRequest,
    SubmitAcpPermissionResponseRequest,
};
pub use session_options::{
    AcpAvailableCommand, AcpPlanEntry, AcpSessionContextUsage, AcpSessionModelOption, AcpSessionOptions,
};
pub use stream::AcpClientStreamEvent;
