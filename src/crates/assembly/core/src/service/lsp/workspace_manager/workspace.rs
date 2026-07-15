//! Workspace state sync, types, and core lifecycle.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

use super::super::config_watcher::ConfigWatcher;
use super::super::manager::LspManager;
use super::super::project_detector::{ProjectDetector, ProjectInfo};
use crate::infrastructure::events::EventEmitter;

/// LSP event types (pushed to the frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LspEvent {
    /// Server state changed.
    ServerStateChanged {
        workspace_path: String,
        language: String,
        status: String,
        message: Option<String>,
    },
    /// Document opened.
    DocumentOpened {
        workspace_path: String,
        uri: String,
        language: String,
    },
    /// Document closed.
    DocumentClosed { workspace_path: String, uri: String },
    /// Workspace opened.
    WorkspaceOpened { workspace_path: String },
    /// Workspace closed.
    WorkspaceClosed { workspace_path: String },
    /// Server error.
    ServerError {
        workspace_path: String,
        language: String,
        error: String,
    },
    /// Project detection completed.
    ProjectDetected {
        workspace_path: String,
        project_info: ProjectInfo,
    },
    /// Indexing progress updated.
    IndexingProgress {
        workspace_path: String,
        language: String,
        plugin_name: String,
        progress: u32,
        message: String,
    },
    /// Indexing completed.
    IndexingComplete {
        workspace_path: String,
        language: String,
        plugin_name: String,
    },
    /// Diagnostics (errors, warnings, etc.).
    Diagnostics {
        workspace_path: String,
        uri: String,
        diagnostics: Vec<serde_json::Value>,
    },
}

/// Server status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ServerStatus {
    /// Stopped.
    Stopped,
    /// Starting.
    Starting,
    /// Running.
    Running,
    /// Failed.
    Failed,
    /// Restarting.
    Restarting,
}

/// Server state details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerState {
    /// Status.
    pub status: ServerStatus,
    /// Language identifier.
    pub language: String,
    /// Start time.
    pub started_at: Option<u64>,
    /// Last error message.
    pub last_error: Option<String>,
    /// Restart count.
    pub restart_count: u32,
    /// Open document count.
    pub document_count: usize,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            status: ServerStatus::Stopped,
            language: String::new(),
            started_at: None,
            last_error: None,
            restart_count: 0,
            document_count: 0,
        }
    }
}

/// Document state.
#[derive(Debug, Clone)]
pub(crate) struct DocumentState {
    // reason: uri is reserved for the upcoming document-diagnostics API (today's diagnostics are looked up by content hash)
    pub(crate) uri: String,
    pub(crate) language: String,
    pub(crate) version: i32,
    // reason: opened_at is reserved for the upcoming staleness-check surface (today's version increments cover staleness)
    pub(crate) opened_at: SystemTime,
}

/// Token state.
#[derive(Debug, Clone)]
pub(crate) enum TokenState {
    /// Created but not started.
    Created,
    /// In progress (includes percentage).
    InProgress(u32),
    /// Completed.
    Completed,
}

/// Token tracking info.
#[derive(Debug, Clone)]
pub(crate) struct TokenInfo {
    /// Token identifier.
    pub(crate) token: String,
    /// Token state.
    pub(crate) state: TokenState,
    /// Token title/description.
    pub(crate) title: String,
    /// Created time.
    pub(crate) created_at: SystemTime,
    /// Last updated time.
    pub(crate) last_updated: SystemTime,
}

