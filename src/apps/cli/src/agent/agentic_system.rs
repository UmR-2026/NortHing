use anyhow::{Context, Result};

use northhing_core::infrastructure::ai::AIClientFactory;
use northhing_core::service::config::initialize_global_config;

pub use northhing_core::agentic::system::{init_agentic_system, AgenticSystem};

pub async fn init_agentic_system_for_cli() -> Result<AgenticSystem> {
    initialize_global_config()
        .await
        .context("Failed to initialize global config service")?;
    AIClientFactory::initialize_global()
        .await
        .context("Failed to initialize global AIClientFactory")?;
    init_agentic_system()
        .await
        .context("Failed to initialize agentic system")
}
