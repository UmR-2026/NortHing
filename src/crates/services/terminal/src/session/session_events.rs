//! Session event forwarding + subscribers.
//!
//! Standalone sibling — `impl SessionManager` for:
//! - `start_event_forwarding` (PTY events -> TerminalEvent emitter)
//! - `event_emitter` accessor
//! - `subscribe_session_output` (raw output tap fan-out)
//!
//! Together with `session_lifecycle`, `session_shell_integration`,
//! and `session_commands` siblings, this replaces the previous
//! monolithic 1391-line `impl SessionManager` block.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::events::{TerminalEvent, TerminalEventEmitter};
use crate::pty::{ProcessProperty, PtyServiceEvent};
use crate::shell::ShellIntegrationEvent;

use super::super::SessionStatus;
use super::session_manager::SessionManager;

impl SessionManager {
    /// Start forwarding PTY service events to terminal events
    pub(super) fn start_event_forwarding(&self) {
        let pty_service = self.pty_service.clone();
        let event_emitter = self.event_emitter.clone();
        let sessions = self.sessions.clone();
        let pty_to_session = self.pty_to_session.clone();
        let session_integrations = self.session_integrations.clone();
        let output_taps = self.output_taps.clone();

        tokio::spawn(async move {
            loop {
                if let Some(event) = pty_service.recv_event().await {
                    let pty_id = match &event {
                        PtyServiceEvent::ProcessData { id, .. } => *id,
                        PtyServiceEvent::ProcessReady { id, .. } => *id,
                        PtyServiceEvent::ProcessExit { id, .. } => *id,
                        PtyServiceEvent::ProcessProperty { id, .. } => *id,
                        PtyServiceEvent::ResizeCompleted { id, .. } => *id,
                    };

                    // Retry the pty_to_session lookup a few times for
                    // non-Data events.  create_session sets the mapping
                    // AFTER create_process returns, but event forwarding
                    // can deliver ProcessReady before the mapping exists.
                    let session_id = {
                        let mapping = pty_to_session.read().await;
                        match mapping.get(&pty_id).cloned() {
                            Some(sid) => Some(sid),
                            None if !matches!(event, PtyServiceEvent::ProcessData { .. }) => {
                                drop(mapping);
                                let mut found = None;
                                for _ in 0..50 {
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                    let m = pty_to_session.read().await;
                                    if let Some(sid) = m.get(&pty_id).cloned() {
                                        found = Some(sid);
                                        break;
                                    }
                                }
                                found
                            }
                            None => None,
                        }
                    };

                    if let Some(session_id) = session_id {
                        let terminal_event = match event {
                            PtyServiceEvent::ProcessData { data, .. } => {
                                // Update last activity and record to history
                                if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                    session.touch();
                                    // Record output to history for frontend recovery
                                    let data_str = String::from_utf8_lossy(&data).to_string();
                                    session.add_output(&data_str);
                                }

                                // Convert to string (lossy for now)
                                let data_str = String::from_utf8_lossy(&data).to_string();

                                // Process through shell integration
                                {
                                    let mut integrations = session_integrations.write().await;
                                    if let Some(integration) = integrations.get_mut(&session_id) {
                                        let si_events = integration.process_data(&data_str);

                                        // Emit shell integration events as terminal events
                                        for si_event in si_events {
                                            match si_event {
                                                ShellIntegrationEvent::CommandStarted { command, command_id } => {
                                                    let _ = event_emitter
                                                        .emit(TerminalEvent::CommandStarted {
                                                            session_id: session_id.clone(),
                                                            command,
                                                            command_id,
                                                        })
                                                        .await;
                                                }
                                                ShellIntegrationEvent::CommandFinished { command_id, exit_code } => {
                                                    let _ = event_emitter
                                                        .emit(TerminalEvent::CommandFinished {
                                                            session_id: session_id.clone(),
                                                            command_id,
                                                            exit_code: exit_code.unwrap_or(0),
                                                        })
                                                        .await;
                                                }
                                                ShellIntegrationEvent::CwdChanged { cwd } => {
                                                    if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                                        session.update_cwd(cwd.clone());
                                                    }
                                                    let _ = event_emitter
                                                        .emit(TerminalEvent::CwdChanged {
                                                            session_id: session_id.clone(),
                                                            cwd,
                                                        })
                                                        .await;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }

                                // Fan out raw data to output taps (e.g. background session file loggers)
                                if let Some(mut senders) = output_taps.get_mut(&session_id) {
                                    senders.retain(|tx| tx.try_send(data_str.clone()).is_ok());
                                }

                                TerminalEvent::Data {
                                    session_id,
                                    data: data_str,
                                }
                            }
                            PtyServiceEvent::ProcessReady { pid, cwd, .. } => {
                                // Update session
                                if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                    session.pid = Some(pid);
                                    session.cwd = cwd.clone();
                                    session.status = SessionStatus::Active;
                                    session.touch();
                                }

                                TerminalEvent::Ready { session_id, pid, cwd }
                            }
                            PtyServiceEvent::ProcessExit { exit_code, .. } => {
                                // Update session
                                if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                    session.set_exited(exit_code.map(|c| c as i32));
                                }

                                TerminalEvent::Exit {
                                    session_id,
                                    exit_code: exit_code.map(|c| c as i32),
                                }
                            }
                            PtyServiceEvent::ProcessProperty { property, .. } => match property {
                                ProcessProperty::Title(title) => TerminalEvent::TitleChanged { session_id, title },
                                ProcessProperty::Cwd(cwd) => {
                                    if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                        session.update_cwd(cwd.clone());
                                    }
                                    TerminalEvent::CwdChanged { session_id, cwd }
                                }
                                ProcessProperty::ShellType(shell_type) => {
                                    TerminalEvent::ShellTypeChanged { session_id, shell_type }
                                }
                                _ => continue,
                            },
                            PtyServiceEvent::ResizeCompleted { cols, rows, .. } => {
                                // Update session dimensions
                                if let Some(session) = sessions.write().await.get_mut(&session_id) {
                                    session.cols = cols;
                                    session.rows = rows;
                                }
                                TerminalEvent::Resized { session_id, cols, rows }
                            }
                        };

                        let _ = event_emitter.emit(terminal_event).await;
                    }
                }
            }
        });
    }

    /// Get the event emitter for subscribing to events
    pub fn event_emitter(&self) -> Arc<TerminalEventEmitter> {
        self.event_emitter.clone()
    }

    /// Subscribe to the raw PTY output of a specific session.
    ///
    /// Returns a receiver that yields raw output strings as they arrive from the PTY.
    /// The receiver will return `None` (channel closed) when the session is destroyed.
    /// Multiple subscriptions to the same session are supported.
    pub fn subscribe_session_output(&self, session_id: &str) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(256);
        self.output_taps.entry(session_id.to_string()).or_default().push(tx);
        rx
    }
}
