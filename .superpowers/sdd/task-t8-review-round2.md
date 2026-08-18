# T8 Review Round 2 — competition groups, persistence, natural suppression

**SPEC PASS** — **QUALITY PASS**

- Reviewed range: `8b64aa8..aa53f35` (3 commits: `99d82dd` feat, `5481dbd` fixer I1–I5, `aa53f35` I4 wording; 9 files, +1270/−41 — diffstat matches the report).
- Worktree `E:\agent-project\northing\.worktrees\growth-core-0804`, read-only: nothing edited, committed, or dispatched; `git status --porcelain` clean.
- Method: full BASE→HEAD diff plus full-file reads of `competition_groups.rs`, `memory_db.rs` (suppression path + schema), `growth_adapter.rs`, `competition.rs`/`ports.rs` diffs, and both test files; behavior compared against **both** `8b64aa8` and `99d82dd`. Implementer tests were **not** re-run (per brief).

All five round-1 Important findings are independently closed. **No Critical, no Important remain — the fixer round is closed.** Residual Minor items are listed for final triage.

---

## Round-1 findings — independent closure evidence

### 1. Same-turn two-member boost (I1) — CLOSED

`growth_adapter.rs:414-424` builds the group index **once** (`BTreeMap<group_id, Vec<CompetitionMember>>` plus a `topic -> group_id` map, first group by `group_id` order wins), and `growth_adapter.rs:443-453` mutates that working map: `*members = durable` runs **only inside the `else` branch of the save**, i.e. only after a successful `save_group_members`. So

- iteration 2 of the same turn boosts from the durable result of iteration 1, not from the pre-turn snapshot;
- on a save failure the working map keeps the last **persisted** state, which is exactly what the DB holds — no divergence between memory and storage;
- the failure leg is still `tracing::warn!` + continue, and `boost_keyword` (line 457) still runs, so the turn's keyword signal is never lost.

M1 (per-iteration `BTreeMap` rebuild + member clones) is resolved by the same restructure.

Regression test `boost_turn_topics_two_group_members_one_turn_both_preserved` (`growth_adapter/tests.rs:797-858`) mentions both `pnpm` and `npm` in one turn via `llm_keywords`. Both survive `normalize_topic_candidates` (3-char minimum; `extract.rs:177-205`, `extract.rs:346-362`), so the test is not vacuous. I recomputed the two paths by hand from `{pnpm .5, npm .3, yarn .2}` with `MAX_BOOST_PER_EVENT = 0.15`:

| | pnpm | npm | yarn |
|---|---|---|---|
| cumulative (correct) | 0.4915 | 0.3573 | 0.1512 |
| stale snapshot (bug) | 0.4348 | 0.3913 | 0.1739 |

The per-topic assertion (`|m.share - exp.share| < SHARE_SUM_EPSILON`) fails on `pnpm` under the buggy path, so the test genuinely pins the regression. It also asserts sum 1.0, 3 surviving members, and both mentioned members above the unmentioned `yarn`.

### 2. Explicit group id in save / `rehydrate_group` (I2) — CLOSED

- `competition_groups.rs:71-78` rejects any member whose `group_id != group_id` argument **before** the `DELETE`, and `competition_groups.rs:91-105` binds the **`group_id` parameter** (not `m.group_id`) in the `INSERT`. Delete id ≡ insert id is now a property of the API, not of the caller.
- `competition.rs::rehydrate_group(group_id, members, boosted, updated_at_ms)` takes the id explicitly and stamps it on **every** returned member, including members with no previous row; the `unwrap_or_default()` empty-string fallback is gone (verified by full-text search: no remaining `unwrap_or_default` on a group id).
- Tests: `save_competition_group_rejects_group_id_mismatch` (`memory_db_tests.rs:1004-1029`) asserts `Err`, unchanged previous membership, **and** no stray rows under the member's own id; `durable_empty_and_missing_are_safe` (`competition.rs:790-812`) covers the fresh-group insert path (`group_id == "g1"`, fresh metadata, `created_at_ms == updated_at_ms`); `rehydrate_overrides_previous_group_id` (`competition.rs:815-838`) pins identity override with metadata preserved.
- T9 compatibility: the advertised load → modify → `rehydrate_group(gid, …)` → `save_group_members(gid, …)` cycle now cannot delete one id and write another; a hand-built mismatched member list fails loudly (`Err`, warn-only at the host edge) instead of silently relocating rows.

