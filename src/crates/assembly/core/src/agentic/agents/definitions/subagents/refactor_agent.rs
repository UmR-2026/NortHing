use crate::agentic::agents::{Agent, UserContextPolicy};
use async_trait::async_trait;

pub struct RefactorAgent {
    default_tools: Vec<String>,
}

impl Default for RefactorAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl RefactorAgent {
    pub fn new() -> Self {
        Self {
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Delete".to_string(),
                "ExecCommand".to_string(),
            ],
        }
    }
}

#[async_trait]
impl Agent for RefactorAgent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        "Refactor"
    }

    fn name(&self) -> &str {
        "Refactor"
    }

    fn description(&self) -> &str {
        r#"Specialized subagent for refactoring code. Performs small, behavior-preserving restructuring steps and validates with tests."#
    }

    fn prompt_template_name(&self, _model_name: Option<&str>) -> &str {
        "refactor_agent"
    }

    fn default_tools(&self) -> Vec<String> {
        self.default_tools.clone()
    }

    fn user_context_policy(&self) -> UserContextPolicy {
        UserContextPolicy::empty()
            .with_workspace_context()
            .with_workspace_instructions()
            .with_project_layout()
    }

    fn is_readonly(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::RefactorAgent;
    use crate::agentic::agents::Agent;

    #[test]
    fn uses_expected_default_tool_order() {
        let agent = RefactorAgent::new();
        assert_eq!(
            agent.default_tools(),
            vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Delete".to_string(),
                "ExecCommand".to_string(),
            ]
        );
    }

    #[test]
    fn always_uses_default_prompt_template() {
        let agent = RefactorAgent::new();
        assert_eq!(agent.prompt_template_name(Some("gpt-5.1")), "refactor_agent");
        assert_eq!(agent.prompt_template_name(Some("claude-sonnet-4")), "refactor_agent");
        assert_eq!(agent.prompt_template_name(None), "refactor_agent");
    }
}
