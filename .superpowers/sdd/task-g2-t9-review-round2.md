# T9 Review — Round 2 (`aa53f35..67a6947`)

SPEC PASS
QUALITY FAIL

- **SPEC PASS** — every behavioral requirement of the original T9 brief still holds, and all four
  production/rule mandates of the fix brief are satisfied in source. **Round 1 I1, I2, I3, I4 are
  all CLOSED** with file:line evidence below.
- **QUALITY FAIL** — one new Important finding: the I1 rollback-branch regression test does not
  discriminate the fix (it passes unchanged at `5d85c13`), so fix-brief required proof #2 is not
  actually established. Everything else is Minor / report-evidence.

The full fixer round is therefore **not yet closed**: one narrow test-strength fix remains
(one extra seeded member, no production change).

## Review method

- Read-only inspection of `E:\agent-project\northing\.worktrees\growth-core-0804`; branch
  `feat/growth-core-0804`; `git status --short` empty (clean tree, no untracked artifact).
- Range `git log --oneline aa53f35..67a6947` = `bc2012b`, `5d85c13`, `0efeb29`, `67a6947`.
- Whole range re-reviewed, plus targeted diffs `5d85c13..67a6947` (fixer production + rules) and
  `0efeb29..67a6947` (the corrected test assertions).
- No source edited, no commit, no child agent dispatched, no reported test or checker run rerun.
- Where the round-1 conclusions rest on unchanged files, I re-verified the files are unchanged
  (`git diff --stat 5d85c13..67a6947` touches only 3 files: `forbidden-rules.mjs`,
  `competition_review.rs`, `competition_review_tests.rs`).

## Round 1 I1-I4 closure

| # | Finding | Decision | Primary evidence |
| --- | --- | --- | --- |
| I1 | Confirm/Rollback applied against stale pre-sweep snapshot | **CLOSED** | `competition_review.rs:227-233` live re-read, `:244` plans against `&live_members` |
| I2 | Destructive live group-id collision | **CLOSED** | `competition_review.rs:236-241` warned rejection with `continue` before any write/audit |
| I3 | Module-tree boundary hole | **CLOSED** | `forbidden-rules.mjs:2368-2426` (11 patterns kept) + `:2428-2481` (10 patterns, no `conn_locked`), no `allowPaths` |
| I4 | Incomplete trigger proof | **CLOSED** | report:99-125 = 11 production + 10 test-file failures with verbatim messages; source scan shows no planted text |

### I1 — CLOSED

- Production: `ReviewDecision::Confirm` now re-reads the DB per decision —
  `competition_review.rs:227-233` (`db.load_all_competition_members()`; on error `warn!` +
  `continue`, so other decisions survive, exactly per fix-brief:30), and the planner is fed the
  fresh list at `:244` (`plan_confirmation(&live_members, ...)`). The stale parameter is now used
  only for parsing/evidence (`:195` `group_snapshots(all_members)`), which fix-brief:33 explicitly
  permits. Rollback stays single-shot and in decision order (`:291-307` inside the ordered
  `for decision in &decisions` loop at `:209`).
- Overlapping-confirm ordering verified by construction: `record_proposals` pushes
  `ProposeAccepted` then `Confirm` per proposal in LLM array order (`propose.rs:277-291`), so the
  second confirm's live read observes the first confirm's write, and `plan_confirmation` strips the
  shared topic from the earlier group (`route.rs:78-89`).
- Test 1 (`two_overlapping_confirms_leave_topics_in_one_group`,
  `competition_review_tests.rs:244-277`) is a **real** regression guard: at `5d85c13` both confirms
  plan against the same empty snapshot, so `pnpm` lands in `g1` *and* `g2` and the assertion at
  `:272` (`pnpm_groups.len() == 1`) fails. Post-fix the second confirm dissolves `g1`
  (`route.rs:87-89`), which is why `:273` honestly asserts `npm_groups.len() == 0`.
