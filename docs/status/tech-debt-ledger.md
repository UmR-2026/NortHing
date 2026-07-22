# Tech Debt Ledger

> Living document. Each entry: symptom, evidence, proposed fix, status.
> Aligned with `docs/tech-debt-cleanup-guide.md` §7 (frozen line — items registered but not addressed in this wave).
> Update this file when a new debt item is discovered or an existing one is resolved.

## P0 — User-blocking issues (active surfaces only)

### P0-1: Desktop message queuing — messages sent during active turn are silently lost

- **Symptom**: When a dialog turn is running, `on_send_message` does not check `streaming_session` state. The UI does not disable the input box. User messages submitted during an active turn may be silently dropped or cause state corruption.
- **Evidence**: `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:22-67` — `on_send_message` closure does not gate on `app_state.get_streaming_session()`. `src/apps/desktop/src/ui/main.slint:92,258` — `is-streaming` bound to visual state only, not input disable.
- **Proposed fix**: (1) Gate `on_send_message` on streaming state; queue messages when active. (2) Or disable input box via `is-streaming` binding. (3) Implement `DialogSteeringAction` / `RoundInjection` consumption path for queued messages.
- **Status**: `resolved` — fixed by `1b5225d` (W3a-4, 2026-07-18): DialogScheduler queues messages during active turns

### P0-2: Hang triple — AskUserQuestion no timeout + tool execution no cancel select + turn no overall timeout

- **Symptom**: (1) `AskUserQuestion` waits indefinitely for user input — no `timeout` field. (2) Tool execution does not respond to cancel token within AskUserQuestion's blocking future. (3) Main dialog turn has no overall timeout (only subagent has `timeout_seconds`).
- **Evidence**: `src/crates/execution/agent-runtime/src/user_questions.rs:1-80` — no timeout. `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_lifecycle/lifecycle.rs:142` — subagent has timeout, main turn does not. Search for `turn_timeout` / `TURN_TIMEOUT` in `src/` returns no matches.
- **Proposed fix**: (1) Add `timeout_ms` to AskUserQuestion with default (e.g. 5 min). (2) Wrap user input wait in `tokio::select!` with cancel token. (3) Add configurable turn-level timeout (e.g. 30 min) that auto-cancels and emits `DialogTurnFailed`.
- **Status**: `resolved` — fixed by `3de7ced` / `26f392e` / `ad5ffa0` (W3a-1/2/3, 2026-07-18): AskUserQuestion timeout+cancel, 300s tool/confirmation defaults, turn watchdog + cancel convergence

## P1 — Safety and reliability

### P1-1: Non-atomic config write — power loss during save = corrupted app.json

- **Symptom**: `save_app_settings` uses `tokio::fs::write` directly. No temp-file + rename pattern. Code comment acknowledges: "Phase 1: simple write — upgrade to atomic in Phase 5".
- **Evidence**: `src/apps/desktop/src/app_state/settings.rs:655-667`. `src/crates/assembly/core/src/infrastructure/storage/persistence.rs:15-20` has file lock mechanism but `save_app_settings` does not use it.
- **Proposed fix**: Write to `app.json.tmp`, then `tokio::fs::rename` (atomic on same filesystem). Use existing `FILE_LOCKS` from persistence.rs.
- **Status**: active (code comment says Phase 5)

### P1-2: API key stored in plaintext

- **Symptom**: `ProviderConfig.api_key` stored as plaintext string in `app.json`. No keyring, encryption, or obfuscation. Code comment: "Stored in plaintext in app.json. Never logged."
- **Evidence**: `src/apps/desktop/src/app_state/settings.rs:104-105`. Search for `keyring` / `encrypt` in `src/` returns no matches (except unrelated relay E2E encryption).
- **Proposed fix**: (1) Short-term: use OS keyring crate. (2) Mid-term: AES-256-GCM with machine-derived key. (3) Long-term: env var injection, no disk storage.
- **Status**: active

### P1-3: Delete bypasses recycle bin

