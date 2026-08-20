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
- **Status**: `resolved` — fixed by `9be74ec` (Task 7 / H-9 desktop settings atomic落盘). Ledger flipped retroactively per `.superpowers/sdd/final-review.md` §3.2.

### P1-2: API key stored in plaintext

- **Symptom**: `ProviderConfig.api_key` stored as plaintext string in `app.json`. No keyring, encryption, or obfuscation. Code comment: "Stored in plaintext in app.json. Never logged."
- **Evidence**: `src/apps/desktop/src/app_state/settings.rs:104-105`. Search for `keyring` / `encrypt` in `src/` returns no matches (except unrelated relay E2E encryption).
- **Proposed fix**: (1) Short-term: use OS keyring crate. (2) Mid-term: AES-256-GCM with machine-derived key. (3) Long-term: env var injection, no disk storage.
- **Status**: `resolved` (2026-08-04, `fix/p1-security-0804`, C3). `ProviderConfig.api_key` migrated to OS keyring via `keyring` crate v4.1.6. See below for details.
- **Resolution details**: `KeyringBackend` trait with `ProductionKeyring` (wraps `keyring` crate) and `MockKeyring` (Mutex-guarded HashMap for tests). Sentinel `"__kr__"` replaces plaintext on disk after migration. Load-time migration (`keyring_migrate_providers` at `io.rs:79-113`) moves plaintext keys to keyring atomically; fail-closed on keyring error. Update path also migrates newly entered keys before save (`io.rs:138-148`). `resolve_api_key()` unified entry point reads from keyring when sentinel present (`keyring.rs:196-200`). All `provider.api_key` call points updated: `sync.rs` (`provider_to_ai_model_config`), `provider_test.rs` (test callback). No log prints any API key value (grep verified). 15 keyring unit tests (keyring.rs) + 4 IO integration tests (io_tests.rs), verified by grep `^[[:space:]]*#\[test\]` / `^[[:space:]]*#\[tokio::test\]`.

### P1-3: Delete bypasses recycle bin

- **Symptom**: `delete_local_path` calls `fs::remove_file` / `fs::remove_dir_all` directly. Remote uses `rm -rf`. Deletions are irreversible.
- **Evidence**: `src/crates/execution/tool-execution/src/fs/delete_path.rs:49-64` (local), `:70-75` (remote `rm -rf`). No `trash` / `recycle` references in `src/`.
- **Proposed fix**: Use `trash` crate for local deletes. Add config option for recycle bin vs permanent. Remote: keep `rm` but add confirmation.
- **Status**: `resolved` — trash crate v5.2.6 integrated; `DeleteLocalPathRequest.permanent` field; fail-closed: trash error returns Err; test seam with thread-local mock; 5 new unit tests + 1 integration test updated (trash default, permanent bypass, fail-closed, dir, nonexistent paths).

### P1-4: Mobile-web re-pairing has no guidance + ~~desktop Rust i18n mojibake~~

- **Symptom**: `PairingPage.tsx` has pairing logic but no re-pairing guidance when connection drops.
- **Evidence**: `src/mobile-web/src/pages/PairingPage.tsx` — no re-pair UI.
- **Proposed fix**: Add re-pair guidance UI to PairingPage.
- **Status**: `resolved` — mobile-web 面已整删（T2-2 C6 commit `646f93d`），条目随删除关闭；`docs/architecture/backend-roadmap.md:118` 已预先声明此关闭方式。

### P1-4b: ~~Desktop Rust i18n mojibake~~ (resolved)

