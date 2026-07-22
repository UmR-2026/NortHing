//! Audit log for judge gate operations.
//!
//! Append-only audit trail written to `user_data_dir()/judge-gate/audit-{YYYYMMDD}.jsonl`.
//! This implements the audit requirements from C4 Phase 0 design §6.

use crate::infrastructure::app_paths::path_manager_arc;
use chrono::Utc;
use northhing_agent_runtime::judge_gate::{ActionKind, RuleCheck};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info};

const AUDIT_DIR: &str = "judge-gate";
const AUDIT_FILE_PREFIX: &str = "audit-";
const AUDIT_FILE_SUFFIX: &str = ".jsonl";

/// Kind of action recorded in the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditActionKind {
    PromoteSkillCandidate,
    GovernanceOverride,
}

/// Audit entry written to the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub entry_id: String,
    pub ts: u64,
    pub action_kind: AuditActionKind,
    pub subject_digest: String,
    pub evidence_summary: String,
    pub verdict: String,
    pub rule_checks: Vec<RuleCheck>,
    pub reject_class: Option<String>,
    pub judge_turn_id: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Governance override entry written to the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceOverrideEntry {
    pub entry_id: String,
    pub ts: u64,
    pub action_kind: AuditActionKind,
    pub subject_digest: String,
    pub reason: String,
    pub operator: String,
}

impl AuditEntry {
    /// Create a new audit entry with a generated ID and current timestamp.
    pub fn new(
        action_kind: ActionKind,
        subject_digest: String,
        evidence_summary: String,
        verdict: String,
        rule_checks: Vec<RuleCheck>,
        reject_class: Option<String>,
        judge_turn_id: Option<String>,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            entry_id: uuid::Uuid::new_v4().to_string(),
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            action_kind: match action_kind {
                ActionKind::PromoteSkillCandidate => AuditActionKind::PromoteSkillCandidate,
            },
            subject_digest,
            evidence_summary,
            verdict,
            rule_checks,
            reject_class,
            judge_turn_id,
            duration_ms,
        }
    }

    /// Serialize to a JSON line.
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"entry_id":"{}","error":"serialization_failed:{}"}}"#, self.entry_id, e)
        })
    }
}

impl GovernanceOverrideEntry {
    /// Create a new governance override entry.
    pub fn new(subject_digest: String, reason: String, operator: String) -> Self {
        Self {
            entry_id: uuid::Uuid::new_v4().to_string(),
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            action_kind: AuditActionKind::GovernanceOverride,
            subject_digest,
            reason,
            operator,
        }
    }

    /// Serialize to a JSON line.
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"entry_id":"{}","error":"serialization_failed:{}"}}"#, self.entry_id, e)
        })
    }
}

/// Get the audit directory path per design spec §6:
/// user_data_dir()/judge-gate/
pub(crate) fn audit_dir() -> PathBuf {
    path_manager_arc().user_data_dir().join(AUDIT_DIR)
}

/// Get today's audit file path.
pub(crate) fn today_audit_path() -> PathBuf {
    let date_str = Utc::now().format("%Y%m%d").to_string();
    audit_dir().join(format!("{}{}{}", AUDIT_FILE_PREFIX, date_str, AUDIT_FILE_SUFFIX))
}

/// Ensure the audit directory exists.
fn ensure_audit_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(audit_dir())?;
    Ok(())
}

/// Process-level lock serializing all audit file appends.
static APPEND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Append an audit entry to today's audit file.
/// Returns an error if the write fails (does not panic or log silently).
pub(crate) fn append_audit_entry(entry: &AuditEntry) -> std::io::Result<()> {
    ensure_audit_dir()?;
    let path = today_audit_path();

    let _guard = APPEND_LOCK.lock().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = entry.to_json_line();
    use std::io::Write;
    writeln!(file, "{}", line)?;
    file.flush()?;
    file.sync_all()?;

    info!(
        entry_id = %entry.entry_id,
        action_kind = ?entry.action_kind,
        subject_digest = %entry.subject_digest,
        "audit entry written"
    );

    Ok(())
}

/// Append a governance override entry to today's audit file.
/// Returns an error if the write fails.
pub(crate) fn append_governance_override(entry: &GovernanceOverrideEntry) -> std::io::Result<()> {
    ensure_audit_dir()?;
    let path = today_audit_path();

    let _guard = APPEND_LOCK.lock().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = entry.to_json_line();
    use std::io::Write;
    writeln!(file, "{}", line)?;
    file.flush()?;
    file.sync_all()?;

    info!(
        entry_id = %entry.entry_id,
        subject_digest = %entry.subject_digest,
        "governance override audit entry written"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_agent_runtime::judge_gate::{RuleCheck, RuleStatus};
    use std::collections::HashMap;

    fn with_temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("northhing_audit_test_{}", std::process::id()))
    }

    #[test]
    fn audit_entry_serialization() {
        let entry = AuditEntry::new(
            ActionKind::PromoteSkillCandidate,
            "sha256:v1:abc123".to_string(),
            "3 traces, 2 fs_diffs, S1, H1".to_string(),
            "approve".to_string(),
            vec![
                RuleCheck {
                    rule: "I-NEG-1".to_string(),
                    status: RuleStatus::Pass,
                },
                RuleCheck {
                    rule: "I-NEG-2".to_string(),
                    status: RuleStatus::Pass,
                },
            ],
            None,
            None,
            Some(1500),
        );

        let json = entry.to_json_line();
        assert!(json.contains("\"verdict\":\"approve\""));
        assert!(json.contains("\"action_kind\":\"promote_skill_candidate\""));
        assert!(json.contains("\"subject_digest\":\"sha256:v1:abc123\""));
        assert!(!json.is_empty());
    }

    #[test]
    fn governance_override_entry_serialization() {
        let entry = GovernanceOverrideEntry::new(
            "sha256:v1:def456".to_string(),
            "Emergency promotion for security patch".to_string(),
            "admin".to_string(),
        );

        let json = entry.to_json_line();
        assert!(json.contains("\"action_kind\":\"governance_override\""));
        assert!(json.contains("\"operator\":\"admin\""));
        assert!(!json.is_empty());
    }

    #[test]
    fn audit_dir_uses_user_data_dir() {
        let dir = audit_dir();
        assert!(dir.to_string_lossy().contains("judge-gate"));
    }

    #[test]
    fn today_audit_path_format() {
        let path = today_audit_path();
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.starts_with(AUDIT_FILE_PREFIX));
        assert!(filename.ends_with(AUDIT_FILE_SUFFIX));
        // Date format: YYYYMMDD
        assert!(regex::Regex::new(r"audit-\d{8}\.jsonl")
            .unwrap()
            .is_match(&filename));
    }
}
