//! Kernel facade: pure passthrough implementation of the kernel-api traits.
//!
//! K1b1 — core passthrough impl for Bootstrap/Session/Turn/Events.
//! K1b2 — core passthrough impl for Settings/Agents/Tools/Usage/Platform.
//! DTO conversions live here; the kernel-api crate is pure definition and is
//! never modified from this side.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::{Mutex as AsyncMutex, Notify};
use std::time::Duration;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::{
    agents::{AgentInfoDto, SkillInfoDto, SkillOverridesDto, SubagentDto, SubagentScopeDto},
    platform::{
        AnalysisDto, ArtifactDto, CoreHealthDto, ImageContextDto, InspectorDataDto, PanelDto,
        PanelsConfigDto, SkillStatusDto, TerminalConfigDto,
    },
    settings::{
        AIModelConfigDto, ConfigLocationDto, GlobalConfigDto, GlobalConfigPatchDto, MCPServerDto,
        MCPServerStatusDto, ProviderConfigDto, ProviderFormDto, ProviderTestResultDto,
    },
    tools::{ToolInfoDto, ToolPort, UserInputRequestDto, UserInputResponseDto},
    usage::{TokenUsageDto, TurnUsageDto, UsageReportDto, UsageRequestDto},
};
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

// Concurrent-safe idempotent gate for init_core.
// NotStarted → InProgress(guard) → Ready  or  NotStarted → Ready (fast path)
// ── Init gate helper ──────────────────────────────────────────────────────────

/// Generic init gate: handles the three-state Mutex + Notify wait/wake/take-over
/// protocol so callers only provide the actual init future.
///
/// - Fast path: FACADE_READY already true → return Ok immediately.
/// - InProgress: wait on INIT_NOTIFY, then re-check; if still InProgress return
///   Internal error (timeout); if NotStarted (failed init reset it) → take over.
/// - NotStarted: claim InProgress, run `init`, write final state, notify.
async fn run_init_gate<Fut>(init: Fut) -> Result<(), KernelError>
where
    Fut: std::future::Future<Output = Result<(), KernelError>>,
{
    if FACADE_READY.load(Ordering::SeqCst) {
        return Ok(());
    }

    let mut guard = INIT_STATE.lock().await;
    match *guard {
        InitState::Ready => return Ok(()),
        InitState::InProgress => {
            drop(guard);
            INIT_NOTIFY.notified().await;
            if FACADE_READY.load(Ordering::SeqCst) {
                return Ok(());
            }
            let mut guard = INIT_STATE.lock().await;
            if matches!(*guard, InitState::Ready) {
                return Ok(());
            }
            if matches!(*guard, InitState::InProgress) {
                return Err(KernelError::Internal(
                    "init_core timed out waiting for concurrent initialization".to_string(),
                ));
            }
            // NotStarted (failed init reset it) — fall through to take over.
            *guard = InitState::InProgress;
            drop(guard);
        }
        InitState::NotStarted => {
            *guard = InitState::InProgress;
            drop(guard);
        }
    }

    let result = init.await;

    {
        let mut guard = INIT_STATE.lock().await;
        match result {
            Ok(()) => *guard = InitState::Ready,
            Err(_) => *guard = InitState::NotStarted,
        }
    }
    INIT_NOTIFY.notify_waiters();

    if result.is_ok() {
        FACADE_READY.store(true, Ordering::SeqCst);
        info!("kernel facade core initialized");
    }
    result
}

static INIT_STATE: AsyncMutex<InitState> = AsyncMutex::const_new(InitState::NotStarted);
static INIT_NOTIFY: Notify = Notify::const_new();

enum InitState {
    NotStarted,
    InProgress,
    Ready,
}

pub struct KernelFacade {
    /// Set by `init_core()` after `init_agentic_system()` succeeds. Never `panic`/expect.
    coordinator: OnceLock<Arc<crate::agentic::coordination::ConversationCoordinator>>,
}

static FACADE: OnceLock<Arc<KernelFacade>> = OnceLock::new();

/// Returns the global `KernelFacade` instance. Safe to call before `init_core()`;
/// facade methods return `KernelError::Internal` until the coordinator is set.
pub fn kernel_facade() -> Arc<KernelFacade> {
    FACADE.get_or_init(|| Arc::new(KernelFacade::new())).clone()
}

impl KernelFacade {
    fn new() -> Self {
        Self {
            coordinator: OnceLock::new(),
        }
    }

    /// Sets the coordinator after `init_agentic_system()` succeeds. Idempotent —
    /// if already set, this is a no-op (prevents double-initialization issues).
    fn set_coordinator(&self, coordinator: Arc<crate::agentic::coordination::ConversationCoordinator>) {
        let _ = self.coordinator.set(coordinator);
    }

    fn coordinator(&self) -> Result<&Arc<crate::agentic::coordination::ConversationCoordinator>, KernelError> {
        self.coordinator.get().ok_or_else(|| {
            KernelError::Internal("coordinator not yet initialized — call init_core() first".to_string())
        })
    }

    /// Best-effort lookup of the session that owns a given turn. Scans the
    /// in-memory session list for a session whose `dialog_turn_ids` contains
    /// the target turn id.
    async fn find_session_for_turn(&self, turn_id: &str) -> Option<String> {
        // The coordinator does not expose a turn→session index, so we scan
        // by listing all sessions and checking dialog_turn_ids on each.
        let coordinator = match self.coordinator() {
            Ok(c) => c,
            Err(_) => return None,
        };
        let workspace = default_workspace_path();
        let Ok(summaries) = coordinator
            .list_sessions(Path::new(&workspace))
            .await
        else {
            return None;
        };
        for summary in summaries {
            if let Some(session) = coordinator
                .session_manager()
                .get_session(&summary.session_id)
            {
                if session.dialog_turn_ids.iter().any(|t| t == turn_id) {
                    return Some(session.session_id);
                }
            }
        }
        None
    }

