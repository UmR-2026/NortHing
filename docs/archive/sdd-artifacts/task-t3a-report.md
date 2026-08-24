# Task T3a Report - Self-cognition store (storage + one-time migration, no prompt change)

Base commit: `fd61f5e` (branch `feat/growth-core-0804`)

## Files changed

### Modified (tracked)
- `src/crates/assembly/core/src/service/agent_memory/memory_db.rs:46-56` - added
  `conn_locked` pub(crate) helper (10 lines); `:128-133` - added the
  `self_cognition` schema statement inside the existing `CREATE TABLE IF NOT
  EXISTS` batch (6 lines).
- `src/crates/assembly/core/src/service/agent_memory/mod.rs:8` - registered
  `mod self_cognition;`; `:18` - re-exported the access-module functions.
- `src/crates/assembly/core/src/agentic/growth_adapter.rs:1-37` - updated module
  doc + imports; `:117-196` - added `SelfCognitionDbStore` port adapter,
  `init_self_cognition_store`, and `load_self_cognition` warn-only helper.

### New (untracked)
- `src/crates/assembly/core/src/service/agent_memory/self_cognition.rs` (257
  lines) - SQLite access layer for `self_cognition` + one-time migration.
- `src/crates/assembly/core/src/service/agent_memory/self_cognition_tests.rs`
  (395 lines) - 15 tests covering all 8 brief cases.

## Exact schema statement

Added inside `MemoryDb::create_tables`'s `execute_batch`, right after
`fact_reviews` (same `CREATE TABLE IF NOT EXISTS` pattern as the four existing
tables, so it runs on every open and needs no version bump):

```sql
CREATE TABLE IF NOT EXISTS self_cognition (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    trigger TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

No `workspace_key` column (brief §2/§3.1: self-cognition is global by design,
mirroring `identity.md`). `id` is a uuid, same style as `facts`.

## How idempotence is guaranteed (reviewer's focus)

**Round 2 (current): idempotence by migration-row identity, not "table empty".**

The migration inserts under a **fixed deterministic id**
(`MIGRATION_ROW_ID` = `"migration:identity-md"`) using `INSERT OR IGNORE`. The
guard is "does the migration row exist", not "is the table empty":

1. `migrate_identity_into_self_cognition(db)` reads `identity.md` (if absent
   or empty after BOM-strip+trim, it returns - empty store is valid).
2. It calls `insert_migration_row(db, &content, created_at)`, which issues
   `INSERT OR IGNORE INTO self_cognition (id, text, trigger, created_at)
   VALUES ('migration:identity-md', ?, 'migrated-from-identity-md', ?)`.
3. `INSERT OR IGNORE` returns 1 row affected on first insert, 0 if the row
   already exists (PRIMARY KEY conflict, silently ignored). The function
   returns `true` (inserted) or `false` (already present).
4. No "done" marker is written on the failure path. If the insert fails (e.g.
   transient DB error), no row is left behind, so the next initialization
   retries.

This fixes the two round-1 defects:
- **Race**: two concurrent initializers both attempt the insert; the first
  wins, the second hits the PRIMARY KEY and is ignored. Two migration rows
  are impossible without a `BEGIN IMMEDIATE` transaction.
- **Onboarding loss**: a pre-existing non-migration note (e.g. a future T17
  agent write) no longer blocks the migration - the guard is the migration
  row's identity, not the table row count. The onboarding paragraph is still
  imported.

This survives restarts, crashes, and DB reopens because the migration row
persists in the SQLite file. Verified by `migration_idempotent_across_db_reopen`
(reopen the same DB file; migration must not re-run) and
`migration_runs_even_when_table_has_non_migration_note` (a pre-existing note
does not block the migration).

## Call sites added (§3.5 - D9 agent-exclusive)

**No production call sites were added.** Per brief §3.5, adding no call sites
is acceptable and expected; the consumer arrives in T3b/T17. The new symbols
`init_self_cognition_store`, `SelfCognitionDbStore::new`, and
`load_self_cognition` (growth_adapter) have no production callers - they are
the intended entry points for future wiring. No judge-mom, dream/garden, or
review code path references the new store or port.

### D9 side effect: `conn_locked` escape hatch (I-4, addressed in round 2)

The new `pub(crate) fn conn_locked` (`memory_db.rs:50`) returns a raw
`MutexGuard<Connection>`, so any crate-internal caller with a `&MemoryDb` can
issue arbitrary SQL against any table, including `self_cognition`. Before this
helper existed, `MemoryDb.conn` was a private field and each table had only
narrowly-scoped `pub(crate)` methods, so judge-mom / dream / review paths
could not reach `self_cognition` (the table did not exist).

**Current state**: the crate callers (`dream.rs`, `judge_memory.rs`,
`auto_memory.rs`, `turn_persist_facts`) do **not** call `conn_locked()`, so D9
is not violated today. But the structural enforcement is weakened: the
boundary now relies on caller self-discipline.

**Action taken in round 2**: the `conn_locked` doc comment now explicitly
documents this as an escape hatch, states that judge-mom/dream/review paths
must not use it to read `self_cognition`, and names T7 as the owner of the
hard enforcement (`forbidden-rules.mjs` entry banning `\bconn_locked\b` in
those files). The hard enforcement is deliberately out of scope for T3a --
within the same crate, visibility alone cannot truly isolate judge-mom from
`self_cognition` because they are siblings in the same module tree.

## Intentionally-unused new production symbols

The crate has `#![allow(dead_code)]` / `#![allow(unused_imports)]`, so these do
not emit warnings, but they have no production caller yet (by design):

