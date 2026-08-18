# Task T4b Review — Host Distillation State Convergence

> Reviewer: judge-m3 (independent)
> Reviewed: `985bbb9` against `1c986a4` on `feat/growth-core-0804`
> Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`

---

## 1. Verdicts

| Aspect | Verdict |
|--------|---------|
| **SPEC** | **PASS** |
| **QUALITY** | **PASS** |
| **Overall** | **APPROVED** |
| **Behavioural equivalence** | **Confirmed** (one authorised deviation only: auto-pause warn log fires once on transition instead of every turn once paused) |

**Findings count: 0 Critical, 0 Important, 1 Minor.**

---

## 2. Findings

### Minor

- **M1 — `growth_adapter.rs:35` uses a `§` symbol in a doc comment** (`matching the existing turn_persist.rs pattern`)
  - Spec §6 requires "English-only, no emoji". `§` is a Unicode section sign (U+00A7), not technically an emoji, but it is a non-ASCII Unicode character intended for typographic section markers. The report flagged this and the doc comment uses `turn_persist.rs` rather than `§` — the offending occurrence is in the *pre-existing* `SystemClock` doc comment (line 35), not in this task's new code. The task comment "--- begin_distill_turn / finish_distill_turn tests (spec 4.1) ---" matches the code change in `growth_adapter.rs:131` and uses `4.1` not `§`. **Out of scope for this task** (pre-existing). No action required.

### Critical
_None._

### Important
_None._

---

## 3. Constraints (10-row checklist)

| # | Constraint | Status | Evidence |
|---|------------|--------|----------|
| 1 | Only 2 files changed; the explicitly forbidden files untouched | ✅ | `git diff --name-only 1c986a4..985bbb9` → `turn_persist.rs`, `growth_adapter.rs` only. grep confirms no path match against `dream`, `distiller`, `memory_db`, `facts`, `judge_memory`, `src/agentic`, `Cargo.toml`. |
| 2 | Behavioural equivalence (7 spec bullets) | ✅ | See §4 below — all 7 equivalent. |
| 3 | Only authorised observable deviation: warn log goes from per-turn to transition-once | ✅ | `scheduler.rs:98-106` returns `Some(ev)` only on `false -> true`; subsequent calls return `None`. Persisted `paused` value is identical. |
| 4 | Garden (`dream.rs`, `dream_last_sweep_at`) untouched; new code never writes `GrowthState.garden` | ✅ | `record_distill_outcome` (`scheduler.rs:88-109`) only touches `state.distill.turns`, `state.distill.hit_turns`, `state.distill.paused`. `growth_adapter.rs` imports show zero references to `state.garden` outside the test module (test line 294 sets it deliberately for a round-trip test). |
| 5 | 3 old bare keys not deleted, only no longer written | ✅ | `set_judge_state` / `get_judge_state` calls in `growth_adapter.rs` are test-only (lines 182-408). `state.rs` save path writes only the blob key (`set_blob` with `GROWTH_STATE_KEY`). Test `finish_distill_turn_does_not_rewrite_legacy_keys` (line 388) and `legacy_keys_are_preserved_after_migration_and_save` (line 196) confirm. |
| 6 | warn-only: no `?` propagation, no panic in non-test code | ✅ | `finish_distill_turn` (lines 132-142) uses `if let Some(...)` and `save_growth_state` (lines 109-114) uses `if let Err(...)` with `tracing::warn!`. `load_growth_state` returns `GrowthState` directly (never `Result`). All `unwrap`/`expect` in `growth_adapter.rs` are in `#[cfg(test)]` (lines 144+); non-test code (lines 1-142) has only `.unwrap_or(0)` (line 43) which is non-panicking. |
| 7 | `get_judge_state` / `set_judge_state` still exist; only trimmed from `turn_persist.rs` use | ✅ | `turn_persist.rs:437` `use` no longer imports them, but `grep` across `agentic/` shows `growth_adapter.rs:25` still imports them (used in `JudgeMomStateStore` impl). Functions are defined in `judge_memory.rs` re-exported by `agent_memory/mod.rs`, still used by `dream.rs`. |
| 8 | `cargo fmt` not run; English-only; no emoji; `growth_adapter.rs` < 800 lines; no schema / dep change | ✅ | Diff stat: `2 files changed, 144 insertions(+), 39 deletions(-)`. No formatter-driven whitespace changes. `growth_adapter.rs` = 417 lines. `Cargo.toml` not in diff. `GROWTH_STATE_KEY` reuses existing `judge_mom` table; no schema migration. |
| 9 | 7 spec tests, including "real round-trip", "counting continues while paused", "old keys not rewritten", "only-old-keys migration path" | ✅ | All 7 tests present in `growth_adapter.rs:316-416`; all use `unique_test_memory_db_path()` + `load_growth_state` after `save_growth_state` (real round-trip). Test 4 (`finish_distill_turn_continues_counting_while_paused`) seeds `paused=true` and verifies `turns=21` after `finish(produced_facts=false)`. Test 6 verifies legacy key stays `"7"`. Test 7 verifies only-legacy-keys path migrates to `paused=true` at `turns=20`. |
| 10 | §5 6 commands raw output complete; `cargo check` warning count = 19 (baseline) | ✅ | Report §5 contains all 6 outputs verbatim. Warning count: `warning: northhing-core (lib) generated 19 warnings` (matches baseline of 19). Zero new warnings — all 19 are pre-existing in unrelated files (`session/mod.rs`, `bash_tool`, `task_tool`, `sub_handle_*`, `memory_db.rs`, `dream.rs`). |