    /// Inner initialization — runs after the gate lock is acquired.
    /// Returns Ok(()) on success; failure variants are translated to KernelError
    /// by the caller, which then resets INIT_STATE to NotStarted.
    async fn init_core_inner(&self) -> Result<(), KernelError> {
        initialize_global_config()
            .await
            .map_err(|e| KernelError::Runtime(format!("initialize_global_config failed: {e}")))?;

        AIClientFactory::initialize_global()
            .await
            .map_err(|e| KernelError::Runtime(format!("AIClientFactory init failed: {e}")))?;

        let system = init_agentic_system()
            .await
            .map_err(|e| KernelError::Runtime(format!("init_agentic_system failed: {e}")))?;

        let coordinator = system.coordinator.clone();
        let session_manager = coordinator.session_manager().clone();
        let scheduler = DialogScheduler::new(coordinator.clone(), session_manager);

        let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
        let injection_ok =
            coordinator.set_round_injection_source(scheduler.round_injection_monitor());
        if !notifier_ok || !injection_ok {
            return Err(KernelError::Runtime("dialog scheduler wiring conflict".to_string()));
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

        // Inject coordinator into facade — must succeed before marking ready.
        self.set_coordinator(coordinator.clone());

        Ok(())
    }
}

// ── KernelBootstrapApi ──────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelBootstrapApi for KernelFacade {
    async fn init_core(&self) -> Result<(), KernelError> {
        run_init_gate(self.init_core_inner()).await
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
            .coordinator()?
            .create_session(name, config.agent_type, core_config)
            .await
            .map_err(|e| KernelError::Runtime(format!("create_session failed: {e}")))?;
        Ok(session.session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, KernelError> {
        let workspace = default_workspace_path();
        let summaries = self
            .coordinator()?
            .list_sessions(Path::new(&workspace))
            .await
            .map_err(|e| KernelError::Runtime(format!("list_sessions failed: {e}")))?;
        Ok(summaries.into_iter().map(summary_to_dto).collect())
    }

    async fn get_session(&self, id: &SessionId) -> Result<SessionDto, KernelError> {
        let session = self
            .coordinator()?
            .session_manager()
            .get_session(id)
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        Ok(session_to_dto(&session))
    }

    async fn delete_session(&self, id: &SessionId) -> Result<(), KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        self.coordinator()?
            .delete_session(&workspace, id)
            .await
            .map_err(|e| KernelError::Runtime(format!("delete_session failed: {e}")))?;
        Ok(())
    }

    async fn rename_session(&self, id: &SessionId, name: &str) -> Result<(), KernelError> {
        self.coordinator()?
            .update_session_title(id, name)
            .await
            .map_err(|e| KernelError::Runtime(format!("rename_session failed: {e}")))?;
        Ok(())
    }

    async fn get_messages(&self, session_id: &SessionId) -> Result<Vec<MessageDto>, KernelError> {
        let messages = self
            .coordinator()?
            .get_messages(session_id)
            .await
            .map_err(|e| KernelError::Runtime(format!("get_messages failed: {e}")))?;
        Ok(messages.into_iter().map(message_to_dto).collect())
    }

    async fn get_session_metadata(&self, id: &SessionId) -> Result<SessionMetadataDto, KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {id}")))?;
        let metadata = self
            .coordinator()?
            .session_manager()
            .load_session_metadata(&workspace, id)
            .await
            .map_err(|e| KernelError::Runtime(format!("load_session_metadata failed: {e}")))?;
        match metadata {
            Some(m) => Ok(metadata_to_dto(&m)),
            None => Err(KernelError::NotFound(format!(
                "session metadata not found: {id}"
            ))),
        }
    }

    async fn create_branch(&self, request: SessionBranchDto) -> Result<BranchId, KernelError> {
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(&request.parent_session_id)
            .await
            .ok_or_else(|| {
                KernelError::NotFound(format!(
                    "parent session not found: {}",
                    request.parent_session_id
                ))
            })?;
        let branch_name = request
            .name
            .unwrap_or_else(|| format!("branch-{}", system_time_to_ms()));
        let result = northhing_services_integrations::git::GitService::create_branch(
            &workspace,
            &branch_name,
            None,
        )
        .await
        .map_err(|e| KernelError::Runtime(format!("create_branch failed: {e}")))?;
        if result.success {
            Ok(branch_name)
        } else {
            Err(KernelError::Runtime(
                result.error.unwrap_or_else(|| "git create_branch failed".to_string()),
            ))
        }
    }

    async fn get_persistence_handle(&self) -> Result<PersistenceHandleDto, KernelError> {
        // NEEDS_CONTEXT: PersistenceManager folding deferred to K4b.
        Err(KernelError::Internal(
            "not yet wired: get_persistence_handle — PersistenceManager folding deferred (K4b)".to_string(),
        ))
    }
}

// ── KernelTurnApi ────────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelTurnApi for KernelFacade {
    async fn submit_turn(&self, input: TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError> {
        // Workspace resolution priority:
        // 1. input.workspace_path (explicit, from caller)
        // 2. resolve_session_workspace_path (session record; needed for scheduler restore)
        // 3. default_workspace_path (last resort)
        let scheduler = global_scheduler().ok_or_else(|| {
            KernelError::Runtime("global scheduler not available — init_core not called".to_string())
        })?;
        let workspace = if let Some(ref wp) = input.workspace_path {
            wp.clone()
        } else {
            match self.coordinator().ok() {
                Some(c) => match c.resolve_session_workspace_path(&input.session_id).await {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => default_workspace_path(),
                },
                None => default_workspace_path(),
            }
        };
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
                Some(workspace),
                policy,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| KernelError::Runtime(format!("submit_turn failed: {e}")))?;
        Ok(outcome_to_dto(outcome))
    }

    async fn stop_turn(&self, turn_id: &TurnId) -> Result<(), KernelError> {
        let session_id = self
            .find_session_for_turn(turn_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        self.coordinator()?
            .cancel_dialog_turn(&session_id, turn_id)
            .await
            .map_err(|e| KernelError::Runtime(format!("stop_turn failed: {e}")))?;
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
            .ok_or_else(|| KernelError::NotFound(format!("turn not found: {turn_id}")))?;
        let workspace = self
            .coordinator()?
            .resolve_session_workspace_path(&session_id)
            .await
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let session = self
            .coordinator()?
            .session_manager()
            .get_session(&session_id)
            .ok_or_else(|| KernelError::NotFound(format!("session not found: {session_id}")))?;
        let turn_index = session
            .dialog_turn_ids
            .iter()
            .position(|t| t == turn_id)
            .ok_or_else(|| {
                KernelError::NotFound(format!("turn not found in session: {turn_id}"))
            })?;
        let turn = self
            .coordinator()?
            .session_manager()
            .persistence_manager
            .load_dialog_turn(&workspace, &session_id, turn_index)
            .await
            .map_err(|e| KernelError::Runtime(format!("load_dialog_turn failed: {e}")))?
            .ok_or_else(|| {
                KernelError::NotFound(format!("turn not found in storage: {turn_id}"))
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
        callback: Box<dyn Fn(KernelEventDto) + Send + 'static>,
    ) -> Result<SubscriptionId, KernelError> {
        // NOTE: Unlike event_bridge.rs:75-96 which uses a 500ms retry pattern
        // for reconnect, this facade directly subscribes assuming init_core has
        // already been called and the coordinator is stable.
        let coordinator = match self.coordinator() {
            Ok(c) => c,
            Err(e) => {
                warn!("subscribe_events called before init_core(): {e}");
                return Err(KernelError::Runtime(
                    "kernel facade not initialized — init_core not called".to_string(),
                ));
            }
        };
        let id = format!("sub-{}", uuid::Uuid::new_v4());
        let subscriber = KernelEventSubscriber {
            callback: Arc::new(Mutex::new(callback)),
        };
        coordinator.subscribe_internal(id.clone(), subscriber);
        Ok(id)
    }

    async fn unsubscribe_events(&self, id: SubscriptionId) -> Result<(), KernelError> {
        self.coordinator()?.unsubscribe_internal(&id);
        Ok(())
    }

    async fn emit_backend_event(&self, event: BackendEventDto) -> Result<(), KernelError> {
        let be = crate::infrastructure::events::BackendEvent::Custom {
            event_name: event.event_type,
            payload: event.payload.unwrap_or(serde_json::Value::Null),
        };
        crate::infrastructure::events::emit_global_event(be)
            .await
            .map_err(|e| KernelError::Runtime(format!("emit_backend_event failed: {e}")))
    }
}

// ── Event subscriber adapter ─────────────────────────────────────────────────

struct KernelEventSubscriber {
    callback: Arc<Mutex<Box<dyn Fn(KernelEventDto) + Send + 'static>>>,
}

impl KernelEventSubscriber {
    fn invoke_callback(&self, dto: KernelEventDto) {
        let guard = match self.callback.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!(
                    "KernelEventSubscriber callback lock poisoned, recovering: {}",
                    poisoned
                );
                poisoned.into_inner()
            }
        };
        (guard)(dto);
    }
}

