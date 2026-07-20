//! northhing-kernel-api crate.
//!
//! Facade crate defining the public API surface between host and kernel.
//! Contains only DTOs, traits, and error types — no business logic.
//!
//! ## Version
//!
//! K1 facade frozen schema — see `k1-facade-surface.md` §5 for FROZEN types.

#![allow(clippy::too_many_arguments)]

pub mod error;
pub mod bootstrap;
pub mod session;
pub mod turn;
pub mod events;
pub mod settings;
pub mod agents;
pub mod tools;
pub mod usage;
pub mod platform;
pub mod util;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::{KernelError, KernelResult};
pub use bootstrap::KernelBootstrapApi;
pub use session::{
    KernelSessionApi, SessionId, BranchId, SessionConfigDto, SessionSummaryDto, SessionDto,
    SessionStateDto, SessionKindDto, SessionMetadataDto, SessionRelationshipDto, SessionStatusDto,
    SessionBranchDto, PersistenceHandleDto, MessageDto, MessageRoleDto, MessageContentDto,
    MessageMetadataDto, ToolCallStub,
};
pub use turn::{
    KernelTurnApi, TurnId, TurnInputDto, SubmissionPolicyDto, TriggerSourceDto,
    DialogSubmitOutcomeDto, TurnStateDto, TurnStateKind,
};
pub use events::{
    KernelEventsApi, SubscriptionId, ToolCallDto, ToolCallPhase, BannerLevel,
    KernelEventDto, BackendEventDto,
};
pub use settings::{
    KernelSettingsApi, GlobalConfigDto, GlobalConfigPatchDto, AIModelConfigDto,
    MCPServerDto, MCPServerConfigDto, MCPServerStatusDto, ConfigLocationDto,
    ProviderTestResultDto, ProviderFormDto, ProviderConfigDto,
};
pub use agents::{
    KernelAgentsApi, AgentInfoDto, SubagentDto, SubagentScopeDto, SkillInfoDto,
    SkillScopeDto, SkillOverridesDto, SkillOverrideEntry, ProjectSkillsDto, ProjectSkillEntry,
};
pub use tools::{
    KernelToolsApi, ToolPort, ToolInfoDto, ToolRenderOptionsDto, ToolResultDto,
    ToolUseContextDto, ValidationResultDto, UserInputRequestDto, UserInputResponseDto,
};
pub use usage::{
    KernelUsageApi, UsageRequestDto, UsageReportDto, TurnUsageDto, TokenUsageDto,
};
pub use platform::{
    KernelPlatformApi, TerminalConfigDto, ImageContextDto, AnalysisDto, CoreHealthDto,
    PanelsConfigDto, PanelDto, InspectorDataDto, SkillStatusDto, ArtifactDto,
};
pub use util::strip_prompt_markup;
