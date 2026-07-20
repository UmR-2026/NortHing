//! Kernel bootstrap API.

use crate::error::KernelError;

#[async_trait::async_trait]
pub trait KernelBootstrapApi: Send + Sync {
    /// Bootstrap the core (abnormal items 2/4).
    /// Source: #1 #48 #77 #6 #7 #58 #59
    async fn init_core(&self) -> Result<(), KernelError>;

    /// Check if core is initialized.
    /// Source: F0.2 core_ready()
    fn core_ready(&self) -> bool;
}