#[async_trait]
impl crate::agentic::events::EventSubscriber for KernelEventSubscriber {
    async fn on_event(
        &self,
        event: &crate::agentic::events::AgenticEvent,
    ) -> crate::util::errors::NortHingResult<()> {
        if let Some(dto) = agentic_event_to_dto(event) {
            self.invoke_callback(dto);
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
            turn_id: turn_id.clone(),
            state: TurnStateKind::Completed,
            duration_ms: Some(*duration_ms),
        }),
        AgenticEvent::DialogTurnCancelled {
            session_id,
            turn_id,
            ..
        } => Some(KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
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
                params,
                ..
            } => {
                let params_str = params.to_string();
                let summary = extract_summary_from_params(params);
                Some(KernelEventDto::ToolCall(ToolCallDto {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: tool_id.clone(),
                    name: tool_name.clone(),
                    phase: ToolCallPhase::Started,
                    summary,
                    detail: Some(truncate_4000(&params_str)),
                }))
            }
            crate::agentic::events::ToolEventData::Completed {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                ..
            } => {
                let result_str = result.to_string();
                let summary = first_line_truncated(
                    result_for_assistant.as_deref().unwrap_or(&result_str),
                );
                Some(KernelEventDto::ToolCall(ToolCallDto {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: tool_id.clone(),
                    name: tool_name.clone(),
                    phase: ToolCallPhase::Completed,
                    summary,
                    detail: Some(truncate_4000(&result_str)),
                }))
            }
            crate::agentic::events::ToolEventData::Failed {
                tool_id,
                tool_name,
                error,
                ..
            } => Some(KernelEventDto::ToolCall(ToolCallDto {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                call_id: tool_id.clone(),
                name: tool_name.clone(),
                phase: ToolCallPhase::Completed,
                summary: first_line_truncated(error),
                detail: Some(truncate_4000(error)),
            })),
            crate::agentic::events::ToolEventData::Cancelled {
                tool_id,
                tool_name,
                reason,
                ..
            } => Some(KernelEventDto::ToolCall(ToolCallDto {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                call_id: tool_id.clone(),
                name: tool_name.clone(),
                phase: ToolCallPhase::Completed,
                summary: first_line_truncated(&format!("cancelled: {reason}")),
                detail: Some(truncate_4000(reason)),
            })),
            _ => None,
        },
        _ => None,
    }
}

fn summary_to_dto(s: crate::agentic::core::SessionSummary) -> SessionSummaryDto {
    SessionSummaryDto {
        id: s.session_id,
        name: s.session_name,
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
                images: images.iter().filter_map(|img| img.image_path.clone()).collect(),
            },
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
                ..
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
                        arguments: Some(tc.arguments.clone()),
                        is_error: tc.is_error,
                    })
                    .collect(),
            },
        },
        metadata: Some(metadata_to_message_dto(&m.metadata)),
    }
}

fn metadata_to_message_dto(m: &crate::agentic::message::MessageMetadata) -> MessageMetadataDto {
    MessageMetadataDto {
        turn_id: m.turn_id.clone(),
        round_id: m.round_id.clone(),
        tokens: m.tokens,
        thinking_signature: m.thinking_signature.clone(),
        // K1b2 fix: use format!("{:?}") for enum fields per kernel-api spec
        semantic_kind: m.semantic_kind.as_ref().map(|k| format!("{:?}", k)),
        internal_reminder_kind: m.internal_reminder_kind.as_ref().map(|k| format!("{:?}", k)),
        // K1b2 fix: compression_payload uses serde_json::to_value
        compression_payload: m.compression_payload.as_ref().map(|p| {
            serde_json::to_value(p).unwrap_or(serde_json::Value::Null)
        }),
    }
}

fn metadata_to_dto(
    m: &northhing_services_core::session::SessionMetadata,
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
            northhing_services_core::session::SessionStatus::Active => {
                SessionStatusDto::Active
            }
            northhing_services_core::session::SessionStatus::Archived => {
                SessionStatusDto::Archived
            }
            northhing_services_core::session::SessionStatus::Completed => {
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
    s: &northhing_services_core::session::TurnStatus,
) -> TurnStateKind {
    match s {
        northhing_services_core::session::TurnStatus::InProgress => TurnStateKind::Started,
        northhing_services_core::session::TurnStatus::Completed => TurnStateKind::Completed,
        northhing_services_core::session::TurnStatus::Error => TurnStateKind::Failed,
        northhing_services_core::session::TurnStatus::Cancelled => TurnStateKind::Cancelled,
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

/// Shared first-line error helper for provider test results.
/// Takes first line, trims, caps at 120 chars, falls back to "connection failed" if empty.
fn first_line_error(detail: &str) -> String {
    let first_line = detail.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        "connection failed".to_string()
    } else {
        first_line.chars().take(120).collect()
    }
}

/// Extracts first line, trims, and caps at 120 chars. Returns empty string if input is empty.
fn first_line_truncated(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().chars().take(120).collect()
}

/// Truncates a string to at most 4000 characters (by char count).
fn truncate_4000(s: &str) -> String {
    s.chars().take(4000).collect()
}

/// Extracts a human-readable summary from tool-call params JSON.
/// Tries "command", "path", "file_path", "content", "query" keys in order;
/// falls back to the full params string. Result is first-line truncated to 120 chars.
fn extract_summary_from_params(params: &serde_json::Value) -> String {
    let candidates = ["command", "path", "file_path", "content", "query"];
    for key in candidates {
        if let Some(val) = params.get(key).and_then(|v| v.as_str()) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return first_line_truncated(trimmed);
            }
        }
    }
    first_line_truncated(&params.to_string())
}

