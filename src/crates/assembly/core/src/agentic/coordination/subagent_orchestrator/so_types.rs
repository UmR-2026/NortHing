//! Sub-domain: pure helpers.
//! Spec step-3.7 — free functions extracted from subagent_orchestrator.rs (R50b refactor).

use super::super::coordinator::SubagentResult;
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::agentic::tools::{
    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
};
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::util::errors::NortHingError;
use northhing_runtime_ports::DelegationPolicy;

pub(crate) fn format_background_subagent_delivery_text(
    background_task_id: &str,
    agent_type: &str,
    outcome: Result<&SubagentResult, &NortHingError>,
) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed with partial timeout result:\n<partial_result status=\"partial_timeout\">\n{}\n</partial_result>",
                    agent_type, background_task_id, result.text
                )
            } else {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed successfully:\n<result>\n{}\n</result>",
                    agent_type, background_task_id, result.text
                )
            }
        }
        Err(error) => {
            format!(
                "Background subagent '{}' (background_task_id='{}') failed before producing a final result.\nError: {}",
                agent_type, background_task_id, error
            )
        }
    }
}

pub(crate) fn format_background_subagent_display_text(outcome: Result<&SubagentResult, &NortHingError>) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                "Background subagent completed with a partial timeout result.".to_string()
            } else {
                "Background subagent completed successfully.".to_string()
            }
        }
        Err(_) => "Background subagent failed before producing a final result.".to_string(),
    }
}

pub(crate) fn build_subagent_session_relationship(
    parent_info: Option<&SubagentParentInfo>,
    agent_type: &str,
) -> SessionRelationship {
    SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: parent_info.map(|info| info.session_id.clone()),
        parent_request_id: None,
        parent_dialog_turn_id: parent_info.map(|info| info.dialog_turn_id.clone()),
        parent_turn_index: None,
        parent_tool_call_id: parent_info.map(|info| info.tool_call_id.clone()),
        subagent_type: Some(agent_type.to_string()),
    }
}

pub(crate) fn fork_subagent_system_reminder() -> String {
    "<system_reminder>You are now running as a forked subagent. Messages before this reminder were inherited from the parent agent as context. Messages after this reminder are the request for you. Do not call the Task tool to launch another subagent. Use the tools available to complete the task directly.</system_reminder>".to_string()
}

pub(crate) fn runtime_tool_restrictions_for_delegation_policy(
    delegation_policy: DelegationPolicy,
) -> ToolRuntimeRestrictions {
    let mut restrictions = ToolRuntimeRestrictions::default();
    if !delegation_policy.allow_subagent_spawn {
        restrictions.denied_tool_names.insert("Task".to_string());
        restrictions.denied_tool_messages.insert(
            "Task".to_string(),
            "Recursive subagent delegation is blocked. Use direct tools instead.".to_string(),
        );
    }
    restrictions
}
