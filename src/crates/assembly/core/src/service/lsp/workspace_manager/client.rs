//! LSP client lifecycle (spawn/connect/start/stop).

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, warn};

use super::workspace::{LspEvent, ServerState, ServerStatus, WorkspaceLspManager};
use crate::infrastructure::events::EventEmitter;

use anyhow::{anyhow, Result};

impl WorkspaceLspManager {
    /// Starts a server (with retries).
    pub(crate) async fn start_server(&self, language: &str) -> Result<()> {
        let notify = Arc::new(tokio::sync::Notify::new());
        {
            let mut locks = self.starting_locks.write().await;
            locks.insert(language.to_string(), notify.clone());
        }

        {
            let mut states = self.server_states.write().await;
            states.insert(
                language.to_string(),
                ServerState {
                    status: ServerStatus::Starting,
                    language: language.to_string(),
                    started_at: None,
                    last_error: None,
                    restart_count: 0,
                    document_count: 0,
                },
            );
        }

        self.emit_server_state_changed(language).await;

        let result = self.start_server_internal(language).await;

        let final_result = match result {
            Ok(_) => {
                {
                    let mut states = self.server_states.write().await;
                    if let Some(state) = states.get_mut(language) {
                        state.status = ServerStatus::Running;
                        state.started_at = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                            Ok(duration) => Some(duration.as_secs()),
                            Err(e) => {
                                warn!(
                                    "Failed to compute LSP server start timestamp: language={}, error={}",
                                    language, e
                                );
                                Some(0)
                            }
                        };
                    }
                    info!("LSP server started: {}", language);
                }

                self.emit_server_state_changed(language).await;

                notify.notify_waiters();

                Ok(())
            }
            Err(e) => {
                {
                    let mut states = self.server_states.write().await;
                    if let Some(state) = states.get_mut(language) {
                        state.status = ServerStatus::Failed;
                        state.last_error = Some(e.to_string());
                    }

                    error!("Failed to start LSP server {}: {}", language, e);
                }

                self.emit_server_state_changed(language).await;

                notify.notify_waiters();

                Err(e)
            }
        };

        {
            let mut locks = self.starting_locks.write().await;
            locks.remove(language);
        }