- **Symptom**: GBK/UTF-8 corruption in desktop Rust Chinese strings (e.g. mojibake where "当前没有正在运行的回复" belonged).
- **Resolution**: Not present in the current codebase — grep for `褰` / `鈥` across `src/apps/desktop/src/` returns zero matches (verified 2026-07-22). The desktop code now carries proper UTF-8 Chinese strings (e.g. `"当前没有正在运行的回复"`, `"已排队，将在当前回复完成后发送"`, `"LLM 调用失败: {error}"`). The cited location was rewritten by `ad349f9` (desktop event bridge, 2026-07-17; found via `git log --follow -S "当前没有正在运行的回复"`); remaining fixes absorbed into the W3a-4 / D2j desktop rewrites (2026-07-18).
- **Status**: resolved (`ad349f9` + W3a-4 rewrites, verified 2026-07-22)

### P1-5: Relay server defaults to 0.0.0.0 with no authentication

- **Symptom**: Relay server defaults to `0.0.0.0:9700`, `api_key: None`, CORS `*`. `RELAY_API_KEY` env var exists but is optional.
- **Evidence**: `src/apps/relay-server/src/config.rs:30,41-42,63-67`. `routes/api.rs:32-72` — `AuthExtractor` only enforces when `api_key` is `Some`.
- **Proposed fix**: (1) Default bind to `127.0.0.1`. (2) Auto-generate API key on first run. (3) CORS default to `http://localhost:*`. (4) Print security warning if running unauthenticated on 0.0.0.0.
- **Status**: resolved (2026-08-04, `fix/p1-security-0804`). See details below.
- **Resolution details**: Default bind changed to `127.0.0.1:9700` (`src/apps/relay-server/src/config.rs`). `RELAY_BIND` env var overrides the full socket addr. Auto-generates API key on first run at `~/.northhing/relay/api_key` with atomic write (tmp+rename). `RELAY_API_KEY` env always takes priority. Non-loopback bind without key → `from_env` returns error (fail-closed). CORS defaults to localhost-origin predicate (any port) instead of `*`; `RELAY_CORS_ALLOW_ORIGINS` env var overrides. CORS `cors_allow_origins` config field now wired to the axum router (was previously unused — `build_relay_router` used hardcoded `CorsLayer::permissive()` at `relay-core/src/lib.rs:168`). Embedded relay (P1-7) remains open mode per product requirement.

### P1-7: Embedded relay open mode — 0.0.0.0 with no API key (LAN pairing product requirement)

- **Symptom**: `start_embedded_relay` binds `0.0.0.0:{port}` and passes `None` to `build_relay_router`, leaving pair/command endpoints open. This is a product-required open surface for LAN/ngrok mobile phone pairing — the pairing protocol itself must carry an out-of-band key.
- **Evidence**: `src/crates/assembly/core/src/service/remote_connect/embedded_relay.rs:28-33` (passes `None`), `:44-46` (binds `0.0.0.0:{port}`).
- **Proposed fix**: Thread an API key through the embedded relay path, gated by the pairing protocol handshake (design task). Options: (1) Generate ephemeral key on each desktop start and include in QR code/pairing URL. (2) Use a configurable key from desktop settings. (3) Pairing-level token exchange before relay commands.
- **Status**: `resolved` — relay-server + relay-core 已整删（T2-2 C5 commit `f6a011b`，PEND-1），embedded relay 入口不复存在，条目随删除关闭。

### P1-8: MCPServerConfig.env serialized as plaintext in app.json

- **Symptom**: `MCPServerConfig.env` (`HashMap<String, String>`) stores environment variables for stdio subprocesses as plaintext in `app.json`. These env vars commonly carry credentials (e.g. `OPENAI_API_KEY=sk-xxx`, `AWS_ACCESS_KEY_ID=...`), creating the same plaintext-on-disk risk as P1-2.
- **Evidence**: `src/apps/desktop/src/app_state/settings/types.rs:161-162` — `pub env: HashMap<String, String>` in `MCPServerConfig`. The field is serialized/deserialized without any encryption or keyring-backed indirection.
- **Proposed fix**: Defer to a future wave — the same `KeyringBackend` pattern from P1-2 (C3) can be reused: a per-variable sentinel or a single keyring entry per MCP server holding the full env block. C3 scope is strictly `ProviderConfig.api_key`; this concern is registered per brief §7 ("发现即登记，不擅自改").
- **Status**: active (discovered by C3 review 2026-08-04, registered as concern per brief §7)

