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
pub use events::{BannerLevel, KernelEventDto, KernelEventsApi, SubscriptionId, ToolCallDto, ToolCallPhase};
pub use memory::{EpisodeDto, FactDto, KernelMemoryApi, ToolFailureRecordDto, ToolUseRecordDto};
pub use northhing_core_types::errors::{classify_ai_error_message, ErrorCategory};
pub use platform::{
    AnalysisDto, ArtifactDto, CoreHealthDto, FileTreeEntryDto, ImageContextDto, InspectorDataDto, KernelPlatformApi,
    PanelDto, PanelsConfigDto, SkillStatusDto, TerminalConfigDto,
};
pub use session::{
    BranchId, KernelSessionApi, MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto,
    PersistenceHandleDto, SessionBranchDto, SessionConfigDto, SessionDto, SessionId, SessionKindDto,
    SessionMetadataDto, SessionRelationshipDto, SessionStateDto, SessionStatusDto, SessionSummaryDto, ToolCallStub,
    WorkspaceSessionsDto,
};
pub use settings::{
    AIModelConfigDto, ConfigLocationDto, GlobalConfigDto, KernelSettingsApi, MCPServerConfigDto, MCPServerDto,
    MCPServerStatusDto, ProviderConfigDto, ProviderFormDto, ProviderTestResultDto,
};
pub use tools::{
    KernelToolsApi, ToolInfoDto, ToolPort, ToolRenderOptionsDto, ToolResultDto, ToolUseContextDto, UserInputRequestDto,
    UserInputResponseDto, ValidationResultDto,
};
pub use turn::{
    DialogSubmitOutcomeDto, DialogSubmitOutcomeKindDto, KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId,
    TurnInputDto, TurnStateDto, TurnStateKind,
};
pub use usage::{KernelUsageApi, TokenUsageDto, TurnUsageDto, UsageReportDto, UsageRequestDto};
pub use util::strip_prompt_markup;

#[cfg(test)]
mod contract_shape_tests {
    /// Secret-shaped words banned from contract DTO shapes.
    ///
    /// Scheme C invariant (2026-08-23 design decision): no DTO that the kernel
    /// RETURNS may be able to carry a secret value. API keys enter the kernel
    /// only through explicit write-method parameters (`upsert_model_config`).
    /// This source-level check guards against the field creeping back in via
    /// a future DTO — governance wired as code, not markdown.
    const BANNED_SECRET_WORDS: [&str; 7] = [
        "api_key",
        "access_key",
        "private_key",
        "secret",
        "password",
        "credential",
        "token",
    ];

    /// Inbound-only secret fields explicitly exempt from the ban, as
    /// `(file_suffix, field_name)`.
    ///
    /// These are fields the CALLER sends INTO the kernel (never returned),
    /// where carrying a secret is the point. The exemption list is deliberate
    /// and hand-verified: `ProviderFormDto.api_key` feeds
    /// `test_provider_config`, an inbound-only connectivity probe. Adding an
    /// entry requires stating why the shape is inbound-only; extending this
    /// list to dodge a returned-DTO hit is a Scheme C violation.
    const ALLOWED_INBOUND_SECRET_FIELDS: &[(&str, &str)] = &[("settings.rs", "api_key")];

    /// True when `name` contains a banned word on an underscore boundary.
    ///
    /// Boundary matching (C1 fix): the whole name, or delimited by underscores
    /// on either side — so `api_key`/`my_api_key`/`access_key_id` hit, while
    /// `prompt_tokens` (plural counter) and bare `key` (map key) do not. The
    /// previous segment matcher compared `name.split('_')` parts against
    /// compound banned words and could never fire on them.
    fn is_secret_shaped(name: &str) -> bool {
        BANNED_SECRET_WORDS.iter().any(|b| {
            name == *b
                || name.starts_with(&format!("{b}_"))
                || name.ends_with(&format!("_{b}"))
                || name.contains(&format!("_{b}_"))
        })
    }

    /// Field name when `line` declares a `pub` struct field that is secret-shaped,
    /// else `None`. Function parameters (no `pub` prefix) never match.
    fn secret_pub_field_on_line(line: &str) -> Option<&str> {
        let rest = line.trim_start().strip_prefix("pub ")?;
        let colon = rest.find(':')?;
        let name = rest[..colon].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        is_secret_shaped(name).then_some(name)
    }

    #[test]
    fn contract_dtos_expose_no_secret_shaped_fields() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut checked = 0usize;
        let mut violations: Vec<String> = Vec::new();
        visit(&src_dir, &mut |path| {
            let Ok(content) = std::fs::read_to_string(path) else {
                return;
            };
            checked += 1;
            for line in content.lines() {
                let Some(name) = secret_pub_field_on_line(line) else {
                    continue;
                };
                let exempt = ALLOWED_INBOUND_SECRET_FIELDS
                    .iter()
                    .any(|(file, field)| path.ends_with(file) && name == *field);
                if !exempt {
                    violations.push(format!("{}: pub field `{name}`", path.display()));
                }
            }
        });
        assert!(
            checked >= 10,
            "expected to scan the crate sources, found only {checked} files"
        );
        assert!(
            violations.is_empty(),
            "secret-shaped contract fields (Scheme C violation):\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn secret_shape_matcher_boundaries() {
        // Compound banned words must hit (C1 regression: the old segment
        // matcher never fired on underscore-containing words).
        assert!(is_secret_shaped("api_key"));
        assert!(is_secret_shaped("access_key_id"));
        assert!(is_secret_shaped("my_private_key"));
        assert!(is_secret_shaped("session_secret"));
        // Plural usage counters and bare map keys stay exempt.
        assert!(!is_secret_shaped("prompt_tokens"));
        assert!(!is_secret_shaped("completion_tokens"));
        assert!(!is_secret_shaped("key"));
        // Function parameters carry no `pub` and must not be flagged.
        assert_eq!(secret_pub_field_on_line("        api_key: Option<String>,"), None);
        // A real `pub` secret field must be detected.
        assert_eq!(
            secret_pub_field_on_line("    pub api_key: Option<String>,"),
            Some("api_key")
        );
        // The inbound allowlist entry resolves to the settings form field.
        assert!(ALLOWED_INBOUND_SECRET_FIELDS.contains(&("settings.rs", "api_key")));
    }

    fn visit(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, f);
            } else if path.extension().is_some_and(|e| e == "rs") {
                f(&path);
            }
        }
    }
}
