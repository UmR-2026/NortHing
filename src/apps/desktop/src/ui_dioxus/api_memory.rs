// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room memory API (W10-1 split).
// Read-only wrappers over `northhing_core::kernel_facade()`.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::memory::{FactDto, KernelMemoryApi};

/// Lists memory facts, optionally filtered by workspace.
pub async fn list_facts(workspace_slug: Option<&str>) -> Result<Vec<FactDto>, KernelError> {
    kernel_facade().list_facts(workspace_slug).await
}

/// Full-text searches memory facts, optionally filtered by workspace.
pub async fn search_facts(
    query: &str,
    workspace_slug: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<FactDto>, KernelError> {
    kernel_facade().search_facts(query, workspace_slug, limit).await
}
