//! RemoteExecProcess lifecycle: spawn, channel ownership, and control.

use super::types::{
    RemoteExecCommandRequest, RemoteExecControlAction, RemoteExecControlOrigin, RemoteExecError,
    RemoteExecProcessLifecycleEvent, RemoteExecProcessLifecycleStatus, RemoteExecResult,
    RemoteExecSessionCompletion, RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus,
};
use crate::remote_ssh::SSHConnectionManager;
use anyhow::{anyhow, Context};
use rand::Rng;
use russh::client::Msg;
use russh::{Channel, ChannelMsg, Sig};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio::time::{Duration as TokioDuration, Instant};
use tracing::warn;
use uuid::Uuid;

use super::output::{OutputInner, OutputState};
use super::types::{RemoteExecCommandResponse, REMOTE_CONTROL_DRAIN_TIMEOUT_MS, REMOTE_INTERRUPT_GRACE_TIMEOUT_MS};

pub struct RemoteExecProcess {
    pub(crate) output: Arc<OutputState>,
    pub(crate) command_tx: mpsc::Sender<RemoteExecProcessCommand>,
    pub(crate) out_of_band_control_action: StdMutex<Option<RemoteExecControlAction>>,
}

pub enum RemoteExecProcessCommand {
    Write(Vec<u8>),
    Control(RemoteExecControlAction),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RemotePipeControlState {
    InterruptGrace { deadline: Instant },
    KillDrain { deadline: Instant },
}

impl RemotePipeControlState {
    pub(super) fn deadline(self) -> Instant {
        match self {
            Self::InterruptGrace { deadline } | Self::KillDrain { deadline } => deadline,
        }
    }
}
impl Drop for RemoteExecProcess {
    fn drop(&mut self) {
        self.request_control(RemoteExecControlAction::Kill);
    }
}

impl RemoteExecProcess {
    pub(crate) fn mark_out_of_band_control(&self, action: RemoteExecControlAction) {
        if let Ok(mut out_of_band_action) = self.out_of_band_control_action.lock() {
            *out_of_band_action = Some(action);
        }
    }

    pub(crate) fn out_of_band_control_action(&self) -> Option<RemoteExecControlAction> {
        self.out_of_band_control_action.lock().ok().and_then(|action| *action)
    }

