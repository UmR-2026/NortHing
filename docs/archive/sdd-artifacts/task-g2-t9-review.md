# T9 Review — competition recognition (range `aa53f35..5d85c13`)

SPEC FAIL
QUALITY FAIL

Two separate verdicts, each justified below. The two **prior fixer defects are verified fixed
and that fixer round is closed**, but a new fixer round is required: 4 Important findings
(0 Critical). Residual Minor items are listed for final triage.

## Review method and scope

- Read-only inspection of `E:\agent-project\northing\.worktrees\growth-core-0804`
  (branch `feat/growth-core-0804`, clean tree, `git status --short` empty).
- Range verified: `git log --oneline aa53f35..5d85c13` = `bc2012b` (impl) + `5d85c13` (fixer).
- Diffstat: 10 files, `1374 insertions(+), 1 deletion(-)`; no schema/migration file, no
  `memory_db.rs` / `memory_db_tests.rs` / `growth_adapter.rs` / `dream.rs` / `docs/status/surfaces.md`
  entry in the diff.
- No source edited, no commit, no child agent dispatched, no reported test rerun. The
  boundary checker was **not** executed (it is one of the report's verification commands).

## Verified capacity / hygiene facts (independent)

| Fact | Result |
| --- | --- |
| `memory_db.rs` line count | 999 — unchanged, absent from diff (PASS) |
| `memory_db_tests.rs` line count | 1098 — unchanged, absent from diff (PASS) |
| Touched production `.rs` < 800 | propose.rs 536, route.rs 201, competition_review.rs 309, competition_groups.rs 350, turn_persist_facts.rs 386 (PASS) |
| `docs/status/surfaces.md` unchanged | Correct: `rg "agent_memory|judge_memory|dream|competition" docs/status/surfaces.md` → 0 matches; the report's reason (report:120) is file-verified (PASS) |
| Emoji / non-ASCII in new code | Only intended CJK truncation fixture `propose.rs:431` and pre-existing CJK doc text in `competition_groups.rs:293-299` (outside the added hunk). No emoji (PASS) |
| `supersede` / hard retirement in review route | 0 matches in propose.rs / route.rs / competition_review.rs / competition_review_tests.rs (PASS) |
| `strip_json_fence` copies | single definition `src/agentic/src/llm_output.rs`; reused at `propose.rs:3,156` (PASS — T5b interface honored) |
| Crate-side purity | 0 matches for `SystemTime|Instant::now|tracing|println!|std::fs|reqwest` in propose.rs + route.rs (PASS) |
| `git diff --check` | **58 trailing-whitespace diagnostics** in 3 files — see M1 (FAIL, Minor) |

## Prior fixer findings — verification

1. **Double-emission test (CLOSED, correct resolution).** `propose.rs:277-291` emits
   `ProposeAccepted{evidence:3}` **and** `Confirm` on the threshold sweep. This is required by
   spec, not a bug: brief:140 demands one `propose_competition` row per accepted/reinforced
   propose and brief:196 demands `propose_competition ×3 + confirm_competition ×1`. The fixer
   correctly changed the test, not production. Verified assertions:
   `propose.rs:458` `assert_eq!(d3.len(), 2)`, `propose.rs:461` exactly one
   `ProposeAccepted { evidence: 3, .. }`, `propose.rs:463-471` exactly one `Confirm` with
   `group_id == "g-new"`, `propose.rs:472` pending removed.
2. **Host audit-count assertions (CLOSED, claim true).**
   `competition_review_tests.rs:85` `propose_competition` count `== 3`;
   `competition_review_tests.rs:86` `confirm_competition` count `== 1`. Exact counts, not `>=`.
3. **`load_competition_share_map` indentation (CLOSED).** The function is at column 0
   (`competition_groups.rs:228`), and the final diff for that file is a single hunk at
   `@@ -118,6 +118,23 @@` adding only `list_top_keyword_weights`. No whitespace churn on the
   T8 helper. The max-share read-side defense is intact (`competition_groups.rs:247`).

## SPEC decision: FAIL

Compliant (each verified in source):

- Evidence threshold exactly 3 (`propose.rs:7`, consumed at `competition_review.rs:201`).
- Consistency key is the normalized member set, not `group_id` (`propose.rs:222-227,268`);
  the latest sweep's sanitized id wins (`propose.rs:271`); one evidence per set per sweep
  (`propose.rs:238,261-263`).
- Evidence isolated per workspace via `judge_mom` KV key
  `competition_pending:<workspace_key>` (`competition_review.rs:173`, key derived at
  `competition_review.rs:131` with the dream derivation `to_string_lossy`, cf. `dream.rs:65`).
  Cross-workspace non-accumulation proven by `competition_review_tests.rs:139-161`
  (ws-a evidence 2, ws-b evidence 1, no confirm).
- Confirmed groups global, no `workspace_key` column: no schema change in the diff; writes go
  through the shipped T8 `save_competition_group` (`competition_groups.rs:58`).
- Forced single membership implemented as a pure re-slot planner (`route.rs:17-117`):
  removal from other groups (`route.rs:78-84`), dissolve `<2` (`route.rs:87-89`),
  renormalization of shrunk groups (`route.rs:92-96`), live-weight shares with 1.0 baseline
  (`route.rs:43-51`), `created_at_ms` preserved (`route.rs:64,107`), one full-replacement
  write per affected group only (`route.rs:68,111`). `normalize` preserves input order and
  cardinality (`topics/competition.rs:168-193`), so the `unwrap()` at `route.rs:100` cannot
  panic. — **but see I1/I2 for the application layer.**
- Rollback single-shot without an evidence gate (`propose.rs:326-337`), deletes only
  competition rows (`save_competition_group(gid, &[])` → `DELETE FROM competition_groups
  WHERE group_id=?1`, `competition_groups.rs:80-83`), facts and keyword weights untouched,
  audit row written (`competition_review.rs:279-289`).
- Audit vocabulary: reviewer `judge-mom` and actions `propose_competition` /
  `confirm_competition` / `rollback_competition` (`competition_review.rs:215-216,265-266,282-283`);
  synthetic `competition:<group_id>` documented in the module doc (`competition_review.rs:3-5`)
  and applied consistently (`:214,264,281`).
- Bad JSON → zero actions (`propose.rs:157-160`, swallow-and-continue like `parse_verdicts`);
  real gates for whitelist + canonical substitution (`propose.rs:191-198`), non-string array
  rejection (`:199-206`), `<2`/`>6` full drop (`:207-209`), dedup (`:195`), group-id sanitize
  or drop (`:128-140,170-173`), char-based rationale truncation at `MAX_REASON_CHARS`
  (`:143-149,179`, reusing `review/verdict.rs:14`).
- Host is warn-only (no `Result`, every failure a `warn!`: `competition_review.rs:43,69,77,111,
  116,178,187,221,240,271,277,288,297,301`), 24h cadence gate (`:24,53,61`), gate advanced on
  every early-exit path (`:83,112,120,127,300`), call site exactly where the brief specified —
  immediately after `run_dream_sweep` (`turn_persist_facts.rs:237-238`) — and does not read or
  write self-cognition (0 matches for `self_cognition|SelfCognition`).
- No early T10/T11/T12/T4c work (0 matches for `merge_groups|negation|garden|facade|retire`).
- T8 reuse rather than duplication: `CompetitionMember`, explicit group id, metadata columns,
  and full-replacement `save_competition_group` / `load_all_competition_members` are used
  as-is; the T8 write-side and read-side defenses are untouched.
- Required test inventory complete: 14 crate tests (propose 12 + route 2) covering every
  bullet of brief:179-190, and 7 host tests covering brief:194-209 one-for-one.

Spec failures:

- **I1** — user decision 1 ("one topic belongs to at most one group" is a system invariant) is
  only enforced against a pre-sweep snapshot; a single sweep with two confirms, or a rollback
  plus a confirm, can leave a topic in two groups or resurrect a rolled-back group.
- **I4** — brief:172-174 mandates proving that *every* pattern of the new boundary group fires;
  the report proves one of eleven.

## QUALITY decision: FAIL

Structure, naming, purity, and test design are largely good: decisions are pure and
port-free, the post-LLM part is extracted into a synchronous `apply_competition_sweep`
(`competition_review.rs:165`) exactly as T5b did, the host mirrors `dream.rs` step for step,
tests are behavioural rather than fixture tautologies (rollback proves real visibility
recovery via `search_facts`, `competition_review_tests.rs:121` vs `:130`), and the
mutex-poisoning fault is isolated (each `MemoryDb` owns its own `Mutex<Connection>`,
`memory_db.rs:93-95`; the temp DB path is per-test UUID, `competition_review_tests.rs:5`;
the panicking thread is joined and asserted, `:230`) so it cannot contaminate other tests.

Quality fails on I1, I2 and I3 below.

---

## Findings

### Critical

None.

### Important

**I1 — Confirm/Rollback are applied against a stale pre-sweep snapshot; the single-membership
invariant and rollback can both be broken within one sweep.**
`competition_review.rs:132-139` snapshots `all_members` once, and every decision in the loop
(`:209-293`) plans against that same immutable snapshot: `plan_confirmation(all_members, ...)`
at `:227-235`. Nothing re-reads `load_all_competition_members` between decisions, and
`plan_confirmation` only writes groups it considers "affected" (`route.rs:86-113`).

Two reachable consequences:

1. Two overlapping sets confirming in the same sweep (e.g. `{a,b}` and `{b,c}`, each at
   evidence 2 and both re-proposed) produce two `Confirm` decisions (`propose.rs:284-288` is
   reached per proposal). The second confirm cannot see the group the first one just wrote, so
   `b` is left in both new groups — a direct violation of user decision 1.
2. `Rollback gX` followed by a `Confirm` whose member set contains a member of `gX`: the
   rollback deletes `gX` (`:276`), then `plan_confirmation` sees `gX` in the stale snapshot,
   plans a "shrunk" write for it, and `save_competition_group` **recreates** `gX`
   (`route.rs:111` → `:239`) while a `rollback_competition` audit row claims it was removed.

The T8 read-side max-share defense (`competition_groups.rs:247`) keeps suppression sane, so
this degrades rather than corrupts — hence Important, not Critical. Suggested fix: apply
decisions against live state (re-read members before each `Confirm`, or fold planned writes
back into an in-memory member list) and/or order rollbacks before confirms.

**I2 — A confirmed `group_id` that collides with an unrelated live group silently destroys
that group with no audit trace.**
`plan_confirmation` treats `confirmed_group_id` as the confirmed group unconditionally
(`route.rs:53-68`): the full-replacement write drops every pre-existing member of that id that
is not in the confirmed set. This is correct and tested for the intended re-confirm case
(`route.rs:180-200`), but the same path fires when the LLM reuses a live id for a *different*
set — plausible because the prompt lists existing group ids and instructs id reuse
(`propose.rs:106,121`), and the live-set no-op guard (`propose.rs:256`) only skips *equal*
sets. The confirm audit row cannot record the loss either: `competition_review.rs:242` excludes
`gid == group_id` from `affected_other_groups`, so the destroyed members appear in no
`reason` (`:252-260`) and in no `rollback_competition` row. This contradicts brief:140's
"evidence trail is human-reconstructible". Suggested fix: detect the collision (live id whose
member set differs from the confirmed set) and either re-slot it as an "other group" or reject
the confirm with a warn.

**I3 — The new boundary group has a coverage hole inside its own module tree, and the delivery
uses the banned symbol in that hole.**
`checkForbiddenContent(rule.path, rule.patterns)` matches exactly one file per rule
(`checker.mjs:971-972`; exact `rule.path === path` semantics also at `:434-435`), and the new
group covers only `.../agent_memory/competition_review.rs` (`forbidden-rules.mjs:2368`).
`competition_review_tests.rs` is compiled as a submodule of that very file
(`competition_review.rs:307-309`, `#[cfg(test)] #[path = ...] mod tests;`) yet is covered by no
rule — and it calls the explicitly banned escape hatch at `competition_review_tests.rs:225`
(`db2_clone.conn_locked().unwrap()`). The precedent module keeps its tests inline and therefore
inside the rule's reach (`dream.rs:251-252`), so this hole is new. Note the pattern set itself
is a faithful copy of the T7a judge_memory group (11 patterns, identical regexes;
`forbidden-rules.mjs:2308-2366` vs `:2368-2426`) and the planted-violation proof does target
the new production file (report:84-86 message matches `forbidden-rules.mjs:2422-2424`
verbatim). Cheapest fix: add a `forbiddenContentRules` group for
`competition_review_tests.rs` with the ten self-cognition patterns (keeping the `conn_locked`
exception explicit and justified), or replace the poisoning fault with one that does not need a
raw guard. This is a boundary/D9-coverage decision, so record whichever option is chosen.

**I4 — Boundary-rule proof evidence is incomplete (10 of 11 patterns unproven).**
brief:172-174: "Prove every pattern fires: temporarily plant a violation in the new file, run
`node scripts/check-core-boundaries.mjs`, observe the expected error, then restore. Record the
proof in the report." The report records exactly one planted violation and one checker line
(report:84-86), for `\bconn_locked\b` only. The ten self-cognition patterns have no recorded
trigger output. The report also contains no clean-run output for
`node scripts/check-core-boundaries.mjs` at all (the Verification Executions block,
report:96-116, lists only cargo commands). Fix: plant one line containing all eleven symbols,
capture the eleven-failure checker output plus the clean run after restore, and record both.

### Minor (for final triage)

- **M1 — `git diff --check` is not clean: 58 trailing-whitespace diagnostics.**
  `src/agentic/src/review/propose.rs` 12 (production lines `187`, `308`; the other 10 in
  `#[cfg(test)]`), `src/agentic/src/review/route.rs` 10 (production lines `50`, `57`, `97`; 7 in
  tests), `.../competition_review_tests.rs` 36. All are whitespace-only blank lines; zero
  functional impact and the host production file `competition_review.rs` is clean. No CI
  `cargo fmt --check` gate was found (`package.json:22` exposes `fmt:rs` =
  `scripts/format-changed-rust.mjs`), so this is cosmetic — **Minor**. Remediation must use
  `pnpm run fmt:rs` or a manual strip, never bare `cargo fmt`.
- **M2 — Report wording is stale/self-contradictory; treat its non-test numbers as unverified.**
  report:113-116 shows `cargo check -p northhing-core --features product-full` completing with
  "19 warnings ... Finished in 2m 04s", while report:122 states the same command "timed out due
  to cache congestion limits over 120s". Both cannot be true; classify as **Minor (report
  hygiene, no source impact)** — but the consequence is that the "warning count matches the
  baseline exactly (19)" claim (report:117) is **Cannot verify from diff**. Two line counts are
  also wrong: `forbidden-rules.mjs` is 3237 lines, not 3188 (base was 3175, +60 in this range),
  and `agent_memory/mod.rs` is 29 lines, not 28. The verified counts elsewhere in report:7-16
  are correct, and the +16 crate-test delta (169 → 185) is consistent with the 14 propose + 2
  route tests I counted in source.
