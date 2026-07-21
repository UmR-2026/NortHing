//! Episode log types for growth/learning experience storage.

use serde::{Deserialize, Serialize};

/// An episode log entry representing a single dialog turn's execution experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Schema version for future migrations.
    pub schema_version: u32,
    /// Turn identifier.
    pub turn_id: String,
    /// Session this turn belongs to.
    pub session_id: String,
    /// Workspace slug (hashed, filesystem-safe).
    pub workspace_slug: String,
    /// Agent type used for this turn.
    pub agent_type: String,
    /// User input summary (first 120 chars).
    pub task_summary: String,
    /// Tool calls made during this turn with success status.
    pub tools_used: Vec<ToolUseRecord>,
    /// Tool failures recorded during this turn.
    pub failures: Vec<ToolFailureRecord>,
    /// Turn outcome.
    pub outcome: EpisodeOutcome,
    /// Turn duration in milliseconds, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Unix timestamp in milliseconds when this turn was recorded.
    pub ts: u64,
    /// Redline security verdicts (future gate: currently always empty).
    #[serde(default)]
    pub redline_verdicts: Vec<RedlineVerdict>,
}

/// Record of a tool call attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseRecord {
    /// Tool name.
    pub name: String,
    /// Whether the tool call succeeded.
    pub ok: bool,
}

/// Record of a tool call failure with optional repair tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFailureRecord {
    /// Tool name that failed.
    pub tool: String,
    /// First line of the error message (max 200 chars).
    pub error: String,
    /// If the same tool succeeded later in this turn, the summary of the first successful call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<String>,
}

/// Turn completion outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeOutcome {
    /// Turn completed normally.
    Completed,
    /// Turn failed with an error.
    Failed,
    /// Turn was cancelled.
    Cancelled,
}

/// A redline security verdict (future: judge gate integration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedlineVerdict {
    /// Rule identifier (e.g., "I-NEG-1" .. "I-NEG-4").
    pub rule: String,
    /// Verdict status.
    pub status: RedlineStatus,
}

/// Redline verdict status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedlineStatus {
    /// Rule passed.
    Pass,
    /// Rule violated.
    Violation,
    /// Not yet evaluated.
    NotEvaluated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redline_verdicts_default_empty() {
        let ep = Episode {
            schema_version: 1,
            turn_id: "t1".to_string(),
            session_id: "s1".to_string(),
            workspace_slug: "ws-abc".to_string(),
            agent_type: "agentic".to_string(),
            task_summary: "test".to_string(),
            tools_used: vec![],
            failures: vec![],
            outcome: EpisodeOutcome::Completed,
            duration_ms: None,
            ts: 1000,
            redline_verdicts: vec![],
        };
        assert!(ep.redline_verdicts.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let ep = Episode {
            schema_version: 1,
            turn_id: "t1".to_string(),
            session_id: "s1".to_string(),
            workspace_slug: "ws-abc".to_string(),
            agent_type: "agentic".to_string(),
            task_summary: "test task".to_string(),
            tools_used: vec![ToolUseRecord { name: "Bash".to_string(), ok: true }],
            failures: vec![ToolFailureRecord {
                tool: "Read".to_string(),
                error: "file not found".to_string(),
                repair: Some("success after retry".to_string()),
            }],
            outcome: EpisodeOutcome::Completed,
            duration_ms: Some(1500),
            ts: 1000,
            redline_verdicts: vec![],
        };
        let json = serde_json::to_string(&ep).unwrap();
        let decoded: Episode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.turn_id, "t1");
        assert_eq!(decoded.tools_used.len(), 1);
        assert_eq!(decoded.failures.len(), 1);
        assert_eq!(decoded.failures[0].repair, Some("success after retry".to_string()));
    }
}