/// Maps `MCPServerStatus` to `MCPServerStatusKind` DTO.
fn map_mcp_status_kind(
    status: crate::service::mcp::MCPServerStatus,
) -> northhing_kernel_api::settings::MCPServerStatusKind {
    match status {
        crate::service::mcp::MCPServerStatus::Connected
        | crate::service::mcp::MCPServerStatus::Healthy => {
            northhing_kernel_api::settings::MCPServerStatusKind::Connected
        }
        crate::service::mcp::MCPServerStatus::Starting
        | crate::service::mcp::MCPServerStatus::Uninitialized
        | crate::service::mcp::MCPServerStatus::Reconnecting => {
            northhing_kernel_api::settings::MCPServerStatusKind::Starting
        }
        crate::service::mcp::MCPServerStatus::NeedsAuth => {
            northhing_kernel_api::settings::MCPServerStatusKind::Failed {
                message: "needs authentication".to_string(),
            }
        }
        crate::service::mcp::MCPServerStatus::Failed => {
            northhing_kernel_api::settings::MCPServerStatusKind::Failed {
                message: "runtime reported failure".to_string(),
            }
        }
        crate::service::mcp::MCPServerStatus::Stopping
        | crate::service::mcp::MCPServerStatus::Stopped => {
            northhing_kernel_api::settings::MCPServerStatusKind::Disabled
        }
    }
}

/// Maps a probe result (timeout-wrapped Result<MCPServerStatus>) to `MCPServerStatusKind`.
/// Used by `get_inspector_data` where the probe result has three cases: Ok(status), Ok(err), Err(timeout).
#[allow(clippy::type_complexity)]
fn map_mcp_probe_status(
    probe_status: Result<
        Result<crate::service::mcp::MCPServerStatus, crate::util::errors::NortHingError>,
        tokio::time::error::Elapsed,
    >,
) -> northhing_kernel_api::settings::MCPServerStatusKind {
    match probe_status {
        Ok(Ok(status)) => map_mcp_status_kind(status),
        Ok(Err(_)) => northhing_kernel_api::settings::MCPServerStatusKind::Failed {
            message: "status probe failed".to_string(),
        },
        Err(_) => northhing_kernel_api::settings::MCPServerStatusKind::ProbeTimeout,
    }
}

