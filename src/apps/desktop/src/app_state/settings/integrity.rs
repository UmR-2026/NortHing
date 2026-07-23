use super::AppSettings;

// ===== Q6/Q7 Session Integrity Validation =====
/// 2026-06-26 (Phase 5): integrity issues detected by
/// `validate_session_integrity`. The UI maps these into banner +
/// inline error messages and the per-session `is-workspace-broken`
/// / `provider-deleted` flags (already in the SessionItem DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIntegrityIssue {
    pub session_id: String,
    /// "provider-deleted" (Q6) or "workspace-removed" (Q7)
    pub kind: String,
    /// The provider_id that was deleted, or the workspace path that
    /// was removed. Empty when not applicable.
    pub related_id: String,
}

impl AppSettings {
    /// Spec Q6/Q7: scan all sessions and detect which ones are now
    /// broken because the provider they referenced was deleted (Q6)
    /// or the workspace they belong to was removed (Q7). The caller
    /// (Rust `app_state::mod.rs`) maps these into UI errors.
    ///
    /// `session_provider_id` and `session_workspace_path` are
    /// closures that read from the session's stored state. We pass
    /// them as closures rather than taking the full `SessionState`
    /// struct so this stays decoupled from the agent-runtime crate's
    /// `Session` type — the only thing we need is "which provider
    /// does this session use" and "which workspace does it belong to".
    ///
    /// Returns one issue per broken session; sessions that are still
    /// healthy produce no issue.
    pub fn validate_session_integrity<I, P, W>(
        &self,
        session_ids: I,
        session_provider_id: P,
        session_workspace_path: W,
    ) -> Vec<SessionIntegrityIssue>
    where
        I: IntoIterator<Item = String>,
        P: Fn(&str) -> Option<String>,
        W: Fn(&str) -> Option<std::path::PathBuf>,
    {
        let known_provider_ids: std::collections::HashSet<&str> =
            self.providers.iter().map(|p| p.id.as_str()).collect();
        let known_workspace_paths: std::collections::HashSet<std::path::PathBuf> =
            self.workspaces.iter().map(|w| w.path.clone()).collect();

        let mut issues = Vec::new();
        for sid in session_ids {
            // Q6: provider referenced by the session is gone.
            if let Some(pid) = session_provider_id(&sid) {
                if !pid.is_empty() && !known_provider_ids.contains(pid.as_str()) {
                    issues.push(SessionIntegrityIssue {
                        session_id: sid.clone(),
                        kind: "provider-deleted".to_string(),
                        related_id: pid,
                    });
                    // A session can be both Q6 and Q7; we still
                    // report both so the UI shows the full picture.
                }
            }
            // Q7: workspace that the session belongs to was removed.
            if let Some(wpath) = session_workspace_path(&sid) {
                if !known_workspace_paths.contains(&wpath) {
                    issues.push(SessionIntegrityIssue {
                        session_id: sid,
                        kind: "workspace-removed".to_string(),
                        related_id: wpath.to_string_lossy().to_string(),
                    });
                }
            }
        }
        issues
    }
}
