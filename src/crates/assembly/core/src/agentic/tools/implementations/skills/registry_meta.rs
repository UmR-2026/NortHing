//! Skill registry public query and serialization API.
//!
//! This module owns the high-level methods that external callers (UI,
//! prompt builder, tool wrappers) use to ask the registry for skill data.
//! Each method composes the lower-level scan / filter / sort pieces owned
//! by [`super::registry_store`] and [`super::registry_dispatch`].
//!
//! Behavior is unchanged from the pre-split `registry.rs`.

use super::mode_overrides::{
    load_disabled_mode_skills_local, load_disabled_mode_skills_remote, load_user_mode_skill_overrides,
    UserModeSkillOverrides,
};
use super::registry::SkillRegistry;
use super::registry_types::{
    annotate_shadowed_skills, build_mode_skill_infos, dedupe_preserving_order, filter_candidates_for_mode,
    resolve_visible_skills, sort_resolved_skills_for_presentation, sort_skills,
};
use super::types::{ModeSkillInfo, SkillInfo};
use crate::agentic::workspace::WorkspaceFileSystem;
use std::collections::HashSet;
use std::path::Path;

impl SkillRegistry {
    /// All skills visible for the given workspace (local user-level plus the
    /// workspace's project-level skills).
    pub async fn get_all_skills_for_workspace(&self, workspace_root: Option<&Path>) -> Vec<SkillInfo> {
        sort_skills(annotate_shadowed_skills(
            self.scan_skill_candidates_for_workspace(workspace_root).await,
        ))
    }

    /// Remote-workspace counterpart of [`Self::get_all_skills_for_workspace`].
    pub async fn get_all_skills_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
    ) -> Vec<SkillInfo> {
        sort_skills(annotate_shadowed_skills(
            self.scan_skill_candidates_for_remote_workspace(fs, remote_root).await,
        ))
    }

    /// Skills selected for runtime injection given the workspace and mode,
    /// in presentation order.
    pub async fn get_resolved_skills_for_workspace(
        &self,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> Vec<SkillInfo> {
        let candidates = self.scan_skill_candidates_for_workspace(workspace_root).await;
        let filtered = self
            .apply_mode_filters_for_workspace(candidates, workspace_root, agent_type)
            .await;
        sort_resolved_skills_for_presentation(resolve_visible_skills(filtered))
    }

    /// Remote-workspace counterpart of [`Self::get_resolved_skills_for_workspace`].
    pub async fn get_resolved_skills_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> Vec<SkillInfo> {
        let candidates = self.scan_skill_candidates_for_remote_workspace(fs, remote_root).await;
        let filtered = self
            .apply_mode_filters_for_remote_workspace(candidates, fs, remote_root, agent_type)
            .await;
        sort_resolved_skills_for_presentation(resolve_visible_skills(filtered))
    }

    /// Per-mode skill rows for the given workspace, including shadowed
    /// skills (so callers can show the full set with effective state).
    pub async fn get_mode_skill_infos_for_workspace(
        &self,
        workspace_root: Option<&Path>,
        mode_id: &str,
    ) -> Vec<ModeSkillInfo> {
        let candidates = self.scan_skill_candidates_for_workspace(workspace_root).await;
        let all_skills = sort_skills(annotate_shadowed_skills(candidates.clone()));
        let user_overrides = load_user_mode_skill_overrides(mode_id)
            .await
            .unwrap_or_else(|_| UserModeSkillOverrides::default());
        let disabled_project = match workspace_root {
            Some(root) => load_disabled_mode_skills_local(root, mode_id).await.unwrap_or_default(),
            None => Vec::new(),
        };
        let disabled_project: HashSet<String> = dedupe_preserving_order(disabled_project).into_iter().collect();
        let filtered = filter_candidates_for_mode(candidates, mode_id, &user_overrides, &disabled_project);
        let resolved = resolve_visible_skills(filtered);

        build_mode_skill_infos(all_skills, resolved, mode_id, &user_overrides, &disabled_project)
    }

    /// Remote-workspace counterpart of [`Self::get_mode_skill_infos_for_workspace`].
    pub async fn get_mode_skill_infos_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        mode_id: &str,
    ) -> Vec<ModeSkillInfo> {
        let candidates = self.scan_skill_candidates_for_remote_workspace(fs, remote_root).await;
        let all_skills = sort_skills(annotate_shadowed_skills(candidates.clone()));
        let user_overrides = load_user_mode_skill_overrides(mode_id)
            .await
            .unwrap_or_else(|_| UserModeSkillOverrides::default());
        let disabled_project = load_disabled_mode_skills_remote(fs, remote_root, mode_id)
            .await
            .unwrap_or_default();
        let disabled_project: HashSet<String> = dedupe_preserving_order(disabled_project).into_iter().collect();
        let filtered = filter_candidates_for_mode(candidates, mode_id, &user_overrides, &disabled_project);
        let resolved = resolve_visible_skills(filtered);

        build_mode_skill_infos(all_skills, resolved, mode_id, &user_overrides, &disabled_project)
    }

    /// Look up a skill by its stable key in the local workspace.
    pub async fn find_skill_by_key_for_workspace(
        &self,
        skill_key: &str,
        workspace_root: Option<&Path>,
    ) -> Option<SkillInfo> {
        self.get_all_skills_for_workspace(workspace_root)
            .await
            .into_iter()
            .find(|skill| skill.key == skill_key)
    }

    /// Remote-workspace counterpart of [`Self::find_skill_by_key_for_workspace`].
    pub async fn find_skill_by_key_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        skill_key: &str,
    ) -> Option<SkillInfo> {
        self.get_all_skills_for_remote_workspace(fs, remote_root)
            .await
            .into_iter()
            .find(|skill| skill.key == skill_key)
    }

    /// XML snippet per resolved skill — used by prompt builders that want
    /// to inline skill descriptions.
    pub async fn get_resolved_skills_xml_for_workspace(
        &self,
        workspace_root: Option<&Path>,
        agent_type: Option<&str>,
    ) -> Vec<String> {
        self.get_resolved_skills_for_workspace(workspace_root, agent_type)
            .await
            .into_iter()
            .map(|skill| skill.to_xml_desc())
            .collect()
    }

    /// Remote-workspace counterpart of [`Self::get_resolved_skills_xml_for_workspace`].
    pub async fn get_resolved_skills_xml_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
        agent_type: Option<&str>,
    ) -> Vec<String> {
        self.get_resolved_skills_for_remote_workspace(fs, remote_root, agent_type)
            .await
            .into_iter()
            .map(|skill| skill.to_xml_desc())
            .collect()
    }
}
