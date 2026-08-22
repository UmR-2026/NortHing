# Task T4b Report - Host Distillation State Convergence (3 bare keys -> growth state, behaviour-equivalent)

> Status: **DONE**

## 1. Summary

Routed distillation scheduling in `turn_persist.rs::append_facts_entry` through the growth state
adapter (`begin_distill_turn` / `finish_distill_turn`) instead of the 3 legacy bare KV keys
(`distiller_paused` / `distill_turns` / `distill_hit_turns`). The crate-side pure decision functions
(`should_distill` / `record_distill_outcome`) and the host adapter (`load_growth_state` /
`save_growth_state`) from the prior two tasks were already complete and reviewed; this task only
adds 2 thin wrapper functions and rewires the call site.

**Files changed (exactly 2, per spec §3):**

| File | Change |
|------|--------|
| `src/crates/assembly/core/src/agentic/growth_adapter.rs` | +2 functions (`begin_distill_turn`, `finish_distill_turn`), +7 tests |
| `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` | Replaced `:458-514` region; trimmed `use` imports |

**Commit:** `985bbb9 refactor(growth): route distillation scheduling through growth state`
**Base:** `1c986a4`
**Branch:** `feat/growth-core-0804`

---

## 2. Equivalence Table (spec §7 requirement)

Each row maps a §2 current-behaviour semantic to the post-change code that fulfills it, and how
equivalence was confirmed.

| # | §2 Current semantic | Post-change code (new line refs in `turn_persist.rs`) | How confirmed equivalent |
|---|---------------------|------------------------------------------------------|--------------------------|
| 1 | `:455-456` DB opened with `MemoryDb::open(&db_path)` (Result, no unwrap) | `turn_persist.rs:457-458` — unchanged: `let db = MemoryDb::open(&db_path);` | Diff shows no change to DB open line |
| 2 | `:459-463` Pause gate: read `distiller_paused`, only `"true"` means paused; DB open/read failure -> not paused | `turn_persist.rs:460-464` — `match &db { Ok(db) => begin_distill_turn(db), Err(_) => (true, GrowthState::default()) }`. `begin_distill_turn` calls `load_growth_state` (which reads migrated `distiller_paused` via `state::load_state`, case-sensitive `"true"` check) then `should_distill` returns `!paused`. On `Err(_)` DB, returns `(true, ...)` = not paused. | `state.rs:117` does `paused_str == "true"` (case-sensitive, identical). Test `begin_distill_turn_returns_false_when_paused` confirms paused -> false; `begin_distill_turn_returns_true_on_unpaused_db` confirms fresh DB -> true. `load_growth_state` is warn-only and returns defaults on any port error (`state.rs:135-138`), so read failure -> default state -> `paused=false` -> not paused, matching legacy. |
| 3 | `:466-476` If paused, `candidates = Vec::new()` (skip LLM); else call `distill_facts_with_llm` | `turn_persist.rs:466-477` — `if run_distill { distill_facts_with_llm(...) } else { Vec::new() }`. Same 4 args, same `last_assistant_text.as_deref()` source. | Diff: only variable name changed (`distiller_paused` -> `run_distill`), branch arms swapped order but identical logic. The `distill_facts_with_llm` call signature and args are byte-identical. |
| 4 | `:479-482` `now_ms` = `SystemTime::now().duration_since(UNIX_EPOCH).map(\|d\| d.as_millis() as u64).unwrap_or(0)` | `turn_persist.rs:480-483` — byte-identical expression | Diff shows zero change to `now_ms` computation |
| 5 | `:484-508` Counting (only when DB Ok): `distill_turns` unconditionally +1 (incl. paused turns); `distill_hit_turns` +1 only when `!candidates.is_empty()`; read failure -> 0; both written back immediately | `turn_persist.rs:485-487` — `if let Ok(db) = &db { finish_distill_turn(db, &mut growth_state, !candidates.is_empty(), now_ms); }`. Inside `finish_distill_turn` (`growth_adapter.rs:132-142`): calls `record_distill_outcome(state, produced_facts)` which does `turns = saturating_add(1)` unconditionally, `hit_turns = saturating_add(1)` only if `produced_facts`, then `save_growth_state`. | `scheduler.rs:88-109`: turns always +1, hit_turns +1 only on `produced_facts`. Test `finish_distill_turn_triggers_pause_at_threshold_and_persists` verifies `turns` 19->20 with `produced_facts=false`; test `finish_distill_turn_with_facts_increments_hits_and_no_pause` verifies hit_turns +1 with `produced_facts=true`. DB-unavailable path skips `finish_distill_turn` entirely (no counting), matching legacy. `load_growth_state` defaults unreadable counts to 0 (`state.rs:101,109` use `.unwrap_or(0)`), matching legacy `.unwrap_or(0)`. |
| 6 | `:510-513` Self-brake: `distill_turns >= 20 && distill_hit_turns == 0` -> write `distiller_paused="true"` + `warn!("Distiller auto-paused: 0 hits in {} turns", distill_turns)` | `growth_adapter.rs:138-140` — `if let Some(ev) = record_distill_outcome(...) { tracing::warn!("Distiller auto-paused: 0 hits in {} turns", ev.turns); }`. Inside `record_distill_outcome` (`scheduler.rs:98-106`): when `!paused && turns >= 20 && hit_turns == 0`, sets `paused = true` and returns `Some(AutoPauseEvent { turns })`. | Warn text is byte-identical: `"Distiller auto-paused: 0 hits in {} turns"`, filled with the same value (post-increment `turns`). `ev.turns` = `state.distill.turns` after increment, identical to legacy `distill_turns` (already incremented). Threshold check `>= 20` is `DISTILL_AUTO_PAUSE_TURNS` (`scheduler.rs:45,99`). Test `finish_distill_turn_triggers_pause_at_threshold_and_persists` confirms pause at turns=20. |
| 7 | **Ordering critical**: counting and brake happen BEFORE `:516-518` `if candidates.is_empty() { return; }` — turns with no candidates still count | `turn_persist.rs:485-491` — `finish_distill_turn` is called at line 486, `if candidates.is_empty() { return; }` is at line 489. The counting happens before the early return. | Diff preserves the ordering: `finish_distill_turn` call -> `if candidates.is_empty() { return; }`. Test `finish_distill_turn_triggers_pause_at_threshold_and_persists` uses `produced_facts=false` (empty candidates path) and verifies turns still incremented to 20. |

