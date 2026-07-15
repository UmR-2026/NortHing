//! Skill and agent snapshot resolution and loading pipeline.
//!
//! Resolves the current `TurnSkillAgentSnapshot` from the skill and agent
//! registries, then wraps it with `ToolListingSections` for prompt injection.

use crate::agentic::agents::{
    agent_registry, SubagentListScope, SubagentQueryContext, ToolListingSections, UserContextPolicy,
};
use crate::agentic::tools::implementations::skills::skill_registry;
use crate::agentic::tools::manifest_resolver::{resolve_tool_manifest, ResolvedToolManifest};
use crate::agentic::tools::product_runtime::GetToolSpecTool;
use crate::agentic::tools::tool_context_runtime;
use crate::agentic::workspace::WorkspaceServices;
use crate::agentic::WorkspaceBinding;
use std::collections::HashMap;

use super::diff_render::{
    render_full_agent_listing_body, render_full_skill_listing_body, render_resolved_skill_listing_body,
};
use super::types::{AgentSnapshotEntry, SkillSnapshotEntry, TurnSkillAgentSnapshot};

pub struct SkillAgentSnapshotResolution {
    pub snapshot: TurnSkillAgentSnapshot,
    pub tool_listing_sections: ToolListingSections,
}

pub async fn resolve_skill_agent_snapshot(
    agent_type: &str,
    workspace: Option<&WorkspaceBinding>,
    workspace_services: Option<&WorkspaceServices>,
    enable_tools: bool,
    context_vars: &HashMap<String, String>,
    user_prompt: Option<&str>,
) -> SkillAgentSnapshotResolution {
    if !enable_tools {
        return SkillAgentSnapshotResolution {
            snapshot: TurnSkillAgentSnapshot::default(),
            tool_listing_sections: ToolListingSections::default(),
        };
    }

    let agent_registry = agent_registry();
    if let Some(workspace) = workspace {
        if !workspace.is_remote() {
            agent_registry.load_custom_subagents(workspace.root_path()).await;
        }
    }

    let tool_policy = agent_registry
        .get_agent_tool_policy(agent_type, workspace.map(|binding| binding.root_path()))
        .await;

    let tool_description_context = tool_context_runtime::build_tool_description_context(
        agent_type,
        workspace,
        workspace_services,
        true,
        context_vars,
        None, // K.2.3 follow-up: skill manifest doesn't need actor_runtime
    );
    let manifest = resolve_tool_manifest(
        &tool_policy.allowed_tools,
        &tool_policy.exposure_overrides,
        &tool_description_context,
    )
    .await;

    let snapshot = build_skill_agent_snapshot(workspace, workspace_services, agent_type, &manifest).await;
    let tool_listing_sections = build_tool_listing_sections(&manifest, &snapshot, user_prompt);

    SkillAgentSnapshotResolution {
        snapshot,
        tool_listing_sections,
    }
}

async fn build_skill_agent_snapshot(
    workspace: Option<&WorkspaceBinding>,
    workspace_services: Option<&WorkspaceServices>,
    agent_type: &str,
    manifest: &ResolvedToolManifest,
) -> TurnSkillAgentSnapshot {
    let has_tool = |tool_name: &str| {
        manifest
            .tool_definitions
            .iter()
            .any(|definition| definition.name == tool_name)
    };

    let mut snapshot = TurnSkillAgentSnapshot::default();

    if has_tool("Skill") {
        snapshot.skills = load_skill_entries(workspace, workspace_services, Some(agent_type)).await;
    }

    if has_tool("Task") {
        snapshot.subagents = load_subagent_entries(workspace, Some(agent_type)).await;
    }

    snapshot
}

fn build_tool_listing_sections(
    manifest: &ResolvedToolManifest,
    snapshot: &TurnSkillAgentSnapshot,
    user_prompt: Option<&str>,
) -> ToolListingSections {
    let has_tool = |tool_name: &str| {
        manifest
            .tool_definitions
            .iter()
            .any(|definition| definition.name == tool_name)
    };

    ToolListingSections {
        skill_listing: has_tool("Skill")
            .then(|| {
                if super::USE_SKILL_REGISTRY {
                    render_resolved_skill_listing_body(&snapshot.skills, user_prompt)
                } else {
                    render_full_skill_listing_body(&snapshot.skills)
                }
            })
            .filter(|body| !body.is_empty()),
        agent_listing: has_tool("Task")
            .then(|| render_full_agent_listing_body(&snapshot.subagents))
            .filter(|body| !body.is_empty()),
        collapsed_tool_listing: if has_tool("GetToolSpec") {
            GetToolSpecTool::build_collapsed_tools_context_section(&manifest.collapsed_tool_summaries)
        } else {
            None
        },
    }
}

async fn load_skill_entries(
    workspace: Option<&WorkspaceBinding>,
    workspace_services: Option<&WorkspaceServices>,
    agent_type: Option<&str>,
) -> Vec<SkillSnapshotEntry> {
    let registry = skill_registry();
    let skills = match workspace {
        Some(workspace) if workspace.is_remote() => {
            if let Some(services) = workspace_services {
                registry
                    .get_resolved_skills_for_remote_workspace(
                        services.fs.as_ref(),
                        &workspace.root_path_string(),
                        agent_type,
                    )
                    .await
            } else {
                Vec::new()
            }
        }
        Some(workspace) => {
            registry
                .get_resolved_skills_for_workspace(Some(workspace.root_path()), agent_type)
                .await
        }
        None => registry.get_resolved_skills_for_workspace(None, agent_type).await,
    };

    skills.into_iter().map(SkillSnapshotEntry::from_skill_info).collect()
}

async fn load_subagent_entries(
    workspace: Option<&WorkspaceBinding>,
    agent_type: Option<&str>,
) -> Vec<AgentSnapshotEntry> {
    let registry = agent_registry();
    let workspace_root = workspace
        .filter(|workspace| !workspace.is_remote())
        .map(|workspace| workspace.root_path());
    let agents = registry
        .get_subagents_for_query(&SubagentQueryContext {
            parent_agent_type: agent_type,
            workspace_root,
            list_scope: SubagentListScope::TaskVisible,
            include_disabled: false,
        })
        .await;

    agents
        .into_iter()
        .map(|agent| AgentSnapshotEntry {
            id: agent.id,
            description: agent.description,
            default_tools: agent.default_tools,
        })
        .collect()
}