// ── KernelSettingsApi ─────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelSettingsApi for KernelFacade {
    async fn get_global_config(&self) -> Result<GlobalConfigDto, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let config: crate::service::config::GlobalConfig = cfg_svc
            .config(None)
            .await
            .map_err(|e| KernelError::Config(format!("get global config: {e}")))?;
        Ok(GlobalConfigDto {
            providers: models
                .iter()
                .map(|m| ProviderConfigDto {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    base_url: m.base_url.clone(),
                    api_key: m.api_key.clone(),
                    model: m.model_name.clone(),
                    extra: None,
                })
                .collect(),
            default_provider_id: config.ai.default_models.primary.clone(),
            workspace_config: None,
        })
    }

    async fn update_global_config(&self, patch: GlobalConfigPatchDto) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        if let Some(providers) = patch.providers {
            for p in providers {
                let model_cfg = crate::service::config::runtime::AIModelConfig {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    provider: p.id.clone(),
                    model_name: p.model.clone(),
                    base_url: p.base_url.clone(),
                    request_url: None,
                    api_key: p.api_key.clone(),
                    context_window: None,
                    max_tokens: None,
                    temperature: None,
                    top_p: None,
                    enabled: true,
                    category: Default::default(),
                    capabilities: vec![],
                    recommended_for: vec![],
                    metadata: None,
                    enable_thinking_process: false,
                    reasoning_mode: None,
                    inline_think_in_text: false,
                    custom_headers: None,
                    custom_headers_mode: None,
                    skip_ssl_verify: false,
                    reasoning_effort: None,
                    thinking_budget_tokens: None,
                    custom_request_body: None,
                    custom_request_body_mode: None,
                    auth: Default::default(),
                };
                // Upsert: check if model exists, then update or add.
                let existing = cfg_svc
                    .get_ai_models()
                    .await
                    .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?
                    .iter()
                    .any(|m| m.id == p.id);
                if existing {
                    cfg_svc
                        .update_ai_model(&p.id, model_cfg)
                        .await
                        .map_err(|e| KernelError::Config(format!("update_ai_model: {e}")))?;
                } else {
                    cfg_svc
                        .add_ai_model(model_cfg)
                        .await
                        .map_err(|e| KernelError::Config(format!("add_ai_model: {e}")))?;
                }
            }
        }
        if let Some(default_id) = patch.default_provider_id {
            cfg_svc
                .set_config("ai.default_models.primary", default_id.as_str())
                .await
                .map_err(|e| KernelError::Config(format!("set default provider: {e}")))?;
        }
        Ok(())
    }

    async fn list_model_configs(&self) -> Result<Vec<AIModelConfigDto>, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        Ok(models
            .into_iter()
            .map(|m| AIModelConfigDto {
                id: m.id,
                provider_id: m.provider,
                model: m.model_name,
                display_name: Some(m.name),
                max_tokens: m.max_tokens,
                temperature: m.temperature,
            })
            .collect())
    }

    async fn upsert_model_config(&self, config: AIModelConfigDto) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        // Check if model already exists to preserve existing fields DTO doesn't carry.
        let existing_models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let existing = existing_models.iter().find(|m| m.id == config.id);
        let model_cfg = if let Some(existing_model) = existing {
            // Preserve existing base_url, api_key, and other fields; override only what DTO carries.
            crate::service::config::runtime::AIModelConfig {
                id: config.id.clone(),
                name: config.display_name.unwrap_or_else(|| existing_model.name.clone()),
                provider: config.provider_id.clone(),
                model_name: config.model.clone(),
                base_url: existing_model.base_url.clone(),
                request_url: existing_model.request_url.clone(),
                api_key: existing_model.api_key.clone(),
                context_window: existing_model.context_window,
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                top_p: existing_model.top_p,
                enabled: existing_model.enabled,
                category: existing_model.category.clone(),
                capabilities: existing_model.capabilities.clone(),
                recommended_for: existing_model.recommended_for.clone(),
                metadata: existing_model.metadata.clone(),
                enable_thinking_process: existing_model.enable_thinking_process,
                reasoning_mode: existing_model.reasoning_mode.clone(),
                inline_think_in_text: existing_model.inline_think_in_text,
                custom_headers: existing_model.custom_headers.clone(),
                custom_headers_mode: existing_model.custom_headers_mode.clone(),
                skip_ssl_verify: existing_model.skip_ssl_verify,
                reasoning_effort: existing_model.reasoning_effort.clone(),
                thinking_budget_tokens: existing_model.thinking_budget_tokens,
                custom_request_body: existing_model.custom_request_body.clone(),
                custom_request_body_mode: existing_model.custom_request_body_mode.clone(),
                auth: existing_model.auth.clone(),
            }
        } else {
            crate::service::config::runtime::AIModelConfig {
                id: config.id.clone(),
                name: config.display_name.unwrap_or_default(),
                provider: config.provider_id.clone(),
                model_name: config.model.clone(),
                base_url: String::new(),
                request_url: None,
                api_key: String::new(),
                context_window: None,
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                top_p: None,
                enabled: true,
                category: Default::default(),
                capabilities: vec![],
                recommended_for: vec![],
                metadata: None,
                enable_thinking_process: false,
                reasoning_mode: None,
                inline_think_in_text: false,
                custom_headers: None,
                custom_headers_mode: None,
                skip_ssl_verify: false,
                reasoning_effort: None,
                thinking_budget_tokens: None,
                custom_request_body: None,
                custom_request_body_mode: None,
                auth: Default::default(),
            }
        };
        if existing.is_some() {
            cfg_svc
                .update_ai_model(&config.id, model_cfg)
                .await
                .map_err(|e| KernelError::Config(format!("update_ai_model: {e}")))?;
        } else {
            cfg_svc
                .add_ai_model(model_cfg)
                .await
                .map_err(|e| KernelError::Config(format!("add_ai_model: {e}")))?;
        }
        Ok(())
    }

    async fn delete_model_config(&self, id: &str) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        cfg_svc
            .delete_ai_model(id)
            .await
            .map_err(|e| KernelError::Config(format!("delete_model_config: {e}")))
    }

    async fn set_default_provider(&self, id: &str) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        cfg_svc
            .set_config("ai.default_models.primary", id)
            .await
            .map_err(|e| KernelError::Config(format!("set_default_provider: {e}")))
    }

    async fn list_mcp_servers(&self) -> Result<Vec<MCPServerDto>, KernelError> {
        let mcp_svc = crate::service::mcp::global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        let configs = mcp_svc
            .config_service()
            .load_all_configs()
            .await
            .map_err(|e| KernelError::Runtime(format!("list_mcp_servers: {e}")))?;
        Ok(configs
            .into_iter()
            .map(|c| MCPServerDto {
                id: c.id.clone(),
                name: c.name.clone(),
                config: northhing_kernel_api::settings::MCPServerConfigDto {
                    command: c.command.unwrap_or_default(),
                    args: c.args.clone(),
                    env: Some(c.env),
                },
                location: match c.location {
                    crate::service::mcp::config::ConfigLocation::User => ConfigLocationDto::User,
                    crate::service::mcp::config::ConfigLocation::Project => ConfigLocationDto::Project,
                    crate::service::mcp::config::ConfigLocation::BuiltIn => ConfigLocationDto::BuiltIn,
                },
            })
            .collect())
    }

    async fn upsert_mcp_server(&self, config: MCPServerDto) -> Result<(), KernelError> {
        let mcp_svc = crate::service::mcp::global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        // Map location from DTO to ConfigLocation.
        let location = match config.location {
            northhing_kernel_api::settings::ConfigLocationDto::User => {
                northhing_services_integrations::mcp::config::ConfigLocation::User
            }
            northhing_kernel_api::settings::ConfigLocationDto::Project => {
                northhing_services_integrations::mcp::config::ConfigLocation::Project
            }
            northhing_kernel_api::settings::ConfigLocationDto::BuiltIn => {
                northhing_services_integrations::mcp::config::ConfigLocation::BuiltIn
            }
        };
        // server_type cannot be determined from DTO — placeholder until trait is extended.
        let server_type = northhing_services_integrations::mcp::server::MCPServerType::Local;
        let mcp_config = crate::service::mcp::MCPServerConfig {
            id: config.id.clone(),
            name: config.name.clone(),
            server_type,
            transport: None,
            command: Some(config.config.command),
            args: config.config.args,
            env: config.config.env.unwrap_or_default(),
            headers: Default::default(),
            url: None,
            auto_start: true,
            enabled: true,
            location,
            capabilities: vec![],
            settings: Default::default(),
            oauth: None,
            xaa: None,
        };
        mcp_svc
            .config_service()
            .save_server_config(&mcp_config)
            .await
            .map_err(|e| KernelError::Config(format!("save_server_config: {e}")))
    }

    async fn delete_mcp_server(&self, id: &str) -> Result<(), KernelError> {
        let mcp_svc = crate::service::mcp::global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        mcp_svc
            .config_service()
            .delete_server_config(id)
            .await
            .map_err(|e| KernelError::Config(format!("delete_server_config: {e}")))
    }

    async fn get_mcp_status(&self, id: &str) -> Result<MCPServerStatusDto, KernelError> {
        let mcp_svc = crate::service::mcp::global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        let status = tokio::time::timeout(
            Duration::from_millis(30),
            mcp_svc.server_manager().get_server_status(id),
        )
        .await
        .map_err(|_| KernelError::Timeout)?
        .map_err(|e| KernelError::Runtime(format!("get_mcp_status: {e}")))?;
        Ok(MCPServerStatusDto {
            id: id.to_string(),
            status: map_mcp_status_kind(status),
        })
    }

    async fn test_provider(&self, id: &str) -> Result<ProviderTestResultDto, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let model = models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| KernelError::NotFound(format!("provider not found: {id}")))?;
        let ai_config = crate::util::types::AIConfig::try_from(model.clone())
            .map_err(|e| KernelError::Validation(format!("invalid config: {e}")))?;
        let client = crate::infrastructure::ai::AIClient::new(ai_config);
        match client.test_connection().await {
            Ok(result) => Ok(ProviderTestResultDto {
                success: result.success,
                error: result.error_details.map(|d| first_line_error(&d)),
            }),
            Err(e) => Ok(ProviderTestResultDto {
                success: false,
                error: Some(first_line_error(&e.to_string())),
            }),
        }
    }

    async fn test_provider_config(
        &self,
        form: ProviderFormDto,
    ) -> Result<ProviderTestResultDto, KernelError> {
        // Build an in-memory AIClient from the form and test connection.
        use crate::service::config::runtime::AIModelConfig;
        let model_cfg = AIModelConfig {
            id: form.provider_id.clone(),
            name: form.provider_id.clone(),
            provider: form.provider_id.clone(),
            model_name: form.model.clone().unwrap_or_default(),
            base_url: form.base_url.clone().unwrap_or_default(),
            request_url: None,
            api_key: form.api_key.clone().unwrap_or_default(),
            context_window: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            enabled: true,
            category: Default::default(),
            capabilities: vec![],
            recommended_for: vec![],
            metadata: None,
            enable_thinking_process: false,
            reasoning_mode: None,
            inline_think_in_text: false,
            custom_headers: None,
            custom_headers_mode: None,
            skip_ssl_verify: false,
            reasoning_effort: None,
            thinking_budget_tokens: None,
            custom_request_body: None,
            custom_request_body_mode: None,
            auth: Default::default(),
        };
        let ai_config = crate::util::types::AIConfig::try_from(model_cfg)
            .map_err(|e| KernelError::Validation(format!("invalid config: {e}")))?;
        let client = crate::infrastructure::ai::AIClient::new(ai_config);
        match client.test_connection().await {
            Ok(result) => Ok(ProviderTestResultDto {
                success: result.success,
                error: result.error_details.map(|d| first_line_error(&d)),
            }),
            Err(e) => Ok(ProviderTestResultDto {
                success: false,
                error: Some(first_line_error(&e.to_string())),
            }),
        }
    }
}

