# Plan Compliance Checker — Crate Layout

> The full crate lives at `tools/plan-compliance-checker/`. It is a
> Cargo workspace member (registered in `Cargo.toml:29`). All 4 phases
> of the implementation plan are complete.

## File inventory

```
tools/plan-compliance-checker/
├── Cargo.toml                       (workspace member; clap, tokio, serde,
│                                     serde_json, pulldown-cmark, anyhow)
├── README.md                        (end-user docs)
├── src/
│   ├── lib.rs                       (module declarations, ~120 B)
│   ├── main.rs                      (clap CLI entry, ~1.6 KB)
│   ├── plan.rs                      (Plan/Task/Step/FilesSpec types + parse_plan,
│   │                                  7.2 KB) ★★★ heaviest file
│   ├── task.rs                      (check_plan + CheckResult/TaskResult enums,
│   │                                  5.0 KB) ★★★
│   ├── path_resolver.rs             (find_workspace_root + detect_path_mismatch,
│   │                                  1.4 KB)
│   ├── git_inspector.rs             (commits_since via git log, 1.7 KB)
│   ├── command_runner.rs            (run_command, 377 B)
│   └── report.rs                    (format_human + format_json, 4.4 KB)
└── tests/
    ├── plan_struct_test.rs          (Plan/Task/Step default + serialize)
    ├── plan_parser_test.rs          (parse_plan end-to-end on inline sample)
    ├── path_resolver_test.rs        (find_workspace_root + detect_path_mismatch)
    ├── git_inspector_test.rs        (commits_since returns ≥ 3 commits)
    ├── task_checker_test.rs         (check_plan on empty plan + hand-built Plan)
    ├── cli_test.rs                  (binary --help + no-args)
    ├── fixture_test.rs              (4 fixtures parse + run)
    └── fixtures/
        ├── good-plan.md             (points at Cargo.toml)
        ├── path-mismatch-plan.md    (points at crates/nonexistent/Cargo.toml)
        ├── missing-file-plan.md     (points at this-file-does-not-exist...)
        └── bad-commit-plan.md       (points at some-path/Cargo.toml, no commit)
```

## Phase status (per `docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md`)

| Phase | Tasks | State |
|---|---|---|
| 1 | Skeleton (manifest, CLI, structs, verify) | **Done** |
| 2 | Markdown parser (5 tasks) | **Done** |
| 3 | Git inspector + task checker + report + CLI wiring | **Done** |
| 4 | Fixtures + actor plan correction + README + tag | **Done** (Task 4.4 bulk-sed skipped per plan note) |

## What this crate is NOT in scope of (per `docs/notes/plan-compliance-checker.md`)

- CI integration
- Lints / clippy overrides
- Property-based tests
- Coverage reporting

## When to extend this crate

| You need to… | Touch these files |
|---|---|
| Add a new check type | `task.rs` (CheckResult variant + check_plan branch) + `report.rs` (3 places: check_to_json, print_check, format_human/json) |
| Add a new plan field | `plan.rs` (struct + parse_plan) + `plan_struct_test.rs` (serialize roundtrip) |
| Add a new CLI flag | `main.rs` (Cli struct + dispatch) + `cli_test.rs` (--help coverage) |
| Add a new fixture | `tests/fixtures/<name>.md` + `fixture_test.rs` (assertions) |
| Change path resolution | `path_resolver.rs` (the only path logic) + `path_resolver_test.rs` |
| Change git history parsing | `git_inspector.rs` (the only git code) + `git_inspector_test.rs` |
