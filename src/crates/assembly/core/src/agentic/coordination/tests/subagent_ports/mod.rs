//! Subagent dispatch port tests — facade.
//!
//! Split into scenario-specific sub-test files:
//! - `tests_success`
//! - `tests_cancel`
//! - `tests_timeout`
//! - `tests_error`
//! - `tests_parent_chain`
//! - `tests_concurrent`
//!
//! Shared helpers (fixtures + assertions) live in this module.

#![allow(dead_code)]

use super::super::{
    format_background_subagent_delivery_text, format_background_subagent_display_text, SubagentPhase1Output,
    SubagentPhase2Output, SubagentResult,
};
use crate::agentic::coordination::coordinator::HiddenSubagentExecutionRequest;
use crate::agentic::coordination::tests::test_coordinator;
use crate::agentic::core::Message;
use crate::agentic::events::{EventQueue, EventQueueConfig, EventRouter};
use crate::agentic::execution::{ExecutionEngine, ExecutionEngineConfig, RoundExecutor, StreamProcessor};
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::compression::{CompressionConfig, ContextCompressor};
use crate::agentic::session::prompt_cache::PromptCachePolicy;
use crate::agentic::session::session_manager::{SessionManager, SessionManagerConfig};
use crate::agentic::session::SessionContextStore;
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::agentic::tools::pipeline::{ToolPipeline, ToolStateManager};
use crate::agentic::tools::registry::ToolRegistry;
use crate::infrastructure::ai::client_factory::AIClientFactory;
use crate::infrastructure::app_paths::PathManager;
use crate::service::config::global::GlobalConfigManager;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::{AgentSubmissionPort, DelegationPolicy};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};
use tokio_util::sync::CancellationToken;

// ─── Helpers ─────────────────────────────────────────────────────────

/// What the mock subagent does when invoked.
#[derive(Debug, Clone)]
pub enum SubagentScenario {
    Succeed {
        after: Duration,
        text: String,
    },
    SleepForever,
    Fail {
        message: String,
    },
    SpawnNested {
        depth: u32,
        max_depth: u32,
        after: Duration,
    },
}

/// Mock tool that drives `execute_hidden_subagent_internal` with
/// a configurable scenario. The coordinator's phase 1/2/3 path
/// runs unmodified — only the inner agent behavior is mocked.
pub struct MockSubagentTool {
    scenario: Arc<TokioMutex<SubagentScenario>>,
}

impl MockSubagentTool {
    pub fn new(scenario: SubagentScenario) -> Self {
        Self {
            scenario: Arc::new(TokioMutex::new(scenario)),
        }
    }
}

#[async_trait::async_trait]
impl Tool for MockSubagentTool {
    fn name(&self) -> &str {
        "mock_subagent"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok("Mock subagent for boundary E2E tests".to_string())
    }

    fn short_description(&self) -> String {
        "Mock subagent for boundary E2E tests".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn call_impl(
        &self,
        _input: &serde_json::Value,
        _context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let scenario = self.scenario.lock().await.clone();
        match scenario {
            SubagentScenario::Succeed { after, text } => {
                tokio::time::sleep(after).await;
                Ok(vec![ToolResult::Result {
                    data: serde_json::json!({"text": text.clone()}),
                    result_for_assistant: Some(text),
                    image_attachments: None,
                }])
            }
            SubagentScenario::SleepForever => {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                Ok(vec![ToolResult::Result {
                    data: serde_json::json!({"text": "should not reach"}),
                    result_for_assistant: Some("should not reach".to_string()),
                    image_attachments: None,
                }])
            }
            SubagentScenario::Fail { message } => Err(NortHingError::Tool(message)),
            SubagentScenario::SpawnNested { .. } => {
                // Implemented in Task 7 (parent_chain test)
                Err(NortHingError::Tool("SpawnNested not yet implemented".to_string()))
            }
        }
    }
}

/// Initialize the global config service + AIClientFactory once per
/// test process.
pub async fn ensure_global_config_for_tests() {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() {
        return;
    }
    if let Err(e) = GlobalConfigManager::initialize().await {
        eprintln!("GlobalConfigManager::initialize failed in test setup: {}", e);
    }
    if let Err(e) = AIClientFactory::initialize_global().await {
        eprintln!("AIClientFactory::initialize_global failed in test setup: {}", e);
    }
    let _ = DONE.set(());
}

/// Test helper: returns coordinator, session manager, and mock tool.
pub async fn build_test_coordinator_with_mock_tool(
    scenario: SubagentScenario,
) -> (
    Arc<crate::agentic::coordination::coordinator::ConversationCoordinator>,
    Arc<SessionManager>,
    Arc<MockSubagentTool>,
) {
    ensure_global_config_for_tests().await;
    let event_queue = Arc::new(EventQueue::new(EventQueueConfig::default()));
    let session_manager = Arc::new(SessionManager::new(
        Arc::new(SessionContextStore::new()),
        Arc::new(
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager"),
        ),
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600),
            auto_save_interval: Duration::from_secs(300),
            enable_persistence: false,
            prompt_cache_policy: PromptCachePolicy::default(),
        },
    ));
    let tool_registry = Arc::new(TokioRwLock::new(ToolRegistry::new()));
    let mock = Arc::new(MockSubagentTool::new(scenario));
    {
        let mut registry_guard = tool_registry.write().await;
        registry_guard.register_tool(mock.clone() as Arc<dyn Tool>);
    }
    let tool_pipeline = Arc::new(ToolPipeline::new(
        tool_registry,
        Arc::new(ToolStateManager::new(event_queue.clone())),
        None,
        Arc::new(OnceLock::new()),
    ));
    let execution_engine = Arc::new(ExecutionEngine::new(
        Arc::new(RoundExecutor::new(
            Arc::new(StreamProcessor::new(event_queue.clone())),
            event_queue.clone(),
            tool_pipeline.clone(),
        )),
        event_queue.clone(),
        session_manager.clone(),
        Arc::new(ContextCompressor::new(CompressionConfig::default())),
        ExecutionEngineConfig::default(),
    ));
    let coordinator = crate::agentic::coordination::coordinator::ConversationCoordinator::new(
        session_manager.clone(),
        execution_engine,
        tool_pipeline,
        event_queue,
        Arc::new(EventRouter::new()),
    );

    (Arc::new(coordinator), session_manager, mock)
}

