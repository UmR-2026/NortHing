# Startup-Hang Static Trace

Read-only investigation. Repo: `E:\agent-project\NortHing`. Suspect range:
`09a0c69` (W14-1c-4a init gate local-state refactor) and `c678e6b`
(W14-1c-4b C/D lock discipline + ENV_LOCK + RAII).

Verified facts observed at runtime:
- Window opens then `(Not Responding)`, sustained ~58 s CPU per 60 s wall.
- stdout tail stuck at `MCP: ... smoke-echo ... handshake ... MCP reconnect failed: attempt=5 next_retry_in=32s ...`.
- `.northhing/debug.log` receives no writes from the current process.
- A pre-W15-1 rollback binary hangs identically ⇒ pre-existing bug.

---

## Q1 — Does the init gate block UI startup until all MCP servers are ready?

**Conclusion: NO.** Init core does **not** wait on MCP servers. MCP
`initialize_all()` is launched as a detached task; `init_core()` returns as
soon as the other bootstrap steps finish.

**Evidence**

`src/crates/assembly/core/src/kernel_facade/lifecycle.rs:107-180`
(`init_core_inner`) — order of awaits:

1. L108 `initialize_global_config().await …`
2. L112 `AIClientFactory::initialize_global().await …`
3. L116-121 `init_agentic_system_with_queue_config(...).await …`
4. L125-131 wire `DialogScheduler` (`set_scheduler_notifier`,
   `set_round_injection_source`) — non-blocking fast checks.
5. L135-149 — `get_global_config_service().await` →
   `MCPService::new(cfg_svc)` (sync `Ok`/`Err`) →
   `set_global_mcp_service(...)` →
   **L140-144**:
   ```rust
   tokio::spawn(async move {
       if let Err(e) = mcp_service.server_manager().initialize_all().await {
           warn!("failed to initialize MCP servers: {e}");
       }
   });
   ```
   The `.await` is **inside** `tokio::spawn`, not `init_core_inner`.
6. L151-175 — workspace + skill-watch, also `tokio::spawn` for the slow
   `sync_watched_paths().await`.
7. L177 `self.set_coordinator(...)` (sync `OnceLock::set`).
8. L179 `Ok(())`.

The gate itself (`run_init_gate_with`, `lifecycle.rs:36-92`) does nothing
MCP-shaped. It is a 3-state `AtomicBool + AsyncMutex<InitState> + Notify`
with `state.lock().await` and `notify.notified().await`, returning as soon
as the inner future resolves. No polling, no `tokio::time::sleep`.

`09a0c69 --stat` shows the change is purely a refactor of the test code
that runs the gate against local `AtomicBool/AsyncMutex/Notify`
(`tests.rs:72`, `lifecycle.rs:25-30`). The report
`.superpowers/sdd/w14-1c-4a-report.md:1-30` confirms:

> "`run_init_gate` 瘦身为委托：`run_init_gate_with(&FACADE_READY, &INIT_STATE, &INIT_NOTIFY, init).await`. ... 状态机迁移、错误文案、Notify 唤醒、ready 置位时机、`info!` 日志逐行不变（diff 为纯参数化改名）。"

The three module-level statics (`FACADE_READY`, `INIT_STATE`, `INIT_NOTIFY`,
lines 20/94/95) keep their old production behaviour.

The desktop launcher path then resolves to the same body via:
`src/apps/desktop/src/main.rs:18-22`:
```rust
northhing_core::kernel_facade::kernel_facade()
    .init_core()
    .await
    .map_err(|e| anyhow::anyhow!("init_core failed: {e}"))?;
```

So `init_core` returns `Ok(())` after the core bootstrap; MCP server
readiness happens in a sibling task and is **not** a blocker.

**Confidence: HIGH.** Source is post-W14-1c-4a HEAD (current working tree
state read by `read`); 09a0c69 is documented as a name-only refactor.

---

## Q2 — Does the MCP reconnect loop have a CPU-spin path?

**Conclusion: NO.** Reconnect backoff is a real `tokio::time::interval`
sleep, gated by a per-server `next_retry_at` Instant. There is no busy
loop, no `std::sync::Mutex` spin, no `futures::busy_wait`.

**Evidence**

`src/crates/assembly/core/src/service/mcp/server/manager/reconnect.rs:17-25`
(`run_reconnect_monitor`):
```rust
let mut interval = tokio::time::interval(self.reconnect_policy.poll_interval);
loop {
    interval.tick().await;                              // real async sleep
    if let Err(e) = self.reconnect_once().await {
        warn!("MCP reconnect monitor tick failed: {}", e);
    }
}
```

