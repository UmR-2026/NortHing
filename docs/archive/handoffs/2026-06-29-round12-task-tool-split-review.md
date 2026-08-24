# Round 12 review guide: task_tool.rs 3085 → 1 facade + 5 sub-handlers

> Reviewer (QClaw / Kimi): please review commit range
> `f0f9bc0..aa1fc75` (spec → refactor) on branch
> `impl/round12-task-tool-split`. Handoff doc:
> `docs/handoffs/2026-06-29-round12-task-tool-split-impl.md`.

## What to review

| File | Lines | Note |
|---|---|---|
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` (deleted) | -3085 | original god object |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/mod.rs` (new) | 20 | sub-facade |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool.rs` (new) | 582 | facade: TaskTool struct + Tool trait impl + call_impl orchestrator + 6 tests |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_input.rs` (new) | 402 | 5 input validation fns + prepare_call_inputs + CallInputs + 2 tests |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_subagent.rs` (new) | 438 | 10 subagent_* fns + dispatch_background_subagent + execute_subagent_loop + 2 tests |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_agents.rs` (new) | 300 | 6 agent_* fns + build_completion_result + PromptOrderTestAgent + 2 tests |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review.rs` (new) | 1693 | 37 deep_review_* fns + setup_deep_review_for_call + 30 tests |

## Critical observations (please verify)

### 1. call_impl god method split (R7 turn_internal pattern)

Original `call_impl` was ~858 lines mixing input validation + DeepReview setup
+ background dispatch + main execution loop + completion result. Split into
5 phase helpers called from a slim ~80-line orchestrator in facade:

```rust
pub async fn call_impl(...) -> NortHingResult<Vec<ToolResult>> {
    Self::ensure_delegation_allowed(context)?;
    let inputs = prepare_call_inputs(self, input, context).await?;
    let mut dr_ctx = DeepReviewContext::default();
    let mut timeout_seconds = inputs.timeout_seconds;
    if let Some((ctx, ts)) = setup_deep_review_for_call(...).await? { ... } else { ... }
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

Verify:
- All 5 phases produce the same observable behavior as original call_impl
- `CallInputs` carries the same fields as the local variables in original call_impl
- `DeepReviewContext` carries all deep_review-related state through the phases
- `ExecuteOutcome` enum handles all 4 early-exit / success paths

### 2. D-deviation (R12b follow-up REQUIRED)

`task_tool_deep_review.rs` = 1693 lines, exceeds 1000 cap by 693 lines (169%).

Cause: 30+ deep_review tests colocated with 37 deep_review_* fns (820 lines of tests
+ 870 lines of code).

This is the third-largest D-deviation after R8 round_executor (+104%) and R11
remote_command_handlers (+63%). Per project convention, accept the deviation
and create R12b follow-up split rather than blocking merge.

**R12b plan** (suggested):
- `task_tool_deep_review_policy.rs` (~870 lines): 37 deep_review_* fns
- `task_tool_deep_review_tests.rs` (~820 lines): 30+ tests
- OR co-locate each test next to its fn (less explicit split, same outcome)

### 3. Pre-existing vs new violations (R11 lesson)

R11 review taught us to distinguish:
- **Pre-existing debt**: unwraps / panic / let _ = that were in the original
  file before the split. Move them, don't fix them.
- **NEW violations**: unwraps / panic / let _ = introduced by the refactor.

This split has **0 NEW violations**. To verify:

```bash
git diff f0f9bc0..aa1fc75 -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ \
  | grep -E '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
```

Expected: 0 lines.

### 4. External caller compatibility

`src/crates/assembly/core/src/agentic/execution/turn_lifecycle.rs:93` calls
`TaskTool::build_available_agents_context_section(context)` as a static method.
After split, this fn lives in `task_tool_agents` as a free fn.

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

Verify: turn_lifecycle.rs:93 still compiles and behaves identically. Also
verify no other callers are broken (the public API `pub use task_tool::TaskTool`
in mod.rs preserves the path `task_tool::TaskTool`).

### 5. Iron rules verification

```bash
# 0 NEW unwrap/panic/unreachable
git diff f0f9bc0..aa1fc75 -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ \
  | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expect: 0

# 0 NEW let _ = Result
git diff f0f9bc0..aa1fc75 -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ \
  | grep -cE '^\+.*let _ = .*Result'
# Expect: 0

# 0 fns dropped (98 → 98)
py -c "
import re
from pathlib import Path
import subprocess
result = subprocess.run(['git', 'show', 'f0f9bc0:src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs'],
                       capture_output=True, text=True)
old_fns = set(re.findall(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', result.stdout, re.M))
print(f'old fns: {len(old_fns)}')

import os
wt_dir = Path(r'src/crates/assembly/core/src/agentic/tools/implementations/task_tool')
new_fns = set()
for f in wt_dir.glob('*.rs'):
    new_fns.update(re.findall(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'new fns: {len(new_fns)}')
print(f'dropped: {old_fns - new_fns}')
"
# Expect: old=98 new=98 dropped={}
```

### 6. Cargo verification

```bash
# Baseline preserved
cargo check -p northhing-core --features product-full --lib  # 0 errors
cargo check -p northhing-tools-execution --features product-full --lib  # 0 errors
cargo check -p northhing-tool-provider-groups --features product-full --lib  # 0 errors
cargo test -p northhing-core --features product-full --lib  # 899 passed; 0 failed; 1 ignored
cargo fmt --check  # clean
```

All checks verified pre-merge.

## Questions for reviewer

1. **R12b necessity**: should R12b (deep_review secondary split) be scheduled
   as a follow-up round, or is 1693 lines acceptable for a "split" round?

2. **call_impl orchestrator size**: at ~80 lines the orchestrator is slim, but
   still inlined rather than extracted to a separate `call_impl_orchestrator.rs`.
   Acceptable, or prefer explicit separation?

3. **`is_concurrency_safe` sync check**: I replaced the original `.then_some(())`
   on `validate_task_input` (which doesn't compile in sync context) with a
   manual `fork_context` boolean check. Verify this matches the original
   observable behavior:
   - Original: returned `false` if validation succeeded (since `.then_some(())`
     on future isn't valid, the original code likely never compiled — or the
     logic was different). My check: returns `false` if `fork_context=true`.
   - This may be a behavior change vs. original intent. Please flag if the
     sync check should differ.

4. **`turn_lifecycle.rs` external caller**: facade wrapper preserves the static
   method call signature. Verify this is the correct layering (no facade
   boundary violations).

## Refs

- R12 spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0)
- R12 handoff: `docs/handoffs/2026-06-29-round12-task-tool-split-impl.md` (548c1ea)
- R11 review report (precedent for pre-existing vs new): `docs/handoffs/2026-06-29-round11-remote-connect-split-review-report.md`
- R11b handoff: `docs/handoffs/2026-06-29-round11b-remote-connect-secondary-split-impl.md`
- Iron rules reference: `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md`

## Sign-off request

Please provide:
1. **APPROVE / REJECT** decision with score (1-10)
2. List of any **minor observations** (non-blocking)
3. Confirmation of R12b necessity decision
4. Any structural concerns about the 5-sibling + mod.rs sub-facade layout

Reply format: standard project review report ending in
`*-review-report.md` (will be committed by reviewer per established pattern).