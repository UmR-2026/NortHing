//! Chat mode agent selector — cycle, show, apply.
use crate::chat_state::ChatState;
use crate::ui::agent_selector::AgentItem;
use crate::ui::chat::ChatView;

use northhing_core::agentic::agents::{agent_registry, AgentInfo};

use super::ChatMode;

impl ChatMode {
    pub(crate) fn get_mode_agents(&self, rt_handle: &tokio::runtime::Handle) -> Vec<AgentInfo> {
        let registry = agent_registry();
        let modes = tokio::task::block_in_place(|| rt_handle.block_on(registry.get_modes_info()));
        modes
    }

    pub(crate) fn cycle_agent(
        &mut self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        self.switch_agent_by_offset(1, chat_view, chat_state, rt_handle);
    }

    pub(crate) fn cycle_agent_reverse(
        &mut self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        self.switch_agent_by_offset(-1, chat_view, chat_state, rt_handle);
    }

    pub(crate) fn switch_agent_by_offset(
        &mut self,
        offset: isize,
        _chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let modes = self.get_mode_agents(rt_handle);
        if modes.len() <= 1 {
            return;
        }

        let current_idx = modes.iter().position(|m| m.id == self.agent_type).unwrap_or(0);

        let len = modes.len() as isize;
        let next_idx = ((current_idx as isize + offset) % len + len) % len;
        let next = &modes[next_idx as usize];

        self.agent_type = next.id.clone();
        chat_state.agent_type = next.id.clone();
    }

    /// Show agent selector popup with all available agent modes
    pub(crate) fn show_agent_selector(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let modes = self.get_mode_agents(rt_handle);
        if modes.is_empty() {
            chat_state.add_system_message("No mode agents available".to_string());
            return;
        }

        let agent_items: Vec<AgentItem> = modes
            .into_iter()
            .map(|m| AgentItem {
                id: m.id,
                description: m.description,
            })
            .collect();

        chat_view.show_agent_selector(agent_items, Some(self.agent_type.clone()));
    }

    /// Apply agent selection: switch agent type
    pub(crate) fn apply_agent_selection(&mut self, selected: &AgentItem, chat_state: &mut ChatState) {
        if selected.id == self.agent_type {
            return;
        }
        self.agent_type = selected.id.clone();
        chat_state.agent_type = selected.id.clone();
        tracing::info!("Switched to agent: {}", selected.id);

        if selected.id == "HarmonyOSDev" {
            let deveco_home = std::env::var("DEVECO_HOME").ok();
            let missing = deveco_home.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
            if missing {
                chat_state.add_system_message(
                    "HarmonyOSDev tip: HmosCompilation requires DEVECO_HOME (DevEco Studio install path). If compilation fails, set DEVECO_HOME and restart the terminal."
                        .to_string(),
                );
            }
        }
    }
}