- `growth_adapter::init_self_cognition_store` - future wiring entry point.
- `growth_adapter::SelfCognitionDbStore::new` - used by `init_self_cognition_store`
  and tests only.
- `growth_adapter::load_self_cognition` - warn-only loader for future consumer.
- `self_cognition::SelfCognitionRow` - returned by `load_self_cognition`, used by
  the adapter and tests.

All other new symbols (`append_self_cognition`, `count_self_cognition`,
`load_self_cognition`, `migrate_identity_into_self_cognition`,
`resolve_identity_path`, `read_identity_content`, `identity_mtime_epoch_ms`,
`wall_now_ms`) are used within the module or by the adapter.

## Append-only invariant (§5.8)

The new module `self_cognition.rs` issues only three SQL statement kinds:
`SELECT ... FROM self_cognition` (load), `INSERT INTO self_cognition` (append),
and `SELECT COUNT(*) FROM self_cognition` (count). Grep evidence:

```
$ Select-String -Path self_cognition.rs -Pattern "UPDATE|DELETE" -CaseSensitive
(only matches are in doc comments asserting the invariant, lines 9, 10, 65)
```

No `UPDATE` or `DELETE` against `self_cognition` exists in the module source.

## §6.1 Schema-safety check

The schema-init path is **not version-gated**. `MemoryDb::open` always calls
`create_tables`, which runs `CREATE TABLE IF NOT EXISTS self_cognition (...)` in
the same `execute_batch` as the four existing tables. Opening an existing DB
created before this change succeeds and gains the new table on the next open -
no migration version bump is needed.

Verified by `existing_db_gains_self_cognition_table_on_reopen`: create a DB,
drop `self_cognition` to simulate a pre-existing DB, reopen, and confirm the
table reappears (COUNT from `sqlite_master` = 1) and is usable (empty, no
error).

## §6.2 Proof of no prompt change

`git diff --stat -- src/crates/assembly/core/src/agentic/agents/prompt_builder/`
produces **no output** (no modifications to any prompt-building file).

`cargo test -p northhing-core --features product-full prompt_injection` passes
unchanged: **4 passed; 0 failed** (see full verification output below).

`system_prompt.rs` was not touched (not in the diff).

## Verification (complete raw stdout+stderr)

Prefix for all cargo commands: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 1. `cargo test -p northhing-agentic-growth` - must be exactly 139

