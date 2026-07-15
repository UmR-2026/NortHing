use northhing_agent_tools::{PortableToolContextProvider, ToolContextFacts, ToolWorkspaceKind};

impl super::context_init::ToolUseContext {
    pub fn to_tool_context_facts(&self) -> ToolContextFacts {
        let workspace_kind = self.workspace.as_ref().map(|workspace| {
            if workspace.is_remote() {
                ToolWorkspaceKind::Remote
            } else {
                ToolWorkspaceKind::Local
            }
        });

        ToolContextFacts {
            tool_call_id: self.tool_call_id.clone(),
            agent_type: self.agent_type.clone(),
            session_id: self.session_id.clone(),
            dialog_turn_id: self.dialog_turn_id.clone(),
            workspace_kind,
            workspace_root: self
                .workspace
                .as_ref()
                .map(|workspace| workspace.session_identity.logical_workspace_path().to_string()),
            runtime_tool_restrictions: self.runtime_tool_restrictions.clone(),
        }
    }
}

impl PortableToolContextProvider for super::context_init::ToolUseContext {
    fn tool_context_facts(&self) -> ToolContextFacts {
        self.to_tool_context_facts()
    }
}
