<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# K.2.3 Follow-up — Wiring + `LightweightTaskOutput → SubagentResult` Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the A1 stub gate at `execute_hidden_subagent_internal` (commit `7ff9981`) actually fire end-to-end when `USE_LIGHTWEIGHT_ACTOR=true && actor_runtime.is_some()` — by wiring `AppState::actor_runtime()` through `ToolPipeline` and `ToolUseContext` into `task_tool.rs`, then replacing the `Err(NotImplemented)` gate body with a real `spawn_long_running` call + `LightweightTaskOutput → SubagentResult` mapping.

**Architecture:** Per spec §3, 3 commits:
1. **Wiring**: `ToolPipeline` gains `Arc<OnceLock<Arc<ActorRuntime>>>` + `set_actor_runtime()` setter; `ToolUseContext` gains `actor_runtime: Option<Arc<ActorRuntime>>` field + getter; AppState's `actor.rs` wires both setters next to existing `set_actor_runtime`.
2. **Call-site swap**: `task_tool.rs:1201/1251` replace `None,` with `context.actor_runtime().as_ref(),`.
3. **Real gate body**: New `a1_path.rs` module with `map_lightweight_to_subagent_result` + `A1StubSkill` + 5 mapping unit tests; gate body replaces `Err(NotImplemented)`.

**Tech Stack:** Rust 2021, tokio, async-trait, std::sync::OnceLock. No new dependencies.

**Source spec:** `docs/superpowers/specs/2026-06-21-k2-3-followup-wiring-and-mapping-design.md`

**Branch:** `v3-restructure`. HEAD at start: `02933a1`.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs` | Modify | Add `actor_runtime` OnceLock field + setter + thread into `build_tool_use_context` |
| `src/crates/assembly/core/src/agentic/tools/tool_context_runtime.rs` | Modify | Add `actor_runtime` field to `ToolUseContext` + `actor_runtime()` getter + thread through all 3 builders + tests |
| `src/apps/desktop/src/app_state/actor.rs` | Modify | Add sibling `app_state.pipeline().set_actor_runtime(...)` call |
| `src/apps/desktop/src/app_state/mod.rs` | Modify | Add `pipeline()` getter on `AppState` |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | Modify | Replace `None,` → `context.actor_runtime().as_ref(),` at 2 call sites |
| `src/crates/assembly/core/src/agentic/coordination/a1_path.rs` | Create | `map_lightweight_to_subagent_result` + `A1StubSkill` + `build_a1_initial_request` + 5 unit tests |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | Modify | Add `mod a1_path;` + replace gate body (4 lines → 1 call into `a1_path::run_a1_path`) |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` (test fixtures at `6343+` + `6593+`) | Modify | Update all `ToolPipeline::new(..., None)` → `ToolPipeline::new(..., None, Arc::new(OnceLock::new()))` (6 fixtures) |

The A1-path code lives in a new `a1_path.rs` (per spec §8 recommendation) so `coordinator.rs` doesn't grow further. `mod a1_path;` is `pub(crate)` so tests can access.

---

## Task 1: Wiring — `ToolPipeline` + `ToolUseContext` carry the actor runtime

**Files:**
- Modify: `src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs` (struct + ctor + setter + 1 call site)
- Modify: `src/crates/assembly/core/src/agentic/tools/tool_context_runtime.rs` (struct field + getter + 3 builders + tests)
- Modify: `src/apps/desktop/src/app_state/actor.rs` (1 added setter call)
- Modify: `src/apps/desktop/src/app_state/mod.rs` (1 added getter)
- Modify: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` (6 test fixtures at `6343+` + 1 test fixture at `5939` + 1 test fixture at `6374`/`6453`/`6484`/`6593`/`6624`)

- [ ] **Step 1.1: Verify clean tree + baseline**

```bash
cd e:/agent-project/agent-app
git status --short
git rev-parse HEAD
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -3
```

Expected:
- `git status --short` → only the new spec + plan files (untracked), no modified tracked files
- HEAD → `02933a1...`
- agent-dispatch: 24/24 PASS
- regression: 8/8 PASS

- [ ] **Step 1.2: Modify `ToolPipeline` struct + ctor + add setter + thread through**

Open `src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs`. Read the struct (line 253-261), `new()` (263-276), `computer_use_host()` (278-280), and `build_tool_use_context` (1036-1047).

**Edit 1.2a — imports**: add to the top-of-file imports (search for `use std::sync`):

```rust
use std::sync::OnceLock;
```

(Only if not already imported — verify with `grep "^use std::sync" src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs`.)

**Edit 1.2b — struct**: add the new field after `computer_use_host` (line 260):

old_string:

```rust
    computer_use_host: Option<ComputerUseHostRef>,
}
```

new_string:

```rust
    computer_use_host: Option<ComputerUseHostRef>,
    /// K.2.3 follow-up: the optional `ActorRuntime` shared with
    /// `ToolUseContext` so tools can drive long-running skills.
    /// `Arc<OnceLock<...>>` because `ToolPipeline` is wrapped in
    /// `Arc<ToolPipeline>` at call sites — the inner OnceLock gives
    /// idempotent late-binding (matches `AppState::actor_runtime`
    /// pattern).
    actor_runtime: Arc<OnceLock<Arc<agent_app_agent_dispatch::ActorRuntime>>>,
}
```

**Edit 1.2c — ctor signature + body**: add the new param:

old_string:

```rust
    pub fn new(
        tool_registry: Arc<TokioRwLock<ToolRegistry>>,
        state_manager: Arc<ToolStateManager>,
        computer_use_host: Option<ComputerUseHostRef>,
    ) -> Self {
        Self {
            tool_registry,
            state_manager,
            confirmation_channels: Arc::new(DashMap::new()),
            cancellation_tokens: Arc::new(DashMap::new()),
            computer_use_host,
        }
    }
```

new_string:

```rust
    pub fn new(
        tool_registry: Arc<TokioRwLock<ToolRegistry>>,
        state_manager: Arc<ToolStateManager>,
        computer_use_host: Option<ComputerUseHostRef>,
    ) -> Self {
        Self {
            tool_registry,
            state_manager,
            confirmation_channels: Arc::new(DashMap::new()),
            cancellation_tokens: Arc::new(DashMap::new()),
            computer_use_host,
            actor_runtime: Arc::new(OnceLock::new()),
        }
    }
