//! Skill registry shared types and pure helpers.
//!
//! This module collects the constants, slot tables, internal candidate types,
//! and pure helper functions shared by the `SkillRegistry` impl blocks living
//! in [`super::registry_store`], [`super::registry_dispatch`], and
//! [`super::registry_meta`].
//!
//! Keeping these helpers in one place lets the sibling impl files focus on
//! IO, mode filtering, and public query/serialization logic without
//! re-declaring the same support code.

use super::catalog::builtin_skill_group_key;
use super::mode_overrides::UserModeSkillOverrides;
use super::resolver::resolve_skill_state_for_mode;
use super::types::{ModeSkillInfo, SkillData, SkillInfo, SkillLocation};
use crate::agentic::workspace::WorkspaceDirEntry;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ----- Prefix and slot-name constants --------------------------------------

pub(super) const USER_PREFIX: &str = "user";
pub(super) const PROJECT_PREFIX: &str = "project";
pub(super) const NORTHHING_USER_SLOT: &str = "northhing";
pub(super) const NORTHHING_SYSTEM_SLOT: &str = "northhing-system";
pub(super) const NORTHHING_SYSTEM_DIR_NAME: &str = ".system";

/// Project-level skill roots under a workspace.
pub(super) const PROJECT_SKILL_SLOTS: &[(&str, &str, &str)] = &[
    (".northhing", "skills", "northhing"),
    (".claude", "skills", "claude"),
    (".codex", "skills", "codex"),
    (".cursor", "skills", "cursor"),
    (".opencode", "skills", "opencode"),
    (".agents", "skills", "agents"),
];

/// Home-directory based user-level skill roots.
pub(super) const USER_HOME_SKILL_SLOTS: &[(&str, &str, &str)] = &[
    (".claude", "skills", "home.claude"),
    (".codex", "skills", "home.codex"),
    (".cursor", "skills", "home.cursor"),
    (".agents", "skills", "home.agents"),
];

/// Config-directory based user-level skill roots.
pub(super) const USER_CONFIG_SKILL_SLOTS: &[(&str, &str, &str)] = &[
    ("opencode", "skills", "config.opencode"),
    ("agents", "skills", "config.agents"),
];

// ----- Internal candidate / root types -------------------------------------

#[derive(Debug, Clone)]
pub(super) struct SkillRootEntry {
    pub(super) path: PathBuf,
    pub(super) level: SkillLocation,
    pub(super) slot: &'static str,
    pub(super) priority: usize,
    pub(super) is_builtin: bool,
}

#[derive(Debug, Clone)]
pub(super) struct RemoteSkillRootEntry {
    pub(super) path: String,
    pub(super) slot: &'static str,
    pub(super) priority: usize,
}

#[derive(Debug, Clone)]
pub(super) struct SkillCandidate {
    pub(super) info: SkillInfo,
    pub(super) priority: usize,
}

impl SkillCandidate {
    pub(super) fn from_data(
        mut data: SkillData,
        slot: &str,
        key_prefix: &str,
        priority: usize,
        is_builtin: bool,
    ) -> Self {
        data.source_slot = slot.to_string();
        data.key = build_skill_key(key_prefix, slot, &data.dir_name);
        let group_key = if is_builtin {
            builtin_skill_group_key(&data.dir_name).map(str::to_string)
        } else {
            None
        };

        Self {
            info: SkillInfo {
                key: data.key,
                name: data.name,
                description: data.description,
                path: data.path,
                level: data.location,
                source_slot: data.source_slot,
                dir_name: data.dir_name,
                is_builtin,
                group_key,
                is_shadowed: false,
                shadowed_by_key: None,
            },
            priority,
        }
    }
}

// ----- Pure helpers --------------------------------------------------------

pub(super) fn build_skill_key(prefix: &str, slot: &str, dir_name: &str) -> String {
    format!("{}::{}::{}", prefix, slot, dir_name)
}

pub(super) fn normalize_dir_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn normalize_remote_dir_name(path: &str) -> Option<String> {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

pub(super) fn dedupe_preserving_order(keys: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for key in keys {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }

        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            normalized.push(owned);
        }
    }

    normalized
}

pub(super) fn sort_skills(mut skills: Vec<SkillInfo>) -> Vec<SkillInfo> {
    skills.sort_by(|a, b| {
        skill_level_rank(a.level)
            .cmp(&skill_level_rank(b.level))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.key.cmp(&b.key))
    });
    skills
}

pub(super) fn skill_level_rank(level: SkillLocation) -> u8 {
    match level {
        SkillLocation::Project => 0,
        SkillLocation::User => 1,
    }
}

