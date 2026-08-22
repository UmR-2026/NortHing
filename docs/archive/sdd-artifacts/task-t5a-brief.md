# Task T5a — Move distiller pure logic into the growth crate

Base commit: `2e986ce` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope

Move the distiller's **pure logic** out of
`src/crates/assembly/core/src/service/agent_memory/distiller.rs` (656 lines) into the growth
crate at `src/agentic`, filling the two placeholder modules that already exist for it:

- `src/agentic/src/distill/prompt.rs` (currently a 1-line placeholder) — prompt construction
- `src/agentic/src/distill/parse.rs` (currently a 1-line placeholder) — JSON response parsing

The core side shrinks to a **thin adapter**. **No decision semantics may change.**

**Out of scope**:
- The dream verdict parsing (`dream.rs`) — that is task T5b. Do not touch `dream.rs`.
- LLM orchestration and client/config resolution: `distill_facts_with_llm` (`:58`),
  `read_distiller_config` (`:159`), `resolve_distiller_client` (`:181`),
  `resolve_memory_llm_client` (`:242`), `DISTILL_TIMEOUT_SECS` (`:27`), `DistillResult` (`:36`).
  These are IO and stay in core.
- Any change to the prompt **text** itself (§5.4-A/R-4 already finalized it) or to any
  threshold value.

## 2. Why the naive move is impossible (read this before designing anything)

`parse_distilled_facts` (`distiller.rs:336`) cannot move as-is. Verified facts:

| Obstacle | Where | Why it blocks the move |
|---|---|---|
| `uuid::Uuid::new_v4()` | `:411` | The crate has **no `uuid` dependency** (`src/agentic/Cargo.toml` deps are only async-trait, serde, serde_json, thiserror, tracing) and id generation is not pure |
| `SystemTime::now()` | `:353` | Not pure; the crate is decision logic and must be deterministic |
| Returns `Vec<Fact>` | `:340`, `:409-421` | `Fact`/`FactProvenance`/`FactType`/`FactConfidence`/`FactScope` live in `core/src/service/agent_memory/facts.rs` = **assembly layer**. Per `src/agentic/AGENTS.md:1` the crate may depend only on the contracts layer — it cannot see these types |
| `warn!` on parse failure | `:345` | Established crate convention is that pure logic does not log; the host does. See `src/agentic/src/scheduler.rs:27`, which documents exactly this split ("The legacy host code re-applies the brake and emits a `warn!` log") |
| `Vec<Message>` return | `:260`, `:322-323` | `Message` is `crate::util::types::Message` (`distiller.rs:12`) — core-internal, not visible to the crate |

So the move requires a **neutral crate-side data shape plus a core-side mapping layer**. The
design below is decided; implement it as specified rather than inventing an alternative.

## 3. Required design (decided — do not redesign)

### 3.1 `distill/parse.rs` — crate side

Define neutral types that carry **no id, no timestamp, no provenance**:

```rust
pub enum DistilledFactType { User, Feedback, Project, Reference }
pub enum DistilledConfidence { High, Med, Low }
pub enum DistilledScope { Workspace, Global }

pub struct DistilledFact {
    pub text: String,                    // already trimmed and truncated
    pub fact_type: DistilledFactType,
    pub confidence: DistilledConfidence,
    pub scope: DistilledScope,
}

pub struct DistillParseOutcome {
    pub facts: Vec<DistilledFact>,
    pub keywords: Vec<String>,
    pub was_empty_array: bool,
    pub parse_error: Option<String>,     // replaces the warn! — the host logs it
}
```

