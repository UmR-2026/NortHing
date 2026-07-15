# Session Domain — Signatures

> One-page function signature card. Find the right function here, then
> open the corresponding `NN-*.rs` mirror for the full body.

## `ConversationCoordinator` — 6 public entry points

Source: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs`

| # | Method | Line | Signature | Purpose |
|---|---|---|---|---|
| 1 | `create_session` | 1085 | `async fn(name: Option<String>, agent_type: String, config: SessionConfig) -> NortHingResult<SessionSummary>` | Create a new session. |
| 2 | `start_dialog_turn` | 1929 | `async fn(session_id: String, user_input: String, original_user_input: Option<String>, turn_id: Option<String>, agent_type: String, workspace_path: Option<PathBuf>, submission_policy: DialogSubmissionPolicy, user_message_metadata: Option<serde_json::Value>) -> NortHingResult<DialogSubmitOutcome>` | Start a turn. **The hot path.** |
| 3 | `delete_session` | 3670 | `async fn(workspace_path: &Path, session_id: &str) -> NortHingResult<()>` | Delete a session. **Takes 2 args.** |
| 4 | `list_sessions` | 3866 | `async fn(workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>>` | List all sessions in a workspace. |
| 5 | `get_messages` | 3880 | `async fn(session_id: &str) -> NortHingResult<Vec<Message>>` | Full message history. |
| 6 | `get_messages_paginated` | 3885 | `async fn(session_id: &str, before: Option<MessageId>, limit: usize) -> NortHingResult<Vec<Message>>` | Paginated fetch. |

## `start_dialog_turn` — 4 sibling facades (DON'T copy this pattern — see NOTES.md)

All funnel into `start_dialog_turn_internal` at line 2717.

| Function | Line | Adds on top of `start_dialog_turn` |
|---|---|---|
| `start_dialog_turn` | 1929 | base |
| `start_dialog_turn_with_prepended_messages` | 1957 | `prepended_messages: Vec<Message>` |
| `start_dialog_turn_with_image_contexts` | 1986 | `image_contexts: Vec<ImageContext>` |
| `start_dialog_turn_with_image_contexts_and_prepended_messages` | 2015 | both of the above |

## `start_dialog_turn_internal` — the guard point

Source: `coordinator.rs:2717`. **The state-machine guard.** Only
`SessionState::Idle` or `SessionState::Error {..}` allow a new turn;
`Processing {..}` is rejected with a `Validation` error. This is the
place to extend when adding new states.

## State machine types

Source: `src/crates/assembly/core/src/agentic/core/state.rs` (full mirror in 04)

```rust
pub enum SessionState {
    Idle,
    Processing { current_turn_id: String, phase: ProcessingPhase },
    Error { error: String, recoverable: bool },
}

pub enum ProcessingPhase {
    Starting,
    Compacting,
    Thinking,
    Streaming,
    ToolCalling,
    ToolConfirming,
}
```

## `SessionStateManager` (DashMap-keyed store)

Source: `src/crates/assembly/core/src/agentic/coordination/state_manager.rs` (135 lines, full mirror in 05)

| Method | Line | Purpose |
|---|---|---|
| `new(event_router)` | — | Constructor; takes the event router to emit state changes. |
| `update_state(session_id, new_state)` | — | Atomic update + emit `AgenticEvent::SessionStateChanged`. |
| `set_processing_phase(session_id, phase)` | — | Convenience for the common "still processing, just changed phase" case. |
| `set_idle(session_id)` / `set_error(session_id, err)` | — | Terminal state setters. |
| `can_start_new_turn(session_id)` | — | The boolean gate the coordinator uses. |
| `is_processing(session_id)` | — | Cheap check. |
| `get_state(session_id)` | — | Current state. |

## `DialogTriggerSource` / `DialogSubmissionPolicy`

Source: `src/crates/contracts/runtime-ports/src/lib.rs:695-765` (full source in 02)

```rust
pub enum DialogTriggerSource {  // alias: AgentSubmissionSource
    DesktopUi, DesktopApi, AgentSession, ScheduledJob, RemoteRelay, Bot, Cli
}

pub enum DialogQueuePriority { Low = 0, Normal = 1, High = 2 }

pub struct DialogSubmissionPolicy {
    pub trigger_source: DialogTriggerSource,
    pub queue_priority: DialogQueuePriority,
    pub skip_tool_confirmation: bool,
}

impl DialogSubmissionPolicy {
    pub const fn for_source(src: DialogTriggerSource) -> Self;
    pub const fn with_queue_priority(self, p: DialogQueuePriority) -> Self;
    pub const fn with_skip_tool_confirmation(self, b: bool) -> Self;
}
```

`for_source` mapping:
- `AgentSession` / `ScheduledJob` → Low, skip-confirmation
- `DesktopUi` / `DesktopApi` / `Cli` → Normal, don't skip
- `RemoteRelay` / `Bot` → Normal, skip-confirmation

## `DialogSubmitOutcome` (return type)

Source: `runtime-ports/src/lib.rs:767-771` (full source in 03)

```rust
pub enum DialogSubmitOutcome {
    Started { session_id: String, turn_id: String },
    Queued   { session_id: String, turn_id: String },
}
```

## Slint callbacks (desktop app)

Source: `src/apps/desktop/src/app_state.rs` (full mirror in 06)

| # | Callback | Line | Coordinator call |
|---|---|---|---|
| 1 | `on_send_message(text)` | 142 | `start_dialog_turn` + `get_messages` |
| 2 | `on_new_session()` | 228 | `create_session` + `list_sessions` + `get_messages` |
| 3 | `on_switch_session(session_id)` | 286 | local `set_current_session_id` + `get_messages` (via `refresh_messages_ui`) |
| 4 | `on_delete_session(session_id)` | 314 | `delete_session(workspace, sid)` + `list_sessions` |
| 5 | `on_toggle_theme()` | 362 | local UI flip (no coordinator) |

**Note:** The HANDOFF/spec mentioned 6 callbacks; only 5 exist. The
sixth may be a planned `mcp_status` or `model_status` update that lives
elsewhere or hasn't landed. See HANDOFF.md "Known Issues".
