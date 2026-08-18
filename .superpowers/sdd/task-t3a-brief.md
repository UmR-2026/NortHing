# Task T3a — Self-cognition store (storage + one-time migration, no prompt change)

Base commit: `fd61f5e` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope boundary (read this first)

T3 in the plan bundles storage, migration, prompt-injection split, and permission gating.
It is split in two. **This task is T3a: storage + migration only.**

- **In scope**: new SQLite table, new access module, `SelfCognitionStore` port
  implementation, one-time non-destructive migration from `identity.md`.
- **Out of scope (T3b, do NOT touch)**: `system_prompt.rs` and anything that changes what
  goes into a prompt; boundary-rule changes; deleting or rewriting `identity.md`.
- **Acceptance includes proving prompt output did not change** (§6.2).

Rationale for the split: this task migrates user-visible data, the next one changes agent
behavior. Bundling both would make a rollback ambiguous.

## 2. Verified facts (do not re-derive; all line numbers checked)

- Port already defined, `src/agentic/src/ports.rs:189-194`:
  ```rust
  /// Self-cognition store. Agent-exclusive: judge-mom, the garden pass, and the
  /// review path must never receive this port.
  pub trait SelfCognitionStore {
      fn load(&self) -> GrowthResult<Vec<SelfNote>>;
      fn append(&self, note: &SelfNote) -> GrowthResult<()>;
  }
  ```
- `SelfNote`, `src/agentic/src/ports.rs:131-135`: `{ text: String, created_at_ms: u64,
  trigger: String }`.
- `src/agentic/src/selfcog.rs` is a 1-line placeholder. **Leave it alone** — it is reserved
  for G3-T17 (agent-initiated writes). This task adds no crate code.
- Existing identity file, `src/crates/assembly/core/src/agentic/identity.rs` (70 lines):
  - `IDENTITY_FILE_NAME = "identity.md"` (`:4`)
  - `identity_path()` = `dirs::config_dir()/northhing/identity.md` (`:14-19`) — **global,
    not per-workspace**
  - `identity_exists()` (`:21`), `load_identity() -> Option<String>` (`:25`),
    `save_identity` (`:41`), `clear_identity` (`:34`)
  - Content is a single 50-80 character first-person paragraph produced during onboarding
    (see `build_identity_prompt`, `:50`), **not** a list of notes.
- Memory DB is **global**: `default_memory_db_path()` =
  `dirs::config_dir()/northhing/memory/memory.db`
  (`service/agent_memory/memory_db.rs:817-827`). Same config root as `identity.md`, so
  self-cognition stays global; per-workspace scoping (as `facts` does with a workspace key)
  **must not** be introduced here.
- Existing tables created with `CREATE TABLE IF NOT EXISTS` at
  `memory_db.rs:78` (facts), `:107` (keyword_weights), `:115` (judge_mom),
  `:121` (fact_reviews). Follow that same pattern.
- Test isolation helpers exist and must be used by your tests:
  `with_test_memory_db_path` (`:870`), `unique_test_memory_db_path` (`:877`),
  RAII cleanup via `MemoryDbPathGuard`.
- `memory_db.rs` is **943 lines**, already over the 800 limit (pre-existing, owned by T7).
  `memory_db_tests.rs` is **799 lines**, at the cap.

## 3. Required change

### 3.1 New table

Add `self_cognition` to the same DB, created in the same schema-init path as the four
existing tables, using `CREATE TABLE IF NOT EXISTS`. Columns:

| column | type | notes |
|---|---|---|
| `id` | TEXT PRIMARY KEY | uuid, same style as `facts` |
| `text` | TEXT NOT NULL | note body |
| `trigger` | TEXT NOT NULL | why it was written |
| `created_at` | INTEGER NOT NULL | epoch ms |

**No workspace column** (see §2: self-cognition is global by design).

### 3.2 New access module

Put all SQLite access for this table in a **new file**
`src/crates/assembly/core/src/service/agent_memory/self_cognition.rs`, not in
`memory_db.rs`. Two reasons: the plan requires a separate access module for this library
(D4 分库), and `memory_db.rs` is already over the line limit.

Register the module wherever the sibling modules in `service/agent_memory/` are declared.
Only the schema statement itself may need to touch `memory_db.rs`; keep any such addition
minimal.

### 3.3 Port implementation

Implement `northhing_agentic_growth::ports::SelfCognitionStore` for a core-side adapter,
following the existing pattern used for `GrowthStateStore` in
`src/crates/assembly/core/src/agentic/growth_adapter.rs` (266 lines) — read it first and
mirror its structure, error mapping, and naming.

- `load()` returns notes ordered **oldest first** (`created_at` ASC, tie-break by `id` for
  a total order).
- `append()` inserts one note. Append-only: **never** UPDATE or DELETE rows in this table.
- Errors map to `GrowthResult` the same way `growth_adapter.rs` maps them; warn-only
  logging, no panics.

### 3.4 One-time migration from `identity.md`

The existing onboarding paragraph must not be lost. On store initialization:

- If the `self_cognition` table has **zero rows** AND `identity_exists()` is true AND
  `load_identity()` yields non-empty content after trimming, insert exactly one note:
  - `text` = the file content (trimmed of surrounding whitespace, otherwise verbatim —
    do not reformat, re-wrap, or translate it)
  - `trigger` = `"migrated-from-identity-md"`
  - `created_at` = the file's mtime in epoch ms if obtainable, else now
