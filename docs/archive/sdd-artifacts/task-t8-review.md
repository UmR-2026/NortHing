# T8 Review — competition groups, persistence, natural suppression

- Reviewed range: `8b64aa8..99d82dd` (single commit `99d82dd`, 9 files, +1004/−41 — diffstat matches the report)
- Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804` (read-only inspection; nothing edited, committed, or dispatched)
- Method: full-file reads of every changed file plus base-to-head diff; the implementer's test suite was **not** re-run (per review brief).

## Verdicts

- **SPEC: FAIL** — I1 (a same-turn boost of two members of one group is silently discarded, violating "boosting a member raises its share and squeezes every sibling") and I4 (the brief's mandatory documentation of the fact↔topic association *and its limits* is absent, while the code comment and the report assert a stronger guarantee than the storage provides).
- **QUALITY: FAIL** — I2 (`save_competition_group` deletes by the `group_id` parameter but inserts `m.group_id`; combined with `rehydrate_group`'s empty-string fallback this is a silent data-placement hazard that lands squarely on T9), I3 (nondeterministic share resolution for a topic in two groups), I5 (`memory_db.rs` crossed the 1000-line god-file gate with no split and no `// allow-god-file`).

A fixer round is required. No Critical findings — no data loss of user facts, no `supersede`, no status mutation, no self-cognition access, no blocking of `turn_persist`.

---

## Findings

### Critical

None.

### Important

#### I1 — Same-turn boost of two members of one group silently loses the first boost (stale snapshot)

`src/crates/assembly/core/src/agentic/growth_adapter.rs:396-427`

`all_members` is loaded **once** before the topic loop (line 397) and is never refreshed after a save (line 424). `group_containing(&all_members, topic)` therefore reads the pre-turn snapshot on every iteration.

Trace with a real, common input — `extract_topics` returns both ASCII tokens for "pnpm 还是 npm？":

- group `g1` = {pnpm 0.70, npm 0.30}
- iteration 1, `topic = "pnpm"`: boost from the snapshot → {pnpm 0.74, npm 0.26} → saved.
- iteration 2, `topic = "npm"`: `group_containing` returns the **same stale** {0.70, 0.30} → boost → {pnpm 0.61, npm 0.39} → saved, overwriting iteration 1.

Net result: pnpm's boost is gone from durable state. The sum invariant still holds, so no test catches it — `boost_turn_topics_repeated_boosts_preserve_members_and_sum` (`growth_adapter/tests.rs:718-745`) only ever mentions one member per turn. `extract_topics` dedups topics (`src/agentic/src/topics/extract.rs:298-312`), so this is *not* a same-topic double-boost; it is a genuine lost update across two distinct members.

The fix is already in the codebase: `CompetitionGroupStore::load_group_members` (`ports.rs:200`, implemented at `growth_adapter.rs:193`) exists and is **never called** in production. Reload the group inside the loop, or write the saved `durable` back into `all_members` before the next iteration. Add a regression test that mentions two members of one group in a single turn and asserts both rose relative to a non-mentioned third member.

#### I2 — `save_competition_group` deletes by the parameter but inserts `m.group_id`

`src/crates/assembly/core/src/service/agent_memory/memory_db/competition_groups.rs:62-94`

`DELETE FROM competition_groups WHERE group_id = ?1` uses the `group_id` argument (line 63-64), while the `INSERT` binds `m.group_id` from each member (line 79). Nothing validates that they agree. If they diverge, the call empties the target group *and* writes the rows into a different group id — a silent full-membership move, still reported as `Ok(())`.

This is reachable from the T9 flow the port doc advertises. `rehydrate_group` (`src/agentic/src/topics/competition.rs:117-125`) falls back to `members.first().map(...).unwrap_or_default()` — the empty string — for any boosted member with no previous row. T9's advertised "propose a new group" cycle is load (empty) → apply → `rehydrate_group` → `save_group_members("pm-preference", …)`, which will persist every row with `group_id = ""` while deleting `pm-preference`. The T8 adapter path happens to be safe only because `group_containing` guarantees a non-empty, single-group member list and `apply_boost` never inserts (the topic is known to be present) — an accident of the caller, not a property of the API.