    pub(crate) fn request_control(&self, action: RemoteExecControlAction) {
        if let Err(e) = self.command_tx.try_send(RemoteExecProcessCommand::Control(action)) {
            warn!("Failed to send control command to remote process: {e}");
        }
    }
}

pub(crate) async fn spawn_remote_process(request: RemoteExecCommandRequest) -> anyhow::Result<RemoteExecProcess> {
    if request.tty {
        spawn_remote_pty_process(request).await
    } else {
        spawn_remote_pipe_process(request).await
    }
}

async fn spawn_remote_pipe_process(request: RemoteExecCommandRequest) -> anyhow::Result<RemoteExecProcess> {
    let channel = request
        .ssh_manager
        .open_exec_channel(&request.connection_id, &request.command)
        .await?;
    let output = Arc::new(OutputState::new(request.output_capture_tx.clone()));
    let (command_tx, command_rx) = mpsc::channel::<RemoteExecProcessCommand>(8);
    tokio::spawn(remote_pipe_owner(channel, command_rx, output.clone()));

    Ok(RemoteExecProcess {
        output,
        command_tx,
        out_of_band_control_action: StdMutex::new(None),
    })
}

async fn spawn_remote_pty_process(request: RemoteExecCommandRequest) -> anyhow::Result<RemoteExecProcess> {
    let channel = request
        .ssh_manager
        .open_pty_exec_channel(&request.connection_id, &request.command, 80, 24)
        .await?;
    let output = Arc::new(OutputState::new(request.output_capture_tx.clone()));
    let (command_tx, command_rx) = mpsc::channel::<RemoteExecProcessCommand>(64);
    tokio::spawn(remote_pty_owner(channel, command_rx, output.clone()));

    Ok(RemoteExecProcess {
        output,
        command_tx,
        out_of_band_control_action: StdMutex::new(None),
    })
}

async fn remote_pipe_owner(
    mut channel: Channel<Msg>,
    mut command_rx: mpsc::Receiver<RemoteExecProcessCommand>,
    output: Arc<OutputState>,
) {
    let mut exit_code = None;
    let mut control_state: Option<RemotePipeControlState> = None;

    loop {
        if let Some(state) = control_state {
            if Instant::now() >= state.deadline() {
                match state {
                    RemotePipeControlState::InterruptGrace { .. } => {
                        if let Err(e) = channel.signal(Sig::KILL).await {
                            warn!("Failed to send KILL signal to remote pipe: {e}");
                        }
                        if let Err(e) = channel.eof().await {
                            warn!("Failed to send EOF to remote pipe: {e}");
                        }
                        control_state = Some(RemotePipeControlState::KillDrain {
                            deadline: Instant::now() + Duration::from_millis(REMOTE_CONTROL_DRAIN_TIMEOUT_MS),
                        });
                    }
                    RemotePipeControlState::KillDrain { .. } => {
                        if let Err(e) = channel.close().await {
                            warn!("Failed to close remote pipe channel: {e}");
                        }
                        break;
                    }
                }
            }
        }

        let wait_budget = control_state
            .map(RemotePipeControlState::deadline)
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .filter(|duration| !duration.is_zero())
            .unwrap_or_else(|| Duration::from_millis(100));

        tokio::select! {
            biased;

            command = command_rx.recv() => {
                match command {
                    Some(RemoteExecProcessCommand::Write(_)) => {}
                    Some(RemoteExecProcessCommand::Control(RemoteExecControlAction::Interrupt)) => {
                        if let Err(e) = channel.signal(Sig::INT).await {
                            warn!("Failed to send INT signal to remote pipe: {e}");
                        }
                        if let Err(e) = channel.eof().await {
                            warn!("Failed to send EOF to remote pipe: {e}");
                        }
                        control_state = Some(RemotePipeControlState::InterruptGrace {
                            deadline: Instant::now()
                                + Duration::from_millis(REMOTE_INTERRUPT_GRACE_TIMEOUT_MS),
                        });
                    }
                    Some(RemoteExecProcessCommand::Control(RemoteExecControlAction::Kill)) => {
                        if let Err(e) = channel.signal(Sig::TERM).await {
                            warn!("Failed to send TERM signal to remote pipe: {e}");
                        }
                        if let Err(e) = channel.eof().await {
                            warn!("Failed to send EOF to remote pipe: {e}");
                        }
                        control_state = Some(RemotePipeControlState::KillDrain {
                            deadline: Instant::now()
                                + Duration::from_millis(REMOTE_CONTROL_DRAIN_TIMEOUT_MS),
                        });
                    }
                    None => {
                        if let Err(e) = channel.signal(Sig::KILL).await {
                            warn!("Failed to send KILL signal to remote pipe: {e}");
                        }
                        if let Err(e) = channel.close().await {
                            warn!("Failed to close remote pipe channel: {e}");
                        }
                        break;
                    }
                }
            }

            message = channel.wait() => {
                match message {
                    Some(ChannelMsg::Data { data }) => output.push_chunk(data.to_vec()).await,
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        output.push_chunk(data.to_vec()).await;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        exit_code = Some(exit_status as i32);
                    }
                    Some(ChannelMsg::ExitSignal { signal_name, .. }) => {
                        exit_code = Some(match signal_name {
                            Sig::INT => 130,
                            Sig::KILL => 137,
                            Sig::TERM => 143,
                            _ => -1,
                        });
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    Some(_) => {}
                }
            }

            _ = tokio::time::sleep(wait_budget), if control_state.is_some() => {}
        }
    }

    output.close(exit_code).await;
}

async fn remote_pty_owner(
    mut channel: Channel<Msg>,
    mut command_rx: mpsc::Receiver<RemoteExecProcessCommand>,
    output: Arc<OutputState>,
) {
    let mut exit_code = None;
    let mut close_after_control_at: Option<Instant> = None;

    loop {
        if close_after_control_at.is_some_and(|deadline| Instant::now() >= deadline) {
            if let Err(e) = channel.close().await {
                warn!("Failed to close remote pty channel: {e}");
            }
            break;
        }

        let wait_budget = close_after_control_at
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .filter(|duration| !duration.is_zero())
            .unwrap_or_else(|| Duration::from_millis(100));

        tokio::select! {
            biased;

            command = command_rx.recv() => {
                match command {
                    Some(RemoteExecProcessCommand::Write(bytes)) => {
                        if let Err(e) = channel.data(&bytes[..]).await {
                            warn!("Failed to write data to remote pty: {e}");
                        }
                    }
                    Some(RemoteExecProcessCommand::Control(RemoteExecControlAction::Interrupt)) => {
                        if let Err(e) = channel.data(&[0x03u8][..]).await {
                            warn!("Failed to send interrupt to remote pty: {e}");
                        }
                    }
                    Some(RemoteExecProcessCommand::Control(RemoteExecControlAction::Kill)) => {
                        if let Err(e) = channel.signal(Sig::KILL).await {
                            warn!("Failed to send KILL signal to remote pty: {e}");
                        }
                        if let Err(e) = channel.eof().await {
                            warn!("Failed to send EOF to remote pty: {e}");
                        }
                        close_after_control_at = Some(
                            Instant::now() + Duration::from_millis(REMOTE_CONTROL_DRAIN_TIMEOUT_MS)
                        );
                    }
                    None => {
                        if let Err(e) = channel.signal(Sig::KILL).await {
                            warn!("Failed to send KILL signal to remote pipe: {e}");
                        }
                        if let Err(e) = channel.close().await {
                            warn!("Failed to close remote pipe channel: {e}");
                        }
                        break;
                    }
                }
            }

            message = channel.wait() => {
                match message {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        output.push_chunk(data.to_vec()).await;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        exit_code = Some(exit_status as i32);
                    }
                    Some(ChannelMsg::ExitSignal { signal_name, .. }) => {
                        exit_code = Some(match signal_name {
                            Sig::INT => 130,
                            Sig::KILL => 137,
                            Sig::TERM => 143,
                            _ => -1,
                        });
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    Some(_) => {}
                }
            }

            _ = tokio::time::sleep(wait_budget), if close_after_control_at.is_some() => {}
        }
    }

    output.close(exit_code).await;
}

