//! Skill registry local + remote filesystem scan, and cache management.
//!
//! This module owns the IO-heavy half of [`super::SkillRegistry`]:
//! resolving the on-disk search roots, walking each root for `SKILL.md`
//! directories, and the cached `Vec<SkillInfo>` that holds the user-level
//! skill set between refreshes.
//!
//! It uses [`super::registry_types`] for the slot tables, internal
//! candidate types, and pure helper functions; behavior is otherwise
//! unchanged from the pre-split `registry.rs`.

use super::builtin::ensure_builtin_skills_installed;
use super::registry::SkillRegistry;
use super::registry_types::{
    annotate_shadowed_skills, normalize_dir_name, normalize_remote_dir_name, sort_remote_dir_entries, sort_skills,
    RemoteSkillRootEntry, SkillCandidate, SkillRootEntry, NORTHHING_SYSTEM_DIR_NAME, NORTHHING_SYSTEM_SLOT,
    NORTHHING_USER_SLOT, PROJECT_PREFIX, PROJECT_SKILL_SLOTS, USER_CONFIG_SKILL_SLOTS, USER_HOME_SKILL_SLOTS,
    USER_PREFIX,
};
use super::types::SkillLocation;
use crate::agentic::workspace::WorkspaceFileSystem;
use crate::infrastructure::path_manager_arc;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error};

impl SkillRegistry {
    /// Resolve every on-disk directory that may contain skill roots for the
    /// given workspace, in priority order.
    pub(super) fn get_possible_paths_for_workspace(workspace_root: Option<&Path>) -> Vec<SkillRootEntry> {
        let mut entries = Vec::new();
        let mut priority = 0usize;

        if let Some(workspace_path) = workspace_root {
            for (parent, sub, slot) in PROJECT_SKILL_SLOTS {
                let path = workspace_path.join(parent).join(sub);
                if path.exists() && path.is_dir() {
                    entries.push(SkillRootEntry {
                        path,
                        level: SkillLocation::Project,
                        slot,
                        priority,
                        is_builtin: false,
                    });
                }
                priority += 1;
            }
        }

        if let Some(home) = dirs::home_dir() {
            for (parent, sub, slot) in USER_HOME_SKILL_SLOTS {
                let path = home.join(parent).join(sub);
                if path.exists() && path.is_dir() {
                    entries.push(SkillRootEntry {
                        path,
                        level: SkillLocation::User,
                        slot,
                        priority,
                        is_builtin: false,
                    });
                }
                priority += 1;
            }
        }

        // northhing's own user-defined skills sit between home slots and config slots.
        // This lets other agent directories (e.g. ~/.claude/skills) take precedence
        // while still keeping config-level overrides after northhing defaults.
        let path_manager = path_manager_arc();
        let northhing_skills = path_manager.user_skills_dir();
        if northhing_skills.exists() && northhing_skills.is_dir() {
            entries.push(SkillRootEntry {
                path: northhing_skills,
                level: SkillLocation::User,
                slot: NORTHHING_USER_SLOT,
                priority,
                is_builtin: false,
            });
        }
        priority += 1;

        let builtin_skills = path_manager.builtin_skills_dir();
        if builtin_skills.exists() && builtin_skills.is_dir() {
            entries.push(SkillRootEntry {
                path: builtin_skills,
                level: SkillLocation::User,
                slot: NORTHHING_SYSTEM_SLOT,
                priority,
                is_builtin: true,
            });
        }
        priority += 1;

        if let Some(config_dir) = dirs::config_dir() {
            for (parent, sub, slot) in USER_CONFIG_SKILL_SLOTS {
                let path = config_dir.join(parent).join(sub);
                if path.exists() && path.is_dir() {
                    entries.push(SkillRootEntry {
                        path,
                        level: SkillLocation::User,
                        slot,
                        priority,
                        is_builtin: false,
                    });
                }
                priority += 1;
            }
        }

        entries
    }

    /// Walk a single skill-root directory and collect the `SkillCandidate`s
    /// it contains, sorted by directory name.
    pub(super) async fn scan_skills_in_dir(entry: &SkillRootEntry) -> Vec<SkillCandidate> {
        let mut skills = Vec::new();
        if !entry.path.exists() {
            return skills;
        }

        let Ok(mut read_dir) = fs::read_dir(&entry.path).await else {
            return skills;
        };

        while let Ok(Some(item)) = read_dir.next_entry().await {
            let path = item.path();
            if !path.is_dir() {
                continue;
            }

            let Some(dir_name) = normalize_dir_name(&path) else {
                continue;
            };

            if entry.slot == NORTHHING_USER_SLOT && dir_name == NORTHHING_SYSTEM_DIR_NAME {
                continue;
            }

            let skill_md_path = path.join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            match fs::read_to_string(&skill_md_path).await {
                Ok(content) => match super::types::SkillData::from_markdown(
                    path.to_string_lossy().to_string(),
                    &content,
                    entry.level,
                    false,
                ) {
                    Ok(mut skill_data) => {
                        skill_data.dir_name = dir_name;
                        let key_prefix = match entry.level {
                            SkillLocation::User => USER_PREFIX,
                            SkillLocation::Project => PROJECT_PREFIX,
                        };
                        skills.push(SkillCandidate::from_data(
                            skill_data,
                            entry.slot,
                            key_prefix,
                            entry.priority,
                            entry.is_builtin,
                        ));
                    }
                    Err(error) => {
                        error!("Failed to parse SKILL.md in {}: {}", path.display(), error);
                    }
                },
                Err(error) => {
                    debug!("Failed to read {}: {}", skill_md_path.display(), error);
                }
            }
        }

        skills.sort_by(|a, b| {
            a.info
                .dir_name
                .to_lowercase()
                .cmp(&b.info.dir_name.to_lowercase())
                .then_with(|| a.info.dir_name.cmp(&b.info.dir_name))
                .then_with(|| a.info.key.cmp(&b.info.key))
        });
        skills
    }

