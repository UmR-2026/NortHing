//! Task tool — agent registry + description rendering + call_impl completion (Round 12 split)
//!
//! Owns 6 agent-related fns + 2 tests + `PromptOrderTestAgent` test helper +
//! `build_completion_result` extracted from `call_impl` god method (R7 pattern).
//!
//! Spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0).

use super::task_tool_input::CallInputs;
use super::TaskTool;
use crate::agentic::agents::{agent_registry, AgentInfo, SubagentListScope, SubagentQueryContext};
use crate::agentic::deep_review::task_adapter as deep_review_task_adapter;
use crate::agentic::deep_review_policy::DeepReviewExecutionPolicy;
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};

/// Format a list of agent descriptions for the prompt template.
pub(super) fn format_agent_descriptions(agents: &[AgentInfo]) -> String {
    if agents.is_empty() {
        return String::new();
    }
    let mut out = String::from("<available_agents>\n");
    for agent in agents {
        out.push_str(&format!(
            "<agent type=\"{}\">\n<description>\n{}\n</description>\n<tools>{}</tools>\n</agent>\n",
            agent.id,
            agent.description,
            agent.default_tools.join(", ")
        ));
    }
    out.push_str("</available_agents>");
    out
}

/// Build the `<available_agents>` context section that gets injected into the
/// Task tool's prompt.
pub(super) async fn build_available_agents_context_section(context: Option<&ToolUseContext>) -> Option<String> {
    let agents = get_enabled_agents(context).await;
    let agent_descriptions = format_agent_descriptions(&agents);
    if agent_descriptions.trim().is_empty() {
        None
    } else {
        Some(agent_descriptions)
    }
}

pub(super) async fn get_enabled_agents(context: Option<&ToolUseContext>) -> Vec<AgentInfo> {
    let registry = agent_registry();
    let workspace_root = context.and_then(|ctx| ctx.workspace_root());
    if let Some(workspace_root) = workspace_root {
        registry.load_custom_subagents(workspace_root).await;
    }
    registry
        .get_subagents_for_query(&SubagentQueryContext {
            parent_agent_type: context.and_then(|ctx| ctx.agent_type.as_deref()),
            workspace_root,
            list_scope: SubagentListScope::TaskVisible,
            include_disabled: false,
        })
        .await
}

/// Subagent type ids available in the current context.
pub(super) async fn get_agents_types_impl(_self: &TaskTool, context: Option<&ToolUseContext>) -> Vec<String> {
    get_enabled_agents(context)
        .await
        .into_iter()
        .map(|agent| agent.id)
        .collect()
}

/// Completion result builder (Phase 6 of call_impl). Builds the final
/// `ToolResult` from a successful subagent execution.
pub(super) fn build_completion_result(
    result_text: &str,
    is_partial_timeout: bool,
    reason: Option<&str>,
    ledger_event_id: Option<&str>,
    inputs: &CallInputs,
    deep_review_effective_policy: Option<&DeepReviewExecutionPolicy>,
    deep_review_subagent_id: &str,
    deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
    delegate_target_label: &str,
    duration_ms: u128,
    should_emit_retry_guidance: bool,
    is_retry: bool,
) -> ToolResult {
    // Build retry hint for deep review reviewer timeouts.
    let retry_hint = if should_emit_retry_guidance {
        let retries_used = crate::agentic::deep_review_policy::deep_review_retries_used(
            &inputs.dialog_turn_id,
            deep_review_subagent_id,
        );
        let max_retries = deep_review_task_adapter::deep_review_retry_guidance_max_retries(
            deep_review_effective_policy,
            &inputs.dialog_turn_id,
        );
        deep_review_task_adapter::deep_review_retry_guidance(retries_used, max_retries)
    } else {
        String::new()
    };

    let (data, result_for_assistant) = deep_review_task_adapter::deep_review_task_completion_result(
        delegate_target_label,
        result_text,
        inputs.context_mode.as_str(),
        duration_ms,
        is_partial_timeout,
        reason,
        ledger_event_id,
        &retry_hint,
    );

    ToolResult::Result {
        data,
        result_for_assistant: Some(result_for_assistant),
        image_attachments: None,
    }
}

#[cfg(test)]
mod tests {
    use super::build_available_agents_context_section;
    use crate::agentic::agents::{
        agent_registry, Agent, AgentCategory, CustomSubagentConfig, SubAgentSource, UserContextPolicy,
    };
    use crate::agentic::tools::framework::ToolUseContext;
    use crate::agentic::tools::ToolRuntimeRestrictions;
    use async_trait::async_trait;
    use northhing_runtime_ports::SubagentContextMode;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn find_agent_block_index(description: &str, agent_id: &str) -> usize {
        description
            .find(&format!("<agent type=\"{}\">", agent_id))
            .unwrap_or_else(|| panic!("expected agent block for {}", agent_id))
    }

