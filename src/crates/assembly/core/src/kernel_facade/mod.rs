//! Kernel facade: pure passthrough implementation of the kernel-api traits.
//!
//! K1b1 — core passthrough impl for Bootstrap/Session/Turn/Events.
//! DTO conversions live here; the kernel-api crate is pure definition and is
//! never modified from this side.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use tracing::{info, warn};

use crate::agentic::coordination::{
    global_coordinator, global_scheduler, set_global_scheduler, DialogScheduler,
};
use crate::agentic::core::{Message, MessageContent, MessageRole, SessionConfig, SessionState};
use crate::agentic::system::init_agentic_system;
use crate::infrastructure::ai::AIClientFactory;
use crate::service::config::{get_global_config_service, initialize_global_config};
use crate::service::mcp::{set_global_mcp_service, MCPService};

// ── Re-exports for DTO types used in trait method signatures ────────────────

pub use northhing_kernel_api::events::{
    BackendEventDto, BannerLevel, KernelEventDto, SubscriptionId, ToolCallDto, ToolCallPhase,
};
pub use northhing_kernel_api::session::{
    BranchId, MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto,
    PersistenceHandleDto, SessionBranchDto, SessionConfigDto, SessionDto, SessionId,
    SessionKindDto, SessionMetadataDto, SessionRelationshipDto, SessionStateDto,
    SessionStatusDto, SessionSummaryDto, ToolCallStub,
};
pub use northhing_kernel_api::turn::{
    DialogSubmitOutcomeDto, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto,
    TurnStateDto, TurnStateKind,
};

// ── KernelFacade ────────────────────────────────────────────────────────────

static FACADE_READY: AtomicBool = AtomicBool::new(false);

pub struct KernelFacade {
    coordinator: Arc<crate::agentic::coordination::ConversationCoordinator>,
}

static FACADE: OnceLock<Arc<KernelFacade>> = OnceLock::new();

/// Returns the global `KernelFacade` instance. Panics if the coordinator has
/// not been initialized (i.e. `init_core` was never called).
pub fn kernel_facade() -> Arc<KernelFacade> {
    FACADE.get_or_init(|| Arc::new(KernelFacade::new())).clone()
}

impl KernelFacade {
    fn new() -> Self {
        let coordinator = global_coordinator()
            .expect("kernel_facade() called before init_core() — coordinator not available");
        Self { coordinator }
    }

    fn coordinator(&self) -> &Arc<crate::agentic::coordination::ConversationCoordinator> {
        &self.coordinator
    }

    /// Best-effort lookup of the session that owns a given turn. Scans the
    /// in-memory session list for a session whose `dialog_turn_ids` contains
    /// the target turn id.
    async fn find_session_for_turn(&self, turn_id: &str) -> Option<String> {
        // The coordinator does not expose a turn→session index, so we scan
        // the in-memory session store. This is O(n) over sessions but
        // acceptable for the passthrough facade.
        if let Ok(store) = self.coordinator().session_manager().list_sessions_safe().await {
            for session in store {
                if session.dialog_turn_ids.iter().any(|t| t == turn_id) {
                    return Some(session.session_id);
                }
            }
        }
        None
    }
}

// ── KernelBootstrapApi ──────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelBootstrapApi for KernelFacade {
    async fn init_core(&self) -> Result<(), KernelError> {
        initialize_global_config()
            .await
            .map_err(|e| KernelError::runtime(format!("initialize_global_config failed: {e}")))?;

        AIClientFactory::initialize_global()
            .await
            .map_err(|e| KernelError::runtime(format!("AIClientFactory init failed: {e}")))?;

        let system = init_agentic_system()
            .await
            .map_err(|e| KernelError::runtime(format!("init_agentic_system failed: {e}")))?;

        let coordinator = system.coordinator.clone();
        let session_manager = coordinator.session_manager().clone();
        let scheduler = DialogScheduler::new(coordinator.clone(), session_manager);

        let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
        let injection_ok =
            coordinator.set_round_injection_source(scheduler.round_injection_monitor());
        if !notifier_ok || !injection_ok {
            return Err(KernelError::runtime("dialog scheduler wiring conflict"));
        }

        set_global_scheduler(scheduler.clone());

        // Register a global MCPService and initialize in background (mirrors desktop main.rs).
        match get_global_config_service().await {
            Ok(cfg_svc) => match MCPService::new(cfg_svc) {
                Ok(mcp_service) => {
                    let mcp_service = Arc::new(mcp_service);
                    set_global_mcp_service(mcp_service.clone());
                    tokio::spawn(async move {
                        if let Err(e) = mcp_service.server_manager().initialize_all().await {
                            warn!("failed to initialize MCP servers: {e}");
                        }
                    });
                }
                Err(e) => warn!("failed to construct MCPService: {e}"),
            },
            Err(e) => warn!("failed to fetch global config service: {e}"),
        }

        FACADE_READY.store(true, Ordering::SeqCst);
        info!("kernel facade core initialized (K1b1)");
        Ok(())
    }

    fn core_ready(&self) -> bool {
        FACADE_READY.load(Ordering::SeqCst)
    }
}

