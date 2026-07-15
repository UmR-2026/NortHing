//! Bot command question reply handling (Round 14 split).
//!
//! Owns:
//! - `handle_question_reply` — multi-question answer flow with custom-text fallback
//! - `submit_question_answers` — submit to the user_input_manager
//!
//! Lifted from `command_router_dispatch` to keep the dispatcher file under the
//! 800-line cap (Round 14 D-deviation mitigation).

use serde_json::Value;

use super::command_router::{BotQuestion, HandleResult};
use super::command_router_dispatch::pending_invalid;
use super::command_router_state::{BotChatState, PendingAction};
use super::command_router_util::{parse_question_numbers, result_from_menu};
use super::command_router_view::build_question_view;
use super::locale::BotStrings;
use super::menu::MenuView;

pub(super) async fn handle_question_reply(
    state: &mut BotChatState,
    tool_id: String,
    questions: Vec<BotQuestion>,
    current_index: usize,
    mut answers: Vec<Value>,
    awaiting_custom_text: bool,
    pending_answer: Option<Value>,
    message: &str,
    s: &'static BotStrings,
) -> HandleResult {
    let Some(question) = questions.get(current_index).cloned() else {
        return result_from_menu(state, MenuView::plain(s.question_invalid_state));
    };

    if awaiting_custom_text {
        let custom_text = message.trim();
        if custom_text.is_empty() {
            state.set_pending(PendingAction::AskUserQuestion {
                tool_id,
                questions,
                current_index,
                answers,
                awaiting_custom_text: true,
                pending_answer,
            });
            return result_from_menu(state, MenuView::plain(s.question_custom_required));
        }
        let final_value = match pending_answer {
            Some(Value::Array(existing)) => {
                let mut values: Vec<Value> = existing.into_iter().filter(|v| v.as_str() != Some("Other")).collect();
                values.push(Value::String(custom_text.to_string()));
                Value::Array(values)
            }
            _ => Value::String(custom_text.to_string()),
        };
        answers.push(final_value);
    } else {
        let selections = match parse_question_numbers(message) {
            Some(values) => values,
            None => {
                state.set_pending(PendingAction::AskUserQuestion {
                    tool_id,
                    questions,
                    current_index,
                    answers,
                    awaiting_custom_text: false,
                    pending_answer: None,
                });
                return Box::pin(pending_invalid(state, s)).await;
            }
        };
        if !question.multi_select && selections.len() != 1 {
            state.set_pending(PendingAction::AskUserQuestion {
                tool_id,
                questions,
                current_index,
                answers,
                awaiting_custom_text: false,
                pending_answer: None,
            });
            return Box::pin(pending_invalid(state, s)).await;
        }
        let other_index = question.options.len() + 1;
        let mut labels = Vec::new();
        let mut includes_other = false;
        for selection in selections {
            if selection == other_index {
                includes_other = true;
                labels.push(Value::String(s.item_other.to_string()));
            } else if selection >= 1 && selection <= question.options.len() {
                labels.push(Value::String(question.options[selection - 1].label.clone()));
            } else {
                state.set_pending(PendingAction::AskUserQuestion {
                    tool_id,
                    questions,
                    current_index,
                    answers,
                    awaiting_custom_text: false,
                    pending_answer: None,
                });
                let _ = other_index;
                return Box::pin(pending_invalid(state, s)).await;
            }
        }
        let pending_answer_next = if question.multi_select {
            Some(Value::Array(labels.clone()))
        } else {
            labels.into_iter().next()
        };
        if includes_other {
            state.set_pending(PendingAction::AskUserQuestion {
                tool_id,
                questions,
                current_index,
                answers,
                awaiting_custom_text: true,
                pending_answer: pending_answer_next,
            });
            return result_from_menu(state, MenuView::plain(s.question_custom_for_other_prefix));
        }
        answers.push(if question.multi_select {
            pending_answer_next.unwrap_or_else(|| Value::Array(Vec::new()))
        } else {
            pending_answer_next.unwrap_or_else(|| Value::String(String::new()))
        });
    }

    if current_index + 1 < questions.len() {
        let view = build_question_view(s, &questions, current_index + 1, false);
        state.set_pending(PendingAction::AskUserQuestion {
            tool_id,
            questions,
            current_index: current_index + 1,
            answers,
            awaiting_custom_text: false,
            pending_answer: None,
        });
        return result_from_menu(state, view);
    }

    state.clear_pending();
    submit_question_answers(&tool_id, &answers, s).await
}

pub(super) async fn submit_question_answers(tool_id: &str, answers: &[Value], s: &'static BotStrings) -> HandleResult {
    use crate::agentic::tools::user_input_manager::user_input_manager;

    let mut payload = serde_json::Map::new();
    for (idx, value) in answers.iter().enumerate() {
        payload.insert(idx.to_string(), value.clone());
    }
    let manager = user_input_manager();
    match manager.send_answer(tool_id, Value::Object(payload)) {
        Ok(_) => HandleResult {
            reply: s.answers_submitted.to_string(),
            actions: vec![],
            forward_to_session: None,
            menu: MenuView::plain(s.answers_submitted),
        },
        Err(e) => HandleResult {
            reply: format!("{}{e}", s.answers_submit_failed_prefix),
            actions: vec![],
            forward_to_session: None,
            menu: MenuView::plain(format!("{}{e}", s.answers_submit_failed_prefix)),
        },
    }
}
