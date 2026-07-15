//! Skill and agent snapshot diffing and rendering.
//!
//! Computes diffs between consecutive snapshots and renders skill/agent
//! listings for prompt injection.

use crate::agentic::agents::{PromptBuilder, ToolListingSections, UserContextPolicy};
use crate::agentic::tools::implementations::skills::{resolve_for_prompt, SkillInfo};
use crate::agentic::WorkspaceBinding;

use super::types::{AgentSnapshotEntry, SkillAgentDiff, SkillSnapshotEntry, TurnSkillAgentSnapshot};
use std::collections::{BTreeMap, HashSet};

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

/// Render a filtered skill listing based on keyword relevance to the prompt.
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

    let resolved_names: HashSet<&str> = resolved.iter().map(|r| r.skill.name.as_str()).collect();

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

impl SkillAgentDiff {
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