- **M3 — A transient pending-state read failure silently discards accumulated evidence.**
  `competition_review.rs:186-189` falls back to an empty `pending` on a read error (correct,
  warn-only), but `:295-299` then unconditionally overwrites the KV blob with that empty-derived
  state. Consider skipping the write when the load failed.
- **M4 — Dead imports.** `competition_review.rs:7` (`AIClient`) and `:19` (`Arc`) are never
  named in the file. Harmless today: masked by `#![allow(unused_imports)]` (core `lib.rs:4`) and
  copied verbatim from the dream precedent (`dream.rs:9,16`), which is why the warning count can
  still match the baseline.
- **M5 — The cadence gate is global while evidence is per-workspace.** `competition_review.rs:53,61`
  uses a workspace-less key `competition_review_last_at`, so one workspace's sweep blocks all
  others for 24h and a multi-workspace user's per-workspace evidence accrues slowly or starves.
  This is exactly what the brief specified (brief:119-121) and matches `dream.rs:52`, so it is a
  documented design consequence, not a deviation.
- **M6 — `MAX_PENDING_PROPOSALS` is enforced only on the push path** (`propose.rs:310`, inside
  the `matched_idx.is_none()` branch), so an over-long persisted list is not trimmed until the
  next brand-new set arrives.
- **M7 — Propose audit rows scatter when the LLM renames a set.** The pending entry adopts the
  latest `group_id` (`propose.rs:271`) and the audit `fact_id` is derived from it
  (`competition_review.rs:214`), so the three `propose_competition` rows for one set can land on
  different `competition:<id>` keys. Consistent with the brief's "latest id wins" rule, but it
  weakens `reviews_for_fact`-based reconstruction. Worth a doc line.
