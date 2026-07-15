# Fix `create_ui_runs_with_noop_platform` — Wrap in `#[tokio::test]`

> **Status:** Design Complete — Awaiting User Review
> **Date:** 2026-06-23
> **Scope:** Re-enable the previously `#[ignore]`-d test `create_ui_runs_with_noop_platform` by wrapping it in a multi-thread tokio runtime.

---

## 1. Motivation

Per the user review verdict (commit `42e4c67`, HANDOVER section "✅ 用户 Review 结论"):

> "create_ui_runs_with_noop_platform 测试被 #[ignore]，需要后续修复为 #[tokio::test] 或手动启动 runtime"
> 建议优先级：低（生产路径不受影响，仅 mock 测试路径）

User selected **`#[tokio::test]` with multi_thread runtime** as the fix approach.

This is technical debt cleanup for the A2 activation. The test is a useful regression check that `create_ui` works end-to-end on a no-op Slint platform; we should not lose this coverage permanently.

---

## 2. Goal

Re-enable `create_ui_runs_with_noop_platform` by adding a multi-thread tokio runtime around the test body. The test will:

1. Spin up a tokio multi-thread runtime (1 worker thread, sufficient for `ActorRuntime::new`).
2. Set the `NoopPlatform` for Slint.
3. Call `create_ui` and assert initial UI properties.

End state:
- The `#[ignore]` attribute is **removed**.
- A `#[tokio::test(flavor = "multi_thread", worker_threads = 1)]` attribute replaces the bare `#[test]`.
- The test passes deterministically.

---

## 3. Non-Goals

- Not adding new test scenarios (only re-enabling the existing one).
- Not changing `NoopPlatform` or `create_ui` logic.
- Not switching to `current_thread` runtime (user selected `multi_thread` because ActorRuntime::new spawns a tokio handle that may need worker threads).
- Not introducing new dependencies — `tokio` is already in `[dependencies]` of `northhing` (workspace inheritance from `tokio = "1.52"`).

---

## 4. Design

### 4.1 Single-line attribute change

**File:** `src/apps/desktop/src/app_state/mod.rs` (line 1231-1237)

```diff
-    #[test]
-    #[ignore = "A2 activation (2026-06-23): create_ui calls ActorRuntime::new which requires \
-               a tokio runtime. This test runs without one and panics. Production path has a \
-               tokio runtime (slint::run_event_loop spawns it). Fix requires tokio::test or a \
-               manual runtime::Runtime::new(). See docs/superpowers/specs/2026-06-23-\
-               activate-lightweight-actor-design.md §6 (regression handling)."]
-    fn create_ui_runs_with_noop_platform() {
+    /// Verifies `create_ui` boots a Slint UI against the no-op platform.
+    /// Uses `multi_thread` runtime (1 worker) because `ActorRuntime::new`
+    /// requires a tokio handle. The runtime is torn down automatically
+    /// when the test exits.
+    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
+    async fn create_ui_runs_with_noop_platform() {
         // Set the no-op platform before creating the UI.
         slint::platform::set_platform(Box::new(NoopPlatform)).unwrap();
```

### 4.2 Why `multi_thread` and not `current_thread`

`ActorRuntime::new()` (in `src/crates/execution/agent-dispatch/src/runtime.rs:157`) calls `tokio::runtime::Handle::current()`. On a `current_thread` runtime, this works — the current thread IS the runtime worker. However:

1. The `HeartbeatActor::tick()` spawns a tokio task via `tokio::spawn` from within `log_debug_event`. On `current_thread`, this would only execute when the test yields.
2. The `OneShot` actor's handle is dropped (line 102 of `app_state/actor.rs`), but the actor body still needs the runtime to process.
3. `multi_thread` with 1 worker is the safest choice that:
   - Provides a real worker thread
   - Doesn't overload CI with parallelism
   - Matches the production runtime setup (which is multi-threaded in `slint::run_event_loop`)

### 4.3 Risk: actor spawn leakage

When `create_ui` calls `maybe_construct_actor_runtime`, it:
1. Calls `ActorRuntime::new(dispatcher, telemetry)`
2. Calls `runtime.spawn_actor(Box::new(HeartbeatActor{...}), ActorSchedule::OneShot)`
3. Drops the `JoinHandle`

The spawned `HeartbeatActor::tick()` future is now sitting in the tokio runtime's task queue. On `current_thread`, it would never execute because the test never yields. On `multi_thread`, the worker thread may pick it up.

**Risk**: The heartbeat task logs via `log_debug_event` which writes to `.northhing/debug.log`. In tests this is a side effect we don't want.

**Mitigation**: 
- The existing `log_debug_event` already swallows errors (it's fire-and-forget).
- The heartbeat writes to a log file that the test doesn't read, so it's harmless.
- If log leakage becomes a concern, future work can wrap `log_debug_event` with a `DEBUG_LOG_ENABLED` env check or feature flag.

---

## 5. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `tokio::test` attribute not in scope | Low | Low | `tokio` is in workspace deps; `tokio::test` is in the `tokio` crate root (always exported). No new import needed. |
| Actor heartbeat pollutes debug.log | Medium | Low | Existing `log_debug_event` is fire-and-forget; test doesn't read log. Future work can gate it. |
| Test flakes due to async scheduling | Low | Low | `OneShot` actor + dropped handle means task runs to completion or is dropped at runtime teardown. Test doesn't depend on actor completion. |
| `slint::platform::set_platform` panics on second call | Low | Low | Tests run in separate processes; `set_platform` is called once per test process. Safe. |

---

## 6. Acceptance Criteria

- [ ] `#[ignore]` attribute removed
- [ ] `#[test]` replaced with `#[tokio::test(flavor = "multi_thread", worker_threads = 1)]`
- [ ] Function signature changed from sync to `async fn` (required by `tokio::test`)
- [ ] Test passes: `cargo test -p northhing --lib app_state::phase_i_tests::create_ui_runs_with_noop_platform`
- [ ] All other northhing lib tests still pass (18 + 1 newly enabled = 19 passed, 0 ignored)
- [ ] No new dependencies introduced
- [ ] HANDOVER note about the technical debt is updated to record the fix

---

## 7. Acceptance Commands

```bash
cd E:/agent-project/northhing
cargo test -p northhing --lib app_state::phase_i_tests::create_ui_runs_with_noop_platform
# Expect: 1 passed; 0 failed; 0 ignored

cargo test -p northhing --lib
# Expect: 19 passed; 0 failed; 0 ignored
```

---

## 8. Out of Scope

- Multi-test runtime sharing (this test only)
- Heartbeat actor log suppression
- Migrating other tests to `#[tokio::test]` (none currently need it)

---

## 9. Plan Reference

Implementation: 1 trivial commit (T1) + 1 commit updating HANDOVER (T2).

Total estimated time: ~15min.

---

**Last updated:** 2026-06-23
**Status:** Awaiting user review before execution.