# R3 — Services + Execution + Adapters Audit

Branch: `main` @ `74ea164` · Tree: clean · Layer scope: services/*, execution/*, adapters/*

## Verdict: **needs attention**

The three layers are well-architected overall. Secrets vaults are fail-closed with
atomic writes (H-5 hardening, commit `88c719a`). The SSE parser, tool-call
accumulator, and JSON-repair paths handle malformed provider output with multiple
fallback levels. The LSP plugin manager's transactional uninstall + symlink
defense-in-depth is solid. The `unsafe` surface is small (3 blocks across 2 files)
and limited to sound FFI (`#[link]`, `libc::setsid`/`libc::setpgid`/`libc::killpg`,
`win32job::Job` RAII).

The hot-spots live in **process lifecycle** and **resource accounting**:
two call sites drop children correctly (`workspace_search/flashgrep/client.rs`,
`terminal/exec/output.rs`); five+ call sites do not (LSP, MCP, `system/command.rs`,
`git/utils.rs`, several `assembly/core/util` paths). One bounded-by-design in-memory
collector is currently unbounded. One secret file is written non-atomically while
its containing vault is hardened.

No findings are user-blocking on the happy path; the Important items are
**leak/robustness** issues that surface under crash, force-kill, or pathological
input.

## Findings

### F1 · Important — LSP child can orphan when stop_server is bypassed or shutdown fails

- **Where**: `src/crates/assembly/core/src/service/lsp/process_spawn.rs:49-52`,
  `src/crates/assembly/core/src/service/lsp/process.rs:46,67-71`,
  `src/crates/assembly/core/src/service/lsp/process_runtime.rs:42-117`
- **What**: `cmd.spawn()` in `process_spawn.rs` does NOT set `kill_on_drop(true)`
  and does NOT apply `process_manager::configure_process_group`. `LspServerProcess`
  holds `child: Arc<RwLock<Child>>` and `Drop` only logs (`process.rs:67-71`).
- **Why it matters**: `stop_server` (`manager.rs:231-243`) calls
  `process.shutdown().await` which catches Err as a log and removes the map entry.
  If `shutdown()` fails (server already crashed, hanging on `exit` notification,
  network/lsp-protocol corruption), the process entry is dropped but the OS-level
  child lives on. The 3 spawned tokio tasks (`start_read_task`, `start_stderr_task`,
  `start_notification_task`) keep running until they see EOF on stdout/stderr, but
  if stdout is open in some pathological state they loop on `MAX_CONSECUTIVE_TIMEOUTS`
  indefinitely (`process_runtime.rs:44-97`). Each orphaned LSP child ties up a real
  PID and any subprocesses it may have forked (e.g. TS server kids, language tools
  for monorepos).
- **Compare to reference pattern**: `services-integrations/src/workspace_search/flashgrep/client.rs:430-431`
  applies both `kill_on_drop(true)` AND `configure_process_group(&mut command)`
  plus a `Drop` impl that calls `spawn_child_process_tree_cleanup` (line 667-674).
- **Fix direction**: (1) Add `.kill_on_drop(true)` and `process_manager::configure_process_group(&mut cmd)`
  to `process_spawn.rs:43-52`. (2) Replace the trivial `Drop` in `process.rs:67-71`
  with a `start_kill()` on the child if the handle is still live. (3) Track
  JoinHandles for the three spawned tasks and abort them in Drop or on `stop_server`.
- **Effort**: M

### F2 · Important — MCP server `stop()` doesn't kill process tree; Drop is best-effort only

- **Where**: `src/crates/services/services-integrations/src/mcp/server/process.rs:244-251,396-401`
- **What**: `stop()` calls `child.kill().await` on the direct child only, never
  `terminate_child_process_tree`. `Drop` calls `child.start_kill()` (non-blocking,
  best-effort). Process group is also not configured at spawn (line 85).
- **Why it matters**: MCP servers commonly spawn helper subprocesses (`npx`-wrappers,
  language runtimes, browser automation). After `stop()` (or Drop on the
  `MCPServerProcess`), the helpers become orphans. `max_restarts=3` (`process.rs:49`)
  means a flapping server can restart 3× and each restart can leave subprocesses
  behind without bumping the counter.
- **Compare to reference pattern**: `terminal/exec/output.rs:421-422,432,483`
  uses `configure_pipe_process_group`, captures the pgid, and on kill sends
  `SIGKILL` to the whole group.
- **Fix direction**: (1) Apply `process_manager::configure_process_group` in
  `start()` (line 85). (2) Replace `child.kill().await` (line 245) with
  `process_manager::terminate_child_process_tree(&mut child, graceful_timeout)`,
  matching the pattern at `flashgrep/client.rs:539`. (3) Consider not gating restart
  counter on a subprocess-leak case separately.
- **Effort**: S

### F3 · Important — Vault key file write is non-atomic, asymmetric with hardened vault content

- **Where**: `src/crates/services/services-integrations/src/remote_ssh/password_vault.rs:55-68`
  and `src/crates/services/services-integrations/src/mcp/auth.rs:112-127`
- **What**: Both vaults encrypt-then-atomic-write the value file via
  `JsonFileStore::write_atomic` (`password_vault.rs:123-137`, `auth.rs:184-198`),
  with `.bak` backup + 0o600 restore. But the 32-byte master key is written via
  `tokio::fs::write(&self.key_path, key.as_slice())` — a plain non-atomic write
  (`password_vault.rs:57`, `auth.rs:114`). The 0o600 chmod happens AFTER the data
  is on disk (TOCTOU window in the millisecond range between write and chmod).
- **Why it matters**: A crash between the key bytes going to disk and the chmod
  leaves a partially-written key file. The next `ensure_key` read returns a
  short buffer, `bytes.len() != 32` (line 45-47) fails closed with
  `anyhow::bail!("invalid ssh password vault key length")`. Result — every
  previously-stored password becomes permanently inaccessible; vault content on
  disk is now dead data. `ensure_key` is called from `store`/`migrate_entry`
  but not `load` (`load` reads the existing key directly at `password_vault.rs:152-156`),
  so the lock-on-error behavior is asymmetric.
- **Compare to**: `JsonFileStore::write_atomic` (`services-core/src/json_store.rs:136-200`)
  is the hardened pattern right next door — tmp file with nonce + rename with
  Windows retry + PermissionDenied fallback. The same primitive is available
  via the `northhing_services_core::JsonFileStore` import already in scope
  (line 10).
- **Fix direction**: Replace the key-write block in both vaults with a
  `JsonFileStore.write_atomic` call on the key file (or extract a tiny
  `write_secret_bytes_atomic` helper), and `fsync` the directory for durability.
  Chmod can be tightened via `set_permissions` BEFORE the rename (tmp file
  inherits 0o600 from the parent dir if the parent dir is set correctly, or
  explicitly chmod the tmp before rename).
- **Effort**: S

### F4 · Important — SSE log collector buffer is unbounded while flush output is capped

- **Where**: `src/crates/execution/agent-stream/src/sse_log_collector.rs:14,26-28`
  + `src/crates/execution/agent-stream/src/stream_processor.rs:419-434`
- **What**: `SseLogCollector` holds `buffer: Vec<String>` with no cap. The
  emitted log is head/tail-capped by `SseLogConfig.max_output` (lines 50-77),
  but `SseLogConfig::default()` is `max_output: None` (`types.rs:84-88`) and
  `stream_processor.rs:420` explicitly uses `SseLogConfig::default()` with the
  comment "No limit for now". Each incoming SSE chunk is pushed into the buffer
  via a spawned drain task (line 425-429).
- **Why it matters**: A model emitting at max token rate can produce thousands of
  small chunks per stream (Anthropic tool-use deltas, OpenAI 1-token deltas).
  For a 100k-token output that's 100k String allocations held in memory until
  stream end (the buffer is dropped at function exit, only flushed on error).
  Multiple concurrent streams compound. The buffer being unbounded on a non-error
  path also defeats the "diagnose on error" rationale — a noisy stream that
  succeeds still allocates, and a noisy stream that fails could OOM before the
  error path runs.
- **Fix direction**: Either (a) keep a bounded ring buffer with capacity based on
  `SseLogConfig.max_output` and overwrite on overflow, or (b) stream the raw
  events directly to the logger (no buffer) and only keep the last N for the
  on-error flush. Same SseLogConfig shape — `max_output` already exists.
- **Effort**: S

### F5 · Minor — `process_group` + `kill_on_drop` inconsistent across `create_tokio_command` call sites

- **Where**: Missing at
  `src/crates/assembly/core/src/service/lsp/process_command.rs:125,142,178,187,211`,
  `src/crates/services/services-integrations/src/mcp/server/process.rs:85`,
  `src/crates/services/services-core/src/system/command.rs:280`,
  `src/crates/services/services-integrations/src/git/utils.rs:201`,
  `src/crates/assembly/core/src/service/workspace/workspace_info_impl.rs:380`,
  `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/utilities.rs:227,239,274,295`
  and the matching `app_control.rs:425`. Present at
  `services-integrations/src/workspace_search/flashgrep/client.rs:431`,
  `terminal/exec/output.rs:422` (its own pipe-process-group variant).
- **What**: `process_manager::create_tokio_command` (services-core/process_manager.rs:108-127)
  only sets `CREATE_NO_WINDOW` on Windows; process-group isolation is the caller's
  responsibility via `configure_process_group` (line 171). Many call sites use the
  command for `output().await` (one-shot, parent waits) — for those the absence
  is benign. For `spawn()` callers (LSP, MCP, git long-poll, computer-use), it
  means children-forked-by-children survive parent termination.
- **Why it matters**: Not a hot-spot on the happy path (most call sites are
  short-lived `output()` calls). On long-running spawn sites this is the same
  risk as F1/F2 — addressed by fixing F1 and F2 individually. Listed separately
  because the inconsistency is a *pattern* problem, not a single bug.
- **Fix direction**: Either roll `configure_process_group` into
  `create_tokio_command` for the `spawn`-oriented callers (with a separate
  `create_command_one_shot` for `output().await`-style callers), or audit each
  spawn site and apply the helper. The flashgrep pattern
  (`kill_on_drop(true)` + `configure_process_group` + Drop-cleanup) should be
  the default.
- **Effort**: M

### F6 · Minor — `FileWatchService::create_watcher` re-spawns the entire pipeline on every watch/unwatch

- **Where**: `src/crates/services/services-integrations/src/file_watch/service.rs:87-154`
- **What**: Every `watch_path` (line 57-72) and `unwatch_path` (line 74-85) call
  calls `create_watcher()`, which builds a brand-new `notify::RecommendedWatcher`,
  re-subscribes ALL watched paths (line 100-110), AND spawns a new
  `tokio::task::spawn_blocking` (line 122) that holds the new `rx`. The previous
  watcher + previous rx are dropped, the previous task self-terminates on
  `RecvTimeoutError::Disconnected` (line 141).
- **Why it matters**: Not a true leak (old task exits via Disconnected), but
  every watch_path call (1) re-registers all paths through notify's
  ReadDirectoryChangesW / inotify backends and (2) leaves a brief window with 2
  background tasks alive. Inotify has OS-level FD limits; rebuilding the whole
  watcher wastes user-mode state. Drop of FileWatchService does not await the
  spawned task — if the parent process drops the service quickly, the task keeps
  running until next poll (50 ms by default).
- **Fix direction**: Track the JoinHandle of the spawn_blocking task; on next
  `create_watcher`, `handle.abort()` the previous one before spawning the new.
  Or refactor so the watcher is built once and `watcher.watch(path, mode)` /
  `watcher.unwatch(path)` are called incrementally (notify's `Watcher` trait
  supports this — see `service.rs:107-109`).
- **Effort**: S

### F7 · Minor — `uninstall_plugin` rollback on `stop_server` failure is dead code

- **Where**: `src/crates/assembly/core/src/service/lsp/manager.rs:128-134,231-243`
- **What**: Transaction claim (manager.rs:97-102) says "if any step after
  unregistering fails, the registration is rolled back". `stop_server` always
  returns `Ok(())` (line 231-243) — it `warn!`-logs failures but never
  propagates. So the rollback branch on line 131-132 is unreachable.
- **Why it matters**: A bug fix that relies on the rollback branch would be
  silently broken. The right behavior on shutdown failure is ambiguous: should
  we leave the registry missing the plugin (files still on disk, servers
  stopped) or re-register and leave the broken servers on disk? Today the
  answer is "leave both inconsistent" — file deletion in step 3 will fail
  loudly anyway (line 137-141), so users will see an error message.
- **Fix direction**: Either make `stop_server` actually return Err on shutdown
  failure (and let the rollback run), or remove the rollback call and update
  the docstring. Already tracked in `tech-debt-ledger.md` P2-18 — not a new
  finding, just a confirmed persistence.
- **Effort**: S

### F8 · Minor — `SSE Parsing Error` substring matching controls recoverability

- **Where**: `src/crates/execution/agent-stream/src/stream_processor.rs:480-481`
- **What**: `non_recoverable_stream_error = error_msg.contains("SSE Parsing Error")`.
  Upstream in `ai-adapters/src/stream/stream_handler/openai.rs:159` the prefix
  is hardcoded as `"SSE parsing error: ..."` (lowercase "parsing"). If any
  provider adapter renames the prefix or the capitalization changes, partial
  recovery silently flips to "no recovery" or vice versa. OpenAI responses
  handler uses `"Responses SSE parsing error: ..."` (responses.rs:280) which
  matches but Anthropic/Gemini adapters (same directory) may not.
- **Why it matters**: Behaviour-equivalence risk during future adapter edits.
  Low severity on its own (recovery is a UX nicety, not correctness), but a
  typed `StreamErrorKind` enum would be more durable than string sniffing.
- **Fix direction**: Introduce a `StreamParseError` variant in the AI adapters
  `Result<UnifiedResponse, _>` (or wrap anyhow with a custom error), and use
  `matches!(err.downcast_ref::<StreamErrorKind>(), Some(NonRecoverable))` in
  stream_processor.
- **Effort**: M

### F9 · Minor — `spawn_child_process_tree_cleanup` spawns a new tokio runtime in a fresh thread

- **Where**: `src/crates/services/services-core/src/process_manager.rs:228-245`
- **What**: Builds `tokio::runtime::Builder::new_current_thread().enable_all()`,
  runs the cleanup on it inside a fresh std::thread.
- **Why it matters**: Correct (separate thread = no runtime-within-runtime
  panic). But expensive: thread creation + runtime build per Drop, and the
  graceful-then-force sequence runs synchronously in the new thread instead of
  cooperating with any cancellation token the caller may have. The only current
  caller (`flashgrep/client.rs:672`) is in Drop, which doesn't have a token —
  acceptable but it locks in the pattern.
- **Fix direction**: Either (a) when called from Drop, use `child.kill().await`
  inline if a tokio runtime is already current (`Handle::try_current()`), or
  (b) when `kill_on_drop(true)` is set (which `flashgrep/client.rs:430` already
  does), this whole function is unnecessary — just drop the Child. Recommend
  (b): deprecate the helper.
- **Effort**: S

### F10 · Minor — `process_stream_with_options` spawns a detached SSE log drain task

- **Where**: `src/crates/execution/agent-stream/src/stream_processor.rs:425-429`
- **What**: `tokio::spawn(async move { while let Some(data) = rx.recv().await { ... } })`
  is detached (no JoinHandle stored in the surrounding scope).
- **Why it matters**: If the function returns early (e.g. from
  `graceful_shutdown_from_ctx`), the collector instance lives until the stream's
  `tx_raw_sse` is dropped upstream (when the bytes_stream ends), so the drain
  task does eventually exit. But if the upstream produces hundreds of streams in
  rapid succession with abnormal early returns, detached tasks accumulate until
  each `rx` closes. Bounded by stream lifetime, so probably fine — but worth
  noting alongside F4.
- **Fix direction**: Track JoinHandle and `abort()` in early-return paths, or
  rely on the bounded-by-F4 ring buffer to make the drain task cheap.
- **Effort**: S

## Sample coverage note

Deep-read (full file):
- `services-core/src/process_manager.rs` (249 L)
- `services-core/src/json_store.rs` (260 L)
- `services-integrations/src/remote_ssh/password_vault.rs` (353 L)
- `services-integrations/src/mcp/auth.rs` (523 L)
- `services-integrations/src/remote_ssh/manager.rs` (195 L)
- `services-integrations/src/mcp/server/process.rs` (402 L)
- `services-integrations/src/file_watch/service.rs` (385 L)
- `services-integrations/src/workspace_search/flashgrep/client.rs:400-692`
- `services-integrations/src/git/utils.rs:196-213`
- `services-core/src/session/metadata_store.rs` (529 L)
- `services-core/src/system/command.rs:270-318`
- `terminal/src/exec/platform.rs` (357 L)
- `terminal/src/services-integrations/src/lib.rs` (35 L, unsafe audit)
- `execution/agent-stream/src/stream_processor.rs` (635 L)
- `execution/agent-stream/src/stream_context.rs` (191 L)
- `execution/agent-stream/src/types.rs` (138 L)
- `execution/agent-stream/src/sse_log_collector.rs` (81 L)
- `execution/agent-stream/src/tool_call_accumulator.rs:1-100`
- `execution/agent-stream/src/tool_call_repair.rs` (108 L)
- `execution/agent-runtime/src/scheduler.rs` (131 L) + `scheduler/sched_state.rs:1-300`
- `execution/agent-runtime/src/runtime.rs` (280 L)
- `execution/agent-runtime/src/agents.rs` (443 L)
- `execution/agent-runtime/src/scheduled_job.rs` (216 L)
- `execution/runtime-services/src/lib.rs` (254 L)
- `assembly/core/src/service/lsp/process_spawn.rs` (89 L)
- `assembly/core/src/service/lsp/process.rs` (71 L)
- `assembly/core/src/service/lsp/process_command.rs` (219 L)
- `assembly/core/src/service/lsp/process_runtime.rs` (450 L)
- `assembly/core/src/service/lsp/registry.rs` (198 L)
- `assembly/core/src/service/lsp/plugin_loader.rs` (746 L)
- `assembly/core/src/service/lsp/manager.rs:60-260`
- `adapters/ai-adapters/src/stream/stream_handler/openai.rs` (333 L)
- `adapters/ai-adapters/src/stream/stream_handler/responses.rs` (682 L)

Skim + targeted grep:
- `services-integrations/Cargo.toml` (boundary check confirmation)
- `agent-runtime/src/post_call_hooks.rs`, `checkpoint.rs`, `user_questions.rs` (read briefly)
- `tool-contracts/src/*.rs` (skipped per instructions)
- `runtime-services/src/lib.rs` (skim)

Searched but found nothing of concern: `unsafe` keyword is limited to 3 sites
(`services-integrations/src/lib.rs:35` is a `#[link]` decl, `terminal/exec/platform.rs:37,179`
are libc FFI in pre_exec closure and `killpg` — both sound; the pre_exec runs in
the forked child between fork and exec where async-signal-safety rules apply and
setsid/setpgid is the documented correct usage).

Excluded per audit instructions: tech-debt-ledger.md P0/P1/P2 items, god-file
manifest entries, GNU/MSVC env issue, `russh-keys` optional-dep boundary
checker flag (tracked separately).