Fix: bind the `group_id` parameter in the `INSERT` (or return an error on mismatch), and give `rehydrate_group` an explicit `group_id: &str` argument instead of the empty-string fallback. Cover both with tests.

#### I3 — Suppression share lookup collapses `(group_id, member_topic)` to `member_topic`, nondeterministically

`src/crates/assembly/core/src/service/agent_memory/memory_db/competition_groups.rs:187-208`

`load_competition_share_map` selects `member_topic, share` with **no `ORDER BY`** and inserts into a `HashMap<String, f64>` keyed on the topic alone. The primary key is `(group_id, member_topic)` (`memory_db.rs:168-177`), so the same topic in two groups is schema-legal, nothing validates single-group membership, and the surviving share is whichever row SQLite happens to return last. The suppression decision for that topic — and therefore whether a fact appears in the prompt — can differ between two identical searches.

This also contradicts the deterministic rule the adapter chose for the write side (`growth_adapter.rs:451-466`, first group by `group_id` order) and the report's "deterministic `group_containing` helper" claim, which only covers half the system. T9 generates groups from LLM proposals, which makes overlapping membership realistically reachable.

Fix: resolve deterministically and conservatively — e.g. aggregate with `f64::max` (least suppression wins) or add `ORDER BY group_id` and document last-wins. Pin it with a test that puts one topic in two groups with different shares.

#### I4 — Required documentation of the fact↔topic association and its limits is missing; the stated guarantee is stronger than the code delivers

`src/crates/assembly/core/src/service/agent_memory/memory_db.rs:605-667`; report §Key Decisions

The T8 brief is explicit: *"use the existing keyword/FTS relation and document the exact association and its limits. Do not silently claim stronger semantics than the storage proves."* The report contains no such section, and both the code comment (line 613-616) and the report assert "ungrouped and unrelated facts are unaffected". That claim is false as written.

The association is token overlap, not a fact↔topic relation: line 637-638 keeps a keyword if **any single token** of the segmented keyword is in the fact's token set. `segment_for_fts` (`memory_db.rs:1018-1047`) emits CJK **bigrams**, so a suppressed topic `依赖安装` produces `依赖 赖安 安装` and hides any fact containing `安装` alone. On the ASCII side, `extract_topics` genuinely produces generic single words — its own test asserts `["prefer", "pnpm", "dependency"]` (`src/agentic/src/topics/extract.rs:377-382`) — so a suppressed generic member such as `dependency` would hide every fact containing that word, regardless of the group's subject.

Blast radius is zero **today** (T8 ships no group writer, so `competition_groups` is empty in every real DB), which is why I rate this Important rather than Critical. T9 turns the writer on, so the limit must be documented now and, preferably, the gate narrowed (for example require the suppressed keyword to also be a query token, or require a multi-token/whole-keyword match before a fact may be dropped entirely rather than merely down-weighted).

Also note the strength asymmetry the report does not state: a non-suppressed match only *lowers the weight*, but a solely-suppressed match *removes the fact from the result set* (line 665-667) — the most aggressive of the available options, chosen without the brief mandating removal over de-prioritization ("affect retrieval visibility/priority").

#### I5 — `memory_db.rs` crossed the >1000-line god-file gate with no split and no `// allow-god-file`

`src/crates/assembly/core/src/service/agent_memory/memory_db.rs` — 980 lines at `8b64aa8`, **1054** at `99d82dd`; no `allow-god-file` marker present.

Root `AGENTS.md:92`: *"production `.rs` files over 800 lines raise review pressure; over 1000 lines must be split or carry a `// allow-god-file` justification comment at the top of the file."* The change crossed the hard line, not the soft one. The implementer correctly created `memory_db/competition_groups.rs` for the SQL but left the ~62-line suppression block (lines 605-667) inline in `search_facts`; extracting it as e.g. `competition_groups::effective_keyword_weight(&keyword_map, &group_shares, &fact_tokens) -> Option<f64>` (`None` = suppress) puts the file back under 1000 and makes the gate unit-testable in isolation.

Related, note only (not a violation): `src/agentic/src/topics/competition.rs` grew 638 → 834 lines (over the 800 "review pressure" line, test-inclusive), and `memory_db_tests.rs` is now 1003 lines (a test file; the rule targets production `.rs`).

