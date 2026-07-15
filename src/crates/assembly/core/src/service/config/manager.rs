//! Configuration manager implementation
//!
//! A complete configuration management system based on the Provider mechanism.

use super::providers::ConfigProviderRegistry;
use super::types::*;
use crate::infrastructure::{try_get_path_manager_arc, PathManager};
use crate::util::errors::*;
use tracing::debug;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) type ConfigMigrationFn = fn(Value) -> NortHingResult<Value>;
pub(super) type ConfigMigration = (&'static str, &'static str, ConfigMigrationFn);

pub(super) fn canonical_config_path(path: &str) -> &str {
    match path {
        "ai.review_teams.rate_limit_status" => "ai.review_team_rate_limit_status",
        "ai.review_teams.project_strategy_overrides" => "ai.review_team_project_strategy_overrides",
        _ => path,
    }
}

/// Configuration manager.
pub struct ConfigManager {
    pub(super) config_dir: PathBuf,
    pub(super) config: GlobalConfig,
    pub(super) providers: ConfigProviderRegistry,
    pub(super) config_file: PathBuf,
    pub(super) path_manager: Arc<PathManager>,
}

/// Configuration manager settings.
#[derive(Debug, Clone)]
pub struct ConfigManagerSettings {
    pub path_manager: Option<Arc<PathManager>>,
    pub auto_save: bool,
    pub backup_count: usize,
}

impl Default for ConfigManagerSettings {
    fn default() -> Self {
        Self {
            path_manager: None,
            auto_save: true,
            backup_count: 5,
        }
    }
}

/// Configuration statistics.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigStatistics {
    pub total_ai_models: usize,
    pub has_default_model: bool,
    pub config_directory: PathBuf,
    pub providers_count: usize,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

impl ConfigManager {
    /// Creates a new unified configuration manager.
    pub async fn new(settings: ConfigManagerSettings) -> NortHingResult<Self> {
        let path_manager = match settings.path_manager {
            Some(path_manager) => path_manager,
            None => try_get_path_manager_arc()?,
        };

        path_manager.initialize_user_directories().await?;

        let config_dir = path_manager.user_config_dir();
        let config_file = path_manager.app_config_file();

        let providers = ConfigProviderRegistry::new();

        let mut manager = Self {
            config_dir,
            config: GlobalConfig::default(),
            providers,
            config_file,
            path_manager,
        };

        manager.load_or_create_config().await?;
        #[cfg(feature = "ai-adapter-runtime")]
        {
            northhing_ai_adapters::diagnostics::set_include_sensitive_diagnostics(
                manager.config.app.logging.include_sensitive_diagnostics,
            );
        }

        debug!("ConfigManager initialized at {:?}", manager.config_file);
        Ok(manager)
    }

    /// Returns the path manager.
    pub fn path_manager(&self) -> &Arc<PathManager> {
        &self.path_manager
    }

    /// Returns the full configuration.
    pub fn config(&self) -> &GlobalConfig {
        &self.config
    }

    /// Returns configuration statistics.
    pub fn statistics(&self) -> ConfigStatistics {
        ConfigStatistics {
            total_ai_models: self.config.ai.models.len(),
            has_default_model: self.config.ai.default_models.primary.is_some(),
            config_directory: self.config_dir.clone(),
            providers_count: self.providers.provider_names().len(),
            last_modified: self.config.last_modified,
        }
    }

    /// Registers a configuration provider.
    pub fn register_provider(&mut self, provider: Box<dyn ConfigProvider>) {
        self.providers.register(provider);
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_config_path;

    #[test]
    fn canonicalizes_legacy_review_team_auxiliary_paths() {
        assert_eq!(
            canonical_config_path("ai.review_teams.rate_limit_status"),
            "ai.review_team_rate_limit_status"
        );
        assert_eq!(
            canonical_config_path("ai.review_teams.project_strategy_overrides"),
            "ai.review_team_project_strategy_overrides"
        );
        assert_eq!(
            canonical_config_path("ai.review_teams.default"),
            "ai.review_teams.default"
        );
    }
}

/// Deeply merges JSON values.
///
/// Merges values from `overlay` into `base`:
/// - For objects, recursively merges all key/value pairs
/// - For other types, `overlay` overwrites `base`
/// - Keeps fields that exist in `base` but not in `overlay`
pub(crate) fn deep_merge(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Object(mut base_obj), Value::Object(overlay_obj)) => {
            for (key, overlay_value) in overlay_obj {
                if let Some(base_value) = base_obj.get(&key) {
                    base_obj.insert(key.clone(), deep_merge(base_value.clone(), overlay_value));
                } else {
                    base_obj.insert(key.clone(), overlay_value);
                }
            }
            Value::Object(base_obj)
        }
        (_, overlay) => overlay,
    }
}

/// Returns whether two versions match.
pub(crate) fn versions_match(v1: &str, v2: &str) -> bool {
    v1 == v2
}

/// Returns whether `v1 >= v2`.
pub(crate) fn version_gte(v1: &str, v2: &str) -> bool {
    parse_version(v1) >= parse_version(v2)
}

/// Returns whether `v1 < v2`.
pub(crate) fn version_lt(v1: &str, v2: &str) -> bool {
    parse_version(v1) < parse_version(v2)
}

/// Parses a version string into a tuple `(major, minor, patch)`.
pub(crate) fn parse_version(version: &str) -> (u32, u32, u32) {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// Migration function: `0.0.0 -> 1.0.0`.
///
/// This migration is an example showing how to handle configuration upgrades.
pub(crate) fn migrate_0_0_0_to_1_0_0(mut config: Value) -> NortHingResult<Value> {
    debug!("Executing config migration: 0.0.0 -> 1.0.0");

    if let Some(app) = config.get_mut("app").and_then(|v| v.as_object_mut()) {
        if !app.contains_key("ai_experience") {
            app.insert(
                "ai_experience".to_string(),
                serde_json::json!({
                    "enable_session_title_generation": true,
                    "enable_welcome_panel_ai_analysis": false
                }),
            );
        }
    }

    if let Some(ai) = config.get_mut("ai").and_then(|v| v.as_object_mut()) {
        if !ai.contains_key("super_agent_models") {
            ai.insert("super_agent_models".to_string(), Value::Object(serde_json::Map::new()));
        }
        if !ai.contains_key("sub_agent_models") {
            ai.insert("sub_agent_models".to_string(), serde_json::json!({}));
        }
        if !ai.contains_key("func_agent_models") {
            let func_keys = [
                "compression",
                "startchat-func-agent",
                "session-title-func-agent",
                "git-func-agent",
            ];
            let mut fa = serde_json::Map::new();
            if let Some(am) = ai.get("agent_models").and_then(|v| v.as_object()) {
                for k in func_keys {
                    if let Some(v) = am.get(k) {
                        fa.insert(k.to_string(), v.clone());
                    }
                }
            }
            ai.insert("func_agent_models".to_string(), Value::Object(fa));
        }
    }

    debug!("Migration 0.0.0 -> 1.0.0 completed");
    Ok(config)
}
