// R20b split: ACP session state mutation helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_session_state.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation
//         +67% over QClaw 242 tolerance)
// Sub-domain D (6 fns): session state mutations from event stream —
//              drain_pending_session_metadata_updates,
//              discard_pending_session_updates_if_needed,
//              update_session_from_events (3 pub);
//              update_session_context_usage,
//              update_session_available_commands,
//              update_session_config_options (3 file-local).
// R20b sibling files:
//             manager_session_helpers_identity.rs (sub-domain A)
//             manager_session_helpers_session_response.rs (sub-domain B + C)
//                — calls update_session_from_events from sub-domain D.
// R19 sibling files (consumers of sub-domain D fns):
//             manager_prompt.rs (discard_pending_session_updates_if_needed,
//                                 update_session_from_events)
//             manager_session.rs (drain_pending_session_metadata_updates)
//             manager_session_read.rs (in impl/r20a-manager-session-split
//                                       branch; not in this worktree; the
//                                       R20a branch will need to rebase /
//                                       re-apply the same use change when
//                                       R20a is merged).
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::{
    AcpRemoteSession, LOAD_REPLAY_DRAIN_MAX_DURATION, LOAD_REPLAY_DRAIN_QUIET_WINDOW,
    SESSION_METADATA_DRAIN_MAX_DURATION, SESSION_METADATA_DRAIN_QUIET_WINDOW,
};
use super::manager_errors::protocol_error;
use super::stream::{acp_dispatch_to_stream_events_with_tracker, AcpClientStreamEvent, AcpToolCallTracker};
use agent_client_protocol::SessionMessage;
use northhing_core::util::errors::NortHingResult;
use std::time::Instant;
use tracing::{debug, info, warn};

pub async fn drain_pending_session_metadata_updates(session: &mut AcpRemoteSession) -> NortHingResult<()> {
    let started_at = Instant::now();
    let mut drained_count = 0usize;
    let mut tool_call_tracker = AcpToolCallTracker::new();

    while started_at.elapsed() < SESSION_METADATA_DRAIN_MAX_DURATION {
        let update = {
            let Some(active) = session.active.as_mut() else {
                return Ok(());
            };
            tokio::time::timeout(SESSION_METADATA_DRAIN_QUIET_WINDOW, active.read_update()).await
        };

        match update {
            Ok(Ok(SessionMessage::SessionMessage(dispatch))) => {
                let events = acp_dispatch_to_stream_events_with_tracker(dispatch, &mut tool_call_tracker).await?;
                update_session_from_events(session, &events);
                drained_count += 1;
            }
            Ok(Ok(SessionMessage::StopReason(_))) => {
                drained_count += 1;
            }
            Ok(Ok(_)) => {
                drained_count += 1;
            }
            Ok(Err(error)) => return Err(protocol_error(error)),
            Err(_) => break,
        }
    }

    if drained_count > 0 {
        debug!("Drained ACP session metadata updates: count={}", drained_count);
    }

    Ok(())
}

pub async fn discard_pending_session_updates_if_needed(session: &mut AcpRemoteSession) {
    if !session.discard_pending_updates_before_next_prompt {
        return;
    }

    session.discard_pending_updates_before_next_prompt = false;
    let started_at = Instant::now();
    let mut discarded_count = 0usize;
    while started_at.elapsed() < LOAD_REPLAY_DRAIN_MAX_DURATION {
        let update = {
            let Some(active) = session.active.as_mut() else {
                return;
            };
            tokio::time::timeout(LOAD_REPLAY_DRAIN_QUIET_WINDOW, active.read_update()).await
        };

        match update {
            Ok(Ok(SessionMessage::SessionMessage(dispatch))) => {
                let mut tracker = AcpToolCallTracker::new();
                if let Ok(events) = acp_dispatch_to_stream_events_with_tracker(dispatch, &mut tracker).await {
                    update_session_from_events(session, &events);
                }
                discarded_count += 1;
            }
            Ok(Ok(SessionMessage::StopReason(_))) => {
                discarded_count += 1;
            }
            Ok(Ok(_)) => {
                discarded_count += 1;
            }
            Ok(Err(error)) => {
                warn!(
                    "Failed to discard ACP load replay update before prompt: error={}",
                    error
                );
                break;
            }
            Err(_) => break,
        }
    }

    if discarded_count > 0 {
        info!(
            "Discarded ACP load replay updates before prompt: count={}",
            discarded_count
        );
    }
}

pub fn update_session_from_events(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent]) {
    update_session_context_usage(session, events);
    update_session_available_commands(session, events);
    update_session_config_options(session, events);
}

fn update_session_context_usage(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent]) {
    let Some(usage) = events.iter().rev().find_map(|event| match event {
        AcpClientStreamEvent::ContextUsageUpdated(usage) => Some(usage.clone()),
        _ => None,
    }) else {
        return;
    };

    session.context_usage = Some(usage);
}

fn update_session_available_commands(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent]) {
    let Some(commands) = events.iter().rev().find_map(|event| match event {
        AcpClientStreamEvent::AvailableCommandsUpdated(commands) => Some(commands.clone()),
        _ => None,
    }) else {
        return;
    };

    session.available_commands = commands;
}

fn update_session_config_options(session: &mut AcpRemoteSession, events: &[AcpClientStreamEvent]) {
    let Some(options) = events.iter().rev().find_map(|event| match event {
        AcpClientStreamEvent::ConfigOptionsUpdated(options) => Some(options.clone()),
        _ => None,
    }) else {
        return;
    };

    session.config_options = options;
}
