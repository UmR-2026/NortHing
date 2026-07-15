# R28b god-object split: session_manager.rs 1391 → facade + 4 sibling

**Date:** 2026-07-04 (continued from R28 retry earlier today)
**Branch:** main
**Round:** R28b (QClaw review blocker fix on R28)
**Author:** Mavis (Mavis take-over mode)
**Reviewer:** pending re-verification (QClaw block cleared; Kimi APPROVE pre-existing)

---

## Reason for retry

QClaw review (commit time 2026-07-04 22:50 against R25/R28/R29/R30/R31 batch) flagged R28
as a **blocker**:

> R28 session_manager.rs 1391 lines, 超 800 cap +591
> impl SessionManager 块 1230 行 (L108-L1338) is a new god-file. This is the same
> pattern as R27 manager_impl.rs (1234 lines) — R27's lesson wasn't applied.
> Recommended R28b sub-domain split ...

Kimi's combined-5-round verdict was APPROVE 9.0/10 because Kimi focuses on
concept-level design rather than line cap (memory rule: "QClaw 抓 line cap /
count, Kimi 抓 design"). Per memory "Reviewer 引用 line count 必 re-verify",
verified `wc -l session_manager.rs` = 1391 lines, +591 over the 800-line cap.

## Pre-split baseline (verified before commit)

- `git log -1 --format='%H'` → HEAD `ec7b4a0` (R31 cleanup commit)
- `src/crates/services/terminal/src/session/session_manager.rs` = 1391 lines
- `cargo check -p terminal-core` = 0 errors
- `cargo test -p terminal-core` = 22 passed

## Sub-domain split structure (final)

```
src/crates/services/terminal/src/session/
├── mod.rs                            # 320 lines (unchanged: TerminalSession struct + helpers)
├── session_manager.rs                # 170 lines (FACADE: struct + helpers + tests)
├── session_lifecycle.rs              # 430 lines (NEW: new/binding/create/get/list/IO/close/shutdown/Drop)
├── session_events.rs                 # 231 lines (NEW: start_event_forwarding/event_emitter/subscribe_session_output)
├── session_shell_integration.rs      # 215 lines (NEW: inject_shell_integration/wait_for_ready/state accessors)
├── session_commands.rs               # 474 lines (NEW: execute_command/execute_command_stream/send/wait_active)
├── binding.rs                        # unchanged
├── persistent.rs                     # unchanged
├── serializer.rs                     # unchanged
├── singleton.rs                      # unchanged
└── types.rs                          # 77 lines (unchanged)
```

## Method buckets

| Sibling | Method count | Methods |
|---|---|---|
| `session_manager.rs` (facade) | 0 inherent | struct SessionManager + 9 `pub(super)` fields + 3 free fn helpers + `cfg(test)` tests |
| `session_lifecycle.rs` | 13 methods + 1 Drop | `new`, `binding`, `create_session`, `create_session_with_options`, `get_session`, `list_sessions`, `write`, `resize`, `signal`, `close_session`, `acknowledge_data`, `shutdown_all`, `impl Drop for SessionManager` |
| `session_events.rs` | 3 methods | `start_event_forwarding`, `event_emitter`, `subscribe_session_output` |
| `session_shell_integration.rs` | 6 methods | `inject_shell_integration`, `wait_for_session_ready`, `wait_for_session_ready_static`, `integration_manager`, `has_shell_integration`, `get_command_state` |
| `session_commands.rs` | 6 methods | `execute_command`, `execute_command_with_options`, `execute_command_stream`, `execute_command_stream_with_options`, `send_command`, `wait_for_session_active` |

## Visibility strategy

- **SessionManager fields** (9 total): converted from private → `pub(super)`
  to allow sibling files within the `session` module to access private state.
  Documented as the established R28 pattern (per memory: "R28 子域 split:
  100+ fields pub(super) to make sibling visible").

- **Free helper functions** (3): `compute_stream_output_delta`,
  `get_integration_output_snapshot`, `get_post_command_terminal_state`
  declared `pub(super) fn` so `session_commands.rs` can import them as
  `use super::session_manager::{...}`. Const `COMMAND_TIMEOUT_INTERRUPT_GRACE_MS`
  similarly `pub(super)`.

- **Cross-sibling inherent methods** (2): `start_event_forwarding` (called
  from `session_lifecycle.rs::new`) and `inject_shell_integration` (called
  from `session_lifecycle.rs::create_session_with_options`) declared
  `pub(super) fn` within their respective impl blocks so siblings can
  access them via `self.<method>(...)` dispatch.

## Re-bucketing deviation from QClaw's exact plan

QClaw's recommended split specified lifecycle ~100 lines (new/binding/drop only).
Actual lifecycle ended up 430 lines because `create_session_with_options`
(173 lines) is functionally a lifecycle operation (session creation):
- It generates session ID, defaults shell type, captures cwd, generates nonce
- It creates the PTY process and registers session
- The shell integration injection within it (calling `inject_shell_integration`
  from shell_integration module) is one step in a longer lifecycle flow

Moving it to shell_integration would force duplication of `enable_integration &&
shell_type.supports_integration()` conditional across sibling boundaries.
Keeping it in lifecycle keeps the call pathway clean. Other buckets
(end up at 215-474 lines) all well under 800-cap.

## Pattern reference (R30 best practice)

This split uses "horizontal sub-domain" rather than "facade delegate":

- **Facadedelegate** (R30 pattern): split into `pub(super) fn name_impl(mgr, ...) -> ...`
  free functions + thin facade impl delegate methods. Best when sub-domains
  operate on DIFFERENT structs (e.g., R30 had 4 different structs:
  CommandExec, ControlSession, ExecProcess, StdinChannel).

- **Horizontal sub-domain** (R28b pattern): split `impl X` for the SAME struct
  by feature area into 4 sibling files. Best when:
  - All methods are on a single struct (SessionManager here)
  - Method names across sub-domains are DISJOINT (no E0592 conflict)
  - Fields with `pub(super)` visibility enable sibling access

  Each sibling opens with `impl SessionManager {` and closes with `}`.
  Multiple impl blocks for the same struct are valid Rust as long as
  methods don't collide.

## Verification (all 4 axes)

```text
cargo check -p terminal-core --message-format=short  → 0 errors (6 pre-existing warnings)
cargo check -p northhing-cli   --message-format=short  → 0 errors (3 pre-existing warnings)
cargo check -p northhing       --message-format=short  → 0 errors (5 pre-existing warnings)
cargo test  -p terminal-core                          → 22 passed
cargo fmt   --check -p terminal-core                 → clean (exit 0)
```

Sibling line counts all < 800:
- session_lifecycle.rs       430 lines  (< 800)
- session_events.rs          231 lines  (< 800)
- session_shell_integration.rs 215 lines  (< 800)
- session_commands.rs        474 lines  (< 800) — limited by
  `execute_command_stream_with_options` body (intrinsically 297 lines)

## Pre-existing noise NOT touched (per user instruction)

- 156 uncommitted fmt changes (workspace-wide)
- 12 untracked review/spec docs
- 22 unused-import warnings across northhing-core
- 6 unused `exec/mod.rs` constants + 1 unused TerminalError import
  (pre-existing in terminal-core)

## Commits

- `chore(fmt): strip BOM from api.rs + apply rustfmt to terminal-core` (ec7b4a0)
- `refactor(terminal-core): R28b sub-domain split session_manager.rs 1391 -> facade + 4 sibling` (0b3cfc7)

## Next review checkpoint

This unblocks R28 in the batch review (QClaw 6.5/10 → 9.0/10 expected).
Kimi APPROVE 9.0/10 unchanged (already approved). R28 batch review now APPROVE-able
unless new blocker surfaces.
