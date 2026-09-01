# Task 1 (W3-1): r2#5 — Ghost Session Rollback on Persistence Failure Report

## What was implemented

- **`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs:179-194`**:
  Wrapped `self.persistence_manager.save_session(&session_storage_path, &session).await` in error check. On `Err(err)`:
  - Removes `session_id` from `self.sessions`
  - Removes `session_id` from `self.session_workspace_index`
  - Removes in-memory cache/state from `self.context_store`, `self.turn_skill_agent_snapshot_store`, and `self.file_read_state_store`
  - Emits `warn!("Failed to persist new session, rolled back in-memory state: session_id={}, error={}", session_id, err);`
  - Returns `Err(err)`
- **Key-collision confirmation (Spec item 2)**:
  Confirmed in code: all callers of `create_session_with_id_and_details` supply either `None` (where `Session::new` generates a brand new `Uuid::new_v4().to_string()`) or a freshly generated unique ID (e.g. `so_handlers.rs` `/btw` child session, dialog turn subagent session). The ID does not collide with existing active sessions prior to insertion, so removing `session_id` on rollback only affects the newly inserted session.
- **`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/session_manager_lifecycle_tests_rollback_delete.rs:332-384`**:
  Added unit test `create_session_persistence_failure_rolls_back_in_memory_state` asserting that when persistence fails, `Err` is returned, `sessions` map and `session_workspace_index` are clean with 0 entries, `get_session` returns `None`, and context store has no leftover data.

## 复用侦察 (Reuse Reconnaissance)

- Searched `src/crates/assembly/core/src/agentic/session/` for existing test helpers and lifecycle tests.
- Reused `TestWorkspace` and `test_manager(persistence_manager)` fixtures from `session_manager_tests.rs`.
- Reused existing test file `session_manager_lifecycle_tests_rollback_delete.rs` which already tests rollback and delete logic.
- Simulated persistence failure naturally by creating a regular file at the home directory path in `TestWorkspace`, inducing an I/O error during persistence directory creation without introducing synthetic mocks.

## Verification

### 1. Focused Unit Test
Command:
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib create_session_persistence_failure_rolls_back_in_memory_state
```
Output:
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib test) generated 18 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 17 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 33.41s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 1 test
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::create_session_persistence_failure_rolls_back_in_memory_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1052 filtered out; finished in 0.01s
```

### 2. Session Lifecycle Test Suite
Command:
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full --lib session_manager_lifecycle_tests
```
Output:
```text
warning: `northhing-core` (lib test) generated 18 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 17 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.38s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 17 tests
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::build_messages_from_turns_skips_model_invisible_turns ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_session_state_reset::reset_session_state_if_processing_resets_the_matching_turn ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_session_state_reset::update_session_state_for_turn_if_processing_ignores_a_newer_turn ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_session_state_reset::update_session_state_for_turn_if_processing_updates_matching_turn ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_session_state_reset::reset_session_state_if_processing_ignores_a_newer_turn ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::delete_session_removes_workspace_cache_entry ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_ephemeral_lineage::ephemeral_child_session_is_kept_in_memory_without_persisting ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::create_session_persistence_failure_rolls_back_in_memory_state ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::restore_session_resets_processing_state_without_marking_unread_completion ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_ephemeral_lineage::persist_session_lineage_updates_structured_relationship_and_clears_legacy_projection ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_to_empty_history_clears_last_user_dialog_agent_type ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::start_dialog_turn_with_existing_context_persists_turn_and_snapshot ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_ephemeral_lineage::append_completed_local_command_turn_persists_without_model_context ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_restore_dialog::restore_session_sanitizes_pre_cutoff_listing_diff_snapshot ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_context_deletes_persisted_turns_from_target ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::session_manager_lifecycle_tests_rollback_delete::rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff ... ok
test agentic::session::session_manager_tests::session_manager_lifecycle_tests::continuity_selfcheck::continuity_selfcheck_seed_restore_diff ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 0.19s
```

### 3. Workspace Check
Command:
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check --workspace
```
Output:
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 29s
```

## Compile errors encountered

- `GNU toolchain gcc missing for ring/sqlite`: Fixed at toolchain selection layer (`stable-x86_64-pc-windows-msvc` per AGENTS.md desktop toolchain guidelines).
- `E0433 on cargo check without features`: Fixed at mechanism layer by adding `--features product-full` for test execution on `northhing-core`.

## Self-review findings

- No global constraints violated.
- Commit created cleanly: `d82a074 fix(assembly/core): rollback in-memory session insert on persistence failure (r2#5)`.
- No files under `.superpowers/` were staged or committed.

## Concerns

- None.