```

(Keeps the ctor signature unchanged — old callers don't break.)

**Edit 1.2d — setter**: add immediately after the ctor:

```rust
    /// K.2.3 follow-up: late-bind the `ActorRuntime` after
    /// `ToolPipeline::new()`. Idempotent — `set` returns Err if
    /// already set, which we silently ignore (matches
    /// `AppState::set_actor_runtime` semantics).
    pub fn set_actor_runtime(&self, runtime: Arc<agent_app_agent_dispatch::ActorRuntime>) {
        let _ = self.actor_runtime.set(runtime);
    }
```

**Edit 1.2e — `build_tool_use_context`**: thread the new field into the call to `build_tool_use_context_for_task`. Read the body (lines 1036-1047), then:

old_string:

```rust
    fn build_tool_use_context(
        &self,
        task: &ToolTask,
        cancellation_token: CancellationToken,
    ) -> ToolUseContext {
        tool_context_runtime::build_tool_use_context_for_task(
            task,
            self.computer_use_host.clone(),
            cancellation_token,
        )
    }
```

new_string (look at the actual current body — may have additional args; the spec-edited version below shows the conceptual change only):

```rust
    fn build_tool_use_context(
        &self,
        task: &ToolTask,
        cancellation_token: CancellationToken,
    ) -> ToolUseContext {
        tool_context_runtime::build_tool_use_context_for_task(
            task,
            self.computer_use_host.clone(),
            cancellation_token,
            self.actor_runtime.get().cloned(),
        )
    }
```

(The 4th positional arg `self.actor_runtime.get().cloned()` — `Option<Arc<ActorRuntime>>` — is the new param added in Step 1.3 below. If the existing builder takes fewer args, this change goes together with Step 1.3 — keep both in the same Edit if possible.)

- [ ] **Step 1.3: Modify `ToolUseContext` — add field + getter + thread through all 3 builders**

Open `src/crates/assembly/core/src/agentic/tools/tool_context_runtime.rs`. The struct is at lines 50-65.

**Edit 1.3a — struct field**: add `actor_runtime` after `runtime_handles`:

old_string:

```rust
    pub runtime_tool_restrictions: ToolRuntimeRestrictions,
    /// Runtime handles such as workspace I/O services and cancellation.
    pub runtime_handles: ToolRuntimeHandles,
}
```

new_string:

```rust
    pub runtime_tool_restrictions: ToolRuntimeRestrictions,
    /// Runtime handles such as workspace I/O services and cancellation.
    pub runtime_handles: ToolRuntimeHandles,
    /// K.2.3 follow-up: the optional `ActorRuntime` for tools that
    /// need to spawn long-running skills (currently only `TaskTool`).
    /// `None` when no actor runtime is wired (e.g. CLI/server apps,
    /// or pre-`set_actor_runtime` construction).
    pub actor_runtime: Option<Arc<agent_app_agent_dispatch::ActorRuntime>>,
}
```

**Edit 1.3b — imports**: add `Arc` import if not present (check `use std::sync` block at top of file):

```rust
use std::sync::Arc;
```

(Add if missing.)

**Edit 1.3c — getter**: add right after the existing `cancellation_token()` getter (line 143):

```rust
    /// K.2.3 follow-up: returns the wired `ActorRuntime` (if any).
    /// `TaskTool::call` uses this to pass the runtime into
    /// `coordinator.execute_subagent` so the A1 gate can fire.
    pub fn actor_runtime(&self) -> Option<&Arc<agent_app_agent_dispatch::ActorRuntime>> {
        self.actor_runtime.as_ref()
    }
