// REFERENCE — extracted from
//   src/crates/contracts/runtime-ports/src/lib.rs (lines 695-765)
// Last synced: 2813b36 (v3-restructure)
// The trigger source enum and its policy mapping. The whole policy is just
// 7 variants + a small match — copy verbatim if you need a new trigger
// source.

use serde::{Deserialize, Serialize};

/// Who started this dialog turn. The actor/dispatcher should reuse this enum
/// rather than introducing a new "trigger" type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSubmissionSource {
    DesktopUi,
    DesktopApi,
    AgentSession,
    ScheduledJob,
    RemoteRelay,
    Bot,
    Cli,
}

/// Type alias used by the coordinator. `DialogTriggerSource` and
/// `AgentSubmissionSource` are the same type — pick whichever name reads
/// better at the call site.
pub type DialogTriggerSource = AgentSubmissionSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogQueuePriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogSubmissionPolicy {
    pub trigger_source: DialogTriggerSource,
    pub queue_priority: DialogQueuePriority,
    pub skip_tool_confirmation: bool,
}

impl DialogSubmissionPolicy {
    pub const fn new(
        trigger_source: DialogTriggerSource,
        queue_priority: DialogQueuePriority,
        skip_tool_confirmation: bool,
    ) -> Self {
        Self { trigger_source, queue_priority, skip_tool_confirmation }
    }

    /// ★ The default policy mapping. Copy this match when adding a new
    /// trigger source.
    pub const fn for_source(trigger_source: DialogTriggerSource) -> Self {
        let (queue_priority, skip_tool_confirmation) = match trigger_source {
            DialogTriggerSource::AgentSession   => (DialogQueuePriority::Low, true),
            DialogTriggerSource::ScheduledJob   => (DialogQueuePriority::Low, true),
            DialogTriggerSource::DesktopUi
            | DialogTriggerSource::DesktopApi
            | DialogTriggerSource::Cli           => (DialogQueuePriority::Normal, false),
            DialogTriggerSource::RemoteRelay
            | DialogTriggerSource::Bot           => (DialogQueuePriority::Normal, true),
        };
        Self::new(trigger_source, queue_priority, skip_tool_confirmation)
    }

    pub const fn with_queue_priority(mut self, queue_priority: DialogQueuePriority) -> Self {
        self.queue_priority = queue_priority;
        self
    }

    pub const fn with_skip_tool_confirmation(mut self, skip_tool_confirmation: bool) -> Self {
        self.skip_tool_confirmation = skip_tool_confirmation;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Where each variant is used (from the codebase sweep on 2026-06-19)
// ═══════════════════════════════════════════════════════════════════════
//
// AgentSession    — most common; scheduler.rs:240/298/361/852/981/1354
// DesktopUi       — apps/server/src/rpc_dispatcher.rs:392
// DesktopApi      — apps/desktop/src/app_state.rs:188; coordinator.rs:1907,5148
// Cli             — apps/cli/src/agent/core_adapter.rs:233,255
// ScheduledJob    — service/cron/service.rs:846
// RemoteRelay     — scheduler.rs:1415
// Bot             — scheduler.rs:1419
//
// When adding a new variant, add it to:
//   1. The enum above.
//   2. The match in `for_source`.
//   3. The scheduled-job and RPC dispatcher if it's triggered by either.
//   4. The actor/dispatcher design — it should route on the same source.