// ── KernelSessionApi ─────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelSessionApi for KernelFacade {
    async fn create_session(&self, config: SessionConfigDto) -> Result<SessionId, KernelError> {
        let workspace = config
            .workspace_path
            .clone()
            .unwrap_or_else(default_workspace_path);
        let mut core_config = SessionConfig {
            workspace_path: Some(workspace),
            ..Default::default()
        };
        if !config.model_name.is_empty() {
            core_config.model_id = Some(config.model_name.clone());
        }
        let name = format!("session-{}", system_time_to_ms());
        let session = self
            .coordinator()
            .create_session(name, config.agent_type, core_config)
            .await
            .map_err(|e| KernelError::runtime(format!("create_session failed: {e}")))?;
        Ok(session.session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, KernelError> {
        let workspace = default_workspace_path();
        let summaries = self
            .coordinator()
            .list_sessions(Path::new(&workspace))
            .await
            .map_err(|e| KernelError::runtime(format!("list_sessions failed: {e}")))?;
        Ok(summaries.into_iter().map(summary_to_dto).collect())
    }

    async fn get_session(&self, id: &SessionId) -> Result<SessionDto, KernelError> {
        let session = self
            .coordinator()
            .session_manager()
            .get_session(id)
            .await
            .map_err(|e| KernelError::runtime(format!("get_session failed: {e}")))?;
        Ok(session_to_dto(&session))
    }

    async fn delete_session(&self, id: &SessionId) -> Result<(), KernelError> {
        let workspace = self
            .coordinator()
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::not_found(format!("session not found: {id}")))?;
        self.coordinator()
            .delete_session(&workspace, id)
            .await
            .map_err(|e| KernelError::runtime(format!("delete_session failed: {e}")))?;
        Ok(())
    }

    async fn rename_session(&self, id: &SessionId, name: &str) -> Result<(), KernelError> {
        self.coordinator()
            .update_session_title(id, name)
            .await
            .map_err(|e| KernelError::runtime(format!("rename_session failed: {e}")))?;
        Ok(())
    }

    async fn get_messages(&self, session_id: &SessionId) -> Result<Vec<MessageDto>, KernelError> {
        let messages = self
            .coordinator()
            .get_messages(session_id)
            .await
            .map_err(|e| KernelError::runtime(format!("get_messages failed: {e}")))?;
        Ok(messages.into_iter().map(message_to_dto).collect())
    }

    async fn get_session_metadata(&self, id: &SessionId) -> Result<SessionMetadataDto, KernelError> {
        let workspace = self
            .coordinator()
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::not_found(format!("session not found: {id}")))?;
        let metadata = self
            .coordinator()
            .session_manager()
            .load_session_metadata(&workspace, id)
            .await
            .map_err(|e| KernelError::runtime(format!("load_session_metadata failed: {e}")))?;
        match metadata {
            Some(m) => Ok(metadata_to_dto(&m)),
            None => Err(KernelError::not_found(format!(
                "session metadata not found: {id}"
            ))),
        }
    }

    async fn create_branch(&self, request: SessionBranchDto) -> Result<BranchId, KernelError> {
        let workspace = self
            .coordinator()
            .resolve_session_workspace_path(&request.parent_session_id)
            .await
            .ok_or_else(|| {
                KernelError::not_found(format!(
                    "parent session not found: {}",
                    request.parent_session_id
                ))
            })?;
        let branch_name = request
            .name
            .unwrap_or_else(|| format!("branch-{}", system_time_to_ms()));
        let result = crate::service::git::create_branch(&workspace, &branch_name, None)
            .await
            .map_err(|e| KernelError::runtime(format!("create_branch failed: {e}")))?;
        if result.success {
            Ok(branch_name)
        } else {
            Err(KernelError::runtime(
                result.error.unwrap_or_else(|| "git create_branch failed".to_string()),
            ))
        }
    }

    async fn get_persistence_handle(&self) -> Result<PersistenceHandleDto, KernelError> {
        Ok(PersistenceHandleDto {
            handle_id: "global".to_string(),
        })
    }
}