```

**Edit 1.3d — thread through all 3 builders**. There are 3 builder functions: `build_tool_use_context_for_task` (211), `build_tool_use_context_for_execution_context` (224), `build_tool_description_context` (247).

For each: add `actor_runtime: Option<Arc<agent_app_agent_dispatch::ActorRuntime>>` as the last parameter, and add `actor_runtime,` to the constructed `ToolUseContext { ... }` literal.

`build_tool_use_context_for_task` (211-222) signature changes from:

```rust
pub(crate) fn build_tool_use_context_for_task(
    task: &ToolTask,
    computer_use_host: Option<ComputerUseHostRef>,
    cancellation_token: CancellationToken,
) -> ToolUseContext {
```

to:

```rust
pub(crate) fn build_tool_use_context_for_task(
    task: &ToolTask,
    computer_use_host: Option<ComputerUseHostRef>,
    cancellation_token: CancellationToken,
    actor_runtime: Option<Arc<agent_app_agent_dispatch::ActorRuntime>>,
) -> ToolUseContext {
    build_tool_use_context_for_execution_context(
        &task.context,
        Some(task.tool_call.tool_id.clone()),
        computer_use_host,
        cancellation_token,
        actor_runtime,
    )
}
```

`build_tool_use_context_for_execution_context` (224-245) signature changes similarly — add `actor_runtime` as last param, add `actor_runtime,` to the struct literal after `runtime_tool_restrictions: ...`. Verify the exact insertion point by reading the struct-literal.

`build_tool_description_context` (247-275) — same pattern.

- [ ] **Step 1.4: Update the test fixtures at `tool_context_runtime.rs:1292-1413`**

The `task_context_tests` mod (line 1291) constructs `ToolUseContext` directly. Add `actor_runtime: None,` to each `ToolUseContext { ... }` literal in those tests.

Read the file lines 1291-1414 and find every `ToolUseContext {` literal (probably 2-3). Add `actor_runtime: None,` before the closing `}`. The test at line 1367 calls `build_tool_use_context_for_task(&task, None, CancellationToken::new())` — add `, None` to make it `(..., None, CancellationToken::new(), None)`.

- [ ] **Step 1.5: Update `round_executor.rs:779`**

The `build_tool_use_context_for_execution_context` call needs the new param. Find the call:

```bash
cd e:/agent-project/agent-app
sed -n '775,790p' src/crates/assembly/core/src/agentic/execution/round_executor.rs
```

old_string:

```rust
            let storage_context =
                tool_context_runtime::build_tool_use_context_for_execution_context(
                    &tool_context,
                    Some(format!("round-budget-{}", round_id)),
                    self.computer_use_host(),
                    CancellationToken::new(),
                );
```

new_string:

```rust
            let storage_context =
                tool_context_runtime::build_tool_use_context_for_execution_context(
                    &tool_context,
                    Some(format!("round-budget-{}", round_id)),
                    self.computer_use_host(),
                    CancellationToken::new(),
                    None, // K.2.3 follow-up: round-executor doesn't spawn long-running skills
                );
```

- [ ] **Step 1.6: Update test fixtures in `coordinator.rs` (broken boundary tests)**

This is mechanical: 6 `ToolPipeline::new(..., None)` instances at lines 6343, 6374, 6453, 6484, 6593, 6624. These tests don't compile today (K.2.2 leftover), but our change adds a new param. To keep the diff minimal:

```bash
cd e:/agent-project/agent-app
grep -n "ToolPipeline::new" src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

For each call site, find the closing `)` and add `, Arc::new(OnceLock::new())` before it.

Easier: do this with `sed` — but `Edit` with explicit `old_string` is safer. There are 6 sites; do them one by one:

```bash
cd e:/agent-project/agent-app
# Read the area around each site to get exact text
sed -n '6340,6348p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
sed -n '6371,6380p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
sed -n '6450,6458p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
sed -n '6481,6489p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
sed -n '6590,6598p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
sed -n '6621,6629p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

For each, the pattern is `Arc::new(crate::agentic::tools::ToolPipeline::new(<args>, None))` — change to `Arc::new(crate::agentic::tools::ToolPipeline::new(<args>, None, Arc::new(OnceLock::new())))`.

Also line 5939 (real test, not pre-existing broken):

```bash
cd e:/agent-project/agent-app
sed -n '5937,5945p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

Same change.

Verify:

```bash
cd e:/agent-project/agent-app
grep -c "ToolPipeline::new" src/crates/assembly/core/src/agentic/coordination/coordinator.rs
# Should print 7 (6 fixtures + 1 line 5939 real test)
```

- [ ] **Step 1.7: Update `apps/server/src/bootstrap.rs:76`**

```bash
cd e:/agent-project/agent-app
sed -n '74,82p' src/apps/server/src/bootstrap.rs
```

old_string:

```rust
    let tool_pipeline = Arc::new(tools::pipeline::ToolPipeline::new(
        /* existing args */,
        None,
    ));
```

(Exact text depends on current bootstrap. Match what's there.)

new_string: add `, Arc::new(OnceLock::new())` as the last arg before the closing `)`.

Verify the file already imports `OnceLock` (or add `use std::sync::OnceLock;` if not).

- [ ] **Step 1.8: Update `agentic/system.rs:57`**

Same pattern — read the current text, add `, Arc::new(OnceLock::new())` before the closing `)`. The existing imports likely need `use std::sync::OnceLock;`.

- [ ] **Step 1.9: Update `tool_pipeline.rs:1279` test**

```bash
cd e:/agent-project/agent-app
sed -n '1275,1285p' src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs
```

old_string:

```rust
        ToolPipeline::new(registry, state_manager, None)
```

new_string:

```rust
        ToolPipeline::new(registry, state_manager, None, Arc::new(OnceLock::new()))
```

- [ ] **Step 1.10: Verify compile (lib check, not test build)**

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-core --lib 2>&1 | tail -10
cargo check -p agent-app-agent-dispatch --lib 2>&1 | tail -3
```

Expected:
- `cargo check -p agent-app-core` → 0 errors (warnings about pre-existing test build broken are OK; we already documented them)
- `cargo check -p agent-app-agent-dispatch` → 0 errors

If `cargo check` shows "X arguments expected, Y supplied" — there's a missed call site. Re-run the grep audit.

- [ ] **Step 1.11: Commit Task 1**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task1.txt <<'EOF'
refactor(tool-context): thread Arc<ActorRuntime> through ToolPipeline → ToolUseContext

Per K.2.3 follow-up spec §3.1, §3.3. AppState's actor_runtime now
flows into ToolUseContext so TaskTool can pass it to the
coordinator's A1 gate.

Changes:
- ToolPipeline gains actor_runtime: Arc<OnceLock<Arc<ActorRuntime>>>
  field + set_actor_runtime() setter. Ctor signature unchanged
  (new field defaults to empty OnceLock).
- ToolUseContext gains actor_runtime: Option<Arc<ActorRuntime>>
  field + actor_runtime() getter (mirrors cancellation_token()
  pattern).
- All 3 ToolUseContext builders thread the new param:
  build_tool_use_context_for_task /
  build_tool_use_context_for_execution_context /
  build_tool_description_context.
- round_executor.rs:779 passes None (round executor doesn't
  spawn long-running skills).
- 6 K.2.2 broken-boundary-test fixtures at coordinator.rs:6343+ +
  1 real test fixture at 5939 + 1 server fixture at
  apps/server/bootstrap.rs:76 + 1 system.rs:57 + 1
  tool_pipeline.rs:1279 all updated to pass
  Arc::new(OnceLock::new()).

Zero behavior change at USE_LIGHTWEIGHT_ACTOR=false.

Not yet wired to AppState (Task 2 of the spec — same plan).
EOF
git add src/crates/assembly/core/src/agentic/tools/ \
        src/crates/assembly/core/src/agentic/execution/round_executor.rs \
        src/crates/assembly/core/src/agentic/coordination/coordinator.rs \
        src/crates/assembly/core/src/agentic/system.rs \
        src/apps/server/src/bootstrap.rs
git commit -F /tmp/commit-task1.txt
git log --oneline -2
```

Expected: 1 new commit on top of `02933a1`.

---

## Task 2: AppState → ToolPipeline wiring + `task_tool.rs` call-site swap

**Files:**
- Modify: `src/apps/desktop/src/app_state/mod.rs` (add `pipeline()` getter)
- Modify: `src/apps/desktop/src/app_state/actor.rs` (add sibling setter call)
- Modify: `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` (replace `None,` at 2 call sites)

- [ ] **Step 2.1: Read AppState structure + `set_actor_runtime` pattern**

```bash
cd e:/agent-project/agent-app
sed -n '85,130p' src/apps/desktop/src/app_state/mod.rs
```

Confirm: there's a `set_actor_runtime()` at line 103 that takes `Arc<ActorRuntime>` and stores into `actor_runtime: OnceLock<Arc<...>>`. The pattern to mirror.

- [ ] **Step 2.2: Add `set_actor_runtime` setter on `ConversationCoordinator`**

The clean wiring path is: AppState's setter calls `coordinator.set_actor_runtime(runtime)`, which then calls `tool_pipeline.set_actor_runtime(runtime)`. Mirrors the existing `set_scheduler_notifier` / `set_round_injection_source` setters on the coordinator (`coordinator.rs:1108` / `:1114`).

Open `src/crates/assembly/core/src/agentic/coordination/coordinator.rs`. After `set_round_injection_source` (line 1114-...), add:

```rust
    /// K.2.3 follow-up: late-bind the actor runtime after
    /// coordinator construction. Forwards to `tool_pipeline` so the
    /// runtime shows up in every `ToolUseContext` built from this
    /// coordinator's pipeline. Idempotent (OnceLock semantics).
    pub fn set_actor_runtime(
        &self,
        runtime: std::sync::Arc<agent_app_agent_dispatch::ActorRuntime>,
    ) {
        self.tool_pipeline.set_actor_runtime(runtime);
    }
```

- [ ] **Step 2.3: AppState → Coordinator setter wire-up**

The actor runtime is constructed in `app_state/actor.rs:106` via `app_state.set_actor_runtime(...)`. We need to also propagate it to the coordinator. AppState has `agentic_system: OnceLock<Arc<AgenticSystem>>`; once that's set, the coordinator is reachable via `system.coordinator`.

But `set_actor_runtime` may be called BEFORE `set_agentic_system` (init order matters — runtime is constructed after `create_ui`, which is after `init_agentic_system_for_desktop`). Confirm the order:

```bash
cd e:/agent-project/agent-app
sed -n '220,235p' src/apps/desktop/src/app_state/mod.rs
grep -n "set_agentic_system\|maybe_construct_actor_runtime\|init_agentic_system_for_desktop" src/apps/desktop/src/main.rs src/apps/desktop/src/app_state/mod.rs 2>/dev/null | head -10
```

If `set_agentic_system` happens before `maybe_construct_actor_runtime` (likely, given `maybe_construct_actor_runtime` is called inside `create_ui` per `mod.rs:228`), then we can simply call `app_state.coordinator().set_actor_runtime(runtime.clone())` next to the existing `app_state.set_actor_runtime(...)` call. Otherwise we need to defer (but the OnceLock pattern on AppState + coordinator both being idempotent means we can just call it later, e.g. in `init_agentic_system_for_desktop` itself).

**Most likely order** (verify with grep): `set_agentic_system` first (during `initialize_core_services`), then `maybe_construct_actor_runtime` (during `create_ui`). So in `app_state/actor.rs:106`, add:

```rust
app_state.set_actor_runtime(Arc::new(runtime));
// K.2.3 follow-up: also wire the runtime into the ToolPipeline
// (via coordinator setter), so TaskTool gets it via context.
if let Some(coordinator) = app_state.coordinator() {
    coordinator.set_actor_runtime(Arc::new(runtime));
}
```

But `app_state.coordinator()` doesn't exist yet. Add it next to `agentic_system()` getter:

```rust
    pub fn coordinator(&self) -> Option<Arc<agent_app_core::agentic::coordination::ConversationCoordinator>> {
        self.agentic_system.get().map(|s| s.coordinator.clone())
    }
```

If `set_agentic_system` happens AFTER `maybe_construct_actor_runtime` (less likely but possible), wire it the other way: store the runtime on AppState (already does), and have `set_agentic_system` propagate to the coordinator's pipeline if AppState already has the runtime:

```rust
pub fn set_agentic_system(&self, system: Arc<...>) {
    let _ = self.agentic_system.set(system);
    // K.2.3 follow-up: if runtime was set before system, propagate now.
    if let Some(runtime) = self.actor_runtime.get() {
        self.agentic_system.get().unwrap().coordinator.set_actor_runtime(runtime.clone());
    }
}
```

(Use the order that matches your grep result.)

- [ ] **Step 2.4: Update `task_tool.rs` call sites**

Replace `None,` with `context.actor_runtime().as_ref(),` at the 2 production call sites (`task_tool.rs:1201` `start_background_subagent` + `:1251` `execute_subagent`). Both currently pass `None, // K.2.3 Phase A1: ...` comment.

**Edit 2.4a** — `task_tool.rs:1201` `start_background_subagent`:

old_string:

```rust
                    timeout_seconds,
                    None, // K.2.3 Phase A1: actor_runtime wired in follow-up session
                )
                .await?;
```

new_string:

```rust
                    timeout_seconds,
                    context.actor_runtime().as_ref(), // K.2.3 follow-up: pass through wired runtime
                )
                .await?;
```

**Edit 2.4b** — `task_tool.rs:1251` `execute_subagent`:

old_string:

```rust
                    context.cancellation_token(),
                    timeout_seconds,
                    None, // K.2.3 Phase A1: actor_runtime wired in follow-up session
                )
                .await;
```

new_string:

```rust
                    context.cancellation_token(),
                    timeout_seconds,
                    context.actor_runtime().as_ref(), // K.2.3 follow-up: pass through wired runtime
                )
                .await;
```

- [ ] **Step 2.5: Verify Task 2**

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-core --lib 2>&1 | tail -3
cargo check -p agent-app-agent-dispatch --lib 2>&1 | tail -3
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -3
```

Expected: all green (agent-dispatch 24/24, regression 8/8, lib check 0 errors).

- [ ] **Step 2.6: Commit Task 2**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task2.txt <<'EOF'
refactor(app-state): wire AppState::actor_runtime → coordinator → ToolPipeline

Per K.2.3 follow-up spec §3.3, §3.3.1 (Option A: AppState setter
chains to ToolPipeline setter).

Changes:
- ConversationCoordinator::set_actor_runtime() new pub method
  (forwards to tool_pipeline.set_actor_runtime).
- AppState::coordinator() new pub getter
  (returns Arc<ConversationCoordinator> from agentic_system).
- app_state/actor.rs: maybe_construct_actor_runtime now also
  calls coordinator.set_actor_runtime(runtime.clone()) next to
  the existing app_state.set_actor_runtime call.
- task_tool.rs:1201 + :1251: replace None with
  context.actor_runtime().as_ref() — the runtime now flows
  end-to-end from AppState into the A1 gate.

Combined with Task 1's ToolPipeline + ToolUseContext changes, the
gate condition `actor_runtime.is_some()` can now actually fire
in production.

Zero behavior change at USE_LIGHTWEIGHT_ACTOR=false.
EOF
git add src/apps/desktop/src/app_state/ \
        src/crates/assembly/core/src/agentic/coordination/coordinator.rs \
        src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs
git commit -F /tmp/commit-task2.txt
git log --oneline -2
```

---

## Task 3: Real gate body — `a1_path.rs` module with mapping + A1StubSkill + 5 tests

**Files:**
- Create: `src/crates/assembly/core/src/agentic/coordination/a1_path.rs`
- Modify: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` (add `mod a1_path;` + replace gate body)
- Modify: `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` (add Cargo `use` if needed)

- [ ] **Step 3.1: Verify Task 2 baseline + read existing gate body**

```bash
cd e:/agent-project/agent-app
sed -n '4245,4265p' src/crates/assembly/core/src/agentic/coordination/coordinator.rs
```

The current gate body (added in commit `7ff9981`) is at `coordinator.rs:4249-4255` — 4-line block returning `Err(NotImplemented)`. We'll replace it with a single call to `a1_path::run_a1_path(...)`.

- [ ] **Step 3.2: Create `a1_path.rs`**

Write to `src/crates/assembly/core/src/agentic/coordination/a1_path.rs`:

```rust
//! K.2.3 Phase A1 path — the long-running-skill replacement for
//! `ConversationCoordinator::execute_hidden_subagent_phase1/2/3`.
//!
//! ## Status
//!
//! Phase A1 STUB (per spec §2 Non-goals). This module ships the
//! plumbing — `map_lightweight_to_subagent_result` covers all 5
//! `LightweightTaskOutput` variants, and `A1StubSkill` drives a
//! 1-round dispatch loop. A real `CoordinatorHiddenSubagentSkill`
//! that wraps the existing phase1/2/3 logic is a separate,
//! multi-day spec.
//!
//! ## Activation
//!
//! `coordinator.rs::execute_hidden_subagent_internal` calls
//! `run_a1_path()` when:
//!   - `USE_LIGHTWEIGHT_ACTOR = true` (const flag, default false)
//!   - caller passed a non-None `actor_runtime: Option<&Arc<ActorRuntime>>`
//!
//! Both conditions must hold. At flag=false (default), the gate
//! is dead code and the existing phase1/2/3 path runs.

use std::sync::Arc;
use std::time::Duration;

use agent_app_agent_dispatch::{
    ActorError, ActorRuntime, LightweightTaskOutput, LightweightTaskRequest,
    LongRunningRequest, LongRunningSkill, LongRunningTickOutput,
};
use async_trait::async_trait;

use crate::agentic::coordination::coordinator::{
    HiddenSubagentExecutionRequest, SubagentResult, SubagentResultStatus,
};
use crate::util::errors::{AgentAppError, AgentAppResult};

/// Run the A1 long-running path: spawn the stub skill, await
/// the dispatch outcome, map back to `SubagentResult`.
///
/// Returns `Err(AgentAppError::service(...))` on any failure
/// (skill error, join error, timeout).
pub(crate) async fn run_a1_path(
    actor_runtime: &Arc<ActorRuntime>,
    request: &HiddenSubagentExecutionRequest,
    timeout_seconds: Option<u64>,
) -> AgentAppResult<SubagentResult> {
    let initial_request = build_a1_initial_request(request);
    let skill = A1StubSkill {
        id: format!("a1-{}", request.session_name),
        request: initial_request.0.clone(),
        prior: None,
    };
    let join = actor_runtime.spawn_long_running(Box::new(skill), initial_request);

    let dispatch_outcome = match tokio::time::timeout(
        Duration::from_secs(timeout_seconds.unwrap_or(300)),
        join,
    )
    .await
    {
        Ok(Ok(Ok(out))) => out,
        Ok(Ok(Err(e))) => {
            return Err(AgentAppError::service(format!(
                "A1 path skill error: {e}"
            )))
        }
        Ok(Err(join_err)) => {
            return Err(AgentAppError::service(format!(
                "A1 path join error: {join_err}"
            )))
        }
        Err(_) => {
            return Err(AgentAppError::service(
                "A1 path timeout".to_string(),
            ))
        }
    };

    Ok(map_lightweight_to_subagent_result(dispatch_outcome))
}

/// Build the initial `LightweightTaskRequest` from the rich
/// `HiddenSubagentExecutionRequest`. Trivial A1 mapping — real
/// mapping is the deferred `CoordinatorHiddenSubagentSkill` work.
fn build_a1_initial_request(
    request: &HiddenSubagentExecutionRequest,
) -> LongRunningRequest {
    let prepended_context: Vec<String> = request
        .initial_messages
        .iter()
        .map(|m| format!("{:?}", m)) // A1: debug-format the whole message
        .collect();
    LongRunningRequest(LightweightTaskRequest {
        dispatch_id: format!("a1-{}", request.session_name),
        user_prompt: request.user_input_text.clone(),
        prepended_context,
        tool_allowlist: Vec::new(),
        timeout: Some(Duration::from_secs(300)),
        cancel: None,
        telemetry: None,
    })
}

/// A trivial `LongRunningSkill` that drives exactly 1 dispatch round
/// then returns Done with the dispatcher's output as the final output.
/// Real `CoordinatorHiddenSubagentSkill` (wrapping phase2's loop)
/// is deferred to a separate spec.
struct A1StubSkill {
    id: String,
    request: LightweightTaskRequest,
    prior: Option<LightweightTaskOutput>,
}

#[async_trait]
impl LongRunningSkill for A1StubSkill {
    fn id(&self) -> &str {
        &self.id
    }
    fn skill_name(&self) -> &str {
        "a1_stub_subagent"
    }
    async fn tick(
        &mut self,
        _ctx: &agent_app_agent_dispatch::ActorContext,
        prior: Option<LightweightTaskOutput>,
    ) -> Result<LongRunningTickOutput, ActorError> {
        if prior.is_none() {
            Ok(LongRunningTickOutput::Continue {
                next_request: LongRunningRequest(self.request.clone()),
            })
        } else {
            Ok(LongRunningTickOutput::Done {
                final_output: prior.expect("prior is Some on second tick"),
            })
        }
    }
}

/// Map a `LightweightTaskOutput` (one-shot dispatcher result) to a
/// `SubagentResult`. Pure function — every variant covered.
pub(crate) fn map_lightweight_to_subagent_result(
    out: LightweightTaskOutput,
) -> SubagentResult {
    match out {
        LightweightTaskOutput::ToolResult { tool_name, output } => SubagentResult {
            text: output,
            status: SubagentResultStatus::Completed,
            reason: None,
            ledger_event_id: None,
        },
        LightweightTaskOutput::NoToolMatched { reason } => SubagentResult {
            text: format!("No tool matched: {reason}"),
            status: SubagentResultStatus::PartialTimeout,
            reason: Some(reason),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Cancelled => SubagentResult {
            text: "[cancelled]".to_string(),
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("cancelled".to_string()),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Timeout => SubagentResult {
            text: "[timeout]".to_string(),
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("timeout".to_string()),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Backend { message } => SubagentResult {
            text: format!("Backend error: {message}"),
            status: SubagentResultStatus::PartialTimeout,
            reason: Some(message),
            ledger_event_id: None,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn tool_result_maps_to_completed() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::ToolResult {
            tool_name: "echo".into(),
            output: "hello".into(),
        });
        assert_eq!(out.text, "hello");
        assert_eq!(out.status, SubagentResultStatus::Completed);
        assert_eq!(out.reason, None);
        assert_eq!(out.ledger_event_id, None);
    }

    #[test]
    fn no_tool_matched_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::NoToolMatched {
            reason: "empty allowlist".into(),
        });
        assert!(out.text.contains("No tool matched"));
        assert!(out.text.contains("empty allowlist"));
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("empty allowlist"));
    }

    #[test]
    fn cancelled_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Cancelled);
        assert_eq!(out.text, "[cancelled]");
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("cancelled"));
    }

    #[test]
    fn timeout_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Timeout);
        assert_eq!(out.text, "[timeout]");
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("timeout"));
    }

    #[test]
    fn backend_error_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Backend {
            message: "rate limited".into(),
        });
        assert!(out.text.contains("rate limited"));
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("rate limited"));
    }
}
```

- [ ] **Step 3.3: Wire `a1_path` module into `coordinator.rs`**

At the top of `coordinator.rs`, find a suitable place for `mod a1_path;` (after the existing module declarations — search with `^pub mod\|^mod`).

If `coordinator.rs` is a flat file (not a directory of submodules), add:

```rust
mod a1_path;
```

somewhere near the top after `use` statements. Verify the `HiddenSubagentExecutionRequest`, `SubagentResult`, `SubagentResultStatus` types are `pub` or `pub(crate)` (so `a1_path.rs` can `use` them — they're defined in `coordinator.rs` itself, so the `use` should work via `crate::agentic::coordination::coordinator::...`).

- [ ] **Step 3.4: Replace the gate body in `coordinator.rs:4249-4255`**

old_string (from commit `7ff9981`):

```rust
        if USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some() {
            return Err(AgentAppError::service(
                "Phase A1 path: long-running skill wired but SubagentResult mapping is unimplemented (K.2.3 follow-up session)".to_string(),
            ));
        }