### Minor

- **M1** `growth_adapter.rs:451-466` — `group_containing` rebuilds a `BTreeMap` and **clones every member** on each loop iteration: O(topics × members) clones per turn. Group once before the loop. (Fixing I1 will likely restructure this anyway.)
- **M2** `memory_db.rs:629-635` — `candidate_keywords` is rebuilt and reallocated for **every candidate fact row** (up to `max(limit*3, 30)` rows) although it depends only on `keyword_map`/`group_shares`. Hoist it above the row loop.
- **M3** `competition_groups.rs:194, 200, 204` — `load_competition_share_map` swallows every error **silently**; there is no `tracing::warn!` anywhere in it. The global constraint is 失败只 `tracing::warn!`, and the report/module doc call it "warn-only", which is inaccurate. (It matches the pre-existing `load_keyword_weights` style, so this is consistency-vs-constraint, not a regression.)
- **M4** `memory_db.rs:654-667` — the `keyword_weight = 0.0` branch (line 656-657) is dead: the only way to reach it is `any_suppressed_match && !any_positive_match`, which `continue`s three lines later. Collapse to a single early `continue`.
- **M5** `growth_adapter.rs:182-190` — `pub struct TopicDbStore` lives in `pub mod growth_adapter` (`agentic/mod.rs:51`) but its only constructor is `pub(crate) fn new`, so it is exported and unconstructable outside the crate. Make the struct `pub(crate)`.
- **M6** `memory_db.rs:168-177` — `competition_groups` has no `workspace_key`, so a group learned in one workspace suppresses `scope='global'` facts and facts of every other workspace. This is consistent with the already-global `keyword_weights` and is defensible, but it is an undocumented scope widening of suppression; it belongs in the report's limits section (see I4).
- **M7** `auto_memory.rs:332-347` — suppression also skips `touch_fact`, so a suppressed fact's `last_mentioned_at` stops refreshing and its recency term keeps decaying. Revival is weight-driven and independent, so this is not a one-way trap, but the compounding effect is unstated in the report.
- **M8** `competition_groups.rs:160-166` + `growth_adapter.rs:397-403` — one unreadable row (e.g. a `NULL` share from an externally-edited DB) makes `load_all_competition_members` fail wholesale, disabling group normalization for **all** groups for that turn. A per-row skip (as the share-map reader already does at line 204) would degrade more gracefully.
- **M9** Legacy-DB score nuance, `memory_db.rs:654-655` — the old code was `fold(1.0, f64::max)`, so `keyword_weight` was floored at 1.0; the new code returns `best_weight` directly. With today's writers this is equivalent (weights are `{0.0} ∪ [1.0, 5.0]`: `boost_keyword` inserts 1.0 / `+1.0` capped at 5.0, `decay_all_weights` floors at `TOPIC_DECAY_FLOOR = 1.0`, `set_keyword_ignored` writes exactly 0.0, and 0.0 is excluded by the `w > 0.0` guard). A DB carrying weights in `(0.0, 1.0)` from an older floor would now score lower than before. Low risk; worth a one-line note rather than a change.
- **M10** Report accuracy — "New tests (10)" undercounts: 13 new `#[test]` functions (competition.rs 21 → 24, `growth_adapter/tests.rs` +3, `memory_db_tests.rs` +7) plus one added assertion in `open_creates_tables`. The Files-Changed row's "24 tests" for competition.rs is correct. The report also substitutes `cargo test -p northhing-core --lib --features product-full` for the brief's two filtered commands; that is a strict superset and fine, but the substitution is not flagged as a deviation.
- **M11** Required test 8's host-adapter leg ("storage failures remain warn-only where the host adapter is involved") has no test. Verified by inspection instead: `growth_adapter.rs:397-403` and `424-426` both `tracing::warn!` and continue; nothing propagates into `turn_persist`.

---

## Spec coverage matrix — the eight required test behaviors

