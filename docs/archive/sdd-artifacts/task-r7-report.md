# Task R-7 Report: Facts Distillation Main-Session Gate

## Round 2: fixes for review findings

This round addresses the four findings from the Round 1 review
(`task-r7-review.md`): C1, I1, I2, I3. All changes are in the same single
file (`turn_persist.rs`); no second file was needed.

### C1 — In-memory lookup failure can suppress a valid main-dialog turn after eviction

**Fix:** `resolve_distill_signals` now falls back to persisted
`SessionMetadata` when `session_manager.get_session(session_id)` returns
`None`. The fallback resolves the workspace path via
`session_manager.resolve_session_workspace_path(session_id)` (which consults
the retained `session_workspace_index` and, if needed, scans workspaces --
`session_manager_workspace_path.rs:71-136`), then loads persisted metadata
via `session_manager.persistence_manager.load_session_metadata(...)`
(`metadata_subhandlers.rs:115-124`). The persisted `SessionMetadata` carries
all three classification fields (`session_kind`, `relationship.parent_session_id`,
`created_by`), so the same predicate applies.

Only when **both** the in-memory cache and persisted storage fail to
establish metadata does the gate deny with a `warn!` (fail-closed). This
removes the C1 regression: an evicted main turn that was persisted by the
cleanup task (`session_manager_auto_save_cleanup.rs:200-217` saves before
removing) is now recovered from disk and distilled.

**Why no second file was needed:** `SessionManager.persistence_manager`
(`session_manager.rs:118`) is already `pub(crate)`, and
`resolve_session_workspace_path` (`session_manager_workspace_path.rs:71`) is
already `pub(crate)`. Both are accessible from `turn_persist.rs` (same crate),
so the fallback was implemented entirely within the one file. No new pub
methods, no schema changes.

### I1 — Required fail-closed lookup/missing-metadata test was absent

**Fix:** the predicate was refactored from
`is_main_dialog_session(parent: Option<&str>, created_by: Option<&str>) -> bool`
to `should_distill_facts(signals: Option<&SessionSignals>) -> bool`. The outer
`Option` now represents metadata availability: `None` => deny (fail-closed),
`Some(signals)` => classify. This distinguishes "known main session with no
child markers" (allow) from "metadata unavailable" (deny), which the previous
`(None, None)`-means-allow design could not.

The new test `none_signals_denies_distillation` asserts `None => false`. The
call site (`resolve_distill_signals`) is the only place that produces `None`,
by exhausting both metadata sources.

### I2 — One public hidden-subagent creation path can bypass both checked signals

**Fix:** `SessionKind` is now the **primary** signal.
`should_distill_facts` rejects `SessionKind::Subagent` and
`SessionKind::EphemeralChild` regardless of the other fields.

`EphemeralChild` semantics verified: it is the `/btw` side-thread child kind,
created at `so_handlers.rs:67` via `create_session_with_id_and_details` with
`SessionKind::EphemeralChild`. `is_internal_hidden`
(`session_metadata.rs:355-357`) treats both `Subagent` and `EphemeralChild` as
non-user-input child sessions. `should_persist_session_kind`
(`session_manager_persistence_predicate.rs:58-63`) persists `Subagent` but
not `EphemeralChild`; an `EphemeralChild` turn therefore never reaches the
facts gate via the normal persist path (it returns at
`should_persist_session_id`), but the kind check is still correct defense in
depth.

This closes the I2 bypass: the public `create_hidden_subagent_session_with_workspace`
(`coordinator_session.rs:139-154`) can create a `SessionKind::Subagent` with
`created_by = None` and no in-memory `relationship`, but the kind signal now
catches it. The regression test
`subagent_kind_no_parent_no_creator_is_rejected` pins this.

`parent_session_id` and `created_by` "session-*" are retained as
defense-in-depth fallback signals (any one of the three rejects).

### I3 — Verification evidence was incomplete; reported line count was wrong

**Root cause of the wrong line count:** Round 1 used
`(Get-Content $f | Measure-Object -Line).Lines`, which counts *newline
characters* (708), not *lines*. The reviewer and the Read tool count array
elements / display line numbers (781), which is the correct interpretation of
"file line count". Round 2 uses `(Get-Content $f).Count`, which matches.

**Fix:** all six brief commands were rerun with complete stdout+stderr
captured verbatim (see §Verification below). `cargo fmt` was not run.

