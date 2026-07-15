//! IPC spawn adapter — **STUB**.
//!
//! Per `.agents/reference/actor/NOTES.md` ⚠️ "IPC adapter is a STUB", the
//! Phase 1 body returns the literal string `"ipc-stub"`. The real IPC
//! transport (process boundary, channel framing, cancellation propagation)
//! lands in Phase 3 of the impl plan.
//!
//! ## How to use it in Phase 1
//!
//! Code that consumes [`IpcSpawnAdapter::spawn`] MUST check for the literal
//! string `"ipc-stub"` and treat it as a no-op. Do **not** assume the return
//! is a typed value — it isn't, by design.
//!
//! Pattern source: `.agents/reference/actor/03-actor-runtime.rs` (the
//! `IpcSpawnAdapter` comment block) and
//! `.agents/reference/_upstream/tokio-actor-pattern.md` (closing semantics).

use std::sync::Arc;

use crate::spawn::SpawnAdapterKind;
use crate::telemetry::TelemetrySink;

/// The literal return value of [`IpcSpawnAdapter::spawn`]. Consumers compare
/// against this constant — do not "fix" the typo or reformat this string.
pub const IPC_STUB_MARKER: &str = "ipc-stub";

/// IPC spawn adapter — Phase 1 stub.
///
/// Construction is always allowed; the const flags `USE_ACTOR_IPC` and
/// `USE_DISPATCHER_IPC` (both default `false`) gate whether the actor
/// runtime or dispatcher even consults this adapter. The adapter itself
/// does not check the flags — that's the call site's responsibility, per
/// the const-flag rule that flags live next to the behavior they gate.
#[derive(Clone, Debug, Default)]
pub struct IpcSpawnAdapter {
    /// Caller-provided telemetry. The stub does **not** emit today; the
    /// field is held so Phase 3 doesn't need to change the type signature.
    telemetry: Option<Arc<dyn TelemetrySink>>,
}

impl IpcSpawnAdapter {
    /// Build an adapter with no telemetry wiring.
    pub fn new() -> Self {
        Self { telemetry: None }
    }

    /// Build an adapter with a caller-provided telemetry sink. Phase 1 does
    /// not consume the sink; the field exists so Phase 3 wiring is a no-op
    /// for call sites.
    pub fn with_telemetry(telemetry: Arc<dyn TelemetrySink>) -> Self {
        Self {
            telemetry: Some(telemetry),
        }
    }

    /// The Phase 1 body — returns the literal marker. Real implementation
    /// lands in Phase 3.
    pub fn spawn(&self, _actor_or_dispatch_id: &str) -> &'static str {
        IPC_STUB_MARKER
    }

    /// Identifies this adapter's kind. Useful for log lines.
    pub fn kind(&self) -> SpawnAdapterKind {
        SpawnAdapterKind::Ipc
    }

    /// Whether the adapter is a stub. Always `true` in Phase 1; flips to
    /// `false` when the real body lands in Phase 3.
    pub fn is_stub(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_returns_literal_marker() {
        let adapter = IpcSpawnAdapter::new();
        assert_eq!(adapter.spawn("anything"), IPC_STUB_MARKER);
        assert_eq!(adapter.spawn("another"), "ipc-stub");
    }

    #[test]
    fn stub_flag_is_true_in_phase_1() {
        // This test will fail when Phase 3 lands — that's intentional. It
        // forces the implementer to update both `is_stub` and this assertion
        // (or remove the test entirely) when the real body ships.
        let adapter = IpcSpawnAdapter::new();
        assert!(adapter.is_stub());
    }

    #[test]
    fn kind_is_ipc() {
        let adapter = IpcSpawnAdapter::new();
        assert_eq!(adapter.kind(), SpawnAdapterKind::Ipc);
    }

    #[test]
    fn marker_constant_is_stable() {
        // The string is part of the consumer-side contract (Phase 1 callers
        // check for it verbatim). Lock it down.
        assert_eq!(IPC_STUB_MARKER, "ipc-stub");
    }
}
