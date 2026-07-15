# Plan Compliance Checker — Maintainer's Note

## What it is
A LLM-independent verification tool. Parses plan markdown, checks file existence, command exit codes, commit presence, and path consistency. Catches weak-model failures that compile-but-don't-match-plan.

## When to update it
- When `superpowers:writing-plans` plan format changes (add new fields to `Plan` / `Task` / `Step`)
- When a new check kind is needed (e.g., dependency graph between tasks)
- When a workspace layout changes (e.g., workspace root moves)

## How to extend
1. Add new fields to `src/plan.rs`
2. Parse them in `parse_plan`
3. Add a unit test in `tests/plan_parser_test.rs`
4. If the check requires running commands, add to `src/task.rs::check_plan`
5. If it requires output formatting, update `src/report.rs`

## Known limitations
- Markdown parser is line-oriented (pulldown-cmark events). Won't handle deeply nested code blocks inside list items.
- Path consistency detection only checks the `<root>/<first_segment>/...` vs `<root>/src/<first_segment>/...` heuristic. It works when a plan uses `crates/foo` but the workspace actually has `src/crates/foo`. It does NOT help when the plan uses a completely wrong path (e.g. `crates/agent-dispatch/...` when no such directory exists anywhere in the workspace).
- Verify command runner does not enforce timeout — could hang on `cargo build` for very large crates. Use `--skip-slow` or `--force-verify` wisely.

## Future work
- Property-based tests on plan parser (use `proptest`)
- Coverage of `parse_plan` with `cargo-llvm-cov`
- Integration with CI to fail PRs that introduce path mismatches
