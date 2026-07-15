//! Bot command router tests (Round 14 split).
//!
//! 4 test mods:
//! - `parse_command_tests` (12 tests)
//! - `state_tests` (3 tests)
//! - `menu_tests` (6 tests)
//! - `handle_chat_tests` (1 test)

use super::*;

use super::command_router::{parse_command, BotCommand};

use super::command_router_state::{now_secs, BotChatState, BotDisplayMode, PendingAction};

use super::command_router_dispatch::handle_chat;

use super::command_router_view::main_menu_view;

use super::locale::{strings_for, BotLanguage};

use super::menu::MenuView;

mod parse_command_tests {
    use super::*;

    #[test]
    fn numeric_menu_with_trailing_dot() {
        assert!(matches!(parse_command("1."), BotCommand::NumberSelection(1)));
        assert!(matches!(parse_command("2。"), BotCommand::NumberSelection(2)));
    }

    #[test]
    fn fullwidth_digit_one() {
        assert!(matches!(parse_command("１"), BotCommand::NumberSelection(1)));
    }

    #[test]
    fn zero_parsed_as_number_selection() {
        // `0` stays as a numeric selection so it can mean "next page" or
        // "back" depending on which pending action is active.  The
        // top-level "no pending" 鈫?main-menu fallback is implemented in
        // `handle_number`.
        assert!(matches!(parse_command("0"), BotCommand::NumberSelection(0)));
    }

    #[test]
    fn menu_aliases() {
        assert!(matches!(parse_command("/menu"), BotCommand::Menu));
        assert!(matches!(parse_command("/m"), BotCommand::Menu));
        assert!(matches!(parse_command("菜单"), BotCommand::Menu));
        assert!(matches!(parse_command("/start"), BotCommand::Menu));
    }

    #[test]
    fn settings_aliases() {
        assert!(matches!(parse_command("/settings"), BotCommand::Settings));
        assert!(matches!(parse_command("设置"), BotCommand::Settings));
    }

    #[test]
    fn verbose_concise_real_commands() {
        assert!(matches!(parse_command("/verbose"), BotCommand::SetVerbose(true)));
        assert!(matches!(parse_command("/concise"), BotCommand::SetVerbose(false)));
    }

    #[test]
    fn switch_aliases() {
        assert!(matches!(parse_command("/switch"), BotCommand::SwitchContext));
        assert!(matches!(parse_command("/switch_workspace"), BotCommand::SwitchContext));
        assert!(matches!(parse_command("/switch_assistant"), BotCommand::SwitchContext));
        assert!(matches!(parse_command("切换"), BotCommand::SwitchContext));
    }

    #[test]
    fn new_session_aliases() {
        assert!(matches!(parse_command("/new"), BotCommand::NewSession));
        assert!(matches!(parse_command("/new_code_session"), BotCommand::NewCodeSession));
        assert!(matches!(
            parse_command("/new_cowork_session"),
            BotCommand::NewCoworkSession
        ));
        assert!(matches!(parse_command("/new_claw_session"), BotCommand::NewClawSession));
    }

    #[test]
    fn resume_aliases() {
        assert!(matches!(parse_command("/resume"), BotCommand::ResumeSession));
        assert!(matches!(parse_command("/r"), BotCommand::ResumeSession));
        assert!(matches!(parse_command("/resume_session"), BotCommand::ResumeSession));
    }

    #[test]
    fn cancel_aliases() {
        assert!(matches!(parse_command("/cancel"), BotCommand::CancelTask(None)));
        match parse_command("/cancel_task turn_abc") {
            BotCommand::CancelTask(Some(id)) => assert_eq!(id, "turn_abc"),
            _ => panic!("expected cancel task with id"),
        }
    }

    #[test]
    fn pairing_code_detected() {
        match parse_command("123456") {
            BotCommand::PairingCode(c) => assert_eq!(c, "123456"),
            _ => panic!("expected pairing code"),
        }
    }

