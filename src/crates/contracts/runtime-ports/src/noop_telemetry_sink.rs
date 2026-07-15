//! Drop-everything telemetry sink for the lightweight task port.
//!
//! Useful as the default when no caller-provided sink is wired up. The
//! producer side (`northhing-agent-dispatch`) has its own richer sink; this
//! port-side type keeps the boundary one-way.

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopLightweightTelemetrySink;

impl LightweightTelemetrySink for NoopLightweightTelemetrySink {
    #[inline]
    fn emit_event(&self, _event_kind: &'static str) {}
}

