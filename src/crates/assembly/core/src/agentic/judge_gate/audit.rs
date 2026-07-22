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
    #[cfg(test)]
    if let Some(dir) = AUDIT_DIR_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
    {
        return dir;
    }
    path_manager_arc().user_data_dir().join(AUDIT_DIR)
}

/// Test-only override for the audit directory. Always pair with `TEST_ENV_LOCK`
/// so parallel tests do not race on the override.
#[cfg(test)]
static AUDIT_DIR_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Serializes tests that mutate global test-only overrides (audit dir, etc.).
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Point the audit directory at a test-owned location (`None` restores the
/// production default). Poison-tolerant: overrides are best-effort test state.
#[cfg(test)]
pub(crate) fn set_audit_dir_override_for_tests(dir: Option<PathBuf>) {
    *AUDIT_DIR_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = dir;
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

    let _guard = APPEND_LOCK.lock().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("audit lock poisoned: {:?}", e))
    })?;
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

    let _guard = APPEND_LOCK.lock().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("audit lock poisoned: {:?}", e))
    })?;
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

    fn unique_test_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "northhing-judge-gate-test-{}-{}",
            tag,
            uuid::Uuid::new_v4()
        ))
    }

    fn make_entry(verdict: &str) -> AuditEntry {
        AuditEntry::new(
            ActionKind::PromoteSkillCandidate,
            "sha256:v1:abc123".to_string(),
            "1 traces, 1 fs_diffs, S1".to_string(),
            verdict.to_string(),
            vec![],
            None,
            None,
            Some(42),
        )
    }

    #[test]
    fn append_then_read_back_fields_match() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_test_dir("readback");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let entry = make_entry("approve");
        let entry_id = entry.entry_id.clone();
        append_audit_entry(&entry).expect("append should succeed");

        let content = std::fs::read_to_string(today_audit_path()).expect("audit file should exist");
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("line should be valid JSON");
        assert_eq!(parsed["entry_id"], entry_id);
        assert_eq!(parsed["verdict"], "approve");
        assert_eq!(parsed["action_kind"], "promote_skill_candidate");
        assert_eq!(parsed["subject_digest"], "sha256:v1:abc123");
        assert_eq!(parsed["duration_ms"], 42);

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_governance_override_and_read_back() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_test_dir("governance");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let entry = GovernanceOverrideEntry::new(
            "sha256:v1:gov".to_string(),
            "manual redline amendment".to_string(),
            "user-root-authority".to_string(),
        );
        append_governance_override(&entry).expect("governance append should succeed");

        let content = std::fs::read_to_string(today_audit_path()).expect("audit file should exist");
        let parsed: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).expect("line should be valid JSON");
        assert_eq!(parsed["action_kind"], "governance_override");
        assert_eq!(parsed["operator"], "user-root-authority");
        assert_eq!(parsed["reason"], "manual redline amendment");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn audit_write_failure_returns_err_not_panic() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Point the audit dir at an existing FILE: create_dir_all on a file path fails.
        let file_path = unique_test_dir("blocked");
        std::fs::write(&file_path, b"not a directory").unwrap();
        set_audit_dir_override_for_tests(Some(file_path.clone()));

        let result = append_audit_entry(&make_entry("approve"));
        assert!(result.is_err(), "append to an unwritable audit dir must return Err");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_file(&file_path);
    }

    #[tokio::test]
    async fn concurrent_50_appends_no_lost_lines_all_unique_ids() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_test_dir("concurrent");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let mut handles = Vec::new();
        for _ in 0..50 {
            handles.push(tokio::spawn(async move {
                append_audit_entry(&make_entry("approve"))
            }));
        }
        let results = futures::future::join_all(handles).await;
        for result in results {
            result.expect("append task should not panic").expect("append should succeed");
        }

        let content = std::fs::read_to_string(today_audit_path()).expect("audit file should exist");
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 50, "all 50 appends must be preserved");

        let mut ids = std::collections::HashSet::new();
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).expect("each line must be valid JSON");
            let id = parsed["entry_id"].as_str().expect("entry_id must be a string").to_string();
            assert!(ids.insert(id), "entry_id must be unique per line");
        }
        assert_eq!(ids.len(), 50);

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
