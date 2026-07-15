# R16 control_hub_tool split — review guide

**Status**: Kimi review pending. QClaw already reviewed (results in *QClaw known findings* below) — Kimi's review should cover the same ground and may surface additional issues.

**Date**: 2026-06-30 / 2026-07-01
**Branch**: `impl/round16-control-hub-tool-split`
**Worktree**: `E:\agent-project\northing-impl-round16`

---

## What to review

R16 was the **first sub-domain split** of `control_hub_tool.rs` (2526 lines). Mechanical extraction done by subagent that hit 30-min cap; Mavis took over to fix 9 import / method-dispatch bugs.

### Commits (review these)

```
142e0ed fix(control-hub-tool): R16 cross-sibling imports + inherent-method dispatch   ← Mavis take-over
b71c0ce scripts(r16): analysis + split + cleanup tooling
41fdea6 refactor(control-hub-tool): R16 sub-domain split (1 facade + 5 siblings)
15c195a docs(spec): R16 control_hub_tool 2526 → facade + 5 sub-siblings (with owner mapping)
fa8890e (NOT R16, R15.3)
```

Plus other main-branch commits already on the branch (4 R15.3 commits including the missing `]` fix; not R16-specific).

### File structure (post-R16, before R17 fixes)

| File | Lines | Target | Status |
|---|---|---|---|
| `control_hub_tool.rs` (facade) | 246 | ≤220 | **+12% (borderline)** |
| `control_hub_tool_browser.rs` | 1332 | ≤750 | **+78% (HARD — R16 D-deviation)** |
| `control_hub_tool_helpers.rs` | 217 | ≤90 | **+141% (HARD — R16 D-deviation)** |
| `control_hub_tool_meta.rs` | 238 | ≤220 | +8% (borderline) |
| `control_hub_tool_terminal.rs` | 125 | ≤130 | OK |
| `control_hub_tool_tests.rs` | 542 | ≤520 | +4% (borderline OK) |

**2 HARD D-deviations** that triggered R17: browser.rs 1332 and helpers.rs 217.

### Tests + iron rules

```
cargo test -p northhing-core --lib --features 'service-integrations,product-full'
Result: 899 passed; 0 failed; 1 ignored; 0 measured; finished in 2.14s

Iron rules Δ = 0: 37 unwraps preserved verbatim across split (5 in browser, 32 in tests, 0 elsewhere).
```

### Public API surface

`pub struct ControlHubTool`, `impl Tool for ControlHubTool`, all public types — no signature changes. External callers unaffected.

### Bug fixes by Mavis take-over (commit `142e0ed`)

Worker subagent completed mechanical file split but left 9 import/struct bugs:
1. 3× facade `use super::control_hub_tool_*:handle_*` imports (methods not free functions)
2. 3× facade `dispatch()` called handle_* as free functions instead of `self.handle_*()`
3. browser.rs missing `use super::ControlHubTool` (for `impl ControlHubTool` block)
4. browser.rs missing `use ...::ToolResult` and `use ...::parse_browser_kind`
5. terminal.rs called `TerminalControlTool::call_impl(...)` — method doesn't exist there; `call_impl` is on `impl Tool for ControlHubTool`. Fixed to `self.call_impl(...)`.
6. terminal.rs missing closing `}` on `fn handle_terminal`
7. tests.rs missing `use ...::which_exists` and `use ...::Tool`

All fixed in `142e0ed`. Diff: 4 files, 18 insertions, 11 deletions.

---

## QClaw known findings (for comparison baseline)

**Verdict (QClaw)**: ⚠️ ACCEPTABLE with CONDITIONAL blocker (D-deviation list must close in R17) — earlier reported by user externally.

**Critical observations (QClaw)**:
- O1: `browser.rs` 1332 (+78% HARD) → R17 must split
- O2: `helpers.rs` 217 (+141% HARD) → R17 must extract description_text
- O3: 3 borderline (facade 246, meta 238, tests 542)

**Minor observations (QClaw)**:
- m1: Mavis had to fix 9 import bugs (cross-sibling imports + method dispatch)
- m2: Worker timed out before writing deliverable.md
- m3: Scripts bundled with refactor in single commit `b71c0ce` instead of separate

---

## What Kimi should check

1. **Did QClaw miss anything?** Look for issues QClaw didn't flag.
2. **Are the 9 Mavis-fixed bugs actually all fixed?** Spot-check by reading the 4 changed files in commit `142e0ed`.
3. **Line cap table above** — confirm post-R16 file structure matches the numbers.
4. **Iron rules Δ** — independently verify 37 → 37 (use grep with exact patterns, see MEMORY section on reviewer cross-validation).
5. **Test count** — confirm 22 control_hub_tool tests in `mod control_hub_tests` still pass and are unchanged structurally.
6. **Public API** — confirm no signature changes in `control_hub_tool.rs` facade.
7. **Pre-existing items** — note any items that should be deferred (not R16 scope).
8. **Scripts commit `b71c0ce`** — Kimi's perspective on whether scripts being included in refactor commit is acceptable.
9. **Mavis take-over pattern** — Kimi's view on whether future rounds should preset extended-timeout to avoid the same pattern.

## Verdict format

Kimi should produce a separate review file at `docs/handoffs/2026-06-30-r16-control-hub-tool-split-kimi-review-report.md`.

Recommended scoring:
- **APPROVE 8.5/10** if no new findings + D-deviations documented as "tracked for R17"
- **APPROVE 8.0/10** if minor additional findings
- **COND 7.5/10** if any new blockers

Include: Summary, What works well, Observations (with QClaw comparison), Iron rules verification, Test verification, Verdict, Sign-off.

## Useful commands

```bash
cd E:\agent-project\northing-impl-round16
git log --oneline 15c195a..HEAD
cargo test -p northhing-core --lib --features 'service-integrations,product-full'
git diff 15c195a..HEAD --stat src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool*.rs
```

## After Kimi review

Compare Kimi's findings against QClaw's:
- Items found by both → high-confidence (commit fix or document as D-deviation)
- Items found only by Kimi → investigate, may be valid
- Items found only by QClaw → Kimi may have missed (inform both)

Then merge R16 + R17 to main together.

---

*Generated by Mavis for Kimi review of R16 control_hub_tool split.*