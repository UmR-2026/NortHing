use super::manager::{ConfigManager, ConfigManagerSettings};
use super::service::ConfigService;
use crate::util::errors::NortHingResult;

/// Config factory for creating configuration-related components.
pub struct ConfigFactory;

impl ConfigFactory {
    /// Creates the default configuration service.
    pub async fn create_default_service() -> NortHingResult<ConfigService> {
        ConfigService::new().await
    }

    /// Creates a configuration service with custom settings.
    pub async fn create_service_with_settings(settings: ConfigManagerSettings) -> NortHingResult<ConfigService> {
        ConfigService::with_settings(settings).await
    }

    /// Creates a configuration manager.
    pub async fn create_manager(settings: ConfigManagerSettings) -> NortHingResult<ConfigManager> {
        ConfigManager::new(settings).await
    }
}