- **Idempotence is mandatory**: the migration must run at most once. Repeated
  initialization must not append a second copy. Derive idempotence from observable state
  (e.g. table non-empty, or a recorded migration marker) — do not rely on a process-level
  flag that resets on restart.
- **Non-destructive**: do **not** delete, move, or rewrite `identity.md`. T3b still reads
  it, and it is user-visible data. `clear_identity` / `save_identity` keep their current
  behavior and callers.
- If the table is non-empty, skip silently (debug log only). If `identity.md` is absent,
  skip silently — an empty store is a valid state.
- Migration failure must be **warn-only** and must never block store creation or any
  caller; a failed migration must be retryable on the next initialization (i.e. do not
  record a "done" marker on the failure path).

### 3.5 Wiring restriction (safety, D9)

D9: self-cognition is **agent-exclusive**; judge-mom has no access, not even read.

For this task that means: implement the store and its port, but **do not** pass it to, call
it from, or otherwise reference it in any judge-mom, dream/garden, or review code path
(`service/agent_memory/{dream,judge_memory}.rs` and anything they call). Adding no call
sites at all is acceptable and expected — the consumer arrives in T3b/T17. State
explicitly in your report which call sites you added, if any.

## 4. Constraints

- warn-only; **never run `cargo fmt`**.
- English-only in code, comments, and log strings; no emoji. Chinese literals allowed only
  in test data (note: real migrated content will be Chinese — that is data, not source).
- No `unwrap` / `expect` / `panic!` / `todo!` in non-test code.
- **No crate changes**: `cargo test -p northhing-agentic-growth` must stay at 139.
- Do **not** modify `scripts/core-boundaries/**`. If a rule blocks you, stop and report
  `BLOCKED` with the exact checker output.
- Do **not** modify `system_prompt.rs` or any prompt-building file.
- Every new production file stays under 800 lines. Put tests in a separate test module or
  file if that is what keeps them under the cap; do not grow `memory_db_tests.rs` past
  800 — start a new test file if needed.
- Mind core `lib.rs:3-4` `#![allow(dead_code)]` / `#![allow(unused_imports)]`: an unchanged
  warning count does **not** prove you left no dead code or unused imports. This task
  legitimately adds code with no production caller yet, so check each new symbol by hand
  and say in the report which ones are intentionally unused.

## 5. Tests

Use `with_test_memory_db_path` + `unique_test_memory_db_path` for isolation. Cover:

1. `append` then `load` round-trips text/trigger/created_at exactly.
2. `load` on a fresh DB with no `identity.md` returns an empty vec (not an error).
3. Ordering: three notes appended out of chronological order come back oldest-first.
4. **Migration happy path**: with a fake `identity.md` present and an empty table, the
   first initialization creates exactly one note whose text equals the file content and
   whose trigger is `migrated-from-identity-md`.
5. **Migration idempotence**: initializing twice (or three times) still yields exactly one
   note. This is the highest-value test in this task.
6. **Migration is non-destructive**: `identity.md` still exists with unchanged content
   after migration.
7. **Migration skipped when table non-empty**: a pre-existing note plus a present
   `identity.md` yields no extra note.
8. Append-only: no code path in the new module issues UPDATE or DELETE against
   `self_cognition` (argue from the module source in the report; a grep is acceptable
   evidence).

If overriding the `identity.md` location in tests requires a test hook, add one modeled on
the existing `TEST_MEMORY_DB_PATH` thread-local pattern (`memory_db.rs:853-873`) and keep
it `#[cfg(test)]`.

## 6. Verification (paste **complete raw stdout+stderr**, not excerpts)

Prefix: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth` — must be exactly 139
2. `cargo check -p northhing-core --features product-full` — warning baseline **19**, must
   not increase
3. `cargo test -p northhing-core --features product-full self_cognition` (new; report total)
4. `cargo test -p northhing-core --features product-full memory_db` (28 now)
5. `cargo test -p northhing-core --features product-full growth_adapter` (30 now)
6. `node scripts/check-core-boundaries.mjs` — exit 0
7. Line counts via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count` for every file
   you created or grew (`Measure-Object -Line` under-reports; do not use it)

### 6.1 Schema-safety check

Confirm and show that opening an **existing** DB created before this change succeeds and
gains the new table (i.e. `CREATE TABLE IF NOT EXISTS` runs on every open, no migration
version bump needed). If the schema-init path is version-gated, say so explicitly and
show how you handled an already-initialized DB — a table that only appears for fresh
installs would be a silent failure for existing users.

### 6.2 Proof of no prompt change

`cargo test -p northhing-core --features product-full prompt_injection` must pass
unchanged, and `git diff --stat` must show **no** modification to
`agentic/agents/prompt_builder/**`. Show both.

## 7. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `feat(growth): `.
- `git status --short` clean before finishing. Do **not** commit anything under
  `.superpowers/`.
- Report to `E:\agent-project\northing\.superpowers\sdd\task-t3a-report.md`: files changed
  with `file:line`, the exact schema statement, how idempotence is guaranteed (this is the
  reviewer's focus), which call sites you added (§3.5), intentionally-unused new symbols,
  §6.1 and §6.2 evidence, full verification output, line counts, and anything ambiguous.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
