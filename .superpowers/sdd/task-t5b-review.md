# Review — T5b: dream verdict parsing moved into the growth crate

## Verdicts

- **SPEC compliance: PASS** — All three §1 changes land, §3 design implemented as specified, §4 behavior checklist met row by row, §5 tests moved + new coverage added, §6 constraints honored, §7 numbers match the orchestrator's independent verification.
- **CODE QUALITY: PASS** — Behavior-preserving refactor; loop body is byte-identical (apart from destructuring), `strip_json_fence` consolidation is faithful, parameterization is sound, crate hygiene matches the T5a bar. A few Minor notes (none worth a fix round).

---

## Priority 1 — The fixture-vocabulary substitution (the judgment call)

The contradiction between §5 ("same input JSON") and §6/§7 ("zero `supersede`/`keep` literals in the crate") is real, not a misreading. Three of the six original fixtures contain the literal `"keep"` and one contains `"supersede"`; those literals cannot survive both in the crate test files and inside `parse_verdicts`'s allow-list argument (`&["keep", "supersede"]`) at the call site. §7's `rg -n "supersede" src/agentic` is a grepable constraint that literally runs against `*.rs` files including `#[cfg(test)]` modules.

**My judgment: the implementer's resolution is correct, and the substitution was genuinely necessary.**

### 1a. Do the rewritten crate tests still exercise the same code paths?

Yes, fixture by fixture against the deleted originals in `dream.rs:289-344` (visible in `git show 71df0dd`):

| Original test | Old action | Old allow-list | New action | New allow-list | Path exercised |
|---|---|---|---|---|---|
| `parse_valid_json_array_maps_fields` | `"keep"` | hard-coded | `"accept"` | `&["accept", "reject"]` | same: allowed action, all 3 fields set |
| `parse_fence_tolerant` | `"supersede"` | hard-coded | `"reject"` | `&["accept", "reject"]` | same: fence strip → allowed action |
| `parse_bad_json_returns_empty` | n/a | n/a | n/a | n/a | identical: `"not json at all"` |
| `parse_index_out_of_bounds_skipped` | `"keep"` | hard-coded | `"accept"` | `&["accept", "reject"]` | same: index 5 ≥ item_count 2 |
| `parse_unknown_action_skipped` | `"maybe"` | hard-coded | `"maybe"` | `&["accept", "reject"]` | same: action not in allow-list |
| `parse_reason_truncated` | `"keep"` | hard-coded | `"accept"` | `&["accept", "reject"]` | same: 250 ASCII chars → truncate to 200 |

For every test, the action vocabulary is irrelevant to the path being asserted — the only thing that matters is whether the action is in the allow-list, and the new allow-list consistently contains the new vocabulary. No behavior is lost.

### 1b. Is any behavior now untested? Specifically, is the dream vocabulary still covered?

`apply_verdicts_applies_keep_and_supersede_end_to_end` (`dream.rs:261-326`) **genuinely closes the gap**, not merely appears to. It calls `parse_verdicts` with the literal dream allow-list `&["keep", "supersede"]` over a payload containing `"keep"` (index 0) and `"supersede"` (index 1), then calls `apply_verdicts` against an isolated `MemoryDb` and asserts four things:

1. `(scanned, superseded, kept, skipped) == (2, 1, 1, 0)` — the `apply_verdicts` counters map to the parsed verdicts correctly.
2. `db.get_facts(None)` contains `f-t5b-a` (the kept fact) but not `f-t5b-b` (the superseded one) — i.e. `supersede_fact` actually flipped the row's status to `'superseded'`, which `get_facts(None)` filters out (`memory_db.rs:285-294`).
3. `db.reviews_for_fact("f-t5b-a")` returns one row with `action == "keep"` and `reviewer == "dream"`.
4. `db.reviews_for_fact("f-t5b-b")` returns one row with `action == "supersede"` and `reviewer == "dream"`.

This exercises the entire `parse_verdicts → apply_verdicts → MemoryDb` chain end to end with the real vocabulary. The crate-side tests still cover `parse_verdicts` over the neutral vocabulary plus the parameterization contract (`parse_disallowed_action_skipped_allowed_kept`, `parse_allow_list_changes_results`). Coverage is complete; the only thing the crate tests cannot prove is the `parse_verdicts(["keep","supersede"])` call site on the host side, which the new core test covers.

