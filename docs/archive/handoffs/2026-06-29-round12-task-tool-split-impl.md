# Round 12 handoff: task_tool.rs 3085 → 1 facade + 5 sub-handlers

> Round 12 critical #2 god object closed.
> Worker session `mvs_280e582ec7534250af8f38e7cf954814` errored during preflight baseline
> (cargo test blocked by fetch-failed service error, 0 code written). Mavis take-over
> executed the 7-step split directly following the worker's analysis.

## Summary

| Item | Value |
|---|---|
| Spec | `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0) |
| Worktree | `northing-impl-round12` |
| Branch | `impl/round12-task-tool-split` |
| Commit | `aa1fc75` |
| Mavis take-over trigger | worker errored @ 12 min, 0 code written (only preflight baseline + planning) |
| Pattern | R11b sub-domain split + R7 turn_internal god-method split |
| cargo check | 0 errors |
| cargo test | **899 passed; 0 failed; 1 ignored** (R11b baseline preserved) |
| Iron rules violations | 0 NEW (pre-existing unwraps preserved not "fixed") |
| Fns dropped | 0 (98 → 98) |

## File structure (3085 → 6 files in `task_tool/` subdir)

```
src/crates/assembly/core/src/agentic/tools/implementations/task_tool/
├── mod.rs                        20    sub-facade: 5 pub mod + pub use re-export
├── task_tool.rs                  571   facade: TaskTool struct + Tool trait impl + call_impl orchestrator + 6 tests
├── task_tool_deep_review.rs      1690  37 deep_review_* fns + 30 tests + setup_deep_review_for_call helper
├── task_tool_subagent.rs         441   10 subagent_* fns + 2 tests + dispatch_background_subagent + execute_subagent_loop
├── task_tool_agents.rs           301   6 agent_* fns + 2 tests + PromptOrderTestAgent + build_completion_result
└── task_tool_input.rs            404   5 input validation fns + 2 tests + prepare_call_inputs + CallInputs struct
```

Total: 3427 lines (was 3085). Net +342 lines due to:
- god-method call_impl split into 5 phase helpers across siblings
- per-sibling use blocks (R10a unused-imports discipline)
- facade re-exports for backward compat (`build_available_agents_context_section`, `get_agents_types_impl_pub`)
- extra CallInputs struct (17 fields)

## call_impl god method split (R7 turn_internal pattern)

`call_impl` reduced from ~858 lines to ~80-line orchestrator:

```rust
pub async fn call_impl(...) -> NortHingResult<Vec<ToolResult>> {
    Self::ensure_delegation_allowed(context)?;
    let inputs = prepare_call_inputs(self, input, context).await?;
    let mut dr_ctx = DeepReviewContext::default();
    let mut timeout_seconds = inputs.timeout_seconds;
    if let Some((ctx, ts)) = setup_deep_review_for_call(&inputs, input, context, timeout_seconds, start_time).await? {
        dr_ctx = ctx;
        timeout_seconds = ts;
    } else {
        // Not DeepReview parent: resolve timeout via configured execution timeout.
        timeout_seconds = Self::resolve_subagent_timeout_seconds(timeout_seconds, configured_timeout);
    }
    if let Some(cached) = dr_ctx.cache_hit_result.take() { return Ok(vec![cached]); }
    let mut prepared_prompt = inputs.prompt.clone();
    if let Some(retry_scope_files) = dr_ctx.retry_scope_files.as_ref() { /* prepend */ }
    if inputs.run_in_background { return dispatch_background_subagent(...).await; }
    let outcome = execute_subagent_loop(...).await?;
    match outcome {
        ExecuteOutcome::Success(result) => Ok(vec![build_completion_result(...)]),
        ExecuteOutcome::CancelledReviewer(r)
        | ExecuteOutcome::ProviderCapacitySkip(r)
        | ExecuteOutcome::LocalCapacitySkip(r) => Ok(vec![r]),
    }
}
```

## D-deviation flag (R12b follow-up REQUIRED)

| File | Lines | Cap | Delta | Status |
|---|---|---|---|---|
| mod.rs | 20 | 200 | -180 | ✅ |
| task_tool.rs (facade) | 571 | 800 | -229 | ✅ |
| task_tool_subagent.rs | 441 | 800 | -359 | ✅ |
| task_tool_agents.rs | 301 | 800 | -499 | ✅ |
| task_tool_input.rs | 404 | 800 | -396 | ✅ |
| **task_tool_deep_review.rs** | **1690** | **1000** | **+690** | ❌ **D-DEVIATION** |

`task_tool_deep_review.rs` exceeds 1000 line cap by 690 lines (169% of cap).
Cause: 30+ deep_review tests are colocated with the 37 deep_review_* fns.
The fn count alone is ~870 lines; tests add ~820 lines.

**R12b follow-up recommended**: split `task_tool_deep_review.rs` into:
- `task_tool_deep_review_policy.rs` (37 deep_review_* fns ~870 lines)
- `task_tool_deep_review_tests.rs` (30+ tests ~820 lines) OR co-locate tests next to each fn group

## Critical fix: external caller compatibility

`turn_lifecycle.rs:93` (outside `task_tool/`) calls `TaskTool::build_available_agents_context_section(context)` as a static method. Originally this was an `impl TaskTool` method. After split it lives in `task_tool_agents` as a free fn.

**Fix**: facade `impl TaskTool` exposes a thin wrapper:
```rust
impl TaskTool {
    pub(crate) async fn build_available_agents_context_section(
        context: Option<&ToolUseContext>,
    ) -> Option<String> {
        super::task_tool_agents::build_available_agents_context_section(context).await
    }
}
```

This preserves the external caller without modifying `turn_lifecycle.rs`.

## 12-class sub-domain errors (R11b lessons reinforced)

1. **Import paths**: each sibling has precise `use super::*` blocks; no copy-paste
2. **Sibling method visibility**: `pub(super)` for cross-sibling fns; `pub(crate)` for external API
3. **Struct field visibility**: `CallInputs`, `DeepReviewContext`, `ExecuteOutcome` use `pub(super)` fields
4. **Cargo.lock drift**: skipped check (pre-existing in R11b, not relevant for split)
5. **mod.rs `pub mod`**: 5 `pub mod` declarations + 1 `pub use` re-export ✅
6. **Test attribute preservation**: all `#[test]` / `#[tokio::test]` preserved ✅
7. **cargo check stop-at-first-error**: ran 3 crates in parallel (northhing-core, northhing-tools-execution, northhing-tool-provider-groups)
8. **Cross-sibling shared enum/trait**: none — only `TaskTool` struct shared from facade
9. **R10a 1130 unused imports**: precise use blocks per sibling
10. **R11a struct owner mapping**: TaskTool struct stays in facade; `CallInputs`/`DeepReviewContext`/`ExecuteOutcome` in their semantic siblings
11. **Per-step line reporting**: tracked via `wc -l` after each cargo check
12. **R11b cross-reference paths**: siblings use `super::sibling::*` for shared types