### P1-6: DeleteFileTool needs_permissions()=false — 删除（含 remote rm -rf）绕过确认门

- **Symptom**: `DeleteFileTool` 显式覆写 `needs_permissions()` 返回 `false`（`delete_file_tool.rs:115-117`），导致本地与 remote 删除均不走 tool framework 的确认通道。`tool_confirmation.rs:55` 在 `!tool_needs_permission` 时短路为 `ToolConfirmationPlan::Skip`，`exec_retry.rs:176-232` 不创建确认通道。remote 删除路径（`build_remote_delete_command` → `rm -rf`）不可逆且无用户确认。
- **Evidence**: `src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs:115-117` — override `fn needs_permissions(...) -> bool { false }`。`src/crates/execution/agent-runtime/src/tool_confirmation.rs:55` — `!tool_needs_permission` 短路。`src/crates/assembly/core/src/agentic/execution/round_subhandlers/process_result.rs:269-287` — `requires_permission=false → needs_confirm=false`。
- **Proposed fix**: (1) 让 remote 删除路径恢复确认门（按 `ToolPathOperation::Delete` 维度判断 `needs_permissions`）。(2) 或按 `recursive` / `remote` 维度细分 `needs_permissions`（递归 remote 删除必须确认）。(3) 本地删除已由 P1-3 回收站缓解，但 `permanent=true` 路径同样无确认门。
- **Status**: active (discovered by C1 review 2026-08-04)

## P2 — Experience and operations

### P2-1: CLI has no release artifact + doctor false positives

- **Symptom**: Two `doctor` entry points (`acp_cli::print_doctor` + `management::print_doctor`). Checks may report false positives (checks process existence, not actual connectivity). No CLI binary release configuration in CI.
- **Evidence**: `src/apps/cli/src/acp_cli.rs`, `src/apps/cli/src/management.rs`, `src/apps/cli/src/main.rs` — `Commands::Doctor` + `McpAction::Doctor`.
- **Proposed fix**: (1) Unify doctor commands. (2) Add actual connection tests. (3) Add CLI binary to GitHub Release workflow.
- **Status**: `partial` — release artifact resolved (`.github/workflows/cli-package.yml` exists with cross-platform matrix + SHA256 + GitHub Release upload). Doctor unification still active (2 entry points remain, no connection tests).

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
- **Status**: resolved (`b15ad46` + `792ff8d`, 2026-07-22: split into 14 files, mod.rs 73 lines, judge-m3 PASS)

### P2-9: core-boundaries checker fully broken (34 stale rule paths + pre-existing failure backlog)

- **Symptom**: `node scripts/check-core-boundaries.mjs` crashes with ENOENT on 34 rule paths referencing pre-split god files (now directories) and absent `src/web-ui`. Behind the crash sit dozens of accumulated boundary failures (crate layout for relay-core/agent-dispatch/test-support/cli-internal, services-integrations optional-dep gates, desktop-tauri product-full coverage, etc.) — the checker is not wired into CI, so rot went unnoticed.
- **Evidence**: 2026-07-22 session crash output; partial repair `7bbe512` (deleted crates dropped, `service_agent_runtime` rules remapped to `sar_*.rs` split); `scripts/core-boundaries/self-test.mjs` is orphaned (not in package.json or workflows).
- **Proposed fix**: Epic, three parts — (1) finish per-path remap per `7bbe512` paradigm (forbidden → `forbiddenContentUnderRules` dir entries; required → per-file split by symbol location; delete absent web-ui rules); (2) triage pre-existing failures into rule updates vs repo fixes (needs architecture decisions, e.g. desktop-tauri coverage, relay-core layout); (3) wire into CI so it cannot rot again. Note: C4 judge_gate zero-dep-edge rule is already added and structurally verified (agent-runtime Cargo.toml has no northhing-core dep).
- **Status**: `resolved` — violations cleared to 0 (`d621b29`, 2026-07-23: final 17 violations → 0; `node scripts/core-boundaries/checker.mjs` exits 0 with zero violations, verified 2026-07-27). Stage 1-2 history: 230 → 37 → 0 across 2026-07-23. **Stage 3 resolved**: `7705c3f` (2026-07-28) — `core-boundaries` job added to `ci.yml` (required check, runs on PR + push to main). Entry-point wrapper `check-core-boundaries.mjs` invokes `runCoreBoundaryCheck()`. P2-9 fully resolved.

