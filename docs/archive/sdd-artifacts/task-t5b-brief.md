# Task T5b — Move dream verdict parsing into the growth crate

Base commit: `71df0dd` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope

Three changes, all behavior-preserving:

1. Move `parse_dream_verdicts` (`service/agent_memory/dream.rs:238-265`) into the growth crate
   as a **generic, allow-list-parameterized** verdict parser at
   `src/agentic/src/review/verdict.rs` (currently a 1-line placeholder).
2. Consolidate `strip_json_fence`: there are currently **two identical private copies** —
   `dream.rs:267-280` and `src/agentic/src/distill/parse.rs:158-171`. Create one shared crate
   module and have both call sites use it. Net copies after this task: **one**.
3. Move the 6 verdict-parsing tests (`dream.rs:289-344`) to the crate.

### Explicitly out of scope (orchestrator scope rulings — do not do these)

- **Dream's candidate selection is NOT moved.** The plan's T5 text mentions it, but: it
  depends on core's `Fact` type (assembly layer, invisible to the crate) so it would need the
  same neutral-type treatment as T5a, **and** task T12 rewrites the selection semantics
  outright (dream becomes the garden pass with four different actions). Moving it now means
  moving it twice. It goes to T12.
- **Do not change any decision semantics.** Dream still supersedes facts today; removing that
  is T12's job.
- Do not touch `dream_d9_tests.rs` or the `#[cfg(test)] #[path]` child-module wiring at
  `dream.rs:341-348`.
- Do not touch `distill/parse.rs` beyond replacing its private `strip_json_fence` with a call
  to the shared one.

## 2. Verified facts (checked at `71df0dd`)

- `parse_dream_verdicts` (`dream.rs:238`) is **already pure**: no IO, no clock, no `uuid`, and
  no core types. Signature: `(json: &str, fact_count: usize) -> Vec<(usize, String, Option<String>)>`.
  This is why it can move as-is, unlike T5a's parser.
- Its **only** blocker is the hard-coded action allow-list at `dream.rs:251-254`:
  `Some("keep") | Some("supersede") => ...`.
- ⚠️ **Why parameterization is mandatory, not stylistic**: `src/agentic/AGENTS.md` §3 states
  *"Any supersede semantics appearing in the `garden` or `review` paths is a violation."*
  Hard-coding `"supersede"` inside `review/verdict.rs` would violate the crate's own layer
  contract on arrival. With the allow-list passed in by the caller, the crate carries **zero
  `supersede` literals** and T12 changes only the argument.
- The two `strip_json_fence` copies are byte-identical in behavior (trim → strip leading
  ```` ``` ```` → optional `json` → `trim_start` → strip trailing ```` ``` ```` → `trim`).
- `review/verdict.rs`'s placeholder comment currently says "judge-mom verdict output. Filled by
  task G2-T9." That is now inaccurate — see §3.3.
- `MAX_REASON_CHARS = 200` (`dream.rs:25`) is a decision parameter and moves with the parser.
- `scripts/core-boundaries/rules/crate-layout.mjs:6` registers the crate by path only and does
  **not** enumerate its internal modules, so adding a new top-level module is safe.

## 3. Required design (decided — do not redesign)

### 3.1 Shared fence stripper

Create a new top-level crate module for LLM-output preprocessing — suggested
`src/agentic/src/llm_output.rs`, declared in `lib.rs` alongside the existing modules — exporting
`pub fn strip_json_fence(raw: &str) -> String`.

Rationale for a shared module rather than making `distill::parse::strip_json_fence` public: the
verdict parser lives under `review/`, and having `review` import from `distill` would invent a
sibling dependency between two unrelated domains. A neutral utility module avoids that and
gives T9/T12 an obvious home for further tolerant-parsing helpers.

Move the body verbatim. Both `distill/parse.rs` and `review/verdict.rs` then use it, and
**neither keeps a private copy**.

### 3.2 Generic verdict parser

In `review/verdict.rs`:

```rust
pub struct Verdict {
    pub index: usize,
    pub action: String,
    pub reason: Option<String>,
}

pub const MAX_REASON_CHARS: usize = 200;

pub fn parse_verdicts(json: &str, item_count: usize, allowed_actions: &[&str]) -> Vec<Verdict>;
```

Use a named struct rather than the current `(usize, String, Option<String>)` tuple — three
positional fields at a call site is a readability trap. Test assertions change from `.0/.1/.2`
to field names; what they assert must not change.

`allowed_actions` semantics: an item whose `action` is missing, or is not present in the slice,
is **skipped** — exactly as the current `_ => continue` does. Do not lowercase, trim, or
otherwise normalize the action before comparison; the current code does not.

