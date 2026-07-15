use crate::agentic::tools::framework::Tool;
use northhing_agent_tools::{ToolDecoratorRef, ToolRegistry as AgentToolRegistry};
use std::sync::Arc;

pub type ToolRef = Arc<dyn Tool>;
pub type ProductToolDecoratorRef = ToolDecoratorRef<dyn Tool>;

pub use northhing_agent_tools::GET_TOOL_SPEC_TOOL_NAME;

/// Tool registry - manages all available tools (using IndexMap to maintain registration order)
pub struct ToolRegistry {
    pub(in crate::agentic::tools) inner: AgentToolRegistry<dyn Tool>,
}