- Test 2 (`rollback_then_confirm_does_not_recreate_rolled_back_group`, `:280-316`) confirms the
  right final state but **does not discriminate the fix** — see N1. I1 is closed on production
  evidence plus test 1, not on test 2.

### I2 — CLOSED

- `competition_review.rs:236-241`: the live snapshot is grouped (`group_snapshots`, which sorts and
  dedups members at `:152-160`) and compared against the confirmed set, which
  `record_proposals` already canonicalized (sorted/deduped, `propose.rs:222-227,286`) — so the
  comparison is normalized-set vs normalized-set. On mismatch it emits a `warn!` naming both sets
  and `continue`s **before** the first `save_competition_group` (`:255`) and before the
  `confirm_competition` audit row (`:278-288`). No rename, no new audit action, no partial write.
- The exact-set case remains the pre-existing no-op at evidence time (`propose.rs:256`), so the
  equality branch is defensive only.
- Regression test `reject_live_group_id_collision` (`competition_review_tests.rs:319-353`) is
  **not a tautology**: at `5d85c13` the same input overwrote `g1` with `{pnpm,yarn}` and returned
  `confirmed == 1`, so both `:339` (`c == 0`) and `:346` (`g1_topics == ["npm","pnpm"]`) would fail.
  It drives the real `apply_competition_sweep` through the `sweep()` helper (`:15-19`) and asserts
  final DB rows plus a zero `confirm_competition` count (`:350`).
- Residual nits: the test asserts surviving *topics* only, not the seeded metadata
  (`share 0.5`, `evidence_count 3`, `created_at_ms 100` at `:326-329`) that fix-brief:60 mentioned —
  acceptable because no write occurs at all, but weaker than asked (M6). And fix-brief:56 asked to
  "document the rejection in the host module"; the only documentation is the inline `warn!` text —
  the module doc (`competition_review.rs:1-5`) and the function doc (`:164`) say nothing about it (M5).

### I3 — CLOSED

- Production rule untouched and complete: `forbidden-rules.mjs:2368-2426`, 11 `regex:` entries
  including `\bconn_locked\b` (counted in source: 11 patterns, 2 `conn_locked` occurrences =
  pattern + message).
- New exact-file rule for the test submodule: `forbidden-rules.mjs:2428-2481`, exactly 10 `regex:`
  entries, **zero** `conn_locked` pattern, and the exemption documented in the tenth message
  (`:2478`: "...the raw guard used for poison testing does not grant self-cognition access").
- Both groups are elements of `forbiddenContentRules` (which spans `:3-2482`), so exact-file
  semantics apply (`checker.mjs:971-972`, `rule.path === path` at `:435`). `forbiddenContentUnderRules`
  starts at `:2484` and is untouched.
- No `allowPaths` in either new group; the only `allowPaths` hits in the file (`:2517`, `:3266`) are
  pre-existing entries outside both groups.
- The hermetic poisoning test survives intact (`competition_review_tests.rs:225`), which was the
  point of the chosen resolution.

### I4 — CLOSED

- report:99-100 records two planted all-symbol comments (11 symbols in the production file, 10 in
  the test file). report:104-114 lists 11 production failures and report:115-124 lists 10 test-file
  failures; I spot-verified the messages are verbatim copies of `forbidden-rules.mjs:2371-2425` and
  `:2431-2478` respectively, including the test-file exemption wording at report:124 / rules `:2478`.
- No proof artifact or planted text survives in the range: `git diff --name-only aa53f35..67a6947`
  lists exactly the 10 expected files (no `boundary_errors.txt`), `git log --diff-filter=A --
  boundary_errors.txt` is empty, and a source scan for
  `SelfCognition|self_cognition|resolve_identity_path` in both files returns nothing.
