# Task T3b — Self-cognition injection from the store + dense-path gating

Base commit: `39fadea` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope

T3a landed the storage: table `self_cognition`, access module
`service/agent_memory/self_cognition.rs`, port impl `SelfCognitionDbStore` in
`agentic/growth_adapter.rs`, and a one-time non-destructive migration of `identity.md`.
It deliberately added **zero** production call sites.

T3b adds the **one** consumer and the gating:

- **In scope**: read self-cognition from the store when building the system prompt;
  fall back to `identity.md`; a rendering policy for multiple notes; a negative test
  proving the dense / judge-mom path cannot see self-cognition.
- **Out of scope**: `forbidden-rules.mjs` / `scripts/core-boundaries/**` changes (T7 owns
  the permission matrix, including the `conn_locked` rule); agent-initiated writes (T17);
  external-memory (facts) injection, which already has its own path and must not change.

## 2. Verified facts (do not re-derive)

- Injection site, `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs:15-42`:
  ```rust
  async fn build_workspace_persona_with_identity(&self, workspace: &Path) -> String {
      let persona = /* build_workspace_persona_prompt(workspace), warn-only, "" on error */;

      if identity_exists() {
          if let Some(identity_content) = load_identity() {
              let mut result = persona;
              if !result.is_empty() {
                  result.push_str("\n\n");
              }
              result.push_str("# Self-cognition\n\n");
              result.push_str(&identity_content);
              result.push_str("\n\n");
              return result;
          }
      }

      persona
  }
  ```
  Imports at `:6`: `use crate::agentic::identity::{identity_exists, load_identity};`
  The function is called from two branches inside the `PLACEHOLDER_PERSONA` replacement in
  the same file (one of them behind `self.context.remote_execution.is_some()`); the file is
  276 lines.
- `identity.md` = `dirs::config_dir()/northhing/identity.md` (`agentic/identity.rs:4,14-19`),
  global. Content is a single 50-80 character first-person paragraph from onboarding.
- Store side (T3a): notes are returned **oldest-first** (`created_at ASC, id ASC`); the
  migrated onboarding note carries `trigger = "migrated-from-identity-md"` and the fixed id
  `"migration:identity-md"`; migrated `text` is the file content minus BOM and surrounding
  whitespace.
- The memory DB is global: `default_memory_db_path()` =
  `dirs::config_dir()/northhing/memory/memory.db`.

## 3. Required change

### 3.1 Read from the store, keep `identity.md` as fallback

Replace the `identity_exists()` / `load_identity()` pair as the *primary* source with a read
from the self-cognition store. Precedence, in order:

1. Store read succeeds and yields >= 1 non-blank note -> render from the store (§3.2).
2. Otherwise (store read errors, or yields nothing) -> **fall back to `identity.md` exactly
   as today**, byte-for-byte identical to current behavior.
3. If neither is available -> return `persona` unchanged, as today.

The fallback is mandatory, not optional: if a user's migration failed or their DB is
missing, they must not silently lose their self-cognition block. A store read error is
warn-only and must never fail prompt building.

Do not delete, rewrite, or stop maintaining `identity.md`; `save_identity` / `clear_identity`
and their existing callers keep working unchanged. (Onboarding still writes the file; keeping
the store in sync with later onboarding rewrites is **out of scope** — if you notice a gap
there, report it, do not fix it.)

### 3.2 Rendering policy for multiple notes (orchestrator decision, implement as written)

Today the store holds exactly one note, so the common path must be indistinguishable from
current output. Rules:

- Notes render **oldest-first**, separated by one blank line, under the same
  `# Self-cognition\n\n` heading, followed by the same trailing `\n\n`. Heading text,
  position, and surrounding blank lines are unchanged.
- Blank / whitespace-only notes are skipped.
- Total budget: **2000 characters** counted over the rendered note bodies (not the heading).
  Define it as a named constant.
- On overflow: **always keep the first (oldest) note** — it is the foundational onboarding
  identity — then fill with the **most recent** notes that still fit, and drop the middle
  ones. Do not emit a truncation marker.
- Count characters, not bytes (the content is Chinese; `chars().count()`).

With one note this policy is unreachable; it exists so T17 cannot produce an unbounded
prompt. State in the report that you implemented it and which tests cover it.

### 3.3 Dense-path gating (D9)

D9: self-cognition is agent-exclusive; judge-mom, the garden/dream pass, and the review path
must never receive it — **not even read access**.

- Do **not** pass the store, its port, or its rendered text into any judge-mom, dream, or
  review code path (`service/agent_memory/{dream,judge_memory}.rs` and anything they call).
