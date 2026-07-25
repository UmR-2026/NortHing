use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    pub distiller_enabled: bool,
    pub distiller_model: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            distiller_enabled: true,
            distiller_model: None,
        }
    }
}