### 1c. Could both constraints have been satisfied by scoping the rule to non-test code?

No, not in the way the brief is written. The §7 grep is `rg -n "supersede" src/agentic` — a flat regex over every `.rs` file including `#[cfg(test)] mod tests`. There is no `tests/`-exclude argument, and §6 phrases the rule as "Zero `supersede` literals in the crate", which on a plain reading includes test files. The brief is internally inconsistent and only the implementer's resolution honors both clauses as written. I would have chosen the same path.

If the plan author wants to relax the grep to exclude tests, that is a future plan amendment, not a critique of the implementer. Worth raising at the orchestrator level: the brief's contradiction is a genuine defect in the plan, not in the implementation.

---

## Priority 2 — Parameterization correctness

### Verbatim comparison
`parse_verdicts` (`verdict.rs:47-50`):
```rust
let action = match item.action.as_deref() {
    Some(a) if allowed_actions.contains(&a) => item.action.unwrap_or_default(),
    _ => continue,
};
```

`Vec<&str>::contains(&a)` calls `PartialEq::eq` between `&&str` and `&&str`, which delegates to `str::eq` on the inner strings. That is verbatim byte comparison. **No trimming, no lowercasing, no normalization introduced.** The original `dream.rs:251-254` had `Some("keep") | Some("supersede")`, which is also verbatim comparison; the refactored guard preserves that contract exactly.

### Missing-action handling
The match arm `Some(a) if ... => item.action.unwrap_or_default()` is only entered when `item.action` is `Some(_)`. The arm cannot fire on `None` — that falls through to `_ => continue`. So a missing `action` field is still skipped, never defaulted. The `unwrap_or_default()` is redundant inside the arm (the guard proves `Some`) but **cannot** turn a `None` into `""` that fails the allow-list for a different reason — same outcome as before, same code path.

Note on `unwrap_or_default()` style: this is verbatim from the original `dream.rs:254`; the implementer preserved the original idiom rather than "improving" it. That's the right call for a behavior-preserving refactor. See Minor finding M1 below.

### Allow-list is genuinely consulted
Two tests in the crate prove it: `parse_disallowed_action_skipped_allowed_kept` (`verdict.rs:133-145`) — same payload with `&["accept"]` keeps only the `accept` row, the `delete` row is skipped; and `parse_allow_list_changes_results` (`verdict.rs:147-160`) — same payload with `&["accept"]` vs `&["accept", "reject"]` yields lengths 1 vs 2. Both would fail if the allow-list were ignored. ✓

---

## Priority 3 — `strip_json_fence` consolidation

### Byte-identical to both deleted copies

The body of `llm_output.rs:12-25` matches the deleted `dream.rs:267-280` (visible in `git show 71df0dd:src/crates/assembly/core/src/service/agent_memory/dream.rs`) and the deleted `distill/parse.rs:158-171` (visible in `git show 71df0dd:src/agentic/src/distill/parse.rs`). All three are character-for-character the same:

