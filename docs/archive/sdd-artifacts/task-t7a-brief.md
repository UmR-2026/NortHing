# Task T7a — Permission-matrix boundary rules (D9 enforcement)

Base commit: `9f261cd` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope

Plan T7 bundles the permission matrix, the `supersede` restriction, and the `memory_db.rs`
split. This task is **T7a: boundary rules only**.

- **In scope**: three new rule groups in `forbidden-rules.mjs`; relocating one existing test
  so a rule can be strict; proving every new rule actually fires.
- **Out of scope**:
  - The **`supersede` rule is deferred to T12** — see §2.3. Do not add it, do not touch
    `supersede_fact` or dream's invalidation behavior.
  - `memory_db.rs` split (user deprioritized: that file is exercised daily, rot risk is low).
  - Any production behavior change. This task changes rules + moves test code only.

## 2. Verified facts (checked at `9f261cd`; do not re-derive, but do confirm symbol spellings)

### 2.1 Rule file shape

`scripts/core-boundaries/rules/source/forbidden-rules.mjs` (3019 lines) exports
`forbiddenContentRules` (line 3) and `forbiddenContentUnderRules` (line 2249). The
path-scoped form you will extend:

```js
{
  path: 'src/crates/assembly/core/src/service_agent_runtime',
  reason: 'why this boundary exists',
  patterns: [
    {
      regex: /\bself\.scheduler\s*\.\s*submit\b/,
      allowPaths: ['src/crates/contracts/product-domains/src/miniapp/runtime.rs'],
      message: 'what the author should do instead',
    },
  ],
}
```

`allowPaths` is the established way to whitelist a reviewed exception (real example at
line 2282). Paths are repo-relative with forward slashes.

### 2.2 Current self-cognition surface

Symbols live in `service/agent_memory/self_cognition.rs` and are re-exported from
`service/agent_memory/mod.rs`; the port adapter is in `agentic/growth_adapter.rs`.
Expected spellings (**verify each against the source before writing a regex** — a rule with
a misspelled symbol silently never fires, which is worse than no rule):

- access module: `load_self_cognition`, `append_self_cognition`, `count_self_cognition`,
  `migrate_identity_into_self_cognition`, `resolve_identity_path`, `SelfCognitionRow`
- adapter: `SelfCognitionDbStore`, `init_self_cognition_store`, `load_self_cognition`
- crate port: `SelfCognitionStore`
- table name in SQL: `self_cognition`

Current references outside the owning module: `growth_adapter.rs` (port impl),
`system_prompt.rs` + `system_prompt_tests.rs` (the one consumer, T3b), `mod.rs`
(re-exports), `memory_db.rs` (schema statement + `conn_locked` doc), and
**`dream.rs` inside its `#[cfg(test)] mod tests`** (the D9 negative test, `dream.rs:289-397`).

`conn_locked` is currently called only from `self_cognition.rs` (4 sites).

### 2.3 Why the `supersede` rule cannot land yet (do not attempt it)

`dream.rs` still calls `db.supersede_fact(...)` in production (`dream.rs:156`, inside the
`"supersede"` verdict arm at `:155`) and records `action: "supersede"` reviews (`:165`); the
verb is also baked into the dream prompt (`:211`, `:214`, `:218`). Plan D8 says the only
legitimate entry is `negation.rs`, and T12 (dream -> garden) owns removing dream's hard
invalidation. Adding the rule now would either fail the checker immediately or push you into
T12's scope. **Leave it; the plan has been annotated to carry it into T12.**

## 3. Required change

### 3.1 Relocate the D9 negative test out of `dream.rs`

The rule in §3.2 must be able to ban **all** self-cognition symbols in `dream.rs` without
`allowPaths` on `dream.rs` itself (whitelisting the whole file would void the rule).

Move the D9 negative test (`dream_payload_never_contains_self_cognition_sentinel`,
`dream.rs:304`, inside the inline `#[cfg(test)] mod tests` that starts at `dream.rs:289`;
it imports `append_self_cognition` at `:293`, seeds at `:313`, and calls the **private**
`build_dream_messages` (`dream.rs:207`) at `:331`) into its own file, included as a child
module so it keeps access to those private items:

```rust
#[cfg(test)]
#[path = "dream_d9_tests.rs"]
mod d9_tests;
```

The repo already uses this exact pattern — `self_cognition.rs:319-320` and
`system_prompt.rs:408-409` both do `#[cfg(test)]` + `#[path = "..._tests.rs"]` + `mod tests;`.
**Do not name the new module `tests`**: `dream.rs` already has an inline `mod tests` at
`:290` and the names would collide. Keep both modules; only the one D9 test moves.

Constraints on the move:
- **Pure relocation**: identical assertions, identical test name, no weakening. The test
  asserts both "sentinel absent from the dream payload" **and** "fact text present" — the
  second assertion is the anti-vacuity guard and must survive verbatim.
- `dream.rs`'s remaining tests stay where they are.
- Test count must not drop: `dream` filter is **7** now; after the move it must still be 7.

### 3.2 Rule group A — dense path must not touch the self-cognition library (D9)

D9: self-cognition is agent-exclusive; judge-mom, the garden/dream pass, and the review path
must never read **or** write it.

Add a path-scoped rule covering the dense-path files (`service/agent_memory/dream.rs` and
`service/agent_memory/judge_memory.rs`; scope by the narrowest `path` that covers both while
not accidentally covering `self_cognition.rs`, `growth_adapter.rs`, or the prompt builder).

Ban the self-cognition symbols from §2.2 (both read and write, plus the raw table name in
SQL string literals). `allowPaths` may list **only** the relocated
`dream_d9_tests.rs`, with a comment saying why (a negative test must be able to seed the
table it proves is unreachable).