### Authorized behavioural deviation (spec §3.4, §6)

The legacy code re-emits the auto-pause `warn!` on every subsequent turn while paused (because
`turns >= 20 && hit_turns == 0` stays perpetually true). The new `record_distill_outcome` returns
`Some(AutoPauseEvent)` **only on the `false -> true` transition**; subsequent calls while already
paused return `None`, so the warn fires exactly once. The persisted `paused` value is identical.
This is the sole authorized observable difference (log noise reduction). Confirmed by scheduler
test `auto_pause_event_fires_only_once` and `paused_state_still_increments_turns`.

---

## 3. Before / After Code (replaced region)

### 3.1 BEFORE — `turn_persist.rs` `:458-514` (original `append_facts_entry` region)

```rust
        // Open DB early for judge state operations.
        let db_path = default_memory_db_path();
        let db = MemoryDb::open(&db_path);

        // Pause gate: check before distillation.
        let distiller_paused = db
            .as_ref()
            .ok()
            .and_then(|db| get_judge_state(db, "distiller_paused").ok().flatten())
            .as_deref() == Some("true");

        // Distill candidate facts from user input using LLM (with keyword fallback).
        let candidates = if distiller_paused {
            Vec::new()
        } else {
            distill_facts_with_llm(
                user_input,
                last_assistant_text.as_deref(),
                session_id,
                turn_id,
            )
            .await
        };

        // Hit-rate counting and self-learning brake (before early return).
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if let Ok(db) = &db {
            let distill_turns = get_judge_state(db, "distill_turns")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
                + 1;
            let distill_hit_turns = if !candidates.is_empty() {
                get_judge_state(db, "distill_hit_turns")
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
                    + 1
            } else {
                get_judge_state(db, "distill_hit_turns")
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
            };

            let _ = set_judge_state(db, "distill_turns", &distill_turns.to_string(), now_ms);
            let _ = set_judge_state(db, "distill_hit_turns", &distill_hit_turns.to_string(), now_ms);

            // Self-learning brake: auto-pause if 0 hits in >= 20 turns.
            if distill_turns >= 20 && distill_hit_turns == 0 {
                let _ = set_judge_state(db, "distiller_paused", "true", now_ms);
                warn!("Distiller auto-paused: 0 hits in {} turns", distill_turns);
            }
        }
```

### 3.2 AFTER — `turn_persist.rs` `:457-491` (same region, new code)

