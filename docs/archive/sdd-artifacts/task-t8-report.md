# T8 Report — Competition Groups: Persistence, Normalization, Natural Suppression

- **Branch**: `feat/growth-core-0804`
- **Baseline**: `8b64aa8` → **Final head**: `aa53f35` (3 commits)
- **Commits**: `99d82dd` feat, `5481dbd` fixer (I1–I5), `aa53f35` I4 wording
- **Date**: 2026-08-07
- **Diff**: `E:\agent-project\northing\.superpowers\sdd\task-t8-diff.patch` (range `8b64aa8..aa53f35`)

## Summary

T8 makes competition groups durable and live: group membership is persisted in a
new SQLite table, topic boosts normalize group shares (mentioned member rises,
siblings squeezed, sum stays 1.0), and a two-gate rule (share strictly `<0.15`
**AND** live weight `<=1.0`, per the user ruling 2026-08-07) naturally suppresses
low-share cold topics from retrieval without deleting or hard-retiring any data.
Data is always preserved and a repeated mention (live weight strictly above the
cold baseline `1.0`) is the revival signal.

A fixer round resolved all five Important review findings:
- **I1** — `boost_turn_topics` now indexes groups once and writes the durable
  result back into the working map after each save, so two members of one group
  mentioned in the same turn build cumulatively (the second boost no longer
  overwrites the first from a stale pre-turn snapshot).
- **I2** — `save_competition_group` validates every member against the explicit
  `group_id` argument (rejecting mismatches before any write) and binds that id
  in the INSERT; `rehydrate_group` takes an explicit `group_id` instead of the
  silent empty-string fallback. T9's create-group cycle can no longer delete one
  id and write another.
- **I3** — `load_competition_share_map` resolves a topic present in several
  groups deterministically and conservatively to its **largest** share (least
  suppression wins), with `ORDER BY group_id, member_topic` for stable iteration.
- **I4** — the association and its limits are documented in code
  (`competition_groups::effective_keyword_weight`) and in this report (see
  below); the previous "ungrouped and unrelated facts are unaffected" claim was
  removed. The product behavior was **not** narrowed (per the fix brief).
- **I5** — the suppression decision was extracted into
  `competition_groups::{suppression_candidates, effective_keyword_weight}`,
  keeping `memory_db.rs` at **999 lines** (under the 1000 hard gate, no
  `// allow-god-file`). M2 (hoisted candidates) and M4 (dead zero-weight branch)
  were folded into the extraction.

## Files Changed (final line counts)

| File | Lines | Change |
|---|---|---|
| `src/agentic/src/topics/competition.rs` | 870 | `COLD_BASELINE_WEIGHT=1.0` replaces `SUPPRESSION_RAW_THRESHOLD=0.20`; durable `CompetitionMember` + `to_group_members`/`rehydrate_group` (explicit group id, I2); two-gate `suppression_state`; 25 tests |
| `src/agentic/src/ports.rs` | 298 | `TopicStore` slimmed to the weight/retrieval contract; new `CompetitionGroupStore` trait (`load_group_members`, `save_group_members`, `load_all_group_members`); object-safety test extended |
| `src/agentic/AGENTS.md` | 50 | §4 constants registered: `SUPPRESSION_SHARE_THRESHOLD=0.15`, `COLD_BASELINE_WEIGHT=1.0`, `MAX_BOOST_PER_EVENT=0.15`, `SHARE_SUM_EPSILON=1e-9` |
| `src/crates/assembly/core/src/service/agent_memory/memory_db.rs` | 999 | `competition_groups` table in the schema batch; `mod competition_groups;`; `search_facts` calls the extracted suppression helpers |
| `src/crates/assembly/core/src/service/agent_memory/memory_db/competition_groups.rs` | 333 | New module: transactional `save_competition_group` (I2 mismatch guard), deterministic `load_competition_share_map` (I3), `suppression_candidates`/`effective_keyword_weight` (I5 + M2/M4) |
| `src/crates/assembly/core/src/agentic/growth_adapter.rs` | 473 | `pub(crate) TopicDbStore<'a>` (M5); `boost_turn_topics` group index + working-map write-back (I1) |
| `src/crates/assembly/core/src/service/agent_memory/mod.rs` | 27 | Re-export `FactConfidence/FactProvenance/FactScope` for crate-internal fact construction |
| `src/crates/assembly/core/src/agentic/growth_adapter/tests.rs` | 858 | +4 T8 adapter tests incl. the I1 same-turn two-member regression |
| `src/crates/assembly/core/src/service/agent_memory/memory_db_tests.rs` | 1098 | +12 T8 tests incl. I2 mismatch guard, I3 duplicate-topic resolution, I5 helper unit tests |

