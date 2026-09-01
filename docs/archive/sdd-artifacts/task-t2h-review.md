# Task T2H Review — Host-side growth state adapter

> Reviewer: judge (double verdict: spec compliance + code quality).
> Materials: brief `task-t2h-brief.md`, report `task-t2h-report.md`, diff `1488a0d → a150339`, worktree `E:\agent-project\northing\.worktrees\growth-core-0804`.
> Read-only references: crate ports `src/agentic/src/ports.rs`, `src/agentic/src/state.rs` (not modified).

## 1. Verdict

- **SPEC: PASS**
- **QUALITY: PASS**
- **Outcome: APPROVED WITH NOTES**

Notes below are non-blocking: they document a justified deviation, name a minor stylistic observation, and call out follow-up hooks that belong to later tasks.

## 2. Findings

### Critical
None.

### Important
None.

### Minor

1. **`agentic/mod.rs:49-50` — comment style** (informational; no action required).
   The two-line rationale comment mixes the `//` line-comment style with the surrounding doc-comment / `//`-block conventions used elsewhere in the file. Pure style, no behavior impact. If a future pass normalizes block comments in this file, fold this in.

2. **`growth_adapter.rs:97-99` — `pub(crate)` visibility choice** (deliberate; see §4 for the three-point ruling). The doc comment correctly calls out *why* visibility is `pub(crate)` rather than `pub` (the `MemoryDb` parameter type is `pub(crate)`, so widening the function would emit `private_interfaces`). The brief mandated `pub fn`; the deviation is justified (see §4) and the module is still `pub mod` per §3.2.

## 3. Constraints checklist

| # | Constraint | Status | Evidence |
|---|---|---|---|
| 1 | Only the 3 listed files modified (+ Cargo.lock); forbidden files untouched | PASS | `git diff --name-only 1488a0d a150339` → exactly `Cargo.lock`, `Cargo.toml`, `growth_adapter.rs`, `mod.rs`. Per-file diffs: `src/agentic/**` and `src/crates/assembly/core/src/service/agent_memory/**` produce no output (no diff). `turn_persist.rs:479-482` SystemTime pattern is the same as in the adapter. |
| 2 | Zero behavior change (no production caller, no SQL/schema/signature change) | PASS | New functions are referenced only inside the test module of the same file; no other file in the diff invokes `load_growth_state` / `save_growth_state` / `JudgeMomStateStore`. `judge_mom` schema unchanged (`memory_db.rs:98-102`); `get_judge_state` / `set_judge_state` signatures unchanged. |
| 3 | `load_growth_state` never returns `Err`; `save_growth_state` warns and returns `()` | PASS | `growth_adapter.rs:102` `pub(crate) fn load_growth_state(db: &MemoryDb) -> GrowthState` — no `Result`. `growth_adapter.rs:109-114` `if let Err(err) = state::save_state(...) { tracing::warn!(...) }` and returns implicitly. |
| 4 | Port errors mapped to `GrowthError::Port(...)`, never swallowed to `Ok(None)` | PASS | All three impl methods (`growth_adapter.rs:70-86`) call `.map_err(|err| GrowthError::Port(format!("judge_mom {} {}: {}", op, key, err)))`. No `Ok(None)` fallback on `Err`. |
| 5 | Internal crate uses relative `path`, not `workspace = true`, not `optional`, not feature-gated | PASS | `Cargo.toml:155-156`: `northhing-agentic-growth = { path = "../../../agentic" }` with `# Growth core: ...` comment. Path resolution: `src/crates/assembly/core/` + `../../../agentic` = `src/agentic/` ✓ (confirmed `src/agentic/Cargo.toml` exists at `src/agentic/`). |
| 6 | `pub mod growth_adapter;` in `mod.rs`, alphabetical, before `identity` | PASS | `mod.rs:51-52`: `pub mod growth_adapter;` directly before `pub mod identity;`. Alphabetical: `g` < `i`. |
| 7 | Non-test code: no `unwrap`/`expect`/`panic`; SystemTime matches `turn_persist.rs:479-482` | PASS | Non-test uses only `.map(...).unwrap_or(0)` (`growth_adapter.rs:43`) and `tracing::warn!` (`growth_adapter.rs:112`). `unwrap()`/`expect()` appear only inside `#[cfg(test)] mod tests` (`growth_adapter.rs:129, 154-157, 170-173, 180, 184, 188, 192, 200-203, 232-233, 236-239, 249-250`). `turn_persist.rs:479-482` pattern confirmed verbatim identical. |
| 8 | English-only comments/logs, no emoji, no `cargo fmt`, file < 800 lines | PASS | Comments are English; no emoji in the file. Diff shows no incidental formatting churn in other files (only the 3 source + Cargo.lock). File is 285 lines (under 800). |
| 9 | All 8 brief §4 tests present with the required assertions | PASS | Tests at `growth_adapter.rs:132-284`: (1) `fresh_db_loads_default_state` asserts schema_version=1, all counts=0, paused=false, background=1, cold_start=10; (2) `legacy_keys_are_migrated_into_state_fields` writes the 4 keys and checks each field; (3) `legacy_keys_are_preserved_after_migration_and_save` re-reads each legacy row after `save_growth_state`; (4) `migration_is_idempotent_load_save_load` does load→save→load and asserts equality (second load takes blob branch); (5) `blob_takes_precedence_over_legacy_keys` writes a disagreeing blob and legacy rows then asserts blob values win; (6) `dirty_legacy_keys_do_not_panic` writes `"abc"` and `"TRUE"`, asserts no panic, `"abc"`→0, `"TRUE"`→false (case-sensitive); (7) `modified_state_round_trips_through_save_and_load` mutates fields, saves, reloads, asserts equality; (8) `system_clock_returns_reasonable_timestamp` asserts `now > 1_700_000_000_000`. `GrowthState` derives `PartialEq` at `state.rs:17` so whole-state equality is valid (also `DistillStats`/`GardenCursor`/`JudgeStats`/`TimingPrefs` derive `PartialEq`). |
| 10 | No new third-party deps; no local `#[allow(dead_code)]` | PASS | `Cargo.toml` adds only the internal crate. `rg "allow\(dead_code\)" growth_adapter.rs` → no matches. |