```rust
pub fn strip_json_fence(json: &str) -> String {
    let mut s = json.trim();
    if s.starts_with("```") {
        s = &s[3..];
        if s.starts_with("json") {
            s = &s[4..];
        }
        s = s.trim_start();
    }
    if s.ends_with("```") {
        s = &s[..s.len() - 3];
    }
    s.trim().to_string()
}
```

(Visibility differs: `pub` in the shared module, private in the old copies. Functionally identical.)

### Branches walked

- Bare ```` ``` ```` fence (`"```\n[1, 2]\n```"`) — strip leading, no `json` branch fires, trim trailing. Covered by `strip_json_fence_plain_fence_removed` (`llm_output.rs:38-41`).
- ```` ```json ```` fence (`"```json\n[1, 2]\n```"`) — strip leading, enter `json` branch, trim trailing. Covered by `strip_json_fence_flagged_fence_removed` (`llm_output.rs:43-47`).
- Unfenced — neither branch fires, just trim. Covered by `strip_json_fence_unfenced_passes_through` (`llm_output.rs:31-35`).
- String starting and ending with a fence — same as the bare-fence case; covered.
- Whitespace-only — `trim()` returns `""`, neither branch fires, returns `""`. Preserved.
- Fence in the middle (`"prefix ```[1,2]``` suffix"`) — neither `starts_with("```")` nor `ends_with("```")` fires, body returned as-is. **Behavior preserved** (the brief doesn't require special handling here; the function only strips boundary fences).

### Distiller call site

`distill/parse.rs:9` adds `use crate::llm_output::strip_json_fence;` and the call at `distill/parse.rs:75` (`let cleaned = strip_json_fence(json);`) is unchanged in argument and result handling. The `distill` suite is unchanged at 31 — confirmed by the orchestrator's run. ✓

### Module home and registration

`llm_output.rs` lives at `src/agentic/src/llm_output.rs` (new top-level crate module), registered in `lib.rs:15` between `garden` and `negation` (alphabetical, consistent with the surrounding `pub mod` lines). The module-level doc comment (`llm_output.rs:1-5`) correctly identifies it as a neutral utility shared by every parser and notes "No logging and no domain vocabulary here". Good.

---

## Priority 4 — The out-of-scope extraction

### Behavior preservation

Verified via `git diff -w 71df0dd 8b64aa8 -- src/crates/assembly/core/src/service/agent_memory/dream.rs`. The orchestrator's claim that the loop body is byte-identical apart from the `for` line destructuring is correct:

- Old: `for (idx, action, reason) in verdicts {`
- New: `for Verdict { index: idx, action, reason } in verdicts {`

Every other line in the apply loop is byte-identical:
- `scanned += 1` at start — same.
- `skipped += 1` on `idx >= candidates.len()` — same.
- `"supersede"` arm: `supersede_fact` then `record_fact_review` with `action: "supersede"`, with the same two `warn!` messages ("failed to supersede fact", "failed to record supersede review") — same.
- `"keep"` arm: `record_fact_review` with `action: "keep"`, with the same `warn!` ("failed to record keep review") — same.
- `_ => skipped += 1` — same.

`set_judge_state` and the summary `info!` still run **after** the apply returns (`dream.rs:145-150`), in the same order: apply → set_judge_state → info! sweep summary. The original ordering is preserved exactly.

### Justification soundness

Sound. To test `parse_verdicts → apply_verdicts → MemoryDb` end to end without the extraction, you would have to drive the entire `async fn run_dream_sweep` path, which opens a real `MemoryDb` and talks to a real LLM. The extraction is the minimal change that exposes the apply logic for a synchronous unit test against an isolated DB. Without it, the brief's §5 "at least one core-side test proving `dream.rs` still applies verdicts end to end" would have required either mocking the LLM/DB layer or moving the apply logic into the crate — both larger changes.

### Doc comment accuracy

`dream.rs:153-158` says: "a `supersede` verdict supersedes the fact and records a supersede review; a `keep` verdict records a keep review; any other action is counted as skipped". Accurate at the level of a one-line summary. The doc does not claim `scanned` is incremented per verdict — and it isn't, it's incremented unconditionally inside the loop body, so this is fine.

One thing the doc comment understates: on `supersede` failure the review row is still recorded (the warn! is the failure signal, the row is still inserted). This is identical to the old behavior — the original inline loop did the same — so calling it out would have been an "improvement" the brief forbids. Acceptable.

---

## Priority 5 — Crate hygiene

### No new dependency
`src/agentic/Cargo.toml` unchanged (`git diff HEAD -- src/agentic/Cargo.toml` is empty). Confirmed.

### No logging in the crate
`llm_output.rs` and `verdict.rs` have no `use tracing` / `use log` and contain no macro calls that log. Confirmed by `grep -E "log|tracing" src/agentic/src/llm_output.rs src/agentic/src/review/verdict.rs` — only doc-comment matches. The pre-existing `state.rs:6 use tracing::warn;` is not in this task's diff.

The deliberate asymmetry with T5a (dream silently returns empty on malformed JSON; distiller surfaces `parse_error`) is preserved. The new code is:
```rust
let raw: Vec<RawVerdict> = match serde_json::from_str(&cleaned) {
    Ok(v) => v,
    Err(_) => return Vec::new(),
};
```
No `parse_error` channel introduced. ✓ The doc comment on `parse_verdicts` (`verdict.rs:26-29`) explicitly notes the asymmetry — good.

### `MAX_REASON_CHARS` registered

`AGENTS.md:41`: `| `review::verdict::MAX_REASON_CHARS` | 200 | max chars per verdict reason |` — matches the T5a row format exactly (path-as-backticked-identifier, numeric value, short description). The preamble sentence at `AGENTS.md:30-31` is also updated to mention `review/verdict.rs`. ✓

### Truncation by chars, not bytes
`verdict.rs:51-57`:
```rust
let reason = item.reason.map(|r| {
    if r.chars().count() > MAX_REASON_CHARS {
        r.chars().take(MAX_REASON_CHARS).collect()
    } else {
        r
    }
});
```
Char-based. Identical to the original `dream.rs:255-261`. Doc on the constant (`verdict.rs:12-13`) and on `parse_verdicts` (`verdict.rs:31-32`) both say "chars, not bytes". ✓

**M2 (Minor)** — `parse_reason_truncated` (`verdict.rs:122-131`) uses ASCII `"a".repeat(250)`. A multi-byte fixture (CJK or emoji) would prove the char-vs-byte distinction holds for non-ASCII. Not a regression (the original `dream.rs` test was also ASCII-only) and the implementer correctly preserved existing coverage; just worth noting that the char-based claim is asserted by the `chars().count() == MAX_REASON_CHARS` equality rather than by a contrasting bytes-vs-chars failure. A future test could close this.

### `Verdict` as public API

`verdict.rs:17-22`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub index: usize,
    pub action: String,
    pub reason: Option<String>,
}
```

Public fields, derives `Debug` + `Clone` + `PartialEq` + `Eq`. Reasonable for a library crate — T9/T12 can consume it via pattern match (`let Verdict { index, action, reason } = verdict;`), `Debug` it for assertion failure messages, and `Clone` it if needed. `Hash` is missing but not required by the brief. Acceptable.

**M1 (Minor)** — `verdict.rs:48` `item.action.unwrap_or_default()` is redundant: the guard `Some(a) if allowed_actions.contains(&a)` has already proven `item.action` is `Some(_)`. Idiomatic Rust would write `a.to_string()` or `item.action.unwrap()`. The implementer preserved the original idiom from `dream.rs:254` verbatim, which is the right call for a behavior-preserving refactor — flagging only so a future task can tidy if it touches this code.

### Doc comments reworded

`review/mod.rs:1-3`: "Review decisions: merge-with-boost, routing, and generic verdict parsing. The verdict parser is action-agnostic: the caller supplies the allowed action vocabulary, so no judge-specific semantics live in this module." — factual, no claim that T9/T12 work exists. ✓

`review/verdict.rs:1-7`: "Generic parsing of LLM verdict output. The parser is deliberately action-agnostic: callers pass the allowed action vocabulary, so this module carries no action semantics of its own and can be shared by every verdict-producing path. There is no IO and no logging here; the host adapts the neutral [`Verdict`]s to its own storage types at the call site." — factual, "can be shared" is a capability claim not a completion claim. ✓

---

## Findings by severity

### Critical
None.

### Important
None.

### Minor

- **M1** `src/agentic/src/review/verdict.rs:48` — `item.action.unwrap_or_default()` is redundant inside the `Some(a) if allowed_actions.contains(&a)` arm (the guard proves `Some`). Idiomatic: `a.to_string()` or `item.action.unwrap()`. Preserved from the original `dream.rs:254`; behavior identical. No fix needed.

- **M2** `src/agentic/src/review/verdict.rs:122-131` — `parse_reason_truncated` uses ASCII `"a"`. A multi-byte (CJK or emoji) fixture would actively prove the char-vs-byte distinction. Pre-existing gap carried forward; not a regression. No fix needed.

---

## Cannot verify from diff

Nothing in this review falls beyond what the diff and full worktree files settle. The orchestrator's test runs (crate 154 → 165, dream 7 → 2, distill 31 unchanged, warnings 19 unchanged, boundary check exit 0) were treated as settled per the brief's ground rules and not re-run.