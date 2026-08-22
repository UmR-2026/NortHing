# Task A1 Fix Brief - Round 1

Fix target: worktree `E:\agent-project\northing\.worktrees\growth-a1`, branch `feat/growth-a1`, current HEAD `88fee23`.
You may ONLY edit `src/agentic/src/state.rs` (and only that file). Do not touch ports.rs or anything else.

## Background

Task A1 implemented `ports.rs` + `state.rs` (commit `88fee23`). Review returned CHANGES REQUESTED with one Critical finding. Your job is to fix ONLY that finding, re-run validation, amend-or-new-commit, and update the report.

## The Critical finding (from task-a1-review.md)

In `load_state`, the legacy-migration branch (blob missing) reads the 4 legacy keys via `store.get_legacy_kv(...)`. The current code uses `if let Ok(Some(...))` which SILENTLY IGNORES an `Err` from `get_legacy_kv` - it neither logs a warning nor returns Default.

This violates the spec constraint (brief §3.2 item 3 + the resolved-ambiguity "growth path warn-only: load_state ANY exception returns Default + tracing::warn!, never propagates"):
- Brief §3.2 item 3: "读取端口报错 -> 返回 Default 并 warn（成长路径 warn-only，绝不向上传播）" = "port read error -> return Default + warn (growth path warn-only, never propagate)".
- Resolved ambiguity: "load_state 任何异常都返回 Default + tracing::warn!" = "load_state ANY exception returns Default + tracing::warn!".

A `get_legacy_kv` Err during migration is a port read error / exception, so it must warn + return Default.

## Required fix

In `load_state`'s migration branch, handle the `Err` case of each `get_legacy_kv` call explicitly: on `Err(e)`, emit `tracing::warn!` with the key name and error, and return `GrowthState::default()`. Do NOT silently continue.

Apply to all 4 legacy key reads (`LEGACY_KEY_DISTILL_TURNS`, `LEGACY_KEY_DISTILL_HIT_TURNS`, `LEGACY_KEY_DISTILLER_PAUSED`, `LEGACY_KEY_DREAM_LAST_SWEEP`).

Keep the existing behavior for:
- `Ok(Some(value))`: parse and fill (parse failure -> 0 / false as before).
- `Ok(None)`: skip (key absent is not an error).

## Required new test

Add a test in `state.rs` `#[cfg(test)] mod tests` covering: `get_blob` returns `Ok(None)` (so migration runs) but `get_legacy_kv` returns `Err`. Assert `load_state` returns `GrowthState::default()` and does not panic. This requires extending the `FakeStore` so the legacy-read path can be forced to error independently of `get_blob` (e.g. a separate `force_legacy_error` flag, or make `legacy` reads error when a flag is set). Name it something like `test_migration_port_error_on_legacy`.

## Hard constraints (unchanged from original brief)

- ONLY edit `src/agentic/src/state.rs`.
- No IO, no new deps, no `cargo fmt` (hand-align 4-space, match error.rs style).
- English-only comments/logs, no emoji.
- File must stay < 800 lines.
- Do not reformat existing code beyond what the fix requires.

## Validation (must run, paste raw output into the updated report)

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

Expected: all tests pass (was 15, should be 16 after adding the new test); `cargo check` clean (0 warnings).

## Commit + report

- Amend the existing commit `88fee23` OR create a new commit on top - your choice, but keep the commit message `feat(growth): define growth ports and persisted state with legacy key migration` if amending, or use `fix(growth): handle legacy-key port errors as warn-only in load_state` if new commit. Before committing, `git status --short` to confirm only state.rs changed.
- Update the report at `E:\agent-project\northing\.superpowers\sdd\task-a1-report.md`: status (DONE), updated line count of state.rs, the new raw validation output (test names + pass count), `git log --oneline -2`, `git status --short`, and a note that this is the post-fix round addressing the review Critical finding.