```
running 139 tests
test error::tests::error_display_includes_context ... ok
test negation::tests::case_insensitive_english ... ok
test negation::tests::chinese_fact_is_wrong ... ok
test negation::tests::chinese_preference_replaced ... ok
test negation::tests::chinese_stop_remembering ... ok
test negation::tests::no_hit_false_friend_ji ... ok
test negation::tests::english_preference_replaced ... ok
test negation::tests::no_hit_empty_or_whitespace ... ok
test negation::tests::english_fact_is_wrong ... ok
test negation::tests::no_hit_not_great ... ok
test negation::tests::parse_zero_candidates_always_empty ... ok
test negation::tests::parse_malformed_returns_empty ... ok
test negation::tests::parse_duplicates_deduped ... ok
test negation::tests::parse_negative_float_string_dropped ... ok
test negation::tests::english_stop_remembering ... ok
test negation::tests::parse_out_of_range_dropped ... ok
test negation::tests::parse_with_json_fence ... ok
test negation::tests::parse_with_surrounding_prose ... ok
test negation::tests::no_hit_vague_negative_chinese ... ok
test negation::tests::priority_fact_is_wrong_over_preference ... ok
test negation::tests::prompt_candidates_numbered_without_fact_id ... ok
test negation::tests::prompt_contains_user_message_tags_and_original_text ... ok
test scheduler::tests::below_auto_pause_threshold_no_event ... ok
test negation::tests::target_hint_extracted ... ok
test ports::tests::test_fact_type_round_trip ... ok
test ports::tests::test_fact_status_round_trip ... ok
test negation::tests::target_hint_none_when_nothing_after ... ok
test scheduler::tests::decide_turn_distill_paused_garden_open ... ok
test negation::tests::prompt_empty_candidates_does_not_panic ... ok
test negation::tests::same_kind_earliest_phrase_wins ... ok
test ports::tests::test_object_safety ... ok
test ports::tests::test_reviewer_round_trip ... ok
test negation::tests::parse_simple_valid ... ok
test scheduler::tests::after_garden_sweep_gate_is_closed ... ok
test scheduler::tests::all_wake_phrases_match ... ok
test scheduler::tests::garden_sweep_exact_interval_returns_true ... ok
test scheduler::tests::auto_pause_event_fires_only_once ... ok
test scheduler::tests::bare_negators_do_not_trigger_wake ... ok
test scheduler::tests::garden_sweep_one_ms_below_interval_returns_false ... ok
test scheduler::tests::decide_turn_both_closed ... ok
test scheduler::tests::decide_turn_distill_open_garden_not_due ... ok
test ports::tests::test_fake_clock ... ok
test scheduler::tests::garden_sweep_both_zero_returns_false ... ok
test scheduler::tests::garden_sweep_clock_backwards_returns_false ... ok
test scheduler::tests::garden_sweep_from_zero_to_interval_returns_true ... ok
test scheduler::tests::decide_turn_both_gates_open ... ok
test scheduler::tests::has_hit_turns_does_not_pause ... ok
test scheduler::tests::hit_turns_increments_only_on_produced_facts ... ok
test negation::tests::target_hint_capped_at_60_chars ... ok
test scheduler::tests::old_blob_without_paused_at_turns_deserialises ... ok
test scheduler::tests::paused_state_still_increments_turns ... ok
test scheduler::tests::probe_hit_resumes_and_resets_window ... ok
test scheduler::tests::old_growth_state_blob_without_paused_at_turns_round_trips ... ok
test scheduler::tests::probe_miss_stays_paused_and_next_window_arrives ... ok
test scheduler::tests::probe_resume_event_fires_once ... ok
test scheduler::tests::probe_window_first_probe_at_anchor_plus_n ... ok
test scheduler::tests::resume_event_fires_only_on_transition ... ok
test scheduler::tests::saturating_add_at_max_does_not_panic ... ok
test scheduler::tests::should_distill_returns_false_when_paused ... ok
test scheduler::tests::should_distill_returns_true_when_not_paused ... ok
test scheduler::tests::triggers_auto_pause_at_twenty ... ok
test scheduler::tests::wake_phrase_resumes_resets_window_and_distils ... ok
test state::tests::test_bad_json ... ok
test state::tests::test_blob_exists_and_valid ... ok
test state::tests::test_migration_all_legacy_present ... ok
test state::tests::test_migration_dirty_legacy_keys ... ok
test state::tests::test_migration_idempotent ... ok
test state::tests::test_migration_no_legacy_keys ... ok
test state::tests::test_migration_port_error_on_legacy ... ok
test state::tests::test_port_error_load ... ok
test state::tests::test_port_error_save ... ok
test state::tests::test_unknown_schema_version ... ok
test topics::competition::tests::all_zero_weights_split_equally ... ok
test topics::competition::tests::boost_clamp_and_negative_noop ... ok
test topics::competition::tests::boost_inserts_new_topic ... ok
test topics::competition::tests::boost_rise_causes_fall ... ok
test topics::competition::tests::duplicate_topic_boost_and_health ... ok
test topics::competition::tests::empty_group_handling ... ok
test topics::competition::tests::health_healthy_group ... ok
test topics::competition::tests::health_sum_drift ... ok
test topics::competition::tests::health_out_of_range ... ok
test topics::competition::tests::no_member_removed_by_boost ... ok
test topics::competition::tests::nan_and_negative_treated_as_zero ... ok
test topics::competition::tests::revive_already_above_returns_none ... ok
test topics::competition::tests::single_member_group ... ok
test topics::competition::tests::sum_conservation_over_many_boosts ... ok
test topics::competition::tests::suppressed_member_can_revive ... ok
test topics::competition::tests::revive_extreme_group_returns ... ok
test topics::competition::tests::suppression_both_below ... ok
test topics::competition::tests::suppression_boundary_strict_less_than ... ok
test topics::competition::tests::suppression_raw_high_stays_active ... ok
test topics::competition::tests::suppression_share_high_stays_active ... ok
test topics::competition::tests::zero_share_can_rise ... ok
test topics::extract::tests::ascii_case_normalized_to_lowercase ... ok
test topics::extract::tests::ascii_connector_chars_survive_inside_tokens ... ok
test topics::extract::tests::ascii_stopwords_are_filtered ... ok
test topics::extract::tests::at_most_max_topics_returned ... ok
test topics::extract::tests::cjk_stopwords_are_filtered ... ok
test topics::extract::tests::connector_chars_stripped_from_ends ... ok
test topics::extract::tests::duplicate_tokens_are_deduplicated ... ok
test topics::extract::tests::empty_input_yields_empty_result ... ok
test topics::extract::tests::long_cjk_topic_is_truncated_by_char_count ... ok
test topics::extract::tests::mixed_cjk_ascii_contains_both_kinds ... ok
test topics::extract::tests::normalize_candidates_caps_at_max_topics ... ok
test topics::extract::tests::normalize_candidates_dedups_case_variants ... ok
test topics::extract::tests::normalize_candidates_discards_empty_whitespace_control ... ok
test topics::extract::tests::normalize_candidates_discards_short_ascii ... ok
test topics::extract::tests::normalize_candidates_discards_short_cjk ... ok
test topics::extract::tests::normalize_candidates_matches_extract_topics_key_space ... ok
test topics::extract::tests::normalize_candidates_preserves_connectors ... ok
test topics::extract::tests::normalize_candidates_truncates_long_candidate ... ok
test topics::extract::tests::only_punctuation_yields_empty_result ... ok
test topics::extract::tests::only_whitespace_yields_empty_result ... ok
test topics::extract::tests::pure_ascii_filters_stopwords_and_short_tokens ... ok
test topics::extract::tests::pure_cjk_keeps_contiguous_run_as_one_topic ... ok
test topics::extract::tests::pure_digit_tokens_are_filtered ... ok
test topics::extract::tests::same_input_produces_same_output_twice ... ok
test topics::extract::tests::short_words_and_single_chars_yield_empty ... ok
test topics::score::tests::best_weight_all_nan ... ok
test topics::score::tests::best_weight_empty ... ok
test topics::score::tests::best_weight_ignores_nan ... ok
test topics::score::tests::dominance_property_loop ... ok
test topics::score::tests::dominance_tw055_es0_loses_to_tw05_es1 ... ok
test topics::score::tests::dominance_tw09_es0_beats_tw05_es1 ... ok
test topics::score::tests::rank_below_floor_dropped ... ok
test topics::score::tests::rank_descending_score ... ok
test topics::score::tests::rank_empty ... ok
test topics::score::tests::rank_nan_candidate_no_panic ... ok
test topics::score::tests::rank_tie_different_tw ... ok
test topics::score::tests::rank_tie_same_score_different_id ... ok
test topics::score::tests::retrieval_floor_only ... ok
test topics::score::tests::retrieval_upper_bound ... ok
test topics::score::tests::retrieval_zero_tw ... ok
test topics::score::tests::sanitize_infinity ... ok
test topics::score::tests::sanitize_mid ... ok
test topics::score::tests::sanitize_nan ... ok
test topics::score::tests::sanitize_neg_infinity ... ok
test topics::score::tests::sanitize_negative ... ok
test topics::score::tests::sanitize_overflow ... ok

test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Result: 139 passed.** (unchanged - no crate code added)

### 2. `cargo check -p northhing-core --features product-full` - warning baseline 19, must not increase

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.04s

warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
```

