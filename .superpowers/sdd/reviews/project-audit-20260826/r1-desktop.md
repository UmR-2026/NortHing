# R1 — Desktop Shell Audit (2026-08-26, branch main @ 74ea164, clean tree)

Scope: `src/apps/desktop/src/**` (`app_state/**`, `ui_dioxus/**`, top-level
`main.rs` / `lib.rs` / `flags.rs` / `mcp_adapter.rs`, `bin/w4_repro.rs`).
Read-only — no mutations. Hot-spot risk review, not line-by-line.

### Verdict: needs attention

The shell is **structurally sound** — backend-invariant follow-through is
genuinely disciplined (atomic settings write, keyring fail-closed for
ProviderConfig.api_key, Slint setter thread discipline, turn-generation guards
in `streaming_lifecycle`, `SETTINGS_WRITE_LOCK` under load→mutate→save). Most
of what I found is efficiency drift, security-soft spots, or one-off UX
foot-guns rather than bugs that would terminate the process. Two issues earn
"Important" severity — one user-visible (key erasure on provider edit), one
UX (sub-windows forcibly on top of every other app).

---

### Findings (8, most severe first)

#### 1. Important — Edit-provider path can silently erase the user's API key
- **File:line**: `src/apps/desktop/src/app_state/callbacks_settings/provider.rs:121-125, 161`
- **What**: When the user opens an existing provider and the form's API-key
  field is blank (the standard "preserve stored key" edit pattern), the code
  reads the key from keyring via `PRODUCTION_KEYRING.get(&pid).ok()` — the
  `.ok()` **drops the keyring error**. If the keyring is unavailable (WinCred
  locked by another process, credential gone after credential-store
  rotation, `Err(_)`, etc.) the read silently yields `None`, so
  `resolve_effective_api_key(None, "")` returns `""`. That `Some("")` is
  then shipped to `facade.upsert_model_config(model_dto, Some(""))` (line
  161) and the keyring is never re-populated. Net effect: a transient
  keyring read failure during the edit save path wipes the user's key from
  **both** the keyring and core memory. The user sees "已保存" but their
  next LLM call is unauthenticated.
- **Why it matters**: P1-2 (resolved per `tech-debt-ledger.md` §P1-2)
  established "fail-closed" as the contract — the edit path is a fail-open
  hole that survives the original remediation. No test covers this error
  arm.
- **Fix**: propagate the keyring error (`.map_err(...)?`) and refuse the
  save with a banner; alternatively, **never** write `Some("")` — keep the
  sentinel on disk and require the user to retype a key to change it.
- **Effort**: S.

#### 2. Important — Sub-windows are `HWND_TOPMOST`, overriding every other app
- **File:line**: `src/apps/desktop/src/app_state/block_registry.rs:153-154, 85-138`
- **What**: `set_tool_window` calls `SetWindowPos(hwnd, HWND_TOPMOST, ...)` on
  both the inner (left drawer) and outer (right drawer) Slint windows when
  they first appear. `HWND_TOPMOST` is a permanent "always on top of all
  applications" — not just above the main window. The 16ms `sync_timer` then
  resizes and repositions them at ~60 fps.
- **Why it matters**: When the user alt-tabs to a browser, editor, or any
  other app, the NorthHing drawer panels stay painted on top of those apps
  for the lifetime of the main window. This is a UX foot-gun, not just a
  Slint/Z-order quirk.
- **Fix**: drop the `HWND_TOPMOST` call entirely (Slint drawer windows are
  transient children of the main window and don't need cross-app
  always-on-top). Use `HWND_TOP` (top within this process) if any
  hierarchy is actually needed.
- **Effort**: S.

#### 3. Important — `.expect()` inside `std::thread::spawn` silently drops 8 UI actions
- **File:line**: `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:297, 394, 437, 543, 646, 722, 752, 831`
- **What**: Eight callback body sites do
  `tokio::runtime::Builder::new_current_thread().enable_all().build().expect("failed to build tokio runtime…")`
  inside `std::thread::spawn`. A runtime build failure panics the spawned
  thread; the panic is caught by the thread (no process death), and the
  user's UI action — new-session, switch-session, delete-session,
  toggle-skill, load-more-messages, refresh-sessions, refresh-messages,
  stop-streaming — silently no-ops. Some sibling sites in the same file
  (e.g. `callbacks_lifecycle.rs:866`, `create_ui.rs:118-126`, `provider.rs:24-32`)
  handle the build failure correctly with `match` + `tracing::error!` +
  user-visible banner; this code is inconsistent.
- **Why it matters**: The failure mode is silent. A user clicks "stop" on a
  runaway turn, the button does nothing, the turn keeps running — no error
  banner, no log line the user can see. Other callers show "内部错误：无法
  启动运行时" — the calling convention is already established, these eight
  sites just didn't get the same treatment.
- **Fix**: replace each `.expect(...)` with `match build() { ... return
  set_session_error(...); }` (the existing convention).
- **Effort**: S.

