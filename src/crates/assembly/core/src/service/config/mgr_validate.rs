use super::manager::{canonical_config_path, ConfigManager};
use crate::service::config::types::*;
use crate::util::errors::*;
use tracing::{debug, info};

impl ConfigManager {
    /// Gets a configuration value (supports dot-paths).
    pub fn get<T>(&self, path: &str) -> NortHingResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = canonical_config_path(path);
        let value = self.get_value_by_path(path)?;
        serde_json::from_value(value)
            .map_err(|e| NortHingError::config(format!("Failed to deserialize config value at '{}': {}", path, e)))
    }

    /// Sets a configuration value (supports dot-paths).
    pub async fn set<T>(&mut self, path: &str, value: T) -> NortHingResult<()>
    where
        T: serde::Serialize,
    {
        let old_config = self.config.clone();
        let json_value = serde_json::to_value(value)
            .map_err(|e| NortHingError::config(format!("Failed to serialize config value: {}", e)))?;

        let path = canonical_config_path(path);
        self.set_value_by_path(path, json_value)?;
        self.config.last_modified = chrono::Utc::now();

        if let Err(e) = self.validate_config().await {
            self.config = old_config;
            return Err(e);
        }

        self.notify_config_changed(path, &old_config).await?;

        self.save_config().await?;

        Ok(())
    }

    /// Resets configuration (supports dot-paths).
    pub async fn reset(&mut self, path: Option<&str>) -> NortHingResult<()> {
        let old_config = self.config.clone();

        if let Some(path) = path {
            let default_config = self.providers.default_config();
            let default_value = self.get_value_by_path_from_config(&default_config, path)?;
            self.set_value_by_path(path, default_value)?;
        } else {
            self.config = self.providers.default_config();
        }

        self.config.last_modified = chrono::Utc::now();

        if let Some(path) = path {
            self.notify_config_changed(path, &old_config).await?;
        } else {
            for provider_name in self.providers.provider_names() {
                self.notify_config_changed(&provider_name, &old_config).await?;
            }
        }

        self.save_config().await?;

        Ok(())
    }

    /// Validates configuration.
    pub async fn validate_config(&self) -> NortHingResult<ConfigValidationResult> {
        self.providers.validate_config(&self.config).await
    }

    /// Exports configuration.
    pub fn export_config(&self) -> NortHingResult<serde_json::Value> {
        serde_json::to_value(&self.config).map_err(|e| NortHingError::config(format!("Failed to export config: {}", e)))
    }

    /// Imports configuration.
    pub async fn import_config(&mut self, config_data: serde_json::Value) -> NortHingResult<()> {
        let old_config = self.config.clone();

        let imported_config: GlobalConfig = serde_json::from_value(config_data)
            .map_err(|e| NortHingError::config(format!("Failed to parse imported config: {}", e)))?;

        let validation_result = self.providers.validate_config(&imported_config).await?;
        if !validation_result.valid {
            let error_messages: Vec<String> = validation_result.errors.iter().map(|e| e.message.clone()).collect();
            return Err(NortHingError::validation(format!(
                "Invalid imported config: {}",
                error_messages.join(", ")
            )));
        }

        self.config = imported_config;
        self.config.last_modified = chrono::Utc::now();

        for provider_name in self.providers.provider_names() {
            self.notify_config_changed(&provider_name, &old_config).await?;
        }

        self.save_config().await?;

        info!("Successfully imported configuration");
        Ok(())
    }

    /// Gets a configuration value by dot-path.
    fn get_value_by_path(&self, path: &str) -> NortHingResult<serde_json::Value> {
        self.get_value_by_path_from_config(&self.config, path)
    }

    /// Gets a configuration value by dot-path from the given config.
    fn get_value_by_path_from_config(&self, config: &GlobalConfig, path: &str) -> NortHingResult<serde_json::Value> {
        let config_value = serde_json::to_value(config)
            .map_err(|e| NortHingError::config(format!("Failed to serialize config: {}", e)))?;

        let keys: Vec<&str> = path.split('.').collect();
        let mut current = &config_value;

        for key in keys {
            current = current
                .get(key)
                .ok_or_else(|| NortHingError::NotFound(format!("Config path '{}' not found", path)))?;
        }

        Ok(current.clone())
    }

    /// Sets a configuration value by dot-path.
    fn set_value_by_path(&mut self, path: &str, value: serde_json::Value) -> NortHingResult<()> {
        if path.is_empty() {
            self.config = serde_json::from_value(value)
                .map_err(|e| NortHingError::config(format!("Failed to deserialize config: {}", e)))?;
            return Ok(());
        }

        let mut config_value = serde_json::to_value(&self.config)
            .map_err(|e| NortHingError::config(format!("Failed to serialize config: {}", e)))?;

        let keys: Vec<&str> = path.split('.').filter(|k| !k.is_empty()).collect();
        if keys.is_empty() {
            self.config = serde_json::from_value(value)
                .map_err(|e| NortHingError::config(format!("Failed to deserialize config: {}", e)))?;
            return Ok(());
        }

        let last_key = keys
            .last()
            .ok_or_else(|| NortHingError::config(format!("Config path '{}' does not contain any keys", path)))?;
        let parent_keys = &keys[..keys.len() - 1];

        let mut current = &mut config_value;
        for key in parent_keys {
            current = current
                .get_mut(key)
                .ok_or_else(|| NortHingError::NotFound(format!("Config path '{}' not found", path)))?;
        }

        if let Some(obj) = current.as_object_mut() {
            obj.insert(last_key.to_string(), value);
        } else {
            return Err(NortHingError::config(format!(
                "Cannot set value at path '{}': parent is not an object",
                path
            )));
        }

        self.config = serde_json::from_value(config_value)
            .map_err(|e| NortHingError::config(format!("Failed to deserialize updated config: {}", e)))?;

        Ok(())
    }

    /// Notifies about a configuration change.
    async fn notify_config_changed(&self, path: &str, old_config: &GlobalConfig) -> NortHingResult<()> {
        self.check_and_broadcast_app_change(path).await;
        self.check_and_broadcast_debug_mode_change(old_config).await;
        self.check_and_broadcast_log_level_change(old_config).await;
        self.check_and_broadcast_sensitive_diagnostics_change(old_config).await;

        self.providers
            .notify_config_changed(path, old_config, &self.config)
            .await
    }

    /// Detects and broadcasts app-scope configuration changes.
    async fn check_and_broadcast_app_change(&self, path: &str) {
        if path == "app" || path.starts_with("app.") {
            use super::global::{ConfigUpdateEvent, GlobalConfigManager};
            GlobalConfigManager::broadcast_update(ConfigUpdateEvent::AppUpdated).await;
        }
    }

    /// Detects and broadcasts debug-mode configuration changes.
    async fn check_and_broadcast_debug_mode_change(&self, old_config: &GlobalConfig) {
        let old_debug = &old_config.ai.debug_mode_config;
        let new_debug = &self.config.ai.debug_mode_config;

        if old_debug.ingest_port != new_debug.ingest_port || old_debug.log_path != new_debug.log_path {
            debug!(
                "Debug Mode config change detected: port {} -> {}, log_path {} -> {}",
                old_debug.ingest_port, new_debug.ingest_port, old_debug.log_path, new_debug.log_path
            );

            use super::global::{ConfigUpdateEvent, GlobalConfigManager};
            GlobalConfigManager::broadcast_update(ConfigUpdateEvent::DebugModeConfigUpdated {
                new_port: new_debug.ingest_port,
                new_log_path: new_debug.log_path.clone(),
            })
            .await;
        }
    }

    /// Detects and broadcasts runtime log-level changes.
    async fn check_and_broadcast_log_level_change(&self, old_config: &GlobalConfig) {
        let old_level = old_config.app.logging.level.trim().to_lowercase();
        let new_level = self.config.app.logging.level.trim().to_lowercase();

        if old_level != new_level {
            debug!("App logging level change detected: {} -> {}", old_level, new_level);

            use super::global::{ConfigUpdateEvent, GlobalConfigManager};
            GlobalConfigManager::broadcast_update(ConfigUpdateEvent::LogLevelUpdated { new_level }).await;
        }
    }

    /// Detects and broadcasts runtime sensitive diagnostics changes.
    async fn check_and_broadcast_sensitive_diagnostics_change(&self, old_config: &GlobalConfig) {
        let old_include = old_config.app.logging.include_sensitive_diagnostics;
        let new_include = self.config.app.logging.include_sensitive_diagnostics;

        if old_include != new_include {
            debug!(
                "App logging sensitive diagnostics preference changed: {} -> {}",
                old_include, new_include
            );

            #[cfg(feature = "ai-adapter-runtime")]
            {
                northhing_ai_adapters::diagnostics::set_include_sensitive_diagnostics(new_include);
            }

            use super::global::{ConfigUpdateEvent, GlobalConfigManager};
            GlobalConfigManager::broadcast_update(ConfigUpdateEvent::LoggingSensitiveDiagnosticsUpdated {
                include_sensitive_diagnostics: new_include,
            })
            .await;
        }
    }
}