### P2-10: 5 new god-files (house rule #3), 2 over 1000 lines, none registered or justified

- **Symptom**: House rule #3 requires production `.rs` > 1000 lines to be split or carry `// allow-god-file`; > 800 raises review pressure. Five files exceed 800 with no justification comment and no ledger entry; two exceed 1000 (mandatory split).
- **Evidence**: `src/apps/desktop/src/app_state/settings.rs` (~1488 lines), `src/apps/desktop/src/app_state/callbacks_settings.rs` (~1100 lines) — both > 1000, no `allow-god-file`. `cli/ui/theme.rs` (~854), `src/apps/desktop/src/app_state/callbacks_lifecycle.rs` (~834), `src/crates/assembly/core/src/agentic/judge_gate/mod.rs` (~813, newly created in C4 Phase 0 already over the line). Found by external review 2026-07-23 + orchestrator scan.
- **Proposed fix**: Split the two > 1000 files (settings panel is a recurring split source — consider a settings/ module family); for the three > 800, split or add `// allow-god-file` with reason. Register a split plan.
- **Status**: `resolved` — 2/2 >1000 files split (`ecbe76e`, 2026-07-23) + 3/3 >800 files registered with `// allow-god-file` (`456b696`, 2026-07-23: theme.rs 855L, callbacks_lifecycle.rs 832L, judge_gate/mod.rs 822L). Verified 2026-07-27: zero unregistered >800 files in src/.

### P2-11: judge_gate ApprovedGateReceipt consumed-set is in-process; restart can reuse a consumed receipt

