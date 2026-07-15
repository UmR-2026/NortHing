//! RemoteExecProcessManager: session table, exec orchestration, and global accessor.

use super::output::{
    completion_for_closed_remote_process, completion_status_for_control_action, deadline_from_now,
    emit_lifecycle, input_bytes_for_write, lifecycle_status_for_completion, new_chunk_id, new_session_id,
    spawn_lifecycle_exit_watcher, OutputCursor, OutputState,
};
use super::process::{spawn_remote_process, RemoteExecProcess, RemoteExecProcessCommand};
use super::types::{
    RemoteExecCommandRequest, RemoteExecCommandResponse, RemoteExecControlAction, RemoteExecControlOrigin,
    RemoteExecControlRequest, RemoteExecError, RemoteExecProcessLifecycleEvent,
    RemoteExecProcessLifecycleStatus, RemoteExecResult, RemoteExecSessionCompletion,
    RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus, RemoteSendStdinRequest,
    RemoteWriteStdinRequest, MAX_COMPLETED_REMOTE_EXEC_SESSIONS, MAX_REMOTE_EXEC_SESSIONS,
};
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Default)]
pub struct RemoteExecProcessManager {
    pub(crate) sessions: tokio::sync::Mutex<HashMap<i32, RemoteExecSessionEntry>>,
    pub(crate) completed_sessions: tokio::sync::Mutex<HashMap<i32, CompletedRemoteExecSession>>,
}

pub(crate) struct RemoteExecSessionEntry {
    pub(crate) process: Arc<RemoteExecProcess>,
    pub(crate) tty: bool,
    pub(crate) cursor: OutputCursor,
    pub(crate) last_used: Instant,
    pub(crate) lifecycle_tx: Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>>,
}

#[derive(Clone)]
pub(crate) struct CompletedRemoteExecSession {
    pub(crate) output: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) original_output_chars: usize,
    pub(crate) completion: RemoteExecSessionCompletion,
    pub(crate) completed_at: Instant,
}

static GLOBAL_REMOTE_EXEC_MANAGER: OnceLock<Arc<RemoteExecProcessManager>> = OnceLock::new();

pub fn global_remote_exec_process_manager() -> Arc<RemoteExecProcessManager> {
    GLOBAL_REMOTE_EXEC_MANAGER
        .get_or_init(|| Arc::new(RemoteExecProcessManager::default()))
        .clone()
}
impl RemoteExecProcessManager {
    pub async fn exec_command(&self, request: RemoteExecCommandRequest) -> RemoteExecResult<RemoteExecCommandResponse> {
        self.exec_command_inner(request, None).await
    }

    pub async fn exec_command_streaming(
        &self,
        request: RemoteExecCommandRequest,
        output_tx: mpsc::Sender<String>,
    ) -> RemoteExecResult<RemoteExecCommandResponse> {
        self.exec_command_inner(request, Some(output_tx)).await
    }

    async fn exec_command_inner(
        &self,
        request: RemoteExecCommandRequest,
        output_tx: Option<mpsc::Sender<String>>,
    ) -> RemoteExecResult<RemoteExecCommandResponse> {
        let process = Arc::new(spawn_remote_process(request.clone()).await?);
        let cursor = OutputCursor { next_seq: 0 };
        let session_id = self
            .store_session(Arc::clone(&process), request.tty, cursor.clone(), request.lifecycle_tx)
            .await;
        let started_at = Instant::now();
        let collected = process
            .output
            .collect_until(
                cursor,
                deadline_from_now(request.yield_time_ms),
                request.max_output_chars.unwrap_or(usize::MAX),
                output_tx.as_ref(),
            )
            .await;

        let exit_code = process.output.exit_code().await;
        let closed = process.output.is_closed().await;
        let completion = if closed {
            Some(completion_for_closed_remote_process(
                process.out_of_band_control_action(),
            ))
        } else {
            None
        };
        self.update_or_remove_session(session_id, &process, collected.cursor.clone(), None, exit_code)
            .await;

        Ok(RemoteExecCommandResponse {
            chunk_id: new_chunk_id(),
            wall_time_seconds: started_at.elapsed().as_secs_f64(),
            output: collected.output,
            session_id: (!closed).then_some(session_id),
            exit_code,
            original_output_chars: collected.original_output_chars,
            completion,
        })
    }

    pub async fn write_stdin(&self, request: RemoteWriteStdinRequest) -> RemoteExecResult<RemoteExecCommandResponse> {
        self.write_stdin_inner(request, None).await
    }

