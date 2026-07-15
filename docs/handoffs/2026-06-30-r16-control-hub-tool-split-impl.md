# R16 Handoff — control_hub_tool sub-domain split

## Summary

control_hub_tool.rs 2526 → facade (245) + 5 siblings. Worker subagent did the mechanical split in commits `41fdea6` + `b71c0ce` but timed out at the 30-min cap before the deliverable was written. Mavis took over to fix 9 import/struct/method-dispatch bugs in commit `142e0ed` so cargo check + 899/0/1 tests pass.

**Branch**: `impl/round16-control-hub-tool-split` (worktree `E:\agent-project\northing-impl-round16`)
**Commits added** (worktree only — NOT merged to main yet):
- `41fdea6` refactor(control-hub-tool): R16 sub-domain split (1 facade + 5 siblings)
- `b71c0ce` scripts(r16): analysis + split + cleanup tooling
- `142e0ed` fix(control-hub-tool): R16 cross-sibling imports + inherent-method dispatch
- (this handoff) `docs(handoff): R16 handoff + review guide`

## Final structure (6 files)

| File | Lines | Target | Status |
|---|---|---|---|
| `control_hub_tool.rs` (facade) | 246 | ≤220 | **+12% (D-deviation)** |
| `control_hub_tool_browser.rs` | 1332 | ≤750 | **+78% (HARD violation — R17 P0)** |
| `control_hub_tool_helpers.rs` | 217 | ≤90 | **+141% (HARD violation — R17 P0)** |
| `control_hub_tool_meta.rs` | 238 | ≤220 | +8% (borderline) |
| `control_hub_tool_terminal.rs` | 125 | ≤130 | OK |
| `control_hub_tool_tests.rs` | 542 | ≤520 | +4% (borderline OK) |
| **Total** | **2700** | target ~2050 | (split adds 174 lines of module headers + sibling struct/trait boilerplate) |

**3 D-deviations**: browser (1332 vs ≤750), helpers (217 vs ≤90), facade (246 vs ≤220).
**Priority for R17**: split browser.rs (handle_browser is 1199 lines — split into per-action files like browser_launch / browser_navigate / browser_snapshot), split helpers.rs (currently contains description_text which is 100+ lines of markdown — should be in a separate file).

## Iron rules Δ = 0

| | main (pre-split) | worktree (post-split) |
|---|---|---|
| `unwrap()\|expect(\|panic!\|let _ =` in `control_hub_tool*.rs` | 37 | 37 |

All 37 pre-existing unwraps preserved verbatim across the split (5 in browser, 32 in tests, 0 elsewhere — 0 NEW introduced). 0 NEW panics or let _ =. Move-not-copy discipline held.

## Tests

```bash
cargo test -p northhing-core --lib --features 'service-integrations,product-full'
# Result: 899 passed; 0 failed; 1 ignored; 0 measured; finished in 2.14s
```

All 22 control_hub_tool tests pass (per `mod control_hub_tests` moved to `control_hub_tool_tests.rs`). Test bodies unchanged.

Test discovery lesson (R14+R15+R16 all hit): default-features gate excludes the bot module entirely. Must use `--features 'service-integrations,product-full'` to see control_hub_tool tests.

## Bug fixes applied (Mavis take-over, commit `142e0ed`)

The worker subagent completed the mechanical file split but did not verify cross-sibling imports or method dispatch. Mavis identified and fixed:

1. **facade broken imports (3)**: `use super::control_hub_tool_browser::handle_browser` etc. These are METHODS on ControlHubTool defined in sibling `impl ControlHubTool { ... }` blocks, not free functions. Removed the broken imports — methods resolve via inherent dispatch.

2. **facade dispatch() calls (3)**: Changed `handle_browser(action, params)` → `self.handle_browser(action, params)` (and similar for terminal + meta).

3. **browser.rs missing imports (3)**:
   - `use super::ControlHubTool;` (needed for `impl ControlHubTool` block to resolve the type)
   - `use crate::agentic::tools::framework::ToolResult;`
   - `use super::control_hub_tool_helpers::parse_browser_kind;`

4. **terminal.rs broken dispatch (2)**:
   - Replaced `TerminalControlTool::new().call_impl(...)` with `self.call_impl(...)` (call_impl is a method on `impl Tool for ControlHubTool`, NOT on TerminalControlTool — this was a copy-paste from the original where both structs coexisted)
   - Added `use crate::agentic::tools::framework::Tool;` (needed to call trait method)
   - Fixed missing closing brace on `fn handle_terminal` (worker accidentally removed during refactor)

