use super::session_manager::SessionManager;
use crate::agentic::coordination::global_coordinator;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::types::AIConfig;
use crate::service::config::{subscribe_config_updates, ConfigUpdateEvent};
use std::collections::HashSet;
use tracing::{debug, info, warn};

impl SessionManager {
    /// Decide whether the given session model id is still usable.
    ///
    /// `model_id` is treated as "usable" when:
    /// - it is a special selector (`auto` / `primary` / `fast` / `default` /
    ///   empty) — these are evaluated again at request time against
    ///   `default_models`, so their long-term validity is governed elsewhere;
    /// - it resolves to a model that exists AND is enabled.
    pub(crate) fn is_session_model_id_usable(ai_config: &AIConfig, model_id: &str) -> bool {
        let trimmed = model_id.trim();
        if trimmed.is_empty() || trimmed == "auto" || trimmed == "default" || trimmed == "primary" || trimmed == "fast"
        {
            return true;
        }
        ai_config.is_model_reference_active(trimmed)
    }

    /// Reset every active session whose bound model id is in
    /// `invalidated_model_ids` back to `"auto"`. Persists the change and emits
    /// `AgenticEvent::SessionModelAutoMigrated` for every migrated session so
    /// the UI can refresh its model selector and surface a notice.
    pub(crate) async fn migrate_sessions_off_invalidated_models(
        &self,
        invalidated_model_ids: &[String],
        reason: &'static str,
    ) {
        if invalidated_model_ids.is_empty() {
            return;
        }
        let invalid: HashSet<&str> = invalidated_model_ids.iter().map(String::as_str).collect();

        // Snapshot affected sessions first to avoid holding DashMap iterators
        // across async writes.
        let affected: Vec<(String, String)> = self
            .sessions
            .iter()
            .filter_map(|entry| {
                let session = entry.value();
                let current = session.config.model_id.as_deref()?.trim().to_string();
                if invalid.contains(current.as_str()) {
                    Some((session.session_id.clone(), current))
                } else {
                    None
                }
            })
            .collect();

        if affected.is_empty() {
            return;
        }

        for (session_id, previous_model_id) in affected {
            if let Err(e) = self.update_session_model_id(&session_id, "auto").await {
                warn!(
                    "Failed to auto-migrate session model after reconcile: session_id={}, previous={}, error={}",
                    session_id, previous_model_id, e
                );
                continue;
            }
            info!(
                "Session model auto-migrated to 'auto': session_id={}, previous_model_id={}, reason={}",
                session_id, previous_model_id, reason
            );

            if let Some(coordinator) = global_coordinator() {
                coordinator
                    .emit_session_model_auto_migrated(&session_id, &previous_model_id, "auto", reason)
                    .await;
            }
        }
    }

    /// Best-effort: drop cached AI clients for invalidated models so the next
    /// request rebuilds against the reconciled config.
    pub(crate) async fn invalidate_ai_clients_for_models(invalidated_model_ids: &[String]) {
        if invalidated_model_ids.is_empty() {
            return;
        }
        if let Ok(factory) = get_global_ai_client_factory().await {
            for model_id in invalidated_model_ids {
                factory.invalidate_model(model_id);
            }
        }
    }

    pub(crate) fn spawn_model_reconciliation_listener(&self) {
        let sessions = self.sessions.clone();
        let session_workspace_index = self.session_workspace_index.clone();
        let context_store = self.context_store.clone();
        let prompt_cache_store = self.prompt_cache_store.clone();
        let turn_skill_agent_snapshot_store = self.turn_skill_agent_snapshot_store.clone();
        let skill_agent_baseline_override_snapshot_store = self.skill_agent_baseline_override_snapshot_store.clone();
        let file_read_state_store = self.file_read_state_store.clone();
        let evidence_ledger = self.evidence_ledger.clone();
        let persistence_manager = self.persistence_manager.clone();
        let manager_config = self.config.clone();

        tokio::spawn(async move {
            let Some(mut receiver) = subscribe_config_updates() else {
                debug!(
                    "SessionManager: config update subscription unavailable; skipping model reconciliation listener"
                );
                return;
            };

            // Re-build a thin handle that mirrors `self` for the listener loop.
            // We can't move `self` into a 'static task, so we recreate the
            // surface area we need from the cloned shared fields above.
            let manager = Self {
                sessions,
                session_workspace_index,
                context_store,
                prompt_cache_store,
                turn_skill_agent_snapshot_store,
                skill_agent_baseline_override_snapshot_store,
                file_read_state_store,
                evidence_ledger,
                persistence_manager,
                config: manager_config,
            };

            loop {
                match receiver.recv().await {
                    Ok(ConfigUpdateEvent::ModelsReconciled {
                        invalidated_model_ids, ..
                    }) => {
                        Self::invalidate_ai_clients_for_models(&invalidated_model_ids).await;
                        manager
                            .migrate_sessions_off_invalidated_models(&invalidated_model_ids, "model_reconciled")
                            .await;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("SessionManager model reconciliation listener: channel closed");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(
                            "SessionManager model reconciliation listener lagged by {} events; continuing",
                            n
                        );
                    }
                }
            }
        });
    }
}
