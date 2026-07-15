//! Session evidence ledger + skill agent snapshot + listing diff + reconciliation
//!
//! Decomposed into focused sibling modules:
//! - ev_collect: evidence event collection (append/record/query/checkpoint/subagent timeout)
//! - ev_snapshot: skill agent snapshot persistence (per-turn baseline + baseline override)
//! - ev_listing: listing diff internal reminders strip + baseline rebuild coordination
//! - ev_reconcile: model reconciliation (migrate off invalidated models + listener)

pub use northhing_runtime_ports::SessionViewRestoreTiming;
