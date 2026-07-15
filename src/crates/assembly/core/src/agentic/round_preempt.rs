//! Compatibility re-exports for round-boundary injection state.

pub use northhing_agent_runtime::scheduler::{
    DialogRoundInjectionInterrupt, NoopDialogRoundInjectionSource, SessionRoundInjectionBuffer,
};
pub use northhing_runtime_ports::{
    DialogRoundInjectionSource, RoundInjection, RoundInjectionKind, RoundInjectionTarget,
};