    pub async fn write_stdin_streaming(
        &self,
        request: RemoteWriteStdinRequest,
        output_tx: mpsc::Sender<String>,
    ) -> RemoteExecResult<RemoteExecCommandResponse> {
        self.write_stdin_inner(request, Some(output_tx)).await
    }

    pub async fn send_stdin(&self, request: RemoteSendStdinRequest) -> RemoteExecResult<()> {
        let (process, tty) = {
            let mut sessions = self.sessions.lock().await;
            let entry = sessions
                .get_mut(&request.session_id)
                .ok_or(RemoteExecError::SessionNotFound(request.session_id))?;
            entry.last_used = Instant::now();
            (Arc::clone(&entry.process), entry.tty)
        };

        let input = input_bytes_for_write(&request.chars, request.append_enter);
        if input.is_empty() {
            return Ok(());
        }
        if !tty {
            return Err(anyhow!("stdin input requires a tty session").into());
        }

        process
            .command_tx
            .send(RemoteExecProcessCommand::Write(input))
            .await
            .context("remote process has already exited")
            .map_err(RemoteExecError::from)
    }

    async fn write_stdin_inner(
        &self,
        request: RemoteWriteStdinRequest,
        output_tx: Option<mpsc::Sender<String>>,
    ) -> RemoteExecResult<RemoteExecCommandResponse> {
        let (process, tty, cursor) = {
            let mut sessions = self.sessions.lock().await;
            let Some(entry) = sessions.get_mut(&request.session_id) else {
                drop(sessions);
                if request.chars.is_empty() {
                    if let Some(completed) = self.take_completed_session(request.session_id).await {
                        return Ok(RemoteExecCommandResponse {
                            chunk_id: new_chunk_id(),
                            wall_time_seconds: 0.0,
                            output: completed.output,
                            session_id: None,
                            exit_code: completed.exit_code,
                            original_output_chars: completed.original_output_chars,
                            completion: Some(completed.completion),
                        });
                    }
                }
                return Err(RemoteExecError::SessionNotFound(request.session_id));
            };
            entry.last_used = Instant::now();
            (Arc::clone(&entry.process), entry.tty, entry.cursor.clone())
        };

        let input = input_bytes_for_write(&request.chars, request.append_enter);
        if !input.is_empty() && tty {
            process
                .command_tx
                .send(RemoteExecProcessCommand::Write(input))
                .await
                .context("remote process has already exited")?;
        }

        let started_at = Instant::now();
        let collected = process
            .output
            .collect_until(
                cursor,
                deadline_from_now(request.yield_time_ms),
                request.max_output_chars.unwrap_or(usize::MAX),
                output_tx.as_ref(),
            )
            .await;

        let closed = process.output.is_closed().await;
        let exit_code = process.output.exit_code().await;
        let completion = if closed {
            Some(completion_for_closed_remote_process(
                process.out_of_band_control_action(),
            ))
        } else {
            None
        };
        self.update_or_remove_session(
            request.session_id,
            &process,
            collected.cursor.clone(),
            completion.map(|completion| lifecycle_status_for_completion(completion.status)),
            exit_code,
        )
        .await;

        Ok(RemoteExecCommandResponse {
            chunk_id: new_chunk_id(),
            wall_time_seconds: started_at.elapsed().as_secs_f64(),
            output: collected.output,
            session_id: (!closed).then_some(request.session_id),
            exit_code,
            original_output_chars: collected.original_output_chars,
            completion,
        })
    }