---

## Status: DONE

## Signal selection (verified against code, Round 2)

`agent_type` is NOT used (persona name, not a main/sub marker). Three
signals, any one marks a non-main session:

1. **`SessionKind::Subagent` / `SessionKind::EphemeralChild`** (primary,
   semantically correct). `SessionKind` is defined at
   `core-types/src/session.rs:5-10` with variants `Standard`, `Subagent`,
   `EphemeralChild`. Set by the subagent creation paths:
   - `Subagent`: `so_lifecycle/spawn.rs` via `create_hidden_subagent_session`
     (which sets `SessionKind::Subagent`).
   - `EphemeralChild`: `so_handlers.rs:67` (`/btw` side-thread child).
   - Both are treated as internal/hidden by `is_internal_hidden`
     (`session_metadata.rs:355-357`).
   - Catches the I2 bypass path (`create_hidden_subagent_session_with_workspace`,
     `coordinator_session.rs:139-154`) that can set neither creator nor
     in-memory parent.

2. **`relationship.parent_session_id`** (fallback). Set by
   `build_subagent_session_relationship` (`so_types.rs:55-68`) for dispatched
   subagents. Available both in-memory (`Session.relationship`) and in
   persisted metadata (`SessionMetadata.relationship`).

3. **`created_by` starts with `"session-"`** (fallback). Set by
   `so_dispatch.rs:45` and `so_handlers.rs:66` as
   `format!("session-{}", parent_session_id)`. Available both in-memory
   (`Session.created_by`) and in persisted metadata
   (`SessionMetadata.created_by`). The `"session-"` prefix check mirrors the
   existing helper at `session_metadata.rs:363`.

## Fail-closed implementation

- **Pure predicate** `should_distill_facts(signals: Option<&SessionSignals>) -> bool`:
  `None` => `false` (metadata unavailable => deny). `SessionSignals` is a
  plain-data struct owning its strings (to avoid lifetime coupling to
  transient `Session` clones returned by `get_session`).
- **Call-site fallback** (`resolve_distill_signals`): tries in-memory
  `get_session` first; on `None`, falls back to persisted `SessionMetadata`
  via `resolve_session_workspace_path` + `load_session_metadata`. Returns
  `None` only when both sources are unavailable.
- **Call-site deny**: when `should_distill_facts` returns `false` and the
  signals were `None`, a `warn!` is logged (observability for the degraded
  case). When signals were `Some` but classified non-main, a `debug!` is
  logged (frequent expected path). Neither panics nor propagates.

## §2.3 confirmation (decay in subagent turns)

Confirmed: `growth_adapter::boost_turn_topics(...)` remains inside
`append_facts_entry`. Because the gate skips `append_facts_entry` entirely for
non-main sessions, **neither** the topic boost **nor** its paired
`decay_all_weights` runs during subagent/ephemeral-child turns. This is the
expected behavior: subagent turns are not user speech and must not advance the
user's topic-weight clock. The decay is NOT extracted out of the gate;
boost and decay stay in lockstep (per the existing in-code comment at the
original `:489-494`).

---

## Verification (Round 2 — complete raw output)

All commands run with `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`
prepended, from worktree `E:\agent-project\northing\.worktrees\growth-core-0804`.
`cargo fmt` was NOT run.

### 1. `cargo check -p northhing-core --features product-full`

Exit 0. **19 warnings** (baseline 19, no new warnings). No errors.

Complete warning list (each is a pre-existing baseline warning, unchanged by
this task):

```
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:300:9
    |
300 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:66:13
   |
66 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:37:13
   |
37 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `event_system`
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:305:9
    |
305 |     let event_system = global_event_system();
    |         ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_event_system`

warning: unused variable: `tool_use_id`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_signal.rs:72:9
   |
72 |     let tool_use_id = tool_use_id.to_string();
   |        ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_tool_use_id`

warning: unused variable: `port`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |        ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:41:13
   |
41 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:386:17
    |
386 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:70:13
   |
70 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.12s
```

**Warning count: 19, no new warnings.** (18 are actionable suggestions + 1
hidden_glob_reexports note = 19 total, matching the baseline.)

### 2. `cargo test -p northhing-core --features product-full turn_persist`