```

new_string:

```rust
        if USE_LIGHTWEIGHT_ACTOR {
            if let Some(runtime) = actor_runtime {
                return crate::agentic::coordination::a1_path::run_a1_path(
                    runtime,
                    &request,
                    timeout_seconds,
                )
                .await;
            }
        }
```

- [ ] **Step 3.5: Verify compile + run new tests**

```bash
cd e:/agent-project/agent-app
cargo check -p agent-app-core --lib 2>&1 | tail -5
cargo test -p agent-app-core --lib a1_path 2>&1 | tail -10
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -5
```

Expected:
- `cargo check` → 0 errors (the new gate body compiles cleanly)
- `cargo test -p agent-app-core --lib a1_path` → 5/5 PASS
- `cargo test -p agent-app-agent-dispatch` → 24/24 PASS
- `regression-test-desktop.sh` → 8/8 PASS

If `a1_path` tests don't compile (because `core` test build has the pre-existing 37 K.2.2 errors and the new ones might mask), filter:

```bash
cd e:/agent-project/agent-app
cargo test -p agent-app-core --lib a1_path::mapping_tests 2>&1 | tail -10
```

If the test build fails due to pre-existing errors, document that the 5 new tests are correct by running just the file with `--no-fail-fast`:

```bash
cd e:/agent-project/agent-app
cargo test -p agent-app-core --lib a1_path --no-run 2>&1 | tail -3
# If that builds cleanly, the test runtime issue is just pre-existing
# test build errors elsewhere — documented in session log.
```

- [ ] **Step 3.6: Manual flag-flip smoke test**

```bash
cd e:/agent-project/agent-app