| # | Required behavior | Status | Evidence |
|---|---|---|---|
| 1 | Group schema present on fresh DB; old DB reopens non-destructively | **Covered** | `memory_db_tests.rs:26` (`open_creates_tables` asserts `competition_groups`); `old_db_without_competition_table_reopens_and_preserves_data` builds a pre-T8 schema, reopens, asserts the legacy fact + keyword survive and the new table is immediately usable. Schema is in the same `CREATE TABLE IF NOT EXISTS` batch (`memory_db.rs:166-177`), so it runs on every open. |
| 2 | Shares sum to 1; boost rise ⇒ sibling fall; zero-division safe; malformed safe | **Covered** | `competition.rs` `boost_rise_causes_fall`, `sum_conservation_over_many_boosts`, `all_zero_weights_split_equally`, `nan_and_negative_treated_as_zero`; live path in `growth_adapter/tests.rs` `boost_turn_topics_raises_group_member_and_squeezes_sibling`. |
| 3 | `<=1.0` + share `<0.15` suppresses; boundaries pinned; `>1.0` stays active | **Covered** | Pure: `suppression_boundary_strict_less_than` pins `(0.15, 1.0) → Active`, `(0.10, 1.0) → Suppressed`, `(0.1499…, 1.0000000001) → Active`. Production: `search_facts_suppresses_cold_group_member_fact`, `search_facts_keeps_high_share_member_at_cold_weight`, `search_facts_revives_low_share_member_when_weight_rises`. Gate implemented at `memory_db.rs:640-645` calling the pure `suppression_state`. User ruling honoured everywhere; `SUPPRESSION_RAW_THRESHOLD = 0.20` fully removed (it had no callers outside `competition.rs`), no second activity score, no `0.1` floor. |
| 4 | Suppressed topics remain stored; a later boost revives | **Covered (production path, not just helpers)** | `growth_adapter/tests.rs` `boost_turn_topics_suppressed_topic_revives_after_repeated_mention` drives the real `boost_turn_topics` + `search_facts`, asserts hidden → hidden → visible, and asserts the group rows are untouched (`loaded.len() == 2`). |
| 5 | Ungrouped topic follows the existing boost/decay path unchanged | **Covered by regression only** | No new dedicated test; the pre-existing `boost_turn_topics_*` suite runs against fresh DBs with zero groups and still passes. `boost_turn_topics` reduces to the pre-T8 body when `all_members` is empty (`growth_adapter.rs:421` is the only new branch) and facts matching no group member keep the 1.0 baseline (`memory_db.rs:658-659`). Acceptable, but a one-line explicit test would be cheap. |
| 6 | No fact row deleted or hard-retired by group maintenance | **Covered** | `group_maintenance_never_deletes_or_supersedes_facts`: the fact is invisible to `search_facts` yet returned by `get_facts`, which filters `status = 'active'` (`memory_db.rs:306`) — so this also proves the status was not superseded. `supersede_fact` is untouched; no new call site (only the pre-existing `dream.rs` one). |
| 7 | Round-trip preserves membership, shares, source/evidence, timestamps | **Covered** | `save_load_competition_group_round_trip` asserts topic/share/`evidence_count`/`source`/`created_at_ms`/`updated_at_ms`, full replacement, `member_topic ASC` ordering, and drop-on-empty; `rehydrate_preserves_metadata_across_boost` covers metadata survival through a boost. |
| 8 | Malformed/empty groups do not panic; adapter storage failures stay warn-only | **Partial** | Malformed/empty is covered on the pure side (`durable_empty_and_missing_are_safe`, `empty_group_handling`, `nan_and_negative_treated_as_zero`, `renormalize`'s explicit `sum == 0.0` branch). The **host-adapter warn-only leg is untested** — see M11; correct by inspection but unproven. |

## Cross-task interface assessment (as requested)

- **T9 readiness — conditionally yes, blocked on I2.** `CompetitionGroupStore` is minimal, object-safe (`ports.rs:294-296` extends the compile-time check), and carries every field T9 needs (`evidence_count`, `source`, `created_at_ms`, `updated_at_ms`), all round-tripped losslessly. Full-replacement `save_group_members` plus `load_group_members` is the right shape for propose/accumulate/rollback. **But** the advertised "create a new group" cycle hits the `group_id = ""` + delete/insert-key-mismatch trap (I2), and T9's LLM-authored groups make the multi-group nondeterminism (I3) and the over-broad token association (I4) live rather than theoretical. Fix I2/I3 in this task; I4 at minimum needs written limits now.
- **Port boundary — clean.** The growth crate stays IO-free: `competition.rs` gained only serde derives and pure helpers; `ports.rs` gained only a trait. All SQLite lives in core, and `TopicDbStore` is the only concrete implementation. Core's `search_facts` importing the growth *pure decision function* follows existing precedent in the same directory (`dream.rs`, `distiller.rs`, and `memory_db.rs` itself already calls `topics::score::retrieval_score`), so it is not a new boundary break; the boundary script passes per the report.
- **No T9/T10/T11/T12/T4c overreach.** No LLM proposal source, no hardcoded group seeding, no merge/dedup, no negation, no garden rewrite, no facade. The diff touches exactly the 9 declared files. Removing `group_members`/`set_group` from `TopicStore` is the "narrowly equivalent port adjustment" the brief permits and breaks nothing — `TopicStore` still has no implementor (a pre-existing dead contract awaiting T10, unchanged by this task).
- **T12 gardener reuse — safe.** Suppression lives only in `search_facts`, whose sole production caller is the query-aware injection at `auto_memory.rs:332`. The dream/garden sweep reads facts through its own path, so the gardener still sees suppressed facts and cannot be starved into mistaking them for absent. One forward-looking caution for the T12 brief: a "never surfaces ⇒ retire" heuristic would interact badly with natural suppression, since suppressed facts are invisible to retrieval *and* stop being `touch_fact`-ed (M7).
- **Warn-only / security — clean.** Every group load/save at the host edge is `match`/`if let Err` → `tracing::warn!` → continue (`growth_adapter.rs:399-402, 424-426`); the keyword boost and the per-turn decay still run after a group failure. No `supersede`, status mutation, deletion, LLM call, or self-cognition access was introduced. English-only source and logs; no emoji added (the only non-ASCII in added lines are `§` in test comments and the CJK test fixture strings). `cargo fmt` was not run.

## Cannot verify from diff

- Test-run counts and timings in the report (`168 passed`, `1225 passed / 1 ignored`, `19 pre-existing warnings`, `Core boundary check passed.`) — deliberately not re-executed per the review brief. Nothing in the diff contradicts them, and the file-level test counts I could check statically are consistent apart from the "New tests (10)" undercount in M10.
- The claim that `cargo check -p northhing` fails only on a pre-existing `keyring v4.1.6` feature issue reproduced at `8b64aa8`. The diff contains no desktop-facing API change, which is consistent with the claim, but the baseline reproduction itself is unverified here.
- Real-world blast radius of I4 depends on what group ids/member topics T9 will actually mint; that data does not exist yet.

## Residual risk and fixer decision

**A fixer round is required.** Recommended single fixer package, in order:

1. **I1** — refresh the group state between iterations in `boost_turn_topics` (use the existing unused `load_group_members`, or write `durable` back into `all_members`) + a two-members-one-turn regression test.
2. **I2** — bind the `group_id` parameter in the `INSERT` (or reject a mismatch) and add an explicit `group_id` argument to `rehydrate_group`; test both the mismatch guard and the fresh-group path.
3. **I3** — make the share map deterministic and conservative for multi-group topics; pin with a test.
4. **I4** — add the association-and-limits section to the report, correct the "unrelated facts are unaffected" comment at `memory_db.rs:613-616`, and state the workspace-scope widening (M6). Narrowing the gate is desirable but is a design call — flag it to the orchestrator rather than deciding it in a fixer.
5. **I5** — extract the suppression block out of `search_facts` into `competition_groups.rs` to get `memory_db.rs` back under 1000 lines (this also gives I4's gate a natural unit-test home).

Minor items M1–M11 do not gate this task; M2 and M4 are near-free to fold into the I5 extraction, and M10/M11 are cheap report/test corrections. The rest should be triaged at branch final review.

Residual risk after those fixes is low: the suppression path is confined to one call site, it is data-preserving by construction, and it is inert until T9 begins writing groups — which is exactly why the I2/I3/I4 corrections are worth making before T9 rather than after.
