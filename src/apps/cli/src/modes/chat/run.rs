//! Chat mode main event loop
//!
//! Owns the terminal setup/teardown and the central render+event+poll loop.
//! Delegates keyboard/non-keyboard event handling to `input.rs` and command
//! execution to `commands.rs`.
use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use northhing_events::AgenticEvent;

use crate::agent::Agent;
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;
use crate::ui::theme::{resolve_appearance, resolve_effective_color_scheme, EffectiveColorScheme, Theme};
use crate::ui::{init_terminal, restore_terminal};

use super::{agent_display_name, ChatExitReason, ChatMode, PendingMcpOp};

/// Spinner/UI redraw interval while a turn is processing.
const SPINNER_REDRAW_INTERVAL_MS: u64 = 100;
/// Coalesce rapid resize bursts to reduce flicker during window drag.
const RESIZE_REDRAW_DEBOUNCE_MS: u64 = 75;

/// Main event loop — extracted from the original `ChatMode::run` body.
pub fn run_loop(
    this: &mut ChatMode,
    existing_terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
) -> Result<ChatExitReason> {
    tracing::info!("Starting Chat mode, Agent: {}", this.agent_type);
    if let Some(ws) = &this.workspace {
        tracing::info!("Workspace: {}", ws);
    }

    let mut terminal = match existing_terminal {
        Some(t) => t,
        None => init_terminal()?,
    };

    let appearance = resolve_appearance(&this.config.ui.theme);
    let scheme = resolve_effective_color_scheme(&this.config.ui.color_scheme);
    let base_is_light = appearance.is_light();
    let base = match (base_is_light, scheme) {
        (_, EffectiveColorScheme::Monochrome) => Theme::monochrome(),
        (true, EffectiveColorScheme::Ansi16) => Theme::light_ansi16(),
        (true, EffectiveColorScheme::Truecolor) => Theme::light(),
        (false, EffectiveColorScheme::Ansi16) => Theme::dark_ansi16(),
        (false, EffectiveColorScheme::Truecolor) => Theme::dark(),
    };
    let theme = this.resolve_configured_theme(base, appearance, scheme);
    let mut chat_view = ChatView::new(theme);

    // Create or restore core session
    let rt_handle = tokio::runtime::Handle::current();

    let (mut session_id, mut chat_state): (String, ChatState) = if let Some(ref restore_id) = this.restore_session_id {
        // Restore existing session
        tracing::info!("Restoring session: {}", restore_id);
        let agent = this.agent.clone();
        let rid = restore_id.clone();
        let agent_type = this.agent_type.clone();
        let workspace = this.workspace.clone();

        tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                // Restore session in core (loads metadata, messages, managers)
                agent.restore_session(&rid).await?;

                // Prefer session's stored workspace_path over startup workspace
                let effective_workspace = agent
                    .coordinator()
                    .session_manager()
                    .get_session(&rid)
                    .and_then(|s| s.config.workspace_path.clone())
                    .or(workspace);

                // Load historical messages for UI display
                let messages = agent.coordinator().get_messages(&rid).await.unwrap_or_default();

                let state = ChatState::from_core_messages(
                    rid.clone(),
                    format!("Restored Session"),
                    agent_type,
                    effective_workspace,
                    &messages,
                );

                tracing::info!("Session restored: {}, {} messages loaded", rid, messages.len());

                Ok::<_, anyhow::Error>((rid, state))
            })
        })?
    } else {
        // Create new session
        let session_id =
            tokio::task::block_in_place(|| rt_handle.block_on(this.agent.ensure_session(&this.agent_type)))?;
        tracing::info!("Core session ready: {}", session_id);

        let state = ChatState::new(
            session_id.clone(),
            format!("CLI Session"),
            this.agent_type.clone(),
            this.workspace.clone(),
        );
        (session_id, state)
    };

    // Keep ChatMode workspace in sync with the session's effective workspace
    this.workspace = chat_state.workspace.clone();

    // Load current model name for display
    this.load_current_model_name(&mut chat_state, &rt_handle);

    if this.agent_type == "HarmonyOSDev" {
        let deveco_home = std::env::var("DEVECO_HOME").ok();
        let missing = deveco_home.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        if missing {
            chat_state.add_system_message(
                "HarmonyOSDev tip: HmosCompilation requires DEVECO_HOME (DevEco Studio install path). If compilation fails, set DEVECO_HOME and restart the terminal."
                    .to_string(),
            );
        }
    }

    // Send initial prompt if provided (from startup page input)
    if let Some(prompt) = this.initial_prompt.take() {
        tracing::info!("Sending initial prompt: {}", prompt);
        if prompt.starts_with('/') {
            // Slash commands will be handled in the main loop
            chat_view.text_input.set_text(&prompt);
        } else {
            let display_name = agent_display_name(&this.agent_type);
            chat_view.set_status(Some(format!("{} is thinking...", display_name)));

            let agent = this.agent.clone();
            let agent_type = this.agent_type.clone();
            match tokio::task::block_in_place(|| rt_handle.block_on(agent.send_message(prompt, &agent_type))) {
                Ok(turn_id) => {
                    tracing::info!("Started initial turn: {}", turn_id);
                }
                Err(e) => {
                    tracing::error!("Failed to send initial prompt: {}", e);
                    chat_view.set_status(Some(format!("Error: {}", e)));
                }
            }
        }
    }

    let event_queue = this.agent.event_queue().clone();

    let mut exit_reason = ChatExitReason::Quit;
    let mut should_quit = false;
    let mut needs_redraw = true;
    let mut subagent_parent_tools: HashMap<String, String> = HashMap::new();
    let mut last_spinner_redraw = Instant::now();
    let mut pending_resize_at: Option<Instant> = None;
    let spinner_redraw_interval = Duration::from_millis(SPINNER_REDRAW_INTERVAL_MS);
    let resize_redraw_debounce = Duration::from_millis(RESIZE_REDRAW_DEBOUNCE_MS);

    while !should_quit {
        // Coalesce rapid resize bursts before invalidating caches and redrawing.
        if let Some(last_resize_at) = pending_resize_at {
            if last_resize_at.elapsed() >= resize_redraw_debounce {
                chat_view.invalidate_lines_cache();
                needs_redraw = true;
                pending_resize_at = None;
            }
        }

        // Keep spinner animation smooth without forcing full redraw every loop.
        // Pause spinner updates while resize is still being debounced.
        if pending_resize_at.is_some() {
            last_spinner_redraw = Instant::now();
        } else if chat_state.is_processing {
            if last_spinner_redraw.elapsed() >= spinner_redraw_interval {
                needs_redraw = true;
                last_spinner_redraw = Instant::now();
            }
        } else {
            last_spinner_redraw = Instant::now();
        }

        // Poll completion of non-blocking MCP operations before rendering.
        if this.poll_mcp_task_completion(&mut chat_view, &mut chat_state, &rt_handle) {
            needs_redraw = true;
        }

        let mut did_render_this_loop = false;
        if needs_redraw {
            terminal.draw(|frame| {
                chat_view.render(frame, &chat_state);
            })?;
            needs_redraw = false;
            did_render_this_loop = true;
        }

        // 1.5. Execute pending MCP operations (after render so loading state is visible)
        if let Some(op) = this.pending_mcp_op.take() {
            if !did_render_this_loop {
                terminal.draw(|frame| {
                    chat_view.render(frame, &chat_state);
                })?;
            }
            match op {
                PendingMcpOp::Toggle(server_id) => {
                    this.execute_mcp_toggle(&server_id, &mut chat_view, &mut chat_state, &rt_handle);
                }
                PendingMcpOp::Add { name, config_json } => {
                    this.execute_mcp_add(&name, &config_json, &mut chat_view, &mut chat_state, &rt_handle);
                }
                PendingMcpOp::Delete(server_id) => {
                    this.execute_mcp_delete(&server_id, &mut chat_view, &mut chat_state, &rt_handle);
                }
            }
            needs_redraw = true;
        }

        // 2. Process core events (non-blocking)
        let events = tokio::task::block_in_place(|| rt_handle.block_on(event_queue.dequeue_batch(20)));
        for envelope in events {
            let event = &envelope.event;

            if let AgenticEvent::SubagentSessionLinked {
                session_id: subagent_session_id,
                parent_session_id,
                parent_tool_call_id,
                ..
            } = event
            {
                if parent_session_id == &session_id {
                    subagent_parent_tools.insert(subagent_session_id.clone(), parent_tool_call_id.clone());
                }
                continue;
            }

            // Check if this is a subagent event that belongs to our session
            if event.session_id() != Some(&session_id) {
                // Check if this event was emitted by a subagent whose parent is in our session
                if let Some(parent_tool_call_id) = event
                    .session_id()
                    .and_then(|event_session_id| subagent_parent_tools.get(event_session_id))
                {
                    // Forward subagent event to the parent Task tool for progress display
                    chat_state.handle_subagent_event(parent_tool_call_id, event);
                    chat_view.invalidate_lines_cache();
                    needs_redraw = true;
                }
                continue;
            }

            tracing::debug!("Processing core event: {:?}", event);

            match event {
                AgenticEvent::DialogTurnStarted {
                    turn_id, user_input, ..
                } => {
                    chat_state.handle_turn_started(turn_id, user_input);
                    chat_view.invalidate_lines_cache();
                    needs_redraw = true;
                }

                AgenticEvent::TextChunk { turn_id, text, .. } => {
                    if chat_state.current_turn_id() == Some(turn_id.as_str()) {
                        chat_state.handle_text_chunk(text);
                        chat_view.invalidate_lines_cache();
                        needs_redraw = true;
                    } else {
                        tracing::debug!(
                            "Ignoring TextChunk for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                    }
                }

                AgenticEvent::ThinkingChunk { turn_id, content, .. } => {
                    if chat_state.current_turn_id() == Some(turn_id.as_str()) {
                        chat_state.handle_thinking_chunk(content);
                        chat_view.invalidate_lines_cache();
                        needs_redraw = true;
                    } else {
                        tracing::debug!(
                            "Ignoring ThinkingChunk for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                    }
                }

                AgenticEvent::ToolEvent {
                    turn_id, tool_event, ..
                } => {
                    if chat_state.current_turn_id() != Some(turn_id.as_str()) {
                        tracing::debug!(
                            "Ignoring ToolEvent for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                        continue;
                    }
                    chat_state.handle_tool_event(tool_event);
                    chat_view.invalidate_lines_cache();
                    needs_redraw = true;
                }

                AgenticEvent::DialogTurnCompleted {
                    turn_id,
                    total_rounds,
                    total_tools,
                    ..
                } => {
                    if chat_state.current_turn_id() == Some(turn_id.as_str()) {
                        chat_state.handle_turn_completed(*total_rounds, *total_tools);
                        chat_view.invalidate_lines_cache();
                        chat_view.set_status(None);
                        needs_redraw = true;
                        tracing::info!("Dialog turn completed");
                    } else {
                        tracing::debug!(
                            "Ignoring DialogTurnCompleted for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                    }
                }

                AgenticEvent::DialogTurnFailed { turn_id, error, .. } => {
                    if chat_state.current_turn_id() == Some(turn_id.as_str()) {
                        chat_state.handle_turn_failed(error);
                        chat_view.invalidate_lines_cache();
                        chat_view.set_status(Some(format!("Error: {}", error)));
                        needs_redraw = true;
                        tracing::error!("Dialog turn failed: {}", error);
                    } else {
                        tracing::debug!(
                            "Ignoring DialogTurnFailed for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                    }
                }

                AgenticEvent::DialogTurnCancelled { turn_id, .. } => {
                    let active_turn_id = chat_state.current_turn_id();
                    if active_turn_id.is_none() || active_turn_id == Some(turn_id.as_str()) {
                        chat_state.handle_turn_cancelled();
                        chat_view.invalidate_lines_cache();
                        chat_view.set_status(Some("Cancelled".to_string()));
                        needs_redraw = true;
                        tracing::info!("Dialog turn cancelled");
                    } else {
                        tracing::debug!(
                            "Ignoring DialogTurnCancelled for non-active turn: active={:?}, event={}",
                            chat_state.current_turn_id(),
                            turn_id
                        );
                    }
                }

                AgenticEvent::TokenUsageUpdated {
                    turn_id, total_tokens, ..
                } => {
                    if chat_state.current_turn_id() == Some(turn_id.as_str()) {
                        chat_state.handle_token_usage(*total_tokens);
                        needs_redraw = true;
                    }
                }

                AgenticEvent::SystemError { error, .. } => {
                    chat_state.add_system_message(format!("[System error: {}]", error));
                    chat_view.invalidate_lines_cache();
                    chat_view.set_status(Some(format!("System error: {}", error)));
                    needs_redraw = true;
                    tracing::error!("System error: {}", error);
                }

                // Other events we don't need to handle in the UI
                _ => {}
            }
        }

        // 3. Process terminal input
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Ok(first_event) = crossterm::event::read() {
                // Batch-collect all immediately available events (paste detection).
                // On Windows, bracketed paste is broken (crossterm #962) and
                // pasted text arrives as rapid Key events with Enter mixed in.
                let mut events = vec![first_event];
                // Short wait to let rapid paste events arrive in the same batch.
                // Duration::ZERO would split pastes across loop iterations.
                while crossterm::event::poll(Duration::from_millis(5))? {
                    if let Ok(ev) = crossterm::event::read() {
                        events.push(ev);
                    } else {
                        break;
                    }
                }

                // Detect if this batch looks like a paste: multiple Key events
                // that include at least one Enter and at least one printable char.
                let is_paste_batch = if events.len() > 2 {
                    let mut has_enter = false;
                    let mut has_char = false;
                    for ev in &events {
                        if let crossterm::event::Event::Key(k) = ev {
                            if k.kind == crossterm::event::KeyEventKind::Press
                                || k.kind == crossterm::event::KeyEventKind::Repeat
                            {
                                match k.code {
                                    crossterm::event::KeyCode::Enter => has_enter = true,
                                    crossterm::event::KeyCode::Char(c) if !c.is_control() => has_char = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                    has_enter && has_char
                } else {
                    false
                };

                if is_paste_batch {
                    // Treat entire batch as pasted text
                    let mut paste_buf = String::new();
                    let mut non_key_events = Vec::new();
                    for ev in events {
                        match ev {
                            crossterm::event::Event::Key(k)
                                if k.kind == crossterm::event::KeyEventKind::Press
                                    || k.kind == crossterm::event::KeyEventKind::Repeat =>
                            {
                                match k.code {
                                    crossterm::event::KeyCode::Char(c) => paste_buf.push(c),
                                    crossterm::event::KeyCode::Enter => paste_buf.push('\n'),
                                    _ => {}
                                }
                            }
                            other => non_key_events.push(other),
                        }
                    }
                    if !paste_buf.is_empty() {
                        let normalized = paste_buf.replace("\r\n", "\n").replace('\r', "\n");
                        for c in normalized.chars() {
                            chat_view.handle_char(c);
                        }
                        needs_redraw = true;
                    }
                    // Process any non-key events that were mixed in
                    for ev in non_key_events {
                        let outcome = ChatMode::handle_non_key_event(
                            ev,
                            this,
                            &mut chat_view,
                            &mut chat_state,
                            &mut session_id,
                            &rt_handle,
                            &mut should_quit,
                            &mut exit_reason,
                        )?;
                        if outcome.request_redraw {
                            needs_redraw = true;
                        }
                        if outcome.resize_seen {
                            pending_resize_at = Some(Instant::now());
                        }
                    }
                } else {
                    // Normal single/few events — process each individually
                    for ev in events {
                        match ev {
                            crossterm::event::Event::Key(key) => {
                                if let Some(reason) =
                                    this.handle_key_event(key, &mut chat_view, &mut chat_state, &rt_handle)?
                                {
                                    ChatMode::apply_exit_reason(
                                        reason,
                                        this,
                                        &mut chat_view,
                                        &mut chat_state,
                                        &mut session_id,
                                        &rt_handle,
                                        &mut should_quit,
                                        &mut exit_reason,
                                    );
                                }
                                if key.kind == crossterm::event::KeyEventKind::Press
                                    || key.kind == crossterm::event::KeyEventKind::Repeat
                                {
                                    needs_redraw = true;
                                }
                            }
                            other => {
                                let outcome = ChatMode::handle_non_key_event(
                                    other,
                                    this,
                                    &mut chat_view,
                                    &mut chat_state,
                                    &mut session_id,
                                    &rt_handle,
                                    &mut should_quit,
                                    &mut exit_reason,
                                )?;
                                if outcome.request_redraw {
                                    needs_redraw = true;
                                }
                                if outcome.resize_seen {
                                    pending_resize_at = Some(Instant::now());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    restore_terminal(terminal)?;
    tracing::info!("Chat mode exited");

    Ok(exit_reason)
}