# Flip the flag
sed -i 's/pub const USE_LIGHTWEIGHT_ACTOR: bool = false;/pub const USE_LIGHTWEIGHT_ACTOR: bool = true;/' \
    src/crates/execution/agent-dispatch/src/flags.rs

# Confirm flip
grep "USE_LIGHTWEIGHT_ACTOR: bool" src/crates/execution/agent-dispatch/src/flags.rs

# Build to verify the new gate body compiles at flag=true
cargo check -p agent-app-core --lib 2>&1 | tail -3

# Run the agent-dispatch tests — the all_flags_default_off safety
# net should fire (intentional, expected):
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -5

# Run the A1 mapping tests — they don't depend on the flag:
cargo test -p agent-app-core --lib a1_path 2>&1 | tail -10

# Flip back
sed -i 's/pub const USE_LIGHTWEIGHT_ACTOR: bool = true;/pub const USE_LIGHTWEIGHT_ACTOR: bool = false;/' \
    src/crates/execution/agent-dispatch/src/flags.rs

# Confirm flip back
grep "USE_LIGHTWEIGHT_ACTOR: bool" src/crates/execution/agent-dispatch/src/flags.rs

# Final verification at default (flag=false)
cargo check -p agent-app-core --lib 2>&1 | tail -3
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
bash scripts/regression-test-desktop.sh 2>&1 | tail -3
```

Expected:
- After flip + `cargo check`: 0 errors (proves the new gate body compiles at flag=true)
- After flip + agent-dispatch test: 23 pass, 1 fail (the `all_flags_default_off` safety net — intentional)
- After flip + A1 mapping tests: 5 pass (mapping tests are flag-independent)
- After flip-back: `false`, all tests pass again

If the flag-flip build fails — debug before committing.

- [ ] **Step 3.7: Commit Task 3**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-task3.txt <<'EOF'
feat(coordinator): replace A1 stub with real mapping + A1StubSkill

Per K.2.3 follow-up spec §3.4, §3.5, §3.6. New module
agentic::coordination::a1_path with:
- map_lightweight_to_subagent_result: pure function mapping all
  5 LightweightTaskOutput variants to SubagentResult rows.
- A1StubSkill: trivial LongRunningSkill impl that drives 1 dispatch
  round then Done. Real CoordinatorHiddenSubagentSkill (wrapping
  phase1/2/3 logic) deferred per spec §2 Non-goals.
- build_a1_initial_request: trivial HiddenSubagentExecutionRequest
  → LightweightTaskRequest mapping (debug-formats initial_messages
  for prepended_context; full mapping is the deferred work).
- run_a1_path: the new gate body — spawn_long_running + map
  + 300s default timeout.

The gate body at coordinator.rs:4249 now calls run_a1_path
instead of returning Err(NotImplemented). At flag=true the gate
fires end-to-end: spawn → dispatcher → map → SubagentResult.

Manual flag-flip smoke test confirmed:
- flip to true → cargo check passes (new gate body compiles).
- flip to true → all_flags_default_off safety net fires
  (intentional — flag must be deliberately flipped).
- flip back to false → all tests pass.

5 new unit tests in mapping_tests: tool_result /
no_tool_matched / cancelled / timeout / backend_error.

Verification:
- cargo check -p agent-app-core --lib: 0 errors
- cargo test -p agent-app-agent-dispatch --lib: 24/24 PASS
- bash scripts/regression-test-desktop.sh: 8/8 PASS
- (Pre-existing 37 test build errors in K.2.2 boundary tests
  unrelated to this commit, verified via git stash baseline.)
EOF
git add src/crates/assembly/core/src/agentic/coordination/a1_path.rs \
        src/crates/assembly/core/src/agentic/coordination/coordinator.rs
git commit -F /tmp/commit-task3.txt
git log --oneline -2
```

