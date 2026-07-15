//! Session lifecycle methods: new, binding, create, query, IO, close, shutdown, Drop.
//!
//! Standalone sibling — `impl SessionManager` covering session lifecycle
//! concerns. Together with `session_events`, `session_shell_integration`,
//! and `session_commands` siblings, this replaces the previous monolithic
//! 1391-line `impl SessionManager` block in `session_manager.rs`.
//!
//! Sub-domain split follows QClaw review recommendation (R28b retry).
//! All session fields are `pub(super)` so sibling files within `session`
//! module can access private state.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::warn;

use crate::config::{ShellConfig, TerminalConfig};
use crate::events::TerminalEvent;
use crate::pty::PtyService;
use crate::shell::{ShellDetector, ShellType};
use crate::{TerminalError, TerminalResult};

use super::super::{SessionSource, SessionStatus, TerminalSession};
use super::session_manager::SessionManager;

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: TerminalConfig) -> Self {
        // Initialize scripts manager and ensure scripts are up-to-date
        let scripts_manager = crate::shell::ScriptsManager::new(config.shell_integration.scripts_dir.clone());
        if let Err(e) = scripts_manager.ensure_scripts() {
            warn!("Failed to ensure shell integration scripts: {}", e);
        }

        let pty_service = Arc::new(PtyService::new(config.clone()));
        let event_emitter = Arc::new(crate::events::TerminalEventEmitter::new(1024));
        let integration_manager = Arc::new(crate::shell::ShellIntegrationManager::new());
        let binding = Arc::new(super::super::TerminalSessionBinding::new());
        let output_taps = Arc::new(dashmap::DashMap::new());

        let manager = Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pty_service,
            event_emitter,
            pty_to_session: Arc::new(RwLock::new(HashMap::new())),
            integration_manager,
            session_integrations: Arc::new(RwLock::new(HashMap::new())),
            binding,
            scripts_manager,
            output_taps,
        };

        // Start event forwarding (sibling impl block in session_events)
        manager.start_event_forwarding();

        manager
    }

    /// Get the session binding manager
    ///
    /// Use this to manage bindings between external entities (e.g., chat sessions)
    /// and terminal sessions.
    pub fn binding(&self) -> Arc<super::super::TerminalSessionBinding> {
        self.binding.clone()
    }

    /// Create a new terminal session with shell integration
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        session_id: Option<String>,
        name: Option<String>,
        shell_type: Option<ShellType>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: Option<u16>,
        rows: Option<u16>,
        source: Option<SessionSource>,
    ) -> TerminalResult<TerminalSession> {
        self.create_session_with_options(session_id, name, shell_type, cwd, env, cols, rows, true, source)
            .await
    }

    /// Create a new terminal session with optional shell integration
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session_with_options(
        &self,
        session_id: Option<String>,
        name: Option<String>,
        shell_type: Option<ShellType>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: Option<u16>,
        rows: Option<u16>,
        enable_integration: bool,
        source: Option<SessionSource>,
    ) -> TerminalResult<TerminalSession> {
        // Use provided session ID or generate a new one
        let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Check if session ID already exists
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(&session_id) {
                return Err(TerminalError::Session(format!(
                    "Session with ID '{}' already exists",
                    session_id
                )));
            }
        }

        // Determine shell type
        let shell_type = shell_type.unwrap_or_else(|| {
            let detected = ShellDetector::default_shell();
            detected.shell_type
        });

        // Determine working directory
        let cwd = cwd.unwrap_or_else(|| {
            self.config.default_cwd.clone().unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })
        });

        // Generate name
        let name = name.unwrap_or_else(|| format!("Terminal {}", &session_id[..8]));

        // Generate nonce for shell integration
        let nonce = uuid::Uuid::new_v4().to_string();

        // Create shell config
        // On Windows, when shell_type is Bash, we need to use the detected Git Bash path
        // instead of just "bash" which might resolve to WSL bash in System32
        #[cfg(windows)]
        let shell_config_base = if matches!(shell_type, ShellType::Bash) {
            // Try to get Git Bash path from detection
            if let Some(detected) = ShellDetector::detect_git_bash() {
                detected.to_config()
            } else {
                // Fallback to default if Git Bash not found
                ShellConfig {
                    executable: shell_type.default_executable().to_string(),
                    args: Vec::new(),
                    env: HashMap::new(),
                    cwd: None,
                    login: false,
                }
            }
        } else {
            ShellConfig {
                executable: shell_type.default_executable().to_string(),
                args: Vec::new(),
                env: HashMap::new(),
                cwd: None,
                login: false,
            }
        };

        #[cfg(not(windows))]
        let shell_config_base = ShellConfig {
            executable: shell_type.default_executable().to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            login: false,
        };

        let mut shell_config = ShellConfig {
            executable: shell_config_base.executable,
            args: shell_config_base.args,
            env: self.config.env.clone(),
            cwd: Some(cwd.clone()),
            login: shell_config_base.login,
        };

        // Add custom environment
        if let Some(custom_env) = env {
            shell_config.env.extend(custom_env);
        }

        // Inject shell integration if enabled and supported
        // (sibling impl block in session_shell_integration)
        if enable_integration && shell_type.supports_integration() {
            self.inject_shell_integration(&mut shell_config, &shell_type, &nonce);
        }

        // Use provided dimensions or fall back to config defaults
        let cols = cols.unwrap_or(self.config.default_cols);
        let rows = rows.unwrap_or(self.config.default_rows);

        // Create the session record
        let session = TerminalSession::new(
            session_id.clone(),
            name,
            shell_type.clone(),
            cwd,
            cols,
            rows,
            source.unwrap_or_default(),
        );

        // Store the session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        // Create shell integration instance
        if enable_integration && shell_type.supports_integration() {
            let mut integration = crate::shell::ShellIntegration::new();
            integration.set_nonce(nonce.clone());

            let mut integrations = self.session_integrations.write().await;
            integrations.insert(session_id.clone(), integration);

            self.integration_manager
                .register_session(&session_id, Some(nonce))
                .await;
        }

        // Create the PTY process
        let pty_id = self
            .pty_service
            .create_process(shell_config, shell_type, cols, rows)
            .await?;

        // Update session with PTY ID
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.pty_id = Some(pty_id);
            }
        }

        // Store PTY to session mapping
        {
            let mut mapping = self.pty_to_session.write().await;
            mapping.insert(pty_id, session_id.clone());
        }

        // Emit creation event
        let _ = self
            .event_emitter
            .emit(TerminalEvent::SessionCreated {
                session_id: session_id.clone(),
                pid: None,
                cwd: session.cwd.clone(),
            })
            .await;

        // Return the session
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| TerminalError::Session("Session was removed".to_string()))
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Write data to a session
    pub async fn write(&self, session_id: &str, data: &[u8]) -> TerminalResult<()> {
        let pty_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .and_then(|s| s.pty_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?
        };

        self.pty_service.write(pty_id, data).await
    }

    /// Resize a session
    ///
    /// This method:
    /// 1. Updates session dimensions
    /// 2. Flushes any buffered data in PTY service
    /// 3. Resizes the PTY
    /// 4. Emits a Resized event for frontend confirmation
    pub async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> TerminalResult<()> {
        // Update session dimensions
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.resize(cols, rows);
            }
        }

        let pty_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .and_then(|s| s.pty_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?
        };

        // Resize PTY (this also flushes buffered data)
        // Do not send Resized event here because Windows ConPTY has a delay
        // PTY sends ResizeCompleted event after resize is completed,
        // This event is forwarded to TerminalEvent::Resized in start_event_forwarding()
        self.pty_service.resize(pty_id, cols, rows).await?;

        Ok(())
    }

    /// Send a signal to a session
    pub async fn signal(&self, session_id: &str, signal: &str) -> TerminalResult<()> {
        let pty_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .and_then(|s| s.pty_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?
        };

        self.pty_service.signal(pty_id, signal).await
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str, immediate: bool) -> TerminalResult<()> {
        let pty_id = {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

            session.status = SessionStatus::Terminating;
            session.pty_id
        };

        // Shutdown PTY if exists
        if let Some(pty_id) = pty_id {
            // Remove mapping
            {
                let mut mapping = self.pty_to_session.write().await;
                mapping.remove(&pty_id);
            }

            self.pty_service.shutdown(pty_id, immediate).await?;
        }

        // Remove shell integration
        {
            let mut integrations = self.session_integrations.write().await;
            integrations.remove(session_id);
        }
        self.integration_manager.unregister_session(session_id).await;

        // Drop output taps so file-writing tasks can detect session end
        self.output_taps.remove(session_id);

        // Remove session
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }

        // Remove any binding pointing to this session so the next get_or_create
        // creates a fresh session rather than returning a stale ID.
        // For primary sessions owner_id == session_id, so unbind(session_id) is sufficient.
        self.binding.unbind(session_id);

        // Emit session destroyed event for frontend
        let _ = self
            .event_emitter
            .emit(TerminalEvent::SessionDestroyed {
                session_id: session_id.to_string(),
            })
            .await;

        Ok(())
    }

    /// Acknowledge data received by frontend
    pub async fn acknowledge_data(&self, session_id: &str, char_count: usize) -> TerminalResult<()> {
        let pty_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .and_then(|s| s.pty_id)
                .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?
        };

        self.pty_service.acknowledge_data(pty_id, char_count).await
    }

    /// Shutdown all sessions
    pub async fn shutdown_all(&self) {
        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.keys().cloned().collect()
        };

        for session_id in session_ids {
            if let Err(e) = self.close_session(&session_id, true).await {
                warn!("Failed to close session {}: {}", session_id, e);
            }
        }

        self.pty_service.shutdown_all().await;
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {}
}
