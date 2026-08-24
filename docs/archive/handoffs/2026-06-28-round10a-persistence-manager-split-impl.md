# Round 10a Impl: persistence/manager.rs 3650 → facade + 6 siblings

> **Status**: ✅ Implemented
> **Branch**: `impl/round10a-persistence-manager-split`
> **Worktree**: `E:\agent-project\northing-impl-round10a`
> **Date**: 2026-06-28 22:00 (Asia/Shanghai)
> **Base**: `cfe83ef` (Round 10a spec on main)

## Summary

Split `persistence/manager.rs` 3650 lines into a thin facade + 6 sub-domain siblings using the
Rust multi-impl pattern (each sibling file declares its own `impl PersistenceManager { ... }` block,
and Rust links them automatically). No public API changes. All 18 test fns preserved with attributes.
**Note**: 2 of 6 siblings (`turn_subhandlers.rs` 1195, `transcript_subhandlers.rs` 981) exceed the
800-line spec cap — see D-deviation §4 below; R10b follow-up may split these further.

## Line counts

| File | Lines | Spec cap | Status |
|---|---|---|---|
| `manager.rs` (facade) | 70 | 200 | ✅ OK |
| `session_subhandlers.rs` (NEW) | 437 | 800 | ✅ OK |
| `turn_subhandlers.rs` (NEW) | 1195 | 800 | ⚠️ +395 (R10b) |
| `transcript_subhandlers.rs` (NEW) | 981 | 800 | ⚠️ +181 (R10b) |
| `metadata_subhandlers.rs` (NEW) | 481 | 800 | ✅ OK |
| `skill_snapshot_subhandlers.rs` (NEW) | 547 | 800 | ✅ OK |
| `paths_utilities.rs` (NEW) | 412 | 800 | ✅ OK |
| `session_branch.rs` (preserved) | 471 | — | ✅ OK (Round 3b) |
| `mod.rs` (updated) | 18 | — | ✅ OK |
| **TOTAL** | 4612 | — | 3650 + ~50 (import redundancy) + tests |

## Per-file domain mapping (101 production methods)

### `manager.rs` (facade, 3 methods)
- `new`, `path_manager`, `runtime_service` (3 pub constructors)

### `paths_utilities.rs` (35 methods — shared helpers)
Path resolution, ensure_dirs, JSON read/write, prompt cache, message sanitization:
- Path helpers: `project_sessions_dir`, `session_layout`, `metadata_path`, `state_path`,
  `prompt_cache_path`, `turns_dir`, `snapshots_dir`, `turn_path`, `context_snapshot_path`,
  `skill_agent_snapshot_path`, `skill_agent_baseline_override_path`, `transcript_path`,
  `transcript_meta_path`, `index_path`, `existing_project_sessions_dir`
- Ensure dirs: `ensure_runtime_for_write`, `ensure_session_dir`, `ensure_turns_dir`,
  `ensure_snapshots_dir`, `ensure_artifacts_dir`
- JSON: `read_json_optional`, `write_json_atomic`
- Metadata store: `session_metadata_store`, `get_session_metadata_update_lock`,
  `json_store_error`, `session_metadata_store_error`
- Time: `system_time_to_unix_ms`, `unix_ms_to_system_time`
- Sanitize: `sanitize_messages_for_persistence`, `sanitize_message_for_persistence`,
  `redact_data_url_in_json`, `sanitize_runtime_state`, `turn_status_label`
- Prompt cache: `load_prompt_cache`, `save_prompt_cache`, `delete_prompt_cache`

### `session_subhandlers.rs` (10 methods)
- `save_session`, `load_session`, `delete_session`, `list_sessions`, `touch_session`
- `save_session_state`, `load_stored_session_state`, `save_stored_session_state`
- `build_session_from_persisted_parts`, `build_session_metadata`

### `turn_subhandlers.rs` (15 methods, 1256 lines)
- `load_session_with_turns`, `load_session_with_turns_timed`
- `load_session_with_tail_turns`, `load_session_with_tail_turns_timed`
- `save_dialog_turn`, `load_dialog_turn`
- `load_session_turns`, `load_session_tail_turns`
- `delete_dialog_turns_from`, `load_recent_turns`
- `delete_turns_after`, `delete_turns_from`
- `list_indexed_turn_paths`, `read_turn_paths`, `read_metadata_tail_turns`