Backoff values (`manager/mod.rs:36-49` `ReconnectPolicy::default()`):
- `poll_interval: 5s`
- `base_delay: 2s`
- `max_delay: 60s`

Per attempt (`reconnect.rs:70-91`):
```rust
let now = Instant::now();
let (attempt_number, next_delay) = {
    let mut reconnect_states = self.reconnect_states.write().await;       // tokio RwLock
    let state = reconnect_states
        .entry(server_id.to_string())
        .or_insert_with(|| ReconnectAttemptState::new(now));

    if now < state.next_retry_at {
        return;                                                           // skip tick
    }

    state.attempts += 1;
    let delay = compute_mcp_backoff_delay(
        self.reconnect_policy.base_delay,
        self.reconnect_policy.max_delay,
        state.attempts,
    );
    state.next_retry_at = now + delay;
    (state.attempts, delay)
};
```

Compute is in
`src/crates/services/services-integrations/src/mcp/server/runtime_policy.rs:27-33`:
```rust
let shift = attempt.saturating_sub(1).min(20);
let factor = 1u64 << shift;
let base_ms = base.as_millis() as u64;
let max_ms  = max.as_millis() as u64;
let delay_ms = base_ms.saturating_mul(factor).min(max_ms);
Duration::from_millis(delay_ms)
```

Per-attempt delays: 2s, 4s, 8s, 16s, 32s, then capped at 60s.
The dynamic evidence `attempt=5 next_retry_in=32s` matches `attempt=5 →
shift=4 → factor=16 → 2s × 16 = 32s`, confirming the backoff is the
real one being applied.

The only stdio child spawn for a smoke-echo entry would block on
`MCPConnection::initialize`'s `LOCAL_INITIALIZE_TIMEOUT` (`server/connection.rs:54`,
`Duration::from_secs(30)`), set status=Failed, return — i.e. one tick
of `try_reconnect_server` ⇒ `stop_server` + `start_server` costs roughly
30s of waiting + the backoff sleep (`reconnect.rs:98-99`). No CPU burn.

**Confidence: HIGH.**

---

## Q3 — Does `initialize_core_services` run on the UI thread or block the Dioxus event loop from starting?

**Conclusion: NO.** `initialize_core_services` runs on its **own worker
thread** with its **own multi-thread tokio runtime**. The Dioxus consult-room
shell runs on the main thread with a separate `main_rt`; both start in
parallel from `main()`.

**Evidence**

`src/apps/desktop/src/main.rs:58-123`:

- L60-63 — `tracing_subscriber::fmt().init()` (synchronous, terminal).
- L65 — `(shutdown_tx, shutdown_rx) = mpsc::channel::<()>();`.
- L67-88 — **worker thread**:
  ```rust
  let worker = thread::Builder::new()
      .stack_size(16 * 1024 * 1024)
      .spawn(move || {
          let runtime = tokio::runtime::Builder::new_multi_thread()
              .enable_all()
              .build()
              .expect("failed to build tokio runtime");

          crate::app_state::turn_runtime::set_turn_runtime_handle(runtime.handle().clone());

          if let Err(e) = runtime.block_on(initialize_core_services()) {
              eprintln!("Error: failed to initialize core services: {e}");
              std::process::exit(1);
          }

          let _ = shutdown_rx.recv();    // park until shutdown signal
      })
      .expect("failed to spawn northhing worker thread");
  ```
- L90-93 — separate `main_rt` built on the main thread:
  ```rust
  let main_rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .expect("failed to build main tokio runtime");
  ```
- L122-123 — UI launched on `main_rt`:
  ```rust
  shell_result = main_rt.block_on(async {
      ui_dioxus::launch(perform_shutdown)
  });
  ```

`initialize_core_services` (L18-43) only `await`s `init_core` and spawns
the cleanup scheduler; it returns as soon as `init_core` resolves. That is
the same `init_core` covered in Q1: it does not wait on MCP servers, so
worker returns `Ok(())` quickly and parks on `shutdown_rx.recv()`. `init_core`
**returning slowly** would only delay the worker's park — it cannot
block `ui_dioxus::launch` on `main_rt`.

**Confidence: HIGH.**

---

## Q4 — Did `c678e6b` add any `ENV_LOCK` on the startup path that could deadlock?