    struct PromptOrderTestAgent {
        id: String,
    }

    #[async_trait]
    impl Agent for PromptOrderTestAgent {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.id
        }

        fn description(&self) -> &str {
            "Prompt ordering test agent"
        }

        fn prompt_template_name(&self, _model_name: Option<&str>) -> &str {
            "test_prompt_order_agent"
        }

        fn user_context_policy(&self) -> UserContextPolicy {
            UserContextPolicy::empty()
        }

        fn default_tools(&self) -> Vec<String> {
            vec!["Read".to_string()]
        }
    }

    fn register_prompt_order_test_subagent(
        id: &str,
        source: SubAgentSource,
        custom_config: Option<CustomSubagentConfig>,
    ) {
        agent_registry().register_agent(
            Arc::new(PromptOrderTestAgent { id: id.to_string() }),
            AgentCategory::SubAgent,
            Some(source),
            custom_config,
        );
    }

    fn find_agent_block_index_local(description: &str, agent_id: &str) -> usize {
        description
            .find(&format!("<agent type=\"{}\">", agent_id))
            .unwrap_or_else(|| panic!("expected agent block for {}", agent_id))
    }

    #[tokio::test]
    async fn description_with_context_filters_restricted_subagents_by_parent_agent() {
        let agentic_context = ToolUseContext {
            tool_call_id: None,
            agent_type: Some("agentic".to_string()),
            session_id: None,
            dialog_turn_id: None,
            workspace: None,
            unlocked_collapsed_tools: Vec::new(),
            custom_data: HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
            actor_runtime: None,
        };

        let deep_review_context = ToolUseContext {
            agent_type: Some("DeepReview".to_string()),
            ..agentic_context.clone()
        };

        let agentic_description = build_available_agents_context_section(Some(&agentic_context))
            .await
            .expect("agentic available agents should render");
        assert!(agentic_description.contains("<agent type=\"Explore\">"));
        assert!(!agentic_description.contains("<agent type=\"ReviewSecurity\">"));
        assert!(!agentic_description.contains("<agent type=\"ResearchSpecialist\">"));

        let deep_review_description = build_available_agents_context_section(Some(&deep_review_context))
            .await
            .expect("deep review available agents should render");
        assert!(deep_review_description.contains("<agent type=\"ReviewSecurity\">"));
        assert!(!deep_review_description.contains("<agent type=\"ResearchSpecialist\">"));
    }

    #[tokio::test]
    async fn prompt_stability_description_with_context_renders_available_agents_in_stable_order() {
        let context = ToolUseContext {
            tool_call_id: None,
            agent_type: Some("agentic".to_string()),
            session_id: None,
            dialog_turn_id: None,
            workspace: None,
            unlocked_collapsed_tools: Vec::new(),
            custom_data: HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
            actor_runtime: None,
        };

        let builtin_a = "AAAPromptOrderBuiltin";
        let builtin_z = "ZZZPromptOrderBuiltin";
        let user_a = "AAAPromptOrderUser";
        let user_z = "ZZZPromptOrderUser";
        register_prompt_order_test_subagent(builtin_z, SubAgentSource::Builtin, None);
        register_prompt_order_test_subagent(builtin_a, SubAgentSource::Builtin, None);
        register_prompt_order_test_subagent(
            user_z,
            SubAgentSource::User,
            Some(CustomSubagentConfig {
                model: "fast".to_string(),
            }),
        );
        register_prompt_order_test_subagent(
            user_a,
            SubAgentSource::User,
            Some(CustomSubagentConfig {
                model: "fast".to_string(),
            }),
        );

        let description = build_available_agents_context_section(Some(&context))
            .await
            .expect("available agents should render");

        let builtin_a_index = find_agent_block_index(&description, builtin_a);
        let builtin_z_index = find_agent_block_index(&description, builtin_z);
        let user_a_index = find_agent_block_index(&description, user_a);
        let user_z_index = find_agent_block_index(&description, user_z);

        assert!(
            builtin_a_index < builtin_z_index,
            "builtin subagents should be sorted alphabetically"
        );
        assert!(
            builtin_z_index < user_a_index,
            "builtin subagents should render before user subagents"
        );
        assert!(
            user_a_index < user_z_index,
            "user subagents should be sorted alphabetically"
        );
    }

    // Silences unused-import warning when `SubagentContextMode` exists
    // for parity with original (original also has redundant locals).
    #[allow(dead_code)]
    fn _unused_marker(_mode: SubagentContextMode) {}
}