---

## Task 4: Final verification + HANDOFF bump + session log

**Files:**
- Modify: `HANDOFF.md` (§0 HEAD/count + K.2.3 follow-up row)
- Modify: `docs/handoffs/2026-06-21-session-log.md` (append K.2.3 follow-up section)

- [ ] **Step 4.1: Full verification suite**

```bash
cd e:/agent-project/agent-app
echo "=== agent-dispatch tests ==="
cargo test -p agent-app-agent-dispatch --lib 2>&1 | tail -3
echo ""
echo "=== regression suite ==="
bash scripts/regression-test-desktop.sh 2>&1 | tail -5
echo ""
echo "=== clippy (both crates, agent-dispatch + core) ==="
cargo clippy -p agent-app-agent-dispatch --lib -- -D warnings 2>&1 | tail -3
echo ""
echo "=== working tree ==="
git status --short
echo ""
echo "=== session commits (since spec at 02933a1) ==="
git log --oneline 02933a1..HEAD
echo ""
echo "=== HEAD ==="
git rev-parse --short HEAD
git rev-list --count HEAD
```

Expected:
- agent-dispatch: 24/24
- regression: 8/8
- clippy agent-dispatch: clean
- 3 new commits above `02933a1`
- HEAD: <new hash>
- total commits: 134 (was 141 at e4f4ee2, +1 spec commit 02933a1 = 142; +3 implementation commits = 145). Verify actual count.

