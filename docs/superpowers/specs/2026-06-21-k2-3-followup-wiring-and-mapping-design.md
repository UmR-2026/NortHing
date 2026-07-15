# K.2.3 Follow-up — Wiring + `LightweightTaskOutput → SubagentResult` Mapping Design

> **Status:** Draft (post-brainstorming, 2026-06-21)
> **Author:** ZCode session
> **Builds on:** `docs/superpowers/specs/2026-06-21-k2-3-long-running-skill-design.md` (the A1 spec) + commit `e4f4ee2` (K.2.3 implementation)
> **Goal:** Make the A1 stub gate (`coordinator.rs:4249`) actually fire end-to-end when `USE_LIGHTWEIGHT_ACTOR=true && actor_runtime.is_some()`.

---

## 1. Motivation

The previous K.2.3 session (`e4f4ee2`) shipped:
- `LongRunningSkill` trait + `ActorRuntime::spawn_long_running` (4 unit tests)
- `actor_runtime: Option<&Arc<ActorRuntime>>` parameter threaded through 3 coordinator methods
- A1 stub gate at `execute_hidden_subagent_internal` line 4249 that returns `Err(NotImplemented)`

What it does NOT do:
- The `actor_runtime=Some` branch never fires in production because `task_tool.rs` passes `None`.
- The gate's `Err(NotImplemented)` would surface if it ever did fire.

This spec closes both gaps:
1. **Wire** `AppState::actor_runtime()` → `ToolPipeline` → `ToolUseContext` → `task_tool.rs` so the call sites can pass `Some(&runtime)`.
2. **Replace** the `Err(NotImplemented)` gate body with a real `LightweightTaskOutput → SubagentResult` mapping + a `LongRunningSkill` impl that drives the loop.

After this spec lands, flipping `USE_LIGHTWEIGHT_ACTOR=true` actually routes subagent execution through the actor runtime path end-to-end (with reduced fidelity — see §3.4 mapping limitations).

## 2. Non-goals

- **Not** a real `CoordinatorHiddenSubagentSkill` impl that wraps `execute_hidden_subagent_phase1/2/3`. The A1 stub uses a **trivial skill** that drives 1 round via the existing `ToolDispatcherPort` and returns. Wrapping the real phase1/2/3 logic into a `LongRunningSkill::tick` is a separate, multi-day spec (the spec at `31799a2` lists it as deferred — K.2.3 follow-up to the follow-up).
- **Not** `LightweightTaskRequest` routing through the real LLM. The existing `ToolDispatcherPort` impls (e.g. `RuntimeServicesBuilder`) are still stub for actor-dispatch. The A1 skill uses `LightweightTaskOutput::NoToolMatched { reason: "phase-a1-stub" }` until A2 lands.
- **Not** flipping `USE_LIGHTWEIGHT_ACTOR=true` in `flags.rs`. The default stays `false`; the smoke test flips it temporarily.
- **Not** fixing the pre-existing 37 test build errors in `coordinator.rs:6267+` (K.2.2 boundary tests) — recorded in session log as separate backlog item.
- **Not** changing `LightweightTaskOutput` or `LightweightTaskRequest` shapes.
- **Not** IPC path (`USE_ACTOR_IPC`). Local tokio runtime only — matches existing `ActorRuntime::spawn_one_shot` semantics.

## 3. Design

### 3.1 Scope of change

