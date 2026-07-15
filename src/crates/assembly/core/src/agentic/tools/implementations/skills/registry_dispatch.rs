//! Skill registry mode-filtering and explicit-skill-loading logic.
//!
//! This module owns the methods that translate a raw set of
//! `SkillCandidate`s into a mode-specific view of skills, and the methods
//! that load a single skill's `SKILL.md` for explicit invocation by a user
//! or model.
//!
//! The mode-filter helpers used here come from [`super::resolver`] and
//! [`super::mode_overrides`]; pure candidate helpers come from
//! [`super::registry_types`]; filesystem scan helpers come from sibling
//! [`super::registry_store`].

use super::mode_overrides::{
    load_disabled_mode_skills_local, load_disabled_mode_skills_remote, load_user_mode_skill_overrides,
    UserModeSkillOverrides,
};
use super::registry::SkillRegistry;
use super::registry_types::{
    dedupe_preserving_order, filter_candidates_for_mode, resolve_visible_skills, SkillCandidate,
};
use super::resolver::resolve_skill_default_enabled_for_mode;
use super::types::{SkillData, SkillInfo, SkillLocation};
use crate::agentic::workspace::WorkspaceFileSystem;
use crate::util::errors::{NortHingError, NortHingResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

impl SkillRegistry {
    /// Apply user-overrides + project-disabled lists for the local workspace
    /// against the candidate set, returning only the candidates that are
    /// effective-enabled for the given mode.
    pub(super) async fn apply_mode_filters_for_workspace(
        &self,
        candidates: Vec<SkillCandidate>,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> Vec<SkillCandidate> {
        let Some(mode_id) = agent_type.map(str::trim).filter(|value| !value.is_empty()) else {
            return candidates;
        };

        let user_overrides = load_user_mode_skill_overrides(mode_id)
            .await
            .unwrap_or_else(|_| UserModeSkillOverrides::default());
        let disabled_project = match workspace_root {
            Some(root) => load_disabled_mode_skills_local(root, mode_id).await.unwrap_or_default(),
            None => Vec::new(),
        };

        let disabled_project: HashSet<String> = dedupe_preserving_order(disabled_project).into_iter().collect();

        filter_candidates_for_mode(candidates, mode_id, &user_overrides, &disabled_project)
    }

    /// Remote-workspace counterpart of [`Self::apply_mode_filters_for_workspace`].
    pub(super) async fn apply_mode_filters_for_remote_workspace(
        &self,
        candidates: Vec<SkillCandidate>,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> Vec<SkillCandidate> {
        let Some(mode_id) = agent_type.map(str::trim).filter(|value| !value.is_empty()) else {
            return candidates;
        };

        let user_overrides = load_user_mode_skill_overrides(mode_id)
            .await
            .unwrap_or_else(|_| UserModeSkillOverrides::default());
        let disabled_project = load_disabled_mode_skills_remote(fs, remote_root, mode_id)
            .await
            .unwrap_or_default();

        let disabled_project: HashSet<String> = dedupe_preserving_order(disabled_project).into_iter().collect();

        filter_candidates_for_mode(candidates, mode_id, &user_overrides, &disabled_project)
    }

    /// Resolve a skill by name for an explicit invocation against the local
    /// workspace. Returns the visible-skill winner, falling back to a
    /// default-hidden builtin if the user explicitly named one.
    pub(super) async fn find_skill_info_for_explicit_invocation_workspace(
        &self,
        skill_name: &str,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillInfo> {
        let candidates = self.scan_skill_candidates_for_workspace(workspace_root).await;
        let filtered = self
            .apply_mode_filters_for_workspace(candidates.clone(), workspace_root, agent_type)
            .await;
        if let Some(info) = resolve_visible_skills(filtered)
            .into_iter()
            .find(|skill| skill.name == skill_name)
        {
            return Ok(info);
        }

        Self::find_default_hidden_builtin_for_explicit_invocation(skill_name, candidates, agent_type)
    }

    /// Remote-workspace counterpart of
    /// [`Self::find_skill_info_for_explicit_invocation_workspace`].
    pub(super) async fn find_skill_info_for_explicit_invocation_remote_workspace(
        &self,
        skill_name: &str,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillInfo> {
        let candidates = self.scan_skill_candidates_for_remote_workspace(fs, remote_root).await;
        let filtered = self
            .apply_mode_filters_for_remote_workspace(candidates.clone(), fs, remote_root, agent_type)
            .await;
        if let Some(info) = resolve_visible_skills(filtered)
            .into_iter()
            .find(|skill| skill.name == skill_name)
        {
            return Ok(info);
        }

        Self::find_default_hidden_builtin_for_explicit_invocation(skill_name, candidates, agent_type)
    }

    /// Look up a builtin skill that the user explicitly named but is hidden
    /// by default for the current mode. Returns an error if the skill is
    /// not a hidden builtin or the mode cannot be resolved.
    pub(super) fn find_default_hidden_builtin_for_explicit_invocation(
        skill_name: &str,
        candidates: Vec<SkillCandidate>,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillInfo> {
        let Some(mode_id) = agent_type.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(NortHingError::tool(format!("Skill '{}' not found", skill_name)));
        };

        let info = resolve_visible_skills(candidates)
            .into_iter()
            .find(|skill| skill.name == skill_name)
            .ok_or_else(|| NortHingError::tool(format!("Skill '{}' not found", skill_name)))?;

        if info.level == SkillLocation::User
            && info.is_builtin
            && info.group_key.as_deref() == Some("gstack")
            && !resolve_skill_default_enabled_for_mode(&info, mode_id)
        {
            return Ok(info);
        }

        Err(NortHingError::tool(format!(
            "Skill '{}' is disabled for mode '{}'. Enable it in mode skill settings or switch to a mode where it is enabled.",
            skill_name, mode_id
        )))
    }

    /// Find and load a skill by name from the local workspace for explicit
    /// invocation; reads the `SKILL.md` file and parses it into a
    /// [`SkillData`].
    pub async fn find_and_load_skill_for_workspace(
        &self,
        skill_name: &str,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillData> {
        let info = self
            .find_skill_info_for_explicit_invocation_workspace(skill_name, workspace_root, agent_type)
            .await?;

        let skill_md_path = PathBuf::from(&info.path).join("SKILL.md");
        let content = fs::read_to_string(&skill_md_path)
            .await
            .map_err(|error| NortHingError::tool(format!("Failed to read skill file: {}", error)))?;

        let mut data = SkillData::from_markdown(info.path.clone(), &content, info.level, true)?;
        data.key = info.key;
        data.source_slot = info.source_slot;
        data.dir_name = info.dir_name;
        Ok(data)
    }

    /// Find and load a skill by stable key from the local workspace for
    /// explicit invocation.
    pub async fn find_and_load_skill_by_key_for_workspace(
        &self,
        skill_key: &str,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillData> {
        let candidates = self.scan_skill_candidates_for_workspace(workspace_root).await;
        let filtered = self
            .apply_mode_filters_for_workspace(candidates, workspace_root, agent_type)
            .await;
        let info = filtered
            .into_iter()
            .map(|candidate| candidate.info)
            .find(|skill| skill.key == skill_key)
            .ok_or_else(|| {
                NortHingError::tool(format!(
                    "Skill key '{}' was not found or is disabled for this mode",
                    skill_key
                ))
            })?;

        let skill_md_path = PathBuf::from(&info.path).join("SKILL.md");
        let content = fs::read_to_string(&skill_md_path)
            .await
            .map_err(|error| NortHingError::tool(format!("Failed to read skill file: {}", error)))?;

        let mut data = SkillData::from_markdown(info.path.clone(), &content, info.level, true)?;
        data.key = info.key;
        data.source_slot = info.source_slot;
        data.dir_name = info.dir_name;
        Ok(data)
    }

    /// Remote-workspace counterpart of [`Self::find_and_load_skill_for_workspace`].
    pub async fn find_and_load_skill_for_remote_workspace(
        &self,
        skill_name: &str,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillData> {
        let info = self
            .find_skill_info_for_explicit_invocation_remote_workspace(skill_name, fs, remote_root, agent_type)
            .await?;

        let content = Self::read_skill_md_for_remote_merge(&info, fs).await?;
        let mut data = SkillData::from_markdown(info.path.clone(), &content, info.level, true)?;
        data.key = info.key;
        data.source_slot = info.source_slot;
        data.dir_name = info.dir_name;
        Ok(data)
    }

    /// Remote-workspace counterpart of [`Self::find_and_load_skill_by_key_for_workspace`].
    pub async fn find_and_load_skill_by_key_for_remote_workspace(
        &self,
        skill_key: &str,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> NortHingResult<SkillData> {
        let candidates = self.scan_skill_candidates_for_remote_workspace(fs, remote_root).await;
        let filtered = self
            .apply_mode_filters_for_remote_workspace(candidates, fs, remote_root, agent_type)
            .await;
        let info = filtered
            .into_iter()
            .map(|candidate| candidate.info)
            .find(|skill| skill.key == skill_key)
            .ok_or_else(|| {
                NortHingError::tool(format!(
                    "Skill key '{}' was not found or is disabled for this mode",
                    skill_key
                ))
            })?;

        let content = Self::read_skill_md_for_remote_merge(&info, fs).await?;
        let mut data = SkillData::from_markdown(info.path.clone(), &content, info.level, true)?;
        data.key = info.key;
        data.source_slot = info.source_slot;
        data.dir_name = info.dir_name;
        Ok(data)
    }

    /// Read the `SKILL.md` body for a remote skill — user-level skills live
    /// on the local disk and project-level skills live on the remote FS.
    pub(super) async fn read_skill_md_for_remote_merge(
        info: &SkillInfo,
        remote_fs: &dyn WorkspaceFileSystem,
    ) -> NortHingResult<String> {
        match info.level {
            SkillLocation::User => {
                let skill_md_path = PathBuf::from(&info.path).join("SKILL.md");
                fs::read_to_string(&skill_md_path)
                    .await
                    .map_err(|error| NortHingError::tool(format!("Failed to read skill file: {}", error)))
            }
            SkillLocation::Project => {
                let skill_md_path = format!("{}/SKILL.md", info.path.trim_end_matches('/'));
                remote_fs
                    .read_file_text(&skill_md_path)
                    .await
                    .map_err(|error| NortHingError::tool(format!("Failed to read skill file: {}", error)))
            }
        }
    }
}