### 3. Duplicate topic across groups (I3) — CLOSED

`load_competition_share_map` (`competition_groups.rs:211-235`) selects with `ORDER BY group_id ASC, member_topic ASC` and folds duplicates with `entry(...).and_modify(|e| *e = e.max(share)).or_insert(share)` — **largest share wins**, i.e. least suppression, and the result is independent of SQLite row order (the `max` fold makes the ordering clause belt-and-braces rather than load-bearing; both are present). `f64::max` also absorbs a `NaN` on either side toward the non-NaN operand, and `suppression_state` treats NaN as Active anyway.

Test `share_map_resolves_duplicate_topic_least_suppression_wins` (`memory_db_tests.rs:1031-1054`) puts `pnpm` in `g1` at 0.05 and `g2` at 0.80, asserts the map resolves to 0.80, **and** asserts end-to-end through `search_facts` that the fact stays visible at cold weight.

### 4. Association claim and limits (I4) — CLOSED

- The false claim is gone: repo-wide search for `unaffected|unrelated` across `service/agent_memory/`, `competition.rs`, and `growth_adapter.rs` returns exactly one hit — the string literal `"completely unrelated"` in a test fixture (`memory_db_tests.rs:1084`).
- The limits are documented at the decision site (`competition_groups.rs:276-298`): segmented-keyword token overlap (not a fact↔topic relation), CJK bigram example (`依赖安装` → `依赖 赖安 安装` overlapping a bare `安装`), generic ASCII example (`dependency`), "a fact can be hidden entirely when its only matching keyword is suppressed", global scope because the table has no `workspace_key`, and "suppressed facts are not touched by retrieval". `memory_db.rs:605-607` points at that module rather than restating a weaker claim.
- Report §"Association Limits & Scope (I4)" (lines 60-84) states the same five points plus the M7 recency side effect, and explicitly records that the behavior was **not** narrowed.
- Verified independently: global scope is real (`memory_db.rs:167-177` — PK `(group_id, member_topic)`, no `workspace_key`); non-mutating is real (`search_facts` only `continue`s; `touch_fact` is called by the caller `auto_memory.rs:344-346` over returned rows only, so a suppressed fact is never touched, never status-mutated, never deleted).

### 5. God-file gate and helper equivalence (I5) — CLOSED

- `memory_db.rs` is **999 lines** (`Get-Content | .Count`), under the hard 1000 gate; repo-wide search finds **no** `allow-god-file` marker anywhere in the changed files.
- Equivalence: I diffed the extracted `effective_keyword_weight` (`competition_groups.rs:299-333`) against the inline block at `99d82dd` line-by-line. Same candidate union (keyword rows + group members defaulted to `COLD_BASELINE_WEIGHT`), same `chars().count() >= 2` guard, same `segment_for_fts` any-token overlap, same `suppression_state` gate, same `best_weight`/`any_positive_match`/`any_suppressed_match` bookkeeping, same `None ⇔ any_suppressed && !any_positive`. M4's dead `keyword_weight = 0.0` branch was collapsed correctly (that value could never survive the `continue`). M2 is folded: `suppression_candidates` is hoisted once per search (`memory_db.rs:502-503`) instead of rebuilt per candidate row.
- Versus BASE (`8b64aa8`, `fold(1.0, f64::max)`): the only divergence is a weight in the open interval `(0.0, 1.0)`, which no current writer produces (M9). Stronger than the report claims — even if such a row existed, `tw_norm = (w/5.0).max(1.0/5.0)` (`memory_db.rs:620`) floors it to the same `0.2`, and `ScoredFact.keyword_weight` has no production consumer beyond `tw_norm` (grep: only `memory_db.rs:15/620/627`). **M9 is score-neutral and can be closed.**
- Helper is directly unit-tested: `effective_keyword_weight_decides_suppression` (`memory_db_tests.rs:1056-1097`) covers suppress / warm-revive / no-overlap-baseline / group-member-without-a-weight-row.

