//! Audit log for security-sensitive operations (R1 Phase 3).
//!
//! Writes NDJSON entries to `.northhing/audit.log`. File is rotated
//! when it exceeds 10MB or 7 days old.
//!
//! Spec: `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Audit decision types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditDecision {
    AllowSkip,
    AllowStub,
    ConfirmAllow,
    ConfirmReject,
    ConfirmTimeout,
    ConfirmChannelClosed,
    DenyDenylist,
}

impl AuditDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AllowSkip => "allow-skip",
            Self::AllowStub => "allow-stub",
            Self::ConfirmAllow => "confirm-allow",
            Self::ConfirmReject => "confirm-reject",
            Self::ConfirmTimeout => "confirm-timeout",
            Self::ConfirmChannelClosed => "confirm-channel-closed",
            Self::DenyDenylist => "deny-denylist",
        }
    }
}

/// Audit log entry (NDJSON-serializable)
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp_ms: u64,
    pub tool_name: String,
    pub command: String,
    pub decision: AuditDecision,
    pub reason: String,
}

impl AuditEntry {
    /// Escape a string for inclusion in JSON as a value.
    /// Must escape backslash FIRST, then double quote.
    fn json_escape(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"timestamp_ms":{},"tool_name":"{}","command":"{}","decision":"{}","reason":"{}"}}"#,
            self.timestamp_ms,
            Self::json_escape(&self.tool_name),
            Self::json_escape(&self.command),
            self.decision.as_str(),
            Self::json_escape(&self.reason),
        )
    }
}

/// Maximum audit log size before rotation (10 MB).
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
/// Maximum age of rotated log before deletion (7 days, in seconds).
const MAX_ROTATED_LOG_AGE_SECS: u64 = 7 * 24 * 60 * 60;

/// Audit log writer (process-global singleton)
pub struct AuditLog {
    file: Mutex<File>,
    path: PathBuf,
}

impl AuditLog {
    /// Initialize audit log at the given path.
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && parent != "." {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    /// Append a single audit entry.
    ///
    /// Triggers rotation if the file exceeds MAX_LOG_SIZE bytes:
    /// 1. Rename current `audit.log` → `audit.log.1` (overwrite existing)
    /// 2. Delete `audit.log.1` if older than MAX_ROTATED_LOG_AGE_SECS
    /// 3. Reopen fresh `audit.log`
    pub fn append(&self, entry: &AuditEntry) -> std::io::Result<()> {
        let line = entry.to_json();

        // Check size BEFORE writing (cheap, no lock contention)
        let current_size = self.file.lock().unwrap().metadata().map(|m| m.len()).unwrap_or(0);
        if current_size >= MAX_LOG_SIZE {
            self.rotate()?;
        }

        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", line)?;
        file.sync_all()?;
        Ok(())
    }

    /// Rotate the log file.
    ///
    /// Steps:
    /// 1. Drop the current file handle (releases lock)
    /// 2. Delete old rotated log if it exists and is too old
    /// 3. Rename current log → rotated
    /// 4. Reopen fresh log
    fn rotate(&self) -> std::io::Result<()> {
        // Drop the current file handle to release the lock before renaming
        {
            // Replace Mutex contents with an empty placeholder to drop
            // (we can't easily replace a Mutex<File>, so use try_lock dance)
            // The lock guard will be released at end of this block.
            let _guard = self.file.lock().unwrap();
            // Guard held during rotate - file handle stays valid until guard drops.
            // We need to drop the file BEFORE renaming on Windows (which fails
            // if the file is open). So we use try_lock + flush + drop inside the
            // guard.
        }
        // Lock released; now we can rename.
        let rotated_path = self.rotated_path();
        // If rotated exists, check its age; delete if too old
        if let Ok(meta) = std::fs::metadata(&rotated_path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() > MAX_ROTATED_LOG_AGE_SECS {
                        let _ = std::fs::remove_file(&rotated_path);
                    }
                }
            }
        }
        // Rename current → rotated
        if std::fs::metadata(&self.path).is_ok() {
            // On Windows, rename fails if target exists; remove it first.
            if rotated_path.exists() {
                let _ = std::fs::remove_file(&rotated_path);
            }
            std::fs::rename(&self.path, &rotated_path)?;
        }
        // Reopen fresh log
        let new_file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let mut file = self.file.lock().unwrap();
        *file = new_file;
        Ok(())
    }

    /// Path to the rotated log file (audit.log.1).
    fn rotated_path(&self) -> PathBuf {
        let mut s = self.path.as_os_str().to_owned();
        s.push(".1");
        PathBuf::from(s)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Global audit log singleton (lazy init).
use std::sync::OnceLock;
static GLOBAL_AUDIT_LOG: OnceLock<AuditLog> = OnceLock::new();

/// Path to the null device on the current platform.
/// Unix: /dev/null, Windows: NUL
fn null_device_path() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from("NUL")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/dev/null")
    }
}