| File | Action | Purpose |
|---|---|---|
| `src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs` | Modify | Add `actor_runtime: Arc<OnceLock<Arc<ActorRuntime>>>` field + `set_actor_runtime()` setter + pass into `build_tool_use_context` |
| `src/crates/assembly/core/src/agentic/tools/tool_context_runtime.rs` | Modify | Add `actor_runtime: Option<Arc<ActorRuntime>>` field to `ToolUseContext`; thread through `build_tool_use_context_for_task` / `_for_execution_context` / `build_tool_description_context` |
| `src/crates/assembly/core/src/agentic/tools/tool_context_runtime.rs` | Modify | Add `actor_runtime()` getter on `ToolUseContext` (mirrors `cancellation_token()` / `workspace_services()` pattern) |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | Modify | Replace A1 stub gate body with real mapping + spawn loop. Map result back to `SubagentResult` |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | Create (new file or new section in coordinator.rs) | The `LightweightTaskOutput → SubagentResult` mapping function + unit tests |
| `src/apps/desktop/src/app_state/actor.rs` | Modify | After constructing the runtime, call `app_state.pipeline().set_actor_runtime(runtime.clone())` — wire AppState's runtime into ToolPipeline too |
| `src/apps/desktop/src/app_state/mod.rs` | Modify | Add `pipeline()` getter (or similar) so `actor.rs` can reach the ToolPipeline |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | Modify | Replace `None,` with `context.actor_runtime().as_ref(),` in the 2 `execute_subagent` / `start_background_subagent` call sites |
| `src/crates/assembly/core/src/agentic/execution/round_executor.rs` | Modify | Pass `None` for the `actor_runtime` parameter at the `build_tool_use_context_for_execution_context` call (already taken; just add `None`) |

**Touch points audit (full grep before plan):**

```bash
grep -rn "build_tool_use_context_for_task\|build_tool_use_context_for_execution_context\|ToolPipeline::new" src/ --include="*.rs"
```

Real production callers: `system.rs:57` (ToolPipeline::new) + `app_state/actor.rs` (Runtime construction, calls `set_actor_runtime`). Test fixtures: `coordinator.rs:6343+` (K.2.2 broken boundary tests, pre-existing).

### 3.2 What does NOT change

- `LongRunningSkill` trait (Task 1 of K.2.3, `95a6f0b`).
- `ActorRuntime::spawn_long_running` (`95a6f0b`).
- `USE_LIGHTWEIGHT_ACTOR` default (`false`).
- The 4 unit tests in `agent-dispatch` (`long_running.rs::tests`).
- `Message` / `ExecutionResult` / `ToolCall` shapes.
- The 3 coordinator method signatures (already take `actor_runtime: Option<&Arc<ActorRuntime>>` from `a8604dd`).
- `ConversationCoordinator::execute_hidden_subagent_phase1/2/3` — the existing path stays byte-identical.

### 3.3 Wiring chain (data flow)

```
AppState::actor_runtime (OnceLock<Arc<ActorRuntime>>)
       │
       │ set via AppState::set_actor_runtime (existing, app_state/actor.rs:106)
       ▼
ToolPipeline::actor_runtime (NEW: Arc<OnceLock<Arc<ActorRuntime>>>)
       │
       │ wired by AppState after construction (NEW: app_state/actor.rs adds this)
       │ OR directly in init_agentic_system_for_desktop() (alt path — see §3.3.1)
       ▼
build_tool_use_context → ToolUseContext::actor_runtime (Option<Arc<ActorRuntime>>)
       │
       │ retrieved by TaskTool::call via context.actor_runtime() (NEW getter)
       ▼
coordinator.execute_subagent(..., context.actor_runtime().as_ref())
       │
       │ passes through to execute_hidden_subagent_internal
       ▼
A1 gate: if USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some() →
       spawn_long_running(StubSkill(...), req).await → map result
```

#### 3.3.1 Wiring choice: `OnceLock` vs setter-after-construction

Two options for the AppState → ToolPipeline hop:

- **Option A — AppState setter calls ToolPipeline setter**: `app_state::actor.rs::maybe_construct_actor_runtime` already constructs the runtime and calls `app_state.set_actor_runtime(runtime.clone())`. We add `app_state.pipeline().set_actor_runtime(runtime.clone())` next to it. Clean and local.
- **Option B — `init_agentic_system_for_desktop()` constructs pipeline with placeholder + sets later via `OnceLock` in `system.rs`**: requires adding `OnceLock` plumbing in `system.rs` too.

**Decision: Option A**. The existing `app_state::actor.rs` is the single point where the actor runtime is constructed; adding a sibling setter call there keeps the change local and matches the existing AppState-as-container pattern.

#### 3.3.2 Why `Arc<OnceLock<Arc<ActorRuntime>>>` on ToolPipeline

