//! Manager for tracking shell integration across multiple sessions.
//!
//! Cross-sibling: imports `CommandState`, `ShellIntegrationEvent` from `super::types`
//! and `ShellIntegration` from `super::shell_integration`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use super::shell_integration::ShellIntegration;
use super::types::{CommandState, ShellIntegrationEvent};

/// Manager for tracking shell integration across multiple sessions
pub struct ShellIntegrationManager {
    /// Integration instances per session
    integrations: Arc<RwLock<HashMap<String, ShellIntegration>>>,
    /// Event sender
    event_tx: mpsc::Sender<(String, ShellIntegrationEvent)>,
    /// Event receiver
    event_rx: Arc<RwLock<mpsc::Receiver<(String, ShellIntegrationEvent)>>>,
}

impl ShellIntegrationManager {
    /// Create a new shell integration manager
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(1024);
        Self {
            integrations: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        }
    }

    /// Register a new session
    pub async fn register_session(&self, session_id: &str, nonce: Option<String>) {
        let mut integrations = self.integrations.write().await;
        let mut integration = ShellIntegration::new();
        if let Some(n) = nonce {
            integration.set_nonce(n);
        }
        integrations.insert(session_id.to_string(), integration);
    }

    /// Unregister a session
    pub async fn unregister_session(&self, session_id: &str) {
        let mut integrations = self.integrations.write().await;
        integrations.remove(session_id);
    }

    /// Process data for a session
    pub async fn process_data(&self, session_id: &str, data: &str) -> Vec<ShellIntegrationEvent> {
        let mut integrations = self.integrations.write().await;

        if let Some(integration) = integrations.get_mut(session_id) {
            let events = integration.process_data(data);

            // Send events through channel
            for event in &events {
                let _ = self.event_tx.send((session_id.to_string(), event.clone())).await;
            }

            events
        } else {
            Vec::new()
        }
    }

    /// Get the current state for a session
    pub async fn get_state(&self, session_id: &str) -> Option<CommandState> {
        let integrations = self.integrations.read().await;
        integrations.get(session_id).map(|i| i.state().clone())
    }

    /// Get the current working directory for a session
    pub async fn get_cwd(&self, session_id: &str) -> Option<String> {
        let integrations = self.integrations.read().await;
        integrations
            .get(session_id)
            .and_then(|i| i.cwd().map(|s| s.to_string()))
    }

    /// Get accumulated output for a session
    pub async fn output(&self, session_id: &str) -> Option<String> {
        let integrations = self.integrations.read().await;
        integrations.get(session_id).map(|i| i.output().to_string())
    }

    /// Clear output buffer for a session
    pub async fn clear_output(&self, session_id: &str) {
        let mut integrations = self.integrations.write().await;
        if let Some(integration) = integrations.get_mut(session_id) {
            integration.clear_output();
        }
    }

    /// Receive the next event
    pub async fn recv_event(&self) -> Option<(String, ShellIntegrationEvent)> {
        let mut rx = self.event_rx.write().await;
        rx.recv().await
    }

    /// Get a clone of the event sender
    pub fn event_sender(&self) -> mpsc::Sender<(String, ShellIntegrationEvent)> {
        self.event_tx.clone()
    }
}

impl Default for ShellIntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}