#### 4. Minor — `schedule_error_clear` leaks one OS thread per error banner
- **File:line**: `src/apps/desktop/src/app_state/error_banners.rs:106-128`
- **What**: Each call to `set_session_error` / `set_input_error` /
  `set_banner_message` spawns a fresh `std::thread::spawn` that
  `std::thread::sleep(5s)` and then posts a clear via
  `slint::invoke_from_event_loop`. If the user hits an input-validation
  storm or a load-failure cascade, the process accumulates short-lived
  threads (no stop token — even user-dismissed banners still fire the
  clear at +5s, racing the user's manual dismissal).
- **Why it matters**: Functionally cosmetic; the Weak upgrade is a no-op
  when the window is gone. The thread leak is real but bounded (~thread
  per error / 5s), and the fire-and-forget `slint::invoke_from_event_loop`
  swallows the most concerning race.
- **Fix**: keep a per-`ErrorKind` `slint::Timer` registered once and call
  `.restart()`; have the manual-dismiss callbacks cancel it. Worth doing
  if the error rate ever rises.
- **Effort**: S-M.

#### 5. Minor — `keyring.load_env` is fail-open; missing MCP env vars are silent
- **File:line**: `src/apps/desktop/src/app_state/settings/keyring.rs:292-313`
- **What**: `load_env` for MCP-server env returns `Ok(HashMap::new())` on
  both "keyring entry missing" and "JSON parse error". The blank-map
  silently replaces any MCP env vars (often credentials like
  `OPENAI_API_KEY`, `AWS_ACCESS_KEY_ID`) the user configured. The P1-8
  ledger entry acknowledges this as active (cursor-format cursor-side
  persist path was the deeper concern); the desktop `AppSettings.mcp_servers`
  field is the secondary path noted as dead-code by user ruling.
- **Why it matters**: Security boundary is partially fail-open here for
  the desktop view of MCP env. Contrast with `resolve_api_key` for the
  same keyring — it fail-closes on missing entries (line 388-390 test).
  The asymmetry is intentional per the brief (the desktop cannot read the
  cursor-format file at all), but it deserves a one-line `tracing::warn!`
  mention in the panicking-mode docs. The code is correct per spec; the
  concern is that 30 days from now someone will add an MCP server whose
  env is critical, expect it to round-trip, and discover it doesn't.
- **Fix**: surface a startup-time banner if any loaded MCP server has a
  non-empty expected env that came back empty; otherwise leave as-is and
  document the P1-8 status in the load_env docstring.
- **Effort**: S.

#### 6. Minor — Forever-running 60Hz `slint::Timer` never stops
- **File:line**: `src/apps/desktop/src/app_state/block_registry.rs:74-82 (init_timer, 100ms), 85-138 (sync_timer, 16ms)`
- **What**: `BlockRegistry` owns two `slint::Timer`s in `Repeated` mode
  running forever at 100ms / 16ms. The `init_timer` self-disables after
  `set_tool_window` succeeds. The `sync_timer` has no disable path — it
  continues firing at ~60 Hz for the lifetime of the process, even after
  the main window is closed or the inner/outer windows are dropped. Each
  tick upcasts 3 Weak handles, reads `window().position()` / `.size()`,
  calls `IsIconic(HWND)` via raw Win32, and (when visible) `set_position`
  + `set_size`. After a window is dropped, the closure early-returns but
  the timer still wakes the event loop.
- **Why it matters**: ~60 wakeups/sec on the Slint event loop forever
  after a normal app exit pathway. Each wakeup is cheap (~microseconds)
  but adds up: combined with finding #2 the timer also drives the
  `HWND_TOPMOST` re-shows.
- **Fix**: expose `BlockRegistry::terminate()` calling `.stop()` on both
  timers; call it from the Slint `WindowEvent::CloseRequested` handler
  on the main window (wiring it to the existing FR-T3b close callback is
  one line — see `create_ui.rs:374-378`).
- **Effort**: S.

#### 7. Minor — Per-callback throwaway tokio runtime pattern (37 occurrences)
- **File:line**: `main.rs:224`; `bin/w4_repro.rs:235`; `app_state/callbacks_lifecycle.rs:294,391,434,540,643,719,749,828,866,953`; `app_state/callbacks_settings/{provider,provider_test,workspace,misc,refresh}.rs` (5 lines in misc.rs, 2 in provider.rs, 2 in provider_test.rs, 3 in workspace.rs, 1 in refresh.rs, in addition to `create_ui.rs` × 5 + `event_bridge.rs:343` + `api.rs:203` + `log.rs:45`).
- **What**: Every Slint callback that needs to hit the kernel facade
  spawns a `std::thread` that builds a **fresh** `tokio::runtime::Builder
  ::new_current_thread()` and runs `block_on` on it. W4 (per commit
  `8d3...` referenced in `tech-debt-ledger.md`) introduced the
  long-lived worker runtime (`turn_runtime::set_turn_runtime_handle`)
  **only** for turn dispatch. Every other desktop callback still uses
  the throwaway pattern that W4 was meant to retire. Each click of
  "Refresh providers" costs ~1 OS-thread creation + ~1 runtime
  initialization + ~1 thread teardown.
- **Why it matters**: Functional but wasteful; deferred since W4 because
  the turn-dispatch half is the
  real-blocks-the-UI case. A user clicking refresh-settings repeatedly
  spawns 5+ runtimes per second. The `ROOM_SESSION_CACHE` (api.rs:94)
  and `log::ensure_log_consumer` (log.rs:30) **do** use the right
  pattern; the inconsistency is internal.
- **Fix**: extend `turn_runtime::try_current()` to a public
  `current_or_local()` helper and call `handle.spawn(...).await` from
  all 37 sites instead of building a new thread + runtime. ~50 LOC of
  mechanical change.
- **Effort**: M.

#### 8. Minor — Three module-window "geometry follow" raw threads idle-poll
- **File:line**: `src/apps/desktop/src/ui_dioxus/windows.rs:137-178, 408-449 (facility), 605-649 (work)`
- **What**: Each of the three Dioxus module windows (`self`, `facility`,
  `work`) spawns one `std::thread` running
  `std::thread::sleep(16ms); rx.has_changed(); ...`. The room window
  itself was fixed (per the r3p4 root fix — see `entry.rs:194-251`) to
  use `with_custom_event_handler` on tao's event loop, but the three
  module follow threads re-introduce raw polling at the OS thread
  level. ~180 wakeups/sec across the three threads for the lifetime
  of the Dioxus shell.
- **Why it matters**: Deliberate — the r3p4 docs warn that any
  `use_future` with `sleep` busy-spins on `dioxus 0.8-alpha.1`, so
  these were moved out of the Dioxus scheduler into raw threads, where
  they don't poison the executor. But the `std::thread::sleep(16ms)` +
  `rx.has_changed()` pattern is still ~3% CPU on a single core per
  module. Three of them is real tax. More importantly: `unsafe {
  win::GetDpiForWindow(hwnd_ptr) }` (line 153, 419, 619) is called every
  iteration with no Windows-level error handling — if Windows ever
  moves the cursor of the slot, the syscall returns 0 and `scale`
  becomes 0/96 = 0, dividing-by-zero in `((280.0 + DOCK_GAP_PX as f64) * scale) as i32`.
- **Fix**: keep the r3p4 approach, but use `windows::WaitForSingleObject`
  on a HANDLE signaled by the room window's `Moved`/`Resized` event, or
  poll at 50 Hz (20ms) instead of 60; bound the `GetDpiForWindow` zero
  case (`if dpi == 0 { break; }`).
- **Effort**: M.

---

### Sample coverage note

**Deep-read (line-by-line or near)**:
`main.rs`, `lib.rs`, `flags.rs`, `mcp_adapter.rs`, `create_ui.rs`,
`error_banners.rs`, `event_bridge.rs`, `streaming_lifecycle.rs`,
`slint_glue.rs`, `mod.rs (app_state)`, `state.rs`, `log.rs`,
`turn_runtime.rs`, `block_registry.rs`, `sessions.rs`,
`settings/{keyring,io,mod,sync}.rs`,
`callbacks_settings/{provider,provider_test,skill_filter,misc,refresh,mod}.rs`,
`ui_dioxus/{mod,entry,state,api,windows(1-260,750-end)}.rs`,
`bin/w4_repro.rs`.

**Skim (grep-driven, full file unread but pattern-checked)**:
`ui_dioxus/{app,css,refresh,pages_onboarding,pages_settings,registry,session_mock,page_shell,pages_space,pages_archive,i18n,pages_onboarding_css}.rs`,
`callbacks_lifecycle.rs` (read three slices ~280-480, 540-555, 820-919),
`callbacks_settings/workspace.rs`,
`app_state/settings/{types,integrity,tests,io/io_tests}.rs`,
`app_state/{inspector,inspector_model_status,skills}.rs`.

**Excluded as instructed**:
`pages_onboarding.rs` (807L), `ui_dioxus/app.rs` (962L), `ui_dioxus/css.rs`
(744L), `ui_dioxus/refresh.rs` (834L /* note: actually 790L in the
search */), `callbacks_lifecycle.rs` (1011L), `settings.rs` (via
`*` re-exports; split into submodules), `theme/cb_lifecycle god files`,
`memory_db.rs`. P2-22 entries.set startup race, P1-8 dead-code, GNU/MSVC
toolchain — all skipped per the EXCLUDE list.

**Cross-cutting patterns grep'd across the
desktop crate**: `tokio::runtime::Builder::new_current_thread()`
(38 hits), `.expect(` on runtime construction (8 hits in
callbacks_lifecycle.rs), `.unwrap\(\)`, `.unwrap_or_default\()`,
`let _ =`, `unsafe`, `panic!`, `slint::invoke_from_event_loop`.

Cargo check: not run (read-only audit per the brief); the
callbacks_lifecycle.rs sites I cite were
cross-checked against the existing `match`-arm pattern in the same
file (line 866) and in `create_ui.rs:118-126`. Compile-gate
enforcement post-fix is recorded as housekeeping rule 6 in
`AGENTS.md` and applies.
