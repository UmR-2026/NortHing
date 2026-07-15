//! Chat mode skill management — selector, reload, enable/disable.
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;
use crate::ui::skill_selector::{SkillItem, SkillSelectorAction};

use northhing_core::agentic::tools::implementations::skills::{
    mode_overrides::{
        load_project_mode_skills_document_local, save_project_mode_skills_document_local,
        set_mode_skill_disabled_in_document, set_user_mode_skill_state,
    },
    registry::SkillRegistry,
    ModeSkillInfo, SkillInfo,
};

use super::ChatMode;

impl ChatMode {
    /// Show skill list/configuration menu.
    pub(crate) fn show_skill_selector(
        &self,
        chat_view: &mut ChatView,
        _chat_state: &mut ChatState,
        _rt_handle: &tokio::runtime::Handle,
    ) {
        chat_view.show_skill_menu();
    }

    /// Re-scan skill directories from disk and rebuild the registry cache.
    ///
    /// Mirrors Claude Code 2.1.152 `/reload-skills`. Safe to call at any
    /// time — does not require `is_processing` to be false because the
    /// registry swap is atomic and a held `SkillInfo` reference is not
    /// kept across the call.
    pub(crate) fn reload_skills_from_disk(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let registry = SkillRegistry::global();
        let workspace = self.agent.workspace_path_buf();
        let outcome = tokio::task::block_in_place(|| {
            // refresh() is the global re-scan entry point; the workspace
            // arg of refresh_for_workspace is currently a no-op upstream,
            // so we call refresh() directly and re-resolve the workspace
            // count afterwards.
            rt_handle.block_on(async {
                registry.refresh().await;
                registry
                    .get_resolved_skills_for_workspace(Some(workspace.as_path()), None)
                    .await
            })
        });

        let count = outcome.len();
        chat_state.add_system_message(format!("Reloaded {} skill(s) from disk.", count));
        chat_view.set_status(Some(format!("Skills reloaded ({} available)", count)));
    }

    pub(crate) fn show_available_skill_list(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let skills = tokio::task::block_in_place(|| {
            let workspace = self.agent.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            rt_handle.block_on(async {
                let registry = SkillRegistry::global();
                registry
                    .get_resolved_skills_for_workspace(Some(workspace.as_path()), Some(&agent_type))
                    .await
            })
        });

        if skills.is_empty() {
            chat_state.add_system_message(format!(
                "No enabled skills found for agent mode '{}'. Add skills in .northhing/skills/, .cursor/skills/, or ~/.cursor/skills/, or enable built-in skills for this mode.",
                self.agent_type
            ));
            return;
        }

        let skill_items: Vec<SkillItem> = skills.into_iter().map(Self::skill_item_from_info).collect();

        if skill_items.is_empty() {
            chat_state.add_system_message("No skills found.".to_string());
            return;
        }

        chat_view.show_skill_list(skill_items);
    }

    pub(crate) fn show_skill_config_selector(
        &self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let skills = tokio::task::block_in_place(|| {
            let workspace = self.agent.workspace_path_buf();
            let agent_type = self.agent_type.clone();
            rt_handle.block_on(async {
                let registry = SkillRegistry::global();
                registry
                    .get_mode_skill_infos_for_workspace(Some(workspace.as_path()), &agent_type)
                    .await
            })
        });

        let skill_items: Vec<SkillItem> = skills.into_iter().map(Self::skill_item_from_mode_info).collect();

        if skill_items.is_empty() {
            chat_state.add_system_message("No skills found.".to_string());
            return;
        }

        chat_view.show_skill_config(skill_items);
    }

    pub(crate) fn handle_skill_selector_action(
        &self,
        action: SkillSelectorAction,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        match action {
            SkillSelectorAction::ListSkills => {
                self.show_available_skill_list(chat_view, chat_state, rt_handle);
            }
            SkillSelectorAction::ConfigureSkills => {
                self.show_skill_config_selector(chat_view, chat_state, rt_handle);
            }
            SkillSelectorAction::Execute(selected) => {
                chat_view.hide_skill_selector();
                self.apply_skill_selection(&selected, chat_view);
            }
            SkillSelectorAction::Toggle(selected) => {
                self.set_skill_enabled(&selected, !selected.enabled, chat_state, rt_handle);
                self.show_skill_config_selector(chat_view, chat_state, rt_handle);
            }
        }
    }

    /// Apply skill selection: fill input box with execution command
    pub(crate) fn apply_skill_selection(&self, selected: &SkillItem, chat_view: &mut ChatView) {
        chat_view.set_input(&format!("Execute the {} skill.", selected.name));
    }

    pub(crate) fn set_skill_enabled(
        &self,
        selected: &SkillItem,
        enabled: bool,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let workspace = self.agent.workspace_path_buf();
        let mode_id = self.agent_type.clone();
        let skill = selected.clone();

        let result: Result<(), String> = tokio::task::block_in_place(|| {
            rt_handle.block_on(async {
                match skill.level.as_str() {
                    "user" => {
                        set_user_mode_skill_state(&mode_id, &skill.key, enabled, skill.default_enabled)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    "project" => {
                        let mut document = load_project_mode_skills_document_local(&workspace)
                            .await
                            .map_err(|error| error.to_string())?;
                        set_mode_skill_disabled_in_document(&mut document, &mode_id, &skill.key, !enabled)
                            .map_err(|error| error.to_string())?;
                        save_project_mode_skills_document_local(&workspace, &document)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    other => {
                        return Err(format!("Unsupported skill level '{}'", other));
                    }
                }

                Ok(())
            })
        });

        match result {
            Ok(()) => chat_state.add_system_message(format!(
                "Skill '{}' {} for mode '{}'.",
                selected.name,
                if enabled { "enabled" } else { "disabled" },
                self.agent_type
            )),
            Err(error) => {
                chat_state.add_system_message(format!("Failed to update skill '{}': {}", selected.name, error))
            }
        }
    }

    pub(crate) fn skill_item_from_info(info: SkillInfo) -> SkillItem {
        SkillItem {
            key: info.key,
            name: info.name,
            description: info.description,
            level: info.level.as_str().to_string(),
            enabled: true,
            selected_for_runtime: true,
            default_enabled: true,
            is_shadowed: info.is_shadowed,
        }
    }

    pub(crate) fn skill_item_from_mode_info(info: ModeSkillInfo) -> SkillItem {
        SkillItem {
            key: info.skill.key,
            name: info.skill.name,
            description: info.skill.description,
            level: info.skill.level.as_str().to_string(),
            enabled: info.effective_enabled,
            selected_for_runtime: info.selected_for_runtime,
            default_enabled: info.default_enabled,
            is_shadowed: info.skill.is_shadowed,
        }
    }
}
