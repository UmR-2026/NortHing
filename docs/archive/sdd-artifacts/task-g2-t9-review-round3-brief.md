# T9 Review Round 3 — N1 Discriminator Closure

## Package

- Source worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
- Exact BASE: `aa53f35`
- Exact HEAD: `1e1f009`
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-brief.md`
- Fix brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-fix-brief.md`
- Round 1 review: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review.md`
- Round 2 review: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review-round2.md`
- Final implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t9-report.md`
- Final full diff: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-diff.patch`
- Write review only to: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review-round3.md`

Review the entire `aa53f35..1e1f009` range. Source worktree is read-only. Do not edit, commit,
dispatch child agents, or rerun the implementer's reported tests.

## N1 Closure

Verify the new test fixture and assertions in commit `1e1f009` actually discriminate the
stale-snapshot failure mode. State `CLOSED` or `OPEN` with file:line evidence.

- The seeded `g1` now has three members (`pnpm`, `npm`, `bun`). The confirmed `g2 = {pnpm, yarn}`
  steals `pnpm`, leaving `g1` at `[npm, bun]`. Confirm by reading the test source that a
  stale-snapshot planner would write back `{npm, bun}` for `g1` and therefore *fail* the
  `!members.iter().any(|m| m.group_id == "g1")` assertion.
- Confirm the live-state planner deletes `g1` via the rollback decision and therefore *passes*
  the same assertion.
- Confirm the test invokes the production path through `apply_competition_sweep` (not a parallel
  implementation) so it cannot disagree with the host code.
- Confirm no production file changed in `1e1f009` (the commit title says `test(growth): …`).

## Quick Regression Sweep

Reaffirm without re-running tests that the following still hold in `aa53f35..1e1f009`:

- All previously verified items (I1-I4 closures from Round 2 review) remain closed.
- `memory_db.rs` and `memory_db_tests.rs` still untouched at 999 / 1098 lines.
- The final diff adds 13 lines and modifies 4 in `competition_review_tests.rs`; nothing else.
- `task-t9-report.md` line counts are now 362 / 3292 / 29 / 536 / 201 / 325 / 350 / 56 / 8 / 29
  / 386. Confirm each with `(Get-Content).Count` and flag any remaining mismatch.
- Trailing whitespace remains a Minor; do not reclassify unless it now crosses a CI gate.

## Required Output

Begin with exactly `SPEC PASS/FAIL` and `QUALITY PASS/FAIL`. State the N1 closure decision first,
then any remaining Critical/Important findings, then residual Minor items. Every claim needs
file:line evidence. Use `Cannot verify from diff` for reported test/check outputs rather than
guessing. State explicitly whether the full T9 fixer round is closed.