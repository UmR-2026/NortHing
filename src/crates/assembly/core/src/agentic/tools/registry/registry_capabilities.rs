//! Tool capability discovery and enumeration.
//!
//! Exposes tool capability metadata: expanded/collapsed exposure split and
//! readonly status, so callers can build manifests without reaching into
//! the inner registry directly.

use super::registry_types::{ToolRef, ToolRegistry};
use crate::agentic::tools::framework::{Tool, ToolExposure};
use std::sync::Arc;

impl ToolRegistry {
    /// Return the names of tools whose default exposure is `Expanded`.
    pub fn expanded_tool_names(&self) -> Vec<String> {
        self.inner
            .all_tools()
            .into_iter()
            .filter(|tool| tool.default_exposure() == ToolExposure::Expanded)
            .map(|tool| tool.name().to_string())
            .collect()
    }

    /// Return the names of tools that are marked as readonly.
    pub fn readonly_tool_names(&self) -> Vec<String> {
        self.inner
            .all_tools()
            .into_iter()
            .filter(|tool| tool.is_readonly())
            .map(|tool| tool.name().to_string())
            .collect()
    }

    /// Return expanded tools as `Arc<dyn Tool>` references.
    pub fn expanded_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.inner
            .all_tools()
            .into_iter()
            .filter(|tool| tool.default_exposure() == ToolExposure::Expanded)
            .collect()
    }

    /// Return readonly tools as `Arc<dyn Tool>` references.
    pub fn readonly_tools_ref(&self) -> Vec<Arc<dyn Tool>> {
        self.inner
            .all_tools()
            .into_iter()
            .filter(|tool| tool.is_readonly())
            .collect()
    }
}

/// Capability summary for the current product registry.
///
/// Returned by [`get_tool_capabilities_summary`]; all counts and name lists
/// are derived from the default product registry so callers do not need to
/// run multiple queries.
#[derive(Debug, Clone)]
pub struct ToolCapabilitiesSummary {
    /// Total registered tools.
    pub total_count: usize,
    /// Tools with `Expanded` exposure (visible in the model manifest).
    pub expanded_names: Vec<String>,
    /// Tools with `Collapsed` exposure (hidden; accessible via GetToolSpec).
    pub collapsed_names: Vec<String>,
    /// Tools that do not mutate state or filesystem.
    pub readonly_names: Vec<String>,
}

/// Build a [`ToolCapabilitiesSummary`] from the default product registry.
pub fn tool_capabilities_summary() -> ToolCapabilitiesSummary {
    let registry = crate::agentic::tools::registry::global_tool_registry();
    // SAFETY: we hold the read lock for the duration of this function; the
    // tokio RwLock read guard is not `Send` but this is a single-threaded
    // call path from the async context. We use `blocking_read` via the
    // standard pattern used elsewhere in this module.
    let registry_lock = registry.blocking_read();
    let all_tools = registry_lock.all_tools();

    let mut expanded_names = Vec::new();
    let mut collapsed_names = Vec::new();
    let mut readonly_names = Vec::new();

    for tool in all_tools {
        let name = tool.name().to_string();
        let is_expanded = matches!(tool.default_exposure(), ToolExposure::Expanded);
        match tool.default_exposure() {
            ToolExposure::Expanded => expanded_names.push(name.clone()),
            ToolExposure::Collapsed => collapsed_names.push(name.clone()),
        }
        if tool.is_readonly() && is_expanded {
            readonly_names.push(name);
        }
    }

    let total_count = expanded_names.len() + collapsed_names.len();

    ToolCapabilitiesSummary {
        total_count,
        expanded_names,
        collapsed_names,
        readonly_names,
    }
}