## R11 lessons applied

- **R11 sub-domain split pattern**: 1 facade + 5 siblings per fn domain ✅
- **R11 mod.rs sub-facade**: `pub mod` + `pub use` re-export ✅
- **R11a struct owner mapping**: explicit, listed in spec §Struct owner ✅
- **R11a per-step line reporting**: enforced in take-over (logged after each fix)
- **R11b cross-reference paths**: shared `TaskTool` struct stays in facade, siblings use `use super::TaskTool;` ✅
- **R10a unused imports**: precise per-sibling use blocks (no copy-paste)
- **R10b Mavis take-over post-error pattern**: worker errored, Mavis finished 30-45 min

## Iron rules check

```
git diff origin/main..HEAD -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ \
  | grep -E '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
```

**Result: 0 NEW violations**. Pre-existing patterns preserved.

## Verification

```bash
cargo check -p northhing-core --features product-full --lib  # 0 errors
cargo check -p northhing-tools-execution --features product-full --lib  # 0 errors
cargo check -p northhing-tool-provider-groups --features product-full --lib  # 0 errors
cargo test -p northhing-core --features product-full --lib  # 899 passed; 0 failed; 1 ignored
cargo fmt  # clean (no diff)
```

## Lessons learned (this round)

1. **Worker errored during preflight baseline (cargo test blocked by fetch-failed service error)**. After 12 min of planning + cargo check on 3 crates (0 errors each), the cargo test command failed with `Service error: TypeError: fetch failed`. Engine has no automatic retry for this class of error. Mavis take-over took ~30 min.