```rust
        // Open DB early for judge state operations.
        let db_path = default_memory_db_path();
        let db = MemoryDb::open(&db_path);

        // Growth state: single source of truth for distillation scheduling.
        let (run_distill, mut growth_state) = match &db {
            Ok(db) => growth_adapter::begin_distill_turn(db),
            Err(_) => (true, GrowthState::default()),
        };

        // Distill candidate facts from user input using LLM (with keyword fallback).
        let candidates = if run_distill {
            distill_facts_with_llm(
                user_input,
                last_assistant_text.as_deref(),
                session_id,
                turn_id,
            )
            .await
        } else {
            Vec::new()
        };

        // Hit-rate counting and self-learning brake (before early return).
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if let Ok(db) = &db {
            growth_adapter::finish_distill_turn(db, &mut growth_state, !candidates.is_empty(), now_ms);
        }
```

### 3.3 Import change — `turn_persist.rs` `:437` (before) -> `:437-440` (after)

**Before:**
```rust
        use crate::service::agent_memory::{append_facts_dedup, distill_facts_with_llm, get_judge_state, set_judge_state};
```

**After:**
```rust
        use crate::service::agent_memory::{append_facts_dedup, distill_facts_with_llm};
        use crate::service::agent_memory::FactReview;
        use crate::agentic::growth_adapter;
        use northhing_agentic_growth::state::GrowthState;
```

### 3.4 New functions — `growth_adapter.rs` `:116-142`

```rust
/// Distillation gate for one dialog turn.
///
/// Returns `(run_distill, state)`. On any storage failure the caller must fall
/// back to running distillation with no counting, mirroring the pre-migration
/// behaviour where an unreadable pause flag meant "not paused".
pub(crate) fn begin_distill_turn(db: &MemoryDb) -> (bool, GrowthState) {
    let state = load_growth_state(db);
    let run_distill = northhing_agentic_growth::scheduler::should_distill(&state);
    (run_distill, state)
}

/// Records the outcome of one distillation attempt and persists growth state.
///
/// Must be called on every finalized turn, including turns that produced no
/// candidates and turns skipped because distillation is paused. Logs the
/// auto-pause warning exactly once, on the transition into the paused state.
pub(crate) fn finish_distill_turn(
    db: &MemoryDb,
    state: &mut GrowthState,
    produced_facts: bool,
    now_ms: u64,
) {
    if let Some(ev) = northhing_agentic_growth::scheduler::record_distill_outcome(state, produced_facts) {
        tracing::warn!("Distiller auto-paused: 0 hits in {} turns", ev.turns);
    }
    save_growth_state(db, state, now_ms);
}
```

---

## 4. Test Results (spec §4)

### 4.1 New tests added to `growth_adapter.rs` (7 tests, per spec §4.1)

| # | Test name | What it verifies | Result |
|---|-----------|------------------|--------|
| 1 | `begin_distill_turn_returns_true_on_unpaused_db` | Fresh (unpaused) DB -> `(true, state)` | ok |
| 2 | `begin_distill_turn_returns_false_when_paused` | State already `paused=true` -> `(false, _)` | ok |
| 3 | `finish_distill_turn_triggers_pause_at_threshold_and_persists` | From `turns=19, hit_turns=0`: one `finish(false)` -> reloaded `turns=20 && paused==true` (real DB round-trip) | ok |
| 4 | `finish_distill_turn_continues_counting_while_paused` | Same as #3 but seed already paused at `turns=20` -> reloaded `turns=21`, still `paused==true` | ok |
| 5 | `finish_distill_turn_with_facts_increments_hits_and_no_pause` | `produced_facts=true` -> `hit_turns` +1, no pause | ok |
| 6 | `finish_distill_turn_does_not_rewrite_legacy_keys` | Write legacy `distill_turns="7"`, run begin+finish, assert legacy key still `"7"` | ok |
| 7 | `finish_distill_turn_uses_migrated_legacy_counts` | Only legacy keys (`distill_turns="19"`, `distill_hit_turns="0"`) -> one `finish(false)` -> `paused==true` at `turns=20` | ok |

All 7 tests perform real DB read/write round-trips through `load_growth_state` / `save_growth_state`
(no in-memory-only state mutation), using `unique_test_memory_db_path()` for isolation.

---

## 5. Verification Command Output (spec §5 — 6 commands, raw output)

### Command 1: `cargo check -p northhing-core --features product-full`