---

## 4. Behavioural Equivalence — 7-Semantic Table

| # | §2 Semantic (legacy file:line) | New code (file:line) | Equivalent? | Basis |
|---|-------------------------------|----------------------|-------------|-------|
| 1 | DB opened with `MemoryDb::open(&db_path)` (Result, no unwrap) — `turn_persist.rs:458` (legacy) | `turn_persist.rs:457-458` (after): `let db = MemoryDb::open(&db_path);` | ✅ | Diff `turn_persist.rs@@-455,17 +457,14` shows the line unchanged. |
| 2 | Pause gate: read `distiller_paused`, only `"true"` (case-sensitive) = paused; open/read failure → not paused | `turn_persist.rs:461-464` (`match &db` → `begin_distill_turn`); `state.rs:117` performs `paused_str == "true"` (case-sensitive, byte-identical); Err branch returns `(true, GrowthState::default())` (not paused) | ✅ | Test `begin_distill_turn_returns_false_when_paused` (line 325) & `begin_distill_turn_returns_true_on_unpaused_db` (line 317) confirm. `state.rs:113-122` failure path returns `GrowthState::default()` whose `paused=false`. |
| 3 | If paused, `candidates = Vec::new()`; else `distill_facts_with_llm(...)` | `turn_persist.rs:467-477` (`if run_distill { distill_facts_with_llm(...) } else { Vec::new() }`) | ✅ | All 4 args byte-identical: `user_input`, `last_assistant_text.as_deref()`, `session_id`, `turn_id`. `load_last_assistant_text` (lines 585-655) call site at line 444-453 unchanged. |
| 4 | `now_ms` = `SystemTime::now().duration_since(UNIX_EPOCH).map(\|d\| d.as_millis() as u64).unwrap_or(0)` | `turn_persist.rs:480-483` (after): byte-identical expression | ✅ | Diff shows zero whitespace or content change to `now_ms` block. |
| 5 | Count (only when DB Ok): `distill_turns` unconditionally +1 (incl. paused); `distill_hit_turns` +1 only when `!candidates.is_empty()`; read failure → 0; immediate write | `turn_persist.rs:485-487` → `finish_distill_turn(..., !candidates.is_empty(), now_ms)`. Inside (`growth_adapter.rs:138-141`): `record_distill_outcome` (`scheduler.rs:88-109`) does `turns = saturating_add(1)` unconditionally (line 90), `hit_turns = saturating_add(1)` only if `produced_facts` (line 93-95). `save_growth_state` writes blob. | ✅ | `saturating_add` is equivalent to legacy read->parse->+1 when DB is Ok (legacy explicitly used `+1`, never saturating). Blob write covers all read-failure -> default-0 mismatch via `state.rs:101,109` `.unwrap_or(0)`. Tests `finish_distill_turn_with_facts_increments_hits_and_no_pause` and `finish_distill_turn_continues_counting_while_paused` confirm. |
| 6 | Self-brake: `distill_turns >= 20 && distill_hit_turns == 0` → write `distiller_paused="true"` + `warn!("Distiller auto-paused: 0 hits in {} turns", distill_turns)` | `scheduler.rs:98-106`: `if !state.distill.paused && state.distill.turns >= DISTILL_AUTO_PAUSE_TURNS && state.distill.hit_turns == 0 { state.distill.paused = true; return Some(AutoPauseEvent { turns: state.distill.turns }); }`. `growth_adapter.rs:138-140`: `tracing::warn!("Distiller auto-paused: 0 hits in {} turns", ev.turns);` | ✅ (with authorised deviation) | `DISTILL_AUTO_PAUSE_TURNS = 20` (scheduler.rs:45). Warn text byte-identical. `ev.turns` = `state.distill.turns` *after* increment = legacy `distill_turns` value. Deviation: warn fires only on `false -> true` transition (brief §3.4 + §6 explicitly authorise this). Test `auto_pause_event_fires_only_once` (scheduler.rs:209) confirms. |
| 7 | Counting + brake happen before `if candidates.is_empty() { return; }` (candidates-empty turns still count) | `turn_persist.rs:486` (`finish_distill_turn`) precedes `turn_persist.rs:489-491` (early return) | ✅ | Diff preserves order. Test `finish_distill_turn_triggers_pause_at_threshold_and_persists` (line 333) uses `produced_facts=false` (candidates-empty path) and asserts `turns=20, paused=true` — counting happened before the early return. |

