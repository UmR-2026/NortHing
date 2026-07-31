# Task 7 Fixnote: missing io_tests module (E0583)

Date: 2026-08-01
Branch: `fix/backend-debug-0731`

## Problem

`src/apps/desktop/src/app_state/settings/io.rs:215` declares `mod io_tests;`
(child module of `io`), but the file was missing → `cargo test -p northhing --lib`
failed with E0583.

A stray draft existed at the wrong path `settings/io_tests.rs` (untracked,
leftover from the Task 7 implementer attempt); rustc resolves `mod io_tests;`
inside `io.rs` to `settings/io/io_tests.rs`, not `settings/io_tests.rs`.

## Fix

- Created `src/apps/desktop/src/app_state/settings/io/io_tests.rs` (new
  `io/` subdirectory), covering the Task 7 brief §4 list plus the fail-closed
  requirement:
  1. `concurrent_updates_preserve_all_writes` — 10 concurrent
     `update_app_settings_at` upserting distinct providers → all 10 survive
     (single-writer lock regression).
  2. `update_with_err_closure_does_not_write_file` — closure returns Err →
     on-disk bytes unchanged (seed known content first, byte-compare after).
  3. `second_write_keeps_previous_version_in_bak` — two saves → `.bak` holds
     v1, main holds v2, **no `.tmp` residue** in the dir.
  4. `leftover_tmp_file_does_not_break_main_file` — simulated crash residue
     never affects the main file.
  5. `load_dedup_migration_still_persists` — seeded duplicate providers →
     dedup on load, result persisted via the atomic writer, first entry kept.
  6. `load_parse_failure_returns_err` — corrupt JSON → Err (fail-closed).
- Removed the stray misplaced `settings/io_tests.rs` (content superseded by
  the correctly-located file).
- No production code changed.

## Verification

`cargo test -p northhing --lib settings` — passed:

```
running 59 tests
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::load_dedup_migration_still_persists ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out
```

`cargo check -p northhing` — clean (Finished, no errors; warnings are
pre-existing `northhing-core` ones).

## Notes for reviewer

- Pre-existing (Task 7 implementer state, not introduced here): warning
  `function save_app_settings is never used` at `io.rs:135` — callers were
  migrated to `update_app_settings` and the retained low-level public wrapper
  has no non-test caller. The brief requires keeping the low-level API; triage
  whether the public wrapper needs a caller or an `#[allow(dead_code)]`/removal
  in Task 7 review (out of fix scope: no production code may be touched).
- `settings/tests.rs`'s `dedup_providers_on_load_*` unit tests remain green;
  this file adds the disk-level coverage the brief asked for.