pub(super) fn skill_candidate_precedence(candidate: &SkillCandidate) -> (usize, u8, String, String, String) {
    (
        candidate.priority,
        skill_level_rank(candidate.info.level),
        candidate.info.name.to_lowercase(),
        candidate.info.name.clone(),
        candidate.info.key.clone(),
    )
}

pub(super) fn sort_resolved_skill_candidates(mut resolved: Vec<SkillCandidate>) -> Vec<SkillCandidate> {
    resolved.sort_by_key(skill_candidate_precedence);
    resolved
}

pub(super) fn sort_skill_candidates_for_resolution(mut candidates: Vec<SkillCandidate>) -> Vec<SkillCandidate> {
    candidates.sort_by(|a, b| {
        skill_candidate_precedence(a)
            .cmp(&skill_candidate_precedence(b))
            .then_with(|| a.info.path.cmp(&b.info.path))
    });
    candidates
}

pub(super) fn sort_remote_dir_entries(entries: &mut [WorkspaceDirEntry]) {
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });
}

pub(super) fn resolve_visible_skills(candidates: Vec<SkillCandidate>) -> Vec<SkillInfo> {
    let mut by_name: HashMap<String, SkillCandidate> = HashMap::new();
    for candidate in sort_skill_candidates_for_resolution(candidates) {
        match by_name.get(&candidate.info.name) {
            Some(existing) if skill_candidate_precedence(existing) <= skill_candidate_precedence(&candidate) => {}
            _ => {
                by_name.insert(candidate.info.name.clone(), candidate);
            }
        }
    }

    sort_resolved_skill_candidates(by_name.into_values().collect())
        .into_iter()
        .map(|candidate| candidate.info)
        .collect()
}

pub(super) fn sort_resolved_skills_for_presentation(skills: Vec<SkillInfo>) -> Vec<SkillInfo> {
    let mut skills = skills;
    skills.sort_by(|a, b| {
        skill_level_rank(a.level)
            .cmp(&skill_level_rank(b.level))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.key.cmp(&b.key))
    });
    skills
}

pub(super) fn filter_candidates_for_mode(
    candidates: Vec<SkillCandidate>,
    mode_id: &str,
    user_overrides: &UserModeSkillOverrides,
    disabled_project_skills: &HashSet<String>,
) -> Vec<SkillCandidate> {
    candidates
        .into_iter()
        .filter(|candidate| {
            resolve_skill_state_for_mode(&candidate.info, mode_id, user_overrides, disabled_project_skills)
                .effective_enabled
        })
        .collect()
}

/// Annotate each candidate with shadowing information.
/// For every skill that has a higher-priority (lower number) skill with the same name,
/// set `is_shadowed = true` and `shadowed_by_key` to the winner's key.
pub(super) fn annotate_shadowed_skills(candidates: Vec<SkillCandidate>) -> Vec<SkillInfo> {
    let mut by_name: HashMap<String, SkillCandidate> = HashMap::new();
    for candidate in &candidates {
        match by_name.get(&candidate.info.name) {
            Some(existing) if existing.priority <= candidate.priority => {}
            _ => {
                by_name.insert(candidate.info.name.clone(), candidate.clone());
            }
        }
    }

    candidates
        .into_iter()
        .map(|mut candidate| {
            if let Some(winner) = by_name.get(&candidate.info.name) {
                if winner.info.key != candidate.info.key {
                    candidate.info.is_shadowed = true;
                    candidate.info.shadowed_by_key = Some(winner.info.key.clone());
                }
            }
            candidate.info
        })
        .collect()
}

/// Build the per-mode `ModeSkillInfo` rows that surface state and selection
/// to UIs.
///
/// The rows are zipped against `all_skills` (which still carries every
/// discovered skill, including shadowed ones) so that callers can show the
/// user the full set even when only a subset is selected for runtime.
pub(super) fn build_mode_skill_infos(
    all_skills: Vec<SkillInfo>,
    resolved_skills: Vec<SkillInfo>,
    mode_id: &str,
    user_overrides: &UserModeSkillOverrides,
    disabled_project_skills: &HashSet<String>,
) -> Vec<ModeSkillInfo> {
    let resolved_keys: HashSet<String> = resolved_skills.into_iter().map(|skill| skill.key).collect();

    all_skills
        .into_iter()
        .map(|skill| {
            let state = resolve_skill_state_for_mode(&skill, mode_id, user_overrides, disabled_project_skills);
            let selected_for_runtime = resolved_keys.contains(&skill.key);

            ModeSkillInfo {
                skill,
                default_enabled: state.default_enabled,
                effective_enabled: state.effective_enabled,
                disabled_by_mode: !state.effective_enabled,
                selected_for_runtime,
                state_reason: state.reason,
            }
        })
        .collect()
}
