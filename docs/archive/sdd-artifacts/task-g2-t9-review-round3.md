# T9 Review — Round 3 (`aa53f35..1e1f009`)

SPEC PASS
QUALITY PASS

**N1 is CLOSED.** No Critical or Important findings remain in the range. **The full T9 fixer round
is closed for source**; what is left are Minor final-triage items, of which four are report-text
hygiene (no code change) and one is the carried-over trailing whitespace.

## Review method

- Read-only inspection of `E:\agent-project\northing\.worktrees\growth-core-0804`; branch
  `feat/growth-core-0804`; `git status --short` empty (clean tree, no untracked artifact).
- Range `git log --oneline aa53f35..1e1f009` = `bc2012b`, `5d85c13`, `0efeb29`, `67a6947`,
  `1e1f009`.
- `git diff --stat 67a6947..1e1f009` = **1 file, 13 insertions, 4 deletions**, all in
  `competition_review_tests.rs` — matches round-3 brief:40 exactly and confirms the commit is
  test-only (brief:32).
- No source edited, no commit, no child agent dispatched, no reported test or checker run rerun.

## N1 closure — CLOSED

The new fixture discriminates the stale-snapshot failure mode. Verified by reading the test and
re-deriving both code paths, not by running anything.

**Fixture (post-fix).** `competition_review_tests.rs:284` adds `boost(&db, "bun", 2)`; `:287-292`
seeds `g1` with **three** members (`pnpm`, `npm`, `bun`, each `share: 1.0/3.0`). Two evidence
sweeps propose `g2 = {pnpm, yarn}` (`:296-298`), then the third sweep sends
`[rollback g1, propose g2]` in that array order (`:301-304`).

**Why it now fails without the fix.** With the stale pre-sweep snapshot (the `5d85c13` code,
`plan_confirmation(all_members, ...)`), the confirm of `g2` removes `pnpm` from the snapshot's
`g1` and the leftover is `[npm, bun]` — length **2**, so `route.rs:87` does *not* take the dissolve
branch; control goes to the shrink branch at `route.rs:90-111`, which pushes a planned write
`("g1", [npm, bun])`. That write is executed at `competition_review.rs:255`, so `g1` is
**recreated after the rollback deleted it**, and the assertion at
`competition_review_tests.rs:313` (`!members.iter().any(|m| m.group_id == "g1")`) fails. This is
exactly the mechanism the round-2 fixture missed: with the old 2-member seed the leftover was
`[npm]`, `< 2`, so `route.rs:87-89` dissolved it and the assertion passed either way.

**Why it passes with the fix.** Decisions are applied in order in the single loop at
`competition_review.rs:209` (order guaranteed by `propose.rs:277-291`, which pushes per proposal in
LLM array order): the rollback branch (`:291-307`) deletes `g1` via
`save_competition_group(group_id, &[])` — an empty member list is a delete
(`competition_groups.rs:80-83`) — and the subsequent confirm re-reads live state at `:227-233`, so
`g1` is simply absent from `live_members`; `plan_confirmation(&live_members, ...)` at `:244` plans
only `("g2", [pnpm, yarn])` and there is no group to shrink or recreate.

**Production path, not a parallel implementation.** The test calls the `sweep()` helper
(`competition_review_tests.rs:305`), which is a thin wrapper over the real host function:
`apply_competition_sweep(db, ws_key, llm_text, &members, &weights, now_ms)` at `:15-19`. Final
state is read back through `db.load_all_competition_members()` (`:309`) and the audit trail through
`actions_for` (`:321-322`), so the assertions cannot drift from host behaviour.

**Added strength beyond the minimum.** `:316-318` now also asserts positively that
`g2 == ["pnpm", "yarn"]`, which forecloses a vacuous pass in which *both* groups end up absent —
a genuine improvement over what I asked for.

**Fixture safety.** The `1.0/3.0` shares are accepted as-is: `save_competition_group` validates
only member/group-id consistency (`competition_groups.rs:71-78`) and has no share-sum constraint,
so the `.unwrap()` at `competition_review_tests.rs:292` cannot trip on floating-point drift.

