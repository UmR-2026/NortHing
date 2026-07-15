// REFERENCE — extracted from
//   src/crates/contracts/runtime-ports/src/lib.rs
// Last synced: 2813b36 (v3-restructure)
// `DialogSubmitOutcome` — the return type of `start_dialog_turn`.

use serde::{Deserialize, Serialize};

/// What happened when a dialog turn was submitted. The session_id is
/// always present; turn_id is the new turn's id (whether started or queued).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogSubmitOutcome {
    /// The turn was started immediately. Scheduler decided it could run.
    Started { session_id: String, turn_id: String },
    /// The turn was queued. Scheduler will run it when a slot frees up.
    Queued { session_id: String, turn_id: String },
}

impl DialogSubmitOutcome {
    pub fn session_id(&self) -> &str {
        match self {
            DialogSubmitOutcome::Started { session_id, .. } => session_id,
            DialogSubmitOutcome::Queued { session_id, .. } => session_id,
        }
    }
    pub fn turn_id(&self) -> &str {
        match self {
            DialogSubmitOutcome::Started { turn_id, .. } => turn_id,
            DialogSubmitOutcome::Queued { turn_id, .. } => turn_id,
        }
    }
    pub fn is_started(&self) -> bool {
        matches!(self, DialogSubmitOutcome::Started { .. })
    }
    pub fn is_queued(&self) -> bool {
        matches!(self, DialogSubmitOutcome::Queued { .. })
    }
}
