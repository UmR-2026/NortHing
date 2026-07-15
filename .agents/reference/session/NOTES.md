# Session Domain — "Do NOT Copy Verbatim" Notes

## ⛔ Do NOT copy the 4-way `start_dialog_turn_*` facade

`coordinator.rs:1929/1957/1986/2015` has four sibling functions:
- `start_dialog_turn` (base)
- `start_dialog_turn_with_prepended_messages` (line 1957)
- `start_dialog_turn_with_image_contexts` (line 1986)
- `start_dialog_turn_with_image_contexts_and_prepended_messages` (line 2015)

All four funnel into `start_dialog_turn_internal` (line 2717). The new
**actor / one-shot dispatcher** design should NOT copy this 4-way facade.
Use a single parameterized entry with `Option<Vec<Message>>` and
`Option<Vec<ImageContext>>` parameters. The 4-way shape was retconned from
an earlier time when prepended messages and image contexts were
experimental; they should never have become top-level entry points.

## ⛔ Do NOT copy `OnceLock<mpsc::Sender<...>>` injection pattern

`coordinator.rs:518, 520` — `scheduler_notify_tx` and `round_injection_source`
are both `OnceLock<...>` and are set lazily after construction. This is
fine for the coordinator because wiring is a one-shot setup step, but
**the actor / dispatcher design should wire at construction** instead.
`OnceLock` for hot-path senders adds an extra branch on every message.

## ⛔ Do NOT skip the state-machine guard

`coordinator.rs:2813-2839` — the `Idle | Error` allow-list. Any new
trigger (CLI, server, bot) that calls `start_dialog_turn` MUST rely on
this guard to prevent concurrent turns in the same session. If you
build a parallel "actor entry point" that doesn't go through this
guard, you will create race conditions on `SessionManager`.

## ⛔ Do NOT touch `coordinator.rs:4172-5025` (existing subagent path)

`execute_hidden_subagent_internal` at line 4173 is the heavy multi-turn
subagent path that the lightweight actor design explicitly replaces. The
spec at `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md`
calls this out: the new actor design should **bypass** this path, not
extend it. If you need a new subagent behavior, add a new dispatch
surface, do not modify this function.

## ⛔ Do NOT duplicate `SessionState` / `ProcessingPhase` definitions

`core/state.rs:13-34` is the canonical source. Previously two copies
existed (`DialogTurnState` and `ModelRoundState`); both were deleted
per the comment at `core/state.rs:125-129` ("Removed to avoid future
ambiguity"). If you need a similar concept, extend the canonical
enum, do not create a new type.

## ⛔ Do NOT add a new `DialogTriggerSource` variant without updating 4 sites

Adding a new variant requires touching:
1. The enum in `runtime-ports/src/lib.rs:701-709`.
2. The match in `DialogSubmissionPolicy::for_source` (line 742-754).
3. The 7 call sites enumerated in `02-dialog-trigger-source.rs` (where
   the new variant would be used).
4. Any new consumer that triggers turns (RPC dispatcher, scheduler, etc.).

If you only touch 1+2, your new variant will hit an "unhandled match arm"
compile error at one of the call sites.

## ⚠️ Per-UI-event tokio runtime

`app_state.rs` constructs a fresh `tokio::runtime::Builder::new_current_thread().enable_all()`
runtime **per UI event**. This is intentional (Slint's callback context is
synchronous) but adds 1-2 ms of overhead per event. If you have a hot
callback path, consider promoting to a worker thread that pulls from an
mpsc. Do NOT just call `block_on` on the global runtime — that deadlocks
when the event was triggered from the runtime's own task.

## ⚠️ `SessionState::Processing` is a struct variant, not a unit variant

`coordinator.rs` and `state_manager.rs` use `matches!(state, SessionState::Processing { .. })`
to detect in-flight state. Do NOT write `matches!(state, SessionState::Processing)` — that
won't compile.

## ⚠️ `delete_session` takes 2 args, not 1

`coordinator.rs:3670` — `delete_session(workspace_path: &Path, session_id: &str)`.
The session id alone is not unique (sessions can be moved between
workspaces), so the workspace path is required to scope the lookup.

## ⚠️ `start_dialog_turn` requires agent_type as String, not enum

`coordinator.rs:1929` — `agent_type: String`. The coordinator does not
type-check the agent type at the API boundary; the type is forwarded
to the agent registry. If you need a typed check, validate in the
caller, not in the coordinator.

## ⚠️ `get_messages` returns the full history, not a stream

`coordinator.rs:3880` — `Vec<Message>` returned in one shot. For sessions
with 10K+ messages this is expensive. Use `get_messages_paginated` for
streaming. Do not modify `get_messages` to be paginated — other callers
depend on the full-history semantics.

## ⚠️ `SessionStateManager` emits an event on every state change

`state_manager.rs` — every `update_state` / `set_*` call emits an
`AgenticEvent::SessionStateChanged` event. If you call `set_processing_phase`
5 times in a row, the event bus sees 5 events. If you want to batch, do
it at the call site, not in the manager.

## ⚠️ There is no dedicated `coordinator.rs` test file

Tests that exercise coordinator-adjacent state live inside
`session_manager.rs` at lines 4595, 4629, 4718, 5018, 5159, 5610, 5899.
Scheduler/policy tests live in
`src/crates/execution/agent-runtime/tests/scheduler_contracts.rs` and
in `runtime-ports/src/lib.rs:1419-1535` (embedded policy tests).

## ✅ Things you SHOULD copy

- The 6-entry-point shape (in 01). It maps cleanly onto the
  create/list/get/delete/start/turn vocabulary every consumer wants.
- The `Idle | Error` state-machine guard (coordinator.rs:2813-2839).
  Even the new actor design should funnel through this guard.
- The `DialogTriggerSource` enum + `for_source` mapping (in 02). The
  default policy is well-thought-out; copy it verbatim for any new
  trigger source.
- The `SessionState` + `ProcessingPhase` enum (in 04). These are the
  canonical types. Do not fork.
- The DashMap-keyed registry pattern in `SessionStateManager` (in 05).
  This is the project's idiomatic way to expose global state to async
  consumers; use it for any new stateful registry.
- The 9 Slint callback patterns in `app_state.rs` (in 06). They show
  the right shape for a Slint-to-coordinator bridge: get the coordinator,
  build a per-event runtime, call the right method, refresh the model.
