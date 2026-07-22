use crate::agentic::agents::{Agent, UserContextPolicy};
use async_trait::async_trait;

pub struct TestAgent {
    default_tools: Vec<String>,
}

impl Default for TestAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAgent {
    pub fn new() -> Self {
        Self {
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "ExecCommand".to_string(),
            ],
        }
    }
}

#[async_trait]
impl Agent for TestAgent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        "Test"
    }

    fn name(&self) -> &str {
        "Test"
    }

    fn description(&self) -> &str {
        r#"Specialized subagent for writing and running tests. Searches existing test patterns first, then writes or updates tests accordingly."#
    }

    fn prompt_template_name(&self, _model_name: Option<&str>) -> &str {
        "test_agent"
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
    use super::TestAgent;
    use crate::agentic::agents::Agent;

    #[test]
    fn uses_expected_default_tool_order() {
        let agent = TestAgent::new();
        assert_eq!(
            agent.default_tools(),
            vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "ExecCommand".to_string(),
            ]
        );
    }

    #[test]
    fn always_uses_default_prompt_template() {
        let agent = TestAgent::new();
        assert_eq!(agent.prompt_template_name(Some("gpt-5.1")), "test_agent");
        assert_eq!(agent.prompt_template_name(Some("claude-sonnet-4")), "test_agent");
        assert_eq!(agent.prompt_template_name(None), "test_agent");
    }
}
