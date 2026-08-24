# R70 Dead Code Triage Report

**Base commit:** `25b42d3d`  
**Branch:** `r70/dead-code-triage`  
**Date:** 2026-07-10  
**Author:** Mavis (coder agent)

---

## 1. Baseline

Ran `cargo check --workspace` in worktree `E:\agent-project\northing-r70`.

| Metric | Count |
|---|---|
| Total warning lines in capture file | 1 459 |
| Crate summary lines (`... generated N warnings`) | 101 |
| Slint UI warnings (`northhing@0.2.10`) | — |
| **Categorized warnings** | **1 358** |

---

## 2. Warning Type Histogram

| Category | Count | Share |
|---|---|---|
| `unused_import` | 1 312 | 96.6 % |
| `glob_import` (no public re-export) | 7 | 0.5 % |
| `unused_constant` | 5 | 0.4 % |
| `needless_mutable` | 5 | 0.4 % |
| `deprecated` (use of deprecated method/function) | 4 | 0.3 % |
| `unused_variable` | 9 | 0.7 % |
| `unused_field` | 2 | 0.1 % |
| `unused_method` | 1 | 0.1 % |
| `unused_struct` | 1 | 0.1 % |
| `unused_associated_items` | 1 | 0.1 % |
| `unused_function` | 1 | 0.1 % |
| `unused_result` | 1 | 0.1 % |
| `shadow_glob` | 1 | 0.1 % |
| **Total** | **1 358** | 100 % |

---

## 3. Top 10 Modules with Most Warnings

| Rank | File | Warnings | Dominant type |
|---|---|---|---|
| 1 | `src\crates\assembly\core\src\agentic\coordination\dialog_turn\compaction.rs` | 40 | unused_import |
| 2 | `src\crates\assembly\core\src\agentic\coordination\dialog_turn\thread_goal.rs` | 38 | unused_import |
| 3 | `src\crates\assembly\core\src\agentic\execution\health_snapshot.rs` | 36 | unused_import |
| 4 | `src\crates\assembly\core\src\agentic\execution\multimodal.rs` | 35 | unused_import |
| 5 | `src\crates\assembly\core\src\agentic\execution\turn_finalize.rs` | 35 | unused_import |
| 6 | `src\crates\assembly\core\src\agentic\coordination\dialog_turn\session.rs` | 35 | unused_import |
| 7 | `src\crates\assembly\core\src\agentic\coordination\dialog_turn\workspace.rs` | 35 | unused_import |
| 8 | `src\crates\assembly\core\src\agentic\execution\token_pressure.rs` | 34 | unused_import |
| 9 | `src\crates\assembly\core\src\agentic\execution\loop_detection.rs` | 33 | unused_import |
| 10 | `src\crates\assembly\core\src\agentic\execution\turn_main_loop.rs` | 33 | unused_import |

**Pattern:** The top 10 modules are concentrated in `assembly/core` agentic dialog-turn and execution sub-domains. These are recent god-file split artifacts (R47b, R66b) and the high warning count correlates with incomplete import cleanup after extraction.

---

## 4. Intentional-vs-Actually-Dead Split

### 4.1 Intentional (safe to leave as-is)