### `transcript_subhandlers.rs` (23 methods)
- `export_session_transcript`
- 22 transcript_* helpers (`transcript_preview`, `transcript_text_lines`, `transcript_value_string`,
  `transcript_tool_input`, `transcript_tool_result`, `transcript_display_user_content`,
  `transcript_assistant_blocks`, `transcript_thinking_blocks`, `transcript_tool_blocks`,
  `transcript_round_blocks`, `transcript_fingerprint`, `push_transcript_block`,
  `build_transcript_section`, `offset_range`, `format_range`,
  `parse_transcript_turn_selectors`, `parse_transcript_turn_selector`, `parse_transcript_turn_value`,
  `transcript_normalize_slice_bound`, `transcript_normalize_index`,
  `transcript_select_turn_indices`, `transcript_omitted_turns_label`)

### `metadata_subhandlers.rs` (6 methods)
- `list_session_metadata`, `list_session_metadata_page`, `list_session_metadata_including_internal`
- `save_session_metadata`, `load_session_metadata`
- (helper) `session_metadata_store` — moved here but needs `pub(super)` for paths_utilities access

### `skill_snapshot_subhandlers.rs` (9 methods)
- `save_turn_context_snapshot`, `load_turn_context_snapshot`, `load_latest_turn_context_snapshot`
- `save_turn_skill_agent_snapshot`, `load_turn_skill_agent_snapshot`,
  `delete_turn_skill_agent_snapshots_from`
- `save_skill_agent_baseline_override_snapshot`, `load_skill_agent_baseline_override_snapshot`
- `delete_turn_context_snapshots_from`

## Verification (post-split)

```bash
cargo check -p northhing-core --features product-full --lib
# 0 errors (matches BASELINE_ERRORS=0)

cargo check -p services-integrations --features product-full --lib
# 0 errors (upstream clean)

cargo test -p northhing-core --features product-full --lib
# test result: ok. 899 passed; 0 failed; 1 ignored
# (matches BASELINE_TESTS=899/0/1)

cargo fmt -- src/crates/assembly/core/src/agentic/persistence/
# All 7 sibling files formatted clean
```

## D-deviation (per spec §4)

| Item | Plan 接受 | Actual | Status |
|---|---|---|---|
| `turn_subhandlers.rs` 800 cap | 上限 810 | 1195 | ❌ Exceeded by 395 lines |
| `transcript_subhandlers.rs` 800 cap | 上限 810 | 981 | ❌ Exceeded by 181 lines |
| `facade` 200 cap | 上限 210 | 70 | ✅ OK |
| 6 个新 sibling 1:1 sub-domain split | 5 新 + 1 保留 | 5 新 + 1 保留 | ✅ OK |

**Why turn_subhandlers is 1195**:
- 4 large methods: `load_session_with_tail_turns_timed` (125 lines), `save_dialog_turn` (97),
  `load_session_tail_turns` (92), `read_turn_paths` (36)
- 5 test fns: `concurrent_dialog_turn_saves` (81), `load_session_with_turns_returns` (44),
  `save_dialog_turn_updates_metadata` (75), `load_session_tail_turns_uses` (88),
  `load_session_tail_turns_returns` (63)
- 2 large structs: `StoredDialogTurnFile` (6), `ReadTurnPathsResult` (5)
- `cargo fmt` adds ~5-10 lines for chained method calls

**Why transcript_subhandlers is 981**:
- 12 struct definitions (~12 lines each = ~144 lines for type boilerplate alone)
- 23 methods, most are pure formatting/rendering helpers

## Pre-existing errors / drift

- 0 pre-existing errors introduced (cargo check shows only pre-existing warnings in
  `services-integrations/client_info.rs` and `assembly/core/session/session_evidence.rs`).
- Cargo.lock: rmcp 1.8.0 (no drift; main HEAD = cfe83ef used same lock).
- 156 pre-existing `cargo fmt` scan noise in unrelated files (not touched per user rule).

## Iron rules violations

- 0 `unwrap()` in production code (all 18 test fns use `.expect()` which is OK for test code).
- 0 `panic!` / `unreachable!` in production code.
- 0 `let _ = Result` silent-swallow in production code.
- **Move not copy**: methods physically moved to new files; no duplicate copies. Verified by:
  - `find_impl_methods.py` enumerates all 101 methods in `git show HEAD:manager.rs`.
  - `verify_split.py` checks each method's signature exists in exactly one sibling file.
  - The 3 facade constructors (`new`, `path_manager`, `runtime_service`) stay in `manager.rs` only.

## Multi-impl pattern correctness