```
    Checking northhing-core v0.2.10 (E:\agent-project\northing\.worktrees\growth-core-0804\src\crates\assembly\core)
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
    |         ----^^^^^^^^^^^^^^^
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
    |         ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_event_system`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `tool_use_id`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_signal.rs:72:9
   |
72 |     let tool_use_id = tool_use_id.to_string();
   |         ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_tool_use_id`

warning: unused variable: `port`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
   |
137 |         let port = params
   |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |            ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:41:13
   |
41 |         let turn_index = ctx.turn_index;
   |         ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:386:17
   |
386 |             let workspace_turn_status = tokio::select! {
   |                 ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:70:13
   |
70 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 56s
```

**Warning count: 19 (identical to baseline of 19). No new warnings introduced.** All 19 warnings
are pre-existing (unused variables, shadowed glob re-exports in unrelated files: `session/mod.rs`,
`bash_tool`, `task_tool`, `sub_handle_*`, `memory_db.rs`, `memory_db/dream.rs`).

---

### Command 2: `cargo test -p northhing-core --features product-full growth_adapter`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 28s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 15 tests
test agentic::growth_adapter::tests::system_clock_returns_reasonable_timestamp ... ok
test agentic::growth_adapter::tests::dirty_legacy_keys_do_not_panic ... ok
test agentic::growth_adapter::tests::modified_state_round_trips_through_save_and_load ... ok
test agentic::growth_adapter::tests::legacy_keys_are_migrated_into_state_fields ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_true_on_unpaused_db ... ok
test agentic::growth_adapter::tests::fresh_db_loads_default_state ... ok
test agentic::growth_adapter::tests::finish_distill_turn_with_facts_increments_hits_and_no_pause ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_false_when_paused ... ok
test agentic::growth_adapter::tests::finish_distill_turn_does_not_rewrite_legacy_keys ... ok
test agentic::growth_adapter::tests::finish_distill_turn_continues_counting_while_paused ... ok
test agentic::growth_adapter::tests::blob_takes_precedence_over_legacy_keys ... ok
test agentic::growth_adapter::tests::finish_distill_turn_triggers_pause_at_threshold_and_persists ... ok
test agentic::growth_adapter::tests::finish_distill_turn_uses_migrated_legacy_counts ... ok
test agentic::growth_adapter::tests::migration_is_idempotent_load_save_load ... ok
test agentic::growth_adapter::tests::legacy_keys_are_preserved_after_migration_and_save ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1135 filtered out; finished in 0.16s

     Running tests\context_profile.rs (target\debug\deps\context_profile-63ba70f3bc09b949.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

     Running tests\git_contracts.rs (target\debug\deps\git_contracts-a855c177a718f974.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\product_assembly.rs (target\debug\deps\product_assembly-71121488534f71db.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-2bd16c4e3f5bb267.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
```

**Result: 15 passed, 0 failed.** (8 pre-existing + 7 new tests)

---

### Command 3: `cargo test -p northhing-core --features product-full auto_memory`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.98s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 7 tests
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_for_empty_query ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_when_no_match ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_some_with_matching_fact ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1143 filtered out; finished in 0.10s
```

**Result: 7 passed, 0 failed.**

---

### Command 4: `cargo test -p northhing-core --features product-full memory_db`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.17s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-d1d6b2e8018e4148.exe)

running 21 tests
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 1129 filtered out; finished in 0.21s
```

**Result: 21 passed, 0 failed.**

---

### Command 5: `cargo test -p northhing-agentic-growth`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.51s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 121 tests
test error::tests::error_display_includes_context ... ok
test negation::tests::case_insensitive_english ... ok
test negation::tests::chinese_fact_is_wrong ... ok
test negation::tests::chinese_preference_replaced ... ok
test negation::tests::chinese_stop_remembering ... ok
test negation::tests::english_fact_is_wrong ... ok
test negation::tests::no_hit_false_friend_ji ... ok
test negation::tests::english_preference_replaced ... ok
test negation::tests::no_hit_empty_or_whitespace ... ok
test negation::tests::no_hit_not_great ... ok
[... 111 more tests, all ok ...]
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

test result: ok. 121 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Result: 121 passed, 0 failed.** (Crate not modified by this task; confirms no breakage.)

---

### Command 6: `node scripts/check-core-boundaries.mjs`

```
Core boundary check passed.
```

**Result: passed.**

---

## 6. Legacy Keys Confirmation (spec §7 requirement)

The 3 legacy bare keys (`distiller_paused`, `distill_turns`, `distill_hit_turns`) are **no longer
written** by the new code path. They are **not deleted** from the database — the new code only
writes the `growth_state_v1` blob via `save_growth_state`.

**Test evidence** (test `finish_distill_turn_does_not_rewrite_legacy_keys`):
- Pre-conditions: `set_judge_state(&db, LEGACY_KEY_DISTILL_TURNS, "7", 1)` writes `"7"` to the
  legacy `distill_turns` key.
- Action: `begin_distill_turn` + `finish_distill_turn` (full round-trip through growth state blob).
- Assertion: `get_judge_state(&db, LEGACY_KEY_DISTILL_TURNS)` still returns `Some("7")`.
- **The legacy key value is untouched.** The new code wrote only the `growth_state_v1` blob.

Additionally, the pre-existing test `legacy_keys_are_preserved_after_migration_and_save` (test #3
from the prior task, still passing) confirms all 4 legacy keys
(`distill_turns`, `distill_hit_turns`, `distiller_paused`, `dream_last_sweep_at`) retain their
original values after a `load` + `save` cycle.

---

## 7. Git State

```
$ git log --oneline -1
985bbb9 refactor(growth): route distillation scheduling through growth state

$ git status --short
(clean — no uncommitted changes)
```

**Diff stat** (`1c986a4..985bbb9`):
```
 .../coordination/dialog_turn/turn_persist.rs       |  51 ++------
 .../assembly/core/src/agentic/growth_adapter.rs    | 132 +++++++++++++++++++++
 2 files changed, 144 insertions(+), 39 deletions(-)
```

---

## 8. Hard Constraints Compliance

| Constraint | Status |
|------------|--------|
| Only 2 files changed | Confirmed — `growth_adapter.rs` + `turn_persist.rs` only |
| `dream.rs`, `distiller.rs`, `memory_db.rs`, `facts.rs`, `judge_memory.rs`, `src/agentic/**`, `Cargo.toml` untouched | Confirmed — diff shows only 2 files |
| `get_judge_state` / `set_judge_state` functions NOT deleted (dream.rs still uses them) | Confirmed — only removed from `turn_persist.rs` use-import; functions exist in `judge_memory.rs` and re-exported from `agent_memory/mod.rs` |
| No `cargo fmt` run | Confirmed — hand-aligned 4-space |
| No `unwrap()` / `expect()` / `panic!()` in non-test code | Confirmed — `begin_distill_turn` and `finish_distill_turn` use no panicking APIs; `saturating_add` in crate avoids overflow panics |
| English-only, no emoji | Confirmed — all comments and logs in English; the `§` symbol in a test comment was replaced with plain `4.1` to avoid encoding ambiguity |
| No new dependencies | Confirmed — no Cargo.toml changes |
| No schema changes, no new tables/columns | Confirmed — uses existing `judge_mom` table via existing `set_blob`/`get_blob` |
| `growth_adapter.rs` < 800 lines | Confirmed — 417 lines total |
| Growth path warn-only (no `?`, no error propagation) | Confirmed — `load_growth_state` returns `GrowthState` (never Err); `save_growth_state` logs warn on failure; `finish_distill_turn` calls both without `?` |
| `finish_distill_turn` called before `if candidates.is_empty() { return; }` | Confirmed — `turn_persist.rs:486` (finish) precedes `:489` (early return) |

---

## 9. Deviations from Brief

**None.** All spec requirements met:
- §3.1: `begin_distill_turn` and `finish_distill_turn` implemented exactly per spec signatures and
  requirements.
- §3.2: `turn_persist.rs` replaced region follows the spec sketch; `now_ms` computation is
  byte-identical; `distill_facts_with_llm` args unchanged; `last_assistant_text` acquisition
  unchanged; `finish_distill_turn` called before early return; DB-unavailable path returns
  `(true, GrowthState::default())` (run distill, no counting).
- §4.1: All 7 tests added and passing.
- §5: All 6 commands executed with raw output captured above.
- §6: All hard constraints satisfied.

The authorized behavioural deviation (auto-pause warn from per-turn to transition-only) is
implemented in the crate-side `record_distill_outcome` (prior task, already reviewed) and surfaced
through `finish_distill_turn`. The warn log text is byte-identical to the legacy
`"Distiller auto-paused: 0 hits in {} turns"`.
