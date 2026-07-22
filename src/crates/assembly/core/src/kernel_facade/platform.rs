//! KernelPlatformApi implementation.

use std::time::Duration;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::platform::{
    AnalysisDto, ArtifactDto, CoreHealthDto, ImageContextDto, InspectorDataDto, PanelDto,
    PanelsConfigDto, SkillStatusDto, TerminalConfigDto,
};

use crate::kernel_facade::lifecycle::FACADE_READY;
use crate::service::config::get_global_config_service;
use crate::service::mcp::global_mcp_service;

#[async_trait]
impl northhing_kernel_api::KernelPlatformApi for super::KernelFacade {
    async fn open_terminal(&self, _config: TerminalConfigDto) -> Result<(), KernelError> {
        Err(KernelError::Internal("not yet wired: open_terminal".to_string()))
    }

    async fn analyze_image(&self, _context: ImageContextDto) -> Result<AnalysisDto, KernelError> {
        Err(KernelError::Internal("not yet wired: analyze_image".to_string()))
    }

    async fn get_core_health(&self) -> Result<CoreHealthDto, KernelError> {
        Ok(CoreHealthDto {
            healthy: FACADE_READY.load(std::sync::atomic::Ordering::SeqCst),
            details: if FACADE_READY.load(std::sync::atomic::Ordering::SeqCst) {
                vec!["core initialized".to_string()]
            } else {
                vec!["core not yet initialized".to_string()]
            },
        })
    }

    async fn read_panels_config(&self) -> Result<PanelsConfigDto, KernelError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| KernelError::Config("cannot find config directory".to_string()))?;
        let panels_path = config_dir.join("northhing").join("config").join("panels.json");
        if !panels_path.exists() {
            return Ok(PanelsConfigDto { panels: vec![] });
        }
        let content = tokio::fs::read_to_string(&panels_path)
            .await
            .map_err(|e| KernelError::Runtime(format!("read panels.json: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| KernelError::Runtime(format!("parse panels.json: {e}")))
    }

    async fn is_onboarding_complete(&self) -> Result<bool, KernelError> {
        Err(KernelError::Internal(
            "not yet wired: is_onboarding_complete".to_string(),
        ))
    }

    async fn complete_onboarding(&self) -> Result<(), KernelError> {
        Err(KernelError::Internal(
            "not yet wired: complete_onboarding".to_string(),
        ))
    }

    async fn get_inspector_data(&self) -> Result<InspectorDataDto, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let config: crate::service::config::GlobalConfig = cfg_svc
            .config(None)
            .await
            .map_err(|e| KernelError::Config(format!("get global config: {e}")))?;
        let model_name = config
            .ai
            .default_models
            .primary
            .clone()
            .unwrap_or_else(|| "not configured".to_string());

        let mcp_status = if let Some(mcp_svc) = global_mcp_service() {
            match mcp_svc.config_service().load_all_configs().await {
                Ok(configs) => {
                    let mut statuses = Vec::new();
                    for config in configs {
                        let probe_status = tokio::time::timeout(
                            Duration::from_millis(30),
                            mcp_svc.server_manager().get_server_status(&config.id),
                        )
                        .await;
                        let kind = crate::kernel_facade::helpers::map_mcp_probe_status(probe_status);
                        statuses.push(northhing_kernel_api::settings::MCPServerStatusDto {
                            id: config.id,
                            status: kind,
                        });
                    }
                    statuses
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        let skills_status = {
            use crate::agentic::tools::implementations::skills::skill_registry;
            let registry = skill_registry();
            let skills = registry.get_all_skills().await;
            skills
                .into_iter()
                .map(|s| SkillStatusDto {
                    skill_id: s.key,
                    name: s.name,
                    enabled: !s.is_shadowed,
                    status: if s.is_shadowed {
                        "shadowed".to_string()
                    } else {
                        "available".to_string()
                    },
                })
                .collect()
        };

        Ok(InspectorDataDto {
            model_name,
            mcp_status,
            skills_status,
        })
    }

    async fn list_artifacts(
        &self,
        _session_id: &super::SessionId,
    ) -> Result<Vec<ArtifactDto>, KernelError> {
        Err(KernelError::Internal("not yet wired: list_artifacts".to_string()))
    }
}
