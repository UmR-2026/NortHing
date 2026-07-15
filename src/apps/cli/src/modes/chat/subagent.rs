//! Chat mode subagent management — selector, list, config, enable/disable.
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;
use crate::ui::subagent_selector::{SubagentItem, SubagentSelectorAction};

use northhing_core::agentic::agents::{
    agent_registry, AgentInfo, SubAgentSource, SubagentListScope, SubagentQueryContext,
};

use super::ChatMode;

impl ChatMode {
    /// Show subagent list/configuration menu.
    pub(crate) fn show_subagent_selector(
        &self,
        chat_view: &mut ChatView,
        _chat_state: &mut ChatState,
        _rt_handle: &tokio::runtime::Handle,
    ) {
        chat_view.show_subagent_menu();
    }

    pub(crate) fn show_available_subagent_list(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let registry = agent_registry();
        let subagents = tokio::task::block_in_place(|| {
            let workspace = self.agent.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            rt_handle.block_on(registry.get_subagents_for_query(&SubagentQueryContext {
                parent_agent_type: Some(&agent_type),
                workspace_root: Some(workspace.as_path()),
                list_scope: SubagentListScope::TaskVisible,
                include_disabled: false,
            }))
        });

        if subagents.is_empty() {
            chat_state.add_system_message(format!(
                "No enabled subagents found for agent mode '{}'.",
                self.agent_type
            ));
            return;
        }

        let subagent_items: Vec<SubagentItem> = subagents.into_iter().map(Self::subagent_item_from_info).collect();

        if subagent_items.is_empty() {
            chat_state.add_system_message("No subagents found.".to_string());
            return;
        }

        chat_view.show_subagent_list(subagent_items);
    }

    pub(crate) fn show_subagent_config_selector(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let registry = agent_registry();
        let subagents = tokio::task::block_in_place(|| {
            let workspace = self.agent.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            rt_handle.block_on(registry.get_subagents_for_query(&SubagentQueryContext {
                parent_agent_type: Some(&agent_type),
                workspace_root: Some(workspace.as_path()),
                list_scope: SubagentListScope::RegistryManagement,
                include_disabled: true,
            }))
        });

        let subagent_items: Vec<SubagentItem> = subagents.into_iter().map(Self::subagent_item_from_info).collect();

        if subagent_items.is_empty() {
            chat_state.add_system_message("No subagents found.".to_string());
            return;
        }

        chat_view.show_subagent_config(subagent_items);
    }

    pub(crate) fn handle_subagent_selector_action(
        &self,
        action: SubagentSelectorAction,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        match action {
            SubagentSelectorAction::ListSubagents => {
                self.show_available_subagent_list(chat_view, chat_state, rt_handle);
            }
            SubagentSelectorAction::ConfigureSubagents => {
                self.show_subagent_config_selector(chat_view, chat_state, rt_handle);
            }
            SubagentSelectorAction::Launch(selected) => {
                chat_view.hide_subagent_selector();
                self.apply_subagent_selection(&selected, chat_view);
            }
            SubagentSelectorAction::Toggle(selected) => {
                self.set_subagent_enabled(&selected, !selected.enabled, chat_state, rt_handle);
                self.show_subagent_config_selector(chat_view, chat_state, rt_handle);
            }
        }
    }

    /// Apply subagent selection: fill input box with launch command
    pub(crate) fn apply_subagent_selection(&self, selected: &SubagentItem, chat_view: &mut ChatView) {
        chat_view.set_input(&format!("Launch subagent {} to finish task: ", selected.name));
    }

    pub(crate) fn set_subagent_enabled(
        &self,
        selected: &SubagentItem,
        enabled: bool,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let registry = agent_registry();
        let workspace = self.agent.workspace_path_buf();
        let mode_id = self.agent_type.clone();
        let subagent = selected.clone();

        let result: Result<(), String> = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                registry
                    .update_subagent_override(&mode_id, &subagent.id, enabled, Some(workspace.as_path()))
                    .await
                    .map_err(|error| error.to_string())
            })
        });

        match result {
            Ok(()) => chat_state.add_system_message(format!(
                "Subagent '{}' {} for mode '{}'.",
                selected.name,
                if enabled { "enabled" } else { "disabled" },
                self.agent_type
            )),
            Err(error) => {
                chat_state.add_system_message(format!("Failed to update subagent '{}': {}", selected.name, error))
            }
        }
    }

    pub(crate) fn subagent_item_from_info(info: AgentInfo) -> SubagentItem {
        let source = match info.subagent_source {
            Some(SubAgentSource::Builtin) => "builtin",
            Some(SubAgentSource::Project) => "project",
            Some(SubAgentSource::User) => "user",
            None => "builtin",
        }
        .to_string();

        SubagentItem {
            key: info.key,
            id: info.id,
            name: info.name,
            description: info.description,
            source,
            enabled: info.effective_enabled,
            default_enabled: info.default_enabled,
        }
    }
}
