// R20b split: ACP session response + turn data drain helpers.
// File: src/crates/interfaces/acp/src/client/manager_session_helpers_session_response.rs
// Origin: manager_session_helpers.rs (405 lines, QClaw R20a P1 D-deviation
//         +67% over QClaw 242 tolerance)
// Sub-domain B (2 pub fns): session response builders from
//              AcpRemoteSession — new_session_response_from_load,
//              new_session_response_from_resume.
// Sub-domain C (2 pub + 2 file-local fns): turn data drain —
//              drain_pending_turn_updates, read_turn_to_string (pub);
//              drain_pending_turn_text, append_agent_text (file-local).
// R20b sibling files:
//             manager_session_helpers_identity.rs (sub-domain A)
//             manager_session_helpers_session_state.rs (sub-domain D)
//                — provides update_session_from_events used by C pub fns.
// R19 sibling files (consumers of B + C fns):
//             manager_session.rs (B: load/resume builders)
//             manager_prompt.rs (C: turn drain + read_turn_to_string)
//             manager_session_resolve.rs (in impl/r20a-manager-session-split
//                                          branch; not in this worktree; the
//                                          R20a branch will need to rebase /
//                                          re-apply the same use change when
//                                          R20a is merged).
// All method bodies are moved verbatim from main. No behavior change.

use super::manager::{AcpRemoteSession, TURN_COMPLETION_DRAIN_MAX_DURATION, TURN_COMPLETION_DRAIN_QUIET_WINDOW};
use super::manager_errors::protocol_error;
use super::manager_session_helpers_session_state::update_session_from_events;
use super::stream::{
    acp_dispatch_to_stream_events_with_tracker, AcpClientStreamEvent, AcpStreamRoundTracker, AcpToolCallTracker,
};
use agent_client_protocol::schema::{LoadSessionResponse, NewSessionResponse, ResumeSessionResponse};
use agent_client_protocol::SessionMessage;
use northhing_core::util::errors::{NortHingError, NortHingResult};
use std::time::Instant;
use tracing::debug;

// =================================================================
// Sub-domain B: Session response builders (2 pub fns)
// =================================================================

pub fn new_session_response_from_load(remote_session_id: &str, response: LoadSessionResponse) -> NewSessionResponse {
    NewSessionResponse::new(remote_session_id.to_string())
        .modes(response.modes)
        .models(response.models)
        .config_options(response.config_options)
        .meta(response.meta)
}

pub fn new_session_response_from_resume(
    remote_session_id: &str,
    response: ResumeSessionResponse,
) -> NewSessionResponse {
    NewSessionResponse::new(remote_session_id.to_string())
        .modes(response.modes)
        .models(response.models)
        .config_options(response.config_options)
        .meta(response.meta)
}

// =================================================================
// Sub-domain C: Turn data drain (2 pub + 2 file-local fns)
// =================================================================

pub async fn drain_pending_turn_updates<F>(
    session: &mut AcpRemoteSession,
    tool_call_tracker: &mut AcpToolCallTracker,
    round_tracker: &mut AcpStreamRoundTracker,
    on_event: &mut F,
) -> NortHingResult<()>
where
    F: FnMut(AcpClientStreamEvent) -> NortHingResult<()> + Send,
{
    let started_at = Instant::now();
    let mut drained_count = 0usize;
    while started_at.elapsed() < TURN_COMPLETION_DRAIN_MAX_DURATION {
        let update = {
            let Some(active) = session.active.as_mut() else {
                return Ok(());
            };
            tokio::time::timeout(TURN_COMPLETION_DRAIN_QUIET_WINDOW, active.read_update()).await
        };

        match update {
            Ok(Ok(SessionMessage::SessionMessage(dispatch))) => {
                let events = acp_dispatch_to_stream_events_with_tracker(dispatch, tool_call_tracker).await?;
                update_session_from_events(session, &events);
                for event in events {
                    for event in round_tracker.apply(event) {
                        on_event(event)?;
                    }
                }
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
        debug!("Drained ACP turn updates after stop reason: count={}", drained_count);
    }

    Ok(())
}

pub async fn read_turn_to_string(session: &mut AcpRemoteSession) -> NortHingResult<String> {
    let mut output = String::new();
    let mut tool_call_tracker = AcpToolCallTracker::new();
    loop {
        let message = {
            let active = session
                .active
                .as_mut()
                .ok_or_else(|| NortHingError::service("ACP session was not initialized"))?;
            active.read_update().await.map_err(protocol_error)?
        };

        match message {
            SessionMessage::SessionMessage(dispatch) => {
                let events = acp_dispatch_to_stream_events_with_tracker(dispatch, &mut tool_call_tracker).await?;
                update_session_from_events(session, &events);
                append_agent_text(&mut output, events);
            }
            SessionMessage::StopReason(_) => {
                drain_pending_turn_text(session, &mut tool_call_tracker, &mut output).await?;
                break;
            }
            _ => {}
        }
    }
    Ok(output)
}

async fn drain_pending_turn_text(
    session: &mut AcpRemoteSession,
    tool_call_tracker: &mut AcpToolCallTracker,
    output: &mut String,
) -> NortHingResult<()> {
    let started_at = Instant::now();
    let mut drained_count = 0usize;
    while started_at.elapsed() < TURN_COMPLETION_DRAIN_MAX_DURATION {
        let update = {
            let Some(active) = session.active.as_mut() else {
                return Ok(());
            };
            tokio::time::timeout(TURN_COMPLETION_DRAIN_QUIET_WINDOW, active.read_update()).await
        };

        match update {
            Ok(Ok(SessionMessage::SessionMessage(dispatch))) => {
                let events = acp_dispatch_to_stream_events_with_tracker(dispatch, tool_call_tracker).await?;
                update_session_from_events(session, &events);
                append_agent_text(output, events);
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
        debug!("Drained ACP text updates after stop reason: count={}", drained_count);
    }

    Ok(())
}

fn append_agent_text(output: &mut String, events: Vec<AcpClientStreamEvent>) {
    for event in events {
        if let AcpClientStreamEvent::AgentText(text) = event {
            output.push_str(&text);
        }
    }
}