- Residual: the clean run is still a paraphrase ("Reverted to strictly clean code. Passed
  validation successfully.", report:126) rather than the captured `Core boundary check passed.`
  line that fix-brief:89-90 asked for. Classified as report-evidence M7, not a source defect — the
  clean state itself is independently verified above, and per instruction I did not run the checker.

## New findings

### Critical

None.

### Important

**N1 — `rollback_then_confirm_does_not_recreate_rolled_back_group` does not discriminate the I1
fix; fix-brief required proof #2 is not established.**
`competition_review_tests.rs:280-316` seeds a **2-member** group `g1 = {pnpm, npm}`
(`:287-290`) and confirms `g2 = {pnpm, yarn}` in the same sweep as `rollback g1` (`:299-302`).
Re-deriving the pre-fix behaviour at `5d85c13`: the stale-snapshot `plan_confirmation` removes
`pnpm` from `g1`, leaving `[npm]`, which is `< 2` and therefore **dissolved** — `route.rs:87-89`
emits `(g1, vec![])`, and an empty save is a delete (`competition_groups.rs:80-83`). So even
without the fix, `g1` is never recreated and all four assertions (`:304`, `:305`, `:309`, `:313`)
pass. The test locks the desired end state but cannot fail on the regression it is meant to guard.

Why this matters concretely: the fix brief chose a live re-read *over* an in-memory fold
(fix-brief:26-29). A future refactor to a fold would still pass the overlapping-confirm test
(N1's sibling) while recreating the rolled-back group here — exactly the scenario this test was
commissioned to prevent.

Minimal fix (test-only, no production change): seed `g1` with three members (e.g. add `vite`) so
the leftover is `{npm, vite}` (`>= 2`, a *shrink* rather than a dissolve), then assert `g1` has
zero rows after the sweep. At `5d85c13` that assertion fails; post-fix it passes.

## Regression review — no regressions found

| Requirement | Status | Evidence |
| --- | --- | --- |
| Evidence threshold exactly 3, member-set keyed, one evidence per set per sweep | intact | `propose.rs:7,222-227,238,261-263`; file unchanged since `5d85c13` |
| Double emission → 3 propose audits + 1 confirm audit | intact | `propose.rs:277-291`; `competition_review_tests.rs:85-86` (`==3` / `==1`) |
| Workspace-isolated pending state | intact | `competition_review.rs:173`; `competition_review_tests.rs:139-161` (ws-a 2, ws-b 1) |
| Confirmed groups global, no schema change | intact | no migration/schema file in `git diff --name-only aa53f35..67a6947` |
| Rollback preserves facts/weights and restores visibility | intact | `competition_review.rs:292`; `competition_review_tests.rs:121` vs `:130` (`search_facts` recovery) |
| Prompt whitelist / bad-JSON zero action / group-id sanitation / member caps / rationale truncation | intact | `propose.rs:96-126,157-160,128-140,207-209,143-149,179` (file unchanged) |
| Pure crate logic, no IO/logging/clock | intact | 0 matches for `SystemTime|Instant::now|tracing|println!|std::fs|reqwest` in `propose.rs`/`route.rs` |
| T8 full-replacement persistence reused | intact | `competition_review.rs:255,292` → `competition_groups.rs:58` |
| No self-cognition, `supersede`, hard retirement, T10/T11/T12/T4c overreach | intact | 0 matches in `competition_review.rs` / `competition_review_tests.rs` / `review/*` |
| Single `strip_json_fence` | intact | only `src/agentic/src/llm_output.rs` defines it; reused at `propose.rs:3,156` |
| `memory_db.rs` / `memory_db_tests.rs` untouched | intact | 999 / 1098 lines, absent from the range diff |
| Every touched production `.rs` < 800 | intact | propose 536, route 201, competition_review 325, competition_groups 350, turn_persist_facts 386 |
| `surfaces.md` correctly unchanged | intact | `rg "agent_memory\|judge_memory\|dream\|competition" docs/status/surfaces.md` → 0 matches |
| Warn-only, English-only, no emoji | intact | non-ASCII scan of both host files returns nothing; every failure path is `warn!` (`:230,238,256,287,293,304,313,317`) |
| 10 host tests, driving the production path | intact | 10 `#[test]`, names match the expected set; all go through `sweep()` → `apply_competition_sweep` (`:15-19`) |

The two assertion corrections in `67a6947` are both **toward** the true production semantics and I
derived the same expectations independently before reading them: `npm` really does end in zero
groups after the dissolve (`competition_review_tests.rs:273`), and `load_all_competition_members`
ordering makes the sorted comparison the correct one (`:342-346`).

## Report-evidence issues (classified separately from source correctness)

None of these affect source correctness; all are report hygiene, and each is independently
contradicted or corrected by the source facts I measured.

- **M2 — Baseline table is wrong.** report:131 states baseline `northhing-core competition_review: 10`,
  but `competition_review_tests.rs` does not exist at `aa53f35` (it appears in the range diff as a
  pure addition, 353 insertions / 0 deletions), so the pre-T9 baseline is 0 and the delta is
  hidden. The round-1 report correctly said 0.
- **M3 — Line counts are stale/wrong in three places.** Measured with `(Get-Content).Count`:
  `competition_review_tests.rs` = **353** (report:10 says 350), `forbidden-rules.mjs` = **3292**
  (report:12 says 3228), `agent_memory/mod.rs` = **29** (report:15 says 28). Correct in the report:
  propose 536, route 201, competition_review 325, competition_groups 350, AGENTS.md 56,
  review/mod.rs 8, turn_persist_facts 386, and the 999/1098 untouched claim.
- **M4 — Fixer-section file:line citations do not match the committed code.** report:26 cites
  `competition_review.rs:242-250` for the live-state load (actual `227-233`); report:31 cites
  `:251-257` for the collision check (actual `236-241`); report:27/28/32 cite tests `228-261` /
  `263-294` / `296-324` (actual `244-277` / `280-316` / `319-353`). The mechanisms are real — only
  the coordinates are wrong — but the orchestrator cannot use these citations as-is.
- **M7 — Clean checker run still paraphrased** (report:126) instead of the captured
  `Core boundary check passed.` line required by fix-brief:89-90.
- **M8 — The `cargo check` contradiction survives unchanged.** report:153-155 shows the command
  completing with 19 warnings in 2m04s while report:162 still says it "timed out ... over 120s".
  Consequence unchanged from round 1: the "19 warnings = baseline" claim (report:157) is
  **Cannot verify from diff**.
- **Cannot verify from diff:** the `185` / `10` / `34` / `38` / `4` test result lines
  (report:139-151), the 19-warning count, and the checker outputs — reported runs were not rerun
  per instruction. Note the `10 passed` line is at least *consistent* with the 10 `#[test]`
  functions I counted in source.

## Trailing whitespace (round-1 M1) — kept as Minor, with a note

`git diff --check aa53f35..67a6947` reports **64** trailing-whitespace diagnostics (round 1: 58):
`src/agentic/src/review/propose.rs` 12 (production lines `187`, `308`; rest in `#[cfg(test)]`),
`src/agentic/src/review/route.rs` 10 (production `50`, `57`, `97`), and
`competition_review_tests.rs` **42** (was 36 — the three new tests added 6 more).
All are whitespace-only blank lines with zero functional impact; the host production file
`competition_review.rs` remains clean. No CI `cargo fmt --check` gate was found
(`package.json:22` exposes `fmt:rs` = `scripts/format-changed-rust.mjs`), so the classification
stays **Minor**. Two observations for triage: (a) it was correctly out of scope for this fixer
(the fix brief limited scope to I1-I4, fix-brief:15-16), and (b) it nevertheless *grew* after being
flagged, so a final `pnpm run fmt:rs` pass on the three files is worth folding into branch
finishing — never bare `cargo fmt`.

## Residual Minor items for final triage

Carried over from round 1 (all still present, all verified unchanged):

- **M-R3** Pending state is overwritten with empty-derived state after a failed KV read
  (`competition_review.rs:186-189` → `:311-315`).
- **M-R4** Dead imports `AIClient` / `Arc` (`competition_review.rs:7,19`), masked by
  `#![allow(unused_imports)]` (core `lib.rs:4`), matching the dream precedent (`dream.rs:9,16`).
- **M-R5** Global cadence gate key vs per-workspace evidence (`competition_review.rs:53,61`;
  brief-mandated, dream precedent `dream.rs:52`).
- **M-R6** `MAX_PENDING_PROPOSALS` enforced only on the push path (`propose.rs:310`).
- **M-R7** Propose audit rows scatter across `competition:<id>` when the LLM renames a set
  (`propose.rs:271` → `competition_review.rs:214`).
- **M-R8** `bad_json_zero_actions_gate_advanced` proves zero audit rows only via the return tuple
  (`competition_review_tests.rs:200-206`).
- **M-R9** `keyword_weights` SQL lives in `competition_groups.rs:121-135` (brief-mandated by the
  999-line freeze on `memory_db.rs`).
- **M-R10** Inline sweep adds up to 15s to turn finalize on gate-open turns
  (`turn_persist_facts.rs:237-238`, `competition_review.rs:26,103-104`).
- **M-R11** A propose equal to a group being rolled back in the same sweep is skipped as a
  live-set no-op (`propose.rs:256`, pre-sweep snapshot).

New Minors from this round:

- **M5** The I2 rejection policy is not documented in the host module as fix-brief:56 required;
  only the inline `warn!` (`competition_review.rs:238`) explains it. The module doc
  (`:1-5`) and `apply_competition_sweep`'s doc (`:164`) are silent.
- **M6** `reject_live_group_id_collision` asserts surviving topics but not the seeded metadata
  (`competition_review_tests.rs:326-329,342-346`) that fix-brief:60 named.
- **M9 (new behavioural consequence of the mandated I2 fix)** A rejected confirmation is
  unrecoverable *and* costly: `record_proposals` has already removed the pending entry at the
  threshold (`propose.rs:290`) before the host rejects it (`competition_review.rs:239`), so three
  sweeps of evidence are discarded and the cycle repeats — three fresh `propose_competition` audit
  rows every three sweeps — until the LLM chooses a different `group_id`. Relatedly, the
  brief-supported "grow an existing group under the same id" path (`route.rs:180-200`,
  `reslot_preserves_created_at_in_confirmed_group`) is now unreachable from the host by design;
  the crate function still supports it, which T10 (merge/dedup) will likely need. Worth one doc
  line and a note in the T10 brief; safe as-is (the fix brief explicitly preferred safe failure
  over destructive reinterpretation, fix-brief:56-57).

## Fixer-round status

- **Round 1 I1-I4: all four CLOSED.** Production semantics, boundary rules, and trigger-proof
  evidence are verified from source and report content, independently of the reported runs.
- **The fixer round is NOT fully closed.** One Important item (N1) remains: a test-only change —
  seed the rollback test's group with a third member so the assertion actually fails without the
  live re-read. No production change, no scope expansion; it can go to the same fixer session with
  M5 (one host doc sentence) and optionally M6 (assert the seeded metadata) folded in.
- Recommended dispatch for the next fixer: N1 + M5 + M6 only, naming
  `competition_review_tests.rs` and `competition_review.rs`; require the report to include the
  corrected line counts (M3), the corrected fixer citations (M4), a truthful baseline row (M2),
  the captured clean checker line (M7), and removal of the stale `cargo check` deviation sentence
  (M8). All other Minors go to branch-level final triage, together with the `pnpm run fmt:rs`
  whitespace pass.