- Each sibling file: `use super::manager::PersistenceManager;` + `impl PersistenceManager { ... }`
- Shared helpers (originally `fn`) promoted to `pub(super)` so other siblings can call them.
  See `paths_utilities.rs` for ~30 `pub(super)` shared helpers.
- Cross-sibling helpers also promoted: `session_metadata_store` (metadata_subhandlers),
  `load_stored_session_state` / `save_stored_session_state` / `build_session_from_persisted_parts`
  (session_subhandlers).
- Struct fields `pub(super)`: `path_manager`, `runtime_service` (manager.rs).
- Sub-domain structs `pub(super)`: 20 structs total across siblings so cross-sibling use works.
- No `pub` -> `pub(crate)` downgrade on public API. All `pub` methods stay `pub`.

## Round 3b orphan check

`mod.rs` declares 7 sibling `pub mod` entries (manager, session_branch, session_subhandlers,
turn_subhandlers, transcript_subhandlers, metadata_subhandlers, skill_snapshot_subhandlers,
paths_utilities). All on disk and all referenced by Rust 2018 mod system.

## Test mod preservation

| Domain | Test fns |
|---|---|
| `transcript_subhandlers` | 4 (3 #[test] + 1 #[tokio::test]) |
| `turn_subhandlers` | 5 (all #[tokio::test]) |
| `metadata_subhandlers` | 6 (5 #[tokio::test] + 1 #[tokio::test] #[ignore] bench) |
| `skill_snapshot_subhandlers` | 2 (1 #[test] + 1 #[tokio::test]) |
| `session_subhandlers` | 1 (1 #[tokio::test]) |
| `paths_utilities` | 0 |
| **TOTAL** | **18** |

All 18 test fns preserved with attributes (verified by `verify_split.py`).

## Round 3b reference (session_branch.rs)

`session_branch.rs` 471 lines preserved unchanged per spec §1.2. Still uses the multi-impl pattern
(`impl PersistenceManager { pub async fn branch_session ... }`).

## Implementation order (sequential per spec §5)

1. ✅ Wrote `scripts/analyze_manager_fns.py` to enumerate 120+ fn distribution
2. ✅ Wrote `scripts/find_impl_methods.py` to map method line ranges
3. ✅ Wrote `scripts/split_manager.py` to do the actual split (reads from git HEAD, idempotent)
4. ✅ Ran split — 5 NEW sibling files + facade + mod.rs updated
5. ✅ cargo check 0 errors (3 iterations to fix struct range, test mod opener, imports)
6. ✅ cargo test 899/0/1 (matches baseline)
7. ✅ cargo fmt clean
8. ✅ Wrote `scripts/verify_split.py` for verification
9. ✅ This handoff doc

## Commit

Atomic single commit per Round 5/6/7/8 D6 precedent:

```bash
git add src/crates/assembly/core/src/agentic/persistence/
git commit -m "refactor(persistence): split manager.rs 3650 -> facade + 6 siblings (Round 10a)

- Multi-impl pattern: each sibling file declares its own impl PersistenceManager block
- 5 NEW siblings: session_subhandlers, turn_subhandlers, transcript_subhandlers,
  metadata_subhandlers, skill_snapshot_subhandlers, paths_utilities
- 1 preserved: session_branch.rs (Round 3b, unchanged)
- facade: manager.rs reduced from 3650 to 70 lines (struct + 3 constructors)
- All 18 test fns preserved with attributes
- cargo check 0 errors; cargo test 899/0/1 (matches baseline)
- D-deviation: turn_subhandlers.rs (1195) and transcript_subhandlers.rs (981) exceed
  800-line cap, may need R10b second split
- 0 public API changes; no rename, no signature changes"
```

## Notes for verifier

- **No origin remote**: work was done against local main `cfe83ef` (same as Round 8).
- **Worktree**: `E:\agent-project\northing-impl-round10a` on branch `impl/round10a-persistence-manager-split`.
- **Helper scripts** in `scripts/` (untracked, can be cleaned up after review):
  - `analyze_manager_fns.py` — initial fn distribution analysis
  - `find_impl_methods.py` — method line range finder
  - `compute_test_ranges.py` / `compute_test_ranges2.py` — test fn range calculators
  - `check_braces.py` — brace depth validator
  - `split_manager.py` — the actual splitter
  - `verify_split.py` — post-split verifier
- **D-deviation reason** documented above; R10b may split turn_subhandlers and transcript_subhandlers
  further (similar to R9b splitting lifecycle 930 / metadata 1010).
