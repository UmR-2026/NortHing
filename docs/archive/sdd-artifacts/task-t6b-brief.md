# Task T6b — Wire the two-layer score into fact retrieval ranking

Base commit: `4f7ba93` (branch `feat/growth-core-0804`, worktree `E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Why

`src/agentic/src/topics/score.rs` (410 lines, fully tested) implements the two-layer
retrieval score, but **nothing calls it**. Topic weights are being boosted and decayed
every turn (T6a, R-4) yet retrieval ranking still ignores the crate formula. Until this
task lands, all the weight work is a dead signal.

`search_facts` is the single path that feeds the `# Remembered facts` prompt block
(`auto_memory.rs:312`, top 5). Changing its ranking changes what the agent sees, so this
task is behavior-visible and requires a before/after comparison (see §6).

## 2. Current state (verified, do not re-derive)

`src/crates/assembly/core/src/service/agent_memory/memory_db.rs:531-553`:

```rust
let keyword_weight = keyword_map
    .iter()
    .filter(|(kw, _)| { /* kw >= 2 chars && token overlap with fact text */ })
    .map(|(_, w)| *w)
    .fold(1.0, f64::max);

let bm25_pos = -rank;
let days = ((now_ms.saturating_sub(last_mentioned_at as u64)) as f64 / 86_400_000.0).max(1.0);
let recency_boost = 1.0 + 0.1 * (1.0 / days);
let score = bm25_pos * keyword_weight * recency_boost;
```

Facts are then sorted by `score` descending and truncated to `limit`. Nothing is dropped.

Raw topic weights live in `[1.0, 5.0]`: floor 1.0, `boost_keyword` does
`(weight + 1.0).min(5.0)` at `memory_db.rs:608`, daily decay `x0.99`.

## 3. Required change

### 3.1 Formula

```
tw_norm     = max over matched keywords of (raw_weight / 5.0)      // empty -> 0.2, see 3.2
entry_score = f(fact.confidence)                                   // see 3.3
two_layer   = northhing_agentic_growth::topics::score::retrieval_score(tw_norm, entry_score)
score       = bm25_pos * two_layer * recency_boost
```

`retrieval_score` already computes `tw * (ENTRY_FLOOR + ENTRY_SPAN * entry_score)` and
sanitizes both inputs, so NaN cannot propagate. **Use it; do not re-implement the
arithmetic.** The crate is the single source of truth for this formula.

`bm25_pos` and `recency_boost` keep their current definitions and stay as multipliers.
This is a deliberate three-factor design (orchestrator decision 1a): the two-layer score
is a *priority modulator*, while bm25 remains the relevance gate. Dropping bm25 would
rank hot-topic-but-irrelevant facts first.

### 3.2 Normalization and the empty case (highest-risk detail)

Normalize each matched raw weight by `/ 5.0` **before** feeding the crate, because
`best_topic_weight` / `sanitize_unit` expect `0.0..=1.0` and would clamp a raw `5.0` to
`1.0` and a raw `3.0` to `1.0` as well — silently destroying all weight resolution.

You may use `best_topic_weight(&normalized_vec)` or compute the max directly, but:

- **When no keyword matches, `tw_norm` MUST be `1.0 / 5.0` (= 0.2), not `0.0`.**
  `best_topic_weight` returns `0.0` for an empty slice; `0.0` makes `two_layer` `0.0`,
  which zeroes the whole score and makes facts on never-boosted topics unrankable.
  This mirrors today's `fold(1.0, f64::max)` starting value (an unmatched fact behaves
  like an unboosted topic). Getting this wrong silently hides memories for any topic the
  user has not mentioned twice — it is the single most damaging mistake available in this
  task. Express the fallback via the same `/ 5.0` normalization of the `1.0` floor, not as
  a bare `0.2` literal.
- Do **not** call `rank_candidates`. It drops candidates below `RETRIEVAL_FLOOR` and
  re-sorts by its own tie-break, which would discard bm25 and change deletion semantics.
  Only `retrieval_score` (and optionally `sanitize_unit` / `best_topic_weight`) are in
  scope. Nothing may be dropped by this change: keep sort + `truncate(limit)` as the only
  filtering, exactly as today.

### 3.3 confidence -> entry_score

Map the three confidence variants parsed at `memory_db.rs:~490-500`:

| confidence | entry_score |
|---|---|
| high | `1.0` |
| medium | `0.6` |
| low | `0.3` |

Define these as named constants next to the call site (not magic numbers inline). State
the exact variant identifiers you matched in the report.