    #[test]
    fn chat_message_fallback() {
        assert!(matches!(parse_command("hello world"), BotCommand::ChatMessage(_)));
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn pending_expires_after_ttl() {
        let mut state = BotChatState::new("c".into());
        state.set_pending(PendingAction::SelectWorkspace { options: vec![] });
        assert!(state.pending_action.is_some());
        assert!(!state.pending_expired());
        state.pending_expires_at = now_secs() - 1;
        assert!(state.pending_expired());
    }

    #[test]
    fn active_workspace_path_prefers_pro_workspace_then_assistant() {
        let mut state = BotChatState::new("c".into());
        assert_eq!(state.active_workspace_path(), None);

        state.current_assistant = Some("/tmp/assistant-ws".to_string());
        assert_eq!(
            state.active_workspace_path().as_deref(),
            Some("/tmp/assistant-ws"),
            "assistant path is the fallback when no Pro workspace is set"
        );

        state.current_workspace = Some("/tmp/pro-ws".to_string());
        assert_eq!(
            state.active_workspace_path().as_deref(),
            Some("/tmp/pro-ws"),
            "Pro workspace wins over the assistant path when both are set"
        );
    }

    #[test]
    fn clear_pending_resets_counters() {
        let mut state = BotChatState::new("c".into());
        state.set_pending(PendingAction::SelectWorkspace { options: vec![] });
        state.pending_invalid_count = 2;
        state.clear_pending();
        assert!(state.pending_action.is_none());
        assert_eq!(state.pending_invalid_count, 0);
        assert_eq!(state.pending_expires_at, 0);
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    #[test]
    fn main_menu_assistant_has_four_items() {
        let state = BotChatState::new("c".into());
        let view = main_menu_view(&state, strings_for(BotLanguage::ZhCN));
        assert_eq!(view.items.len(), 4);
        assert!(view.items.iter().any(|i| i.command == "/new"));
        assert!(view.items.iter().any(|i| i.command == "/resume"));
        assert!(view.items.iter().any(|i| i.command == "/switch"));
        assert!(view.items.iter().any(|i| i.command == "/settings"));
    }

    #[test]
    fn main_menu_expert_has_five_items() {
        let mut state = BotChatState::new("c".into());
        state.display_mode = BotDisplayMode::Pro;
        let view = main_menu_view(&state, strings_for(BotLanguage::ZhCN));
        assert_eq!(view.items.len(), 5);
        assert!(view.items.iter().any(|i| i.command == "/new_code_session"));
    }

    /// Main menu must NOT surface the random session UUID tail. The user
    /// only cares about the workspace / assistant name; the session ID is
    /// noise (see /resume for proper session management).
    #[test]
    fn main_menu_body_omits_session_id() {
        let mut state = BotChatState::new("c".into());
        state.current_assistant = Some("/tmp/my-assistant".to_string());
        state.current_assistant_name = Some("我的助理".to_string());
        state.current_session_id = Some("abcdef12-3456-7890-abcd-ef1234567890".to_string());
        let s = strings_for(BotLanguage::ZhCN);
        let view = main_menu_view(&state, s);
        let body = view.body.as_deref().unwrap_or("");
        assert!(
            !body.contains("567890") && !body.contains("ef1234567890"),
            "session UUID tail leaked into body: {body}"
        );
        assert!(body.contains("我的助理"), "assistant name missing: {body}");
    }

    /// Assistant mode must show the assistant's display name rather than
    /// the workspace directory's `file_name`. The directory is usually a
    /// generic "workspace" / "workspace-<uuid>" folder which is meaningless
    /// to the user.
    #[test]
    fn assistant_mode_body_uses_display_name_not_dir_name() {
        let mut state = BotChatState::new("c".into());
        state.current_assistant = Some("/tmp/northhing_assistants/workspace-abc123".to_string());
        state.current_assistant_name = Some("默认助理".to_string());
        let s = strings_for(BotLanguage::ZhCN);
        let view = main_menu_view(&state, s);
        let body = view.body.as_deref().unwrap_or("");
        assert!(
            body.contains("默认助理"),
            "expected assistant display name in body, got: {body}"
        );
        assert!(
            !body.contains("workspace-abc123"),
            "workspace directory name leaked into body: {body}"
        );
    }

    /// Expert mode keeps showing the workspace directory name (it IS the
    /// project name, which is what the user expects to see).
    #[test]
    fn expert_mode_body_still_uses_workspace_dir_name() {
        let mut state = BotChatState::new("c".into());
        state.display_mode = BotDisplayMode::Pro;
        state.current_workspace = Some("/tmp/projects/MyApp".to_string());
        // `current_assistant_name` should not affect Pro mode at all.
        state.current_assistant_name = Some("ignored".to_string());
        let s = strings_for(BotLanguage::ZhCN);
        let view = main_menu_view(&state, s);
        let body = view.body.as_deref().unwrap_or("");
        assert!(body.contains("MyApp"), "workspace name missing: {body}");
        assert!(!body.contains("ignored"), "assistant name leaked into Pro mode: {body}");
    }

    /// When the cached assistant display name is missing (e.g. legacy
    /// persisted state), fall back to the path's last segment instead of
    /// rendering an empty label or panicking.
    #[test]
    fn assistant_mode_body_falls_back_to_path_when_name_missing() {
        let mut state = BotChatState::new("c".into());
        state.current_assistant = Some("/tmp/my-assistant-folder".to_string());
        state.current_assistant_name = None;
        let s = strings_for(BotLanguage::ZhCN);
        let view = main_menu_view(&state, s);
        let body = view.body.as_deref().unwrap_or("");
        assert!(
            body.contains("my-assistant-folder"),
            "expected fallback to path tail, got: {body}"
        );
    }

    #[test]
    fn main_menu_body_omits_session_label_text() {
        let mut state = BotChatState::new("c".into());
        state.current_assistant = Some("/tmp/my-assistant".to_string());
        state.current_session_id = Some("session-xyz".to_string());
        let s = strings_for(BotLanguage::ZhCN);
        let view = main_menu_view(&state, s);
        let body = view.body.as_deref().unwrap_or("");
        assert!(
            !body.contains(s.current_session_label),
            "current_session_label leaked into body: {body}"
        );
    }
}

#[cfg(test)]
mod handle_chat_tests {
    use super::*;

    /// `handle_chat` must NOT push a "Processing…[Cancel Task]" interstitial
    /// to the user. The session manager queues new messages automatically;
    /// showing a cancel button just adds noise (and on WeChat costs a
    /// context_token slot per send).
    #[tokio::test]
    async fn chat_message_forwards_silently_without_processing_menu() {
        let mut state = BotChatState::new("peer".into());
        state.paired = true;
        state.current_assistant = Some("/tmp/a".into());
        state.current_session_id = Some("s1".into());
        let s = strings_for(BotLanguage::ZhCN);
        let result = handle_chat(&mut state, "hello northhing", vec![], s).await;

        assert!(
            result.forward_to_session.is_some(),
            "chat message must still be forwarded to the session"
        );
        assert!(
            result.menu.title.is_empty()
                && result.menu.items.is_empty()
                && result.menu.body.is_none()
                && result.menu.footer_hint.is_none(),
            "handle_chat must return an empty MenuView so adapters skip the send: {:?}",
            result.menu
        );
        assert!(
            !result.reply.contains(s.processing) && !result.reply.contains(s.queued),
            "processing/queued text must not be sent: {}",
            result.reply
        );
        assert!(
            !result.reply.contains(s.item_cancel_task),
            "cancel-task button must not be sent: {}",
            result.reply
        );
    }
}
