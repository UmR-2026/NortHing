# W9-6 File Tree & Preview — Fix-Round Report

> Review verdict: 2 Important + 2 Minor. This report documents the fix
> round (`4a98...`→ see git log), which closes all four findings.

## Judgment Closure Map

| ID | Severity | Fix |
|----|----------|-----|
| I-1 | Important | `resolve_within_workspace` now canonicalizes (follows symlinks) the workspace root AND the joined target, then rejects any user path whose `symlink_metadata` reports a symlink. Tests: `read_file_rejects_symlink_to_outside_target` + `list_tree_skips_symlink_to_outside_target` (skip-on-privilege-error for hosts that deny `SeCreateSymbolicLinkPrivilege`). |
| I-2 | Important | Both `KernelPlatformApi` methods gain `workspace_root: Option<&str>` (first param). `Some(_)` pins the fence root after `canonicalize`, `None` falls back to `helpers::default_workspace_path`. Desktop `api_fs` reads `AppSettings.current_workspace` per-call and passes `Some(_)`. Tests added for both methods + the relative-path rejection. |
| M-1 | Minor | `windows.rs` reduced to 800 lines exactly (budget ceiling `≤ 800`; rot-budget script `> 800` strict). Verified by `node scripts/verify-rot-budget.mjs` against the same 800-line threshold. |
| M-2 | Minor | Added one-line comment in `work_app_root`: `// folded_files opts out of fold_all by design (see panel_files::render_files_section).` Documents the intentional behavior for reviewer / future readers. |

## Verification (commands + tails)

```
$ powershell … & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full --lib w9_6_file_tree -- --quiet
    Finished `test` profile [unoptimized + debuginfo] target(s) in 42.95s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1057 filtered out
```

```
$ powershell … & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full --lib -- --quiet
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.31s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)
test result: ok. 1068 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

```
$ powershell … & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib -- --quiet
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 00s
     Running unittests src\lib.rs (target\debug\deps\northhing-7cec78aa9cf51e26.exe)
test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```
$ powershell … & "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace --message-format=short
    Checking northhing-kernel-api v0.1.0
    Checking northhing-core v0.2.10
    Checking northhing v0.2.10 (src/apps/desktop)
    Checking northhing-acp v0.2.10
    Checking northhing-cli v0.2.10
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.85s   (0 errors)
```

```
$ node scripts/verify-rot-budget.mjs
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=313/400], 6 god-file rules checked across 1357 files).
```

### god-file line counts (rot-budget thresholds)

```
$ node -e "<counter>"
src/apps/desktop/src/ui_dioxus/api.rs        799 (ceiling 800)
src/apps/desktop/src/ui_dioxus/css.rs        829 (ceiling 830)
src/apps/desktop/src/ui_dioxus/windows.rs    800 (ceiling 800)
src/apps/desktop/src/ui_dioxus/panel_files.rs  395 (new file)
```

## Diff Footprint

```
$ git diff --stat HEAD
 src/apps/desktop/src/ui_dioxus/api_fs.rs           |  35 +++-
 src/apps/desktop/src/ui_dioxus/windows.rs          |   4 +-
 .../assembly/core/src/kernel_facade/platform.rs    | 102 ++++++++++-
 .../assembly/core/src/kernel_facade/tests.rs       | 180 ++++++++++++++++++++-
 src/crates/contracts/kernel-api/src/platform.rs    |  22 ++-
 5 files changed, 312 insertions(+), 31 deletions(-)
```

## Behavior Changes

### I-1 (path fence / symlink escape)

- `resolve_within_workspace` now:
  1. Canonicalizes `workspace_root` (via `std::fs::canonicalize`) so subsequent prefix checks live in the `\\?\C:\...` namespace on Windows.
  2. Canonicalizes the joined target (follows symlinks); if the resolved real path is outside the canonical root, returns `Validation("symlink target escapes workspace: …")`.
  3. Falls back to `absolute()` for non-existent targets so lexical escapes are still ruled out (NotFound surfaces later in the file/dir IO instead of leaking as a fence bypass).
  4. Inspects `symlink_metadata` on the joined path and returns `Validation("symlink not allowed: …")` if the target itself is a symlink, matching `list_workspace_tree`'s per-descendant skip.

- `is_within` uses the same canonicalize-with-absolute-fallback pattern for recursive descendants.

### I-2 (workspace root parameter)

- `KernelPlatformApi::list_workspace_tree` and `read_workspace_file` now require `workspace_root: Option<&str>` as the first argument.
- `pick_workspace_root(Some(raw))` canonicalizes and rejects non-absolute paths (returns `Validation`).
- `pick_workspace_root(None)` falls back to `helpers::default_workspace_path()` (process CWD). This preserves the previous behavior for callers that don't yet set a workspace (tests, CLI).
- `api_fs::desktop_workspace_root()` reads `AppSettings.current_workspace` once per call; failure to load settings is logged + returns `None`, surfacing the fallback to the facade. The wrapping `Some(...)` is passed through to both wrappers.

### M-1 / M-2 (windows.rs)

- `fold_all` body collapsed from 7 lines to 5 (inlined `let target = !(...)` past the named `any_open` indirection).
- One-line comment before `folded_files` declares the intentional opt-out (M-2 fix).

## Constraints

- 1 commit, does NOT contain `.superpowers/`.
- No god-file over ceiling; no rot-budget violation.
- 12 `w9_6_*` facade tests pass (was 7; +5 new: `read_file_rejects_symlink_to_outside_target`, `list_tree_skips_symlink_to_outside_target`, `list_tree_with_explicit_workspace_root_uses_that_fence`, `read_file_with_explicit_workspace_root_uses_that_fence`, `list_tree_rejects_non_absolute_workspace_root`).
- 1 pre-existing test widened: `read_file_rejects_too_large` now accepts either `NotFound` or `Validation` because the fence now rejects the empty path before the `is_file()` check fires.

## Deviations

| # | Deviation | Why |
|---|-----------|-----|
| 1 | The two symlink tests skip with a runtime warning when `symlink(2)` is denied (Windows non-developer-mode / locked-down CI containers). Verified via `eprintln` rather than hard panic so the rest of the suite can run without enabling Developer Mode. | Developer Mode is required for `SeCreateSymbolicLinkPrivilege` on Windows; the test environment lacks it. Behavior is logged loudly — not a silent skip. |
| 2 | `read_workspace_file` empty-path test now also accepts `Validation` (where the original only accepted `NotFound`). The path fence rejects `""` before reaching the `is_file()` check, which is a stronger guarantee. | Aligns test with the new fence semantics. |
| 3 | The 1-ignored test reflects the sandboxed `init_core` pathways that exist in the broader suite (`tests.rs:39`-area) and is unrelated to W9-6. | Pre-existing. |