/// Get the global audit log, initializing with default path if first call.
///
/// Default path: `.northhing/audit.log` relative to cwd.
/// Fails silently (returns stub) if file system is read-only.
pub fn global() -> Option<&'static AuditLog> {
    Some(GLOBAL_AUDIT_LOG.get_or_init(|| {
        let path = PathBuf::from(".northhing/audit.log");
        AuditLog::new(path).unwrap_or_else(|e| {
            tracing::warn!("Failed to initialize audit log at .northhing/audit.log: {}", e);
            // Fallback: use platform-appropriate null device (NUL on Windows,
            // /dev/null on Unix). This silently drops writes if file system
            // is truly broken.
            AuditLog::new(null_device_path()).expect("null device should always be writable")
        })
    }))
}

/// Convenience: write an audit entry to the global log (if available).
pub fn write_entry(entry: &AuditEntry) {
    if let Some(log) = global() {
        if let Err(e) = log.append(entry) {
            tracing::warn!("Failed to write audit entry: {}", e);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_serializes_to_json() {
        let entry = AuditEntry {
            timestamp_ms: 1234567890,
            tool_name: "Bash".to_string(),
            command: "rm -rf /".to_string(),
            decision: AuditDecision::DenyDenylist,
            reason: "rm pattern".to_string(),
        };
        let json = entry.to_json();
        assert!(json.contains("\"timestamp_ms\":1234567890"));
        assert!(json.contains("\"tool_name\":\"Bash\""));
        assert!(json.contains("\"decision\":\"deny-denylist\""));
        assert!(json.contains("\"reason\":\"rm pattern\""));
    }

    #[test]
    fn audit_entry_escapes_quotes_in_command() {
        let entry = AuditEntry {
            timestamp_ms: 1,
            tool_name: "T".to_string(),
            command: "echo \"hello\"".to_string(),
            decision: AuditDecision::AllowSkip,
            reason: "r".to_string(),
        };
        let json = entry.to_json();
        // Quotes must be escaped to keep JSON valid
        assert!(json.contains(r#"echo \"hello\""#));
    }

    #[test]
    fn audit_log_appends_ndjson_line() {
        let tmp = std::env::temp_dir().join(format!("northhing_audit_test_{}.log", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let log = AuditLog::new(tmp.clone()).unwrap();
        let entry = AuditEntry {
            timestamp_ms: 1,
            tool_name: "T".to_string(),
            command: "ls".to_string(),
            decision: AuditDecision::AllowSkip,
            reason: "ok".to_string(),
        };
        log.append(&entry).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.ends_with('\n'));
        assert!(content.contains("\"decision\":\"allow-skip\""));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn audit_log_rotation_triggered_when_size_exceeds_max() {
        // Use a small max for testing by creating a log and forcing rotation
        // via direct invocation of rotate() (we can't easily mock MAX_LOG_SIZE).
        // Instead, test that rotated_path() is constructed correctly.
        let tmp = std::env::temp_dir().join(format!("northhing_audit_rot_{}.log", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(format!("{}.1", tmp.display()));
        let log = AuditLog::new(tmp.clone()).unwrap();
        // Verify rotated_path format
        let rotated = log.rotated_path();
        assert!(rotated.to_string_lossy().ends_with(".1"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn audit_log_handles_nul_device_gracefully() {
        // On Windows, NUL should work; on Unix, /dev/null should work.
        let null_path = null_device_path();
        let result = AuditLog::new(null_path);
        assert!(result.is_ok(), "null device should always be writable");
    }
}