// ── KernelTurnApi ────────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelTurnApi for KernelFacade {
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError> {
        let scheduler = global_scheduler().ok_or_else(|| {
            KernelError::runtime("global scheduler not available — init_core not called")
        })?;
        let policy = crate::agentic::coordination::DialogSubmissionPolicy::for_source(
            crate::agentic::coordination::DialogTriggerSource::DesktopApi,
        );
        let outcome = scheduler
            .submit(
                input.session_id.clone(),
                input.text,
                None,
                None,
                input.mode,
                None,
                policy,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| KernelError::runtime(format!("submit_turn failed: {e}")))?;
        Ok(outcome_to_dto(outcome))
    }

    async fn stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError> {
        let session_id = self
            .find_session_for_turn(turn_id)
            .await
            .ok_or_else(|| KernelError::not_found(format!("turn not found: {turn_id}")))?;
        self.coordinator()
            .cancel_dialog_turn(&session_id, turn_id)
            .await
            .map_err(|e| KernelError::runtime(format!("stop_turn failed: {e}")))?;
        Ok(())
    }

    async fn get_turn_state(&self, turn_id: &TurnId) -> Result<TurnStateDto, KernelError> {
        // Core does not expose a direct turn-state query. Best-effort: scan
        // the in-memory session's dialog_turn_ids to find the owning session,
        // then read the persisted turn and map status → TurnStateKind.
        // duration_ms is None when unavailable (flagged in report).
        let session_id = self
            .find_session_for_turn(turn_id)
            .await
            .ok_or_else(|| KernelError::not_found(format!("turn not found: {turn_id}")))?;
        let workspace = self
            .coordinator()
            .resolve_session_workspace_path(&session_id)
            .await
            .ok_or_else(|| KernelError::not_found(format!("session not found: {session_id}")))?;
        let session = self
            .coordinator()
            .session_manager()
            .get_session(&session_id)
            .await
            .map_err(|e| KernelError::runtime(format!("get_session failed: {e}")))?;
        let turn_index = session
            .dialog_turn_ids
            .iter()
            .position(|t| t == turn_id)
            .ok_or_else(|| {
                KernelError::not_found(format!("turn not found in session: {turn_id}"))
            })?;
        let turn = self
            .coordinator()
            .session_manager()
            .load_dialog_turn(&workspace, &session_id, turn_index)
            .await
            .map_err(|e| KernelError::runtime(format!("load_dialog_turn failed: {e}")))?
            .ok_or_else(|| {
                KernelError::not_found(format!("turn not found in storage: {turn_id}"))
            })?;
        Ok(TurnStateDto {
            state: turn_status_to_kind(&turn.status),
            duration_ms: turn.duration_ms,
        })
    }
}

// ── KernelEventsApi ──────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelEventsApi for KernelFacade {
    async fn subscribe_events(
        &self,
        callback: Box<dyn Fn(KernelEventDto) + Send>,
    ) -> SubscriptionId {
        let id = format!("sub-{}", uuid::Uuid::new_v4());
        let subscriber = Arc::new(KernelEventSubscriber { callback });
        self.coordinator()
            .subscribe_internal(id.clone(), subscriber);
        id
    }

    async fn unsubscribe_events(&self, id: SubscriptionId) -> Result<(), KernelError> {
        self.coordinator().unsubscribe_internal(&id);
        Ok(())
    }

    async fn emit_backend_event(&self, event: BackendEventDto) -> Result<(), KernelError> {
        crate::infrastructure::events::BackendEventManager::emit_backend_event(
            &event.event_type,
            event.payload,
        )
        .await
        .map_err(|e| KernelError::runtime(format!("emit_backend_event failed: {e}")))
    }
}

// ── Event subscriber adapter ─────────────────────────────────────────────────

struct KernelEventSubscriber {
    callback: Box<dyn Fn(KernelEventDto) + Send>,
}

