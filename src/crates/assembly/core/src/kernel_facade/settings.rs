//! KernelSettingsApi implementation.

use std::time::Duration;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::settings::{
    AIModelConfigDto, ConfigLocationDto, GlobalConfigDto, GlobalConfigPatchDto, MCPServerDto,
    MCPServerStatusDto, ProviderConfigDto, ProviderTestResultDto,
};

use crate::service::config::{get_global_config_service, GlobalConfig};
use crate::service::mcp::global_mcp_service;

#[async_trait]
impl northhing_kernel_api::KernelSettingsApi for super::KernelFacade {
    async fn get_global_config(&self) -> Result<GlobalConfigDto, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let config: GlobalConfig = cfg_svc
            .config(None)
            .await
            .map_err(|e| KernelError::Config(format!("get global config: {e}")))?;
        Ok(GlobalConfigDto {
            providers: models
                .iter()
                .map(|m| ProviderConfigDto {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    base_url: m.base_url.clone(),
                    api_key: m.api_key.clone(),
                    model: m.model_name.clone(),
                    extra: None,
                })
                .collect(),
            default_provider_id: config.ai.default_models.primary.clone(),
            workspace_config: None,
        })
    }

    async fn update_global_config(&self, patch: GlobalConfigPatchDto) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        if let Some(providers) = patch.providers {
            for p in providers {
                let model_cfg = crate::service::config::runtime::AIModelConfig {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    provider: p.id.clone(),
                    model_name: p.model.clone(),
                    base_url: p.base_url.clone(),
                    request_url: None,
                    api_key: p.api_key.clone(),
                    context_window: None,
                    max_tokens: None,
                    temperature: None,
                    top_p: None,
                    enabled: true,
                    category: Default::default(),
                    capabilities: vec![],
                    recommended_for: vec![],
                    metadata: None,
                    enable_thinking_process: false,
                    reasoning_mode: None,
                    inline_think_in_text: false,
                    custom_headers: None,
                    custom_headers_mode: None,
                    skip_ssl_verify: false,
                    reasoning_effort: None,
                    thinking_budget_tokens: None,
                    custom_request_body: None,
                    custom_request_body_mode: None,
                    auth: Default::default(),
                };
                let existing = cfg_svc
                    .get_ai_models()
                    .await
                    .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?
                    .iter()
                    .any(|m| m.id == p.id);
                if existing {
                    cfg_svc
                        .update_ai_model(&p.id, model_cfg)
                        .await
                        .map_err(|e| KernelError::Config(format!("update_ai_model: {e}")))?;
                } else {
                    cfg_svc
                        .add_ai_model(model_cfg)
                        .await
                        .map_err(|e| KernelError::Config(format!("add_ai_model: {e}")))?;
                }
            }
        }
        if let Some(default_id) = patch.default_provider_id {
            cfg_svc
                .set_config("ai.default_models.primary", default_id.as_str())
                .await
                .map_err(|e| KernelError::Config(format!("set default provider: {e}")))?;
        }
        Ok(())
    }

    async fn list_model_configs(&self) -> Result<Vec<AIModelConfigDto>, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        Ok(models
            .into_iter()
            .map(|m| AIModelConfigDto {
                id: m.id,
                provider_id: m.provider,
                model: m.model_name,
                display_name: Some(m.name),
                max_tokens: m.max_tokens,
                temperature: m.temperature,
            })
            .collect())
    }

    async fn upsert_model_config(&self, config: AIModelConfigDto) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let existing_models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let existing = existing_models.iter().find(|m| m.id == config.id);
        let model_cfg = if let Some(existing_model) = existing {
            crate::service::config::runtime::AIModelConfig {
                id: config.id.clone(),
                name: config.display_name.unwrap_or_else(|| existing_model.name.clone()),
                provider: config.provider_id.clone(),
                model_name: config.model.clone(),
                base_url: existing_model.base_url.clone(),
                request_url: existing_model.request_url.clone(),
                api_key: existing_model.api_key.clone(),
                context_window: existing_model.context_window,
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                top_p: existing_model.top_p,
                enabled: existing_model.enabled,
                category: existing_model.category.clone(),
                capabilities: existing_model.capabilities.clone(),
                recommended_for: existing_model.recommended_for.clone(),
                metadata: existing_model.metadata.clone(),
                enable_thinking_process: existing_model.enable_thinking_process,
                reasoning_mode: existing_model.reasoning_mode.clone(),
                inline_think_in_text: existing_model.inline_think_in_text,
                custom_headers: existing_model.custom_headers.clone(),
                custom_headers_mode: existing_model.custom_headers_mode.clone(),
                skip_ssl_verify: existing_model.skip_ssl_verify,
                reasoning_effort: existing_model.reasoning_effort.clone(),
                thinking_budget_tokens: existing_model.thinking_budget_tokens,
                custom_request_body: existing_model.custom_request_body.clone(),
                custom_request_body_mode: existing_model.custom_request_body_mode.clone(),
                auth: existing_model.auth.clone(),
            }
        } else {
            crate::service::config::runtime::AIModelConfig {
                id: config.id.clone(),
                name: config.display_name.unwrap_or_default(),
                provider: config.provider_id.clone(),
                model_name: config.model.clone(),
                base_url: String::new(),
                request_url: None,
                api_key: String::new(),
                context_window: None,
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                top_p: None,
                enabled: true,
                category: Default::default(),
                capabilities: vec![],
                recommended_for: vec![],
                metadata: None,
                enable_thinking_process: false,
                reasoning_mode: None,
                inline_think_in_text: false,
                custom_headers: None,
                custom_headers_mode: None,
                skip_ssl_verify: false,
                reasoning_effort: None,
                thinking_budget_tokens: None,
                custom_request_body: None,
                custom_request_body_mode: None,
                auth: Default::default(),
            }
        };
        if existing.is_some() {
            cfg_svc
                .update_ai_model(&config.id, model_cfg)
                .await
                .map_err(|e| KernelError::Config(format!("update_ai_model: {e}")))?;
        } else {
            cfg_svc
                .add_ai_model(model_cfg)
                .await
                .map_err(|e| KernelError::Config(format!("add_ai_model: {e}")))?;
        }
        Ok(())
    }

    async fn delete_model_config(&self, id: &str) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        cfg_svc
            .delete_ai_model(id)
            .await
            .map_err(|e| KernelError::Config(format!("delete_model_config: {e}")))
    }

    async fn set_default_provider(&self, id: &str) -> Result<(), KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        cfg_svc
            .set_config("ai.default_models.primary", id)
            .await
            .map_err(|e| KernelError::Config(format!("set_default_provider: {e}")))
    }

    async fn list_mcp_servers(&self) -> Result<Vec<MCPServerDto>, KernelError> {
        let mcp_svc = global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        let configs = mcp_svc
            .config_service()
            .load_all_configs()
            .await
            .map_err(|e| KernelError::Runtime(format!("list_mcp_servers: {e}")))?;
        Ok(configs
            .into_iter()
            .map(|c| MCPServerDto {
                id: c.id.clone(),
                name: c.name.clone(),
                config: northhing_kernel_api::settings::MCPServerConfigDto {
                    command: c.command.unwrap_or_default(),
                    args: c.args.clone(),
                    env: Some(c.env),
                },
                location: match c.location {
                    crate::service::mcp::config::ConfigLocation::User => ConfigLocationDto::User,
                    crate::service::mcp::config::ConfigLocation::Project => ConfigLocationDto::Project,
                    crate::service::mcp::config::ConfigLocation::BuiltIn => ConfigLocationDto::BuiltIn,
                },
            })
            .collect())
    }

    async fn upsert_mcp_server(&self, config: MCPServerDto) -> Result<(), KernelError> {
        let mcp_svc = global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        let location = match config.location {
            northhing_kernel_api::settings::ConfigLocationDto::User => {
                northhing_services_integrations::mcp::config::ConfigLocation::User
            }
            northhing_kernel_api::settings::ConfigLocationDto::Project => {
                northhing_services_integrations::mcp::config::ConfigLocation::Project
            }
            northhing_kernel_api::settings::ConfigLocationDto::BuiltIn => {
                northhing_services_integrations::mcp::config::ConfigLocation::BuiltIn
            }
        };
        let server_type = northhing_services_integrations::mcp::server::MCPServerType::Local;
        let mcp_config = crate::service::mcp::MCPServerConfig {
            id: config.id.clone(),
            name: config.name.clone(),
            server_type,
            transport: None,
            command: Some(config.config.command),
            args: config.config.args,
            env: config.config.env.unwrap_or_default(),
            headers: Default::default(),
            url: None,
            auto_start: true,
            enabled: true,
            location,
            capabilities: vec![],
            settings: Default::default(),
            oauth: None,
            xaa: None,
        };
        mcp_svc
            .config_service()
            .save_server_config(&mcp_config)
            .await
            .map_err(|e| KernelError::Config(format!("save_server_config: {e}")))
    }

    async fn delete_mcp_server(&self, id: &str) -> Result<(), KernelError> {
        let mcp_svc = global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        mcp_svc
            .config_service()
            .delete_server_config(id)
            .await
            .map_err(|e| KernelError::Config(format!("delete_mcp_server: {e}")))
    }

    async fn get_mcp_status(&self, id: &str) -> Result<MCPServerStatusDto, KernelError> {
        let mcp_svc = global_mcp_service()
            .ok_or_else(|| KernelError::Internal("MCP service not initialized".to_string()))?;
        let status = tokio::time::timeout(
            Duration::from_millis(30),
            mcp_svc.server_manager().get_server_status(id),
        )
        .await
        .map_err(|_| KernelError::Timeout)?
        .map_err(|e| KernelError::Runtime(format!("get_mcp_status: {e}")))?;
        Ok(MCPServerStatusDto {
            id: id.to_string(),
            status: crate::kernel_facade::helpers::map_mcp_status_kind(status),
        })
    }

    async fn test_provider(&self, id: &str) -> Result<ProviderTestResultDto, KernelError> {
        let cfg_svc = get_global_config_service()
            .await
            .map_err(|e| KernelError::Config(format!("get_global_config_service: {e}")))?;
        let models = cfg_svc
            .get_ai_models()
            .await
            .map_err(|e| KernelError::Config(format!("get_ai_models: {e}")))?;
        let model = models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| KernelError::NotFound(format!("provider not found: {id}")))?;
        let ai_config = crate::util::types::AIConfig::try_from(model.clone())
            .map_err(|e| KernelError::Validation(format!("invalid config: {e}")))?;
        let client = crate::infrastructure::ai::AIClient::new(ai_config);
        match client.test_connection().await {
            Ok(result) => Ok(ProviderTestResultDto {
                success: result.success,
                error: result.error_details.map(|d| crate::kernel_facade::helpers::first_line_error(&d)),
            }),
            Err(e) => Ok(ProviderTestResultDto {
                success: false,
                error: Some(crate::kernel_facade::helpers::first_line_error(&e.to_string())),
            }),
        }
    }

    async fn test_provider_config(
        &self,
        form: northhing_kernel_api::settings::ProviderFormDto,
    ) -> Result<ProviderTestResultDto, KernelError> {
        use crate::service::config::runtime::AIModelConfig;
        let model_cfg = AIModelConfig {
            id: form.provider_id.clone(),
            name: form.provider_id.clone(),
            provider: form.provider_id.clone(),
            model_name: form.model.clone().unwrap_or_default(),
            base_url: form.base_url.clone().unwrap_or_default(),
            request_url: None,
            api_key: form.api_key.clone().unwrap_or_default(),
            context_window: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            enabled: true,
            category: Default::default(),
            capabilities: vec![],
            recommended_for: vec![],
            metadata: None,
            enable_thinking_process: false,
            reasoning_mode: None,
            inline_think_in_text: false,
            custom_headers: None,
            custom_headers_mode: None,
            skip_ssl_verify: false,
            reasoning_effort: None,
            thinking_budget_tokens: None,
            custom_request_body: None,
            custom_request_body_mode: None,
            auth: Default::default(),
        };
        let ai_config = crate::util::types::AIConfig::try_from(model_cfg)
            .map_err(|e| KernelError::Validation(format!("invalid config: {e}")))?;
        let client = crate::infrastructure::ai::AIClient::new(ai_config);
        match client.test_connection().await {
            Ok(result) => Ok(ProviderTestResultDto {
                success: result.success,
                error: result.error_details.map(|d| crate::kernel_facade::helpers::first_line_error(&d)),
            }),
            Err(e) => Ok(ProviderTestResultDto {
                success: false,
                error: Some(crate::kernel_facade::helpers::first_line_error(&e.to_string())),
            }),
        }
    }
}