// ── KernelAgentsApi ────────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelAgentsApi for KernelFacade {
    async fn list_agents(&self) -> Result<Vec<AgentInfoDto>, KernelError> {
        let registry = crate::agentic::agents::agent_registry();
        let agents = registry.get_modes_info().await;
        Ok(agents
            .into_iter()
            .map(|a| AgentInfoDto {
                id: a.key.clone(),
                name: a.name.clone(),
                agent_type: a.id.clone(),
                description: Some(a.description),
                capabilities: None,
            })
            .collect())
    }

    async fn list_subagents(
        &self,
        scope: SubagentScopeDto,
    ) -> Result<Vec<SubagentDto>, KernelError> {
        let registry = crate::agentic::agents::agent_registry();
        // workspace_path not available in SubagentScopeDto; pass None until trait is extended.
        let subagents = registry.get_subagents_info(None).await;
        Ok(subagents
            .into_iter()
            .map(|a| SubagentDto {
                id: a.key.clone(),
                name: a.name.clone(),
                agent_type: a.id.clone(),
                parent_session_id: scope.parent_session_id.clone(),
                status: None,
            })
            .collect())
    }

    async fn list_skills(&self) -> Result<Vec<SkillInfoDto>, KernelError> {
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        Ok(skills
            .into_iter()
            .map(|s| SkillInfoDto {
                id: s.key.clone(),
                name: s.name.clone(),
                description: s.description.clone(),
                enabled: false, // enabled state is mode-dependent; requires mode context
                mode: None,
                tags: None,
            })
            .collect())
    }

    async fn get_skill(&self, id: &str) -> Result<SkillInfoDto, KernelError> {
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        skills
            .into_iter()
            .find(|s| s.key == id)
            .map(|s| SkillInfoDto {
                id: s.key,
                name: s.name,
                description: s.description,
                enabled: false, // enabled state is mode-dependent; requires mode context
                mode: None,
                tags: None,
            })
            .ok_or_else(|| KernelError::NotFound(format!("skill not found: {id}")))
    }

    async fn set_skill_enabled(
        &self,
        _id: &str,
        _scope: northhing_kernel_api::agents::SkillScopeDto,
        _enabled: bool,
    ) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: mode_id required but not present in SkillScopeDto.
        Err(KernelError::Internal("not yet wired: set_skill_enabled — mode_id not in scope".to_string()))
    }

    async fn load_skill_overrides(&self) -> Result<SkillOverridesDto, KernelError> {
        // NEEDS_CONTEXT: mode_id required but not present in trait signature.
        Err(KernelError::Internal("not yet wired: load_skill_overrides — mode_id not available".to_string()))
    }

    async fn load_project_skills(&self) -> Result<northhing_kernel_api::agents::ProjectSkillsDto, KernelError> {
        // NEEDS_CONTEXT: workspace_path required but not present in trait signature.
        Err(KernelError::Internal("not yet wired: load_project_skills — workspace_path not available".to_string()))
    }

    async fn save_project_skills(
        &self,
        doc: northhing_kernel_api::agents::ProjectSkillsDto,
    ) -> Result<(), KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::{
            load_project_mode_skills_document_local, save_project_mode_skills_document_local,
            set_disabled_mode_skills_in_document,
        };
        use crate::service::config::agent_profile_project_store::ProjectAgentProfilesDocument;
        use std::collections::HashMap;

        let workspace_root = std::path::Path::new(&doc.workspace_path);
        let mut document = load_project_mode_skills_document_local(workspace_root)
            .await
            .map_err(|e| KernelError::Config(format!("load_project_mode_skills_document_local: {e}")))?;

        for skill_entry in &doc.skills {
            // mode_id is not in ProjectSkillEntry; use default profile.
            // NEEDS_CONTEXT: proper implementation requires mode_id per skill.
            let _ = set_disabled_mode_skills_in_document(
                &mut document,
                "default",
                vec![skill_entry.skill_id.clone()],
            );
        }

        save_project_mode_skills_document_local(workspace_root, &document)
            .await
            .map_err(|e| KernelError::Config(format!("save_project_mode_skills_document_local: {e}")))
    }

    async fn resolve_skill_default_enabled(
        &self,
        skill_id: &str,
        mode: &str,
    ) -> Result<bool, KernelError> {
        use crate::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode;
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        match skills.into_iter().find(|s| s.key == skill_id) {
            Some(skill) => Ok(resolve_skill_default_enabled_for_mode(&skill, mode)),
            None => Err(KernelError::NotFound(format!(
                "skill not found: {skill_id}"
            ))),
        }
    }
}

// ── KernelToolsApi ─────────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelToolsApi for KernelFacade {
    async fn list_tools(&self) -> Result<Vec<ToolInfoDto>, KernelError> {
        // NEEDS_CONTEXT: tool registry is not exposed through a simple passthrough.
        Err(KernelError::Internal("not yet wired: list_tools".to_string()))
    }

    async fn register_tool(&self, _tool: std::sync::Arc<dyn ToolPort>) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: ACP tool registration requires tool pipeline wiring.
        Err(KernelError::Internal("not yet wired: register_tool".to_string()))
    }

    async fn request_user_input(
        &self,
        _request: UserInputRequestDto,
    ) -> Result<UserInputResponseDto, KernelError> {
        // NEEDS_CONTEXT: user input flow requires UI integration.
        Err(KernelError::Internal("not yet wired: request_user_input".to_string()))
    }
}

// ── KernelUsageApi ─────────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelUsageApi for KernelFacade {
    async fn generate_session_usage(
        &self,
        _request: UsageRequestDto,
    ) -> Result<UsageReportDto, KernelError> {
        Err(KernelError::Internal("not yet wired: generate_session_usage".to_string()))
    }

    async fn render_usage_markdown(&self, report: &UsageReportDto) -> String {
        // NOTE: Cannot forward to `render_usage_report_markdown` because UsageReportDto
        // is not type-isomorphic with SessionUsageReport — requires a DTO→SessionUsageReport
        // adapter (P2 trait extension territory). Hand-written format retained as fallback.
        format!(
            "## Usage Report\n\nSession: {}\n\nTotal tokens: {}\nPrompt tokens: {}\nCompletion tokens: {}\nTurn count: {}\nTool call count: {}",
            report.session_id,
            report.total_tokens,
            report.prompt_tokens,
            report.completion_tokens,
            report.turn_count,
            report.tool_call_count
        )
    }

    async fn get_token_usage(&self, _session_id: &SessionId) -> Result<TokenUsageDto, KernelError> {
        // NEEDS_CONTEXT: requires TokenUsageService access and PersistenceManager.
        Err(KernelError::Internal("not yet wired: get_token_usage".to_string()))
    }
}

// ── KernelPlatformApi ──────────────────────────────────────────────────────────

