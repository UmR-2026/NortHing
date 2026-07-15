//! Sub-domain: workspace.
//! Spec §2.1 — extracted from dialog_turn.rs (Round 6 refactor).
//! Contains private/pub(crate) helper methods; public API stays in the facade mod.rs.
//!
//! Sibling imports `use super::super::coordinator::*` for the struct definition.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::turn_outcome::TurnOutcome;

use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use crate::agentic::agents::agent_registry;
use crate::agentic::context_profile::ContextProfilePolicy;
use crate::agentic::core::{
    InternalReminderKind, Message, MessageContent, ProcessingPhase, Session, SessionConfig, SessionKind, SessionState,
    SessionSummary, TurnStats,
};
use crate::agentic::events::{
    AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
};
use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
use crate::agentic::fork_agent::ForkAgentContextSnapshot;
use crate::agentic::goal_mode::{
    effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
    should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
    user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::remote_file_delivery::{
    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
};
use crate::agentic::round_preempt::DialogRoundInjectionSource;
use crate::agentic::session::SessionManager;
use crate::agentic::side_question::build_btw_user_input;
use crate::agentic::skill_agent_snapshot::{
    diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
};
use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
use crate::agentic::tools::{
    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
};
use crate::agentic::workspace::WorkspaceServices;
use crate::agentic::WorkspaceBinding;
use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
use crate::service::config::global::GlobalConfigManager;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::service::workspace::{global_workspace_service, WorkspaceCreateOptions, WorkspaceKind};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use dashmap::DashMap;
use northhing_runtime_ports::{
    AgentBackgroundResultRequest, AgentThreadGoalDeliveryKind, AgentThreadGoalDeliveryRequest, DelegationPolicy,
    SubagentContextMode, ThreadGoal, ThreadGoalContinuationPlan, ThreadGoalStatus,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

use northhing_agent_dispatch::{ActorRuntime, USE_LIGHTWEIGHT_ACTOR};
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

const MANUAL_COMPACTION_COMMAND: &str = "/compact";
const CONTEXT_COMPRESSION_TOOL_NAME: &str = "ContextCompression";
const DEFAULT_SUBAGENT_MAX_CONCURRENCY: usize = 5;
const MAX_SUBAGENT_MAX_CONCURRENCY: usize = 64;

impl ConversationCoordinator {
    pub(super) async fn resolve_workspace_id_for_config(config: &SessionConfig) -> Option<String> {
        let explicit = config
            .workspace_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if explicit.is_some() {
            return explicit;
        }

        let workspace_path = config.workspace_path.as_deref()?;
        let workspace_service = global_workspace_service()?;

        if config.remote_connection_id.is_some() || config.remote_ssh_host.is_some() {
            let normalized_path = normalize_remote_workspace_path(workspace_path);
            let desired_connection_id = config
                .remote_connection_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let desired_ssh_host = config
                .remote_ssh_host
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());

            return workspace_service
                .list_workspace_infos()
                .await
                .into_iter()
                .find(|workspace| {
                    if workspace.workspace_kind != WorkspaceKind::Remote {
                        return false;
                    }
                    if normalize_remote_workspace_path(&workspace.root_path.to_string_lossy()) != normalized_path {
                        return false;
                    }
                    if let Some(connection_id) = desired_connection_id {
                        if workspace.remote_ssh_connection_id() != Some(connection_id) {
                            return false;
                        }
                    }
                    if let Some(ssh_host) = desired_ssh_host {
                        let workspace_ssh_host = workspace
                            .metadata
                            .get("sshHost")
                            .and_then(|value| value.as_str())
                            .map(str::trim)
                            .filter(|value| !value.is_empty());
                        if workspace_ssh_host != Some(ssh_host) {
                            return false;
                        }
                    }
                    true
                })
                .map(|workspace| workspace.id);
        }

        workspace_service
            .get_workspace_by_path(Path::new(workspace_path))
            .await
            .map(|workspace| workspace.id)
    }

    pub(super) async fn track_session_workspace_activity_best_effort(config: &SessionConfig, reason: &str) {
        let Some(workspace_path) = config.workspace_path.as_ref() else {
            return;
        };

        let Some(workspace_service) = global_workspace_service() else {
            return;
        };

        let mut options = WorkspaceCreateOptions {
            auto_set_current: false,
            add_to_recent: true,
            ..Default::default()
        };

        if config.remote_connection_id.is_some() {
            options.workspace_kind = WorkspaceKind::Remote;
            options.remote_connection_id = config.remote_connection_id.clone();
            options.remote_ssh_host = config.remote_ssh_host.clone();
        }

        if let Err(error) = workspace_service
            .track_workspace_activity(PathBuf::from(workspace_path), options)
            .await
        {
            warn!(
                "Failed to track session workspace activity: reason={}, workspace_path={}, error={}",
                reason, workspace_path, error
            );
        }
    }

    /// Build a workspace binding that is remote-aware.
    /// If the global remote workspace is active and matches the session path,
    /// returns a `WorkspaceBinding` with remote metadata and correct local
    /// session storage path.
    ///
    /// When the session's `remote_connection_id` does not match any active
    /// SSH connection (e.g. the user changed the port and the old ID is now
    /// stale), this method attempts to remap to the current workspace
    /// registration so that historical sessions continue to work.

    pub(crate) async fn build_workspace_binding(config: &SessionConfig) -> Option<WorkspaceBinding> {
        let workspace_path = config.workspace_path.as_ref()?;
        let path_buf = PathBuf::from(workspace_path);
        let workspace_id = Self::resolve_workspace_id_for_config(config).await;

        let identity = crate::service::remote_ssh::workspace_state::resolve_workspace_session_identity(
            workspace_path,
            config.remote_connection_id.as_deref(),
            config.remote_ssh_host.as_deref(),
        )
        .await?;

        if let Some(rid) = identity.remote_connection_id.as_deref() {
            // Try to look up the connection by the session's stored ID first.
            let lookup = crate::service::remote_ssh::workspace_state::lookup_remote_connection_with_hint(
                workspace_path,
                Some(rid),
            )
            .await;

            // If the stored connection_id does not resolve to a registered
            // workspace, attempt a path-only lookup.  This covers the case
            // where the user changed the SSH port: the old connection_id is
            // no longer registered, but the same remote path is now bound to
            // a new connection with the updated port.
            let (effective_rid, entry) = if lookup.is_some() {
                (rid.to_string(), lookup)
            } else {
                let path_entry =
                    crate::service::remote_ssh::workspace_state::lookup_remote_connection(workspace_path).await;
                if let Some(ref pe) = path_entry {
                    tracing::info!(
                        "Session connection_id {} not registered for workspace {}; remapping to {}",
                        rid,
                        workspace_path,
                        pe.connection_id
                    );
                    (pe.connection_id.clone(), path_entry)
                } else {
                    (rid.to_string(), lookup)
                }
            };

            let connection_name = entry
                .map(|e| e.connection_name)
                .unwrap_or_else(|| effective_rid.clone());

            // Re-resolve identity with the effective connection_id so the
            // session storage path is correct.
            let effective_identity = crate::service::remote_ssh::workspace_state::resolve_workspace_session_identity(
                workspace_path,
                Some(&effective_rid),
                config.remote_ssh_host.as_deref(),
            )
            .await
            .unwrap_or(identity);

            let binding = WorkspaceBinding::new_remote(
                workspace_id.clone(),
                path_buf,
                effective_rid,
                connection_name,
                effective_identity,
            );

            return Some(binding);
        }

        let binding = WorkspaceBinding::new(workspace_id, path_buf);

        Some(binding)
    }

    pub(crate) async fn build_session_config_for_workspace(
        workspace_path: String,
        model_id: Option<String>,
    ) -> SessionConfig {
        let remote_entry = crate::service::remote_ssh::workspace_state::lookup_remote_connection(&workspace_path).await;

        let mut config = SessionConfig {
            workspace_path: Some(workspace_path),
            model_id,
            ..SessionConfig::default()
        };

        if let Some(entry) = remote_entry {
            config.remote_connection_id = Some(entry.connection_id);
            if !entry.ssh_host.trim().is_empty() {
                config.remote_ssh_host = Some(entry.ssh_host);
            }
        }

        config
    }

    /// Build `WorkspaceServices` from a resolved `WorkspaceBinding`.
    /// For remote bindings, wires up SSH-backed FS/shell; for local ones,
    /// returns local implementations.

    pub(crate) async fn build_workspace_services(
        binding: &Option<WorkspaceBinding>,
    ) -> Option<crate::agentic::workspace::WorkspaceServices> {
        let binding = binding.as_ref()?;

        if binding.is_remote() {
            let manager = match crate::service::remote_ssh::workspace_state::remote_workspace_manager() {
                Some(m) => m,
                None => {
                    tracing::warn!("build_workspace_services: RemoteWorkspaceStateManager not initialized");
                    return None;
                }
            };
            let ssh_manager = match manager.get_ssh_manager().await {
                Some(m) => m,
                None => {
                    tracing::warn!("build_workspace_services: SSH manager not available in state manager");
                    return None;
                }
            };
            let file_service = match manager.get_file_service().await {
                Some(f) => f,
                None => {
                    tracing::warn!("build_workspace_services: File service not available in state manager");
                    return None;
                }
            };
            let connection_id = match binding.connection_id() {
                Some(id) => id.to_string(),
                None => {
                    tracing::warn!("build_workspace_services: No connection_id in workspace binding");
                    return None;
                }
            };
            tracing::info!(
                "build_workspace_services: Built remote services for connection_id={}",
                connection_id
            );
            Some(crate::agentic::workspace::remote_workspace_services(
                connection_id,
                file_service,
                ssh_manager,
                binding.root_path_string(),
            ))
        } else {
            Some(crate::agentic::workspace::local_workspace_services(
                binding.root_path_string(),
            ))
        }
    }

    pub(super) fn require_main_session_workspace(&self, session_id: &str) -> NortHingResult<PathBuf> {
        let session = self
            .session_manager
            .get_session(session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {session_id}")))?;
        if matches!(session.kind, SessionKind::Subagent | SessionKind::EphemeralChild) {
            return Err(NortHingError::Validation(
                "Thread goals are only available for main sessions".to_string(),
            ));
        }
        session
            .config
            .workspace_path
            .as_deref()
            .map(Path::new)
            .map(Path::to_path_buf)
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {session_id}")))
    }
}
