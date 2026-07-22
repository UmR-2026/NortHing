//! Judge runner trait and production implementation.
//!
//! The `JudgeRunner` trait abstracts the execution of judge gate evaluations.
//! Production use `SubagentJudgeRunner` which delegates to the GateJudge subagent
//! via `ConversationCoordinator::execute_subagent`.

use crate::agentic::coordination::{global_coordinator, SubagentExecutionRequest};
use crate::agentic::deep_review_policy::GATE_JUDGE_AGENT_TYPE;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_runtime::judge_gate::GateExecutionContext;
use northhing_runtime_ports::{DelegationPolicy, SubagentContextMode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

/// Error types from judge runner execution.
#[derive(Debug, Clone, PartialEq)]
pub enum JudgeRunError {
    /// Execution timed out.
    Timeout,
    /// Execution was cancelled.
    Cancelled,
    /// Runner is unavailable with a reason.
    Unavailable(String),
}

/// Trait for judge gate execution.
/// Implementations can be production (SubagentJudgeRunner) or test fakes (FakeJudgeRunner).
#[async_trait::async_trait]
pub(crate) trait JudgeRunner: Send + Sync {
    /// Run the judge gate evaluation.
    ///
    /// # Arguments
    /// * `coordinator` - The conversation coordinator (unused for fake runners)
    /// * `brief` - The judge brief string to send to the judge
    /// * `ctx` - The gate execution context
    ///
    /// # Returns
    /// * `Ok(String)` - The judge's raw text response
    /// * `Err(JudgeRunError)` - If execution failed
    async fn run_judge(
        &self,
        coordinator: &Arc<dyn std::any::Any + Send + Sync>,
        brief: String,
        ctx: &GateExecutionContext,
    ) -> Result<String, JudgeRunError>;
}

/// Production judge runner that delegates to the GateJudge subagent.
pub(crate) struct SubagentJudgeRunner;

impl SubagentJudgeRunner {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl JudgeRunner for SubagentJudgeRunner {
    async fn run_judge(
        &self,
        coordinator: &Arc<dyn std::any::Any + Send + Sync>,
        brief: String,
        ctx: &GateExecutionContext,
    ) -> Result<String, JudgeRunError> {
        // Downcast coordinator to ConversationCoordinator
        let coordinator = coordinator
            .downcast_ref::<crate::agentic::coordination::ConversationCoordinator>()
            .ok_or_else(|| {
                JudgeRunError::Unavailable("coordinator downcast failed".to_string())
            })?;

        // Build subagent parent info
        // Per task spec: parent_session_id = "judge-gate" or ctx.parent_session_id,
        // parent_dialog_turn_id = audit_correlation_id or new uuid
        let parent_session_id = ctx.parent_session_id.clone().unwrap_or_else(|| "judge-gate".to_string());
        let parent_dialog_turn_id = ctx
            .audit_correlation_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let parent_info = SubagentParentInfo {
            tool_call_id: format!("judge-gate-{}", uuid::Uuid::new_v4()),
            session_id: parent_session_id.clone(),
            dialog_turn_id: parent_dialog_turn_id.clone(),
        };

        // Build the execution request
        let request = SubagentExecutionRequest {
            task_description: brief,
            context_mode: SubagentContextMode::Fresh,
            subagent_type: Some(GATE_JUDGE_AGENT_TYPE.to_string()),
            workspace_path: ctx.workspace_path.clone(),
            model_id: None,
            subagent_parent_info: parent_info,
            context: HashMap::new(),
            delegation_policy: DelegationPolicy::top_level().spawn_child(),
        };

        let cancel_token: Option<&CancellationToken> = ctx.cancel_token.as_ref();
        let timeout_seconds = ctx.timeout_seconds.or(Some(600));

        debug!(
            "Executing GateJudge subagent, parent_session_id={}, timeout_seconds={:?}",
            parent_session_id, timeout_seconds
        );

        let result = coordinator
            .execute_subagent(request, cancel_token, timeout_seconds, None)
            .await;

        match result {
            Ok(response) => {
                debug!(
                    "GateJudge subagent returned {} chars",
                    response.text.len()
                );
                Ok(response.text)
            }
            Err(NortHingError::Timeout(msg)) => {
                warn!("GateJudge subagent timed out: {}", msg);
                Err(JudgeRunError::Timeout)
            }
            Err(NortHingError::Cancelled(msg)) => {
                warn!("GateJudge subagent cancelled: {}", msg);
                Err(JudgeRunError::Cancelled)
            }
            Err(e) => {
                warn!("GateJudge subagent unavailable: {}", e);
                Err(JudgeRunError::Unavailable(e.to_string()))
            }
        }
    }
}

impl Default for SubagentJudgeRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// FakeJudgeRunner for testing - returns configurable responses.
/// Moved out of #[cfg(test)] so it can be used by sibling test modules.
pub(crate) struct FakeJudgeRunner {
    pub verdict_text: Option<String>,
    pub error: Option<JudgeRunError>,
    pub delay_ms: Option<u64>,
}

impl FakeJudgeRunner {
    pub fn new() -> Self {
        Self {
            verdict_text: None,
            error: None,
            delay_ms: None,
        }
    }

    /// Set the verdict text to return.
    pub fn with_verdict_text(mut self, text: String) -> Self {
        self.verdict_text = Some(text);
        self
    }

    /// Set the error to return.
    pub fn with_error(mut self, error: JudgeRunError) -> Self {
        self.error = Some(error);
        self
    }

    /// Set an artificial delay.
    pub fn with_delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }
}

impl Default for FakeJudgeRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl JudgeRunner for FakeJudgeRunner {
    async fn run_judge(
        &self,
        _coordinator: &Arc<dyn std::any::Any + Send + Sync>,
        _brief: String,
        _ctx: &GateExecutionContext,
    ) -> Result<String, JudgeRunError> {
        if let Some(delay_ms) = self.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        if let Some(ref error) = self.error {
            return Err(error.clone());
        }

        self.verdict_text
            .clone()
            .ok_or_else(|| JudgeRunError::Unavailable("FakeJudgeRunner: no verdict configured".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_judge_runner_returns_configured_verdict() {
        let runner = FakeJudgeRunner::new()
            .with_verdict_text(r#"VERDICT_JSON_BEGIN
{"verdict":"approve","rule_checks":[{"rule":"I-NEG-1","status":"pass"},{"rule":"I-NEG-2","status":"pass"},{"rule":"I-NEG-3","status":"pass"},{"rule":"I-NEG-4","status":"pass"}],"evidence_assessment":"T1, F1","rationale":"All rules pass."}
VERDICT_JSON_END"#.to_string())
            .with_delay_ms(10);

        assert!(runner.verdict_text.is_some());
        assert!(runner.error.is_none());
    }

    #[test]
    fn fake_judge_runner_returns_error() {
        let runner = FakeJudgeRunner::new().with_error(JudgeRunError::Timeout);
        assert!(runner.error.is_some());
        match runner.error {
            Some(JudgeRunError::Timeout) => {}
            _ => panic!("expected Timeout"),
        }
    }
}
