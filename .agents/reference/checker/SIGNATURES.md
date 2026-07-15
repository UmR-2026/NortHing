# Plan Compliance Checker — Signatures

## Data structures — `plan.rs`

```rust
pub struct Plan {
    pub path: PathBuf,
    pub title: String,
    pub tasks: Vec<Task>,
    pub plan_start_sha: String,    // defaults to "HEAD~1" if --start-sha not given
}

pub struct Task {
    pub id: String,                // e.g. "1.1"
    pub title: String,
    pub files: FilesSpec,
    pub steps: Vec<Step>,
}

pub struct FilesSpec {
    pub create: Vec<PathBuf>,
    pub modify: Vec<ModifyTarget>,
}

pub struct ModifyTarget {
    pub path: PathBuf,
    pub range: Option<LineRange>,
}

pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

pub struct Step {
    pub index: usize,
    pub description: String,
    pub expected_outcome: ExpectedOutcome,
    pub verify_command: Option<String>,
}

pub enum ExpectedOutcome {
    Pass,
    Fail(String),
    Custom(i32),
}

/// Main parser. Markdown → Plan. Uses pulldown-cmark events, not regex.
pub fn parse_plan(input: &str) -> Plan;
```

## Check logic — `task.rs`

```rust
pub enum TaskResult {
    Pending  { task_id: String, checks: Vec<CheckResult> },
    Pass     { task_id: String, checks: Vec<CheckResult> },
    Fail     { task_id: String, checks: Vec<CheckResult> },
}

pub enum CheckResult {
    FileExists        { path: String, ok: bool },
    FileModified      { path: String, ok: bool, sha: Option<String> },
    CommitPresent     { ok: bool, sha: Option<String> },
    CommitFilesMatch  { ok: bool, expected: Vec<String>, actual: Vec<String> },
    PathConsistency   { path: String, ok: bool, suggestion: Option<String> },
}

/// Main check function. Iterates over plan.tasks, produces a TaskResult per task.
pub fn check_plan(plan: &Plan, workspace_root: &Path) -> Vec<TaskResult>;

/// Helper: compares two paths (exact match + path-segment suffix match
/// with `/` boundary check). Used in CommitFilesMatch.
pub fn path_matches(commit_path: &str, task_path: &str) -> bool;
```

## Path resolution — `path_resolver.rs`

```rust
pub struct PathMismatch {
    pub exists_relative: bool,
    pub suggestion: Option<PathBuf>,
}

/// Walks UP from `start` until it finds a `Cargo.toml` containing
/// the literal string "[workspace]". Returns the directory holding
/// that manifest, or None.
pub fn find_workspace_root(start: &Path) -> Option<PathBuf>;

/// Checks if `<workspace_root>/<plan_path>` exists. If not, tries
/// the v3-layout heuristic `<workspace_root>/src/<plan_path>`.
/// See NOTES.md.
pub fn detect_path_mismatch(plan_path: &str, workspace_root: &Path) -> PathMismatch;
```

## Git inspector — `git_inspector.rs`

```rust
/// Runs `git log <since>..HEAD --name-only --pretty=format:"%H|%s"` and
/// returns the parsed commit list.
pub fn commits_since(workspace_root: &Path, since: &str) -> Result<Vec<CommitInfo>, GitError>;

pub struct CommitInfo {
    pub sha: String,
    pub subject: String,
    pub files: Vec<String>,
}
```

## Command runner — `command_runner.rs`

```rust
/// Runs a shell command. Used (potentially) by future "verify_command"
/// support; currently NOT wired into check_plan. See NOTES.md.
pub fn run_command(cmd: &str) -> Result<i32, CommandError>;
```

## Report formatter — `report.rs`

```rust
pub struct Report {
    pub plan_path: PathBuf,
    pub plan_title: String,
    pub results: Vec<TaskResultJson>,
}

pub struct TaskResultJson { /* mirrors TaskResult for JSON */ }
pub struct CheckResultJson { /* mirrors CheckResult for JSON */ }

/// Human-readable output. Color-coded terminal output.
pub fn format_human(report: &Report) -> String;

/// JSON output. Stable shape (used by CI).
pub fn format_json(report: &Report) -> String;

/// Single-check-to-JSON helper. Touch this when adding a new CheckResult variant.
pub fn check_to_json(check: &CheckResult) -> serde_json::Value;

/// Single-check-to-human-string helper.
pub fn print_check(check: &CheckResult) -> String;
```

## CLI — `main.rs`

```rust
#[derive(Parser)]
#[command(name = "plan-compliance-checker", version)]
struct Cli {
    plan: PathBuf,                              // positional: plan.md path
    #[arg(long)] task: Option<String>,          // e.g. "1.3" — **NOT WIRED**, see NOTES
    #[arg(long)] skip_slow: bool,               // **NOT WIRED**, see NOTES
    #[arg(long)] force_verify: bool,            // **NOT WIRED**, see NOTES
    #[arg(long)] start_sha: Option<String>,     // defaults to "HEAD~1"
    #[arg(long, value_enum, default_value_t = Format::Human)] format: Format,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format { Human, Json }
```

## Tests

7 test files, 4 fixtures. All under `tools/plan-compliance-checker/tests/`.

| File | What it covers |
|---|---|
| `plan_struct_test.rs` | Default + serialize roundtrip |
| `plan_parser_test.rs` | End-to-end parse on inline sample |
| `path_resolver_test.rs` | `find_workspace_root` + `detect_path_mismatch` |
| `git_inspector_test.rs` | `commits_since` returns ≥ 3 commits |
| `task_checker_test.rs` | Empty plan + hand-built single-task plan |
| `cli_test.rs` | `--help` + no-args |
| `fixture_test.rs` | 4 fixtures parse + run |