- `ToolPipeline` is wrapped in `Arc<ToolPipeline>` at every call site (e.g. `system.rs:57`). Sharing via `OnceLock` keeps the setter idempotent (matches `AppState::set_actor_runtime`'s "idempotent" comment at `mod.rs:101`).
- `Arc<ActorRuntime>` inner value is the cheap-to-clone handle; cloning into `ToolUseContext` is one `Arc::clone`.
- The `Arc<OnceLock<...>>` outer wrapper ensures multiple `ToolPipeline` clones (if any — none today, but future-proof) all observe the same setter.

### 3.4 `LightweightTaskOutput → SubagentResult` mapping table

`SubagentResult` shape (verified from `coordinator.rs:87`):

```rust
pub struct SubagentResult {
    pub text: String,
    pub status: SubagentResultStatus,  // Completed | PartialTimeout
    pub reason: Option<String>,
    pub ledger_event_id: Option<String>,
}
```

`LightweightTaskOutput` variants (`runtime-ports/src/lightweight_task.rs:68`):

```rust
pub enum LightweightTaskOutput {
    ToolResult { tool_name: String, output: String },
    NoToolMatched { reason: String },
    Cancelled,
    Timeout,
    Backend { message: String },
}
```

**Mapping**:

| `LightweightTaskOutput` variant | `SubagentResult` |
|---|---|
| `ToolResult { tool_name, output }` | `{ text: output, status: Completed, reason: None, ledger_event_id: None }` |
| `NoToolMatched { reason }` | `{ text: format!("No tool matched: {reason}"), status: PartialTimeout, reason: Some(reason), ledger_event_id: None }` |
| `Cancelled` | `{ text: "[cancelled]".to_string(), status: PartialTimeout, reason: Some("cancelled".to_string()), ledger_event_id: None }` |
| `Timeout` | `{ text: "[timeout]".to_string(), status: PartialTimeout, reason: Some("timeout".to_string()), ledger_event_id: None }` |
| `Backend { message }` | `{ text: format!("Backend error: {message}"), status: PartialTimeout, reason: Some(message), ledger_event_id: None }` |
| (LongRunningSkill returns `Err(ActorError)`) | map to `Err(NortHingError::service(format!("A1 path error: {message}")))` — keep as gate failure |

**Mapping limitations** (documented, not bugs):
- `SubagentResult.text` is a `String`. Tool outputs may be JSON-serialized structs — the mapping preserves them as-is (a JSON string), losing structure. Phase A2 can add `serde_json::from_str` to attempt typed extraction.
- `ledger_event_id` is always `None` for A1 — the existing phase1/2/3 path populates it via `record_checkpoint_created`. A1 doesn't call into the ledger. Tracked as a follow-up.

### 3.5 The A1 stub skill

A trivial `LongRunningSkill` impl that:
- Returns `Continue` on the first tick with a fixed `LightweightTaskRequest`.
- On the second tick (after the dispatcher returned), returns `Done` with the dispatcher's output as the `final_output`.

```rust
struct A1StubSkill {
    id: String,
    request: LightweightTaskRequest,
    prior: Option<LightweightTaskOutput>,
}
#[async_trait]
impl LongRunningSkill for A1StubSkill {
    fn id(&self) -> &str { &self.id }
    fn skill_name(&self) -> &str { "a1_stub_subagent" }
    async fn tick(&mut self, _ctx: &ActorContext, prior: Option<LightweightTaskOutput>) -> Result<LongRunningTickOutput, ActorError> {
        if prior.is_none() {
            // First tick: request one dispatch.
            Ok(LongRunningTickOutput::Continue { next_request: LongRunningRequest(self.request.clone()) })
        } else {
            // Second tick: done.
            Ok(LongRunningTickOutput::Done { final_output: prior.unwrap() })
        }
    }
}
```

This runs 1 round then exits — no real LLM loop. The whole point of A1 is to verify the **plumbing** (gate fires, spawn happens, result maps correctly), not the actual subagent logic.

### 3.6 The new gate body

```rust
async fn execute_hidden_subagent_internal(
    &self,
    request: HiddenSubagentExecutionRequest,
    cancel_token: Option<&CancellationToken>,
    timeout_seconds: Option<u64>,
    actor_runtime: Option<&Arc<ActorRuntime>>,
) -> NortHingResult<SubagentResult> {
    if USE_LIGHTWEIGHT_ACTOR {
        if let Some(runtime) = actor_runtime {
            // A1 path: drive a 1-round stub skill and map the
            // dispatcher output to SubagentResult.
            let initial_request = build_a1_initial_request(&request);
            let skill = A1StubSkill { id: format!("a1-{}", request.session_name), request: initial_request.0, prior: None };
            let join = runtime.spawn_long_running(Box::new(skill), initial_request);
            let dispatch_outcome = match tokio::time::timeout(
                Duration::from_secs(timeout_seconds.unwrap_or(300)),
                join,
            ).await {
                Ok(Ok(Ok(out))) => out,
                Ok(Ok(Err(e))) => return Err(NortHingError::service(format!("A1 skill error: {e}"))),
                Ok(Err(e)) => return Err(NortHingError::service(format!("A1 join error: {e}"))),
                Err(_) => return Err(NortHingError::service("A1 timeout".to_string())),
            };
            return Ok(map_lightweight_to_subagent_result(dispatch_outcome));
        }
    }

    // Existing path — UNCHANGED.
    let phase1 = self.execute_hidden_subagent_phase1(request, cancel_token, timeout_seconds).await?;
    let phase2 = self.execute_hidden_subagent_phase2(&phase1, cancel_token).await?;
    self.execute_hidden_subagent_phase3(phase2).await
}
```

`build_a1_initial_request` constructs a `LightweightTaskRequest` from the `HiddenSubagentExecutionRequest` fields (`session_name`, `initial_messages`, `user_input_text`). Trivial mapping for A1 — `user_prompt: request.user_input_text`, `prepended_context: messages.last().map(|m| m.content.to_string()).collect()`.

### 3.7 Unit tests for the mapping

```rust
#[cfg(test)]
mod a1_mapping_tests {
    use super::*;

    #[test]
    fn tool_result_maps_to_completed() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::ToolResult {
            tool_name: "echo".into(), output: "hello".into(),
        });
        assert_eq!(out.text, "hello");
        assert_eq!(out.status, SubagentResultStatus::Completed);
        assert_eq!(out.reason, None);
    }

    #[test]
    fn no_tool_matched_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::NoToolMatched {
            reason: "empty allowlist".into(),
        });
        assert!(out.text.contains("No tool matched"));
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

## 4. Verification criteria

1. `cargo check --workspace` — 0 errors.
2. `cargo test -p northhing-agent-dispatch --lib` — 24/24 still pass (K.2.3 tests not regressed).
3. `bash scripts/regression-test-desktop.sh` — 8/8 still pass.
4. `cargo clippy -p northhing-agent-dispatch --lib -- -D warnings` — clean.
5. **Manual flag-flip smoke test**: flip `USE_LIGHTWEIGHT_ACTOR=true`, build, run any test that exercises `execute_subagent`, confirm the gate fires and the test sees `SubagentResult { text: "..." }` rather than `Err(NotImplemented)`. Flip back to `false` after.
6. The 5 new mapping unit tests pass.
7. `all_flags_default_off_in_phase_1` safety test still fires when flag flipped (sanity check that the safety net wasn't bypassed).

## 5. Out of scope (explicitly deferred)

- Real `CoordinatorHiddenSubagentSkill` that wraps `execute_hidden_subagent_phase1/2/3` as a `LongRunningSkill::tick` — multi-day spec, separate session.
- `ledger_event_id` population in A1 path — needs ledger API access; deferred.
- `SubagentResult.text` JSON parsing — preserves output as String, loses structure.
- IPC path (`USE_ACTOR_IPC`) — separate session.
- Pre-existing 37 test build errors in `coordinator.rs:6267+` — separate fix-up backlog item.

## 6. Risks

| Risk | Mitigation |
|---|---|
| `OnceLock::set` returns Err if already set (race between AppState construction and pipeline construction in test fixtures) | `set_actor_runtime` is idempotent: ignore the Err return value (matches existing `app_state::mod.rs:107` pattern). |
| The gate body lives in `coordinator.rs` which already has 6267+ lines and 37 broken boundary tests | New code is a single `if USE_LIGHTWEIGHT_ACTOR { ... }` block at line 4249 + the `map_lightweight_to_subagent_result` fn (small). Total additions ~50 lines. |
| Manual smoke test pollutes local `flags.rs` if not reverted | Smoke test is a 4-step `sed` round-trip in plan Step 4.5; reverting is part of the test command. |
| `build_a1_initial_request` discards the rich `HiddenSubagentExecutionRequest` shape | Documented in spec §3.6 as A1 limitation; full request → request mapping is the deferred `CoordinatorHiddenSubagentSkill` work. |
| The 5 mapping tests don't catch edge cases (e.g. empty `tool_name`, multi-line output) | Mapping is a pure function — easy to extend. Tests cover all 5 enum variants. |
| Test fixtures at `coordinator.rs:6343+` (K.2.2 broken boundary tests) might also fail to compile due to new `ToolPipeline::new` signature | Plan enumerates these fixtures explicitly. New `ToolPipeline::new` adds `actor_runtime: Arc<OnceLock<...>>` as last param; all 6 fixtures need an extra arg. Pre-existing breakage not worsened. |

## 7. Rollout

Three commits on `v3-restructure`:

1. **`refactor(tool-context): thread Arc<ActorRuntime> through ToolPipeline → ToolUseContext`**
   - `ToolPipeline` gets `actor_runtime: Arc<OnceLock<Arc<ActorRuntime>>>` field + `set_actor_runtime()` setter + passes into `build_tool_use_context`.
   - `ToolUseContext` gets `actor_runtime: Option<Arc<ActorRuntime>>` field + `actor_runtime()` getter.
   - All `build_tool_use_context_*` builders thread the new value.
   - All 6 `ToolPipeline::new` test fixtures at `coordinator.rs:6343+` updated with the new arg (still don't compile — pre-existing).
   - `app_state/actor.rs` calls `app_state.pipeline().set_actor_runtime(runtime.clone())` next to existing `set_actor_runtime` call.
   - `app_state/mod.rs` adds `pipeline()` getter.
   - Zero behavior change at `USE_LIGHTWEIGHT_ACTOR=false`.

2. **`refactor(task-tool): pass actor_runtime=Some(&runtime) instead of None`**
   - `task_tool.rs:1201/1251`: replace `None,` with `context.actor_runtime().as_ref(),`.
   - First commit's wiring now flows the AppState runtime into the coordinator gate.

3. **`feat(coordinator): replace A1 stub with real mapping + A1StubSkill`**
   - Replace `Err(NotImplemented)` gate body with `spawn_long_running(A1StubSkill) + map_lightweight_to_subagent_result`.
   - Add `build_a1_initial_request`, `A1StubSkill`, `map_lightweight_to_subagent_result` in new module `a1_path.rs` next to `coordinator.rs` (or as a new section — TBD by plan).
   - Add 5 mapping unit tests.

4. **HANDOFF bump + session log follow-up** (docs-only, not strictly a code commit — see plan §4).

## 8. Open questions

- Should `map_lightweight_to_subagent_result` live in `coordinator.rs` (next to its only caller) or in a new `a1_path.rs` module? **Recommendation: new module** for testability and to keep `coordinator.rs` growth bounded. Plan can decide.

---

## Appendix A — Self-review

**Placeholder scan:** No TBD/TODO/vague phrases. The "TBD by plan" note in §7 is for a binary choice (same-file vs new-module), not a vague deferral.

**Internal consistency:**
- §3.1 lists 8 file changes; §7 lists 3 commits. Each commit maps to a coherent subset.
- §3.4 mapping table maps every `LightweightTaskOutput` variant (5) to a unique `SubagentResult` row. The "LongRunningSkill returns Err" row is documented separately as "keep as gate failure".
- §3.6 gate body uses the same `Option<&Arc<ActorRuntime>>` shape threaded from commit 2 (`task-tool`).

**Scope check:** Single-session scope. ~50 lines added to coordinator.rs (replacing the 1-line stub); ~30 lines added to a new `a1_path.rs`; 5 unit tests. ~50 lines added to ToolPipeline + ToolUseContext + threading. Plus the 1 setter call in app_state. Fits one implementation plan.

**Ambiguity check:**
- "Option A vs B wiring" — single recommendation in §3.3.1 (Option A).
- "`Arc<OnceLock<...>>` vs `OnceLock<...>`" — single choice in §3.3.2 (Arc-wrapped, future-proof).
- New module vs same file for A1 path code — flagged in §8 as a plan-level decision.