    pub async fn control_session(
        &self,
        request: RemoteExecControlRequest,
    ) -> RemoteExecResult<RemoteExecCommandResponse> {
        let (process, cursor) = {
            let mut sessions = self.sessions.lock().await;
            let entry = sessions
                .get_mut(&request.session_id)
                .ok_or(RemoteExecError::SessionNotFound(request.session_id))?;
            entry.last_used = Instant::now();
            if request.origin == RemoteExecControlOrigin::OutOfBand {
                entry.process.mark_out_of_band_control(request.action);
            }
            (Arc::clone(&entry.process), entry.cursor.clone())
        };

        process
            .command_tx
            .send(RemoteExecProcessCommand::Control(request.action))
            .await
            .context("remote process has already exited")?;

        let started_at = Instant::now();
        let collected = process
            .output
            .collect_until(
                cursor.clone(),
                deadline_from_now(request.yield_time_ms),
                request.max_output_chars.unwrap_or(usize::MAX),
                None,
            )
            .await;

        let closed = process.output.is_closed().await;
        let exit_code = process.output.exit_code().await;
        let completion = closed.then_some(RemoteExecSessionCompletion {
            status: completion_status_for_control_action(request.action),
            source: match request.origin {
                RemoteExecControlOrigin::ModelTool => RemoteExecSessionCompletionSource::Process,
                RemoteExecControlOrigin::OutOfBand => RemoteExecSessionCompletionSource::OutOfBandControl,
            },
        });
        let lifecycle_status = completion.map(|completion| lifecycle_status_for_completion(completion.status));
        self.update_or_remove_session(
            request.session_id,
            &process,
            if request.origin == RemoteExecControlOrigin::ModelTool {
                collected.cursor.clone()
            } else {
                cursor
            },
            lifecycle_status,
            exit_code,
        )
        .await;
        if request.origin == RemoteExecControlOrigin::OutOfBand && closed {
            self.store_completed_session(
                request.session_id,
                CompletedRemoteExecSession {
                    output: collected.output.clone(),
                    exit_code,
                    original_output_chars: collected.original_output_chars,
                    completion: completion.expect("closed process should have completion"),
                    completed_at: Instant::now(),
                },
            )
            .await;
        }

        Ok(RemoteExecCommandResponse {
            chunk_id: new_chunk_id(),
            wall_time_seconds: started_at.elapsed().as_secs_f64(),
            output: collected.output,
            session_id: (!closed).then_some(request.session_id),
            exit_code,
            original_output_chars: collected.original_output_chars,
            completion,
        })
    }

    async fn store_session(
        &self,
        process: Arc<RemoteExecProcess>,
        tty: bool,
        cursor: OutputCursor,
        lifecycle_tx: Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>>,
    ) -> i32 {
        let (session_id, pruned_entry) = {
            let mut sessions = self.sessions.lock().await;
            let pruned = if sessions.len() >= MAX_REMOTE_EXEC_SESSIONS {
                sessions
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_used)
                    .map(|(id, _)| *id)
                    .and_then(|id| sessions.remove(&id).map(|entry| (id, entry)))
            } else {
                None
            };

            let session_id = new_session_id(&sessions);
            sessions.insert(
                session_id,
                RemoteExecSessionEntry {
                    process: Arc::clone(&process),
                    tty,
                    cursor,
                    last_used: Instant::now(),
                    lifecycle_tx: lifecycle_tx.clone(),
                },
            );
            (session_id, pruned)
        };

        if let Some((pruned_session_id, entry)) = pruned_entry {
            emit_lifecycle(
                entry.lifecycle_tx.clone(),
                RemoteExecProcessLifecycleEvent {
                    session_id: pruned_session_id,
                    status: RemoteExecProcessLifecycleStatus::Pruned,
                    exit_code: None,
                },
            );
            entry.process.request_control(RemoteExecControlAction::Kill);
        }

        emit_lifecycle(
            lifecycle_tx.clone(),
            RemoteExecProcessLifecycleEvent {
                session_id,
                status: RemoteExecProcessLifecycleStatus::Running,
                exit_code: None,
            },
        );
        spawn_lifecycle_exit_watcher(session_id, process, lifecycle_tx);

        session_id
    }

    async fn update_or_remove_session(
        &self,
        session_id: i32,
        process: &RemoteExecProcess,
        cursor: OutputCursor,
        lifecycle_status: Option<RemoteExecProcessLifecycleStatus>,
        exit_code: Option<i32>,
    ) {
        if process.output.is_closed().await {
            let mut sessions = self.sessions.lock().await;
            if let Some(entry) = sessions.remove(&session_id) {
                if let Some(status) = lifecycle_status {
                    emit_lifecycle(
                        entry.lifecycle_tx.clone(),
                        RemoteExecProcessLifecycleEvent {
                            session_id,
                            status,
                            exit_code,
                        },
                    );
                }
            }
        } else {
            let mut sessions = self.sessions.lock().await;
            if let Some(entry) = sessions.get_mut(&session_id) {
                entry.cursor = cursor;
            }
        }
    }

    async fn store_completed_session(&self, session_id: i32, completed: CompletedRemoteExecSession) {
        let mut completed_sessions = self.completed_sessions.lock().await;
        if completed_sessions.len() >= MAX_COMPLETED_REMOTE_EXEC_SESSIONS {
            if let Some(oldest_session_id) = completed_sessions
                .iter()
                .min_by_key(|(_, session)| session.completed_at)
                .map(|(id, _)| *id)
            {
                completed_sessions.remove(&oldest_session_id);
            }
        }
        completed_sessions.insert(session_id, completed);
    }

    async fn take_completed_session(&self, session_id: i32) -> Option<CompletedRemoteExecSession> {
        self.completed_sessions.lock().await.remove(&session_id)
    }
}

