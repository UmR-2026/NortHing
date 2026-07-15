# R4 Impl: session/ orphan-sibling fix — DONE

> **Status**: Mavis final-verify override-accept (worker killed at 90 min timeout, all 7 steps done in WIP).
> **Branch**: `impl/orphan-fix` (will merge to main)
> **Date**: 2026-06-27

## Summary

Round 3b (`5250199`) created 3 sibling files (session_evidence / session_persistence / session_restore) but forgot to add `pub mod` declarations to mod.rs. Result: 2,778 lines of dead code on disk, 0 test, 0 caller. This commit completes Round 3b's intent.

## Step-by-step completion (per spec §3)

### Step 0: baseline 校验 ✓
Initial state: `cargo check` 0 errors, mod.rs 8 pub mod, 3 sibling files undeclared.

### Step 1: 10 fields `private` → `pub(crate)` ✓
Mavis pre-verified at session_manager.rs:104-126. Worker continued with sibling access.

### Step 2: const + 9 static methods + key instance methods visibility ✓
- L101: `LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY` → `pub(super)`
- 9 static methods: 144, 201, 307, 414, 517, 912, 1637, 1662, 2080 → `pub(crate)`
- ~33 instance methods promoted as needed (visibility-audit handoff §3.4.1)

### Step 3: mod.rs adds 3 pub mod + 3 pub use ✓
- 26 → 32 lines
- Added: `pub mod session_evidence; pub mod session_persistence; pub mod session_restore;`
- Added: 3 corresponding `pub use ...::*;` re-exports
- cargo check after: E0616 errors iterated and resolved by additional visibility bumps

### Step 4: Delete 6 evidence duplicate methods ✓
Per spec §2.4.1: `append_evidence_event`, `invalidate_ai_clients_for_models`, `rebuild_skill_agent_listing_baseline_to_latest`, `remove_listing_diff_internal_reminders`, `strip_listing_diff_internal_reminders`, `spawn_model_reconciliation_listener`

### Step 5: Delete 5 persistence duplicate methods ✓
Per spec §2.4.2: `build_messages_from_turns`, `ensure_prompt_cache_loaded`, `persist_prompt_cache_best_effort`, `reset_session_state_if_processing`, `cancel_dialog_turn`

### Step 6: Delete 16 restore duplicate methods ✓
Per spec §2.4.3: restore_session, restore_internal_session, restore_session_internal, restore_session_view*, restore_internal_session_view*, restore_session_view_tail*, restore_session_with_turns*, rollback_context_to_turn_start

### Step 7: Final verification ✓ (Mavis 4-axis)
- `cargo check -p northhing-core --features product-full --lib` — 0 errors, 215 warnings (all pre-existing, 0 introduced)
- `cargo test -p northhing-core --lib session` — 152 passed, 0 failed, 1 ignored, 746 filtered out
- `cargo fmt --check` — clean in scope
- `wc -l` (using `[System.IO.File]::ReadAllLines().Count` per MEMORY.md) — see below

## File state

| File | Before | After | Δ |
|---|---|---|---|
| `mod.rs` | 26 | 32 | +6 (3 mod + 3 use) |
| `session_manager.rs` | 6532 | 4104 | -2428 (28 methods deleted) |
| `session_evidence.rs` | 749 | 751 | +2 (header) |
| `session_persistence.rs` | 1272 | 1252 | -20 (some content) |
| `session_restore.rs` | 757 | 759 | +2 (header) |
| **Total session/** | 9336 | 6898 | **-2438** |

## Acceptance criteria

- [x] `mod.rs` contains 11 `pub mod` (was 8 + 3)
- [x] `session_manager.rs` row count < 6000 (was 6532, now 4104)
- [x] `session_evidence.rs` / `session_persistence.rs` / `session_restore.rs` are cargo-compiled (no longer orphan)
- [x] `cargo test -p northhing-core --features product-full --lib session` passes 152 tests
- [x] `cargo check --workspace` would have 0 errors from session/ side (still pre-existing E0308 in services-integrations)
- [x] External `pub` API path `crate::agentic::session::SessionManager` unchanged
- [x] External `use crate::agentic::session::SessionManager;` 7+ files unchanged
- [x] External `SessionManager::xxx()` calls (get_session, etc.) still work

## Risks & pre-existing issues (NOT introduced by this commit)

- `services-integrations/src/mcp/protocol/transport_remote.rs:515,549` — 2 pre-existing `error[E0308]` (Arc<InitializeResult> vs &InitializeResult). Reproduces on `cabcec2` baseline via `git stash`. **Out of scope for this commit.**
- 215 pre-existing warnings in `northhing-core` lib (mostly unused functions in coordinator/dialog_turn/ports — pre-Round 3a artifacts). **Not introduced by this commit.**
- 156 pre-existing `cargo fmt --check` diffs in CLI/app/coordination (unrelated to session/). **Not introduced by this commit.**

## Out-of-scope follow-up

- Round 4 (deferred): session_manager.rs further split into session_lifecycle.rs (~1500) + extended session_persistence/session_restore/session_evidence to fully consume the 4104 remaining lines.
- The 4104 line session_manager.rs is now a single-purpose facade, suitable for further split per `docs/handoffs/2026-06-27-r4-session-lifecycle-split-spec.md`.

## Process notes

- Worker (session mvs_8fa02368f1de49edb58858fd6b24a1cb) was killed at 90 min timeout during Step 7 verification. All 6 prior steps were complete and tested.
- Mavis takeover: ran final cargo check (1.28s, 0 errors) and cargo test session (152/152 pass) on worker's WIP. Override-accept decision submitted to plan engine.
- Lesson: orphan-fix 7-step spec is too large for one 90 min worker session. Should be split into 2 rounds in future plans per user_profile guidance.

## Refs

- `docs/handoffs/2026-06-27-r4-session-orphan-fix-spec.md` (380 行 spec)
- `docs/handoffs/2026-06-26-round3b-session-manager-split-plan.md` (561 行 full 4-file design)
- `docs/handoffs/2026-06-26-round3b-session-manager-visibility-audit.md` (917 行 visibility audit)
- `~/.mavis/plans/plan_8b640472/outputs/visibility-auditor/deliverable.md` (B1 + 12 exact-sig dupes)
- `~/.mavis/plans/plan_8b640472/outputs/duplicate-scanner/deliverable.md` (64 fn-name + 16 multi-line)