- **Symptom**: `delete_local_path` calls `fs::remove_file` / `fs::remove_dir_all` directly. Remote uses `rm -rf`. Deletions are irreversible.
- **Evidence**: `src/crates/execution/tool-execution/src/fs/delete_path.rs:49-64` (local), `:70-75` (remote `rm -rf`). No `trash` / `recycle` references in `src/`.
- **Proposed fix**: Use `trash` crate for local deletes. Add config option for recycle bin vs permanent. Remote: keep `rm` but add confirmation.
- **Status**: active

### P1-4: Mobile-web re-pairing has no guidance + ~~desktop Rust i18n mojibake~~

- **Symptom**: `PairingPage.tsx` has pairing logic but no re-pairing guidance when connection drops.
- **Evidence**: `src/mobile-web/src/pages/PairingPage.tsx` — no re-pair UI.
- **Proposed fix**: Add re-pair guidance UI to PairingPage.
- **Status**: active (mobile-web: frozen surface)

### P1-4b: ~~Desktop Rust i18n mojibake~~ (resolved)

- **Symptom**: GBK/UTF-8 corruption in desktop Rust Chinese strings (e.g. mojibake where "当前没有正在运行的回复" belonged).
- **Resolution**: Not present in the current codebase — grep for `褰` / `鈥` across `src/apps/desktop/src/` returns zero matches (verified 2026-07-22). The desktop code now carries proper UTF-8 Chinese strings (e.g. `"当前没有正在运行的回复"`, `"已排队，将在当前回复完成后发送"`, `"LLM 调用失败: {error}"`). The cited location was rewritten by `ad349f9` (desktop event bridge, 2026-07-17; found via `git log --follow -S "当前没有正在运行的回复"`); remaining fixes absorbed into the W3a-4 / D2j desktop rewrites (2026-07-18).
- **Status**: resolved (`ad349f9` + W3a-4 rewrites, verified 2026-07-22)

### P1-5: Relay server defaults to 0.0.0.0 with no authentication

- **Symptom**: Relay server defaults to `0.0.0.0:9700`, `api_key: None`, CORS `*`. `RELAY_API_KEY` env var exists but is optional.
- **Evidence**: `src/apps/relay-server/src/config.rs:30,41-42,63-67`. `routes/api.rs:32-72` — `AuthExtractor` only enforces when `api_key` is `Some`.
- **Proposed fix**: (1) Default bind to `127.0.0.1`. (2) Auto-generate API key on first run. (3) CORS default to `http://localhost:*`. (4) Print security warning if running unauthenticated on 0.0.0.0.
- **Status**: active (partially mitigated — `RELAY_API_KEY` available but off by default)

## P2 — Experience and operations

### P2-1: CLI has no release artifact + doctor false positives

- **Symptom**: Two `doctor` entry points (`acp_cli::print_doctor` + `management::print_doctor`). Checks may report false positives (checks process existence, not actual connectivity). No CLI binary release configuration in CI.
- **Evidence**: `src/apps/cli/src/acp_cli.rs`, `src/apps/cli/src/management.rs`, `src/apps/cli/src/main.rs` — `Commands::Doctor` + `McpAction::Doctor`. No release workflow for CLI binary.
- **Proposed fix**: (1) Unify doctor commands. (2) Add actual connection tests. (3) Add CLI binary to GitHub Release workflow.
- **Status**: active (CLI is frozen surface)

### P2-2: No single-instance lock — two app instances corrupt config

- **Symptom**: No single-instance / lock file mechanism in desktop app. Two instances share `~/.northhing/config/app.json` — last write wins, session state conflicts.
- **Evidence**: Search `single.*instance|lock.*file|already.*running` in `src/apps/desktop/` returns no matches. `save_app_settings` does not use `FILE_LOCKS` from persistence.rs.
- **Proposed fix**: (1) Create lock file on startup (`~/.northhing/app.lock`). (2) Or use single-instance plugin. (3) Make `save_app_settings` use file lock.
- **Status**: active

### P2-3: Context compression has no visible marker