---

## Additional regression checks

| Check | Result | Evidence |
|---|---|---|
| User ruling exact | **Pass** | `suppression_state`: `share < SUPPRESSION_SHARE_THRESHOLD (0.15)` **and** `raw_weight <= COLD_BASELINE_WEIGHT (1.0)` (`competition.rs:246-254`). `SUPPRESSION_RAW_THRESHOLD` fully removed (repo-wide search: 0 hits). No second heat/activity score; `TOPIC_DECAY_FLOOR` still `1.0` (`growth_adapter.rs:65`) — no `0.1` floor anywhere. Boundaries pinned in `suppression_boundary_strict_less_than`: `(0.15, 1.0) → Active`, `(0.10, 1.0) → Suppressed`, `(0.1499…, 1.0000000001) → Active`. |
| Shares sum to 1, boost cap, invalid handling | **Pass** | `apply_boost`/`renormalize` bodies are **unchanged** in the range (only doc comments moved); `MAX_BOOST_PER_EVENT = 0.15` unchanged; NaN/negative/zero-sum tests retained. Live sum invariant asserted in three adapter tests. |
| Ungrouped topics on the old path | **Pass** | `growth_adapter.rs:443` is the only new branch and is gated on `topic_index.get(topic)`; empty group table ⇒ pre-T8 body exactly. `effective_keyword_weight` returns `Some(1.0)` for facts matching no keyword. (No *dedicated* test — see Minor R6.) |
| No delete / status mutation / supersede / self-cognition | **Pass** | Diff adds no `supersede` call (repo search: only the pre-existing `dream.rs` site and a "no retire/supersede exists" comment in `competition.rs:510`); `search_facts` only `continue`s; `group_maintenance_never_deletes_or_supersedes_facts` proves the row survives via `get_facts`, which filters `status='active'`. |
| Helper preserves empty-query / CJK / workspace / multi-keyword / BM25 / two-layer | **Pass** | The `search_facts` diff touches exactly two places: the hoist at 496-503 and the weight computation at 601-612. Empty-query early return (448-455), FTS `MATCH` construction, workspace/global predicate, `candidate_limit`, `bm25`, `recency_boost`, `tw_norm`, `retrieval_score`, sort and truncate are byte-identical to BASE. |
| Transaction / migration / ordering / metadata | **Pass** | Schema added to the same `CREATE TABLE IF NOT EXISTS` batch (`memory_db.rs:167-177`); `old_db_without_competition_table_reopens_and_preserves_data` builds a genuine pre-T8 schema and asserts legacy fact + weight survive and the new table is usable. Save is one transaction (validate → delete → insert → commit); any error path drops the guard and rolls back. Read order `member_topic ASC` / `group_id ASC, member_topic ASC`; metadata round-trip asserted incl. `evidence_count`, `source`, `created_at_ms`, `updated_at_ms`. |
| Line counts / boundary rules | **Pass** | Verified on disk: `memory_db.rs` 999, `competition_groups.rs` 333, `growth_adapter.rs` 473, `ports.rs` 298, `competition.rs` 870, `mod.rs` 27, `growth_adapter/tests.rs` 858, `memory_db_tests.rs` 1098, `AGENTS.md` 50 — **every number in the report's Files-Changed table is correct**. Only `competition.rs` (870, test-inclusive) is over the 800 soft line. Core→growth-crate import follows the pre-existing `topics::score::retrieval_score` precedent in the same function; no new `service → agentic` cross-layer reference. No emoji in added lines (scanned); English-only source and logs; `docs/status/surfaces.md` not applicable (no crate added). |
| T9/T10/T12 not early, port sufficient | **Pass** | Diff touches exactly the 9 declared files; no LLM proposal source, no hardcoded seeding, no merge/dedup, no negation, no garden rewrite, no facade. `CompetitionGroupStore` (`ports.rs:196-205`) carries `load_group_members` / `save_group_members` / `load_all_group_members` over `CompetitionMember` with `evidence_count` / `source` / `created_at_ms` / `updated_at_ms`, is object-safe (checked at `ports.rs:295`), and round-trips losslessly. Removing `group_members`/`set_group` from `TopicStore` breaks nothing: repo-wide search finds no remaining reference outside a doc comment. |