- The only new consumer is the system-prompt path in §3.1.
- Add a **negative test** proving the dense / judge-mom prompt path does not contain
  self-cognition content: seed the store with a distinctive sentinel string, build whatever
  prompt/payload the judge-mom or dream path builds, and assert the sentinel is absent.
  If the existing structure makes such a test impossible without new seams, say so
  explicitly in the report with the reason and what you asserted instead — do **not** invent
  a new seam just to make the test possible, and do not silently skip it.

## 4. Constraints

- warn-only; **never run `cargo fmt`**.
- English-only in code, comments, and log strings; no emoji. Chinese allowed only in test
  data and migrated content.
- No `unwrap` / `expect` / `panic!` / `todo!` in non-test code.
- **No crate changes**: `cargo test -p northhing-agentic-growth` must stay at 139. Do not
  touch `src/agentic/src/selfcog.rs` (reserved for T17).
- Do **not** modify `scripts/core-boundaries/**`. If a boundary rule blocks the
  prompt-builder from reading the store, stop and report `BLOCKED` with the exact checker
  output. (Expected to be allowed: the plan's T7 states `prompt_builder/**` may read growth
  state read-only.)
- Every production file stays < 800 lines. `system_prompt.rs` is 276 lines now. If a helper
  gets long, put it in a new file rather than growing an existing one past the cap.
- Mind core `lib.rs:3-4` `#![allow(dead_code)]` / `#![allow(unused_imports)]`: an unchanged
  warning count does **not** prove you left no dead code. T3a left
  `count_self_cognition` with no production caller — if your change gives it one, say so;
  if not, leave it alone.

## 5. Tests

1. **Single migrated note reproduces current output** — store holds only the migrated
   onboarding note; the rendered block matches the current `identity.md`-based output
   (see §6.1 for the exact expectation and how to report any residual difference).
2. **Fallback when store empty** — empty store + `identity.md` present -> output identical
   to today's.
3. **Fallback when store read fails** — simulate a store/DB failure -> output identical to
   today's, and prompt building still succeeds (warn-only).
4. **Neither source** — empty store + no `identity.md` -> `persona` returned unchanged.
5. **Multiple notes** — three notes render oldest-first, blank line separated, under one
   heading.
6. **Blank notes skipped**.
7. **Budget overflow** — first note always kept, newest fill, middle dropped, total within
   the 2000-character budget.
8. **Dense-path negative test** (§3.3): sentinel in store must be absent from the
   judge-mom / dream path output.
9. **Persona interaction preserved** — empty persona vs non-empty persona produce the same
   joining whitespace as today (`"\n\n"` only when persona is non-empty).

Use the T3a test isolation helpers (`with_test_memory_db_path`, `unique_test_memory_db_path`)
so tests never touch the real DB, and make sure no test reads or writes the user's real
`identity.md`.

## 6. Verification (paste **complete raw stdout+stderr**, not excerpts)

Prefix: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth` — exactly 139
2. `cargo check -p northhing-core --features product-full` — warning baseline **19**, must
   not increase
3. `cargo test -p northhing-core --features product-full prompt_injection`
4. `cargo test -p northhing-core --features product-full system_prompt`
5. `cargo test -p northhing-core --features product-full self_cognition` (18 now)
6. `cargo test -p northhing-core --features product-full growth_adapter` (30 now)
7. `node scripts/check-core-boundaries.mjs` — exit 0
8. Line counts via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count` for every file
   created or grown

### 6.1 Behavior-equivalence evidence (hard acceptance criterion)

This task changes what the agent sees about itself, so equivalence must be demonstrated,
not asserted. Produce the **full rendered `# Self-cognition` block, before and after**, for
these three realistic inputs, and show them verbatim in the report:

- (a) `identity.md` whose content has **no** trailing newline
- (b) `identity.md` whose content **ends with a trailing newline**
- (c) `identity.md` with a UTF-8 BOM

Note that T3a's migration trims surrounding whitespace and strips the BOM, while today's
code injects the file content verbatim. So for (b) and (c) the output may legitimately
differ by trailing whitespace or a stripped BOM. **Do not hide or "fix" this by re-adding
whitespace.** Report the exact difference per case and let the reviewer judge. If any case
differs by more than leading/trailing whitespace or a BOM, treat that as a problem and say
so explicitly.

## 7. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `feat(growth): `.
- `git status --short` clean before finishing. Do **not** commit anything under
  `.superpowers/`.
- Report to `E:\agent-project\northing\.superpowers\sdd\task-t3b-report.md`: files changed
  with `file:line`, the precedence chain as implemented, the §3.2 policy and its tests, the
  §3.3 negative test (or the documented reason it was impossible), the §6.1 three-case
  before/after blocks, full verification output, line counts, and anything ambiguous.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
