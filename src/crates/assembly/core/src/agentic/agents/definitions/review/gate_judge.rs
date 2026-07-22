use crate::agentic::agents::AgentToolPolicyOverrides;
use crate::agentic::deep_review_policy::GATE_JUDGE_AGENT_TYPE;
use crate::agentic::tools::framework::ToolExposure;
use crate::define_readonly_subagent_with_overrides;

fn gate_judge_tool_exposure_overrides() -> AgentToolPolicyOverrides {
    let mut overrides = AgentToolPolicyOverrides::default();
    overrides.insert("GetFileDiff".to_string(), ToolExposure::Expanded);
    overrides.insert("Git".to_string(), ToolExposure::Expanded);
    overrides
}

define_readonly_subagent_with_overrides!(
    GateJudgeAgent,
    GATE_JUDGE_AGENT_TYPE,
    "Gate Judge",
    r#"Independent redline arbiter that validates skill promotion candidates against four frozen invariant rules (I-NEG-1 through I-NEG-4). You have no authority to define or weight rules — you adjudicate based solely on the redline table provided by the user. For each rule, assess whether the evidence demonstrates pass or violation. Your verdict must cite specific evidence IDs from the evidence pack."#,
    "gate_judge_agent",
    &["Read", "Grep", "Glob", "LS", "GetFileDiff", "Git"],
    gate_judge_tool_exposure_overrides()
);

#[cfg(test)]
mod tests {
    use super::GateJudgeAgent;
    use crate::agentic::agents::{Agent, UserContextPolicy};

    #[test]
    fn gate_judge_is_readonly() {
        let agent = GateJudgeAgent::new();
        assert!(agent.is_readonly());
        assert!(agent.default_tools().contains(&"GetFileDiff".to_string()));
        assert_eq!(
            agent.user_context_policy(),
            UserContextPolicy::empty().with_workspace_instructions()
        );
    }
}