Exit 0. All 11 new tests green (12 total including 1 pre-existing ephemeral
test). Complete output:

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing-core` (lib test) generated 19 warnings (19 duplicates)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.61s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 12 tests
test agentic::coordination::dialog_turn::turn_persist::tests::ephemeral_child_kind_is_rejected ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::empty_parent_string_is_treated_as_set ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::none_signals_denies_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::persisted_fallback_subagent_kind_is_rejected ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_empty_creator_without_parent_allows_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_no_parent_no_creator_allows_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_non_session_creator_without_parent_allows_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_with_both_fallback_signals_is_rejected ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_with_parent_session_id_is_rejected ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::standard_with_session_creator_marker_is_rejected ... ok
test agentic::coordination::dialog_turn::turn_persist::tests::subagent_kind_no_parent_no_creator_is_rejected ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_ephemeral_lineage::append_completed_local_command_turn_persists_without_model_context ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1159 filtered out; finished in 0.10s

     Running tests\context_profile.rs (target\debug\deps\context_profile-63ba70f3bc09b949.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-a855c177a718f97e.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\product_assembly.rs (target\debug\deps\product_assembly-71121488534f71db.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-2bd16a4e3f5bb267.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finishing in 0.00s
```

### 3. `cargo test -p northhing-core --features product-full growth_adapter`

Exit 0. **25 tests** passed, no regression. Complete output:

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing-core` (lib test) generated 19 warnings (19 duplicates)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.64s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 25 tests
test agentic::growth_adapter::tests::system_clock_returns_reasonable_timestamp ... ok
test agentic::growth_adapter::tests::fresh_db_loads_default_state ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_true_on_unpaused_db ... ok
test agentic::growth_adapter::tests::boost_turn_topics_never_mentioned_returns_baseline ... ok
test agentic::growth_adapter::tests::migration_is_idempotent_load_save_load ... ok
test agentic::growth_adapter::tests::boost_turn_topics_warn_only_no_panic_on_healthy_db ... ok
test agentic::growth_adapter::tests::boost_turn_topics_first_mention_equals_baseline_by_design ... ok
test agentic::growth_adapter::tests::boost_turn_topics_cjk_input_produces_a_row ... ok
test agentic::growth_adapter::tests::modified_state_round_trips_through_save_and_load ... ok
test agentic::growth_adapter::tests::boost_turn_topics_second_mention_raises_above_baseline ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_false_when_paused ... ok
test agentic::growth_adapter::tests::finish_distill_turn_with_facts_increments_hits_and_no_pause ... ok
test agentic::growth_adapter::tests::boost_turn_topics_co_occurrence_records_related_row_count ... ok
test agentic::growth_adapter::tests::finish_distill_turn_uses_migrated_legacy_counts ... ok
test agentic::growth_adapter::tests::finish_distill_turn_triggers_pause_at_threshold_and_persists ... ok
test agentic::growth_adapter::tests::finish_distill_turn_does_not_rewrite_legacy_keys ... ok
test agentic::growth_adapter::tests::dirty_legacy_keys_do_not_panic ... ok
test agentic::growth_adapter::tests::finish_distill_turn_continues_counting_while_paused ... ok
test agentic::growth_adapter::tests::legacy_keys_are_migrated_into_state_fields ... ok
test agentic::growth_adapter::tests::blob_takes_precedence_over_legacy_keys ... ok
test agentic::growth_adapter::tests::legacy_keys_are_preserved_after_migration_and_save ... ok
test agentic::growth_adapter::tests::boost_turn_topics_floor_never_broken_by_long_cooling ... ok
test agentic::growth_adapter::tests::boost_turn_topics_empty_and_stopword_input_still_decays ... ok
test agentic::growth_adapter::tests::boost_turn_topics_repeated_mentions_increase_monotonically ... ok
test agentic::growth_adapter::tests::boost_turn_topics_respects_five_cap ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1146 filtered out; finished in 0.30s

     Running tests\context_profile.rs (target\debug\deps\context_profile-63ba70f3bc09b949.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-a855c177a718f97e.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\product_assembly.rs (target\debug\deps\product_assembly-71121488534f71db.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-2bd16a4e3f5bb267.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finishing in 0.00s
```

### 4. `cargo test -p northhing-core --features product-full memory_db`