Note for context (do not implement): a later task (T10) will promote `entry_score` from
repeated evidence. Keep the mapping in one place so that change is local.

### 3.4 `ScoredFact` observability

Add `topic_weight_norm: f64`, `entry_score: f64`, `two_layer: f64` to `ScoredFact`
(`memory_db.rs:12-18`). **Keep the existing `keyword_weight` field** carrying the raw
un-normalized max — `memory_db_tests.rs` and future debugging rely on it. Keep `bm25`,
`recency_boost`, `score` unchanged in meaning.

## 4. Constraints

- Rust edition/toolchain unchanged. warn-only; **never run `cargo fmt`**.
- English-only in code, comments, and log strings; no emoji. Chinese literals are allowed
  only inside test data.
- No `unwrap` / `expect` / `panic!` / `todo!` in non-test code.
- Do **not** modify `scripts/core-boundaries/**`. If a boundary rule blocks importing
  `northhing_agentic_growth` from `service/agent_memory/**`, stop and report `BLOCKED`
  with the exact checker output instead of editing rules. (Expected to be allowed: the
  crate is in `noCoreDependencyCrates`, which constrains crate->core, not core->crate.
  `turn_persist_facts.rs` already imports it from inside core.)
- `memory_db.rs` is **already 918 lines, over the 800-line limit**. This is a known
  pre-existing violation owned by T7. Do **not** attempt to split it here. Keep net
  production growth in that file **<= 25 lines**; put all new tests in
  `memory_db_tests.rs` (692 lines, must stay < 800).
- Mind the crate-level `#![allow(dead_code)]` and `#![allow(unused_imports)]` in core's
  `lib.rs:3-4`: a clean warning count does **not** prove you left no dead imports. Check
  each symbol you add or remove by hand.

## 5. Tests (add to `memory_db_tests.rs`)

Cover at minimum:

1. **Unmatched-topic fact stays retrievable** — a fact whose topics were never boosted
   still gets a non-zero score and is returned. This is the §3.2 trap guard; assert on a
   strictly positive score, not merely on presence.
2. **Weight cap** — a raw weight of `5.0` yields `tw_norm == 1.0`; a raw `3.0` yields
   `0.6` (proves resolution is not clamped away).
3. **Topic weight ordering** — with bm25 and recency equal, a higher-weight topic ranks
   above a lower-weight one.
4. **Confidence ordering** — with topic weight equal, `high` ranks above `medium` above
   `low`.
5. **Topic dominance** — a sufficiently larger topic weight outranks a better confidence
   (the crate exposes `TOPIC_DOMINANCE_RATIO`; construct the case from it rather than
   hardcoding a ratio).
6. **No silent drop** — a fact with the lowest possible combination (`tw_norm` fallback +
   `low` confidence) is still present in results when `limit` allows.

## 6. Verification (paste **complete raw stdout+stderr**, not excerpts)

Prefix: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-agentic-growth` (139 now; must stay 139 — this task adds no
   crate code)
2. `cargo check -p northhing-core --features product-full` — warning baseline is **19**,
   must not increase
3. `cargo test -p northhing-core --features product-full memory_db` (21 now; report new total)
4. `cargo test -p northhing-core --features product-full auto_memory` (report total)
5. `cargo test -p northhing-core --features product-full growth_adapter` (30 now)
6. `node scripts/check-core-boundaries.mjs` — exit 0
7. Line counts via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count` for
   `memory_db.rs` and `memory_db_tests.rs` (`Measure-Object -Line` under-reports; do not
   use it)

### 6.1 Mandatory before/after ranking comparison

The plan's T6 acceptance criterion is that a change to injected prompt content must come
with a before/after comparison. Build a fixed corpus of >= 6 facts with varied
confidence and topic weights, run the top-5 retrieval for one query under the old formula
and the new one, and put **both orderings side by side** in the report as a table
(rank, fact id, bm25, tw_norm, entry_score, final score). A throwaway test or a
temporarily duplicated old-formula helper is fine; do not leave the old formula in the
shipped code.

Explicitly call out in the report: which facts changed rank, and whether any fact that
was in the old top-5 fell out of the new top-5.

## 7. Deliverables

- One commit on `feat/growth-core-0804`, message prefixed `feat(growth): `.
- `git status --short` clean before you finish. Do **not** commit anything under
  `.superpowers/`.
- Write your report to
  `E:\agent-project\northing\.superpowers\sdd\task-t6b-report.md` with: what you changed
  (file:line), the confidence variant identifiers you matched, the §6.1 comparison table,
  full verification output, line counts, and anything you judged ambiguous.
- End the report with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or
  `BLOCKED`.
