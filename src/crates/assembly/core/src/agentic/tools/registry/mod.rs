//! Tool registry facade

pub mod registry_capabilities;
pub mod registry_global;
pub mod registry_lookup;
pub mod registry_provider;
pub mod registry_register;
pub mod registry_types;

pub use registry_capabilities::{tool_capabilities_summary, ToolCapabilitiesSummary};
pub use registry_global::{
    all_tools, create_tool_registry, get_all_registered_tool_names, get_all_registered_tools,
    get_readonly_registered_tool_names, get_readonly_tools, global_tool_registry,
};
pub use registry_types::{ProductToolDecoratorRef, ToolRef, ToolRegistry, GET_TOOL_SPEC_TOOL_NAME};

include!("tests.rs");
