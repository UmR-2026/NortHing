# Plan Compliance Checker — "Do NOT Copy Verbatim" Notes

> Most of the "do not copy" warnings in this domain are **known gaps**
> in the implementation. They are tracked in
> `docs/notes/plan-compliance-checker.md` as future work. If you add
> a new check rule or fix one of these, update this file in the same
> commit.

## ⛔ `--task` flag is parsed but not wired

`main.rs` defines `--task <id>` but the dispatcher doesn't filter on it.
`check_plan` always runs over all tasks. If you want a single-task
report, grep the output manually. The fix is one `if` statement in
`main.rs`; tracked as future work.

## ⛔ `--skip-slow` and `--force-verify` are parsed but not consumed

`main.rs` defines both flags but the dispatcher never reads them. The
`Step.verify_command` field is parsed but `check_plan` never invokes
it. To actually run a verify command, you must:

1. Add a `CommandRunner` to `check_plan`'s signature (or thread it
   through a `CheckerContext`).
2. Read the flag in `main.rs` and pass it through.
3. Update `task_checker_test.rs` to cover the new code path.

## ⛔ `Run:` commands are not executed

`Step.verify_command` is parsed but `check_plan` never invokes
`command_runner::run_command`. The spec called for this; the
implementation is incomplete. See `09-fixture-format.md` for the
gap statement.

## ⛔ `Modify:` range syntax is not supported

`Modify: src/foo.rs:10-20` is parsed as the literal path
`src/foo.rs:10-20` (with the range as part of the string), not as a
path with a `range: Some(LineRange { start: 10, end: 20 })`. The
`ModifyTarget` type and the `LineRange` type exist in `plan.rs` but
the parser doesn't populate them. To fix: extend the
`Event::Code` handler in `parse_plan`.

## ⛔ The v3-layout heuristic only tries `<root>/src/<path>`

`detect_path_mismatch` only tries the `src/`-prepend heuristic. It
will miss any other layout transformation (e.g. `app/`, `src/app/`).
Don't add ad-hoc prepends inside the function — add a configurable
transform list instead.

## ⛔ `find_workspace_root` is a string-contains check, not a TOML parse

`path_resolver.rs` looks for the literal string `"[workspace]"` inside
`Cargo.toml`. It will match a comment that happens to contain that
string. Don't rely on this for a critical "is this a workspace?"
decision; use `cargo metadata` if you need precision.

## ⛔ Don't change `Report` JSON shape without coordinating CI

The JSON output of `format_json` is consumed by CI. If you change the
shape (add a field, rename a key), update:

1. `tools/plan-compliance-checker/src/report.rs` (the `Report` struct)
2. Any CI scripts that parse the JSON output
3. The `cli_test.rs` snapshot if it includes JSON output

## ⛔ Don't add new dependencies without checking workspace `Cargo.toml`

The crate uses `clap`, `tokio`, `serde`, `serde_json`, `pulldown-cmark`,
`anyhow` — all from the workspace. Don't add a new dep without
adding it to the workspace `Cargo.toml` `[workspace.dependencies]`
section first.

## ⚠️ `Task 4.4` of the impl plan was deliberately skipped

The plan's Task 4.4 (bulk-sed the actor plan paths to add `src/`
prefix) was skipped because the actual paths were off-layout (not just
missing `src/` prefix). The plan note in
`docs/notes/plan-compliance-checker.md` records this. **Do not** try
to finish that task — it was deliberately abandoned.

## ⚠️ `parse_plan` is a state machine, not a regex

The parser uses `pulldown-cmark` events with a hand-rolled state
machine (`pending_create` / `pending_modify` / `pending_step_text`
flags). If you change the parser, the simplest way to break it is to
add a new text pattern that doesn't account for the existing flag
state. **Always** add a fixture + test when extending the parser.

## ⚠️ `path_matches` is the only path-equality helper

`task.rs::path_matches` (line 25-41) handles exact match + path-segment
suffix match with `/` boundary check. Don't write a new path-comparison
function — reuse this one. If the logic needs to grow, extend this
function (and its test in `path_resolver_test.rs`).

## ⚠️ `Pending` vs `Pass` vs `Fail` depends on commit presence

Per `task.rs`:
- `Pending` if task has no `create` work OR no matching commit exists.
- `Pass` if commit exists AND all checks pass.
- `Fail` if commit exists but files are missing or files don't match.

A task with a "Done" `Files:` block but no commit ever touching it
will report `Pending` forever. This is by design — "I wrote a plan"
≠ "I did the work".

## ⚠️ `check_plan` doesn't take a `CommandRunner`

`check_plan(plan, workspace_root)` has no `CommandRunner` parameter.
Adding verify-command execution requires a signature change.
Coordinate with all callers (currently just `main.rs`).

## ⚠️ `format_human` and `format_json` destructure every `CheckResult` variant

When you add a new variant, you must touch `report.rs` in 3 places:
- `check_to_json`
- `print_check`
- The destructuring in `format_human` and `format_json`

A compile error will tell you which one you missed.

## ⚠️ No end-to-end CLI test

`cli_test.rs` tests `--help` and no-args, but no test runs the
binary against a real plan and asserts on the output. If you add a
new check that affects CLI output, write an end-to-end test
(`std::process::Command::new(...)` on the binary).

## ✅ Things you SHOULD copy

- The fixture file format (in `09-fixture-format.md`) — copy verbatim
  for any new check rule.
- The 5-step extension process (in `README.md`) — `CheckResult` →
  `check_plan` → `report.rs` (3 places) → test → fixture.
- The `path_matches` helper (in `task.rs`) — reuse, don't fork.
- The "Pending / Pass / Fail" status semantics (in `task.rs`) — these
  are documented and should not change.
- The `find_workspace_root` "walk up + string-contains" pattern —
  simple, fast, and the alternatives (cargo metadata) are heavier.