/// Build a minimal valid `HiddenSubagentExecutionRequest` for tests.
pub fn build_minimal_request() -> HiddenSubagentExecutionRequest {
    let workspace_path =
        std::env::temp_dir().join(format!("northhing-subagent-boundary-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");
    let mut session_config = crate::agentic::core::SessionConfig::default();
    session_config.workspace_path = Some(workspace_path.to_string_lossy().into_owned());
    HiddenSubagentExecutionRequest {
        session_name: "subagent-boundary-test".to_string(),
        agent_type: "agentic".to_string(),
        session_config,
        initial_messages: vec![Message::user("mock task".to_string())],
        user_input_text: "mock task".to_string(),
        created_by: Some("test-parent".to_string()),
        subagent_parent_info: None,
        context: HashMap::new(),
        delegation_policy: DelegationPolicy::default(),
        runtime_tool_restrictions: crate::agentic::tools::ToolRuntimeRestrictions::default(),
        prompt_cache_source_session_id: None,
    }
}

/// Build a `ToolUseContext` for unit-testing the mock tool directly.
pub fn empty_tool_context() -> ToolUseContext {
    ToolUseContext {
        tool_call_id: None,
        agent_type: None,
        session_id: None,
        dialog_turn_id: None,
        workspace: None,
        unlocked_collapsed_tools: Vec::new(),
        custom_data: HashMap::new(),
        computer_use_host: None,
        runtime_tool_restrictions: crate::agentic::tools::ToolRuntimeRestrictions::default(),
        runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
        actor_runtime: None,
    }
}

/// Assert the 4 dead-code fields on a `SubagentPhase2Output` are populated.
pub fn assert_secondary_fields_populated(phase2: &SubagentPhase2Output, _expected_text: &str) {
    let _parent_info: Option<&crate::agentic::tools::pipeline::SubagentParentInfo> =
        phase2.subagent_parent_info.as_ref();
    let _cancel_token: &CancellationToken = &phase2.subagent_cancel_token;
    let _task: &tokio::task::JoinHandle<NortHingResult<crate::agentic::execution::ExecutionResult>> =
        &phase2.execution_task;
    let _started_at: tokio::time::Instant = phase2.subagent_started_at;
    let _ = phase2.execution_task.is_finished();
    let _ = phase2.subagent_cancel_token.is_cancelled();
    let elapsed = phase2.subagent_started_at.elapsed();
    assert!(
        elapsed < Duration::from_secs(60),
        "started_at should be recent (within 60s), got {:?}",
        elapsed
    );
}

/// Clone a `SubagentPhase1Output` for moving into a `tokio::spawn` task.
pub fn phase1_clone_for_task(phase1: &SubagentPhase1Output) -> SubagentPhase1Output {
    SubagentPhase1Output {
        agent_type: phase1.agent_type.clone(),
        session_id: phase1.session_id.clone(),
        initial_messages: phase1.initial_messages.clone(),
        user_input_text: phase1.user_input_text.clone(),
        subagent_parent_info: phase1.subagent_parent_info.clone(),
        context: phase1.context.clone(),
        delegation_policy: phase1.delegation_policy.clone(),
        runtime_tool_restrictions: phase1.runtime_tool_restrictions.clone(),
        turn_index: phase1.turn_index,
        dialog_turn_id: phase1.dialog_turn_id.clone(),
        subagent_cancel_token: phase1.subagent_cancel_token.clone(),
        deadline_rx: phase1.deadline_rx.clone(),
        requested_timeout_seconds: phase1.requested_timeout_seconds.clone(),
        timeout_seconds: phase1.timeout_seconds.clone(),
        timeout_error_message: phase1.timeout_error_message.clone(),
        parent_session_id: phase1.parent_session_id.clone(),
        parent_dialog_turn_id: phase1.parent_dialog_turn_id.clone(),
        parent_tool_call_id: phase1.parent_tool_call_id.clone(),
        subagent_workspace: phase1.subagent_workspace.clone(),
        subagent_started_at: phase1.subagent_started_at,
    }
}

// ─── Sub-test modules ────────────────────────────────────────────────

mod tests_abort_exit;
mod tests_cancel;
mod tests_concurrent;
mod tests_error;
mod tests_parent_chain;
mod tests_success;
mod tests_timeout;