#[async_trait]
impl northhing_kernel_api::KernelPlatformApi for KernelFacade {
    async fn open_terminal(&self, _config: TerminalConfigDto) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: terminal open requires host UI integration.
        Err(KernelError::Internal("not yet wired: open_terminal".to_string()))
    }

    async fn analyze_image(&self, _context: ImageContextDto) -> Result<AnalysisDto, KernelError> {
        Err(KernelError::Internal("not yet wired: analyze_image".to_string()))
    }

    async fn get_core_health(&self) -> Result<CoreHealthDto, KernelError> {
        Ok(CoreHealthDto {
            healthy: FACADE_READY.load(Ordering::SeqCst),
            details: if FACADE_READY.load(Ordering::SeqCst) {
                vec!["core initialized".to_string()]
            } else {
                vec!["core not yet initialized".to_string()]
            },
        })
    }

    async fn read_panels_config(&self) -> Result<PanelsConfigDto, KernelError> {
        // F3: read panels.json from product config directory.
        let config_dir = dirs::config_dir()
            .ok_or_else(|| KernelError::Config("cannot find config directory".to_string()))?;
        let panels_path = config_dir.join("northhing").join("config").join("panels.json");
        if !panels_path.exists() {
            return Ok(PanelsConfigDto { panels: vec![] });
        }
        let content = tokio::fs::read_to_string(&panels_path)
            .await
            .map_err(|e| KernelError::Runtime(format!("read panels.json: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| KernelError::Runtime(format!("parse panels.json: {e}")))
    }

    async fn is_onboarding_complete(&self) -> Result<bool, KernelError> {
        // NEEDS_CONTEXT: onboarding_completed is desktop UI state, not core GlobalConfig.
        Err(KernelError::Internal(
            "not yet wired: is_onboarding_complete".to_string(),
        ))
    }

    async fn complete_onboarding(&self) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: onboarding_completed is desktop UI state, not core GlobalConfig.
        Err(KernelError::Internal(
            "not yet wired: complete_onboarding".to_string(),
        ))
    }

    async fn get_inspector_data(&self) -> Result<InspectorDataDto, KernelError> {
        // Forward to global config for model name, MCP service for MCP status.
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let config: crate::service::config::GlobalConfig = cfg_svc
            .config(None)
            .await
            .map_err(|e| KernelError::Config(format!("get global config: {e}")))?;
        let model_name = config
            .ai
            .default_models
            .primary
            .clone()
            .unwrap_or_else(|| "not configured".to_string());

        // Get MCP status.
        let mcp_status = if let Some(mcp_svc) = crate::service::mcp::global_mcp_service() {
            match mcp_svc.config_service().load_all_configs().await {
                Ok(configs) => {
                    let mut statuses = Vec::new();
                    for config in configs {
                        let probe_status = tokio::time::timeout(
                            Duration::from_millis(30),
                            mcp_svc.server_manager().get_server_status(&config.id),
                        )
                        .await;
                        let kind = map_mcp_probe_status(probe_status);
                        statuses.push(northhing_kernel_api::settings::MCPServerStatusDto {
                            id: config.id,
                            status: kind,
                        });
                    }
                    statuses
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        // Get skills status from skill registry.
        let skills_status = {
            use crate::agentic::tools::implementations::skills::skill_registry;
            let registry = skill_registry();
            let skills = registry.get_all_skills().await;
            skills
                .into_iter()
                .map(|s| SkillStatusDto {
                    skill_id: s.key,
                    name: s.name,
                    enabled: !s.is_shadowed,
                    status: if s.is_shadowed {
                        "shadowed".to_string()
                    } else {
                        "available".to_string()
                    },
                })
                .collect()
        };

        Ok(InspectorDataDto {
            model_name,
            mcp_status,
            skills_status,
        })
    }

    async fn list_artifacts(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<ArtifactDto>, KernelError> {
        // NEEDS_CONTEXT: artifact storage not yet wired.
        Err(KernelError::Internal("not yet wired: list_artifacts".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::events::{AgenticEvent, ToolEventData};
    use northhing_kernel_api::events::KernelEventDto;
    use northhing_kernel_api::KernelSessionApi;

    fn make_started_event(params: serde_json::Value) -> AgenticEvent {
        AgenticEvent::ToolEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            round_id: "r1".into(),
            tool_event: ToolEventData::Started {
                tool_id: "call-abc".into(),
                tool_name: "Bash".into(),
                params,
                timeout_seconds: None,
            },
        }
    }

    fn make_completed_event(result: serde_json::Value, result_for_assistant: Option<String>) -> AgenticEvent {
        AgenticEvent::ToolEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            round_id: "r1".into(),
            tool_event: ToolEventData::Completed {
                tool_id: "call-abc".into(),
                tool_name: "Bash".into(),
                result,
                result_for_assistant,
                duration_ms: 100,
                queue_wait_ms: None,
                preflight_ms: None,
                confirmation_wait_ms: None,
                execution_ms: None,
            },
        }
    }

    fn make_failed_event(error: String) -> AgenticEvent {
        AgenticEvent::ToolEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            round_id: "r1".into(),
            tool_event: ToolEventData::Failed {
                tool_id: "call-abc".into(),
                tool_name: "Bash".into(),
                error,
                duration_ms: None,
                queue_wait_ms: None,
                preflight_ms: None,
                confirmation_wait_ms: None,
                execution_ms: None,
            },
        }
    }

    fn make_cancelled_event(reason: String) -> AgenticEvent {
        AgenticEvent::ToolEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            round_id: "r1".into(),
            tool_event: ToolEventData::Cancelled {
                tool_id: "call-abc".into(),
                tool_name: "Bash".into(),
                reason,
                duration_ms: None,
                queue_wait_ms: None,
                preflight_ms: None,
                confirmation_wait_ms: None,
                execution_ms: None,
            },
        }
    }

    #[test]
    fn test_first_line_truncated() {
        assert_eq!(first_line_truncated("hello world\nsecond line"), "hello world");
        assert_eq!(first_line_truncated("   spaced  \nmore"), "spaced");
        assert_eq!(first_line_truncated(""), "");
        let long = "x".repeat(200);
        assert_eq!(first_line_truncated(&long).len(), 120);
    }

    #[test]
    fn test_truncate_4000() {
        let long = "y".repeat(5000);
        assert_eq!(truncate_4000(&long).len(), 4000);
        assert_eq!(truncate_4000("short").len(), 5);
    }

    #[test]
    fn test_agentic_event_to_dto_started_summary_from_command() {
        let params = serde_json::json!({"command": "ls -la /tmp", "path": "/other"});
        let event = make_started_event(params);
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(matches!(tc.phase, ToolCallPhase::Started));
        assert!(!tc.summary.is_empty(), "summary should not be empty for command key");
        assert!(tc.summary.starts_with("ls"));
        assert!(tc.detail.is_some());
    }

    #[test]
    fn test_agentic_event_to_dto_started_summary_fallback() {
        let params = serde_json::json!({"unknown_field": "value"});
        let event = make_started_event(params);
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(!tc.summary.is_empty());
    }

    #[test]
    fn test_agentic_event_to_dto_completed_summary_and_detail() {
        let result = serde_json::json!({"output": "done"});
        let event = make_completed_event(result, Some("All good".into()));
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(matches!(tc.phase, ToolCallPhase::Completed));
        assert_eq!(tc.summary, "All good");
        assert!(tc.detail.is_some());
    }

    #[test]
    fn test_agentic_event_to_dto_completed_result_fallback() {
        let result = serde_json::json!({"output": "fallback result"});
        let event = make_completed_event(result, None);
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(tc.summary.contains("output") || tc.summary.contains("fallback"));
    }

    #[test]
    fn test_agentic_event_to_dto_failed_maps_to_completed_phase() {
        let event = make_failed_event("connection refused".into());
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(matches!(tc.phase, ToolCallPhase::Completed), "Failed should map to Completed phase");
        assert!(!tc.summary.is_empty(), "summary should not be empty for Failed");
        assert!(tc.detail.is_some());
    }

    #[test]
    fn test_agentic_event_to_dto_completed_truncation_at_120() {
        let long_result = "x".repeat(200);
        let event = make_completed_event(serde_json::json!(long_result), None);
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(tc.summary.len() <= 120, "summary should be truncated to 120 chars");
    }

    #[test]
    fn test_agentic_event_to_dto_cancelled_summary_with_prefix_truncated_to_120() {
        // Regression: "cancelled: " prefix (11 chars) must not push summary over 120.
        // Total "cancelled: <reason>" capped at 120 chars (not 11 + trunc(reason)).
        let long_reason = "x".repeat(200);
        let event = make_cancelled_event(long_reason);
        let dto = agentic_event_to_dto(&event).unwrap();
        let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
        assert!(tc.summary.starts_with("cancelled:"), "summary should have cancelled prefix");
        assert!(tc.summary.len() <= 120, "summary including prefix must be ≤ 120 chars, got {}", tc.summary.len());
        assert!(tc.detail.is_some());
    }

    // ── Lifecycle tests ─────────────────────────────────────────────────────

    #[test]
    fn test_facade_construction_no_panic() {
        // KernelFacade can be constructed without panicking, even if coordinator
        // has not been set (e.g. desktop calls kernel_facade() before init_core).
        let facade = KernelFacade::new();
        // coordinator() on an un-initialized facade must return Err, not panic.
        assert!(facade.coordinator().is_err());
    }

    #[test]
    fn test_result_methods_return_error_before_init() {
        // Verify that facade methods return KernelError (not panic) when called
        // before init_core has been invoked.
        let facade = kernel_facade();
        // coordinator() returns Err before init — verify it is an error, not panic.
        match facade.coordinator() {
            Ok(_) => panic!("coordinator() should be Err before init_core"),
            Err(KernelError::Internal(_)) => {} // expected
            Err(other) => panic!("expected KernelError::Internal, got {:?}", other),
        }
    }

    // test_idempotent_init_core_fast_path removed: INIT_STATE is a static that persists
    // across parallel test runs making it unreliable. The async Mutex gate and
    // FACADE_READY fast-path are tested via integration.

    #[tokio::test]
    async fn test_subscribe_events_returns_err_before_init() {
        use northhing_kernel_api::KernelEventsApi;
        let facade = KernelFacade::new();
        let callback = Box::new(|_event: KernelEventDto| {});
        let result = facade.subscribe_events(callback).await;
        match result {
            Err(KernelError::Runtime(_)) => {} // expected
            Err(other) => panic!("expected KernelError::Runtime before init, got {:?}", other),
            Ok(_) => panic!("subscribe_events should return Err before init_core"),
        }
    }

    // ── Init gate lifecycle tests ─────────────────────────────────────────────
    // Four scenarios run sequentially in one test to avoid static state interference.
    // State is manually reset at the start of each scenario.

    #[tokio::test]
    async fn test_init_gate_lifecycle_all_scenarios() {
        // Reset globals to a clean state for this test.
        FACADE_READY.store(false, Ordering::SeqCst);
        {
            let mut guard = INIT_STATE.lock().await;
            *guard = InitState::NotStarted;
        }

        // ── Scenario ①: Two concurrent calls — init runs exactly once ─────────
        {
            let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
            let call_count_clone = call_count.clone();

            let fake_init = || async move {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                Ok(())
            };

            let call_count_for_r2 = call_count.clone();
            let (r1, r2) = tokio::join!(
                run_init_gate(fake_init()),
                run_init_gate(async move {
                    let cc = call_count_for_r2;
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    cc.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    Ok(())
                })
            );

            assert!(r1.is_ok(), "first concurrent call should succeed");
            assert!(r2.is_ok(), "second concurrent call should succeed");
            // Only ONE of the two fake_inits actually runs to completion (the winner).
            // The loser either waits on InProgress or retries after reset.
            // Net effect: count should be exactly 1.
            assert_eq!(call_count.load(Ordering::SeqCst), 1,
                "init should run exactly once across concurrent calls");
        }

        // ── Scenario ②: Ready之后再调 — init count does not increase ──────────
        {
            FACADE_READY.store(false, Ordering::SeqCst);
            {
                let mut guard = INIT_STATE.lock().await;
                *guard = InitState::NotStarted;
            }

            let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

            // Clone for r2 before r1 moves the original.
            let call_count_for_r2 = call_count.clone();
            let call_count_for_assert = call_count.clone();

            // First call — succeeds and marks Ready.
            let r1 = run_init_gate(async move {
                let cc = call_count;
                cc.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                Ok(())
            }).await;
            assert!(r1.is_ok(), "first init should succeed");

            // Second call on already-Ready facade — fast path, no re-init.
            let r2 = run_init_gate(async move {
                let cc = call_count_for_r2;
                cc.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }).await;
            assert!(r2.is_ok(), "second call on Ready facade should succeed (idempotent)");
            assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 1,
                "init should not re-run when facade is already Ready");
        }

        // ── Scenario ③: First init fails → state resets → second init succeeds ─
        {
            FACADE_READY.store(false, Ordering::SeqCst);
            {
                let mut guard = INIT_STATE.lock().await;
                *guard = InitState::NotStarted;
            }

            let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

            // Clone for r2 before r1 moves the original.
            let call_count_for_r2 = call_count.clone();
            let call_count_for_assert = call_count.clone();

            // First call — returns error.
            let r1 = run_init_gate(async move {
                let cc = call_count;
                cc.fetch_add(1, Ordering::SeqCst);
                Err(KernelError::Internal("simulated init failure".to_string()))
            }).await;
            assert!(r1.is_err(), "first init should fail");
            assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 1);
            // State should be reset to NotStarted — verify by checking INIT_STATE.
            {
                let guard = INIT_STATE.lock().await;
                assert!(matches!(*guard, InitState::NotStarted),
                    "state should reset to NotStarted after failed init");
            }

            // Second call — now succeeds.
            let r2 = run_init_gate(async move {
                let cc = call_count_for_r2;
                cc.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                Ok(())
            }).await;
            assert!(r2.is_ok(), "retry after failure should succeed");
            assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 2,
                "second (retry) init should actually run");
        }

        // ── Scenario ④: list_sessions returns KernelError before init, not panic ──
        {
            FACADE_READY.store(false, Ordering::SeqCst);
            {
                let mut guard = INIT_STATE.lock().await;
                *guard = InitState::NotStarted;
            }

            // Use a fresh KernelFacade (not the global) so coordinator is not set.
            let facade = KernelFacade::new();
            let result: Result<Vec<SessionSummaryDto>, KernelError> = facade.list_sessions().await;
            match result {
                Err(KernelError::Internal(_)) => {} // expected — not panic
                Err(other) => panic!("expected KernelError::Internal before init, got {:?}", other),
                Ok(_) => panic!("list_sessions should return error before init, not Ok"),
            }
        }
    }
}