The `reason` and each `message` must be written so a future author who trips the rule
understands the invariant, not just that they hit a lint. Reference D9.

### 3.3 Rule group B — dense path must not use the `conn_locked` escape hatch

`memory_db.rs`'s `pub(crate) fn conn_locked()` hands out a raw `MutexGuard<Connection>`,
which lets any core module read any table including `self_cognition`. T3a accepted this seam
on the explicit condition that T7 enforce it by rule (the seam is documented in
`conn_locked`'s doc comment).

Ban `conn_locked` in the same dense-path scope as §3.2. No `allowPaths` should be needed —
verify that claim rather than assuming it.

### 3.4 Rule group C — `prompt_builder/**` is read-only on growth state

The prompt builder may read growth state but must not mutate it.

**Two verified exceptions that must be handled honestly** (the boundary checker scans test
code too — it has no notion of `#[cfg(test)]`, so both need explicit `allowPaths`):

1. `system_prompt.rs:6` imports `init_self_cognition_store` and `:317` calls it as
   `let _store = init_self_cognition_store(&db);` — the return value is discarded, so the
   **only** effect is the one-time idempotent identity migration, i.e. the prompt path
   performs a write. Orchestrator decision: keep the lazy migration where it is (it is
   idempotent and self-healing) and make the rule precise instead. Do **not** relocate the
   migration trigger; that is a separate decision, not this task.
2. `system_prompt_tests.rs:36-44` (`append_note`) constructs
   `crate::agentic::growth_adapter::SelfCognitionDbStore::new(db)` and calls `.append(...)`
   to seed fixtures — a legitimate test write.

Therefore:

- Ban growth-state **write** symbols under `prompt_builder/**`: at minimum
  `append_self_cognition`, `SelfCognitionDbStore`, `set_blob`, `boost_keyword`,
  `insert_fact`, `supersede_fact`. Do **not** ban a bare `append` or `init_...` (too noisy /
  would be a no-op rule).
- `allowPaths` is per-pattern, so use separate patterns: the `SelfCognitionDbStore` pattern
  allow-lists only `system_prompt_tests.rs` (test seeding); the others should need no
  exception — verify that rather than assuming it.
- Record the `init_self_cognition_store` migration exception in the rule group's `reason`
  text so the seam is auditable, and state in the report that it is deliberately not banned.
- If you find a write symbol under `prompt_builder/**` that this brief did not anticipate,
  report it — do not silently widen `allowPaths` to make the checker green.

## 4. Constraints

- **Do not change any Rust production behavior.** The only Rust edit is the test relocation
  in §3.1 (plus the `#[cfg(test)] #[path] mod` line in `dream.rs`).
- Do not add, remove, or reword production log strings.
- Match the surrounding style of `forbidden-rules.mjs` exactly (key order, quoting,
  indentation, how `reason`/`message` are phrased). Read several neighbouring entries first.
- English-only; no emoji.
- Do not touch `checker.mjs`, `self-test.mjs`, `crate-rules.mjs`, `crate-layout.mjs`,
  `feature-rules.mjs`, `facade-rules.mjs`, or `required-rules.mjs`. If a rule cannot be
  expressed with the existing `forbiddenContentUnderRules` shape, stop and report `BLOCKED`
  explaining what shape you would need — do not extend the checker.
- Never run `cargo fmt`.
- No new Rust production files; nothing may exceed 800 lines.

## 5. Verification (paste **complete raw stdout+stderr**, not excerpts)

Prefix for cargo: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `node scripts/check-core-boundaries.mjs` — exit 0 on the unmodified-behavior tree
2. `cargo test -p northhing-core --features product-full dream` — must be **7** (the
   relocation must not lose a test)
3. `cargo test -p northhing-core --features product-full self_cognition` — 19 now
4. `cargo test -p northhing-core --features product-full system_prompt` — 21 now
5. `cargo check -p northhing-core --features product-full` — warning baseline **19**,
   must not increase
6. `cargo test -p northhing-agentic-growth` — 139
7. Line count of `forbidden-rules.mjs` via
   `(Get-Content -LiteralPath <path> -Encoding UTF8).Count`

### 5.1 Prove every new rule actually fires (hard acceptance criterion)

A boundary rule that never fires is worse than no rule: it looks like enforcement and is
not. For **each** pattern you add:

1. Introduce a minimal temporary violation in the correct file (e.g. add a line in
   `dream.rs` that calls `load_self_cognition`, or `conn_locked`, or a write symbol under
   `prompt_builder/`).
2. Run `node scripts/check-core-boundaries.mjs` and capture the **exact failure output**,
   showing your rule's `message` and the file it flagged.
3. Revert the temporary violation.
4. Re-run the checker and show it back at exit 0.

Report this as a table: pattern -> the violation line you used -> the checker output line
that caught it. If any pattern cannot be made to fire, that pattern is broken (usually a
misspelled symbol or a `path` scope that does not cover the file) — fix it, do not ship it.

Confirm at the end that `git status --short` is clean, i.e. no temporary violation survived.

## 6. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `chore(boundaries): ` (this task
  ships rules, not features).
- Only these files may change: `forbidden-rules.mjs`, `dream.rs` (the `#[path] mod` line and
  the removal of the relocated test), and the new `dream_d9_tests.rs`.
- `git status --short` clean before finishing. **Do not commit anything under
  `.superpowers/`.**
- Write the report to **`E:\agent-project\northing\.superpowers\sdd\task-t7a-report.md`**
  (the main repo path, **not** the worktree's `.superpowers/`): rules added with
  `file:line`, exact symbol spellings you verified, the §5.1 firing table, every
  `allowPaths` entry with its justification, full verification output, line counts, and
  anything ambiguous.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
