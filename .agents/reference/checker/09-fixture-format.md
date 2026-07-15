# Plan Compliance Checker — Fixture Format

> The 4 fixtures under `tools/plan-compliance-checker/tests/fixtures/`
> define what a plan doc looks like for the checker. Use them as the
> canonical examples when writing new fixtures or new check rules.

## Fixture file: `good-plan.md`

```markdown
# My Good Plan

A test plan that always passes.

### Task 1.1: this is a good task

**Files:**

- Create: `Cargo.toml`

**Step 1**: this step does nothing

**Step 2**: this step verifies the file exists

Run: `test -f Cargo.toml`
Expected: PASS
```

What this exercises:
- `parse_plan` extracts `title = "My Good Plan"`.
- Task 1.1 is parsed with `files.create = ["Cargo.toml"]`.
- Two steps are parsed; step 2 has `verify_command = "test -f Cargo.toml"`,
  `expected_outcome = Pass`.
- `check_plan` finds `Cargo.toml` exists at workspace root → `FileExists` ok.

## Fixture file: `path-mismatch-plan.md`

```markdown
# Path Mismatch Plan

### Task 1.1: this task points at a path under the old layout

**Files:**

- Create: `crates/nonexistent/Cargo.toml`

**Step 1**: nothing
```

What this exercises:
- `parse_plan` succeeds.
- `check_plan` runs `detect_path_mismatch("crates/nonexistent/Cargo.toml", root)`.
- The path doesn't exist relatively; the v3-layout heuristic
  (`<root>/src/<path>`) is tried; the result depends on whether
  `src/crates/nonexistent/Cargo.toml` happens to exist (it doesn't
  in this workspace).
- `CheckResult::PathConsistency { ok: false, suggestion: Some(...) }` is emitted.

## Fixture file: `missing-file-plan.md`

```markdown
# Missing File Plan

### Task 1.1: this task points at a file that doesn't exist

**Files:**

- Create: `this-file-does-not-exist-anywhere-xyz.rs`

**Step 1**: nothing
```

What this exercises:
- `parse_plan` succeeds.
- `check_plan` finds the file doesn't exist and there's no
  reasonable suggestion. `CheckResult::PathConsistency { ok: false,
  suggestion: None }` is emitted.
- Task status: `Pending` (no commit either).

## Fixture file: `bad-commit-plan.md`

```markdown
# Bad Commit Plan

### Task 1.1: this task points at a path no commit ever touched

**Files:**

- Create: `some-path/Cargo.toml`

**Step 1**: nothing
```

What this exercises:
- `parse_plan` succeeds.
- `check_plan` finds the file doesn't exist and no commit matches.
- Task status: `Pending` (no commit).

## Format grammar (informal)

A plan doc is a Markdown file with this shape:

```markdown
# <plan title>                       ← first H1 becomes `plan.title`

(arbitrary body until the first H3)

### Task <id>: <title>                ← each H3 becomes a new Task
                                       id is "N.M" form
                                       title is everything after the colon

**Files:**                            ← enters files-block mode
- Create: `<path>`                    ← next inline code after "Create:" goes here
- Modify: `<path>`                    ← next inline code after "Modify:" goes here
                                       (range syntax not yet supported)

**Step <n>**: <description>           ← each "Step N:" text becomes a new Step
                                       description strips "**Step N**: " and trailing "**"/":"

Run: `<command>`                      ← next inline code after "Run:" becomes
                                       step.verify_command
Expected: PASS|FAIL|<n>               ← parsed by parse_expected
```

## What is NOT yet supported (gaps to be aware of)

These are real limitations in the current parser, called out in
`docs/notes/plan-compliance-checker.md`:

1. **No "Modify" range syntax** — `Modify: src/foo.rs:10-20` is parsed
   as the literal path `src/foo.rs:10-20`, not as a path with a range.
2. **`Run:` commands are not executed** — they are parsed into
   `Step.verify_command` but `check_plan` never invokes them. The
   spec calls for this; the implementation is incomplete.
3. **`--task` flag is parsed but not wired** in the dispatcher
   (`main.rs`). `check_plan` always runs over all tasks.
4. **`--skip-slow` / `--force-verify`** are defined but not consumed
   anywhere.
5. **The v3-layout heuristic** in `detect_path_mismatch` only tries
   `<root>/src/<path>`. It will miss any other layout transformation.

## When to add a new fixture

Add a new `tests/fixtures/<name>.md` and a corresponding entry in
`fixture_test.rs` that:
1. Parses the fixture with `parse_plan`.
2. Asserts the parsed shape (title, task count, expected files).
3. Runs `check_plan` on it.
4. Asserts on the resulting `TaskResult` (status + CheckResults).

The existing `fixture_test.rs` is the template.