### 3.3 Fix the placeholder comment

`review/mod.rs` and `review/verdict.rs` currently attribute verdict parsing to judge-mom / T9.
Since this parser is now generic and shared by the dream/garden path (T5b), judge-mom (T9), and
the garden pass (T12), reword those doc comments to describe a **generic** verdict parser whose
action vocabulary is supplied by the caller. Keep it factual — do not claim T9/T12 work is done.

### 3.4 Core side

`dream.rs` calls `parse_verdicts(&text, facts.len(), &["keep", "supersede"])` and adapts the
`Vec<Verdict>` to whatever the existing verdict-application loop (`:143-205`) expects. Keep that
loop's behavior identical, including the `warn!` sites and the
`scanned/superseded/kept/skipped` counters at `:202-203`.

The `"keep"` / `"supersede"` literals stay in core — that is correct and expected; T7a already
deferred the core-side `supersede` boundary rule to T12 for exactly this reason.

## 4. Behavior that must be preserved exactly

Report per row, with how you know:

1. Malformed JSON → empty vector, no panic, and **no log** (`dream.rs:242` swallows the error
   with `Err(_) => return Vec::new()`; note this differs from the distiller, which reports a
   `parse_error` — do **not** "improve" dream to match, that would change behavior).
2. `index >= item_count` → item skipped (`:248-250`).
3. Missing or unknown `action` → item skipped, never defaulted (`:251-254`).
4. `reason` longer than 200 **chars** (not bytes) → truncated to 200; shorter reasons untouched;
   absent reason stays `None` (`:255-261`).
5. Fence tolerance identical for ```` ``` ````, ```` ```json ````, and unfenced input.
6. Output order follows input order, with skipped items simply absent.

## 5. Tests

Move all 6 tests from `dream.rs:289-344` to the crate with the **same names and same input
JSON**: `parse_valid_json_array_maps_fields`, `parse_fence_tolerant`,
`parse_bad_json_returns_empty`, `parse_index_out_of_bounds_skipped`,
`parse_unknown_action_skipped`, `parse_reason_truncated`.

⚠️ Name collision: `distill/parse.rs` already has tests named `parse_valid_json_array_maps_fields`
and `parse_bad_json_returns_empty`. They live in different modules so this compiles, but
`cargo test <filter>` will now match both. That is acceptable; do **not** rename the moved
tests. Just be aware when reporting counts.

Add to the crate:
- a test that an action **not** in `allowed_actions` is skipped while an allowed one in the same
  payload is kept (this is the parameterization's core contract);
- a test that the same payload parsed with two different allow-lists yields different results
  (proves the allow-list is actually consulted rather than ignored);
- a `strip_json_fence` test in its new home if one does not already exist there.

Keep at least one core-side test proving `dream.rs` still applies verdicts end to end. The
`dream` filter is **7** now (6 parse tests + the D9 negative test in `dream_d9_tests.rs`);
report the new number and account for the difference.

## 6. Constraints

- The crate must gain **no** dependency; `src/agentic/Cargo.toml` must stay unchanged.
- **Zero `supersede` (and zero `keep`) literals in the crate.** Grep and show the result.
- Register `MAX_REASON_CHARS` in `src/agentic/AGENTS.md` §4, matching the format T5a used there.
- Do not run `cargo fmt`. English-only, no emoji. Files under 800 lines.
- Warn-only semantics preserved; do not add or remove a log site.

## 7. Verification (paste complete raw stdout+stderr)

Prefix for cargo: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth` — **154** now; report new number
2. `cargo test -p northhing-core --features product-full dream` — 7 now; report new number and explain it
3. `cargo test -p northhing-core --features product-full distill` — 31, unchanged
4. `cargo check -p northhing-core --features product-full` — warning baseline **19**, must not increase
5. `node scripts/check-core-boundaries.mjs` — exit 0
6. Line counts of every file touched or created
7. `rg -n "supersede" src/agentic` — must return **nothing**

## 8. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `refactor(growth): `.
- All source edits in the worktree, never in the main checkout `E:\agent-project\northing`.
- `git status --short` clean when you finish. Do not commit anything under `.superpowers/`.
- Report to **`E:\agent-project\northing\.superpowers\sdd\task-t5b-report.md`** (main repo path,
  NOT the worktree's `.superpowers/`): the §4 checklist row by row, the §6 grep output, full
  verification output, line counts, the AGENTS.md row you added, and anything ambiguous.
- If §3 proves impossible as written, report `BLOCKED` with the specific obstacle rather than
  substituting a different design.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
