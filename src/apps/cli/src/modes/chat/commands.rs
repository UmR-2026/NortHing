//! Chat mode command palette, slash commands, usage report, and message dispatch.
use anyhow::{anyhow, Result};

use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;

use crate::agent::Agent;

use northhing_core::agentic::persistence::PersistenceManager;
use northhing_core::service::session_usage::{
    generate_session_usage_report, render_usage_report_markdown, SessionUsageReportRequest,
};

use super::{agent_display_name, ChatExitReason, ChatMode, KEYBOARD_SHORTCUTS_HELP};

impl ChatMode {
    /// Handle command palette action
    pub(crate) fn handle_palette_action(
        &mut self,
        action_id: &str,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<Option<ChatExitReason>> {
        // Hide command palette but keep it in stack for back navigation
        // (unless the action switches away or exits)
        let keep_in_stack = matches!(action_id, "new_session" | "exit");
        if !keep_in_stack {
            chat_view.hide_command_palette();
        }

        match action_id {
            // Session group
            "new_session" => {
                if chat_state.is_processing {
                    chat_view.set_status(Some(
                        "Cannot start a new session while processing. Press Ctrl+C to cancel first.".to_string(),
                    ));
                    return Ok(None);
                }
                return Ok(Some(ChatExitReason::NewSession));
            }
            "sessions" => {
                if chat_state.is_processing {
                    chat_view.set_status(Some(
                        "Cannot switch sessions while processing. Press Ctrl+C to cancel first.".to_string(),
                    ));
                    return Ok(None);
                }
                self.show_session_selector(chat_view, chat_state, rt_handle);
            }
            "usage" => {
                self.show_usage_report(chat_view, chat_state, rt_handle);
            }
            // Prompt group
            "skills" => {
                self.show_skill_selector(chat_view, chat_state, rt_handle);
            }
            "subagents" => {
                self.show_subagent_selector(chat_view, chat_state, rt_handle);
            }
            // Models group
            "select_model" => {
                self.show_model_selector(chat_view, chat_state, rt_handle);
            }
            "add_model" => {
                chat_view.show_provider_selector();
            }
            // Agent group
            "switch_agent" => {
                self.show_agent_selector(chat_view, chat_state, rt_handle);
            }
            // MCP group
            "mcp_servers" => {
                self.show_mcp_selector(chat_view, chat_state, rt_handle);
            }
            // System group
            "help" => {
                chat_view.show_info_popup(KEYBOARD_SHORTCUTS_HELP.to_string());
            }
            "exit" => {
                if chat_state.is_processing {
                    tracing::info!("User requested cancellation via palette exit");
                    let agent = self.agent.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.cancel_current_turn().await {
                                tracing::error!("Failed to cancel turn: {}", e);
                            }
                        })
                    });
                }
                return Ok(Some(ChatExitReason::Quit));
            }
            _ => {
                chat_view.set_status(Some(format!("Unknown palette action: {}", action_id)));
            }
        }
        Ok(None)
    }

    /// Handle shortcut commands
    pub(crate) fn handle_command(
        &mut self,
        command: &str,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<Option<ChatExitReason>> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        match parts[0] {
            "/help" => {
                chat_view.show_info_popup(KEYBOARD_SHORTCUTS_HELP.to_string());
            }
            "/clear" => {
                if chat_state.is_processing {
                    tracing::info!("User requested cancellation via /clear");
                    let agent = self.agent.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.cancel_current_turn().await {
                                tracing::error!("Failed to cancel turn: {}", e);
                            }
                        })
                    });
                }
                chat_state.clear_messages();
                chat_view.clear_screen();
                chat_view.set_status(Some("Conversation cleared".to_string()));
            }
            "/agents" => {
                self.show_agent_selector(chat_view, chat_state, rt_handle);
            }
            "/models" => {
                self.show_model_selector(chat_view, chat_state, rt_handle);
            }
            "/theme" => {
                let themes = self.list_available_themes();
                chat_view.begin_theme_preview();
                chat_view.show_theme_selector(themes, Some(self.config.ui.theme_id.clone()));
                chat_view.set_status(Some("Theme selector: ↑↓ preview, Enter apply, Esc cancel".to_string()));
            }
            "/connect" => {
                chat_view.show_provider_selector();
            }
            "/new" => {
                if chat_state.is_processing {
                    chat_view.set_status(Some(
                        "Cannot start a new session while processing. Press Ctrl+C to cancel first.".to_string(),
                    ));
                    return Ok(None);
                }
                return Ok(Some(ChatExitReason::NewSession));
            }
            "/sessions" => {
                if chat_state.is_processing {
                    chat_view.set_status(Some(
                        "Cannot switch sessions while processing. Press Ctrl+C to cancel first.".to_string(),
                    ));
                    return Ok(None);
                }
                self.show_session_selector(chat_view, chat_state, rt_handle);
            }
            "/mcps" => {
                self.show_mcp_selector(chat_view, chat_state, rt_handle);
            }
            "/acp" => {
                chat_state.add_system_message(crate::acp_cli::acp_help_text("northhing-cli"));
                chat_view.set_status(Some(
                    "ACP setup added to the conversation. You can keep typing.".to_string(),
                ));
            }
            "/usage" => {
                self.show_usage_report(chat_view, chat_state, rt_handle);
            }
            "/init" => match crate::prompts::get_cli_prompt("init") {
                Some(prompt) => {
                    self.send_message_to_agent(prompt.to_string(), chat_view, chat_state, rt_handle);
                }
                None => {
                    chat_state.add_system_message(
                        "Init prompt not found. Please create prompts/init.md in the CLI crate.".to_string(),
                    );
                }
            },
            "/skills" => {
                self.show_skill_selector(chat_view, chat_state, rt_handle);
            }
            "/reload-skills" => {
                self.reload_skills_from_disk(chat_view, chat_state, rt_handle);
            }
            "/subagents" => {
                self.show_subagent_selector(chat_view, chat_state, rt_handle);
            }
            "/history" => {
                chat_state.add_system_message(format!(
                    "Current session statistics:\n\
                             • Messages: {}\n\
                             • Tool calls: {}\n\
                             • Tokens: {}",
                    chat_state.metadata.message_count, chat_state.metadata.tool_calls, chat_state.metadata.total_tokens
                ));
            }
            "/exit" => {
                if chat_state.is_processing {
                    tracing::info!("User requested cancellation via /exit");
                    let agent = self.agent.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.cancel_current_turn().await {
                                tracing::error!("Failed to cancel turn: {}", e);
                            }
                        })
                    });
                }
                return Ok(Some(ChatExitReason::Quit));
            }
            _ => {
                chat_state.add_system_message(format!(
                    "Unknown command: {}\nUse /help to see available commands",
                    parts[0]
                ));
            }
        }

        Ok(None)
    }

    pub(crate) fn show_usage_report(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        if chat_state.is_processing {
            chat_view.set_status(Some("Wait until the session is idle before using /usage.".to_string()));
            return;
        }

        let session_id = chat_state.core_session_id.clone();
        let workspace_path = chat_state
            .workspace
            .clone()
            .or_else(|| self.workspace.clone())
            .or_else(|| Some(self.agent.workspace_path_string()));
        let token_usage_service = self.token_usage_service.clone();
        let session_manager = self.agent.coordinator().session_manager();

        let report_result: Result<northhing_core::service::session_usage::SessionUsageReport> =
            tokio::task::block_in_place(|| {
                let session_id = session_id.clone();
                let workspace_path = workspace_path.clone();
                let token_usage_service = token_usage_service.clone();
                let session_manager = session_manager.clone();
                rt_handle.block_on(async move {
                    let workspace_path = workspace_path
                        .filter(|path| !path.trim().is_empty())
                        .ok_or_else(|| anyhow!("Workspace path is required for usage reports"))?;

                    let path_manager = northhing_core::infrastructure::try_get_path_manager_arc()
                        .map_err(|error| anyhow!(error.to_string()))?;
                    let persistence_manager =
                        PersistenceManager::new(path_manager).map_err(|error| anyhow!(error.to_string()))?;

                    let report = generate_session_usage_report(
                        &persistence_manager,
                        Some(token_usage_service.as_ref()),
                        SessionUsageReportRequest {
                            session_id: session_id.clone(),
                            workspace_path: Some(workspace_path),
                            remote_connection_id: None,
                            remote_ssh_host: None,
                            include_hidden_subagents: true,
                        },
                    )
                    .await
                    .map_err(|error| anyhow!(error.to_string()))?;

                    let markdown = render_usage_report_markdown(&report);
                    let generated_at = u64::try_from(report.generated_at).unwrap_or_default();
                    let usage_report = serde_json::to_value(&report)
                        .map_err(|error| anyhow!("Failed to serialize usage report: {}", error))?;
                    let metadata = serde_json::json!({
                        "localCommandKind": "usage_report",
                        "reportId": report.report_id.clone(),
                        "schemaVersion": report.schema_version,
                        "generatedAt": report.generated_at,
                        "modelVisible": false,
                        "usageReport": usage_report,
                        "usageReportStatus": "completed",
                    });

                    session_manager
                        .append_completed_local_command_turn(
                            &session_id,
                            markdown,
                            Some(format!("local-usage-{}", report.report_id)),
                            Some(generated_at),
                            Some(metadata),
                        )
                        .await
                        .map_err(|error| anyhow!(error.to_string()))?;

                    Ok(report)
                })
            });

        match report_result {
            Ok(report) => {
                let markdown = render_usage_report_markdown(&report);
                chat_state.add_assistant_message(markdown);
                chat_view.set_status(Some("Usage report added to conversation".to_string()));
            }
            Err(error) => {
                chat_state.add_system_message(format!("Failed to generate usage report: {}", error));
            }
        }
    }

    /// Send a message to the agent programmatically (used by slash commands like /init)
    pub(crate) fn send_message_to_agent(
        &self,
        message: String,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        if chat_state.is_processing {
            chat_state.add_system_message("Already processing, please wait.".to_string());
            return;
        }

        let display_name = agent_display_name(&self.agent_type);
        chat_view.set_status(Some(format!("{} is thinking...", display_name)));

        let agent = self.agent.clone();
        let agent_type = self.agent_type.clone();
        match tokio::task::block_in_place(|| rt_handle.block_on(agent.send_message(message, &agent_type))) {
            Ok(turn_id) => {
                tracing::info!("Started turn: {}", turn_id);
            }
            Err(e) => {
                tracing::error!("Failed to send message: {}", e);
                chat_view.set_status(Some(format!("Error: {}", e)));
            }
        }
    }
}