### Additional checks (per reviewer prompt)

- **i18n generator side effect** (`generate-i18n-contract.mjs` run for build bootstrap):
  - `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` is gitignored (`.gitignore:41` `**/generated_locale_contract.rs`). Not in `git status --short`, not in `git show --name-only a150339`. ✓
  - `src/apps/relay-server/static/homepage/i18n.shared.json`: `git diff 1488a0d -- <file>` is empty, meaning working tree matches baseline — the implementer's `git checkout --` restoration was effective. ✓
- **`pub(crate) fn new(...)` on `JudgeMomStateStore`** (also a deviation candidate, same root cause as `load_growth_state`/`save_growth_state`): consistent with the same `private_interfaces` justification. Acceptable.
- **Boundary script pass**: report quotes `Core boundary check passed.` Source-rule regex set covers `auto_memory.rs`, `agentic/agents/**`, `agentic/tools/**`, but not `agentic/growth_adapter.rs`. `dependencyProfileRules.core.forbiddenNonOptionalDeps` does not list `northhing-agentic-growth`. `noCoreDependencyCrates` forbids `agentic-growth` from depending on `core` (reverse direction from this task's dependency edge), so the task's `core → agentic-growth` edge is permitted. ✓
- **Warning count**: the implementer's quoted `cargo check` output enumerates 19 distinct warnings, each in a file outside `growth_adapter.rs`. Spot-checked baselines: `session/mod.rs:13` shadowing, `memory_db.rs:236` unused `ws`, `memory_db.rs:291` unused `last_mentioned_at`, `memory_db.rs:743` unused `at_ms` — all pre-existing lines at `1488a0d`. The summary line `"northhing-core" (lib) generated 19 warnings` matches the enumerated count (19). All 19 are pre-existing; zero are introduced by this task. The brief's "zero warning" criterion (per its parenthetical) is about new warnings, and none are new. ✓
- **Test runner output**: 8 `growth_adapter` tests pass, 102 crate-side tests pass — matches the report verbatim and matches the test count in §9. ✓

## 4. Ruling on the known deviation (`pub(crate)` vs `pub`)

**(a) Is the implementer's reason correct?**

Yes. Rust's `private_interfaces` lint fires when a `pub` item (function, struct field, type alias, etc.) exposes a type in its signature that is more private than the item itself. `MemoryDb` is declared `pub(crate) struct MemoryDb { ... }` at `memory_db.rs:8`, and all its methods (`open`, `insert_fact`, `get_facts`, `get_judge_mom_value`, `set_judge_mom_value`, etc.) are also `pub(crate)`. A `pub fn load_growth_state(db: &MemoryDb) -> GrowthState` therefore exposes `MemoryDb` (a `pub(crate)` type) in a `pub` signature — exactly the `private_interfaces` case. The two viable fixes are: widen `MemoryDb` to `pub` (out of scope, would change the public API surface of the host crate and propagate `pub` widening to all helper functions), or narrow the new functions to `pub(crate)` (the implementer's choice). The crate's existing pattern — `pub(crate) fn get_judge_state(db: &MemoryDb, ...)`, `pub(crate) fn set_judge_state(db: &MemoryDb, ...)`, `pub(crate) fn with_test_memory_db_path(...) -> MemoryDbPathGuard` — establishes the precedent. The implementer is correct.