#[async_trait]
impl crate::agentic::events::EventSubscriber for KernelEventSubscriber {
    async fn on_event(
        &self,
        event: &crate::agentic::events::AgenticEvent,
    ) -> crate::util::errors::NortHingResult<()> {
        if let Some(dto) = agentic_event_to_dto(event) {
            (self.callback)(dto);
        }
        Ok(())
    }
}

// ── DTO conversions (helper functions — avoid orphan From impls) ─────────────

fn agentic_event_to_dto(event: &crate::agentic::events::AgenticEvent) -> Option<KernelEventDto> {
    use crate::agentic::events::AgenticEvent;
    match event {
        AgenticEvent::TextChunk {
            session_id, text, ..
        } => Some(KernelEventDto::TextChunk {
            session_id: session_id.clone(),
            text: text.clone(),
        }),
        AgenticEvent::DialogTurnStarted {
            session_id,
            turn_id,
            ..
        } => Some(KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            state: TurnStateKind::Started,
            duration_ms: None,
        }),
        AgenticEvent::DialogTurnCompleted {
            session_id,
            turn_id,
            duration_ms,
            ..
        } => Some(KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone().unwrap_or_default(),
            state: TurnStateKind::Completed,
            duration_ms: Some(*duration_ms),
        }),
        AgenticEvent::DialogTurnCancelled {
            session_id,
            turn_id,
            ..
        } => Some(KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone().unwrap_or_default(),
            state: TurnStateKind::Cancelled,
            duration_ms: None,
        }),
        AgenticEvent::DialogTurnFailed {
            session_id,
            turn_id,
            error,
            ..
        } => Some(KernelEventDto::Error {
            message: error.clone(),
        }),
        AgenticEvent::SystemError {
            error, ..
        } => Some(KernelEventDto::Error {
            message: error.clone(),
        }),
        AgenticEvent::ToolEvent {
            session_id,
            turn_id,
            tool_event,
            ..
        } => match tool_event {
            crate::agentic::events::ToolEventData::Started {
                tool_id,
                tool_name,
                ..
            } => Some(KernelEventDto::ToolCall(ToolCallDto {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                call_id: tool_id.clone(),
                name: tool_name.clone(),
                phase: ToolCallPhase::Started,
                summary: String::new(),
                detail: None,
            })),
            crate::agentic::events::ToolEventData::Completed {
                tool_id,
                tool_name,
                ..
            } => Some(KernelEventDto::ToolCall(ToolCallDto {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                call_id: tool_id.clone(),
                name: tool_name.clone(),
                phase: ToolCallPhase::Completed,
                summary: String::new(),
                detail: None,
            })),
            _ => None,
        },
        _ => None,
    }
}

fn summary_to_dto(s: &crate::agentic::core::SessionSummary) -> SessionSummaryDto {
    SessionSummaryDto {
        id: s.session_id.clone(),
        name: s.session_name.clone(),
        updated_at: system_time_to_ms_i64(s.last_activity_at),
    }
}

fn session_to_dto(s: &crate::agentic::core::Session) -> SessionDto {
    SessionDto {
        id: s.session_id.clone(),
        state: SessionStateDto {
            status: match s.state {
                SessionState::Idle => "idle".to_string(),
                SessionState::Processing { .. } => "processing".to_string(),
                SessionState::Error { .. } => "error".to_string(),
            },
        },
        kind: match s.kind {
            northhing_core_types::SessionKind::Standard => SessionKindDto::Standard,
            northhing_core_types::SessionKind::Subagent => SessionKindDto::Subagent,
            northhing_core_types::SessionKind::EphemeralChild => SessionKindDto::EphemeralChild,
        },
    }
}

fn message_to_dto(m: Message) -> MessageDto {
    MessageDto {
        id: m.id,
        role: match m.role {
            MessageRole::User => MessageRoleDto::User,
            MessageRole::Assistant => MessageRoleDto::Assistant,
            MessageRole::Tool => MessageRoleDto::Tool,
            MessageRole::System => MessageRoleDto::System,
        },
        content: match &m.content {
            MessageContent::Text(t) => MessageContentDto::Text(t.clone()),
            MessageContent::Multimodal { text, images } => MessageContentDto::Multimodal {
                text: text.clone(),
                images: images.clone(),
            },
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
            } => MessageContentDto::ToolResult {
                tool_id: tool_id.clone(),
                tool_name: tool_name.clone(),
                result: result.clone(),
                result_for_assistant: result_for_assistant.clone(),
                is_error: *is_error,
            },
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => MessageContentDto::Mixed {
                reasoning_content: reasoning_content.clone(),
                text: text.clone(),
                tool_calls: tool_calls
                    .iter()
                    .map(|tc| ToolCallStub {
                        tool_name: tc.tool_name.clone(),
                        arguments: tc.arguments.clone(),
                        is_error: tc.is_error,
                    })
                    .collect(),
            },
        },
        metadata: None,
    }
}

