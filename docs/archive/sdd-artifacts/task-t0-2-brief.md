# Task T0-2 Brief: fix inverted bash availability check in run_script

## Source
- backend-roadmap.md T0-2; evidence full-review-2026-08-16.md F-17.
- Verified by orchestrator 2026-08-17: bug still present as described.

## Location
- File: `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs`
- Lines 218-234, the `"bash" =>` match arm.

## Bug
```rust
"bash" => {
    if utilities::which_exists("bash") {
        return Ok(err_response(
            // "bash is not on PATH" ...
```
The condition is inverted: when bash IS on PATH it returns a "bash is not on PATH" error; when bash is absent it proceeds to spawn it. Net effect: `script_type='bash'` is unusable on every platform.

## Required fix
Negate the guard: `if !utilities::which_exists("bash") {`. Nothing else changes — error text, hint, response shape, and all other match arms stay byte-identical.

## Constraints
- Single-file, single-condition edit. Do NOT refactor, reformat, or touch other arms/files.
- Logs/errors stay English-only.

## Verification (run and paste raw output into report)
1. `cargo check -p northhing-core`
2. Search for existing tests covering run_script bash arm (`grep -rn "run_script" src/crates/assembly/core/tests/ src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/ --include *.rs | grep -i test`); if a relevant test module exists, run it with `cargo test -p northhing-core <name>`; if none exists, say so explicitly in the report (do not write one).

## Report
Write to `.superpowers/sdd/task-t0-2-report.md`: diff summary, verification commands + raw output, any concerns. Status line first: DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED.