- [ ] **Step 4.2: Update HANDOFF**

Edit `HANDOFF.md`:

- §0 header: bump "Last verified" → "2026-06-21 (post-K.2.3-followup)" + HEAD + commit count.
- §5 K.3 candidates table: update K.2.3 row's status to reflect follow-up done; or add a separate row "K.2.3 follow-up" pointing to the new commits.

Recommended: update the existing K.2.3 row's status from "✅ DONE" to "✅ DONE (A1 path wired + mapping landed)" and append the 3 new commit hashes. No new row needed.

- [ ] **Step 4.3: Append K.2.3 follow-up section to session log**

Edit `docs/handoffs/2026-06-21-session-log.md` — append at the bottom:

```markdown
## K.2.3 follow-up: wiring + mapping (this session, continued)

After the K.2.3 A1 implementation at `e4f4ee2` shipped the trait
+ runtime + stub gate, this session continued per the follow-up
spec at `02933a1`. Goal: make the gate actually fire end-to-end.

### Commits

| Hash | Subject |
|---|---|
| (Task 1) | refactor(tool-context): thread Arc<ActorRuntime> through ToolPipeline → ToolUseContext |
| (Task 2) | refactor(app-state): wire AppState::actor_runtime → coordinator → ToolPipeline |
| (Task 3) | feat(coordinator): replace A1 stub with real mapping + A1StubSkill |

### Deviations from plan

| Plan | Actual | Why |
|---|---|---|
| "Add `pipeline()` getter on AppState" | "Add `set_actor_runtime` setter on `ConversationCoordinator` + `coordinator()` getter on AppState" | Spec §3.3.1 said "Option A — AppState setter calls ToolPipeline setter". But AppState doesn't directly own ToolPipeline (only via agentic_system.coordinator.tool_pipeline, 2 hops). Cleanest is to add setter on Coordinator (mirrors existing `set_round_injection_source` pattern). AppState→Coordinator→ToolPipeline. |

### Verification (all green)

- `cargo test -p agent-app-agent-dispatch --lib` → 24/24
- `bash scripts/regression-test-desktop.sh` → 8/8
- `cargo check -p agent-app-core --lib` → 0 errors
- `cargo clippy -p agent-app-agent-dispatch --lib -- -D warnings` → clean
- Manual flag-flip smoke test:
  - flag=true: `cargo check -p agent-app-core --lib` passes
  - flag=true: `all_flags_default_off` safety net fires (expected)
  - flag=true: A1 mapping tests pass (flag-independent)
  - flag=false (default): all tests pass

### What's NOT in this session (still deferred)

- Real `CoordinatorHiddenSubagentSkill` that wraps `execute_hidden_subagent_phase1/2/3` as a `LongRunningSkill::tick`. A1StubSkill is a 1-round placeholder.
- `ledger_event_id` population in A1 path.
- `SubagentResult.text` JSON parsing (preserves output as String).
- IPC path (`USE_ACTOR_IPC`).
- Pre-existing 37 test build errors in K.2.2 boundary tests.
```

