# Deliverable: spec-session-persistence-real-wire

## Summary

Wrote R4 spec identifying **42 persistence-domain methods** in `session_manager.rs` (138 fns / 6532 lines) that should move to `session_persistence.rs` beyond the 7 already deduplicated by the prior R3 orphan-fix. Spec covers predicate/helpers (#1–14), session create/write (#16–19), state/title/agent/model updates (#20–26), delete cascade (#29), metadata rw (#30–40), message + compression (#41–43), and background tasks (#44–45) — totaling ~1086 body lines, growing `session_persistence.rs` from 1272 → ~2200–2400 lines. **4 methods explicitly excluded** with reasoning (`refresh_session_context_window`, `touch_session`, pure title/pagination helpers).

## Changed files

Created (write-only; no source files modified per task constraint):

- `C:\Users\UmR\.mavis\plans\plan_c5a5067a\outputs\spec-session-persistence-real-wire\docs\handoffs\2026-06-27-r4-session-persistence-extend-spec.md` (27056 bytes) — primary spec deliverable inside plan workspace
- `E:\agent-project\docs\handoffs\2026-06-27-r4-session-persistence-extend-spec.md` (27056 bytes) — mirror copy at project-level handoff path requested by task brief
- `C:\Users\UmR\.mavis\plans\plan_c5a5067a\outputs\spec-session-persistence-real-wire\deliverable.md` — this file

No source files modified (`session_manager.rs`, `session_persistence.rs`, `mod.rs` all untouched — spec-only per task brief "DO NOT modify source files. Spec only.").

## Notes

### Verification done before spec
1. Read prior duplicate-scanner analysis at `~/.mavis/plans/plan_8b640472/outputs/duplicate-scanner/session_impl_analysis.md` — confirmed 28 fns in `session_persistence.rs` canonical, 7 are dedup targets.
2. Read `session_persistence.rs` (1272 lines) end-to-end to confirm current canonical contents and verify imports.
3. Read `session_manager.rs` method-by-method across sections (L200–515 helpers, L1071–2200 create/update/delete, L3049–3265 metadata, L3990–4360 messages/cleanup) to classify each fn as persistence / session-lifecycle / restore / evidence / pure-helper.
4. Verified `session_evidence.rs` already owns model-migration fns (`is_session_model_id_usable` L137, `migrate_sessions_off_invalidated_models` L157, `invalidate_ai_clients_for_models` L215) — those should NOT be re-moved to persistence (out-of-scope).
5. Verified `session_restore.rs` already owns 16 restore fns — those are out-of-scope.
6. Verified all needed imports are already in `session_persistence.rs` (Appendix A of spec) — no new `use` statements required.

### Spec structure (per user brief)
1. Context — what's done in R3 vs what's needed in R4.
2. Persistence fns identified — table with 45 rows (42 moves + 3 explicit stays). Columns: fn, location, body size, dependencies, move target.
3. Migration steps — 10-step apply order (A through J), each step is one commit.
4. Acceptance criteria — functional (line counts, fn counts), mechanical (mod.rs unchanged, visibility unchanged), out-of-scope.
5. Risks — 12 numbered risks with likelihood/impact/mitigation.
6. Errata — 13 notes explaining borderline decisions (E1–E13), including the 4 explicit stays.

### Key design decisions
- **Sister-impl strategy**: all 42 moves use sibling `impl SessionManager { ... }` blocks per the Rust god-object-split template — no `mod.rs` change, no visibility change, no new imports.
- **`update_session_model_id` (#26) moves** because it persists via `save_session`, even though it also calls `restore_session` (sibling-impl call works). 
- **`refresh_session_context_window` (#27) stays** because it has no persistence call (Errata E1).
- **`touch_session` (#28) stays** because it's in-memory only (Errata E2).
- **Title-generation AI fns stay** because they're AI calls, not persistence (Errata E3).
- **`session_workspace_path` `#[allow(dead_code)]` preserved** with reason comment (Errata E5).

### Pre-req for apply (R3 dependency)
The spec assumes R3 orphan-fix is merged. Verify before applying R4:
```bash
git grep -c "fn build_messages_from_turns" \
  E:/agent-project/northing/src/crates/assembly/core/src/agentic/session/session_manager.rs
```
→ should be `0` (R3 removes the duplicate). If non-zero, **STOP and escalate** — applying R4 before R3 will cause compile errors (duplicate fn definitions). Risk R11 covers this.

### Estimated apply effort
- Step A (helpers): ~15 min — 14 small fns, mechanical.
- Step B (resolve_workspace_path): ~10 min — single 76-line fn with cross-module state reads.
- Step C (create_session family): ~15 min — 4 fns but body of `create_session_with_id_and_details` is 60 lines.
- Step D (update_*): ~25 min — 7 fns with cross-impl calls to `restore_session` and helpers.
- Step E (delete_session): ~15 min — 152-line fn with snapshot-system integration.
- Step F (metadata rw): ~20 min — 11 fns, mostly thin wrappers.
- Step G (messages/compression): ~10 min — 3 small fns.
- Step H (background tasks): ~15 min — 2 spawn fns with sub-store cloning.
- Step I (verify new caller): ~5 min — `cargo check`.
- Step J (final verification): ~10 min — full `cargo check --workspace`, `cargo test -p northhing-core`.

Total estimated apply time: **~2.5 hours** if no surprises; budget **3–4 hours** including review fixes.

### Review considerations (for downstream reviewer)
1. Verify R3 orphan-fix is committed (see pre-req check above).
2. Spot-check that `mod.rs` is unchanged after the moves (orphan-dead-code trap per agent memory 2026-06-27 entry).
3. Confirm `cargo check --workspace` clean.
4. Confirm `cargo test -p northhing-core` clean (935 baseline tests per R3).
5. Confirm `git diff --stat` shows only the two files grew/shrunk.
6. Confirm `tracing::info!` call levels unchanged in `update_session_title` and `create_session_with_id_and_details`.

### Spec limitations
- Body-line totals in §1 are approximate (line spans between fn signature and closing brace, excluding signature and closing brace themselves). Appendix B has the more precise breakdown.
- Spec assumes no caller refactoring is needed. If a downstream caller relies on `crate::agentic::session::SessionManager::update_session_title` going through `&self` correctly across sibling impl blocks (Rust should handle this), no caller change is needed. If a caller somehow captures the impl-block address (rare), refactor needed.
- Spec does not address whether to add a `pub use` re-export at `mod.rs` for any persistence helpers that downstream code wants to import directly. Current downstream code accesses via `SessionManager` methods, so no re-export needed. Verify by `grep -rn "use crate::agentic::session::SessionManager::" E:/agent-project/northing/src/`.