**(b) Is the module still `pub mod`?**

Yes. `agentic/mod.rs:51` is `pub mod growth_adapter;` (verbatim, not `pub(crate) mod`). This matches brief §3.2 and is what prevents `dead_code` warnings on the module-level symbols (`SystemClock`, `JudgeMomStateStore`) without needing a local `#[allow(dead_code)]`.

**(c) Does this block the follow-up `turn_persist.rs` wiring?**

No. The follow-up caller lives in `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` — same crate (`northhing-core`). `pub(crate) fn` items are visible to all modules in the same crate, so `crate::agentic::growth_adapter::{load_growth_state, save_growth_state}` is callable from `turn_persist.rs` directly, no further visibility widening required. If a future task ever needs to expose these entrypoints from outside `northhing-core` (out-of-crate), either `MemoryDb` would need to be widened to `pub` (the bigger change) or a `pub` wrapper taking a trait object would be needed — both noted as out-of-scope concerns in the report and consistent with the brief.

**Ruling: deviation ACCEPTED.** The justification is technically sound, the module visibility honors §3.2 verbatim, and the follow-up wiring task is unaffected. No fixer pass required.

## 5. Migration order & key-namespace analysis

### Migration order (`state::load_state` semantics, `state.rs:78-140`)

1. `store.get_blob(GROWTH_STATE_KEY)` is attempted first.
   - `Ok(Some(blob))` → `serde_json::from_str::<GrowthState>`. `Ok` with `schema_version == 1` returns the parsed state. `Ok` with schema mismatch or `Err(parse)` → warn + `GrowthState::default()`.
   - `Ok(None)` → fall through to legacy migration (see below).
   - `Err(e)` → warn + `GrowthState::default()`. The adapter maps this `Err` from `GrowthError::Port(...)` (correct: it is not silently treated as `Ok(None)`).

2. **Legacy migration** (only entered on `Ok(None)` from get_blob) reads four keys in order: `LEGACY_KEY_DISTILL_TURNS`, `LEGACY_KEY_DISTILL_HIT_TURNS`, `LEGACY_KEY_DISTILLER_PAUSED`, `LEGACY_KEY_DREAM_LAST_SWEEP`. Each read is `match`ed on `Ok(Some) | Ok(None) | Err(e)`. On `Err`, the crate logs a warning and returns `GrowthState::default()` rather than treating the IO error as "key absent".