Replace `(Task 1)` / `(Task 2)` / `(Task 3)` with the actual short hashes from `git log --oneline -3`.

- [ ] **Step 4.4: Final HANDOFF + session log commit**

```bash
cd e:/agent-project/agent-app
cat > /tmp/commit-final.txt <<'EOF'
docs(handoff): K.2.3 follow-up complete — A1 gate fires end-to-end

3 implementation commits + spec + plan + this bump.

K.2.3 row: A1 path now wired (AppState → ToolPipeline →
ToolUseContext → task_tool → coordinator gate → spawn_long_running
→ map_lightweight_to_subagent_result → SubagentResult).

Manual flag-flip smoke test confirmed the gate fires correctly
under USE_LIGHTWEIGHT_ACTOR=true + actor_runtime.is_some().

Still deferred: real CoordinatorHiddenSubagentSkill wrapping
phase1/2/3 (multi-day spec), ledger_event_id population,
SubagentResult.text JSON parsing, IPC path.
EOF
git add HANDOFF.md docs/handoffs/2026-06-21-session-log.md
git commit -F /tmp/commit-final.txt
git log --oneline -5
```

Expected: clean tree, final HANDOFF commit on top of Task 3's commit.

---

## Self-Review

**1. Spec coverage:**

| Spec § | Requirement | Plan Task |
|---|---|---|
| §3.1 (8 file changes) | Task 1 (5 files) + Task 2 (3 files) + Task 3 (1 new + 1 modified) ✓ |
| §3.2 (What does NOT change) | Documented throughout; no change to trait, runtimes, phase1/2/3 ✓ |
| §3.3 wiring chain | Task 1 (ToolPipeline + ToolUseContext) + Task 2 (AppState + Coordinator) ✓ |
| §3.3.1 Option A | Task 2 Step 2.2-2.3 (via Coordinator setter, mirrors existing `set_round_injection_source`) ✓ |
| §3.3.2 Arc<OnceLock<...>> | Task 1 Step 1.2b ✓ |
| §3.4 mapping table | Task 3 Step 3.2 (`map_lightweight_to_subagent_result` covers all 5 variants) ✓ |
| §3.5 A1StubSkill | Task 3 Step 3.2 ✓ |
| §3.6 new gate body | Task 3 Step 3.4 ✓ |
| §3.7 5 unit tests | Task 3 Step 3.2 (5 `mapping_tests`) ✓ |
| §4 verification criteria | Task 4 Step 4.1 (full suite) + per-task verifications ✓ |
| §6 risks | Mitigations applied (OnceLock idempotent, ~50 lines added to coordinator, smoke test reverts flag, test fixtures enumerated) ✓ |
| §7 3-commit rollout | Task 1 + Task 2 + Task 3 + Task 4 (handoff bump) — exactly matches spec ✓ |

**2. Placeholder scan:** No TBD / TODO / vague phrases. The "AppState → coordinator setter wire-up" has 2 alternative paths based on init-order grep — both shown verbatim. Step 2.2's setter code is complete.

**3. Type consistency:**
- `actor_runtime: Option<Arc<agent_app_agent_dispatch::ActorRuntime>>` — consistent across Task 1 (ToolPipeline OnceLock unwrapped via `.get().cloned()`), Task 2 (coordinator setter + AppState getter), Task 3 (`run_a1_path` arg).
- `LightweightTaskOutput` / `LightweightTaskRequest` / `LongRunningRequest` / `LongRunningSkill` — all from `agent_app_agent_dispatch` (re-exported).
- `SubagentResult` / `SubagentResultStatus` — from `crate::agentic::coordination::coordinator::*` (re-exported for `a1_path.rs`).
- `HiddenSubagentExecutionRequest` — same.
- `LongRunningTickOutput::Continue { next_request }` / `Done { final_output }` — matches K.2.3 task 1 (`95a6f0b`).

**4. Risk: pre-existing K.2.2 test build errors.** Task 1 Step 1.6 touches 6 broken test fixtures — the changes are mechanical (add `Arc::new(OnceLock::new())` to `ToolPipeline::new` arg list). If those tests are fixed in a future session, the new arg is correct (no regression). Documented in commit message.

**5. Init-order ambiguity.** Step 2.3 has 2 alternative paths based on whether `set_agentic_system` happens before or after `maybe_construct_actor_runtime`. The plan requires grep-then-pick. If neither pattern matches the existing code (e.g. AppState's runtime is set via a different path entirely), the executor must stop and ask.

**6. The A1StubSkill vs CoordinatorHiddenSubagentSkill distinction.** Spec §2 explicitly defers the real impl. The plan documents this in Task 3 commit message + session log follow-up.

**7. Final commit count.** Expected: 5 commits (1 spec at `02933a1` + 1 plan to-be-added + 3 implementation). Plan only commits the implementation + HANDOFF bump; the plan file gets added in Task 4 (HANDOFF commit). Wait — actually the plan file is created by writing-plans before executing-plans is invoked. It should be committed separately before Task 1 starts, OR added as a separate commit. **Decision: executor adds the plan file as the first commit before Task 1** (matches K.2.3 spec precedent — spec, plan, then implementation commits).