**Result: 19 warnings** (baseline 19, unchanged). No new warnings from the
added code (verified by grepping the full output for `self_cognition`,
`growth_adapter`, `identity` - no matches in warning lines).

### 3. `cargo test -p northhing-core --features product-full self_cognition` (new; report total)

```
running 15 tests
test service::agent_memory::self_cognition::tests::append_then_load_round_trips_fields ... ok
test service::agent_memory::self_cognition::tests::port_adapter_append_and_load_round_trips ... ok
test service::agent_memory::self_cognition::tests::load_on_fresh_db_returns_empty_vec ... ok
test service::agent_memory::self_cognition::tests::load_orders_oldest_first ... ok
test service::agent_memory::self_cognition::tests::load_tiebreak_by_id_for_total_order ... ok
test service::agent_memory::self_cognition::tests::migration_imports_identity_md_when_table_empty ... ok
test service::agent_memory::self_cognition::tests::migration_runs_at_most_once_across_multiple_inits ... ok
test service::agent_memory::self_cognition::tests::migration_idempotent_across_db_reopen ... ok
test service::agent_memory::self_cognition::tests::migration_does_not_modify_identity_md ... ok
test service::agent_memory::self_cognition::tests::migration_skipped_when_table_non_empty ... ok
test service::agent_memory::self_cognition::tests::migration_skipped_when_identity_md_absent ... ok
test service::agent_memory::self_cognition::tests::migration_skipped_when_identity_md_empty_after_trim ... ok
test service::agent_memory::self_cognition::tests::load_self_cognition_returns_empty_on_fresh_db ... ok
test service::agent_memory::self_cognition::tests::existing_db_gains_self_cognition_table_on_reopen ... ok
test service::agent_memory::self_cognition::tests::open_creates_self_cognition_table ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1189 filtered out; finished in 0.21s
```

