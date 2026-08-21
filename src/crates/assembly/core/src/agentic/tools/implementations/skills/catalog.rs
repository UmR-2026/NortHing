//! Built-in skill catalog derived dynamically from embedded skill metadata.
//!
//! Grouping metadata is parsed from embedded `SKILL.md` frontmatter rather than
//! maintained as a static hardcoded table.

use crate::agentic::tools::implementations::skills::builtin::BUILTIN_SKILLS_DIR;
use crate::util::front_matter_markdown::FrontMatterMarkdown;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinSkillGroup {
    Office,
    Meta,
    ComputerUse,
    Gstack,
}

impl BuiltinSkillGroup {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "office" => Some(Self::Office),
            "meta" => Some(Self::Meta),
            "computer-use" | "computer_use" => Some(Self::ComputerUse),
            "gstack" => Some(Self::Gstack),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Office => "office",
            Self::Meta => "meta",
            Self::ComputerUse => "computer-use",
            Self::Gstack => "gstack",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSkillSpec {
    pub dir_name: String,
    pub group: BuiltinSkillGroup,
}

static BUILTIN_SPECS: LazyLock<HashMap<String, BuiltinSkillSpec>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for dir in BUILTIN_SKILLS_DIR.dirs() {
        let Some(dir_name) = dir.path().file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let skill_md_path = dir.path().join("SKILL.md");
        if let Some(file) = dir.get_file(&skill_md_path) {
            if let Ok(content) = std::str::from_utf8(file.contents()) {
                if let Ok((meta, _)) = FrontMatterMarkdown::load_str(content) {
                    if let Some(group_str) = meta.get("group").and_then(|v| v.as_str()) {
                        if let Some(group) = BuiltinSkillGroup::parse(group_str) {
                            map.insert(
                                dir_name.to_string(),
                                BuiltinSkillSpec {
                                    dir_name: dir_name.to_string(),
                                    group,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    map
});

pub fn builtin_skill_spec(dir_name: &str) -> Option<BuiltinSkillSpec> {
    BUILTIN_SPECS.get(dir_name).cloned()
}

pub fn builtin_skill_group(dir_name: &str) -> Option<BuiltinSkillGroup> {
    BUILTIN_SPECS.get(dir_name).map(|spec| spec.group)
}

pub fn builtin_skill_group_key(dir_name: &str) -> Option<&'static str> {
    builtin_skill_group(dir_name).map(BuiltinSkillGroup::as_str)
}

#[cfg(test)]
mod tests {
    use super::{builtin_skill_group, builtin_skill_group_key, BUILTIN_SPECS};
    use crate::agentic::tools::implementations::skills::builtin::builtin_skill_dir_names;
    use std::collections::HashSet;

    #[test]
    fn builtin_skill_groups_match_expected_sets() {
        assert_eq!(builtin_skill_group_key("docx"), Some("office"));
        assert_eq!(builtin_skill_group_key("pdf"), Some("office"));
        assert_eq!(builtin_skill_group_key("ppt-design"), Some("office"));
        assert_eq!(builtin_skill_group_key("pptx"), Some("office"));
        assert_eq!(builtin_skill_group_key("xlsx"), Some("office"));
        assert_eq!(builtin_skill_group_key("find-skills"), Some("meta"));
        assert_eq!(builtin_skill_group_key("writing-skills"), Some("meta"));
        assert_eq!(builtin_skill_group_key("memory"), Some("meta"));
        assert_eq!(builtin_skill_group_key("agent-browser"), Some("computer-use"));
        assert_eq!(builtin_skill_group_key("gstack-review"), Some("gstack"));
        assert_eq!(builtin_skill_group("unknown-skill"), None);
    }

    #[test]
    fn catalog_covers_all_embedded_builtin_skills() {
        let known: HashSet<&str> = BUILTIN_SPECS.keys().map(|s| s.as_str()).collect();

        for dir_name in builtin_skill_dir_names() {
            assert!(
                known.contains(dir_name.as_str()),
                "Missing built-in skill catalog entry for '{}'",
                dir_name
            );
        }
    }
}
