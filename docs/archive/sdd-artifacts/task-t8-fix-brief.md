# T8 fixer round

## Position and evidence

- Fix only in `E:\agent-project\northing\.worktrees\growth-core-0804`.
- Continue the original T8 task on the existing implementation session.
- Current source HEAD: `99d82dd`; original BASE remains `8b64aa8`.
- Review: `E:\agent-project\northing\.superpowers\sdd\task-t8-review.md`.
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-t8-brief.md`.
- Update the final implementation report at `E:\agent-project\northing\.superpowers\sdd\task-t8-report.md`.
- Refresh the complete final diff at `E:\agent-project\northing\.superpowers\sdd\task-t8-diff.patch`.
- Do not edit plan, ledger, handoff, or model notes. Do not dispatch child agents. Commit the
  fixer changes on `feat/growth-core-0804`.

## Must fix (all five Important findings)

1. **I1 stale same-turn group state** (`growth_adapter.rs:396-427`): when one turn mentions
   two distinct members of one group, the second boost must build on the first durable result,
   not overwrite it from the pre-turn snapshot. Use the existing group load API or update the
   in-memory group after each save. Add a real regression test with two members mentioned in
   one turn and assert both moves are preserved relative to an unmentioned sibling.
2. **I2 group-id mismatch / empty-group creation** (`memory_db/competition_groups.rs:62-94`,
   `competition.rs:117-125`): make save bind the explicit group id consistently and make
   `rehydrate_group` take an explicit group id rather than silently falling back to `""`.
   Add tests for a fresh group and mismatch/identity behavior so T9's future create-group
   cycle cannot delete one id and write another.
3. **I3 nondeterministic cross-group share lookup** (`competition_groups.rs:187-208`): the
   same topic may belong to multiple groups. Resolve its retrieval share deterministically and
   conservatively so the least suppression wins (for example aggregate with `f64::max`). Add
   a test with two groups and different shares. Do not rely on SQLite row order.
4. **I4 association claim and limits** (`memory_db.rs:605-667`, report): retain the current
   token/FTS-based suppression behavior, but document it accurately in code and in the final
   report. Explicitly state that suppression operates on segmented keyword overlap, including
   CJK bigrams and generic ASCII tokens; it can hide a fact when its only matching keyword is
   suppressed; it is global because the group table has no workspace key; and suppressed facts
   are not touched by retrieval. Remove claims that unrelated facts are guaranteed unaffected.
   Do not narrow or redesign this product behavior in the fixer without an explicit user
   decision.
5. **I5 god-file breach** (`memory_db.rs` now 1054 lines): extract the approximately 62-line
   suppression decision block from `search_facts` into the competition-groups access module
   or another focused module, keeping `memory_db.rs` below 1000 lines. Preserve behavior and
   make the extracted helper directly testable where practical. Do not add
   `// allow-god-file` as a shortcut.

## Allowed adjacent cleanup

- Fold M2 (candidate keyword allocation per fact) into the extraction if it remains a small,
  behavior-preserving change.
- Fold M4 (unreachable zero-weight branch) into the extraction.
- Make `TopicDbStore` `pub(crate)` if the review finding is still accurate.
- Correct report test counts and explicitly note the substituted full core test command.
- Add the missing host warn-only test only if it is hermetic and small; otherwise report it as
  a remaining limitation.

Do not fix unrelated Minor findings in this round.

## Constraints

Preserve the user ruling: normalized share strictly `<0.15` AND live keyword weight `<=1.0`,
no second activity score, no `0.1` floor. Preserve warn-only behavior, no fact deletion/status
mutation/supersede, English-only source/logs, no emoji, no `cargo fmt`, and production Rust
files below 800 lines where possible (the hard requirement is below 1000 for `memory_db.rs`).

## Verification

After fixes, run and report commands with the required PATH prefix:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo test -p northhing-core --lib --features product-full
cargo check -p northhing-core --features product-full
node scripts/check-core-boundaries.mjs
```

Do not rerun `cargo fmt`. Report exact output and any environmental limitation. The report
must include the final commit range `8b64aa8..<final head>` and the final file line counts.