---

## 5. Extra-Risk Points (6-row table)

| # | Risk | Verdict | Evidence |
|---|------|---------|----------|
| 1 | `GrowthState::default()` fallback when DB fails — could `finish_distill_turn` be called on the default state and overwrite real counts? | **SAFE** | `finish_distill_turn` is only invoked inside `if let Ok(db) = &db` (line 485). The `Err(_) => (true, GrowthState::default())` branch never reaches `finish_distill_turn`, so the default state is discarded. The subsequent `candidates` is computed and used in the LLM call only. |
| 2 | Is the count+save path still executed on "candidates empty + DB Ok" (mirror legacy)? | **YES** | `turn_persist.rs:486` runs `finish_distill_turn` unconditionally when `db` is Ok, regardless of `candidates.is_empty()`. The early return at line 489 is *after* `finish_distill_turn`. This matches the legacy ordering at lines 484-508 (before `:516-518` early return). |
| 3 | New "pause -> resume" path accidentally introduced? | **NO** | `record_distill_outcome` (`scheduler.rs:88-109`) only sets `paused = true`; never sets it `false`. `should_distill` only reads `state.distill.paused`. No new code path can unpause. |
| 4 | `hit_turns` decision uses the same `candidates` boolean at the same timepoint | **YES** | `turn_persist.rs:486` evaluates `!candidates.is_empty()` *once* on the same `candidates` Vec that was just produced at line 467-477. The Vec is not moved/consumed between line 477 and line 486 (only borrowed immutably via `is_empty()`). `record_distill_outcome` (called with `produced_facts = !candidates.is_empty()`) matches legacy `:491` semantics. |
| 5 | Migration one-time: blob existence takes precedence over legacy keys | **YES** | `state.rs:78-95` (`load_state`): if `store.get_blob(GROWTH_STATE_KEY)` returns `Ok(Some(blob))`, the function returns immediately at line 84 (schema-validated state). Legacy key migration (lines 96-131) only runs on `Ok(None)`. Once any `save_growth_state` writes the blob, subsequent `load_growth_state` calls never read legacy keys. Test `blob_takes_precedence_over_legacy_keys` (line 244) and `migration_is_idempotent_load_save_load` (line 226) confirm. |
| 6 | Test 3 actually performs a real round-trip, not in-memory mutation | **YES** | `finish_distill_turn_triggers_pause_at_threshold_and_persists` (line 333): seeds via `save_growth_state`, then `begin_distill_turn` (loads fresh), then `finish_distill_turn`, then **separately** calls `load_growth_state(&db)` (line 346) — this is a fresh DB read, not the in-memory `state` variable. The `reloaded` binding is asserted against. Same pattern in tests 4-7. |

---

## 6. Could Not Verify from Diff

- **End-to-end dialog turn integration**: The unit tests prove `begin_distill_turn` / `finish_distill_turn` semantics in isolation, but the spec conversation-turn path runs in a Tokio runtime with `PersistenceManager`, `MemoryDb::open`, and `append_facts_dedup`. I cannot, from a static diff, observe a real multi-turn session hitting `append_facts_entry` and verify the legacy vs. new code produce identical observable side-effects (DB rows, log output, file state). The unit tests + the `cargo check` (19 warnings = baseline) + `cargo test` of dependent crates (`auto_memory`, `memory_db`, `agentic-growth`) provide coverage but not full end-to-end. This is inherent to the task's testing scope defined in the brief, not a deficiency.
- **Idempotency under concurrent invocations**: `finish_distill_turn` reads/writes the blob via `set_judge_state` on the shared `MemoryDb`. No new locking was added vs. legacy. I cannot rule out race conditions from diff alone, but this matches legacy behaviour (no regression).
- **Real run-time of `record_distill_outcome` for the ≥ 20 turn slow path**: The test `auto_pause_event_fires_only_once` in `scheduler.rs` exists pre-this-task and is unchanged. The new test `finish_distill_turn_continues_counting_while_paused` verifies the paused-state count increment but does not assert warn-log frequency (would require log capture). The authorised deviation is implemented at the crate boundary, not in the new code, so this is inherited from the prior task — not a regression introduced here.

---

## 7. Summary

This is a tight, surgical refactor. The two-file constraint is strictly observed. The 7 behavioural equivalences are verified by test + source line-by-line comparison. The 6 extra-risk points are all confirmed safe. The only finding is a Minor non-actionable pre-existing Unicode `§` in `growth_adapter.rs:35`.

The new code passes the spec (every constraint in §3.1, §3.2, §4.1, §6 of the brief) and meets the quality bar (warn-only, no panic in non-test code, comprehensive tests with real DB round-trips, well-documented, hand-aligned 4-space formatting with no fmt-induced churn).

**Approved.**