**Conclusion: NO.** `c678e6b` adds `ENV_LOCK` only in **test modules**
(`#[cfg(test)]` and a separate `tests/` integration file). No production
code path was modified.

**Evidence**

`git show c678e6b --name-only` (now stored at this commit) lists exactly:

```
.superpowers/sdd/w14-1c-4b-report.md
northing-installer/src-tauri/src/installer/ai_config.rs
src/crates/assembly/core/tests/path_manager_uninit.rs
```

1. **`northing-installer/src-tauri/src/installer/ai_config.rs`** — diff
   shows the new `ENV_LOCK`, `EnvVarGuard`, and corresponding
   `let _guard = ENV_LOCK.lock()...` calls are all **inside the existing
   `mod tests { ... }` block** (diff context lines begin `mod tests {`,
   ending with the matching close brace). The installer Tauri crate is a
   separate binary (`northing-installer`); it is **not** linked into the
   `northhing` desktop binary — the desktop `Cargo.toml` does not
   depend on it. Even when compiled, the `mod tests` code is gated by
   `#[cfg(test)]`.

2. **`src/crates/assembly/core/tests/path_manager_uninit.rs`** — a
   `tests/` integration-test file compiled by `cargo test -p
   northhing-core`, never by the desktop release/debug binary. It
   declares `static ENV_LOCK: Mutex<()> = Mutex::new(());` at the
   module level (L10) and a `let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());`
   inside a single `#[test]` (L38).

3. **Existing `ENV_LOCK` in production code** — `path_manager.rs:257`
   declares `static ENV_LOCK: Mutex<()> = Mutex::new(());` and the only
   `lock()` call is at L261 inside `mod tests` (L252 starts with
   `#[cfg(test)] mod tests {`). This is pre-existing test scaffolding,
   unchanged by `c678e6b` (`c678e6b` only touched
   `tests/path_manager_uninit.rs`, not `path_manager.rs`).

Search `rg 'ENV_LOCK' src/` returns 4 hits, all in either
`tests/path_manager_uninit.rs:10,38` or `path_manager.rs:257,261`
under `#[cfg(test)]`. The startup path of the desktop binary never
acquires `ENV_LOCK`.

`c678e6b` cannot deadlock the production startup.

**Confidence: HIGH.**

---

## Cross-question summary

| Question | Holds in HEAD? | File:line anchor |
|---|---|---|
| Q1 init_core blocks UI on MCP | NO (gate is fast, MCP is spawned) | `kernel_facade/lifecycle.rs:140-144` |
| Q2 reconnect spin path | NO (real `tokio::time::interval`, `next_retry_at` short-circuit, exp backoff cap 60s) | `manager/reconnect.rs:17-25,70-91`; `services-integrations/.../runtime_policy.rs:27-33` |
| Q3 init on UI thread | NO (worker thread + own runtime; UI on `main_rt`) | `apps/desktop/src/main.rs:67-93,122-123` |
| Q4 ENV_LOCK startup deadlock from c678e6b | NO (test-only additions) | `c678e6b` diff; `path_manager_uninit.rs:10,38` under `tests/` |

`09a0c69` is a name-only parameterization of `run_init_gate`; production
behaviour unchanged (per its own report). `c678e6b` is test discipline
only.

## Out of static-trace scope (flagged for downstream review)

These were **not** investigated but the symptoms still warrant attention:

- The observed 100 % single-core CPU is NOT explained by the reconnect
  loop (Q2) nor by init-core blocking (Q1/Q3). Both suspect paths are
  well-behaved async. The hot-spin must live elsewhere:
  - Dioxus consult-room render loop / `use_future` polled bridge, or
  - The two-runtime split (`worker` runtime vs. `main_rt`) crossing
    threads via `app_state::turn_runtime::set_turn_runtime_handle`
    (`main.rs:77`) — a future spawned on the UI runtime may try to
    drive the worker handle.
- `.northhing/debug.log` writes 0 ⇒ the debug-log pipeline was never
  initialised in this process. Diagnose that independently (see
  `src/crates/services/debug-log/src/lib.rs:1-100` — disk-append is
  `LazyLock`-driven and started via `append_log_async` / `log_event`
  callers, none of which `main()` invokes directly).
- `smoke-echo` residue in real user MCP config: remove via
  `lib.rs` `set_global_mcp_service` saves to disk; this is a config
  cleanup, not a code fix.

status: DONE
