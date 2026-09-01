use std::sync::atomic::{AtomicBool, Ordering};

static INCLUDE_SENSITIVE_DIAGNOSTICS: AtomicBool = AtomicBool::new(true);

pub fn set_include_sensitive_diagnostics(enabled: bool) {
    INCLUDE_SENSITIVE_DIAGNOSTICS.store(enabled, Ordering::Relaxed);
}

pub fn include_sensitive_diagnostics() -> bool {
    INCLUDE_SENSITIVE_DIAGNOSTICS.load(Ordering::Relaxed)
}