- **M8 — `bad_json_zero_actions_gate_advanced` proves "zero audit rows" only indirectly**
  (`competition_review_tests.rs:200-206` asserts the `(0,0,0)` return and the gate value, not a
  DB row count). brief:205 asked for zero audit rows explicitly.
- **M9 — `keyword_weights` SQL now lives in `competition_groups.rs`** (`:121-135`) while the
  other `keyword_weights` accessors live in `memory_db.rs`. Brief-mandated (brief:154-157) because
  `memory_db.rs` is frozen at 999 lines; note it so the next module split re-homes it. The query
  itself is deterministic and bounded (`ORDER BY weight DESC, keyword ASC LIMIT ?1`).
- **M10 — Worst-case turn-finalize latency roughly doubles on gate-open turns.** The sweep is
  awaited inline right after the dream sweep (`turn_persist_facts.rs:237-238`) inside the awaited
  finalize path (`turn_persist.rs:334-338`), each with a 15s LLM timeout
  (`competition_review.rs:26,103-104`). It still cannot block on failure (warn-only) and it only
  runs on fact-producing turns (early return at `turn_persist_facts.rs:146-148`) — the known
  coupling T12 owns.
- **M11 — Same-sweep rollback/propose interaction.** A propose whose set equals a group being
  rolled back in the same sweep is skipped as a live-set no-op (`propose.rs:256`) because
  `existing_groups` is the pre-sweep snapshot, so it gains no evidence that sweep.