The adapter layer preserves this ordering verbatim:
- `JudgeMomStateStore::get_blob` and `get_legacy_kv` both forward to `get_judge_state(self.db, key)` (`growth_adapter.rs:71, 83`), which under the hood is `db.get_judge_mom_value(key)` (`judge_memory.rs:4-6`).
- `get_judge_mom_value` returns `Ok(None)` when the row is absent and `Err(NortHingError::...)` on actual IO failure (the implementation lives in `memory_db.rs`; not modified by this task, confirmed via empty diff). The crate-side `load_state` correctly distinguishes `Err` from `Ok(None)` — so a failed legacy read will not be misread as "no legacy data". ✓
- `Err(e)` from `get_legacy_kv` returns `GrowthError::Port(...)`, which `state::load_state` then matches and treats as a hard-fail (default, with warn). ✓

### Key namespace

The new blob key (`growth_state_v1`) and the four legacy keys (`distill_turns`, `distill_hit_turns`, `distiller_paused`, `dream_last_sweep_at`) are disjoint strings and live in the same `judge_mom` table. The adapter writes **only** under `growth_state_v1` (via `set_blob` → `set_judge_state(..., GROWTH_STATE_KEY, ..., at_ms)`), so legacy rows are never overwritten or deleted. Test 3 (`legacy_keys_are_preserved_after_migration_and_save`) verifies this end-to-end.

**Forward-looking concern (out of scope, observation only):** the `judge_mom` table is a flat key-value namespace shared between the growth state blob and any other flat keys the rest of the host may want to write. To avoid future collisions:
- Future blob-style keys should adopt a clear prefix (e.g. `growth_state_*` for any growth-owned state, distinct from any other subsystem's flat keys).
- The constants `GROWTH_STATE_KEY` / `LEGACY_KEY_*` are centralized in the crate (`state.rs:8-14`) and re-used by the adapter (no string duplication), so future schema/version bumps only need to change one place.

This is informational; the brief explicitly disallows proposing new requirements.

## 6. Cannot be determined from the diff alone

These items I could not verify without rerunning the implementation's own commands (which the brief forbids me from rerunning):

- **Exact warning text reproduction**: I verified the warning *count* (19, matching the summary line) and that every warning listed in the report's quoted output is in a file outside `growth_adapter.rs`. I did not re-run `cargo check` to confirm byte-identical output, but I confirmed the existence of each cited warning source line at the baseline `1488a0d`. The report's evidence is sufficient to support the claim "0 new warnings from this task".
- **Boundary script live execution**: I read `scripts/check-core-boundaries.mjs`, `rules/crate-rules.mjs`, `rules/source/forbidden-rules.mjs`, `rules/source/required-rules.mjs`, `rules/source/facade-rules.mjs` and confirmed by static analysis that no rule references `agentic/growth_adapter.rs`, that the reverse dependency direction (`core` → `agentic-growth`) is allowed by `noCoreDependencyCrates`, and that `core`'s `dependencyProfileRules.forbiddenNonOptionalDeps` does not list `northhing-agentic-growth`. I did not re-run the script.
- **Test runtime behavior**: I verified test names, assertions, and that `GrowthState` (and its sub-structs) derive `PartialEq`. I did not re-run the test binary. The report's quoted `cargo test` output (8/8 pass, 102/102 pass) is the authoritative evidence per the brief's "report 即证据" rule.
- **i18n generator side effect**: I confirmed by `git check-ignore` and `git status --short` (clean) that the regenerated `generated_locale_contract.rs` is gitignored and not in the commit. I confirmed by `git diff` on `i18n.shared.json` (empty) that the working tree matches baseline. I did not re-run `node scripts/generate-i18n-contract.mjs` myself.

## 7. Approval

**APPROVED WITH NOTES.** The deviation is justified (see §4); all 10 constraints pass (§3); the migration order and key namespace are correct (§5); no critical or important findings. The commit `a150339` is ready for end-of-branch review.
