# Session Domain — Reference

> Session / Multi-session / ConversationCoordinator code mirrors.
> Read [`SIGNATURES.md`](./SIGNATURES.md) first, then
> [`NOTES.md`](./NOTES.md) for "do NOT copy" warnings.

## What this domain contains

`ConversationCoordinator` is the top-level integration point for
session lifecycle, message stream, state machine, and event emission. It
funnels every UI/API trigger through `start_dialog_turn`, which checks
the session's current state against an `Idle | Error` allow-list before
running.

There are **6 public entry points** that consumers (the Slint desktop app,
the CLI, the server) actually call. Everything else is plumbing.

## File ordering

| # | File | Why |
|---|---|---|
| 01 | [`01-conversation-coordinator.rs`](./01-conversation-coordinator.rs) | The 6 entry points + the state-machine guard. |
| 02 | [`02-dialog-trigger-source.rs`](./02-dialog-trigger-source.rs) | `DialogTriggerSource` enum + policy mapping. |
| 03 | [`03-dialog-submission-policy.rs`](./03-dialog-submission-policy.rs) | `DialogSubmitOutcome` (return type). |
| 04 | [`04-session-state.rs`](./04-session-state.rs) | `SessionState` + `ProcessingPhase` (full mirror). |
| 05 | [`05-session-state-manager.rs`](./05-session-state-manager.rs) | `SessionStateManager` (DashMap-keyed store, full mirror). |
| 06 | [`06-app-state-slint-wiring.rs`](./06-app-state-slint-wiring.rs) | The 9 Slint callbacks wired in `apps/desktop/src/app_state.rs` (full mirror, post Phase A). |

## How a session starts (end-to-end)

```
Slint callback                 DesktopApi branch in app_state.rs
   "send-message"  ─────►     ui.on_send_message(text)
                                     │
                                     ▼
                            tokio::runtime::Builder
                            .new_current_thread()  (per UI event)
                                     │
                                     ▼
                            coordinator.start_dialog_turn(
                                session_id, text, None, None,
                                "code", Some(workspace),
                                DialogSubmissionPolicy::for_source(
                                    DialogTriggerSource::DesktopApi,
                                ),
                                None,
                            )
                                     │
                                     ▼
                            start_dialog_turn_internal (2717)
                                     │
                                     │ checks SessionState: Idle|Error only
                                     │ rejects with Validation if Processing
                                     ▼
                            SessionManager → ExecutionEngine
                                     │
                                     ▼
                            coordinator.get_messages(&sid)
                            → refresh_messages_ui()
```

## Selection guide

| You need to… | Start with |
|---|---|
| Add a new consumer of the coordinator (CLI, server, bot) | 02 (pick a `DialogTriggerSource` variant) + 01 (the 6 entry points) |
| Change the state machine (e.g. add `Paused`) | 04 (the enum) + 01 (the guard at 2717) |
| Add a new Slint callback | 06 (the 5 existing patterns) |
| Tune priority / skip-confirmation for an existing source | 02 (`for_source` match) |
| Change message persistence | (SessionManager is too large to mirror — read `src/.../session/session_manager.rs` directly) |
| Change turn outcome routing | (Scheduler is too large — read `src/.../coordination/scheduler.rs` directly) |

## Public entry points at a glance

```rust
use northhing_core::agentic::coordination::{ConversationCoordinator, DialogTriggerSource};
use northhing_core::contracts::runtime_ports::DialogSubmissionPolicy;

// 1. Create
let summary = coordinator.create_session(name, "code".to_string(), config).await?;

// 2. Start a turn (the hot path)
let outcome = coordinator.start_dialog_turn(
    session_id, user_input, None, None,
    "code".to_string(), Some(workspace),
    DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi),
    None,
).await?;

// 3. List / inspect
let sessions = coordinator.list_sessions(&workspace).await?;
let messages = coordinator.get_messages(&session_id).await?;

// 4. Delete
coordinator.delete_session(&workspace, &session_id).await?;
```
