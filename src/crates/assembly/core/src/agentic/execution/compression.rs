//! Round 48b split sibling: compression facade
//!
//! Split from the original 789-line god-file into facade + 3 siblings:
//! - `compress_scaffold.rs`: scaffold resolution + model-summary retry
//! - `compress_summary.rs`: model-based summary generation
//! - `compress_run.rs`: auto-compression + manual compaction entry points

use std::sync::Arc;

/// Runtime scaffold resolved once per compression turn, then threaded through
/// model-summary generation and the final `compress_turns_with_contract` call.
pub(super) struct CompressionRuntimeScaffold {
    pub(super) ai_client: Arc<crate::infrastructure::ai::AIClient>,
    pub(super) tool_definitions: Option<Vec<crate::util::types::ToolDefinition>>,
    pub(super) system_prompt_message: crate::agentic::core::Message,
    pub(super) prepended_prompt_reminders: crate::agentic::agents::PrependedPromptReminders,
    pub(super) primary_supports_image_understanding: bool,
    pub(super) compression_contract_limit: usize,
}
