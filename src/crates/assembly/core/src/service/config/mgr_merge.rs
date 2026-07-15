use super::manager::{ConfigManager, ConfigMigration};
use crate::util::errors::*;
use serde_json::Value;
use tracing::debug;

impl ConfigManager {
    /// Migrates configuration versions.
    pub(crate) async fn migrate_config_version(&self, from_version: &str, mut config: Value) -> NortHingResult<Value> {
        let migrations: Vec<ConfigMigration> = vec![("0.0.0", "1.0.0", super::manager::migrate_0_0_0_to_1_0_0)];

        let mut current_version = from_version.to_string();

        for (from, to, migrate_fn) in migrations {
            if super::manager::version_gte(&current_version, from) && super::manager::version_lt(&current_version, to) {
                debug!("Executing migration: {} -> {}", from, to);
                config = migrate_fn(config)?;
                current_version = to.to_string();
            }
        }

        Ok(config)
    }
}
