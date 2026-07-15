// REFERENCE — copied from src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs
// Last synced: 2813b36 (v3-restructure)
// Mirror only — NOT compiled. Original file lives in src/.
// If you change the source, re-run: node scripts/copy_reference.js

use crate::agentic::agents::{
    agent_registry, PromptBuilder, SubagentListScope, SubagentQueryContext, ToolListingSections, UserContextPolicy,
};
use crate::agentic::tools::implementations::skills::{resolve_for_prompt, skill_registry, SkillInfo};
use crate::agentic::tools::manifest_resolver::{resolve_tool_manifest, ResolvedToolManifest};
use crate::agentic::tools::product_runtime::GetToolSpecTool;
use crate::agentic::tools::tool_context_runtime;
use crate::agentic::workspace::WorkspaceServices;
use crate::agentic::WorkspaceBinding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A4: Skill system v2 flag.
///
/// When `true`, the skill listing injected into the prompt is filtered by
/// [`resolve_for_prompt`] — only the top-5 most relevant skills (by keyword
/// overlap with the latest user message) are included, cutting the per-turn
/// skill listing from ~12-15K tokens to ~2-5K tokens.
///
/// When `false`, falls back to the v3 behavior of listing all resolved skills
/// (`render_full_skill_listing_body`).
///
/// Rollback: set to `false` to restore the full listing.
pub const USE_SKILL_REGISTRY: bool = true;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSnapshotEntry {
    pub name: String,
    pub description: String,
    pub location: String,
}

impl SkillSnapshotEntry {
    fn from_skill_info(skill: SkillInfo) -> Self {
        Self {
            name: skill.name,
            description: skill.description,
            location: skill.path,
        }
    }

