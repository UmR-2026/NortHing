//! northhing-kernel-api crate.
//!
//! Facade crate defining the public API surface between host and kernel.
//! Contains only DTOs, traits, and error types — no business logic.
//!
//! ## Version
//!
//! K1 facade frozen schema — see `k1-facade-surface.md` §5 for FROZEN types.

#![allow(clippy::too_many_arguments)]

pub mod agents;
pub mod bootstrap;
pub mod error;
pub mod events;
pub mod memory;
pub mod platform;
pub mod session;
pub mod settings;
pub mod tools;
pub mod turn;
pub mod usage;
pub mod util;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use agents::{
    AgentInfoDto, KernelAgentsApi, ProjectSkillEntry, ProjectSkillsDto, SkillInfoDto, SkillOverrideEntry,
    SkillOverridesDto, SkillScopeDto, SubagentDto, SubagentScopeDto,
};
pub use bootstrap::KernelBootstrapApi;
pub use error::{KernelError, KernelResult};
pub use events::{
    BackendEventDto, BannerLevel, KernelEventDto, KernelEventsApi, SubscriptionId, ToolCallDto, ToolCallPhase,
};
pub use memory::{EpisodeDto, KernelMemoryApi, ToolFailureRecordDto, ToolUseRecordDto};
pub use platform::{
    AnalysisDto, ArtifactDto, CoreHealthDto, ImageContextDto, InspectorDataDto, KernelPlatformApi, PanelDto,
    PanelsConfigDto, SkillStatusDto, TerminalConfigDto,
};
pub use session::{
    BranchId, KernelSessionApi, MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto,
    PersistenceHandleDto, SessionBranchDto, SessionConfigDto, SessionDto, SessionId, SessionKindDto,
    SessionMetadataDto, SessionRelationshipDto, SessionStateDto, SessionStatusDto, SessionSummaryDto, ToolCallStub,
    WorkspaceSessionsDto,
};
pub use settings::{
    AIModelConfigDto, ConfigLocationDto, GlobalConfigDto, GlobalConfigPatchDto, KernelSettingsApi, MCPServerConfigDto,
    MCPServerDto, MCPServerStatusDto, ProviderConfigDto, ProviderFormDto, ProviderTestResultDto,
};
pub use tools::{
    KernelToolsApi, ToolInfoDto, ToolPort, ToolRenderOptionsDto, ToolResultDto, ToolUseContextDto, UserInputRequestDto,
    UserInputResponseDto, ValidationResultDto,
};
pub use turn::{
    DialogSubmitOutcomeDto, KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto, TurnStateDto,
    TurnStateKind,
};
pub use usage::{KernelUsageApi, TokenUsageDto, TurnUsageDto, UsageReportDto, UsageRequestDto};
pub use util::strip_prompt_markup;
