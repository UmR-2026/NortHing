//! Skill and agent snapshot assembly.
//!
//! Entry point for the `skill_agent_snapshot` module — owns the feature flag,
//! public re-exports, and the two cross-module unit tests.
//!
//! Implementation is split into sibling sub-modules:
//! - `types`        — DTOs and diff data model
//! - `resolution`   — async resolution + load pipeline
//! - `diff_render`  — diff computation and prompt-body rendering

mod diff_render;
mod resolution;
mod types;

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

// Re-export the public API surface so `agentic::*` and `crate::agentic::skill_agent_snapshot::*`
// continue to resolve the same 13 items.
pub use diff_render::{
    build_embedded_user_context_reminder, build_skill_agent_tool_listing_sections_from_snapshot,
    diff_skill_agent_snapshot, render_full_agent_listing_body, render_full_skill_listing_body,
    render_resolved_skill_listing_body,
};
pub use resolution::{resolve_skill_agent_snapshot, SkillAgentSnapshotResolution};
pub use types::{AgentSnapshotEntry, SkillAgentDiff, SkillSnapshotEntry, TurnSkillAgentSnapshot};

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
