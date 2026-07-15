//! Response formatting, event emission, health checks, and progress aggregation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use super::super::config_watcher::ConfigWatcher;
use super::super::manager::LspManager;
use super::workspace::{ServerStatus, TokenInfo, TokenState, WorkspaceLspManager};
use crate::infrastructure::events::EventEmitter;

use anyhow::Result;
use serde_json;

impl WorkspaceLspManager {
    /// Sends an aggregated overall progress event.
    pub(crate) async fn emit_aggregated_progress(
        tokens: Arc<RwLock<HashMap<String, Vec<TokenInfo>>>>,
        workspace: std::path::PathBuf,
        language: String,
        emitter: Arc<RwLock<Option<Arc<dyn EventEmitter>>>>,
        lsp_manager: Arc<RwLock<LspManager>>,
    ) {
        let tokens_map = tokens.read().await;

        if let Some(lang_tokens) = tokens_map.get(&language) {
            if lang_tokens.is_empty() {
                return;
            }

            let active_tokens: Vec<_> = lang_tokens
                .iter()
                .filter(|t| !matches!(t.state, TokenState::Created))
                .collect();

            if active_tokens.is_empty() {
                return;
            }

            let total = active_tokens.len();
            let completed = active_tokens
                .iter()
                .filter(|t| matches!(t.state, TokenState::Completed))
                .count();
            let in_progress_tokens: Vec<_> = active_tokens
                .iter()
                .filter(|t| matches!(t.state, TokenState::InProgress(_)))
                .collect();

            let progress_sum: u32 = active_tokens
                .iter()
                .map(|t| match t.state {
                    TokenState::Created => 0,
                    TokenState::InProgress(p) => p,
                    TokenState::Completed => 100,
                })
                .sum();

            let overall_progress = if total > 0 { progress_sum / total as u32 } else { 0 };

            let message = if completed == total {
                format!("Indexing completed ({} tasks)", total)
            } else if let Some(active) = in_progress_tokens.first() {
                let title = if active.title.is_empty() { "..." } else { &active.title };
                format!("{} ({}/{})", title, completed, total)
            } else {
                format!("Indexing ({}/{})", completed, total)
            };

            let plugin_name = {
                let lsp_mgr = lsp_manager.read().await;
                lsp_mgr
                    .find_plugin_by_language(&language)
                    .await
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| language.clone())
            };

            let is_completed = completed == total && total > 0;
            if let Some(emit) = emitter.read().await.as_ref() {
                let progress_event = super::workspace::LspEvent::IndexingProgress {
                    workspace_path: workspace.display().to_string(),
                    language: language.clone(),
                    plugin_name: plugin_name.clone(),
                    progress: overall_progress,
                    message: message.clone(),
                };
                if let Ok(event_data) = serde_json::to_value(&progress_event) {
                    let _ = emit.emit("lsp-event", event_data).await;
                }

                if is_completed {
                    info!("[{}] Indexing completed", language);
                    let complete_event = super::workspace::LspEvent::IndexingComplete {
                        workspace_path: workspace.display().to_string(),
                        language: language.clone(),
                        plugin_name: plugin_name.clone(),
                    };
                    if let Ok(event_data) = serde_json::to_value(&complete_event) {
                        let _ = emit.emit("lsp-event", event_data).await;
                    }
                }
            }

            if is_completed {
                drop(tokens_map);
                let mut tokens_map_mut = tokens.write().await;
                if let Some(lang_tokens) = tokens_map_mut.get_mut(&language) {
                    lang_tokens.clear();
                }
            }
        }
    }

    /// Starts health checks.
    pub(crate) async fn start_health_check(&self) {
        let server_states = self.server_states.clone();
        let lsp_manager = self.lsp_manager.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let languages: Vec<String> = {
                    let states = server_states.read().await;
                    states
                        .iter()
                        .filter(|(_, state)| state.status == ServerStatus::Running)
                        .map(|(lang, _)| lang.clone())
                        .collect()
                };

                for language in languages {
                    let states = server_states.read().await;
                    let needs_check = states
                        .get(&language)
                        .map(|s| matches!(s.status, ServerStatus::Running))
                        .unwrap_or(false);
                    drop(states);

                    if needs_check {
                        let lsp = lsp_manager.read().await;
                        let is_alive = lsp.is_server_alive(&language).await;
                        drop(lsp);

                        if !is_alive {
                            error!("Health check detected dead process: {}", language);

                            let mut states = server_states.write().await;
                            if let Some(state) = states.get_mut(&language) {
                                state.status = ServerStatus::Failed;
                                state.last_error = Some("Server process died unexpectedly".to_string());
                            }
                            drop(states);

                            let lsp = lsp_manager.read().await;
                            if let Err(e) = lsp.stop_server(&language).await {
                                warn!("Failed to cleanup dead server {}: {}", language, e);
                            }
                        }
                    }
                }
            }
        });

        let mut handle_lock = self.health_check_handle.write().await;
        *handle_lock = Some(handle);
    }

    /// Starts the config file watcher (internal; requires `Arc<Self>`).
    pub(crate) fn start_config_watcher_internal(self: &Arc<Self>) {
        let workspace_path = self.workspace_path.clone();
        let manager_weak = Arc::downgrade(self);

        let on_config_changed = Arc::new(move |language: String, _config_file: String| {
            if let Some(manager) = manager_weak.upgrade() {
                info!("Config file changed for {}, scheduling server restart", language);

                let manager_clone = manager.clone();
                let language_clone = language.clone();
                tokio::spawn(async move {
                    info!("Restarting {} server due to config change", language_clone);

                    if let Err(e) = manager_clone.stop_server(&language_clone).await {
                        warn!("Failed to stop {} server: {}", language_clone, e);
                        return;
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;

                    if let Err(e) = manager_clone.start_server(&language_clone).await {
                        error!("Failed to restart {} server: {}", language_clone, e);
                    } else {
                        info!("{} server restarted successfully", language_clone);

                        manager_clone
                            .emit_event(super::workspace::LspEvent::ServerStateChanged {
                                workspace_path: manager_clone.workspace_path.display().to_string(),
                                language: language_clone,
                                status: "running".to_string(),
                                message: Some("Config file updated, server restarted".to_string()),
                            })
                            .await;
                    }
                });
            }
        });

        let config_watcher = self.config_watcher.clone();
        let workspace_path_clone = workspace_path.clone();
        tokio::spawn(async move {
            match ConfigWatcher::new(workspace_path_clone, on_config_changed) {
                Ok(watcher) => {
                    let mut config_watcher_lock = config_watcher.write().await;
                    *config_watcher_lock = Some(watcher);
                }
                Err(e) => {
                    warn!("Failed to start config file watcher: {}", e);
                }
            }
        });
    }
}
