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

    async fn list_subagents(
        &self,
        scope: SubagentScopeDto,
    ) -> Result<Vec<SubagentDto>, KernelError> {
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
        _id: &str,
        _scope: northhing_kernel_api::agents::SkillScopeDto,
        _enabled: bool,
    ) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: mode_id required but not present in SkillScopeDto.
        Err(KernelError::Internal("not yet wired: set_skill_enabled — mode_id not in scope".to_string()))
    }

    async fn load_skill_overrides(&self) -> Result<SkillOverridesDto, KernelError> {
        // NEEDS_CONTEXT: mode_id required but not present in trait signature.
        Err(KernelError::Internal("not yet wired: load_skill_overrides — mode_id not available".to_string()))
    }

    async fn load_project_skills(&self) -> Result<northhing_kernel_api::agents::ProjectSkillsDto, KernelError> {
        // NEEDS_CONTEXT: workspace_path required but not present in trait signature.
        Err(KernelError::Internal("not yet wired: load_project_skills — workspace_path not available".to_string()))
    }

    async fn save_project_skills(
        &self,
        doc: northhing_kernel_api::agents::ProjectSkillsDto,
    ) -> Result<(), KernelError> {
        use crate::agentic::tools::implementations::skills::mode_overrides::{
            load_project_mode_skills_document_local, save_project_mode_skills_document_local,
            set_disabled_mode_skills_in_document,
        };
        use crate::service::config::agent_profile_project_store::ProjectAgentProfilesDocument;

        let workspace_root = std::path::Path::new(&doc.workspace_path);
        let mut document = load_project_mode_skills_document_local(workspace_root)
            .await
            .map_err(|e| KernelError::Config(format!("load_project_mode_skills_document_local: {e}")))?;

        for skill_entry in &doc.skills {
            // mode_id is not in ProjectSkillEntry; use default profile.
            // NEEDS_CONTEXT: proper implementation requires mode_id per skill.
            let _ = set_disabled_mode_skills_in_document(
                &mut document,
                "default",
                vec![skill_entry.skill_id.clone()],
            );
        }

        save_project_mode_skills_document_local(workspace_root, &document)
            .await
            .map_err(|e| KernelError::Config(format!("save_project_mode_skills_document_local: {e}")))
    }

    async fn resolve_skill_default_enabled(
        &self,
        skill_id: &str,
        mode: &str,
    ) -> Result<bool, KernelError> {
        use crate::agentic::tools::implementations::skills::resolver::resolve_skill_default_enabled_for_mode;
        use crate::agentic::tools::implementations::skills::skill_registry;
        let registry = skill_registry();
        let skills = registry.get_all_skills().await;
        match skills.into_iter().find(|s| s.key == skill_id) {
            Some(skill) => Ok(resolve_skill_default_enabled_for_mode(&skill, mode)),
            None => Err(KernelError::NotFound(format!(
                "skill not found: {skill_id}"
            ))),
        }
    }
}