fn metadata_to_dto(
    m: &northhing_services_core::session::session_metadata::SessionMetadata,
) -> SessionMetadataDto {
    SessionMetadataDto {
        session_id: m.session_id.clone(),
        session_name: m.session_name.clone(),
        agent_type: m.agent_type.clone(),
        last_user_dialog_agent_type: m.last_user_dialog_agent_type.clone(),
        last_submitted_agent_type: m.last_submitted_agent_type.clone(),
        created_by: m.created_by.clone(),
        session_kind: match m.session_kind {
            northhing_core_types::SessionKind::Standard => SessionKindDto::Standard,
            northhing_core_types::SessionKind::Subagent => SessionKindDto::Subagent,
            northhing_core_types::SessionKind::EphemeralChild => SessionKindDto::EphemeralChild,
        },
        model_name: m.model_name.clone(),
        created_at: m.created_at,
        last_active_at: m.last_active_at,
        turn_count: m.turn_count,
        message_count: m.message_count,
        tool_call_count: m.tool_call_count,
        status: match m.status {
            northhing_services_core::session::session_metadata::SessionStatus::Active => {
                SessionStatusDto::Active
            }
            northhing_services_core::session::session_metadata::SessionStatus::Archived => {
                SessionStatusDto::Archived
            }
            northhing_services_core::session::session_metadata::SessionStatus::Completed => {
                SessionStatusDto::Completed
            }
        },
        terminal_session_id: m.terminal_session_id.clone(),
        snapshot_session_id: m.snapshot_session_id.clone(),
        tags: m.tags.clone(),
        custom_metadata: m.custom_metadata.clone(),
        relationship: m.relationship.as_ref().map(|r| SessionRelationshipDto {
            kind: r.kind.as_ref().map(|k| format!("{k:?}")),
            parent_session_id: r.parent_session_id.clone(),
            parent_request_id: r.parent_request_id.clone(),
            parent_dialog_turn_id: r.parent_dialog_turn_id.clone(),
            parent_turn_index: r.parent_turn_index,
            parent_tool_call_id: r.parent_tool_call_id.clone(),
            subagent_type: r.subagent_type.clone(),
        }),
        todos: m.todos.clone(),
        deep_review_run_manifest: m.deep_review_run_manifest.clone(),
        deep_review_cache: m.deep_review_cache.clone(),
        workspace_path: m.workspace_path.clone(),
        workspace_hostname: m.workspace_hostname.clone(),
        unread_completion: m.unread_completion.clone(),
        needs_user_attention: m.needs_user_attention.clone(),
    }
}

fn outcome_to_dto(o: crate::agentic::coordination::DialogSubmitOutcome) -> DialogSubmitOutcomeDto {
    use crate::agentic::coordination::DialogSubmitOutcome;
    match o {
        DialogSubmitOutcome::Started { turn_id, .. } => DialogSubmitOutcomeDto {
            turn_id,
            accepted: true,
            error: None,
        },
        DialogSubmitOutcome::Queued { turn_id, .. } => DialogSubmitOutcomeDto {
            turn_id,
            accepted: true,
            error: None,
        },
    }
}

fn turn_status_to_kind(
    s: &northhing_services_core::session::dialog_turn::DialogTurnStatus,
) -> TurnStateKind {
    use northhing_services_core::session::dialog_turn::DialogTurnStatus;
    match s {
        DialogTurnStatus::Started | DialogTurnStatus::InProgress => TurnStateKind::Started,
        DialogTurnStatus::Completed => TurnStateKind::Completed,
        DialogTurnStatus::Failed => TurnStateKind::Failed,
        DialogTurnStatus::Cancelled => TurnStateKind::Cancelled,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn default_workspace_path() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

fn system_time_to_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn system_time_to_ms_i64(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