## Cannot verify from diff

- Test pass counts, the 7 host tests' actual results, and the "19 warnings = baseline" claim —
  reported tests were deliberately not rerun; M2 shows the report's own `cargo check` evidence is
  self-contradictory.
- A clean `node scripts/check-core-boundaries.mjs` run, and whether the ten unproven self-cognition
  patterns actually fire (see I4). The checker was not executed in this read-only review.
- Whether the planted violation was restored exactly (the tree is clean at `5d85c13` and no
  `conn_locked` occurrence exists in `competition_review.rs`, which is consistent with a correct
  restore).

## Fixer round status

- Prior fixer round (double-emission test + `load_competition_share_map` indent): **closed**,
  both verified in source, and the fixer's production-vs-test judgement was correct.
- New fixer round **required** for I1, I2, I3, I4. I3 and I4 are boundary/evidence work;
  I1 and I2 are one coherent change ("apply confirmations against live state, and refuse or
  record group-id collisions") and should go to a single fixer with both items plus the named
  test files (`competition_review_tests.rs` for the two-confirm and rollback-then-confirm cases,
  `route.rs` tests unchanged unless the planner signature moves).
- If the orchestrator prefers to accept I1/I2 as designed behaviour, that is a user decision, not
  a review pass: both contradict the fixed user decision 1 invariant text and the
  "human-reconstructible audit trail" requirement, so the finding plus the plan text must go to
  the user together.