Entry point: `pub fn parse_distilled_facts(json: &str) -> DistillParseOutcome`
(note: **no** `session_id` / `turn_id` parameters — provenance is the host's job).

Also move `strip_json_fence` (`:427`), `RawDistilledFact` (`:454`), and the custom
`deserialize_keywords` (`:474`) into this module.

Naming: pick names consistent with the crate's existing style; the shapes above are what
matters. Note the confidence variant is **`Med`, not `Medium`** — this mirrors the existing
`FactConfidence::Med` and the JSON value `"med"`.

### 3.2 `distill/prompt.rs` — crate side

Move `build_distillation_messages` (`:260`) as a function returning a neutral shape, e.g.:

```rust
pub struct DistillPrompt { pub system: String, pub user: String }
pub fn build_distill_prompt(user_input: &str, last_assistant_text: Option<&str>) -> DistillPrompt;
```

The core side then builds `vec![Message::system(p.system), Message::user(p.user)]`.

The prompt **text** must be byte-identical to what `distiller.rs` produces today, including
the `<user_message>` / `<assistant_reply>` anti-injection wrapping and the
`MAX_ASSISTANT_TEXT_CHARS` truncation (`:317`).

### 3.3 Constant ownership

These move to the crate (they are decision parameters) and must be `pub` so the host can
still reference them if needed: `MAX_DISTILL_FACTS` (3), `MAX_FACT_TEXT_CHARS` (300),
`MAX_ASSISTANT_TEXT_CHARS` (500).

`MIN_USER_INPUT_CHARS` (20) gates whether the LLM is called at all — that is orchestration,
so it **stays in core**.

Per `src/agentic/AGENTS.md:4` ("no magic numbers scattered across the crate; all thresholds
must be concentrated in per-module constants and registered here"), you must also **add these
three parameters to the AGENTS.md §4 table** with their values and meaning.

### 3.4 Core side becomes a thin adapter

`distiller.rs` keeps a private function with the same name and signature as today
(`parse_distilled_facts(json, session_id, turn_id) -> (Vec<Fact>, Vec<String>, bool)`) so
`distill_facts_with_llm` (`:131`) needs no change beyond what the mapping requires. It must:

1. call the crate parser,
2. `warn!` if `parse_error` is `Some` — preserving today's log semantics and message intent,
3. map each `DistilledFact` to a `Fact`, filling `schema_version: 1`, a fresh
   `uuid::Uuid::new_v4()` id, `provenance` from `session_id`/`turn_id`, and `created_at` from
   `SystemTime::now()` exactly as `:353-356` computes it (same `unwrap_or(0)` fallback),
4. map the three crate enums to `FactType` / `FactConfidence` / `FactScope`.

Write the enum mapping as **explicit exhaustive `match`** (no catch-all arm) so that adding a
variant on either side becomes a compile error rather than a silent misroute.

## 4. Behavior that must be preserved exactly (equivalence checklist)

Your report must state, per row, that behavior is unchanged and how you know:

1. Max 3 facts, and the cap is checked **before** processing each item (`:361`).
2. `text`: `trim()`, skip if empty after trim, then truncate to 300 **chars** (not bytes) (`:364-373`).
3. Unknown/missing `type` → **skip that item** (not default) (`:374-380`).
4. Unknown/missing `confidence` → skip the item; accepted values are `high`/`med`/`low` (`:381-386`).
5. Unknown/missing `scope` → skip the item; accepted values are `workspace`/`global` (`:387-391`).
6. Keywords: turn-level union, trimmed, empties dropped, **order-preserving dedup, first
   occurrence wins** (`:397-407`).
7. `was_empty_array` is `true` **only** when the LLM returned a syntactically valid empty
   array — and `false` on parse failure (D15 observability; `:346`, `:351`).
8. `deserialize_keywords` **never returns `Err`**: a string, number, object, or an array
   containing non-strings all degrade to "no keywords" while the surrounding facts still
   parse (`:474-492`). This was Important finding I-1 in task R-4 — regressing it silently
   drops every fact of the turn.
9. `strip_json_fence` handling of ```` ```json ```` fences (`:427`).
10. Prompt text byte-identical, including assistant-text truncation at 500 chars.

## 5. Tests

- **Move the parse-semantics tests to the crate.** `distiller.rs:493-656` holds 13 tests
  (`parse_valid_json_array_maps_fields`, `parse_json_fence_wrap`, `parse_bad_json_returns_empty`,
  `parse_four_items_truncates_to_three`, `parse_unknown_fact_type_skipped_valid_kept`,
  `parse_text_over_300_chars_truncated`, `parse_empty_array_returns_empty`,
  `parse_keywords_union_dedup_across_items`, `parse_legacy_json_without_keywords_still_works`,
  and the four `parse_keywords_wrong_type_*` / `parse_keywords_array_with_non_string_elements_*`
  cases). Each must survive in the crate with the **same name and the same input JSON**.
  Assertions necessarily change shape (`DistilledFact` instead of `Fact`) — that is expected;
  what may not change is *what* is asserted.
- **Add core-side adapter tests** (at least): a fact gets a non-empty unique id, `created_at`
  is populated, `provenance` carries the given `session_id`/`turn_id`, `schema_version == 1`,
  and each enum value maps to the right `Fact*` variant.
- Add a crate-side test that the prompt contains the `<user_message>` wrapper and that a
  600-char assistant text is truncated to 500.

## 6. Constraints

- `src/agentic` must not gain any dependency. No `uuid`, no `rusqlite`, no core types.
  `Cargo.toml`'s prohibited list is authoritative.
- Do not run `cargo fmt`. English-only, no emoji.
- Production `.rs` files stay under 800 lines. Report the line count of every file you touch
  or create.
- Warn-only semantics preserved; do not add or remove a log site other than the `warn!` move
  described in §3.4.
- Do not change any threshold value.

## 7. Verification (paste complete raw stdout+stderr)

Prefix for cargo: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth` — **139** now; must be 139 + the tests you moved/added
2. `cargo test -p northhing-core --features product-full distill` — 13 now; report the new number
3. `cargo test -p northhing-core --features product-full auto_memory` — 8, unchanged
4. `cargo test -p northhing-core --features product-full turn_persist` — 12, unchanged
5. `cargo check -p northhing-core --features product-full` — warning baseline **19**, must not increase
6. `node scripts/check-core-boundaries.mjs` — exit 0
7. Line counts via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count`

### 7.1 Prove the prompt text did not drift

The prompt is the highest-risk part of this move: a stray whitespace or reflow changes model
behavior and no test will notice. So: before your change, capture the rendered prompt (both
system and user parts, for an input with and without assistant text); after your change,
capture it again from the new code path; and paste a comparison showing them **identical**.
State explicitly how you compared (byte length + exact equality check, not eyeballing).

## 8. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `refactor(growth): `.
- All source edits in the worktree `E:\agent-project\northing\.worktrees\growth-core-0804`,
  never in the main checkout.
- `git status --short` clean when you finish. Do not commit anything under `.superpowers/`.
- Report to **`E:\agent-project\northing\.superpowers\sdd\task-t5a-report.md`** (main repo
  path, NOT the worktree's `.superpowers/`): the §4 equivalence checklist row by row, the §7.1
  prompt comparison, full verification output, all line counts, the AGENTS.md §4 parameter
  rows you added, and anything ambiguous.
- If the design in §3 turns out to be impossible as written, stop and report `BLOCKED` with
  the specific obstacle — do not silently substitute a different design.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
