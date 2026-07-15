//! Shared types and constants for the agentic::coordination runtime ports.

use crate::agentic::core::Message;
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::agentic::tools::ToolRuntimeRestrictions;
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::util::errors::NortHingError;
use northhing_runtime_ports::DelegationPolicy;

pub use northhing_runtime_ports::DialogTriggerSource;

#[allow(dead_code)]
const MANUAL_COMPACTION_COMMAND: &str = "/compact";
#[allow(dead_code)]
const CONTEXT_COMPRESSION_TOOL_NAME: &str = "ContextCompression";
#[allow(dead_code)]
pub(crate) const DEFAULT_SUBAGENT_MAX_CONCURRENCY: usize = 5;
#[allow(dead_code)]
pub(crate) const MAX_SUBAGENT_MAX_CONCURRENCY: usize = 64;

#[allow(dead_code)]
struct WrappedUserInputPayload {
    content: String,
    prepended_messages: Vec<Message>,
    skill_agent_snapshot: TurnSkillAgentSnapshot,
    snapshot_persistence: SkillAgentSnapshotPersistence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SkillAgentSnapshotPersistence {
    None,
    SaveCurrentTurn,
    RecoverFirstTurnBaseline,
}