---

## Residual Minor items (final-triage list)

- **R1 (was M3)** `competition_groups.rs:211-235` — `load_competition_share_map` still swallows every error silently (`Err(_) => return map` at 219/225, `if let Ok` at 228); no `tracing::warn!`. Two comments now overstate this: `memory_db.rs:499-500` says "warn-only, empty on failure" and the doc at `competition_groups.rs:203-204` says "growth is warn-only". Behavior is *silent*-on-failure. Either add the warn or reword. (Matches the pre-existing `load_keyword_weights` style, so it is consistency-vs-constraint, not a regression.)
- **R2 (was M8)** `competition_groups.rs:178-184` — one unreadable row still fails `load_all_competition_members` wholesale, disabling group normalization for all groups that turn. Per-row skip would degrade more gracefully.
- **R3 (was M11)** No hermetic test for the host-adapter warn-only leg; verified by inspection at `growth_adapter.rs:400-406` and `447-449`. Reported as a limitation — acceptable.
- **R4 (was M10, recurrence)** Report test-count line 56-58 is still inaccurate. Measured over `8b64aa8..aa53f35`: `competition.rs` 21→25 (+4), `growth_adapter/tests.rs` 30→34 (+4), `memory_db_tests.rs` 28→38 (+10) = **18 new `#[test]` functions**, not 13; and "growth crate 168 → 169" describes only the fixer delta, not the task delta. Head-side per-file totals (25 / 34 / 38) are correct.
- **R5 (new)** Write/read asymmetry for a topic in two groups: the boost side (`growth_adapter.rs:419-424`) only ever updates the **first** group by `group_id` order, while the read side takes the **max** share across groups. Both are deterministic and the read side is conservative, but a topic's second group can hold a permanently stale share. Worth one line in the T9 brief.
- **R6 (was round-1 matrix item 5)** Still no dedicated "ungrouped topic unchanged" test; covered by the pre-existing zero-group `boost_turn_topics_*` suite only. Cheap to add later.
- **R7 (new)** `competition_groups.rs:63-78` — the transaction is opened *before* the group-id validation loop. Harmless (an empty tx is rolled back on the error path), but validating before `conn.transaction()` reads better and avoids taking a write transaction for a rejected call.
- **R8 (new)** `memory_db.rs` sits at 999/1000 lines — one line of headroom before the hard gate re-breaches on the next core edit. Schedule a further split (e.g. the search/scoring path) at branch triage rather than letting the next task discover it.
- **R9 (soft, note only)** `competition.rs` is 870 lines (test-inclusive), over the 800 review-pressure line. Not a violation.
- **R10 (nit)** The I1 regression test compares against a re-simulation built from the same production helpers (mirror-implementation). It does catch the stale-snapshot bug (numbers above), so it is adequate; hard-coded expected shares would be strictly stronger.
- **Closed from round 1:** M1, M2, M4, M5 (`TopicDbStore` is now `pub(crate)`, `growth_adapter.rs:185`), M6/M7 (documented in code + report), **M9 (now provably score-neutral via the `tw_norm` floor — recommend closing outright rather than triaging).**

## Cannot verify from diff

- Test execution results quoted in the report (`169 passed` growth, `1229 passed / 1 ignored` core, `19 pre-existing warnings`, `Core boundary check passed.`) — deliberately not re-run per the brief. Nothing in the diff contradicts them; the statically checkable per-file test counts and every line count in the report's table match exactly (the only static discrepancy is the aggregate "13 new tests" figure, R4).
- That the 19 `cargo check` warnings are all pre-existing, and that `cargo check -p northhing` fails only on the pre-existing `keyring v4.1.6` feature issue reproduced at `8b64aa8`. The diff contains no desktop-facing API change, which is consistent with the claim.
- Real-world blast radius of the token-overlap association still depends on the group ids/member topics T9 will mint; that data does not exist yet (`competition_groups` is empty in every real DB until T9 ships a writer).
