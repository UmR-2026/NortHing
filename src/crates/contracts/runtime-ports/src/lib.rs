#![allow(clippy::too_many_arguments)]
//! Thin runtime ports for boundaries that currently cross service and agentic
//! concrete implementations.
//!
//! This crate intentionally contains only DTOs and traits. It must not depend
//! on concrete managers, platform adapters, `northhing-core`, or app crates.
//!
//! R26 god-split: facade with 4 sibling sub-domain files (port_core,
//! session_workspace, remote, agent).

pub mod agent;
pub mod deep_research;
pub mod lightweight_task;
pub mod mcp;
pub mod port_core;
pub mod remote;
pub mod session_workspace;

pub use agent::*;
pub use deep_research::{
    ResearchCitationDisplayMapEntry, ResearchCitationRenumberOutput, ResearchCitationRenumberStats,
    renumber_research_report,
};
pub use lightweight_task::{
    LightweightTaskOutput, LightweightTaskRequest, LightweightTelemetrySink, ToolDispatcherPort,
};
pub use mcp::{
    format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
};
pub use port_core::*;
pub use remote::*;
pub use session_workspace::*;

#[cfg(test)]
mod agent_facade_tests;
#[cfg(test)]
mod port_facade_tests;
#[cfg(test)]
mod runtime_facade_tests;