One design note (not a defect): the discriminator is order-sensitive — it depends on the rollback
being applied before the confirm. The test pins that order in the LLM payload
(`:301-304`), and the order is deterministic through parse → decisions → apply, so the guard is
stable.

## Round-2 I1-I4 — all still CLOSED

No production or rules file changed after `67a6947` (`git diff --stat 67a6947..1e1f009` touches only
the test file), so every round-2 closure stands unchanged, with the same coordinates:

| # | Finding | Status | Evidence (unchanged file) |
| --- | --- | --- | --- |
| I1 | stale pre-sweep state | CLOSED | `competition_review.rs:227-233` live re-read (`warn!` + `continue` on error), `:244` plans against `&live_members`; snapshot now only feeds evidence at `:195` |
| I2 | destructive group-id collision | CLOSED | `:236-241` normalized-set mismatch → `warn!` + `continue` before the first save (`:255`) and before the confirm audit (`:278-288`) |
| I3 | module-tree boundary hole | CLOSED | `forbidden-rules.mjs:2368-2426` (11 patterns incl. `conn_locked`) + `:2428-2481` (10 patterns, none `conn_locked`, exemption documented at `:2478`); no `allowPaths`; both inside `forbiddenContentRules` (`:3-2482`) |
| I4 | incomplete trigger proof | CLOSED | report:99-127 = 11 production + 10 test-file failures with verbatim rule messages; no planted text or artifact survives (`git diff --name-only aa53f35..1e1f009` = the 10 expected files only) |

Also reaffirmed for the whole range:

- `memory_db.rs` = **999** lines, `memory_db_tests.rs` = **1098** lines, both absent from the range
  diff (brief:39 satisfied).
- Host test count is **10** (`rg -c "#\[test\]"` = 10), names unchanged from round 2.
- Every touched production `.rs` stays under 800: propose 536, route 201, competition_review 325,
  competition_groups 350, turn_persist_facts 386.
- `docs/status/surfaces.md` still correctly unchanged (0 matches for
  `agent_memory|judge_memory|dream|competition`).

## Report line counts (round-3 brief:41-42) — now correct

Measured with `(Get-Content).Count` and compared to the report's table at report:7-18. **All ten
entries plus the two frozen files match**; round-2 finding M3 is **CLOSED**:

| File | Report | Measured |
| --- | --- | --- |
| `review/propose.rs` | 536 (report:7) | 536 |
| `review/route.rs` | 201 (report:8) | 201 |
| `competition_review.rs` | 325 (report:9) | 325 |
| `competition_review_tests.rs` | 362 (report:10) | 362 |
| `memory_db/competition_groups.rs` | 350 (report:11) | 350 |
| `forbidden-rules.mjs` | 3292 (report:12) | 3292 |
| `agentic/AGENTS.md` | 56 (report:13) | 56 |
| `review/mod.rs` | 8 (report:14) | 8 |
| `agent_memory/mod.rs` | 29 (report:15) | 29 |
| `turn_persist_facts.rs` | 386 (report:16) | 386 |
| `memory_db.rs` (frozen) | 999 (report:18) | 999 |
| `memory_db_tests.rs` (frozen) | 1098 (report:18) | 1098 |

## New findings

### Critical

None.

### Important

None.

## Residual Minor items for final triage

### Report-text hygiene (no code change needed; not source defects)

- **M2 — baseline row still wrong.** report:134 lists `northhing-core competition_review: 10 tests`
  as the *baseline*, but `competition_review_tests.rs` is added within this range (362 insertions,
  0 deletions in the full-range diffstat), so the pre-T9 baseline is 0 and the delta is invisible.
- **M4 — fixer-section file:line citations remain stale.** report:26 cites
  `competition_review.rs:242-250` for the live-state load (actual `227-233`); report:31 cites
  `:251-257` for the collision check (actual `236-241`); report:27/28/32 cite tests
  `228-261` / `263-294` / `296-324` (actual `244-277` / `279-325` / `328-362`); the new N1 entry at
  report:38 cites `278-316` (actual `279-325`). The described mechanisms are all real and verified —
  only the coordinates are wrong.
