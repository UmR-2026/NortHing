//! Chat mode session management — switch, create, selector, delete.
use anyhow::Result;

use crate::agent::Agent;
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;
use crate::ui::session_selector::SessionItem;

use super::ChatMode;

impl ChatMode {
    /// Switch to a different session: restore it from core, reload messages, update state
    pub(crate) fn switch_to_session(
        &mut self,
        new_session_id: &str,
        session_id: &mut String,
        chat_state: &mut ChatState,
        chat_view: &mut ChatView,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<()> {
        let agent = self.agent.clone();
        let sid = new_session_id.to_string();
        let agent_type = self.agent_type.clone();
        let workspace = self.workspace.clone();

        let (new_state, restored_agent_type) = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                // Restore session in core
                agent.restore_session(&sid).await?;

                // Get session info for agent_type and workspace
                let workspace_path = agent.workspace_path_buf();
                let sessions = agent
                    .coordinator()
                    .list_sessions(&workspace_path)
                    .await
                    .unwrap_or_default();
                let session_summary = sessions.iter().find(|s| s.session_id == sid);
                let restored_agent_type = session_summary
                    .map(|s| s.agent_type.clone())
                    .unwrap_or_else(|| agent_type.clone());
                let session_name = session_summary
                    .map(|s| s.session_name.clone())
                    .unwrap_or_else(|| "Restored Session".to_string());

                // Use the current workspace filtered by the session list; fall back to the
                // workspace supplied when this chat view was created.
                let effective_workspace = workspace
                    .clone()
                    .or_else(|| Some(workspace_path.to_string_lossy().to_string()));

                // Sync global workspace path from restored session
                if let Some(ref ws) = effective_workspace {
                    agent.set_workspace_path(Some(std::path::PathBuf::from(ws))).await;
                }

                // Load historical messages from core.
                let messages = agent.coordinator().get_messages(&sid).await.unwrap_or_default();

                let state = ChatState::from_core_messages(
                    sid.clone(),
                    session_name,
                    restored_agent_type.clone(),
                    effective_workspace,
                    &messages,
                );

                Ok::<_, anyhow::Error>((state, restored_agent_type))
            })
        })?;

        // Update session state
        *session_id = new_session_id.to_string();
        *chat_state = new_state;
        self.agent_type = restored_agent_type;
        self.workspace = chat_state.workspace.clone();

        // Reload model name
        self.load_current_model_name(chat_state, rt_handle);

        // Reset view state
        chat_view.scroll_to_bottom();
        chat_view.set_status(Some(format!("Switched to session: {}", new_session_id)));

        Ok(())
    }

    /// Create a new session: reset state and start fresh
    pub(crate) fn create_new_session(
        &mut self,
        session_id: &mut String,
        chat_state: &mut ChatState,
        chat_view: &mut ChatView,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<()> {
        let agent = self.agent.clone();
        let agent_type = self.agent_type.clone();
        let workspace = self.workspace.clone();

        let new_session_id = tokio::task::block_in_place(|| rt_handle.block_on(agent.create_new_session(&agent_type)))?;

        let new_state = ChatState::new(new_session_id.clone(), "CLI Session".to_string(), agent_type, workspace);

        *session_id = new_session_id;
        *chat_state = new_state;
        self.workspace = chat_state.workspace.clone();

        // Reload model name
        self.load_current_model_name(chat_state, rt_handle);

        // Reset view state
        chat_view.clear_screen();
        chat_view.scroll_to_bottom();
        chat_view.set_status(Some("New session created".to_string()));

        Ok(())
    }

    /// Show session selector popup with all available sessions
    pub(crate) fn show_session_selector(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let agent = self.agent.clone();
        let current_session_id = chat_state.core_session_id.clone();

        let sessions = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                agent
                    .coordinator()
                    .list_sessions(&agent.workspace_path_buf())
                    .await
                    .unwrap_or_default()
            })
        });

        if sessions.is_empty() {
            chat_state.add_system_message("No sessions found.".to_string());
            return;
        }

        let session_items: Vec<SessionItem> = sessions
            .into_iter()
            .map(|s| {
                let last_activity = {
                    let elapsed = s.last_activity_at.elapsed().unwrap_or_default();
                    if elapsed.as_secs() < 60 {
                        "just now".to_string()
                    } else if elapsed.as_secs() < 3600 {
                        format!("{}m ago", elapsed.as_secs() / 60)
                    } else if elapsed.as_secs() < 86400 {
                        format!("{}h ago", elapsed.as_secs() / 3600)
                    } else {
                        format!("{}d ago", elapsed.as_secs() / 86400)
                    }
                };
                SessionItem {
                    session_id: s.session_id,
                    session_name: s.session_name,
                    last_activity,
                    workspace: self.workspace.clone(),
                }
            })
            .collect();

        chat_view.show_session_selector(session_items, Some(current_session_id));
    }

    /// Handle session deletion from the session selector
    pub(crate) fn handle_session_delete(
        &self,
        item: &SessionItem,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        // Prevent deleting the currently active session
        if item.session_id == chat_state.core_session_id {
            chat_view.set_status(Some("Cannot delete the active session".to_string()));
            return;
        }

        let agent = self.agent.clone();
        let sid = item.session_id.clone();

        let result = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                let workspace_path = agent.workspace_path_buf();
                agent.coordinator().delete_session(&workspace_path, &sid).await
            })
        });

        match result {
            Ok(()) => {
                chat_view.session_selector_remove_item(&item.session_id);
                chat_view.set_status(Some(format!("Session deleted: {}", item.session_name)));
                tracing::info!("Deleted session: {}", item.session_id);
            }
            Err(e) => {
                chat_view.set_status(Some(format!("Failed to delete session: {}", e)));
                tracing::error!("Failed to delete session: {}", e);
            }
        }
    }
}