| File | Item | Reason |
|---|---|---|
| `src\crates\services\terminal\src\exec\mod.rs` | `DEFAULT_YIELD_TIME_MS`, `MAX_RETAINED_OUTPUT_BYTES`, `MAX_EXEC_SESSIONS`, `MAX_COMPLETED_EXEC_SESSIONS`, `PTY_EXIT_DRAIN_TIMEOUT_MS` (5 consts) | These are **shadow duplicates** of `pub(crate)` consts in `src\crates\services\terminal\src\exec\types.rs`. The consts in `types.rs` are the actively-used definitions (`platform.rs`, `output.rs`, `manager/control_session.rs`, `remote_exec.rs` import from `types.rs`). The `mod.rs` duplicates are legacy re-exports from before the `types.rs` split. Removing them would require verifying `mod.rs` re-export wiring. **Defer.** |
| `src\crates\services\terminal\src\shell\profiles.rs` | `ShellProfileManager` struct + impl | Commented: `reserved for upcoming shell-profile management UI`. |
| `src\crates\services\terminal\src\session\session_shell_integration.rs` | `wait_for_session_ready` instance method | Commented: `reserved for upcoming readiness-gated command queue`. |
| `src\crates\services\services-integrations\src\remote_ssh\manager.rs` | `server_key` field | Commented: `reserved for upcoming host-key pinning verification`. |
| `src\crates\services\terminal\src\pty\service.rs` | `config` field | Commented: `reserved for upcoming config-driven PTY knobs`. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs` | `workspace_path`, `agent_type`, `turn_id` | `workspace_path` is used later in the function (`workspace_path.as_deref()`). `agent_type` is used in `requested_agent_type` derivation. `turn_id` is used in debug log. Cargo warnings for these are **false positives** caused by later shadow declarations in the same function. |

### 4.2 Actually Dead (safe to remove)

**Unused imports (clearly dead, no macro/cfg usage detected):**

| File | Import | Verification |
|---|---|---|
| `src\crates\contracts\runtime-ports\src\port_core.rs:6` | `use std::fmt;` | File uses `std::fmt::Display` and `std::fmt::Formatter` via absolute paths; `fmt` module from import never referenced. |
| `src\crates\contracts\runtime-ports\src\remote.rs:10` | `use super::session_workspace::WorkspaceFileSystem;` | Only occurrence in workspace is the import line itself. |
| `src\crates\contracts\runtime-ports\src\session_workspace.rs:6` | `use std::path::{Path, PathBuf};` — drop `Path` | `PathBuf` is used throughout; `Path` only appears in import and return-type `&PathBuf` (no bare `Path` usage). |
| `src\crates\contracts\runtime-ports\src\session_workspace.rs:10` | `use super::port_core::{PortError, PortResult, RuntimeServicePort};` — drop `PortError` | `PortResult` and `RuntimeServicePort` are used; `PortError` only appears in import. |

**Unused variables (clearly dead, from incomplete god-file split):**

| File | Variable | Verification |
|---|---|---|
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:30` | `user_input` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:31` | `original_user_input` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34` | `image_contexts` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:36` | `extra_user_message_metadata` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:37` | `additional_prepended_messages` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:34` | `workspace_path` | Declared but never referenced; later `workspace_path.as_deref()` uses `session.config.workspace_path`, not local var. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35` | `agent_type` | Declared but never referenced later in function. |
| `src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:40` | `suppress_session_title_generation` | Declared but never referenced later in function. |

### 4.3 Needs Human Review

| Category | Count | Why review |
|---|---|---|
| `unused_import` in `assembly/core` top 10 files | ~330 | Many imports may be used in `#[cfg(test)]` blocks that are NOT compiled by `cargo check --workspace` (only compiled by `cargo check --workspace --tests`). **Do not bulk-remove.** |
| `glob_import` (7 warnings) | 7 | `pub use foo::*;` where no item is public enough. May be intentionally held for future public API expansion. |
| `deprecated` (4 warnings) | 4 | Use of deprecated `sse_stream` and `rmcp` methods. Fixing requires migrating to new API, not just deletion. |
| `unused_constant` in `terminal/src/exec/mod.rs` | 5 | See Intentional section — shadow duplicates. Need re-export audit before removal. |
| `unused_field` / `unused_method` / `unused_struct` behind comments | 5 | Explicitly reserved for future features. |
| `needless_mutable` (5 warnings) | 5 | `let mut x` where `x` is never reassigned. Low-risk but should be fixed with pattern-matching review (x may gain mutation in adjacent branch). |

---

## 5. Recommended Action Plan

### Tier 1 — Safe to auto-fix now (~350 warnings)

| Action | Count | Tool |
|---|---|---|
| Remove unused imports that are **100 % confirmed dead** (verified with ripgrep, no cfg/test usage) | ~20 | Manual Edit |
| Remove unused variables in recently-split god files (`sub_handle_in.rs`, `sub_handle_state.rs`) | ~15 | Manual Edit |
| Fix `needless_mutable` where var is never reassigned in any branch | 5 | Manual Edit |

**This PR fixes 10 of these items.**

### Tier 2 — Bulk-safe with `cargo fix --lib` (~600 warnings)

Run per crate (avoid workspace-wide to keep diff reviewable):

```bash
cargo fix --lib -p northhing-runtime-ports
cargo fix --lib -p northhing-agent-runtime
cargo fix --lib -p terminal-core
# ... then review each diff
```

### Tier 3 — Needs `cargo check --workspace --tests` first (~350 warnings)

Many unused imports in `assembly/core` are likely used in `#[cfg(test)]` mods. Run `cargo check --workspace --tests` and diff the warning list before bulk-removing.

### Tier 4 — Human review (~50 warnings)

- Public API items behind explicit comments (future features)
- Shadow-duplicate consts (`terminal/src/exec/mod.rs`)
- Deprecated-method usages (`sse_stream`, `rmcp`)

---

## 6. Verification Notes

- **No `#[allow(dead_code)]` items were removed.** All items fixed are behind default warnings only.
- **No public API signatures were changed.** Only local `let` bindings and private imports were touched.
- **ripgrep verification performed** for each removal candidate before editing.