- **M7 — clean checker run still paraphrased.** report:129 ("Reverted to strictly clean code.
  Passed validation successfully.") instead of the captured `Core boundary check passed.` line the
  fix brief asked for. The clean state itself is independently verified: no planted symbols remain
  in either file and no proof artifact exists in the range.
- **M8 — `cargo check` contradiction survives.** report:156-158 shows the command completing with
  19 warnings in 2m04s while report:165 still says it "timed out ... over 120s". Consequence
  unchanged: the "19 warnings = baseline" claim (report:160) is **Cannot verify from diff**.
- **Cannot verify from diff:** the `185` / `10` / `34` / `38` / `4` result lines (report:142-154),
  the 19-warning count, and all checker outputs — not rerun per instruction. The `10 passed` line
  is at least consistent with the 10 `#[test]` functions I counted in source.

### Source-level Minors (carried over, all still accurate)

- **M1 — trailing whitespace, kept Minor per brief:43.** `git diff --check aa53f35..1e1f009` now
  reports **66** diagnostics (round 2: 64): `propose.rs` 12 (production lines `187`, `308`),
  `route.rs` 10 (production `50`, `57`, `97`), `competition_review_tests.rs` **44** (+2 from the N1
  commit's new blank lines). All whitespace-only blank lines; `competition_review.rs` stays clean;
  no CI `cargo fmt --check` gate exists (`package.json:22` exposes `fmt:rs` =
  `scripts/format-changed-rust.mjs`), so it crosses no gate. Fold one `pnpm run fmt:rs` pass on the
  three files into branch finishing — never bare `cargo fmt`.
- **M5** I2's rejection policy is documented only by the inline `warn!`
  (`competition_review.rs:238`); the module doc (`:1-5`) and `apply_competition_sweep`'s doc
  (`:164`) still say nothing (fix-brief:56).
- **M6** `reject_live_group_id_collision` asserts surviving topics but not the seeded metadata
  (`competition_review_tests.rs:335-338` vs `:351-355`).
- **M9** A rejected confirmation discards three sweeps of evidence: the pending entry is removed at
  the threshold (`propose.rs:290`) before the host rejects (`competition_review.rs:239`), so the
  cycle repeats until the LLM picks a different `group_id`. Relatedly, the brief-supported
  "extend a group under the same id" path (`route.rs:180-200`) is now unreachable from the host by
  design — worth a note in the T10 merge/dedup brief.
- **Round-1 carry-overs, unchanged:** pending state overwritten after a failed KV read
  (`competition_review.rs:186-189` → `:311-315`); dead imports `AIClient`/`Arc` (`:7,19`, masked by
  `lib.rs:4`, dream precedent); global cadence gate vs per-workspace evidence (`:53,61`);
  `MAX_PENDING_PROPOSALS` enforced only on the push path (`propose.rs:310`); propose audit rows
  scatter when the LLM renames a set (`propose.rs:271` → `competition_review.rs:214`);
  `bad_json_zero_actions_gate_advanced` proves zero audit rows only via the return tuple
  (`competition_review_tests.rs:200-206`); `keyword_weights` SQL homed in
  `competition_groups.rs:121-135` (capacity-mandated); up to 15s added to turn finalize on
  gate-open turns (`turn_persist_facts.rs:237-238`, `competition_review.rs:26,103-104`); a propose
  equal to a group being rolled back in the same sweep is skipped as a live-set no-op
  (`propose.rs:256`).

## Fixer-round status

- **N1: CLOSED.** The discriminator is real, minimal, test-only, and slightly stronger than
  requested.
- **Round-1 I1-I4: all CLOSED and unaffected** by `1e1f009` (no production or rules file touched).
- **The full T9 fixer round is CLOSED.** No Critical or Important findings remain in
  `aa53f35..1e1f009`; no further fixer dispatch is warranted.
- Recommended before branch finishing, as non-blocking cleanup: a report-only correction pass for
  M2/M4/M7/M8 (baseline row, four stale citations, captured clean checker line, delete the stale
  `cargo check` deviation sentence) and one `pnpm run fmt:rs` whitespace pass for M1. All other
  Minors go to branch-level final triage as-is.