5. **tests.rs missing imports (2)**:
   - `use super::computer_use_actions::which_exists;` (test `which_exists_finds_a_universally_present_binary` needs it)
   - `use crate::agentic::tools::framework::Tool;` (test `description_points_desktop_and_system_work_to_computer_use` calls `.description()` which requires trait in scope)

Total: 9 distinct bugs fixed, 11 lines net added.

## Spec reconciliation

The original spec target was:
- facade ≤220 (actual 246, 12% over — D-deviation)
- meta ≤220 (actual 238, 8% over — borderline)
- browser ≤750 (actual 1332, 78% over — HARD)
- terminal ≤130 (actual 125, OK)
- helpers ≤90 (actual 217, 141% over — HARD)
- tests ≤520 (actual 542, 4% over — borderline OK)

The mechanical split landed handlers in sibling files but the line budgets were not respected. The natural breakdown:
- browser.rs 1332 contains the entire `handle_browser` function (1199 lines pre-split). It is the natural R17 P0 candidate — split per-action (`handle_browser_launch`, `handle_browser_navigate`, `handle_browser_snapshot`, etc.). Each per-action module would be ~150-300 lines.
- helpers.rs 217 contains `description_text` (a 100+ line markdown string of browser action documentation), the 3 `parse_*` helpers, `envelope_wrap_results`, `map_dispatch_error`. The `description_text` should move to a separate file (e.g. `control_hub_tool_descriptions.rs`). The actual helpers are ~80 lines, fitting the target.

## Q1-Q5 (review guide for next reviewer)

| # | Question | Answer |
|---|---|---|
| 1 | Is the sub-domain split structurally correct? | ✅ Yes. Browser/terminal/meta handlers physically extracted. Cross-sibling method dispatch verified via 899/0/1 tests. |
| 2 | Iron rules preserved? | ✅ Yes. Δ=0 (37 unwraps before, 37 after, all preserved). 0 NEW unwrap/panic/let _. |
| 3 | Public API surface unchanged? | ✅ Yes. `pub struct ControlHubTool`, `impl Tool for ControlHubTool`, all public types — no signature changes. |
| 4 | Line caps met? | ❌ 2 HARD violations (browser + helpers) + 3 borderline. D-deviation list documented above. R17 must split browser and helpers. |
| 5 | Should the worker have succeeded on its own? | ❌ No. The worker hit the 30-min cap before the deliverable was written, leaving 9 import bugs. Per MEMORY R14 5-pass take-over pattern, Mavis finished the work in ~30 min of focused fixes. **R17 lesson**: dispatch subagent + extend timeout 60 immediately + cron monitor (per R14 standing rule). The worker did not write `deliverable.md` so engine had no signal to extend. |

## Reviewer checklist (for human review)

- [ ] Read spec at `docs/handoffs/2026-06-30-r16-control-hub-tool-split-spec.md` (177 lines)
- [ ] Read this handoff
- [ ] Verify cross-sibling imports by inspection:
  - `grep '^use super' control_hub_tool*.rs` — all siblings should import `ControlHubTool` (except helpers)
  - `grep '^impl ControlHubTool' control_hub_tool*.rs` — should appear in facade + browser + meta + terminal (4 impl blocks total)
- [ ] Verify iron rules preserved: `git diff main..HEAD -- src/.../implementations/control_hub_tool*.rs | grep -cE '^\+.*unwrap\(\)|^\+.*panic!'` should be 0
- [ ] Verify tests pass: `cargo test -p northhing-core --lib --features 'service-integrations,product-full'`
- [ ] Decide on R17 P0 candidates:
  - **A**: split `handle_browser` (1199 lines) into per-action files (~6-8 sibling modules)
  - **B**: split `helpers.rs` — move `description_text` to `control_hub_tool_descriptions.rs`

## What to merge

```bash
cd E:\agent-project\northing
git fetch origin
git checkout main
git merge --no-ff impl/round16-control-hub-tool-split -m "merge: Round 16 control_hub_tool sub-domain split (facade + 5 siblings; D-deviations: browser+helpers over budget, R17 P0)"
```

Then schedule R17 to close the line-cap D-deviations.