**Result: 15 passed.** Coverage maps to brief §5:
1. `append_then_load_round_trips_fields` (+ port adapter variant) - case 1.
2. `load_on_fresh_db_returns_empty_vec` - case 2.
3. `load_orders_oldest_first` + `load_tiebreak_by_id_for_total_order` - case 3.
4. `migration_imports_identity_md_when_table_empty` - case 4 (happy path).
5. `migration_runs_at_most_once_across_multiple_inits` +
   `migration_idempotent_across_db_reopen` - case 5 (highest-value).
6. `migration_does_not_modify_identity_md` - case 6 (non-destructive).
7. `migration_skipped_when_table_non_empty` - case 7.
8. Append-only: argued from module source above (grep evidence; no
   UPDATE/DELETE SQL in the module).

### 4. `cargo test -p northhing-core --features product-full memory_db` (28 now)

```
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 1176 filtered out; finished in 0.29s
```

**Result: 28 passed** (unchanged).

### 5. `cargo test -p northhing-core --features product-full growth_adapter` (30 now)

```
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 1174 filtered out; finished in 0.36s
```

**Result: 30 passed** (unchanged; the growth_adapter tests do not test
self-cognition, which lives in its own test module).

### 6. `node scripts/check-core-boundaries.mjs` - exit 0

```
Core boundary check passed.
```

**Exit code: 0.** No boundary rules apply to the new `self_cognition.rs`
(only `auto_memory.rs` has an `agent_memory`-scoped rule, forbidding
`judge_memory` references, which this module does not use).

### 7. Line counts via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count`

```
src\crates\assembly\core\src\agentic\growth_adapter.rs                 364
src\crates\assembly\core\src\service\agent_memory\memory_db.rs         961
src\crates\assembly\core\src\service\agent_memory\mod.rs               22
src\crates\assembly\core\src\service\agent_memory\self_cognition.rs    257
src\crates\assembly\core\src\service\agent_memory\self_cognition_tests.rs 395
```

