# T8 review round 2

## Review package

- Source worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
- Exact BASE: `8b64aa8`
- Exact HEAD: `aa53f35`
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-t8-brief.md`
- Fix brief: `E:\agent-project\northing\.superpowers\sdd\task-t8-fix-brief.md`
- Round 1 review: `E:\agent-project\northing\.superpowers\sdd\task-t8-review.md`
- Final implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t8-report.md`
- Final diff: `E:\agent-project\northing\.superpowers\sdd\task-t8-diff.patch`
- Write only to: `E:\agent-project\northing\.superpowers\sdd\task-t8-review-round2.md`

Review the full BASE-to-HEAD range, not only the two fixer commits. The worktree is read-only
for review. Do not rerun the implementer's reported tests, edit source, commit, or dispatch
another agent.

## Round 1 findings that must be independently closed

1. Same-turn two-member boost no longer loses the first update. Inspect the working group map,
   save-failure behavior, and the new regression test.
2. `save_competition_group` validates/binds the explicit group id and
   `rehydrate_group` cannot produce an empty or mismatched id. Check fresh-group and mismatch
   tests and T9 compatibility.
3. Duplicate topic membership across groups resolves deterministically and conservatively
   (largest share / least suppression), independent of SQLite row order. Check test coverage.
4. The token/FTS association and limits are documented accurately; no false claim that all
   unrelated facts are unaffected remains. Verify code comments, report, global scope, CJK
   bigram and generic ASCII limits, and the non-mutating/touch behavior.
5. `memory_db.rs` is below 1000 lines without an allow-god-file shortcut and the extracted
   helper preserves exact retrieval behavior. Check line count evidence and helper tests.

## Additional regression checks

- User ruling remains exact: share `<0.15` AND live keyword weight `<=1.0`; no second heat
  score and no `0.1` floor.
- Group shares still sum to 1, boost cap/invalid handling remain intact, and ungrouped topics
  remain on the old path.
- Retrieval does not delete facts, mutate status, call supersede, or touch self-cognition.
- The new helper does not change empty-query, CJK/FTS, workspace, multiple-keyword, BM25, or
  two-layer score semantics beyond the documented suppression gate.
- Transaction/migration behavior, ordering, and metadata preservation remain correct.
- Production Rust line counts and boundary rules remain compliant.
- T9/T10/T12 are not implemented early and the port remains sufficient for the next task.

## Required output

Begin with `SPEC PASS/FAIL` and `QUALITY PASS/FAIL`. List any remaining findings by Critical,
Important, Minor with file and line. If no Critical/Important remain, say the fixer round is
closed and list residual Minor items for final triage. Use `Cannot verify from diff` for facts
that cannot be established without rerunning tests; do not guess.