Test totals: `competition.rs` 25, `growth_adapter/tests.rs` 34, `memory_db_tests.rs` 38.
New in this task: 13 tests + 1 added assertion in `open_creates_tables` (growth
crate 168 → 169; core 1213 → 1229 pass).

## Association Limits & Scope (I4)

Natural suppression hides a fact **entirely** only when its sole keyword matches
belong to suppressed topics. The association is **segmented-keyword token
overlap** (`segment_for_fts`), not a fact↔topic relation:

- It can hit generic ASCII tokens (a suppressed member `dependency` overlaps
  every fact containing that word) and CJK bigrams (a suppressed topic
  `依赖安装` segments to `依赖 赖安 安装` and overlaps any fact containing `安装`
  alone), regardless of the group's subject.
- Suppression is **global**: the `competition_groups` table has no
  `workspace_key`, so a group learned in one workspace also suppresses
  `scope = 'global'` facts and facts of every other workspace (consistent with
  the already-global `keyword_weights`).
- Suppressed facts are **not touched by retrieval**: no row, status, or data
  mutation — a visibility-only skip, so a later boost revives them unchanged.
- A fact that also matches any non-suppressed keyword keeps the best
  non-suppressed weight; facts matching no keyword keep the existing 1.0
  baseline.
- Side effect (M7, documented): a suppressed fact stops being `touch_fact`-ed,
  so its recency term keeps decaying until explicitly mentioned again; revival
  is weight-driven and independent.

These limits are documented in the helper's doc comment and were deliberately
**not** narrowed in the fixer (per the fix brief's explicit direction).

## Verification (commands + exact output)

All commands ran in `E:\agent-project\northing\.worktrees\growth-core-0804` with
the required PATH prefix.

### 1. `cargo test -p northhing-agentic-growth`

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
```

```
running 169 tests
test result: ok. 169 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 2. `cargo test -p northhing-core --lib --features product-full`

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-core --lib --features product-full
```

```
test result: ok. 1229 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.72s
```

### 3. `cargo check -p northhing-core --features product-full`

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing-core --features product-full
```

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.71s
```

The 19 warnings are pre-existing (unused `ws`/`last_mentioned_at`/`at_ms`
parameters in `memory_db.rs`/`dream.rs`, `suppress_session_title_generation`,
`active_counter`, etc.) — none are introduced by this task.

### 4. `node scripts/check-core-boundaries.mjs`

```
Core boundary check passed.
```

### Environmental limitation (pre-existing)

`cargo check -p northhing` (desktop) fails locally on an unrelated dependency
issue — `keyring v4.1.6` requires a `v1`/`cli` feature that the desktop
dependency graph does not enable. Reproduced at the pristine baseline `8b64aa8`
(working tree stashed); not caused by this change.

### Remaining limitations (reported, not gate)

- **M11 untested host-adapter warn-only leg**: every group load/save failure at
  the adapter edge is `tracing::warn!` + continue (`growth_adapter.rs`); a
  hermetic test forcing SQLite I/O failure on these tables is brittle and was
  not added — verified by inspection instead.
- **M3**: `load_competition_share_map` degrades to an empty map on read errors
  without `tracing::warn!` (matches pre-existing `load_keyword_weights` style).
- **M8**: one unreadable row fails the whole `load_all_competition_members`
  wholesale for that turn (per-row skip left for triage).
- **M9**: legacy-DB nuance — `(0.0, 1.0)` weights from an older floor would
  score lower than the old `fold(1.0, f64::max)`; today's writers only produce
  `{0.0} ∪ [1.0, 5.0]`, so no live impact.
- **M6/M7**: workspace-global suppression scope and recency-decay side effect —
  documented in the I4 section above.

## Cross-Task Interface

- **T9** reuses `CompetitionGroupStore` + `CompetitionMember` fields
  (`evidence_count`, `source`, `created_at_ms`, `updated_at_ms`, lossless
  round-trip) and the full-replacement `save_group_members` for LLM
  proposal/evidence/rollback cycles; the I2 guard guarantees the explicit
  `group_id` is preserved end-to-end (delete id == insert id).
- **T10** reuses the slimmed `TopicStore` for weight/retrieval wiring.
- **T12** reuses the natural-suppression gate and the same table; the gardener
  reads facts through its own path and is unaffected.
- No T9/T10/T11/T12/T4c work was implemented; no hard-retirement/supersede API
  added; `supersede_fact` untouched (only the pre-existing `dream.rs` call
  site).