- **Symptom**: The set of consumed gate receipts lives in process memory. If a `promote` consumes a receipt but the persisting write fails (power loss / crash), a restart resets the consumed set, allowing the same receipt to be replayed — breaking the consume-once guarantee that backs red line #2 (un-gated artifacts must not appear where the agent can auto-hit them).
- **Evidence**: External review 2026-07-23 §四.6; `src/crates/assembly/core/src/agentic/judge_gate/` receipt consumption path (consumed set not persisted — verify exact location when fixing).
- **Proposed fix**: Persist the consumed-receipt set (append-only, per red line #4) so consumption survives restart; or make promote idempotent + write-ahead so a failed promote cannot be replayed into a different outcome.
- **Status**: resolved (`47b6202`, 2026-07-23: `receipt_store.rs` — append-only JSONL at `data_dir/judge-gate/consumed_receipts.jsonl`; LazyLock init replays log; persist on consume/release; best-effort non-blocking; 26 judge_gate tests pass)
- **Note (2026-08-18 T2-2b)**: 适配层整体已删（含 `receipt_store.rs` 的 append-only JSONL + LazyLock 重放实现，`47b6202`）；**教训移交 TH-5（T3-8）**：consume-once 凭证必须 append-only 持久化 + 初始化重放，否则重启可重放已消费凭证（原症状描述见本条 Symptom）。

### P2-12: episodes "agent does not read" boundary is convention-layer, not structure-layer (HIGH PRIORITY)

- **Symptom**: C2's invariant "the agent does not read its own episodes for decisions" (anti self-validation loop) is enforced only by convention — no code reads episodes into the prompt today, but nothing structurally prevents it. A future prompt-builder edit could wire episodes in and silently open the self-validation loop, undermining C4's whole point.
- **Evidence**: External review 2026-07-23 §1 / §四.5; the episodes store under `src/crates/assembly/core/src/agentic/` has no read-side guard.
- **Proposed fix**: Upgrade to structure-layer — a cargo boundary assertion or path blacklist (like the core-boundaries checker) that fails the build if any prompt-builder path imports the episodes store. Make it as physically hard to break as C4's receipt gate.
- **Status**: resolved (2026-07-23: added `forbiddenContentUnderRules` entries in `scripts/core-boundaries/rules/source/forbidden-rules.mjs` — `read_episodes` and `episodes::store::read` forbidden under `agentic/agents/` and `agentic/execution/`; checker + self-test pass; kernel_facade/memory.rs UI display path unaffected)

### P2-13: C1 identity rewritten but agentic_mode.md behavior section not tuned

- **Symptom**: C1 rewrote the identity (IDE tool -> independent colleague): agentic_mode.md front half says "not an IDE, not a coding tool", but the back half is still large blocks of programming guidance. Identity and behavior are split.
- **Evidence**: External review 2026-07-23 §三 / high-priority.3; the agentic_mode.md identity section vs its programming-guidance section.
- **Proposed fix**: Reconcile the behavior section with the new identity — reframe the programming guidance for the "independent colleague" stance or trim it; resolve the "not a coding tool" vs coding-guidance contradiction deliberately.
- **Status**: resolved (2026-07-23) — identity removed from agentic_mode.md (capability layer); self-cognition is a separate persona layer generated at first entry (see docs/design/2026-07-23-self-cognition/first-entry-design.md); "Doing tasks" reframed as conditional

### P2-14: C3 facts dedup is exact-text (fragile); confidence all Med / scope all Workspace (paths unimplemented)

- **Symptom**: facts.jsonl dedup uses exact text match — cannot absorb whitespace/wording variants, so the store bloats with near-duplicates. confidence is always Med and scope always Workspace; the High/Low/Global production paths are not implemented.
- **Evidence**: External review 2026-07-23 §四.4 / §四.8; C3 facts distillation code.
- **Proposed fix**: Normalize before dedup (or similarity-based dedup); implement confidence/scope derivation paths or remove the unused enum variants.
- **Status**: active (low priority)

### P2-15: P1-C3 merged to main while the desktop crate did not compile (process defect)

- **Symptom**: After P1-C3 (keyring-backed API key storage) landed on main, `cargo check -p northhing` failed at the baseline: keyring 4.1.6 raises `compile_error!("At least one of the features 'v1' or 'cli' must be enabled")`, so the desktop crate had never compiled since that merge. The C3 report itself (task-c3-report.md lines 66-71) admitted the desktop verification was not run, and the 2026-08-04 handoff carried a stale "desktop 98/98" figure from before C3.
- **Evidence**: Discovered 2026-08-05 while dispatching Task B3 of the backend follow-ups round; fixed by commit `b0bfe43` (keyring `v1` feature + 3 API/`Lazy` compile fixes + one test import path, zero behavior change, judge-verified line by line). New desktop baseline: `cargo test -p northhing --lib` = 118/118.
- **Root cause (process)**: a security-sensitive change was accepted on a report whose verification section was incomplete, and the round handoff reused an older desktop test figure instead of a fresh measurement.
- **Proposed fix**: gate it structurally — `cargo check -p northhing` must pass before any branch merges to main (recorded as housekeeping rule 6 in `AGENTS.md` / `AGENTS-CN.md`, 2026-08-06), and a round handoff must not carry forward a verification baseline it did not measure itself.
- **Status**: resolved (2026-08-17, T2-1: `cargo check --workspace` in CI includes `northhing` and `northhing-cli`; code defect resolved in `b0bfe43`; process gate recorded in housekeeping rule 6).

### P2-16: `ConfigManager::save_config` writes the whole config file non-atomically

- **Symptom**: `save_config` writes the global config with a plain whole-file write, so an interrupted write can leave a truncated / partial `app.json`.
- **Evidence**: Task B1 review Minor-1 (backend follow-ups round, 2026-08-05); Wave1 final review §5 triage ruled it a separate debt item (out of FU-1 scope).
- **Proposed fix**: route the write through the `json_store::write_atomic` pattern (temp file + rename), matching the settings/vault write paths.
- **Status**: active

### P2-17: `init_once_with` double-checked-lock skeleton is duplicated between core config and AI factory

- **Symptom**: `client_factory.rs` now owns a private `init_once_with` helper implementing the double-checked-locking init skeleton, while `service/config/global.rs` `GlobalConfigManager::initialize` still hand-rolls the same pattern with its own `INIT_MUTEX`.
- **Evidence**: Task B4 review Minor-3 + Wave1 final review §5 (2026-08-06), commit `50b0f44`.
- **Proposed fix**: if a third caller appears, lift the helper into a shared sync utility module and migrate both call sites; not worth it at two call sites.
- **Status**: active (low priority)

### P2-18: `LspManager::uninstall_plugin` has no production caller

- **Symptom**: the uninstall path (fixed in FU-2 so it stops servers by resolved language keys) is currently unreachable from production code — only tests call it.
- **Evidence**: Task B2 review observation + Wave1 final review §5 (2026-08-06), commit `7a4bdca`.
- **Proposed fix**: either wire plugin uninstall into the product surface or record it explicitly as an API kept for a planned surface; also note `stop_server` always returns `Ok`, which makes the new warn branch unreachable.
- **Status**: active (low priority)

### P2-19: `src/apps/server/README.md:5-10` 包含 3 条指向已删 relay-server 的悬空链接

- **Symptom**: `src/apps/server/README.md:5-10` 中存在 3 条指向 `src/apps/relay-server` 的链接与描述引用，但 relay-server 已在 T2-2 C5（commit `f6a011b`）整删。
- **Evidence**: `src/apps/server/README.md:5-10`。
- **Proposed fix**: server 为 frozen 面，留待 server 解冻时同步修整文档链接（来源：T2-2g review M-g-2）。
- **Status**: active (frozen surface)

### P2-20: `pnpm-workspace.yaml` 中注册了孤儿工作区 `desktop-tauri`

- **Symptom**: `pnpm-workspace.yaml` 中包含 `src/apps/desktop-tauri` 注册条目，但磁盘上该目录不存在（已随架构演进清理）。
- **Evidence**: `pnpm-workspace.yaml:5`。
- **Proposed fix**: 作为独立决策项处理，在后续工作区配置清理批次中移除（来源：T2-2h review F1/M-h）。
- **Status**: active

### P2-21: MiniApp 契约层三处 serde/wire 残留（零构造零生产者，反序列化兼容悬置待决）

- **Symptom**: MiniApp 子系统整删后，契约层保留了三处 serde/wire 残留：`core-types/src/surface.rs:52` `RuntimeArtifactKind::MiniApp`、`services-core/src/session/session_metadata.rs:27` `SessionRelationshipKind::Miniapp`、`services-core/src/session/lineage.rs:19` `"miniapp"` tag。当前代码中零构造、零生产者，但直接删除存在旧会话/工件数据反序列化兼容风险。
- **Evidence**: T2-2 MiniApp recon Q7 (`.superpowers/sdd/task-t2-2-miniapp-recon.md`)；`rg` 实测全仓零业务构造。
- **Proposed fix**: 整删三处残留（曾悬置待用户拍板；经实测零生产者，磁盘旧数据不可能含这些值，风险≈0）。
- **Status**: `resolved` — 用户 2026-08-19 拍板删除，T2-2p 执行完毕，commits 见 git log。

## Change Protocol

- **New entry**: Add with next available ID, include evidence (file:line), proposed fix, and status.
- **Resolved**: Mark as `resolved` with commit reference. Do not delete entries.
- **Status change**: Update status field (active / frozen / resolved) with date and reason.