    fn to_xml_desc(&self) -> String {
        format!(
            r#"<skill>
<name>
{}
</name>
<description>
{}
</description>
<location>
{}
</location>
</skill>"#,
            self.name, self.description, self.location
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSnapshotEntry {
    pub id: String,
    pub description: String,
    pub default_tools: Vec<String>,
}

impl AgentSnapshotEntry {
    fn to_xml_desc(&self) -> String {
        format!(
            "<agent type=\"{}\">\n<description>\n{}\n</description>\n<tools>{}</tools>\n</agent>",
            self.id,
            self.description,
            self.default_tools.join(", ")
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSkillAgentSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillSnapshotEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<AgentSnapshotEntry>,
}

impl TurnSkillAgentSnapshot {
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty() && self.subagents.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillAgentDiff {
    pub added_skills: Vec<SkillSnapshotEntry>,
    pub changed_skills: Vec<SkillSnapshotEntry>,
    pub removed_skills: Vec<String>,
    pub added_subagents: Vec<AgentSnapshotEntry>,
    pub changed_subagents: Vec<AgentSnapshotEntry>,
    pub removed_subagents: Vec<String>,
}

impl SkillAgentDiff {
    pub fn is_empty(&self) -> bool {
        self.added_skills.is_empty()
            && self.changed_skills.is_empty()
            && self.removed_skills.is_empty()
            && self.added_subagents.is_empty()
            && self.changed_subagents.is_empty()
            && self.removed_subagents.is_empty()
    }

    pub fn render_skill_listing_update(&self) -> Option<String> {
        if self.added_skills.is_empty() && self.changed_skills.is_empty() && self.removed_skills.is_empty() {
            return None;
        }

        let mut sections = Vec::new();
        if !self.added_skills.is_empty() {
            sections.push(render_titled_skill_entries("Added Skills", &self.added_skills));
        }
        if !self.changed_skills.is_empty() {
            sections.push(render_titled_skill_entries("Changed Skills", &self.changed_skills));
        }
        if !self.removed_skills.is_empty() {
            sections.push(render_removed_name_entries("Removed Skills", &self.removed_skills));
        }

        Some(format!("# Skill Listing Update\n\n{}", sections.join("\n\n")))
    }

    pub fn render_agent_listing_update(&self) -> Option<String> {
        if self.added_subagents.is_empty() && self.changed_subagents.is_empty() && self.removed_subagents.is_empty() {
            return None;
        }

        let mut sections = Vec::new();
        if !self.added_subagents.is_empty() {
            sections.push(render_titled_subagent_entries("Added Agents", &self.added_subagents));
        }
        if !self.changed_subagents.is_empty() {
            sections.push(render_titled_subagent_entries(
                "Changed Agents",
                &self.changed_subagents,
            ));
        }
        if !self.removed_subagents.is_empty() {
            sections.push(render_removed_name_entries("Removed Agents", &self.removed_subagents));
        }

        Some(format!("# Agent Listing Update\n\n{}", sections.join("\n\n")))
    }
}

#[derive(Debug, Clone)]
pub struct SkillAgentSnapshotResolution {
    pub snapshot: TurnSkillAgentSnapshot,
    pub tool_listing_sections: ToolListingSections,
}

pub async fn resolve_skill_agent_snapshot(
    agent_type: &str,
    workspace: Option<&WorkspaceBinding>,
    workspace_services: Option<&WorkspaceServices>,
    enable_tools: bool,
    context_vars: &std::collections::HashMap<String, String>,
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
                if USE_SKILL_REGISTRY {
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

pub fn diff_skill_agent_snapshot(
    previous: &TurnSkillAgentSnapshot,
    current: &TurnSkillAgentSnapshot,
) -> SkillAgentDiff {
    let previous_skills = previous
        .skills
        .iter()
        .cloned()
        .map(|entry| (entry.name.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let current_skills = current
        .skills
        .iter()
        .cloned()
        .map(|entry| (entry.name.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let previous_subagents = previous
        .subagents
        .iter()
        .cloned()
        .map(|entry| (entry.id.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let current_subagents = current
        .subagents
        .iter()
        .cloned()
        .map(|entry| (entry.id.clone(), entry))
        .collect::<BTreeMap<_, _>>();

    let mut diff = SkillAgentDiff::default();

    for (name, entry) in &current_skills {
        match previous_skills.get(name) {
            None => diff.added_skills.push(entry.clone()),
            Some(previous) if previous != entry => diff.changed_skills.push(entry.clone()),
            Some(_) => {}
        }
    }
    for name in previous_skills.keys() {
        if !current_skills.contains_key(name) {
            diff.removed_skills.push(name.clone());
        }
    }

    for (id, entry) in &current_subagents {
        match previous_subagents.get(id) {
            None => diff.added_subagents.push(entry.clone()),
            Some(previous) if !agent_snapshot_entries_match_for_diff(previous, entry) => {
                diff.changed_subagents.push(entry.clone())
            }
            Some(_) => {}
        }
    }
    for id in previous_subagents.keys() {
        if !current_subagents.contains_key(id) {
            diff.removed_subagents.push(id.clone());
        }
    }

    diff
}

fn agent_snapshot_entries_match_for_diff(previous: &AgentSnapshotEntry, current: &AgentSnapshotEntry) -> bool {
    previous.id == current.id
        && previous.description == current.description
        && sorted_tool_names(&previous.default_tools) == sorted_tool_names(&current.default_tools)
}

fn sorted_tool_names(tool_names: &[String]) -> Vec<&str> {
    let mut normalized = tool_names.iter().map(String::as_str).collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized
}

pub async fn build_embedded_user_context_reminder(
    workspace: Option<&WorkspaceBinding>,
    workspace_id: Option<&str>,
    session_id: &str,
    user_context_policy: &UserContextPolicy,
) -> Option<String> {
    let workspace = workspace?;
    let context = crate::agentic::agents::build_prompt_context_for_workspace(
        workspace,
        workspace_id,
        session_id,
        None,
        None,
        ToolListingSections::default(),
        Default::default(),
    )
    .await?;
    PromptBuilder::new(context)
        .build_user_context_reminder(user_context_policy)
        .await
}

pub fn render_full_skill_listing_body(skills: &[SkillSnapshotEntry]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    format!(
        "<available_skills>\n{}\n</available_skills>",
        skills
            .iter()
            .map(SkillSnapshotEntry::to_xml_desc)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// A4: Render a filtered skill listing based on keyword relevance to the prompt.
///
/// Uses [`resolve_for_prompt`] to pick the top-5 most relevant skills. Falls
/// back to the full listing if no prompt is provided or no skills match (so
/// the agent always sees *some* skill list, never an empty one).
pub fn render_resolved_skill_listing_body(skills: &[SkillSnapshotEntry], user_prompt: Option<&str>) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let prompt = match user_prompt {
        Some(p) if !p.trim().is_empty() => p,
        _ => return render_full_skill_listing_body(skills),
    };

    // Convert SkillSnapshotEntry back to SkillInfo for the resolver.
    let skill_infos: Vec<SkillInfo> = skills
        .iter()
        .map(|entry| SkillInfo {
            key: entry.name.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            path: entry.location.clone(),
            level: crate::agentic::tools::implementations::skills::SkillLocation::User,
            source_slot: String::new(),
            dir_name: entry.name.clone(),
            is_builtin: false,
            group_key: None,
            is_shadowed: false,
            shadowed_by_key: None,
        })
        .collect();

    let resolved = resolve_for_prompt(prompt, &skill_infos);

    // If nothing matched, fall back to the full listing rather than showing
    // an empty list (the agent should still know skills exist).
    if resolved.is_empty() {
        return render_full_skill_listing_body(skills);
    }

    let resolved_names: std::collections::HashSet<&str> = resolved.iter().map(|r| r.skill.name.as_str()).collect();

    let filtered: Vec<&SkillSnapshotEntry> = skills
        .iter()
        .filter(|s| resolved_names.contains(s.name.as_str()))
        .collect();

    format!(
        "<available_skills>\n{}\n</available_skills>",
        filtered.iter().map(|s| s.to_xml_desc()).collect::<Vec<_>>().join("\n")
    )
}

pub fn render_full_agent_listing_body(subagents: &[AgentSnapshotEntry]) -> String {
    if subagents.is_empty() {
        return String::new();
    }
    format!(
        "<available_agents>\n{}\n</available_agents>",
        subagents
            .iter()
            .map(AgentSnapshotEntry::to_xml_desc)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub fn build_skill_agent_tool_listing_sections_from_snapshot(snapshot: &TurnSkillAgentSnapshot) -> ToolListingSections {
    ToolListingSections {
        skill_listing: (!snapshot.skills.is_empty())
            .then(|| render_full_skill_listing_body(&snapshot.skills))
            .filter(|body| !body.is_empty()),
        agent_listing: (!snapshot.subagents.is_empty())
            .then(|| render_full_agent_listing_body(&snapshot.subagents))
            .filter(|body| !body.is_empty()),
        collapsed_tool_listing: None,
    }
}

fn render_titled_skill_entries(title: &str, entries: &[SkillSnapshotEntry]) -> String {
    format!(
        "## {}\n\n{}",
        title,
        entries
            .iter()
            .map(SkillSnapshotEntry::to_xml_desc)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn render_titled_subagent_entries(title: &str, entries: &[AgentSnapshotEntry]) -> String {
    format!(
        "## {}\n\n{}",
        title,
        entries
            .iter()
            .map(AgentSnapshotEntry::to_xml_desc)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn render_removed_name_entries(title: &str, names: &[String]) -> String {
    let entries = names
        .iter()
        .map(|name| format!("- {}", name))
        .collect::<Vec<_>>()
        .join("\n");
    format!("## {}\n\n{}", title, entries)
}

#[cfg(test)]
mod tests {
    use super::{diff_skill_agent_snapshot, AgentSnapshotEntry, SkillSnapshotEntry, TurnSkillAgentSnapshot};

    #[test]
    fn skill_agent_diff_renders_changed_added_and_removed_entries() {
        let previous = TurnSkillAgentSnapshot {
            skills: vec![
                SkillSnapshotEntry {
                    name: "skill-a".to_string(),
                    description: "desc-a".to_string(),
                    location: "/a".to_string(),
                },
                SkillSnapshotEntry {
                    name: "skill-b".to_string(),
                    description: "desc-b".to_string(),
                    location: "/b".to_string(),
                },
            ],
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Read".to_string()],
            }],
        };
        let current = TurnSkillAgentSnapshot {
            skills: vec![
                SkillSnapshotEntry {
                    name: "skill-a".to_string(),
                    description: "desc-a2".to_string(),
                    location: "/a".to_string(),
                },
                SkillSnapshotEntry {
                    name: "skill-c".to_string(),
                    description: "desc-c".to_string(),
                    location: "/c".to_string(),
                },
            ],
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Read".to_string(), "Grep".to_string()],
            }],
        };

        let diff = diff_skill_agent_snapshot(&previous, &current);
        let skill_update = diff.render_skill_listing_update().expect("skill update should render");
        let agent_update = diff.render_agent_listing_update().expect("agent update should render");

        assert!(skill_update.contains("## Changed Skills"));
        assert!(skill_update.contains("## Added Skills"));
        assert!(skill_update.contains("## Removed Skills"));
        assert!(skill_update.contains("skill-a"));
        assert!(skill_update.contains("skill-c"));
        assert!(skill_update.contains("- skill-b"));
        assert!(agent_update.contains("## Changed Agents"));
        assert!(agent_update.contains("Grep"));
    }

    #[test]
    fn skill_agent_diff_ignores_default_tool_reordering_for_agents() {
        let previous = TurnSkillAgentSnapshot {
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Read".to_string(), "Grep".to_string()],
            }],
            ..Default::default()
        };
        let current = TurnSkillAgentSnapshot {
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Grep".to_string(), "Read".to_string()],
            }],
            ..Default::default()
        };

        let diff = diff_skill_agent_snapshot(&previous, &current);

        assert!(diff.changed_subagents.is_empty());
        assert!(diff.is_empty());
    }
}