Exit 0. **21 tests** passed, no regression. Complete output:

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing-core` (lib test) generated 19 warnings (19 duplicates)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.79s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 21 tests
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 1150 filtered out; finished in 0.20s

     Running tests\context_profile.rs (target\debug\deps\context_profile-63ba70f3bc09b949.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-a855c177a718f97e.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\product_assembly.rs (target\debug\deps\product_assembly-71121488534f71db.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-2bd16a4e3f5bb267.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finishing in 0.00s
```

### 5. `node scripts/check-core-boundaries.mjs`

Exit 0:

```
Core boundary check passed.
```

### 6. `turn_persist.rs` line count

**799 lines** (measured via `(Get-Content $f).Count`, which matches the Read
tool's line numbering and the reviewer's method). 799 < 800.

(Round 1 incorrectly reported 708 because it used
`Measure-Object -Line`, which counts newline characters, not lines. The
reviewer correctly counted 781 for that version.)

---

## Changed files

Exactly 1 file:

- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` (+107 / -89 relative to the Round 1 commit `e62a8a3`)

No second file was needed: `SessionManager.persistence_manager`
(`session_manager.rs:118`) and `resolve_session_workspace_path`
(`session_manager_workspace_path.rs:71`) are already `pub(crate)`, accessible
from `turn_persist.rs` within the same crate.

## Commits

- `e62a8a3` — Round 1 (rejected): gate facts distillation on main dialog sessions only
- `6365cf5` — Round 2 (this fix): use SessionKind + persisted fallback for facts gate

Both on `feat/growth-core-0804`. Round 2 is a new commit on top of Round 1
(not amended), per the discipline requirement.

## Test coverage (11 new tests)

| # | Test | Finding covered | Asserts |
|---|------|-----------------|---------|
| 1 | `none_signals_denies_distillation` | C1/I1 | `None` => deny (fail-closed) |
| 2 | `standard_no_parent_no_creator_allows_distillation` | positive | main session => allow |
| 3 | `subagent_kind_no_parent_no_creator_is_rejected` | **I2** | Subagent kind, no parent/creator => deny |
| 4 | `ephemeral_child_kind_is_rejected` | I2 | EphemeralChild kind => deny |
| 5 | `standard_with_parent_session_id_is_rejected` | fallback | parent set => deny |
| 6 | `empty_parent_string_is_treated_as_set` | boundary | `Some("")` => deny |
| 7 | `standard_with_session_creator_marker_is_rejected` | fallback | `created_by` "session-*" => deny |
| 8 | `standard_with_both_fallback_signals_is_rejected` | fallback | both signals => deny |
| 9 | `standard_non_session_creator_without_parent_allows_distillation` | boundary | non-"session-" creator => allow |
| 10 | `standard_empty_creator_without_parent_allows_distillation` | boundary | empty creator => allow |
| 11 | `persisted_fallback_subagent_kind_is_rejected` | C1 parity | persisted Subagent metadata => deny (same path) |

## Concerns for the orchestrator

1. **`SessionSignals` owns its strings** (`Option<String>`, not `Option<&str>`).
   This avoids lifetime coupling to the transient `Session` clone returned by
   `get_session` (which cannot be borrowed across the function boundary). The
   cost is one small allocation per turn finalization -- negligible, and the
   predicate is now trivially unit-testable without a `SessionManager`.

2. **The persisted-fallback test (`persisted_fallback_subagent_kind_is_rejected`)
   is a pure-predicate test, not an integration test.** It verifies that a
   `SessionSignals` built from persisted metadata (Subagent kind, no parent,
   no creator -- the I2 bypass shape) is rejected through `should_distill_facts`.
   A full integration test would require constructing a `SessionManager`,
   persisting a session, evicting it, and calling `resolve_distill_signals` --
   disproportionate for a 3-line `?`-chain whose correctness is structural.
   The parity is guaranteed because `resolve_distill_signals` builds
   `SessionSignals` from the same three persisted fields the predicate checks.

3. **`resolve_session_workspace_path` may scan all workspaces** if the
   `session_workspace_index` has no entry for `session_id`. This is the
   existing behavior of that method (not introduced by this change) and only
   triggers on the rare evicted-session path. The fallback is warn-only and
   non-propagating, so a slow scan cannot break finalization.

4. **No `cargo fmt` was run** (verified: `git status` is clean after the
   commit; the diff shows only intentional edits).
