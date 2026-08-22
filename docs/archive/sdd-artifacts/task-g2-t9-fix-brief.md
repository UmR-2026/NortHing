# T9 Fixer Round — Review Findings I1-I4

## Position

- Work only in `E:\agent-project\northing\.worktrees\growth-core-0804`.
- Branch: `feat/growth-core-0804`; current reviewed HEAD: `5d85c13`.
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-brief.md`.
- Review with full evidence: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review.md`.
- Commit fixes as one NEW commit; never amend `bc2012b` or `5d85c13`.
- Update report in place: `E:\agent-project\northing\.superpowers\sdd\task-t9-report.md`.
- Regenerate full diff for `aa53f35..HEAD` at
  `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-diff.patch`.
- Do not dispatch child agents, edit the plan/ledger/model notes, or touch the main checkout source.

Fix all four Important findings below. Do not broaden scope into T10/T11/T12/T4c. The review's
11 Minor findings remain final-triage items and are not part of this fixer.

## I1 — Apply Decisions Against Live State

Current defect: `apply_competition_sweep` receives one pre-sweep `all_members` snapshot and passes
it to every `plan_confirmation`. Two overlapping confirms can leave a topic in two groups; a
rollback followed by a confirm can recreate the rolled-back group from the stale snapshot.

Required fix:

- Before every `ReviewDecision::Confirm`, load the current members from the DB with
  `load_all_competition_members()` and plan against that live state. This is preferred over an
  in-memory fold because prior full-replacement saves are warn-only and may partially fail across
  groups; a re-read reflects what actually committed.
- If the live read fails, warn and skip that confirmation; continue processing other decisions.
- Rollback remains single-shot and should occur in decision order. A following confirm must see
  the post-rollback state and must not recreate the deleted group as a side effect.
- Keep the input `all_members` only for proposal parsing/evidence decisions if still needed; do
  not use it for applying confirmations.

Add host tests in `competition_review_tests.rs` that prove:

1. Two overlapping sets reaching confirmation in the same sweep leave every topic in exactly one
   final group (the later decision wins by the deterministic decision order).
2. A rollback followed in the same sweep by a confirm containing a former member of the rolled
   back group does not recreate the rolled-back group; the rollback audit remains truthful.

## I2 — Reject Live Group-ID Collisions

Current defect: confirming a different member set under an already-live `group_id` full-replaces
and silently destroys the live group. The prompt exposes live ids, so this is reachable.

Required fix:

- Defensively validate each confirmation against the live members loaded for I1.
- If `group_id` already exists and its normalized member set differs from the confirmed set,
  reject the confirmation with `warn!`; do not save any group writes and do not write a
  `confirm_competition` audit row.
- An exact set match remains the existing no-op at proposal/evidence time.
- Do not silently rename the group, overwrite unrelated members, or create a new audit action.
- Document the rejection in the host module. The safe failure is preferable to destructive
  reinterpretation.

Add a host regression test that seeds a live group, drives a threshold confirmation using the
same id with a different set, and proves the original rows and metadata remain intact and no
`confirm_competition` audit row is written.

## I3 — Close the Boundary Module-Tree Hole

Current defect: the production rule targets exactly `competition_review.rs`, while the external
`competition_review_tests.rs` submodule is uncovered and uses `conn_locked` for the hermetic
poisoning test.

Required resolution (preserve the good isolated fault test):

- Add a second exact-file `forbiddenContentRules` group for
  `src/crates/assembly/core/src/service/agent_memory/competition_review_tests.rs`.
- Copy the ten self-cognition access patterns from the production rule, but intentionally omit
  the `conn_locked` pattern. The reason/message must explicitly state that the test-only raw
  guard is allowed solely for per-instance mutex poisoning and does not grant self-cognition
  access.
- Do not add `allowPaths` and do not weaken the production file's 11-pattern rule.

## I4 — Prove Every Boundary Pattern

The report currently proves only the production `conn_locked` pattern and records no clean run.

Required evidence, written into the report:

1. Temporarily plant text in `competition_review.rs` containing all eleven production-banned
   symbols, run `node scripts/check-core-boundaries.mjs`, and capture all eleven expected failures.
2. Temporarily plant text in `competition_review_tests.rs` containing all ten self-cognition
   symbols, run the checker, and capture all ten expected failures from the new test-file rule.
3. Restore both files exactly, run the checker again, and capture the clean
   `Core boundary check passed.` output.
4. Confirm `git diff` contains no planted proof text.

The planted text may be a temporary comment because the checker scans content; it must never be
committed. Every proof line in the report needs the exact checker message and file path.

## Verification

Run from the worktree with:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo test -p northhing-core --features product-full competition_review
cargo test -p northhing-core --features product-full growth_adapter
cargo test -p northhing-core --features product-full memory_db
cargo test -p northhing-core --features product-full prompt_injection
cargo check -p northhing-core --features product-full
node scripts/check-core-boundaries.mjs
```

Report exact result lines and warning count. Also record exact line counts via
`(Get-Content).Count`, confirm `memory_db.rs` and `memory_db_tests.rs` remain untouched, and include
file:line evidence for each I1-I4 closure. Update the report's final range and commit list. Then
regenerate the full diff and reply with status, new commit hash, test outputs, and report/diff
confirmation.