    /// Scan every local skill root that may be relevant for the given
    /// workspace, including user-level slots and builtin slots.
    pub(super) async fn scan_skill_candidates_for_workspace(
        &self,
        workspace_root: Option<&Path>,
    ) -> Vec<SkillCandidate> {
        if let Err(error) = ensure_builtin_skills_installed().await {
            debug!("Failed to install built-in skills: {}", error);
        }

        let mut skills = Vec::new();
        for entry in Self::get_possible_paths_for_workspace(workspace_root) {
            let mut part = Self::scan_skills_in_dir(&entry).await;
            skills.append(&mut part);
        }
        skills
    }

    /// Walk a remote workspace via [`WorkspaceFileSystem`] and collect the
    /// `SkillCandidate`s living under the project's skill roots.
    pub(super) async fn scan_remote_project_skills(
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
    ) -> Vec<SkillCandidate> {
        let mut roots = Vec::new();
        let root = remote_root.trim_end_matches('/');
        for (priority, (parent, sub, slot)) in PROJECT_SKILL_SLOTS.iter().enumerate() {
            let path = format!("{}/{}/{}", root, parent, sub);
            if fs.is_dir(&path).await.unwrap_or(false) {
                roots.push(RemoteSkillRootEntry { path, slot, priority });
            }
        }

        let mut skills = Vec::new();
        for entry in roots {
            let mut entries = match fs.read_dir(&entry.path).await {
                Ok(value) => value,
                Err(_) => continue,
            };
            sort_remote_dir_entries(&mut entries);

            for item in entries {
                if !item.is_dir || item.is_symlink {
                    continue;
                }

                let Some(dir_name) = normalize_remote_dir_name(&item.path) else {
                    continue;
                };
                let skill_md_path = format!("{}/SKILL.md", item.path.trim_end_matches('/'));
                if !fs.is_file(&skill_md_path).await.unwrap_or(false) {
                    continue;
                }

                match fs.read_file_text(&skill_md_path).await {
                    Ok(content) => match super::types::SkillData::from_markdown(
                        item.path.clone(),
                        &content,
                        SkillLocation::Project,
                        false,
                    ) {
                        Ok(mut skill_data) => {
                            skill_data.dir_name = dir_name;
                            skills.push(SkillCandidate::from_data(
                                skill_data,
                                entry.slot,
                                PROJECT_PREFIX,
                                entry.priority,
                                false,
                            ));
                        }
                        Err(error) => {
                            error!("Failed to parse SKILL.md in {}: {}", item.path, error);
                        }
                    },
                    Err(error) => {
                        debug!("Failed to read {}: {}", skill_md_path, error);
                    }
                }
            }
        }

        skills
    }

    /// Compose local user/builtin candidates with the remote workspace's
    /// project candidates.
    pub(super) async fn scan_skill_candidates_for_remote_workspace(
        &self,
        fs: &dyn WorkspaceFileSystem,
        remote_root: &str,
    ) -> Vec<SkillCandidate> {
        let mut skills = self.scan_skill_candidates_for_workspace(None).await;
        skills.extend(Self::scan_remote_project_skills(fs, remote_root).await);
        skills
    }

    /// Lazily populate the cache the first time the registry is queried.
    pub(super) async fn ensure_loaded(&self) {
        let cache = self.cache.read().await;
        if cache.is_empty() {
            drop(cache);
            self.refresh().await;
        }
    }

    /// Re-scan user-level skills (no workspace-specific project skills) and
    /// rebuild the cache.
    pub async fn refresh(&self) {
        let skills = sort_skills(annotate_shadowed_skills(
            self.scan_skill_candidates_for_workspace(None).await,
        ));
        let mut cache = self.cache.write().await;
        *cache = skills;
    }

    /// Refresh the registry; currently the workspace root is informational
    /// because user-level skills are cached globally.
    pub async fn refresh_for_workspace(&self, _workspace_root: Option<&Path>) {
        self.refresh().await;
    }

    /// Snapshot of the cached user-level skill set. Lazily populated on first
    /// call.
    pub async fn get_all_skills(&self) -> Vec<super::types::SkillInfo> {
        self.ensure_loaded().await;
        let cache = self.cache.read().await;
        cache.clone()
    }
}
