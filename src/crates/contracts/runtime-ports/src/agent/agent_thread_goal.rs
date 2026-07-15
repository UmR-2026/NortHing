//! Thread goal constants, types, validation, and continuation plan.

use serde::{Deserialize, Serialize};

// ── Constants ────────────────────────────────────────────────────────────────

/// Legacy session metadata key for the pre-Codex goal mode experiment.
pub const GOAL_MODE_METADATA_KEY: &str = "goal_mode";

/// Persisted thread goal stored in session custom metadata.
pub const THREAD_GOAL_METADATA_KEY: &str = "thread_goal";

pub const MAX_THREAD_GOAL_OBJECTIVE_CHARS: usize = 4_000;

pub const MAX_CONTEXT_SUMMARY_CHARS: usize = 12_000;

/// Max automatic goal continuation dialog turns per objective (legacy goal_mode parity).
pub const MAX_THREAD_GOAL_AUTO_CONTINUATIONS: u32 = 100;

/// Alias retained for migration from legacy `goal_mode` metadata and docs.
pub const MAX_GOAL_CONTINUATIONS: u32 = MAX_THREAD_GOAL_AUTO_CONTINUATIONS;

// ── Thread goal status ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThreadGoalStatus {
    Active,
    Paused,
    Blocked,
    UsageLimited,
    BudgetLimited,
    Complete,
}

impl ThreadGoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Blocked => "blocked",
            Self::UsageLimited => "usageLimited",
            Self::BudgetLimited => "budgetLimited",
            Self::Complete => "complete",
        }
    }
}

pub fn validate_thread_goal_objective(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("goal objective must not be empty".to_string());
    }
    if value.chars().count() > MAX_THREAD_GOAL_OBJECTIVE_CHARS {
        return Err(format!(
            "goal objective must be at most {MAX_THREAD_GOAL_OBJECTIVE_CHARS} characters"
        ));
    }
    Ok(())
}

// ── Thread goal struct ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadGoal {
    pub goal_id: String,
    pub session_id: String,
    pub objective: String,
    pub status: ThreadGoalStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<i64>,
    #[serde(default)]
    pub tokens_used: i64,
    #[serde(default)]
    pub time_used_seconds: i64,
    pub created_at: i64,
    pub updated_at: i64,
    /// Auto-continuation dialog turns scheduled toward this goal (resets on new objective).
    #[serde(default)]
    pub auto_continuation_count: u32,
}

impl ThreadGoal {
    pub fn is_active(&self) -> bool {
        matches!(self.status, ThreadGoalStatus::Active | ThreadGoalStatus::BudgetLimited)
    }

    pub fn remaining_tokens(&self) -> Option<i64> {
        self.token_budget.map(|budget| (budget - self.tokens_used).max(0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetThreadGoalResult {
    pub goal: ThreadGoal,
    pub replaced_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadGoalContinuationPlan {
    pub prepended_reminders: Vec<String>,
    pub display_message: String,
    pub user_message_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadGoalToolResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<ThreadGoal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_budget_report: Option<String>,
}

// ── Thread goal delivery ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentThreadGoalDeliveryKind {
    Resumed,
    ObjectiveUpdated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentThreadGoalDeliveryRequest {
    pub session_id: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    pub kind: AgentThreadGoalDeliveryKind,
    pub goal: ThreadGoal,
}