        final_result
    }

    /// Internal server startup implementation.
    async fn start_server_internal(&self, language: &str) -> Result<()> {
        let language_clone = language.to_string();
        let server_states = self.server_states.clone();
        let workspace_path = self.workspace_path.clone();
        let emitter = self.emitter.clone();

        let crash_callback = Arc::new(move |plugin_id: String| {
            let language = language_clone.clone();
            let states = server_states.clone();
            let workspace = workspace_path.clone();
            let emitter_clone = emitter.clone();

            tokio::spawn(async move {
                error!("LSP server crashed: {} (plugin: {})", language, plugin_id);

                {
                    let mut states = states.write().await;
                    if let Some(state) = states.get_mut(&language) {
                        state.status = ServerStatus::Failed;
                        state.last_error = Some("Server process crashed or became unresponsive".to_string());
                    }
                }

                if let Some(emitter) = emitter_clone.read().await.as_ref() {
                    let error_event = LspEvent::ServerError {
                        workspace_path: workspace.display().to_string(),
                        language: language.clone(),
                        error: "Server process crashed or became unresponsive".to_string(),
                    };
                    if let Ok(event_data) = serde_json::to_value(&error_event) {
                        let _ = emitter.emit("lsp-event", event_data).await;
                    }

                    let state_event = LspEvent::ServerStateChanged {
                        workspace_path: workspace.display().to_string(),
                        language: language.clone(),
                        status: "failed".to_string(),
                        message: Some("Server crashed".to_string()),
                    };
                    if let Ok(event_data) = serde_json::to_value(&state_event) {
                        let _ = emitter.emit("lsp-event", event_data).await;
                    }
                }
            });
        }) as Arc<dyn Fn(String) + Send + Sync>;

        let language_clone2 = language.to_string();
        let indexing_tokens2 = self.indexing_tokens.clone();
        let workspace_path_for_token = self.workspace_path.clone();
        let emitter_for_token = self.emitter.clone();
        let lsp_manager_for_token = self.lsp_manager.clone();

        let token_create_callback = Arc::new(move |token: String| {
            let language = language_clone2.clone();
            let tokens = indexing_tokens2.clone();
            let workspace = workspace_path_for_token.clone();
            let emitter_clone = emitter_for_token.clone();
            let lsp_mgr = lsp_manager_for_token.clone();

            tokio::spawn(async move {
                {
                    let mut tokens_map = tokens.write().await;
                    let lang_tokens = tokens_map.entry(language.clone()).or_insert_with(Vec::new);

                    if !lang_tokens.iter().any(|t| t.token == token) {
                        let now = SystemTime::now();
                        lang_tokens.push(super::workspace::TokenInfo {
                            token: token.clone(),
                            state: super::workspace::TokenState::Created,
                            title: String::new(),
                            created_at: now,
                            last_updated: now,
                        });
                    } else {
                        return;
                    }
                }

                Self::emit_aggregated_progress(tokens, workspace, language, emitter_clone, lsp_mgr).await;
            });
        }) as Arc<dyn Fn(String) + Send + Sync>;

        let language_clone3 = language.to_string();
        let workspace_path3 = self.workspace_path.clone();
        let emitter_for_progress = self.emitter.clone();
        let indexing_tokens3 = self.indexing_tokens.clone();
        let lsp_manager_for_progress = self.lsp_manager.clone();

        let progress_callback = Arc::new(
            move |kind: String, token: String, percentage: Option<u32>, message: String| {
                let language = language_clone3.clone();
                let workspace = workspace_path3.clone();
                let emitter_clone = emitter_for_progress.clone();
                let tokens = indexing_tokens3.clone();
                let lsp_mgr = lsp_manager_for_progress.clone();

                tokio::spawn(async move {
                    {
                        let mut tokens_map = tokens.write().await;
                        if let Some(lang_tokens) = tokens_map.get_mut(&language) {
                            if let Some(token_info) = lang_tokens.iter_mut().find(|t| t.token == token) {
                                token_info.last_updated = SystemTime::now();
                                match kind.as_str() {
                                    "begin" => {
                                        token_info.state = super::workspace::TokenState::InProgress(0);
                                        token_info.title = message.clone();
                                        info!("[{}] Indexing started: {}", language, message);
                                    }
                                    "report" => {
                                        let progress = percentage.unwrap_or(0);
                                        token_info.state = super::workspace::TokenState::InProgress(progress);
                                    }
                                    "end" => {
                                        token_info.state = super::workspace::TokenState::Completed;
                                        info!("[{}] Indexing task completed", language);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    {
                        let mut tokens_map = tokens.write().await;
                        if let Some(lang_tokens) = tokens_map.get_mut(&language) {
                            let now = SystemTime::now();
                            lang_tokens.retain(|t| {
                                if matches!(t.state, super::workspace::TokenState::Created) {
                                    if let Ok(elapsed) = now.duration_since(t.created_at) {
                                        return elapsed.as_secs() <= 5;
                                    }
                                }
                                true
                            });
                        }
                    }

                    Self::emit_aggregated_progress(tokens, workspace, language, emitter_clone, lsp_mgr).await;
                });
            },
        ) as Arc<dyn Fn(String, String, Option<u32>, String) + Send + Sync>;

        let _language_clone4 = language.to_string();
        let workspace_path4 = self.workspace_path.clone();
        let emitter_for_diagnostics = self.emitter.clone();
        let lsp_manager_for_cache = self.lsp_manager.clone();

        let diagnostics_callback = Arc::new(move |uri: String, diagnostics: Vec<serde_json::Value>| {
            let workspace = workspace_path4.clone();
            let emitter_clone = emitter_for_diagnostics.clone();
            let lsp_mgr = lsp_manager_for_cache.clone();

            tokio::spawn(async move {
                {
                    let lsp = lsp_mgr.read().await;
                    lsp.update_diagnostics_cache(uri.clone(), diagnostics.clone()).await;
                }

                let event = LspEvent::Diagnostics {
                    workspace_path: workspace.display().to_string(),
                    uri: uri.clone(),
                    diagnostics: diagnostics.clone(),
                };

                let emitter_guard = emitter_clone.read().await;
                if let Some(emitter) = emitter_guard.as_ref() {
                    debug!("Emitting diagnostics event: uri={}, count={}", uri, diagnostics.len());
                    if let Ok(event_data) = serde_json::to_value(&event) {
                        if let Err(e) = emitter.emit("lsp-event", event_data).await {
                            error!("Failed to emit diagnostics event: {}", e);
                        }
                    }
                }
            });
        }) as Arc<dyn Fn(String, Vec<serde_json::Value>) + Send + Sync>;

        let lsp = self.lsp_manager.read().await;
        lsp.start_server(
            language,
            Some(self.workspace_path.clone()),
            Some(crash_callback),
            Some(progress_callback),
            Some(token_create_callback),
            Some(diagnostics_callback),
        )
        .await
    }

    /// Waits for server startup to complete.
    // reason: wait_for_server_start() is reserved for the upcoming startup-coordination API; today's startup is fire-and-await via individual requests
    pub(crate) async fn wait_for_server_start(&self, language: &str) -> Result<()> {
        let notify = {
            let locks = self.starting_locks.read().await;
            locks.get(language).cloned()
        };

        if let Some(notify) = notify {
            let timeout_duration = Duration::from_secs(60);
            tokio::select! {
                _ = notify.notified() => {

                    let states = self.server_states.read().await;
                    if let Some(state) = states.get(language) {
                        if state.status == ServerStatus::Running {
                            return Ok(());
                        } else {
                            return Err(anyhow!(
                                "Server failed to start: {}",
                                state.last_error.as_deref().unwrap_or("Unknown error")
                            ));
                        }
                    }
                    Err(anyhow!("Server state not found after start"))
                }
                _ = tokio::time::sleep(timeout_duration) => {
                    Err(anyhow!("Server start timeout"))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Pre-starts a server (used during workspace initialization).
    pub async fn prestart_server(&self, language: &str) -> Result<()> {
        info!("Pre-starting LSP server for language: {}", language);

        self.start_server(language).await?;

        Ok(())
    }

    /// Stops a server.
    pub async fn stop_server(&self, language: &str) -> Result<()> {
        info!("Stopping LSP server: {}", language);

        let docs_to_close: Vec<String> = {
            let docs = self.documents.read().await;
            docs.iter()
                .filter(|(_, doc)| doc.language == language)
                .map(|(uri, _)| uri.clone())
                .collect()
        };

        for uri in docs_to_close {
            let _ = self.close_document(uri).await;
        }

        let lsp = self.lsp_manager.read().await;
        lsp.stop_server(language).await?;

        {
            let mut states = self.server_states.write().await;
            states.remove(language);
        }

        self.emit_server_state_changed(language).await;

        info!("LSP server stopped: {}", language);
        Ok(())
    }
}