2. **Worker had done detailed planning + fn allocation analysis** (visible in `mvs_280e582ec7534250af8f38e7cf954814` messages). Take-over directly leveraged this analysis, avoiding re-discovery.

3. **call_impl god-method split is feasible in take-over mode** when:
   - The phases have clear boundaries (input/deep_review/background/execute/complete)
   - A `CallInputs` struct carries shared state between phases
   - An `ExecuteOutcome` enum handles early-exit ToolResult variants

4. **External caller compatibility requires facade re-exports** when a static method like `TaskTool::build_available_agents_context_section(context)` is called from outside `task_tool/` module. Add a thin wrapper on the impl block.

5. **`SubagentResult` vs `SubagentExecutionResult` naming**: original code uses `SubagentResult` (not `SubagentExecutionResult`) returned by `execute_subagent`. The sibling module's `ExecuteOutcome::Success(SubagentResult)` matches the actual API.

6. **`is_concurrency_safe` is sync** (returns `bool`, not `Future<bool>`). Cannot await `validate_task_input` here. Must reproduce original sync logic — check `fork_context` directly without validation.

7. **Rust 2015 edition default for standalone rustfmt** but cargo fmt handles edition correctly via the workspace manifest. Use `cargo fmt` (not standalone `rustfmt`) for code that uses async fn.

8. **deep_review tests are ~50% of task_tool_deep_review.rs line count** (820/1690). R12b follow-up: split tests to a separate sibling or co-locate with their respective fn groups.

---

## Review outcome (post-merge, 2026-06-29)

**Verdict**: QClaw COND APPROVE **7.5/10** with **R12b REQUIRED** (D1).
**Kimi**: concurs, D1 1693 lines requires R12b secondary split.

### Approved

- mod.rs 20 lines (sub-facade pattern, R11 precedent)
- task_tool.rs 582 lines facade (Tool trait impl + 80-line call_impl orchestrator)
- task_tool_agents.rs 300 / task_tool_input.rs 402 / task_tool_subagent.rs 438 — all ≤ cap
- 0 NEW unwrap/panic/unreachable in production (21 diff matches ALL test code)
- 0 NEW `let _ = Result`
- External caller preserved via facade wrapper (`turn_lifecycle.rs:93` compiles)
- `call_impl` 5-phase orchestrator (R7 pattern) correctly delegates
- `is_concurrency_safe` sync check is a **correctness fix**, not behavior change (QClaw confirmed `.then_some(())` on a Future was a compile error in original sync context — my `fork_context` bool check matches intended semantics)
- 899/0/1 tests pass, 0 errors, fmt clean
- All 30+ `deep_review_*` test fns preserved (spot-checked by QClaw)

### D-deviation: R12b REQUIRED

`task_tool_deep_review.rs` 1693 > 1000 cap by 693 lines (69% over).
Severity tier: comparable to R8 round_executor (+63%) and R11 command_handlers (+63%).

### Minor observations (non-blocking)

| Obs | Status |
|---|---|
| R12b recommended | **Scheduled as next round** |
| `is_concurrency_safe` behavior change | **Confirmed correctness fix** (QClaw verified) |
| 11 new helper fns | Deferred to future refactor |
| Fn count discrepancy (98 vs 69) | QClaw accepted Python regex as more accurate |

### R12b plan (from QClaw §10)

- `task_tool_deep_review_policy.rs` (~870 lines): 37 deep_review_* production fns + setup_deep_review_for_call
- `task_tool_deep_review_tests.rs` (~820 lines): 30+ tests
- mod.rs adds 2 new `pub mod` declarations
- Verify 0 fns dropped, 0 new unwrap/panic, 899/0/1 maintained, fmt clean

### Review fix-cleanup cycle

- ✅ Reviewer committed `*-review-report.md` (QClaw: 1db6001, "Kimi" report embedded)
- ⏭️ **`fix(tests):` commit skipped**: no code-level fixes needed; QClaw confirmed `is_concurrency_safe` is correct, 11 helper fns deferred to future refactor
- ✅ **`docs(handoff):` bump**: this section
- ⏭️ **Cleanup**: cargo fmt noise = 0; 4 pre-existing untracked handoff docs (not R12 artifacts) left as-is