use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingStep {
    pub name: &'static str,
    pub duration_ms: u128,
}

pub fn elapsed_ms(started_at: Instant) -> u128 {
    started_at.elapsed().as_millis()
}

pub fn elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}