- **Symptom**: `ContextCompressionStarted` / `Completed` events are defined and emitted, but desktop `event_bridge.rs` and CLI `run.rs` do not handle them. Users see no indication when compression occurs.
- **Evidence**: `compress_run.rs:53-63` emits events. `event_bridge.rs` — no `ContextCompression` match. `run.rs` — no `ContextCompression` handling.
- **Proposed fix**: (1) Handle compression events in `event_bridge.rs` — show temporary banner. (2) CLI: print `[context compressed: N → M tokens]`. (3) Insert system message in history.
- **Status**: active

### P2-4: Snapshot/log cleanup never scheduled

- **Symptom**: `CleanupService` fully implemented (`cleanup_all`, `cleanup_temp_files`, `cleanup_old_logs`, `cleanup_oversized_cache`) but never instantiated or called. `spawn_cleanup_task` cleans expired sessions, not files.
- **Evidence**: `src/crates/assembly/core/src/infrastructure/storage/cleanup.rs:54-76` — full implementation. No code creates `CleanupService` instance. `snapshot_system.rs:446` — `cleanup_orphaned_snapshots` exists but unscheduled.
- **Proposed fix**: (1) Spawn periodic cleanup task on app startup (e.g. every 24h). (2) Trigger cleanup on session deletion. (3) Include orphaned snapshots in `CleanupService`.
- **Status**: active (infrastructure ready, missing scheduler)

### P2-5: Failed turns leave no persistent trace in history

- **Symptom**: `DialogTurnFailed` event handled in event_bridge.rs (sets temporary error) and run.rs (displays error), but failure reason is not persisted to conversation history. After refresh, the failure is invisible.
- **Evidence**: `event_bridge.rs:222-260` — `set_session_error` + `set_inline_error`, not written to message list. `turn_persist.rs` persists turn metadata but not failure reason in message list.
- **Proposed fix**: (1) Insert failure reason as system message in conversation history. (2) Mark failed assistant messages with error badge. (3) CLI: show `[失败] {error}` in history rendering.
- **Status**: active

### P2-6: Event queue silently drops events when full

- **Symptom**: `EventQueue` drops new events when full (`max_queue_size: 10000`), logs `warn!`, returns `Ok` (false success). `StreamEventSink::enqueue` ignores return value with `let _ =`. Critical events (e.g. `DialogTurnFailed`) may be silently lost.
- **Evidence**: `src/crates/assembly/core/src/agentic/events/queue.rs:85` — drops + returns `Ok`. `queue.rs:127` — `let _ = EventQueue::enqueue(...)`.
- **Proposed fix**: (1) Return `Err` when full, let caller decide. (2) Never drop `Critical` priority events. (3) `StreamEventSink` should handle `Err` with error-level log.
- **Status**: active

### P2-7: subagent_ports test family is environment-sensitive (assumes no-LLM microsecond failure)

- **Symptom**: tests_cancel / tests_timeout / tests_concurrent / tests_error / tests_parent_chain assume dev environment has no LLM and init_turn fails in microseconds; on machines with available LLM configuration these tests fail reliably (unrelated to code correctness).
- **Evidence**: `src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/tests_cancel.rs:7-12` (test doc comment self-documents the assumption); `docs/plans/2026-07-21-three-track-refinement-plan.md` §v0.2.4 B5 retro section.
- **Proposed fix**: Inject a deterministic fake AI backend (独立测试基建单), replacing the implicit assumption on local machine configuration.
- **Status**: active

### P2-8: kernel_facade/mod.rs god file (2213 lines)

- **Symptom**: `src/crates/assembly/core/src/kernel_facade/mod.rs` is 2213 lines, exceeding the AGENTS.md house rule #3强制拆分线 of 1000 lines.
- **Proposed fix**: Split into modules per R-family conventions (lifecycle / dto / api / tests); already in backend queue.
- **Status**: active

## Change Protocol

- **New entry**: Add with next available ID, include evidence (file:line), proposed fix, and status.
- **Resolved**: Mark as `resolved` with commit reference. Do not delete entries.
- **Status change**: Update status field (active / frozen / resolved) with date and reason.
