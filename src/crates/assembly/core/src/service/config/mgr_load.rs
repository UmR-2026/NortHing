use super::manager::ConfigManager;
use crate::service::config::types::{AIModelConfig, GlobalConfig};
use crate::util::errors::*;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

impl ConfigManager {
    /// Loads or creates the configuration file.
    pub(crate) async fn load_or_create_config(&mut self) -> NortHingResult<()> {
        if self.config_file.exists() {
            self.load_existing_config().await?;
        } else {
            self.create_default_config().await?;
        }

        Ok(())
    }

    /// Creates the first config file using the already initialized defaults.
    async fn create_default_config(&mut self) -> NortHingResult<()> {
        Self::add_default_agent_models_config(&mut self.config.ai.agent_models);
        Self::add_default_func_agent_models_config(&mut self.config.ai.func_agent_models);
        self.config.version = env!("CARGO_PKG_VERSION").to_string();
        self.save_config().await?;
        debug!("Created default config file");
        Ok(())
    }

    /// Loads an existing config file and migrates it if needed.
    async fn load_existing_config(&mut self) -> NortHingResult<()> {
        let content = fs::read_to_string(&self.config_file)
            .await
            .map_err(|e| NortHingError::config(format!("Failed to read config file: {}", e)))?;

        let mut config_value: Value = serde_json::from_str(&content)
            .map_err(|e| NortHingError::config(format!("Failed to parse config file as JSON: {}", e)))?;

        let file_version = config_value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let current_version = env!("CARGO_PKG_VERSION").to_string();

        let needs_migration = !super::manager::versions_match(&file_version, &current_version);
        if needs_migration {
            info!(
                "Config version change detected: {} -> {}",
                file_version, current_version
            );
            config_value = self.migrate_config_version(&file_version, config_value).await?;

            if let Some(obj) = config_value.as_object_mut() {
                obj.insert("version".to_string(), Value::String(current_version.clone()));
            }
        }

        match serde_json::from_value::<GlobalConfig>(config_value.clone()) {
            Ok(mut config) => {
                Self::ensure_models_config(&mut config.ai.models);
                Self::add_default_agent_models_config(&mut config.ai.agent_models);
                Self::add_default_func_agent_models_config(&mut config.ai.func_agent_models);

                self.config = config;

                if needs_migration {
                    self.config.version = current_version;
                    self.save_config().await?;
                    info!("Config migrated and saved");
                } else {
                    debug!("Loaded config from file");
                }

                Ok(())
            }
            Err(e) => {
                warn!("Config file deserialization failed, starting smart merge: {}", e);

                self.smart_merge_config_from_value(config_value).await
            }
        }
    }

    /// Performs a smart merge from a JSON value.
    async fn smart_merge_config_from_value(&mut self, user_value: Value) -> NortHingResult<()> {
        let base_config = self.providers.default_config();

        let base_value = serde_json::to_value(&base_config)
            .map_err(|e| NortHingError::config(format!("Failed to serialize default config: {}", e)))?;
        let merged_value = super::manager::deep_merge(base_value, user_value);

        let mut config: GlobalConfig = serde_json::from_value(merged_value)
            .map_err(|e| NortHingError::config(format!("Failed to deserialize merged config: {}", e)))?;

        Self::ensure_models_config(&mut config.ai.models);
        Self::add_default_agent_models_config(&mut config.ai.agent_models);
        Self::add_default_func_agent_models_config(&mut config.ai.func_agent_models);

        self.config = config;

        self.config.version = env!("CARGO_PKG_VERSION").to_string();
        self.save_config().await?;
        info!("Config automatically fixed and saved");

        Ok(())
    }

    /// Auto-completes missing fields in model configuration (backward compatible).
    /// Ensures older configurations won't panic.
    fn ensure_models_config(models: &mut [AIModelConfig]) {
        for model in models.iter_mut() {
            model.ensure_category_and_capabilities();
        }
        debug!("Auto-completed category and capabilities for {} models", models.len());
    }

    /// Adds default configuration for the primary agents (`agent_models`).
    fn add_default_agent_models_config(agent_models: &mut std::collections::HashMap<String, String>) {
        let agents_using_fast = vec!["Explore", "FileFinder", "GenerateDoc", "CodeReview"];
        for key in agents_using_fast {
            if !agent_models.contains_key(key) {
                agent_models.insert(key.to_string(), "fast".to_string());
            }
        }
    }

    /// Adds default configuration for functional agents (`func_agent_models`).
    fn add_default_func_agent_models_config(func_agent_models: &mut std::collections::HashMap<String, String>) {
        let func_agents_using_fast = vec![
            "compression",
            "startchat-func-agent",
            "session-title-func-agent",
            "git-func-agent",
        ];
        for key in func_agents_using_fast {
            if !func_agent_models.contains_key(key) {
                func_agent_models.insert(key.to_string(), "fast".to_string());
            }
        }
    }

    /// Saves the configuration file.
    pub(crate) async fn save_config(&self) -> NortHingResult<()> {
        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| NortHingError::config(format!("Config serialization failed: {}", e)))?;

        if let Some(parent) = self.config_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    NortHingError::config(format!("Failed to create config directory {:?}: {}", parent, e))
                })?;
            }
        }

        fs::write(&self.config_file, content)
            .await
            .map_err(|e| NortHingError::config(format!("Failed to write config file {:?}: {}", self.config_file, e)))?;
        Ok(())
    }

    /// Creates a configuration backup.
    pub async fn create_backup(&self) -> NortHingResult<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = self.config_dir.join("backups");

        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .await
                .map_err(|e| NortHingError::config(format!("Failed to create backup directory: {}", e)))?;
        }

        let backup_file = backup_dir.join(format!("config_backup_{}.json", timestamp));

        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| NortHingError::config(format!("Failed to serialize backup: {}", e)))?;

        fs::write(&backup_file, content)
            .await
            .map_err(|e| NortHingError::config(format!("Failed to write backup: {}", e)))?;

        info!("Created config backup: {:?}", backup_file);
        Ok(backup_file)
    }
}