/// Workspace LSP manager.
pub struct WorkspaceLspManager {
    /// Workspace path.
    pub(crate) workspace_path: PathBuf,
    /// LSP manager handle.
    pub(crate) lsp_manager: Arc<RwLock<LspManager>>,
    /// Server states.
    pub(crate) server_states: Arc<RwLock<HashMap<String, ServerState>>>,
    /// Document states.
    pub(crate) documents: Arc<RwLock<HashMap<String, DocumentState>>>,
    /// Startup synchronization locks (prevents duplicate starts).
    pub(crate) starting_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Notify>>>>,
    /// Health check task handle.
    pub(crate) health_check_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// `EventEmitter` used to emit events to the frontend.
    pub(crate) emitter: Arc<RwLock<Option<Arc<dyn EventEmitter>>>>,
    /// Configuration file watcher.
    pub(crate) config_watcher: Arc<RwLock<Option<ConfigWatcher>>>,
    /// Indexing token tracking (`language -> token list`).
    pub(crate) indexing_tokens: Arc<RwLock<HashMap<String, Vec<TokenInfo>>>>,
    /// Workspace initialization complete flag (project detection + pre-start finished).
    pub(crate) workspace_initialized: Arc<tokio::sync::RwLock<bool>>,
}

impl WorkspaceLspManager {
    /// Creates a new workspace manager.
    pub async fn new(workspace_path: PathBuf, lsp_manager: Arc<RwLock<LspManager>>) -> Arc<Self> {
        let manager = Arc::new(Self {
            workspace_path: workspace_path.clone(),
            lsp_manager,
            server_states: Arc::new(RwLock::new(HashMap::new())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            starting_locks: Arc::new(RwLock::new(HashMap::new())),
            health_check_handle: Arc::new(RwLock::new(None)),
            emitter: Arc::new(RwLock::new(None)),
            config_watcher: Arc::new(RwLock::new(None)),
            indexing_tokens: Arc::new(RwLock::new(HashMap::new())),
            workspace_initialized: Arc::new(tokio::sync::RwLock::new(false)),
        });

        manager.initialize().await;

        manager.start_config_watcher_internal();

        let manager_clone = manager.clone();
        let workspace_path_clone = workspace_path.clone();
        tokio::spawn(async move {
            manager_clone.detect_and_prestart(workspace_path_clone).await;
        });

        manager
    }

    /// Detects project type and pre-starts servers.
    async fn detect_and_prestart(&self, workspace_path: PathBuf) {
        debug!("Starting project detection and prestart for: {:?}", workspace_path);

        match ProjectDetector::detect(&workspace_path).await {
            Ok(project_info) => {
                info!("Project detected: languages={:?}", project_info.languages);

                self.emit_event(LspEvent::ProjectDetected {
                    workspace_path: workspace_path.display().to_string(),
                    project_info: project_info.clone(),
                })
                .await;

                let languages_to_start = ProjectDetector::should_prestart(&project_info);

                if !languages_to_start.is_empty() {
                    info!("Pre-starting language servers: {:?}", languages_to_start);

                    for language in languages_to_start {
                        if let Err(e) = self.prestart_server(&language).await {
                            warn!("Failed to prestart {} server: {}", language, e);
                        }
                    }
                } else {
                    debug!("Large project detected, using on-demand loading");
                }
            }
            Err(e) => {
                warn!("Failed to detect project type: {}", e);
            }
        }

        {
            let mut initialized = self.workspace_initialized.write().await;
            *initialized = true;
        }
    }

    /// Sets an `EventEmitter` to enable event emission.
    pub async fn set_emitter(&self, emitter: Arc<dyn EventEmitter>) {
        let mut e = self.emitter.write().await;
        *e = Some(emitter);
    }

    /// Emits an LSP event to the frontend.
    pub(crate) async fn emit_event(&self, event: LspEvent) {
        if let Some(emitter) = self.emitter.read().await.as_ref() {
            let event_data = match serde_json::to_value(&event) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to serialize LSP event: {}", e);
                    return;
                }
            };
            if let Err(e) = emitter.emit("lsp-event", event_data).await {
                error!("Failed to emit LSP event: {}", e);
            }
        }
    }

    /// Initializes the workspace.
    async fn initialize(&self) {
        self.emit_event(LspEvent::WorkspaceOpened {
            workspace_path: self.workspace_path.display().to_string(),
        })
        .await;

        self.start_health_check().await;
    }

    /// Opens a document (auto-starts the server).
    pub async fn open_document(&self, uri: String, language: String, content: String) -> Result<()> {
        {
            let docs = self.documents.read().await;
            if docs.contains_key(&uri) {
                debug!("Document already open: {}", uri);
                return Ok(());
            }
        }

        let server_language = match self.get_running_server_for_language(&language).await {
            Some(lang) => lang,
            None => {
                trace!("LSP server not running for language: {}, skipping didOpen", language);
                return Ok(());
            }
        };

        let lsp = self.lsp_manager.read().await;
        lsp.did_open(&server_language, &uri, &content).await.map_err(|e| {
            error!("Failed to send didOpen: {}", e);
            e
        })?;

        {
            let mut docs = self.documents.write().await;
            docs.insert(
                uri.clone(),
                DocumentState {
                    uri: uri.clone(),
                    language: language.clone(),
                    version: 0,
                    opened_at: SystemTime::now(),
                },
            );
        }

        self.update_server_document_count(&language).await;

        self.emit_event(LspEvent::DocumentOpened {
            workspace_path: self.workspace_path.display().to_string(),
            uri: uri.clone(),
            language: language.clone(),
        })
        .await;

        Ok(())
    }

    /// Updates a document.
    pub async fn change_document(&self, uri: String, content: String) -> Result<()> {
        let (language, version) = {
            let mut docs = self.documents.write().await;
            let doc = docs
                .get_mut(&uri)
                .ok_or_else(|| anyhow!("Document not open: {}", uri))?;

            doc.version += 1;
            (doc.language.clone(), doc.version)
        };

        let server_language = self.get_server_language(&language).await;

        let lsp = self.lsp_manager.read().await;
        lsp.did_change(&server_language, &uri, version, &content).await?;

        Ok(())
    }

    /// Saves a document.
    pub async fn save_document(&self, uri: String) -> Result<()> {
        let language = {
            let docs = self.documents.read().await;
            let doc = docs.get(&uri).ok_or_else(|| anyhow!("Document not open: {}", uri))?;
            doc.language.clone()
        };

        let server_language = self.get_server_language(&language).await;

        let lsp = self.lsp_manager.read().await;
        lsp.did_save(&server_language, &uri).await?;

        Ok(())
    }

    /// Closes a document.
    pub async fn close_document(&self, uri: String) -> Result<()> {
        let language = {
            let mut docs = self.documents.write().await;
            let doc = docs.remove(&uri).ok_or_else(|| anyhow!("Document not open: {}", uri))?;
            doc.language.clone()
        };

        let server_language = self.get_server_language(&language).await;

        let lsp = self.lsp_manager.read().await;
        lsp.did_close(&server_language, &uri).await?;

        self.update_server_document_count(&server_language).await;

        Ok(())
    }

    /// Returns whether a document is open (used by `LspFileSync`).
    pub async fn is_document_opened(&self, uri: &str) -> bool {
        let docs = self.documents.read().await;
        docs.contains_key(uri)
    }

    /// Quickly checks whether a server is running (does not trigger query or startup).
    /// Returns the actual running server language key (may differ from the requested language).
    pub(crate) async fn get_running_server_for_language(&self, language: &str) -> Option<String> {
        let states = self.server_states.read().await;

        if let Some(state) = states.get(language) {
            if state.status == ServerStatus::Running {
                return Some(language.to_string());
            }
        }

        for (lang, state) in states.iter() {
            if state.status == ServerStatus::Running {
                let is_related = (language == "c" && lang == "cpp")
                    || (language == "cpp" && lang == "c")
                    || (language == "javascript" && lang == "typescript")
                    || (language == "typescript" && lang == "javascript")
                    || (language == "javascriptreact" && lang == "javascript")
                    || (language == "typescriptreact" && lang == "typescript");

                if is_related {
                    return Some(lang.clone());
                }
            }
        }

        None
    }

    /// Returns the actual server language key (handles aliases, e.g. c -> cpp).
    async fn get_server_language(&self, language: &str) -> String {
        {
            let states = self.server_states.read().await;
            if states.contains_key(language) {
                return language.to_string();
            }
        }

        let states = self.server_states.read().await;
        for (lang, state) in states.iter() {
            if state.status == ServerStatus::Running {
                let is_related = (language == "c" && lang == "cpp")
                    || (language == "cpp" && lang == "c")
                    || (language == "javascript" && lang == "typescript")
                    || (language == "typescript" && lang == "javascript")
                    || (language == "javascriptreact" && lang == "javascript")
                    || (language == "typescriptreact" && lang == "typescript");

                if is_related {
                    return lang.clone();
                }
            }
        }

        language.to_string()
    }

    /// Ensures the server is running (prevents duplicate starts).
    /// Returns the actual server language key in use (may differ from the requested one, e.g. c -> cpp).
    // reason: ensure_server_running() is reserved for the upcoming server auto-start path; today's caller lazily starts servers on first request
    async fn ensure_server_running(&self, language: &str) -> Result<String> {
        let status = {
            let states = self.server_states.read().await;

            states.get(language).map(|state| state.status.clone())
        };

        if let Some(status) = status {
            match status {
                ServerStatus::Running => {
                    return Ok(language.to_string());
                }
                ServerStatus::Starting | ServerStatus::Restarting => {
                    debug!("Server is starting, waiting: {}", language);
                    self.wait_for_server_start(language).await?;
                    return Ok(language.to_string());
                }
                _ => {}
            }
        }

        let related_lang = {
            let states = self.server_states.read().await;

            let mut result = None;
            for (lang, state) in states.iter() {
                if state.status == ServerStatus::Running {
                    let is_related = (language == "c" && lang == "cpp")
                        || (language == "cpp" && lang == "c")
                        || (language == "javascript" && lang == "typescript")
                        || (language == "typescript" && lang == "javascript");

                    if is_related {
                        result = Some(lang.clone());
                        break;
                    }
                }
            }
            result
        };

        if let Some(related_lang) = related_lang {
            debug!("Using {} server for {}", related_lang, language);
            return Ok(related_lang);
        }

        info!("Starting {} server", language);
        self.start_server(language).await?;
        Ok(language.to_string())
    }

    /// Updates the server document count.
    async fn update_server_document_count(&self, language: &str) {
        let count = {
            let docs = self.documents.read().await;
            docs.values().filter(|doc| doc.language == language).count()
        };

        let mut states = self.server_states.write().await;
        if let Some(state) = states.get_mut(language) {
            state.document_count = count;
        }
    }

    /// Emits a server state change event.
    pub(crate) async fn emit_server_state_changed(&self, language: &str) {
        let state = self.get_server_state(language).await;

        debug!("Server state changed: {} -> {:?}", language, state.status);
    }

    /// Returns server state.
    pub async fn get_server_state(&self, language: &str) -> ServerState {
        let states = self.server_states.read().await;
        states.get(language).cloned().unwrap_or_else(|| ServerState {
            status: ServerStatus::Stopped,
            language: language.to_string(),
            ..Default::default()
        })
    }

    /// Returns all server states.
    pub async fn get_all_server_states(&self) -> HashMap<String, ServerState> {
        let states = self.server_states.read().await;
        states.clone()
    }

    /// Cleans up resources.
    pub async fn dispose(&self) -> Result<()> {
        info!("Disposing workspace LSP manager");

        {
            let mut handle = self.health_check_handle.write().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }

        {
            let mut watcher = self.config_watcher.write().await;
            *watcher = None;
        }

        let docs: Vec<String> = {
            let docs = self.documents.read().await;
            docs.keys().cloned().collect()
        };

        for uri in docs {
            let _ = self.close_document(uri).await;
        }

        let languages: Vec<String> = {
            let states = self.server_states.read().await;
            states.keys().cloned().collect()
        };

        for language in languages {
            let _ = self.stop_server(&language).await;
        }

        info!("Workspace LSP manager disposed");
        Ok(())
    }
}

impl Drop for WorkspaceLspManager {
    fn drop(&mut self) {
        debug!("WorkspaceLspManager dropped");
    }
}
