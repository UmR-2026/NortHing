//! Bot forwarded-turn execution (Round 14).
//!
//! Owns:
//! - `execute_forwarded_turn` (god method ~190 lines — splits into phase helpers if needed)
//! - `truncate_at_char_boundary` helper
//!
//! Split from `command_router.rs` in Round 14 to keep facade under 800 lines.
//! `execute_forwarded_turn` is the long-running async handler that runs
//! a forwarded user message against the session manager and streams the
//! final response back to the IM adapter.

use super::command_router::{
    BotAction, BotInteractiveRequest, BotQuestion, ForwardRequest, ForwardedTurnResult, PendingAction,
};
use super::command_router_view::build_question_view;
use super::locale::{current_bot_language, strings_for};

pub async fn execute_forwarded_turn(
    forward: ForwardRequest,
    interaction_handler: Option<super::command_router::BotInteractionHandler>,
    message_sender: Option<super::command_router::BotMessageSender>,
    verbose_mode: bool,
) -> ForwardedTurnResult {
    use crate::service::remote_connect::remote_server::{get_or_init_global_dispatcher, TrackerEvent};
    use northhing_services_integrations::remote_connect::RemoteConnectSubmissionSource;

    let language = current_bot_language().await;
    let s = strings_for(language);

    let dispatcher = get_or_init_global_dispatcher();
    let tracker = dispatcher.ensure_tracker(&forward.session_id);
    let mut event_rx = tracker.subscribe();

    let target_turn_id = forward.turn_id.clone();

    if let Err(e) = dispatcher
        .send_message(
            &forward.session_id,
            forward.content,
            Some(&forward.agent_type),
            forward.image_contexts,
            RemoteConnectSubmissionSource::Bot,
            Some(forward.turn_id.clone()),
        )
        .await
    {
        let msg = format!("{}{e}", s.send_failed_prefix);
        return ForwardedTurnResult {
            display_text: msg.clone(),
            full_text: msg,
        };
    }

    let result = tokio::time::timeout(std::time::Duration::from_secs(3600), async {
        let mut response = String::new();
        let mut thinking_buf = String::new();

        let streams_our_turn = || {
            tracker
                .snapshot_active_turn()
                .map(|st| st.turn_id == target_turn_id)
                .unwrap_or(false)
        };

        loop {
            match event_rx.recv().await {
                Ok(event) => match event {
                    TrackerEvent::ThinkingChunk(chunk) => {
                        if !streams_our_turn() {
                            continue;
                        }
                        thinking_buf.push_str(&chunk);
                    }
                    TrackerEvent::ThinkingEnd => {
                        if !streams_our_turn() {
                            continue;
                        }
                        if verbose_mode && !thinking_buf.trim().is_empty() {
                            if let Some(sender) = message_sender.as_ref() {
                                let content = truncate_at_char_boundary(&thinking_buf, 500);
                                let msg = format!("[{}] {}", s.thinking_label, content);
                                sender(msg).await;
                            }
                        }
                        thinking_buf.clear();
                    }
                    TrackerEvent::TextChunk(t) => {
                        if !streams_our_turn() {
                            continue;
                        }
                        response.push_str(&t);
                    }
                    TrackerEvent::ToolStarted {
                        tool_id,
                        tool_name,
                        params,
                    } => {
                        if !streams_our_turn() {
                            continue;
                        }
                        if tool_name == "AskUserQuestion" {
                            if let Some(questions_value) = params.and_then(|p| p.get("questions").cloned()) {
                                if let Ok(questions) = serde_json::from_value::<Vec<BotQuestion>>(questions_value) {
                                    let view = build_question_view(s, &questions, 0, false);
                                    let actions: Vec<BotAction> =
                                        view.items.iter().cloned().map(BotAction::from).collect();
                                    let request = BotInteractiveRequest {
                                        reply: view.render_text_block(),
                                        actions,
                                        menu: view,
                                        pending_action: PendingAction::AskUserQuestion {
                                            tool_id,
                                            questions,
                                            current_index: 0,
                                            answers: Vec::new(),
                                            awaiting_custom_text: false,
                                            pending_answer: None,
                                        },
                                    };
                                    if let Some(handler) = interaction_handler.as_ref() {
                                        handler(request).await;
                                    }
                                }
                            }
                        }
                    }
                    TrackerEvent::ToolCompleted { .. } => {}
                    TrackerEvent::TurnCompleted { turn_id } => {
                        if turn_id == target_turn_id {
                            break;
                        }
                    }
                    TrackerEvent::TurnFailed { turn_id, error } => {
                        if turn_id == target_turn_id {
                            let msg = format!("{}{}", s.error_prefix, error);
                            return ForwardedTurnResult {
                                display_text: msg.clone(),
                                full_text: msg,
                            };
                        }
                    }
                    TrackerEvent::TurnCancelled { turn_id } => {
                        if turn_id == target_turn_id {
                            return ForwardedTurnResult {
                                display_text: s.task_cancelled.to_string(),
                                full_text: s.task_cancelled.to_string(),
                            };
                        }
                    }
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Bot event receiver lagged by {n} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }

        let full_text = tracker.accumulated_text();
        let full_text = if full_text.is_empty() { response } else { full_text };

        let display_text = full_text.clone();

        ForwardedTurnResult {
            display_text: if display_text.is_empty() {
                s.no_response.to_string()
            } else {
                display_text
            },
            full_text,
        }
    })
    .await;

    result.unwrap_or_else(|_| ForwardedTurnResult {
        display_text: s.timeout_one_hour.to_string(),
        full_text: String::new(),
    })
}

fn truncate_at_char_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}
