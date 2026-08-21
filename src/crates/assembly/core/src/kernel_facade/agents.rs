//! KernelAgentsApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::agents::{AgentInfoDto, SkillInfoDto, SkillOverridesDto, SubagentDto, SubagentScopeDto};
use northhing_kernel_api::error::KernelError;

#[async_trait]
impl northhing_kernel_api::KernelAgentsApi for super::KernelFacade {
    async fn list_agents(&self) -> Result<Vec<AgentInfoDto>, KernelError> {
        let registry = crate::agentic::agents::agent_registry();
        let agents = registry.get_modes_info().await;
        Ok(agents
            .into_iter()
            .map(|a| AgentInfoDto {
                id: a.key.clone(),
                name: a.name.clone(),
                agent_type: a.id.clone(),
                description: Some(a.description),
                capabilities: None,
            })
            .collect())
    }

    async fn list_subagents(&self, scope: SubagentScopeDto) -> Result<Vec<SubagentDto>, KernelError> {
        let registry = crate::agentic::agents::agent_registry();
        // workspace_path not available in SubagentScopeDto; pass None until trait is extended.
        let subagents = registry.get_subagents_info(None).await;
        Ok(subagents
            .into_iter()
            .map(|a| SubagentDto {
                id: a.key.clone(),
                name: a.name.clone(),
                agent_type: a.id.clone(),
                parent_session_id: scope.parent_session_id.clone(),
                status: None,
            })
            .collect())
    }

    async fn list_skills(&self) -> Result<Vec<SkillInfoDto>, KernelError> {
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        Ok(skills
            .into_iter()
            .map(|s| SkillInfoDto {
                id: s.key.clone(),
                name: s.name.clone(),
                description: s.description.clone(),
                enabled: false, // enabled state is mode-dependent; requires mode context
                mode: None,
                tags: None,
            })
            .collect())
    }

    async fn get_skill(&self, id: &str) -> Result<SkillInfoDto, KernelError> {
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        skills
            .into_iter()
            .find(|s| s.key == id)
            .map(|s| SkillInfoDto {
                id: s.key,
                name: s.name,
                description: s.description,
                enabled: false, // enabled state is mode-dependent; requires mode context
                mode: None,
                tags: None,
            })
            .ok_or_else(|| KernelError::NotFound(format!("skill not found: {id}")))
    }

    async fn set_skill_enabled(
        &self,
        id: &str,
        scope: northhing_kernel_api::agents::SkillScopeDto,
        enabled: bool,
    ) -> Result<(), KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::set_user_mode_skill_state;
        use crate::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode;
        use crate::agentic::tools::implementations::skills::skill_registry;

        let mode_id = scope.mode_id.as_deref().unwrap_or("agentic");
        match scope.scope_type.as_str() {
            "user" => {
                let registry = skill_registry();
                let skills = registry.get_all_skills().await;
                let skill = skills
                    .into_iter()
                    .find(|s| s.key == id)
                    .ok_or_else(|| KernelError::NotFound(format!("skill not found: {id}")))?;
                let default_enabled = resolve_skill_default_enabled_for_mode(&skill, mode_id);
                set_user_mode_skill_state(mode_id, id, enabled, default_enabled)
                    .await
                    .map_err(|e| KernelError::Config(format!("set_user_mode_skill_state: {e}")))?;
                Ok(())
            }
            other => Err(KernelError::Validation(format!("unsupported scope type: {other}"))),
        }
    }

    async fn load_skill_overrides(&self) -> Result<SkillOverridesDto, KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::load_user_mode_skill_overrides;
        use northhing_kernel_api::agents::SkillOverrideEntry;

        let overrides = load_user_mode_skill_overrides("agentic")
            .await
            .map_err(|e| KernelError::Config(format!("load_user_mode_skill_overrides: {e}")))?;
        let mut entries = Vec::new();
        for skill_id in &overrides.enabled_skills {
            entries.push(SkillOverrideEntry {
                skill_id: skill_id.clone(),
                key: "user_enabled".to_string(),
                value: serde_json::Value::Bool(true),
            });
        }
        for skill_id in &overrides.disabled_skills {
            entries.push(SkillOverrideEntry {
                skill_id: skill_id.clone(),
                key: "user_enabled".to_string(),
                value: serde_json::Value::Bool(false),
            });
        }
        Ok(SkillOverridesDto { overrides: entries })
    }

    async fn load_project_skills(&self) -> Result<northhing_kernel_api::agents::ProjectSkillsDto, KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::load_project_mode_skills_document_local;
        use northhing_kernel_api::agents::ProjectSkillEntry;

        let ws = crate::service::workspace::global_workspace_service()
            .ok_or_else(|| KernelError::Internal("workspace service not available".to_string()))?;
        let current_ws = ws
            .current_workspace()
            .await
            .ok_or_else(|| KernelError::NotFound("no current workspace".to_string()))?;

        let workspace_path = current_ws.root_path.to_string_lossy().to_string();
        let document = load_project_mode_skills_document_local(&current_ws.root_path)
            .await
            .map_err(|e| KernelError::Config(format!("load_project_mode_skills_document_local: {e}")))?;

        let mut skills = Vec::new();
        for (_profile_id, entry) in &document {
            for skill_id in &entry.skills.disabled_project_skills {
                if !skills.iter().any(|s: &ProjectSkillEntry| &s.skill_id == skill_id) {
                    skills.push(ProjectSkillEntry {
                        skill_id: skill_id.clone(),
                        enabled: false,
                        config: None,
                    });
                }
            }
        }

        Ok(northhing_kernel_api::agents::ProjectSkillsDto { workspace_path, skills })
    }

    async fn save_project_skills(
        &self,
        doc: northhing_kernel_api::agents::ProjectSkillsDto,
    ) -> Result<(), KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::{
            load_project_mode_skills_document_local, save_project_mode_skills_document_local,
            set_disabled_mode_skills_in_document,
        };

        let workspace_root = if !doc.workspace_path.is_empty() {
            std::path::PathBuf::from(&doc.workspace_path)
        } else {
            let ws = crate::service::workspace::global_workspace_service()
                .ok_or_else(|| KernelError::Internal("workspace service not available".to_string()))?;
            let current_ws = ws
                .current_workspace()
                .await
                .ok_or_else(|| KernelError::NotFound("no current workspace".to_string()))?;
            current_ws.root_path
        };

        let mut document = load_project_mode_skills_document_local(&workspace_root)
            .await
            .map_err(|e| KernelError::Config(format!("load_project_mode_skills_document_local: {e}")))?;

        let disabled_skills: Vec<String> = doc
            .skills
            .iter()
            .filter(|s| !s.enabled)
            .map(|s| s.skill_id.clone())
            .collect();

        let _ = set_disabled_mode_skills_in_document(&mut document, "default", disabled_skills);

        save_project_mode_skills_document_local(&workspace_root, &document)
            .await
            .map_err(|e| KernelError::Config(format!("save_project_mode_skills_document_local: {e}")))
    }

    async fn resolve_skill_default_enabled(&self, skill_id: &str, mode: &str) -> Result<bool, KernelError> {
        use crate::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode;
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        match skills.into_iter().find(|s| s.key == skill_id) {
            Some(skill) => Ok(resolve_skill_default_enabled_for_mode(&skill, mode)),
            None => Err(KernelError::NotFound(format!("skill not found: {skill_id}"))),
        }
    }
}
