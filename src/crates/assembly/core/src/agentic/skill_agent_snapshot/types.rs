//! Skill and agent snapshot data types.
//!
//! Defines the DTOs for per-skill entries, per-agent entries, turn-level
//! snapshots, and the diff between consecutive snapshots.

use crate::agentic::tools::implementations::skills::SkillInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSnapshotEntry {
    pub name: String,
    pub description: String,
    pub location: String,
}

impl SkillSnapshotEntry {
    pub(crate) fn from_skill_info(skill: SkillInfo) -> Self {
        Self {
            name: skill.name,
            description: skill.description,
            location: skill.path,
        }
    }

    pub(crate) fn to_xml_desc(&self) -> String {
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
    pub(crate) fn to_xml_desc(&self) -> String {
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
}