- `memory_db.rs` is 961 lines: pre-existing over the 800 cap (943 at base,
  owned by T7 per brief §2). This task added 18 lines (the `conn_locked`
  helper + the schema statement), which brief §3.2 explicitly permits
  ("Only the schema statement itself may need to touch `memory_db.rs`; keep
  any such addition minimal"). The shared `conn_locked` helper is the minimal
  seam needed so the access module can reach the private `conn` without
  duplicating the lock-poison boilerplate in `memory_db.rs`.
- `memory_db_tests.rs` is **799 lines** (unchanged, at the cap - a new test
  file was started instead of growing it, per brief §4).
- All new files are well under 800.

### §6.2 (repeated) Proof of no prompt change

```
$ git diff --stat -- src/crates/assembly/core/src/agentic/agents/prompt_builder/
(no output - no modifications)
```

```
$ cargo test -p northhing-core --features product-full prompt_injection
running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1200 filtered out; finished in 0.12s
```

Both confirm no prompt change.

## Ambiguities / notes

- The brief §3.2 says "Put all SQLite access for this table in a new file ...
  not in `memory_db.rs`." The private `conn` field prevents the access module
  from reaching the connection directly, so a minimal `pub(crate) fn
  conn_locked` helper was added to `MemoryDb` (10 lines). All table-specific
  SQL (SELECT/INSERT/COUNT, the migration logic, the identity-path override)
  lives in `self_cognition.rs`. This is the smallest seam that satisfies both
  "access logic lives in the new file" and "keep the `memory_db.rs` addition
  minimal."
- The migration's `created_at` uses the file mtime in epoch ms, falling back
  to wall-clock now if mtime is unavailable (brief §3.4). On Windows, mtime
  is reliably obtainable via `std::fs::metadata(...).modified()`.
- `init_self_cognition_store` runs the migration then returns the store. It
  is `pub(crate)` with no production caller (by design, §3.5). When T3b
  wires a consumer, it should call `init_self_cognition_store(db)` once at
  startup and reuse the returned store.

---

## Round 2: fixes for review findings

Review verdict: SPEC PASS WITH FINDINGS / QUALITY PASS WITH FINDINGS
(0 Critical / 4 Important / 6 Minor). Full review:
`task-t3a-review.md`. The orchestrator ruled which findings are fixed in this
task vs. deferred to T7. Commit: `39fadea` (on top of `258d2ea`).

### 必修 1 (I-1 + I-2 + 专项一③): migration guard by row identity

**Problem (round 1)**: the guard was `count_self_cognition == 0`. Two defects:
(a) a TOCTOU race between the count and the insert could produce two migration
rows under concurrent initialization; (b) once T17 lets the agent write notes,
any non-migration note landing first would make "table non-empty" and
permanently skip the identity.md migration -- silently losing the user's
onboarding self-cognition.

**Fix**: replaced the table-empty guard with **migration-row identity**. The
migration now inserts under a fixed deterministic id
(`MIGRATION_ROW_ID = "migration:identity-md"`) using `INSERT OR IGNORE`:

```rust
const MIGRATION_ROW_ID: &str = "migration:identity-md";

fn insert_migration_row(db: &MemoryDb, text: &str, created_at: u64) -> NortHingResult<bool> {
    let conn = db.conn_locked()?;
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO self_cognition (id, text, trigger, created_at) \
         VALUES (?1, ?2, ?3, ?4)",
        params![MIGRATION_ROW_ID, text, MIGRATION_TRIGGER, created_at as i64],
    ).map_err(...)?;
    Ok(inserted > 0)
}
```

This fixes all three problems at once:
1. **Race**: `INSERT OR IGNORE` makes the insert atomic; a racing second
   initializer hits the PRIMARY KEY and is silently ignored. No `BEGIN
   IMMEDIATE` transaction is needed (the reviewer's suggested marker table +
   transaction approach is superseded by this simpler, more thorough design).
2. **Onboarding loss**: a pre-existing non-migration note does NOT block the
   migration -- the guard is "does the migration row exist", not "is the
   table empty". The onboarding paragraph is still imported.
3. **No extra marker table**: the migration row itself (with its deterministic
   id) is the marker. No separate `migration_marker` table is needed.

**New migration guard semantics (authoritative)**:
- The migration runs whenever `identity.md` is present and non-empty (after
  BOM-strip + trim).
- It attempts `INSERT OR IGNORE` under the fixed id. First call inserts;
  any later call (serial or racing) is a no-op.
- A failed insert leaves no row, so the next init retries.
- After the migration row exists, later changes to `identity.md` are NOT
  re-imported (migration is one-time, not a sync).

`count_self_cognition` is now diagnostic-only (no longer the guard); it
remains exported and tested.

**Tests changed/added**:
- `migration_skipped_when_table_non_empty` -> renamed to
  `migration_runs_even_when_table_has_non_migration_note`: a pre-existing
  non-migration note + identity.md present => migration STILL runs, result is
  2 rows (1 pre-existing + 1 migration with id `"migration:identity-md"`).
- `migration_runs_at_most_once_across_multiple_inits`: retained; 3 inits still
  yield exactly 1 migration row.
- `migration_idempotent_across_db_reopen`: retained; reopen does not re-run.
- NEW `migration_does_not_overwrite_or_append_when_identity_md_changed`:
  after migration, change identity.md content, re-init => no overwrite, no
  second row, original content retained.

### 必修 2 (M-1): BOM stripping

**Problem**: `read_identity_content` only trimmed surrounding whitespace; a
leading UTF-8 BOM (`\u{FEFF}`) would be carried verbatim into the `text`
column and later into prompts.

**Fix**: strip a leading BOM before trimming:
```rust
let stripped = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
let trimmed = stripped.trim();
```
Only the BOM and surrounding whitespace are removed; the rest is verbatim (no
re-wrapping, reordering, or translation).

**Tests added**:
- `migration_strips_utf8_bom_when_importing_identity`: identity.md with a
  leading BOM => stored text has no BOM, body preserved.
- `migration_preserves_internal_newlines_in_identity_md`: multi-line content
  => internal newlines preserved, only surrounding whitespace trimmed.

### 必修 3 (I-4): D9 `conn_locked` escape hatch documented

**Problem**: the report's §3.5 only said "no production call sites", without
mentioning that `conn_locked` opens a raw-SQL escape hatch for any crate
caller with a `&MemoryDb`.

**Fix**:
1. The `conn_locked` doc comment (`memory_db.rs:47-69`) now explicitly
   documents it as an escape hatch that bypasses per-table access
   constraints, states that judge-mom/dream/review paths must not use it to
   read `self_cognition`, and names T7 as the owner of the hard enforcement
   (`forbidden-rules.mjs` entry banning `\bconn_locked\b` in those files).
2. This report's §3.5 now has a dedicated subsection "D9 side effect:
   `conn_locked` escape hatch" (above) describing the current state, the
   weakened structural enforcement, and the T7 handoff.

**Not done (deferred to T7 by orchestrator ruling)**: the
`forbidden-rules.mjs` entry. Within the same crate, visibility alone cannot
truly isolate judge-mom from `self_cognition` (they are siblings in the same
module tree); the hard enforcement belongs to T7's permission-matrix work.

### Findings not addressed in this task (orchestrator deferred to T7 or终审 triage)

- I-3 (`conn_locked` structural enforcement via `forbidden-rules.mjs`): T7.
- M-2 (`unwrap_or_else` style): 留终审 triage. The two call sites are
  controlled fallbacks that never panic and match existing `memory_db.rs`
  idiom; no change.
- M-3 (manual `fs::remove_file` instead of `MemoryDbPathGuard` RAII): 留终审
  triage. Current style is explicit and tests pass; no change.
- M-5 (`load_self_cognition` warn-only helper no production caller): by
  design, T17 wiring; no change.
- M-6 (report line-count footnote for growth_adapter growth): addressed here
  -- growth_adapter.rs grew from 266 to 364 lines because brief §3.3
  explicitly requires the `SelfCognitionStore` adapter to live there (same
  pattern as `JudgeMomStateStore`).

## Round 2 verification (complete raw stdout+stderr)

Prefix: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 1. `cargo test -p northhing-agentic-growth` (must still be 139)

```
running 139 tests
...
test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Result: 139 passed** (unchanged - no crate code touched; `src/agentic/`
not in diff).

### 2. `cargo check -p northhing-core --features product-full` (baseline 19, must not increase)

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.00s
```

**Result: 19 warnings** (baseline 19, unchanged). No new warnings from the
round-2 changes (verified: no `self_cognition`/`growth_adapter`/`conn_locked`
lines appear in warning output).

### 3. `cargo test -p northhing-core --features product-full self_cognition` (was 15, report new total)

```
running 18 tests
test service::agent_memory::self_cognition::tests::migration_skipped_when_identity_md_absent ... ok
test service::agent_memory::self_cognition::tests::append_then_load_round_trips_fields ... ok
test service::agent_memory::self_cognition::tests::open_creates_self_cognition_table ... ok
test service::agent_memory::self_cognition::tests::load_on_fresh_db_returns_empty_vec ... ok
test service::agent_memory::self_cognition::tests::load_self_cognition_returns_empty_on_fresh_db ... ok
test service::agent_memory::self_cognition::tests::migration_strips_utf8_bom_when_importing_identity ... ok
test service::agent_memory::self_cognition::tests::load_orders_oldest_first ... ok
test service::agent_memory::self_cognition::tests::migration_skipped_when_identity_md_empty_after_trim ... ok
test service::agent_memory::self_cognition::tests::migration_does_not_overwrite_or_append_when_identity_md_changed ... ok
test service::agent_memory::self_cognition::tests::migration_runs_at_most_once_across_multiple_inits ... ok
test service::agent_memory::self_cognition::tests::migration_does_not_modify_identity_md ... ok
test service::agent_memory::self_cognition::tests::migration_imports_identity_md_when_table_empty ... ok
test service::agent_memory::self_cognition::tests::port_adapter_append_and_load_round_trips ... ok
test service::agent_memory::self_cognition::tests::load_tiebreak_by_id_for_total_order ... ok
test service::agent_memory::self_cognition::tests::migration_preserves_internal_newlines_in_identity_md ... ok
test service::agent_memory::self_cognition::tests::migration_runs_even_when_table_has_non_migration_note ... ok
test service::agent_memory::self_cognition::tests::migration_idempotent_across_db_reopen ... ok
test service::agent_memory::self_cognition::tests::existing_db_gains_self_cognition_table_on_reopen ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 1189 filtered out; finished in 0.23s
```

**Result: 18 passed** (was 15; +3 new tests for round-2 fixes). The 3 new
tests: `migration_does_not_overwrite_or_append_when_identity_md_changed`,
`migration_strips_utf8_bom_when_importing_identity`,
`migration_preserves_internal_newlines_in_identity_md`. The renamed test
`migration_runs_even_when_table_has_non_migration_note` asserts the new
identity-based guard semantics.

### 4. `cargo test -p northhing-core --features product-full memory_db` (28)

```
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 1179 filtered out; finished in 0.29s
```

**Result: 28 passed** (unchanged).

### 5. `cargo test -p northhing-core --features product-full growth_adapter` (30)

```
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 1177 filtered out; finished in 0.36s
```

**Result: 30 passed** (unchanged).

### 6. `cargo test -p northhing-core --features product-full prompt_injection` (4, proof of no prompt change)

```
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1203 filtered out; finished in 0.11s
```

**Result: 4 passed** (unchanged). `git diff --stat -- src/.../prompt_builder/`
is empty (no prompt files touched in either round).

### 7. `node scripts/check-core-boundaries.mjs` (exit 0)

```
Core boundary check passed.
```

**Exit code: 0.** No `scripts/core-boundaries/**` files were modified (the
`forbidden-rules.mjs` `\bconn_locked\b` rule is deferred to T7).

### Line counts (round 2)

```
src\crates\assembly\core\src\agentic\growth_adapter.rs                 364  (unchanged from round 1)
src\crates\assembly\core\src\service\agent_memory\memory_db.rs         980  (+19 from round 1: conn_locked doc)
src\crates\assembly\core\src\service\agent_memory\mod.rs               22   (unchanged)
src\crates\assembly\core\src\service\agent_memory\self_cognition.rs    320  (+63 from round 1: new guard + BOM + docs)
src\crates\assembly\core\src\service\agent_memory\self_cognition_tests.rs 502  (+107 from round 1: 3 new tests)
```

- `memory_db.rs` is 980 lines: pre-existing over the 800 cap (943 at base,
  owned by T7). Round 2 added 19 lines (the `conn_locked` doc comment for
  I-4). Brief §3.2 permits minimal touches to `memory_db.rs`.
- All new files are well under 800.
- `growth_adapter.rs` growth (266 -> 364) is at the location brief §3.3
  explicitly requires (same pattern as `JudgeMomStateStore`).

### Append-only invariant (re-verified round 2)

`self_cognition.rs` still issues only `SELECT` (load), `INSERT`/`INSERT OR
IGNORE` (append/migration), and `SELECT COUNT(*)` (count). No `UPDATE` or
`DELETE` against `self_cognition` in the module source (grep matches are all
in doc comments asserting the invariant).

DONE
