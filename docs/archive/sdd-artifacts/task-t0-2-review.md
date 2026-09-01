SPEC: PASS
QUALITY: PASS

# Review for Task T0-2: Fix Inverted Bash Availability Check in run_script

## Summary of Findings

- **Critical:** 0
- **Important:** 0
- **Minor:** 0

## Review Details

### 1. Spec Compliance (SPEC: PASS)
- **Single-condition edit:** The diff modifies exactly one line in `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs` line 219, changing `if utilities::which_exists("bash")` to `if !utilities::which_exists("bash")`.
- **Preserved behavior & errors:** Error text ("bash is not on PATH"), hint, response shape, and all other match arms (`"shell"`, `"powershell"`, `"cmd"`, etc.) remain byte-identical.
- **English-only:** All error/log strings remain in English.
- **Scope restriction:** Only `app_control.rs` was modified; no extra refactoring or formatting was performed.

### 2. Code Quality & Verification (QUALITY: PASS)
- **Semantics verification:** `utilities::which_exists("bash")` returns `true` when `bash` is found on PATH and `false` otherwise. Negating it (`!utilities::which_exists("bash")`) correctly aligns with the intention: return `ErrorCode::NotAvailable` when `bash` is missing, and proceed to spawn when present.
- **No stale assertions:** Verified against codebase search (`"bash is not on PATH"` and test suites) — no existing test asserted the old inverted behavior.
- **Verification evidence:** Implementer ran `cargo check -p northhing-core` via MSVC toolchain (44.07s, 0 errors).
