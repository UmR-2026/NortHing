# R4 Spec: session_evidence extended

> Spec-only — do not modify source files. Output of plan_c5a5067a task
> `spec-session-evidence-real-wire`. Reads from prior duplicate-scanner analysis
> at `~/.mavis/plans/plan_8b640472/outputs/duplicate-scanner/session_impl_analysis.md`
> and visibility-audit at `~/.mavis/plans/plan_8b640472/outputs/visibility-auditor/deliverable.md`.

## 0. Context

After R3 orphan-fix removes 6 evidence-group duplicates and wires `mod.rs`,
`session_evidence.rs` is canonical for 19 of the 25 evidence `impl SessionManager`
methods it currently contains (the other 6 were never duplicated in
`session_manager.rs`). This R4 spec identifies:

- **19 evidence duplicates** still living in `session_manager.rs` (138 fns / 6532 lines)
 that need deletion so `session_evidence.rs` becomes truly canonical.
- **9 NEW evidence-domain methods** not yet present in `session_evidence.rs` that
 should move there for aggressive split (per user instruction "尽可能分块到单个文件最简，
 可以增大文件量" — 2026-06-27).
- **7 evidence-domain tests** co-located in `session_manager.rs::tests` (after
 line 4363) that move alongside the methods they exercise, to hit the ~1500 row
 target for `session_evidence.rs`.

Evidence-domain rule (per task brief): methods that operate on the **evidence ledger**,
**skill agent snapshot store**, **listing baseline rebuild cutoff**, **model
reconciliation listener**, or **turn-context snapshot persistence for evidence
purposes** (i.e., for snapshots consumed by evidence restore / rollback / cutoff
logic). Anything that reads or writes `self.evidence_ledger`,
`self.turn_skill_agent_snapshot_store`, or
`self.skill_agent_baseline_override_snapshot_store`, plus the model-reconciliation
listener spawned from `new()`.

Excluded domains (already split or planned elsewhere):

- **Restore / load-to-memory** → `session_restore.rs` (canonical, 16 fns)
- **Persistence (auto-save, cleanup, prompt cache rw, metadata rw, delete)** →
 `session_persistence.rs` (canonical, 28 fns)
- **Title generation AI calls** → stays in `session_manager.rs` (pure/AI helpers)
- **Pure predicates + util fns** (`paginate_messages`, `normalize_session_title_input`,
 `fallback_session_title`, `normalize_whitespace`, `truncate_chars`,
 `session_workspace_from_config`, `should_persist_session_kind`,
 `should_persist_session`, `should_persist_session_id`,
 `is_session_expired`, `collect_expired_session_candidates`,
 `cleanup_candidate_matches_session`, `cleanup_snapshot_for_candidate`) → stay
 in `session_manager.rs` (per persistence-extend spec).

## 1. Evidence fns identified (beyond the 6 duplicates from orphan-fix)

Locations are line numbers in current `session_manager.rs` (HEAD `9dbcb9c`,
6532 lines, CRLF). `Body` = approximate body line span (excluding signature +
closing brace). `Move target` = where the method should live after R4.

### 1.1 Evidence duplicates to DELETE from session_manager.rs (already in session_evidence.rs)

These 19 fns are signature-identical to fns already in `session_evidence.rs`
(line ranges in parens). R3 orphan-fix's 6-fns table only covers `append_evidence_event`,
`invalidate_ai_clients_for_models`, `rebuild_skill_agent_listing_baseline_to_latest`,
`remove_listing_diff_internal_reminders`, `strip_listing_diff_internal_reminders`,
`spawn_model_reconciliation_listener`. R4 picks up the remaining 19 so
`session_evidence.rs` is truly canonical.

| # | Fn | session_manager.rs location | session_evidence.rs canonical location | Body size (mgr) | Visibility diff |
|---|---|---|---|---|---|
| 1 | `record_checkpoint_created` | L839–851 | L65–76 (pub(crate)) | 13 | mgr=pub(crate), ev=pub(crate) — same |
| 2 | `evidence_events_for_turn` | L852–858 | L78–84 (pub(crate)) | 7 | same |
| 3 | `evidence_summary_for_session` | L860–866 | L86–92 (pub(crate)) | 7 | same |
| 4 | `compression_contract_for_session` | L868–876 | L94–102 (pub(crate)) | 9 | same |
| 5 | `record_subagent_partial_timeout` | L878–903 | L104–129 (pub(crate)) | 26 | same |
| 6 | `is_session_model_id_usable` | L912–926 | L137–151 (pub(crate)) | 15 | mgr=`fn`, ev=pub(crate) — promote |
| 7 | `migrate_sessions_off_invalidated_models` | L932–986 | L157–211 (pub(crate) async) | 55 | mgr=`async fn`, ev=pub(crate) async — promote |
| 8 | `turn_skill_agent_snapshot` | L1288–1342 | L226–271 (pub(crate) async) | 55 | same |
| 9 | `latest_turn_skill_agent_snapshot_at_or_before` | L1335–1390 | L273–323 (pub(crate) async) | 56 | same |
| 10 | `remember_turn_skill_agent_snapshot` | L1387–1422 | L325–359 (pub(crate) async) | 36 | same |
| 11 | `recover_first_turn_skill_agent_snapshot` | L1423–1471 | L361–408 (pub(crate) async) | 49 | same |
| 12 | `remember_skill_agent_baseline_override_snapshot` | L1472–1505 | L410–442 (pub(crate) async) | 34 | same |
| 13 | `skill_agent_baseline_override_snapshot` | L1506–1544 | L444–481 (pub(crate) async) | 39 | same |
| 14 | `seed_forked_skill_agent_listing_baselines` | L1545–1575 | L482–511 (pub(crate) async) | 31 | same |
| 15 | `listing_baseline_rebuild_turn_index_from_custom_metadata` | L1652–1660 | L588–596 (pub(crate)) | 9 | mgr=`fn`, ev=pub(crate) — promote |
| 16 | `listing_baseline_rebuild_turn_index_from_metadata` | L1662–1668 | L598–604 (pub(crate)) | 7 | mgr=`fn`, ev=pub(crate) — promote |
| 17 | `persist_listing_baseline_rebuild_turn_index_best_effort` | L1733–1752 | L630–649 (pub(crate) async) | 20 | mgr=`async fn`, ev=pub(crate) async — promote |
| 18 | `truncate_listing_baseline_rebuild_turn_index_after_rollback` | L1754–1784 | L651–681 (pub(crate) async) | 31 | same |
| 19 | `persist_context_snapshot_messages_best_effort` | L1670–1692 | L606–628 (pub(crate) async) | 23 | mgr=`async fn`, ev=pub(crate) async — promote |

> **Visibility promotion note**: 6 of the 19 duplicates (#6, #7, #15, #16, #17, #19)
> have different visibility in `session_manager.rs` (`fn` / `async fn` = private)
> vs `session_evidence.rs` (`pub(crate)` / `pub(crate) async`). After R3 orphan-fix
> deletes the manager copy, the canonical `pub(crate)` version in
> `session_evidence.rs` survives. No external caller needs to change because
> the methods were never callable from outside the crate to begin with
> (private → private).

### 1.2 NEW evidence-domain methods to MOVE from session_manager.rs to session_evidence.rs

These 9 fns are NOT duplicates of anything in `session_evidence.rs`. They are
evidence-domain by their dependencies on `self.evidence_ledger`,
`self.turn_skill_agent_snapshot_store`,
`self.skill_agent_baseline_override_snapshot_store`, or
`get_global_ai_client_factory` (model reconciliation).

| # | Fn | session_manager.rs location | Body size | Dependencies | Move target |
|---|---|---|---|---|---|
| 20 | `load_ai_config_for_model_resolution` | L144–148 | 5 | `get_global_config_service`, `AIConfig` | `session_evidence.rs` (reconciliation helper cluster) |
| 21 | `is_auto_model_selector` | L150–153 | 4 | none (pure predicate) | `session_evidence.rs` |
| 22 | `context_window_for_model_selection` | L155–171 | 17 | `AIConfig::default_models`, `model_id` parsing | `session_evidence.rs` |
| 23 | `session_context_window_from_ai_config` | L173–199 | 27 | `AIConfig`, `context_window_for_model_selection` | `session_evidence.rs` |
| 24 | `sync_session_context_window_from_ai_config` | L201–208 | 8 | `self.sessions.get_mut`, `session_context_window_from_ai_config` | `session_evidence.rs` |
| 25 | `persist_context_snapshot_for_turn_best_effort` | L635–664 | 30 | `self.persistence_manager.save_turn_context_snapshot`, `effective_session_workspace_path` | `session_evidence.rs` (snapshot persistence for evidence) |
| 26 | `persist_current_turn_context_snapshot_best_effort` | L666–685 | 20 | `persist_context_snapshot_for_turn_best_effort` (#25) | `session_evidence.rs` |
| 27 | `load_turn_skill_agent_snapshot_from_persistence` | L721–730 | 10 | `self.persistence_manager.load_turn_skill_agent_snapshot` | `session_evidence.rs` (skill agent snapshot loader — reclaims from session_persistence.rs §E2) |
| 28 | `sanitize_listing_diff_context_snapshot_if_needed` | L1694–1731 | 38 | `Self::strip_listing_diff_internal_reminders`, `persist_context_snapshot_messages_best_effort` (#19) | `session_evidence.rs` (listing baseline rebuild sanitizer — reclaims from session_persistence.rs §E1) |

### 1.3 Evidence-domain tests to MOVE from session_manager.rs::tests to session_evidence.rs::tests

These 7 tests exercise the moved methods. They live in `session_manager.rs` line
ranges listed below (test region starts at L4363). Per the Round 3b plan
principle (tests co-located with tested methods), they move with their methods.

| # | Test fn | session_manager.rs location | Body size | Exercises (method #) |
|---|---|---|---|---|
| T1 | `latest_skill_agent_snapshot_scans_persistence_beyond_stale_cache_hit` | L5440–5526 | 87 | #9 `latest_turn_skill_agent_snapshot_at_or_before` |
| T2 | `rebuild_skill_agent_listing_baseline_to_latest_removes_listing_diff_reminders` | L5532–5640 | 109 | #14 `seed_forked_skill_agent_listing_baselines`, `rebuild_skill_agent_listing_baseline_to_latest` (orphan-fix #3), `remove_listing_diff_internal_reminders` (orphan-fix #4), `strip_listing_diff_internal_reminders` (orphan-fix #5) |
| T3 | `restore_session_sanitizes_pre_cutoff_listing_diff_snapshot` | L5646–5745 | 100 | #28 `sanitize_listing_diff_context_snapshot_if_needed` (cross-module: also calls `restore_session` from session_restore.rs) |
| T4 | `rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff` | L5750–5875 | 126 | #28 `sanitize_listing_diff_context_snapshot_if_needed`, #18 `truncate_listing_baseline_rebuild_turn_index_after_rollback` |
| T5 | `records_subagent_partial_timeout_in_evidence_ledger` | L6051–6130 | 80 | #5 `record_subagent_partial_timeout`, #3 `evidence_events_for_turn`, #3 `evidence_summary_for_session` |
| T6 | `skill_agent_baseline_override_snapshot_persists_across_session_restore` | L6133–6190 | 58 | #13 `skill_agent_baseline_override_snapshot`, #12 `remember_skill_agent_baseline_override_snapshot` |
| T7 | `seed_forked_skill_agent_listing_baselines_splits_prompt_and_diff_baselines` | L6191–6287 | 97 | #14 `seed_forked_skill_agent_listing_baselines`, #12 `remember_skill_agent_baseline_override_snapshot`, #8 `turn_skill_agent_snapshot`, #11 `recover_first_turn_skill_agent_snapshot` |

**Total test body size**: 87+109+100+126+80+58+97 = **657 rows**.

> **Test moves rationale**: per Round 3b plan, "tests are downstream — a test
> may explicitly call the session_manager.rs version expecting a specific
> behavior". For R4 aggressive split, we co-locate tests with their tested
> methods so a reviewer reading `session_evidence.rs` sees the full evidence
> story (method + behavior + edge cases) in one file. If the reviewer prefers
> to defer test moves (lower-risk apply), see Errata E5 for the alternative.

### 1.4 Move targets — summary count

| Destination | New fns from this spec | Already present (R3 orphan-fix canonical) | Total after R4 |
|---|---|---|---|
| `session_evidence.rs` | 9 + 7 tests (#20–28, T1–T7) | 19 canonical after orphan-fix dedup (#1–19) | **34 fns + 7 tests** (~1500 rows) |
| `session_manager.rs` (deleted) | 19 duplicates + 9 new moves + 7 tests | n/a | -19 -9 -7 = **-35** |
| `session_manager.rs` (kept) | 0 evidence-domain | 138 → 138 - 19 - 9 - 7 test rows = 103 fns + 31 tests | rows 6532 → ~4982 |

Body lines moving to `session_evidence.rs` (sum of body sizes in #20–28, T1–T7):
~226 method-body rows + 657 test-body rows = **~883 net additions**.
After R4, `session_evidence.rs` ≈ 749 + 883 ≈ **~1632 rows** (matches the ~1500
target; within tolerance for `use` imports + impl-block scaffolding).

`session_manager.rs` drops from 6532 → ~4900 rows after method + test moves (still
large; further R5+ splits planned).

## 2. Migration steps

Pre-req: R3 orphan-fix is merged. `session_manager.rs` no longer contains the
6 originally-orphaned duplicates (append_evidence_event, invalidate_ai_clients_for_models,
rebuild_skill_agent_listing_baseline_to_latest, remove_listing_diff_internal_reminders,
strip_listing_diff_internal_reminders, spawn_model_reconciliation_listener), and
`mod.rs` declares `pub mod session_evidence;` + `pub use session_evidence::*;`.

Verify by:
```bash
git -C northing grep -nE "^mod session_evidence" -- \
 src/crates/assembly/core/src/agentic/session/mod.rs
# → must show `mod session_evidence;` declared (per agent memory: orphan-dead-code
# failure mode is to add files without wiring mod.rs)
```

### Step A — Model reconciliation helper cluster (#20–24)

1. Read `session_manager.rs` L144–208 to capture current bodies.
2. Move fns #20–24 to `session_evidence.rs` as a new cluster of `pub(crate)`/`fn`
 methods inside the existing `impl SessionManager { ... }` block (just append
 after the current L683 `spawn_model_reconciliation_listener` end-of-block at L749).
3. Visibility:
 - `load_ai_config_for_model_resolution` (#20) stays `fn` (private). R3b plan
 kept this in manager; we move it because it's only called from the other
 reconciliation fns that are moving.
 - `is_auto_model_selector` (#21) stays `fn`.
 - `context_window_for_model_selection` (#22) stays `fn`.
 - `session_context_window_from_ai_config` (#23) stays `fn`.
 - `sync_session_context_window_from_ai_config` (#24) stays `fn`.
4. **Cross-module call**: `sync_session_context_window_from_ai_config` (#24) takes
 `&self` and writes to `self.sessions.get_mut(...)` — same-struct field access,
 works in sibling impl block.
5. **External callers**: these 4 helpers are static `Self::xxx()` call sites only
 (from `update_session_model_id` at L2114 and `refresh_session_context_window`
 at L2179). Since the call is `Self::foo`, Rust resolves across all impl blocks
 on `SessionManager`. No caller update needed.
6. Run `cargo check -p northhing-core --features product-full` → expect ~0 errors.

### Step B — Snapshot persistence cluster (#25–26)

1. Read `session_manager.rs` L635–685 to capture current bodies.
2. Move fns #25–26 to `session_evidence.rs`. Append after the reconciliation
 cluster (after Step A).
3. Visibility:
 - `persist_context_snapshot_for_turn_best_effort` (#25) stays `async fn`
 (private). The duplicate in `session_evidence.rs` is `pub(crate) async` —
 this spec REPLACES the current private `session_manager.rs` body with the
 canonical `pub(crate)` version from `session_evidence.rs` (matching the
 pattern for items #1–19).
 - `persist_current_turn_context_snapshot_best_effort` (#26) same pattern.
4. **Cross-module call**: both call `self.persistence_manager.save_turn_context_snapshot`
 and `self.context_store.get_context_messages` — same-struct field access.
 #26 also calls #25 (`Self::persist_context_snapshot_for_turn_best_effort`)
 — sibling-impl static call works.
5. **External callers**: from `add_message` (L4025), `replace_context_messages` (L4034),
 `remove_listing_diff_internal_reminders` (#18), `start_dialog_turn*` (L3357+),
 `complete_dialog_turn` (L3596), `fail_dialog_turn` (L3713), `start_maintenance_turn`
 (L3474), and many more. All use `Self::persist_current_turn_context_snapshot_best_effort`
 static call — resolves across sibling impl blocks. No caller update needed.
6. **Cross-spec conflict**: persistence-extend spec (`spec-session-persistence-real-wire`)
 also moves #25–26 to `session_persistence.rs`. **Resolution: this evidence-extend
 spec takes ownership** (snapshot domain = evidence; reconcile with
 persistence-extend spec author — see Errata E1).
7. Run `cargo check -p northhing-core --features product-full` → expect ~0 errors.

### Step C — Skill agent snapshot persistence loader (#27)

1. Read `session_manager.rs` L721–730 to capture current body.
2. Move fn #27 to `session_evidence.rs`. Append after the snapshot cluster.
3. Visibility: stays `async fn` (private) in manager; promote to `pub(crate) async`
 in evidence.rs (canonical). The existing duplicate in `session_persistence.rs:266`
 is also `pub(crate) async` and identical body — DELETE the persistence.rs copy
 when R4 lands (cross-spec coordination required).
4. **Cross-module call**: takes `&self` and `workspace_path: &Path`. Calls
 `self.persistence_manager.load_turn_skill_agent_snapshot(...)` — same-struct
 field access.
5. **External callers**: only from `turn_skill_agent_snapshot` (#8, canonical in
 `session_evidence.rs`) — `Self::load_turn_skill_agent_snapshot_from_persistence`
 static call works across sibling impl.
6. **Cross-spec conflict**: persistence-extend spec puts #27 in
 `session_persistence.rs` (its `#15` `load_turn_skill_agent_snapshot_from_persistence`).
 **Resolution: this evidence-extend spec takes ownership** (loader for
 evidence-domain snapshot store; the round3b plan put it in persistence but
 the load fn is the persistence half of an evidence-domain object).
 See Errata E2.
7. Run `cargo check` → expect ~0 errors.

### Step D — Listing diff snapshot sanitizer (#28)

1. Read `session_manager.rs` L1694–1731 to capture current body.
2. Move fn #28 to `session_evidence.rs`. Append after #27.
3. Visibility: stays `async fn` (private) in manager; promote to `pub(crate) async`
 in evidence.rs. The existing duplicate in `session_persistence.rs:467` is
 also `pub(crate) async` — DELETE the persistence.rs copy when R4 lands.
4. **Cross-module call**: calls `Self::strip_listing_diff_internal_reminders`
 (#19, canonical in evidence.rs — was orphan-fix #5) — sibling static call.
 Also calls `Self::persist_context_snapshot_messages_best_effort` (#19
 canonical in evidence.rs) — sibling static call.
5. **External callers**: from `restore_session_with_turns_internal` (L2634,
 canonical in `session_restore.rs`) — `Self::sanitize_listing_diff_context_snapshot_if_needed`
 static call resolves across sibling impl. No caller update.
6. **Cross-spec conflict**: persistence-extend spec puts #28 in
 `session_persistence.rs` (its `#25` if it picks this up). **Resolution: this
 evidence-extend spec takes ownership** (sanitizer for evidence-domain
 listing-diff cutoff). See Errata E1.
7. Run `cargo check` → expect ~0 errors.

### Step E — Delete the 19 evidence duplicates (#1–19) from session_manager.rs

Apply after Steps A–D land so the canonical `session_evidence.rs` versions are
in place.

For each of the 19 duplicates, **delete the body** from `session_manager.rs`
(line ranges from §1.1) and replace with a single comment line:
```rust
// evidence: moved to session_evidence.rs (R4 spec-session-evidence-real-wire)
```

Run `cargo check -p northhing-core --features product-full` after each deletion
batch (group: evidence-ledger, then skill-snapshot, then listing-baseline, then
model-reconciliation) — expect ~0 errors each time. The orphan-dead-code failure
mode (agent memory 2026-06-27) does not apply here because the canonical
`session_evidence.rs` versions exist and are wired through `mod.rs` by R3 orphan-fix.

### Step F — Add `#[cfg(test)] mod tests` to session_evidence.rs + move 7 tests (#T1–T7)

1. Append a `#[cfg(test)] mod tests { ... }` block to the bottom of
 `session_evidence.rs` (after the `impl SessionManager { ... }` closing brace
 at L749).
2. Move tests T1–T7 verbatim from `session_manager.rs::tests` to
 `session_evidence.rs::tests`.
3. Imports needed in the new test module (verify each with `cargo check`):
 - `use super::*;` — for `SessionManager`, `EvidenceLedgerEvent`,
 `EvidenceLedgerTargetKind`, `EvidenceLedgerEventStatus`,
 `EvidenceLedgerCheckpoint`, `CompressionContract`,
 `CompressionContractItem`, `Message`, `MessageRole`, `MessageSemanticKind`,
 `TurnSkillAgentSnapshot`, `SkillSnapshotEntry`
 - `use crate::agentic::session::session_store_port::CoreSessionStorePort;`
 - `use crate::agentic::service::test_helpers::TestWorkspace;` (verify path —
 may be `crate::agentic::tests::*` or `crate::test_utils::*`)
 - `use crate::agentic::persistence::PersistenceManager;`
 - `use crate::agentic::session::{DialogTurnData, UserMessageData};`
 - `use crate::agentic::session::session_manager::SessionManagerConfig;` (or
 rebuild via `test_manager` helper — see §2.7 below)
 - `use std::sync::Arc;`
4. **Test helper issue**: the 7 tests use a `test_manager` helper that's defined
 in `session_manager.rs::tests` (private fn). Two options:
 - **Option A (preferred)**: Move `test_manager` helper to `session_evidence.rs::tests`
 too. It's a tiny helper (~10 lines) that creates a `SessionManager` with a
 real persistence manager.
 - **Option B**: Keep `test_manager` in `session_manager.rs::tests`, make it
 `pub(super)` so `session_evidence.rs::tests` can call it via
 `crate::agentic::session::session_manager::tests::test_manager`.
 - **Default**: Option A. Move the helper with the tests.
5. Cross-module call inside tests: `test_manager()` calls `SessionManager::new` (L805
 in `session_manager.rs`) — `SessionManager::new` stays in `session_manager.rs`,
 which is a sibling impl. `test_manager` can be defined in either file; if in
 `session_evidence.rs::tests`, it still calls `SessionManager::new` from the
 sibling impl block.
6. **Cross-module call into session_restore.rs**: T3 calls `manager.restore_session(...)`
 which is canonical in `session_restore.rs` (sibling impl). Works.
7. Run `cargo test -p northhing-core --features product-full session_evidence`
 (or narrower: `cargo test -p northhing-core latest_skill_agent_snapshot_scans_persistence_beyond_stale_cache_hit`).
 Expect all 7 tests pass.

### Step G — Final cleanup

1. Run `cargo check --workspace --features product-full` → expect clean
 (~0 errors, may have additional warnings for now-unused imports in
 session_manager.rs if any persistence-extend or restore-extend specs already
 landed; verify with `cargo check`).
2. Run `cargo test -p northhing-core --features product-full` → expect all
 935 existing tests pass plus the 7 moved tests run from their new home.
3. Run `pnpm run fmt:rs` if any whitespace drifted.
4. Confirm `git diff --stat` shows ONLY:
 - `session_evidence.rs` grew ~883 lines.
 - `session_manager.rs` shrank ~883 lines.
 - `session_persistence.rs` shrank ~38 lines (#27 + #28 deletions per cross-spec).
 - `session_restore.rs` unchanged.
 - `mod.rs` unchanged (orphan-dead-code trap check).
5. Confirm no `cargo fmt` churn beyond whitespace drift (agent memory 2026-06-24:
 pre-existing 156 uncommitted fmt changes; do not touch).

## 3. Acceptance criteria

### Functional

- `session_evidence.rs` grows from 749 → **~1500–1650 rows** (34 fns + 7 tests).
- `session_manager.rs` shrinks from 6532 → **~4900 rows** (103 fns + 31 tests).
- All 19 listed duplicates (#1–19) deleted from `session_manager.rs`.
- All 9 new methods (#20–28) live in `session_evidence.rs` after R4.
- All 7 tests (T1–T7) live in `session_evidence.rs::tests` after R4.
- `session_persistence.rs` loses `load_turn_skill_agent_snapshot_from_persistence`
 (its L266) and `sanitize_listing_diff_context_snapshot_if_needed` (its L467)
 per cross-spec coordination. `session_persistence.rs` row count drops by ~48.
- External API unchanged: every `crate::agentic::session::SessionManager::foo()`
 call site continues to compile without import changes (sibling impl blocks
 preserve the `Self::foo` lookup).
- `cargo check --workspace --features product-full` exits clean.
- `cargo test -p northhing-core --features product-full` exits clean (all 935+
 existing tests pass per R3 baseline + 7 moved tests in new home).

### Mechanical

- mod.rs NOT modified (sibling impl blocks, no new `mod` declarations).
- `#[cfg(test)] mod tests` added to `session_evidence.rs` (new addition).
- Visibility on moved methods promoted from `fn`/`async fn` to `pub(crate)`/
 `pub(crate) async` only where the canonical evidence.rs version already had
 `pub(crate)` (items #6, #7, #15, #16, #17, #19).
- Visibility on NEW moved methods (#20–24, #25–26, #27, #28) follows the
 canonical `session_evidence.rs` pattern: `fn`/`async fn` stays private;
 promotion to `pub(crate)` only when required by external callers (none in
 this spec — all callers use `Self::` from within SessionManager).
- No new `use` imports in `session_manager.rs`; `session_evidence.rs` gains
 `serde_json::json` if not already imported (verify: line 51 currently has it).
- No `#[allow(dead_code)]` removed or added (the existing 106 occurrences
 per audit stay untouched).
- Commit message style: `refactor(session): extract <fn-name>-to-evidence-r4` per
 fn group, OR a single `refactor(session): move 28 evidence fns + 7 tests to
 session_evidence (R4)`.

### Out-of-scope (do not do in R4)

- Do NOT move `restore_*` fns (already in `session_restore.rs`).
- Do NOT move persistence auto-save/cleanup/metadata fns (already in
 `session_persistence.rs` per persistence-extend spec).
- Do NOT touch `update_compression_state` callers outside this module —
 `compression::` may import it; verify no breakage with `cargo check`.
- Do NOT introduce new helpers or refactor bodies during the move.
 Mechanical cut-paste only.
- Do NOT move the auto-save / cleanup / prompt-cache persistence fns even
 if some are evidence-adjacent — those go to persistence per
 persistence-extend spec.

## 4. Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | `self.<evidence_field>` in sibling-impl might fail if field is private and another module reads it | Low | High | All touched fields are on `SessionManager` itself; same struct sibling impl has full access. No fix needed. |
| R2 | Visibility promotion (private `fn` → `pub(crate)`) for items #6, #7, #15, #16, #17, #19 may surface new `dead_code` lint warnings if no caller is `pub(crate)`. | Low | Low | The canonical evidence.rs versions already had `pub(crate)` and compiled clean. After deleting manager copy, no caller breakage. |
| R3 | Some `update_*` fns call `Self::persist_current_turn_context_snapshot_best_effort` (#26). These stay in manager but are called via `Self::` — Rust resolves `Self::` across all impl blocks on the type. | Low | Low | Verified by Rust spec; no fix needed. |
| R4 | `sync_session_context_window_from_ai_config` (#24) mutates `self.sessions.get_mut(...)` — same-struct field access works in sibling impl. | Low | Medium | Verify `cargo check` after Step A. |
| R5 | `sanitize_listing_diff_context_snapshot_if_needed` (#28) is called from `restore_session_with_turns_internal` (L2634, in `session_restore.rs`). Cross-module static call resolves across sibling impl blocks. | Low | Medium | Already `Self::`; verified. |
| R6 | Test imports for the 7 tests (#T1–T7) may differ from session_manager.rs::tests. The new `session_evidence.rs::tests` block needs explicit `use super::*;` plus test-helper imports. | Med | Medium | Read each test's imports verbatim from session_manager.rs::tests; copy to session_evidence.rs::tests. Run `cargo test` to catch missing imports. |
| R7 | `test_manager` helper is private in `session_manager.rs::tests`. After moving 7 tests, this helper needs to move too OR be made `pub(super)`. | Low | Low | Move to `session_evidence.rs::tests` (Option A); small ~10-line helper, mechanical cut-paste. |
| R8 | Cross-spec conflict with persistence-extend: items #25, #26, #27, #28 also claimed by persistence-extend (its #4, #5, #15, #25). If both specs land without coordination, three-place duplicates may occur. | High | High | Errata E1 + E2: this spec takes ownership. Coordinate via Errata before apply; if persistence-extend lands first, conflict resolution is in this spec's scope. |
| R9 | `session_evidence.rs` file size — Rust-analyzer may slow on >1500-line files. Acceptable; future R5 could split `session_evidence_reconciliation.rs` or `session_evidence_baseline.rs` out. | Low | Low | Note in handoff; track for future split. |
| R10 | Mechanical move may shift `tracing::info!` call sites; ensure level matches (currently `info!` for skill-agent baseline persistence, `debug!` for most others). | Low | Low | Don't change during move; preserve log level verbatim. |
| R11 | If R3 orphan-fix is NOT yet merged when R4 starts, the duplicate detection may mis-count and move fns that already exist in `session_evidence.rs`. | Med | High | Verify R3 merged first: `git grep -c "fn rebuild_skill_agent_listing_baseline_to_latest" northing/src/crates/assembly/core/src/agentic/session/session_manager.rs` should be 0. If non-zero, R3 not applied → STOP and escalate. |
| R12 | After R4 apply, `session_manager.rs::tests` loses 7 tests. The 31 remaining tests must still pass. | Low | Low | No shared state between tests; verified by R3 baseline. |
| R13 | `Persist_context_snapshot_messages_best_effort` (#19) is also in `session_persistence.rs:443` per session_impl_analysis §Cross-block duplicates. Triple-place. | Med | High | Errata E3: delete from session_persistence.rs when R4 lands (cross-spec coordination). |

## 5. Errata

### E1 — Snapshot persistence (items #25–26, #28) ownership: evidence, not persistence

The persistence-extend spec (plan_c5a5067a/spec-session-persistence-real-wire)
places these in `session_persistence.rs` (its `#4` `persist_context_snapshot_for_turn_best_effort`,
`#5` `persist_current_turn_context_snapshot_best_effort`,
`#25` if it picks up the sanitizer).

**Resolution (this spec takes ownership)**: The task brief says "evidence-domain
methods (evidence ledger, snapshot, reconciliation, baseline)". The word
"snapshot" is in scope. The fns persist context snapshots that are
**read back by evidence restore / rollback / listing-diff cutoff logic** — they
are the persistence half of an evidence-domain read path. Co-locating the
persist call with the read paths in `session_evidence.rs` keeps the evidence
story in one file.

**Action for persistence-extend author**: drop items `#4`, `#5`, and any `#25`
that picks up `sanitize_listing_diff_context_snapshot_if_needed` from the
persistence-extend spec; defer to this evidence-extend spec. The persistence-extend
spec row count target (~2200–2400) adjusts down by ~100 rows.

### E2 — `load_turn_skill_agent_snapshot_from_persistence` (#27) ownership: evidence, not persistence

The persistence-extend spec places this in `session_persistence.rs` (its `#15`).

**Resolution (this spec takes ownership)**: Same reasoning as E1. The loader
reads from disk into `TurnSkillAgentSnapshot`, which is consumed by
`turn_skill_agent_snapshot` (#8, canonical in evidence.rs). The data IS
evidence; the storage is just the disk layer. Move to evidence.rs.

**Action for persistence-extend author**: drop item `#15` from persistence-extend
spec; defer to this evidence-extend spec. Persistence-extend spec row count
target adjusts down by ~10 rows.

### E3 — `persist_context_snapshot_messages_best_effort` (#19) triple-place duplicate

This fn exists in three places per `session_impl_analysis.md` line 288:
- `session_evidence.rs:606` (canonical, `pub(crate) async`)
- `session_manager.rs:1670` (orphan-fix should delete)
- `session_persistence.rs:443` (orphaned duplicate from R3b)

**Resolution (this spec)**: After R3 orphan-fix and R4 deletion of manager copy,
the function lives in `session_evidence.rs:606` (canonical) and
`session_persistence.rs:443` (still duplicate). This spec adds Step E0:
**delete from `session_persistence.rs`** (cross-spec coordination required).
After Step E0, only `session_evidence.rs:606` remains.

**Action for persistence-extend author**: do NOT add `persist_context_snapshot_messages_best_effort`
to persistence-extend spec — it does not belong there. The persistence spec
target row count adjusts accordingly.

### E4 — Model reconciliation helpers (#20–24) move from manager to evidence

The R3b plan kept `load_ai_config_for_model_resolution`, `is_auto_model_selector`,
`context_window_for_model_selection`, `session_context_window_from_ai_config`,
`sync_session_context_window_from_ai_config` in `session_manager.rs` (round3b
plan §2.1, "model reconciliation helper"). This R4 spec moves them to
`session_evidence.rs` (item #20–24).

**Rationale**: All 4 helpers are private static fns only called from
`update_session_model_id` (L2114) and `refresh_session_context_window` (L2179),
both of which feed the model reconciliation flow. Moving them keeps the
evidence/reconciliation story together. The 4 helpers are small (~60 total
body rows) and co-locate cleanly with `spawn_model_reconciliation_listener`
(canonical in evidence.rs).

**External call impact**: `update_session_model_id` and `refresh_session_context_window`
stay in `session_manager.rs`. They call `Self::load_ai_config_for_model_resolution`
etc. via static `Self::` lookup, which resolves across sibling impl blocks.

### E5 — Test moves (items T1–T7) optional, default-included

If the reviewer prefers to defer test moves to R5 (lower-risk apply, less
re-import churn), drop Steps F entirely. The 7 tests stay in
`session_manager.rs::tests` and continue to call methods that now live in
`session_evidence.rs` via `Self::` static calls.

**With test moves** (default): `session_evidence.rs` reaches **~1632 rows**
(matches ~1500 target with some headroom).

**Without test moves**: `session_evidence.rs` reaches **~975 rows** (misses
~1500 target by ~525 rows). To still hit ~1500, expand R4 to also include
the evidence-domain auto-save / cleanup snapshot helpers (per session_manager.rs
L319–349) — but those are properly persistence-domain per persistence-extend
spec and would require additional cross-spec coordination.

**Default**: include test moves. The 657-row test block is co-located logic
that benefits from being in the same file as the methods it tests.

### E6 — `sanitize_listing_diff_context_snapshot_if_needed` visibility promotion

After R4, this fn becomes `pub(crate) async` (was `async fn` private in
session_manager.rs, `pub(crate) async` in session_persistence.rs:467).

**Reasoning**: The canonical version is `pub(crate)` (in evidence.rs after
move). Promote for symmetry; no external caller impact (private → private
since cross-crate calls go through `crate::agentic::session::*` re-exports
in mod.rs).

### E7 — `session_evidence.rs` size target ~1500 vs actual ~1632

The target ~1500 is approximate (per task brief wording). Actual size after
R4 with test moves: 749 + ~226 method bodies + ~657 test bodies + ~10
import lines = **~1642 rows**. This is within 10% of the ~1500 target and
acceptable per the "smallest files" aggressive-split mandate. Future R5+
could split `session_evidence_reconciliation.rs` or `session_evidence_baseline.rs`
out if rust-analyzer perf degrades.

### E8 — `load_ai_config_for_model_resolution` async-vs-sync

This fn (#20) is `async fn load_ai_config_for_model_resolution() -> Option<AIConfig>`
in session_manager.rs. Its body uses `tokio::spawn`— No — it's a pure function
calling `get_global_config_service().get_config()`. The `async` is required
because `get_global_config_service` is async. Move as-is; do not change to
sync.

### E9 — `update_session_model_id` and `refresh_session_context_window` stay in manager

These two callers (L2114, L2179) consume #20–24 but are NOT themselves
evidence-domain — they are session-CRUD mutation fns that happen to compute
context-window side effects. They stay in `session_manager.rs`. Their
`Self::load_ai_config_for_model_resolution(...)` calls resolve across
sibling impl blocks.

### E10 — `session_evidence.rs` test module pattern

`session_evidence.rs` currently has no `#[cfg(test)] mod tests` block. R4
adds one. Pattern from Round 3a `coordinator_*.rs` (sibling files in
coordinator split): co-locate tests in the same file as tested methods,
use `use super::*;` for `SessionManager` + sibling items, use
`use crate::agentic::service::test_helpers::*;` for `TestWorkspace`. Verify
exact path with `git grep -nE "use .*TestWorkspace" -- "src/crates/assembly/core/src/agentic/session/session_manager.rs"`
before writing spec.
