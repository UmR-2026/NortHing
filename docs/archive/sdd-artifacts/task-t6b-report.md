# Task T6b Report — Wire the two-layer score into fact retrieval ranking

## Status

**DONE**

## What changed

### `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`

1. **`ScoredFact` struct (lines 12-21)** — Added three observability fields:
   - `topic_weight_norm: f64` — raw keyword weight normalized by `/5.0`
   - `entry_score: f64` — confidence-mapped entry score
   - `two_layer: f64` — output of `retrieval_score(tw_norm, entry_score)`
   
   The existing `keyword_weight` field is kept carrying the raw un-normalized max, as required.

2. **Entry-score constants + helper (lines 32-44)** — Named constants and a helper function placed next to the call site (module level, after `FactReview`):
   ```rust
   const ENTRY_SCORE_HIGH: f64 = 1.0;
   const ENTRY_SCORE_MED: f64 = 0.6;
   const ENTRY_SCORE_LOW: f64 = 0.3;
   
   fn entry_score_for_confidence(c: &FactConfidence) -> f64 { ... }
   ```

3. **`search_facts` scoring block (lines 563-578)** — Replaced the old `score = bm25_pos * keyword_weight * recency_boost` with the three-factor design:
   ```rust
   let tw_norm = (keyword_weight / 5.0).max(1.0 / 5.0);
   let entry_score = entry_score_for_confidence(&fact.confidence);
   let two_layer = northhing_agentic_growth::topics::score::retrieval_score(tw_norm, entry_score);
   // score: bm25_pos * two_layer * recency_boost (inlined in ScoredFact push)
   ```

   - `bm25_pos` and `recency_boost` keep their current definitions and stay as multipliers.
   - The crate's `retrieval_score` is the single source of truth for the two-layer arithmetic; no re-implementation.
   - `rank_candidates` is **not** called; filtering remains `sort_by` + `truncate(limit)` only. No candidate is dropped.

### Confidence variant identifiers matched

The three variants of `FactConfidence` (defined at `facts.rs:33-37`):
- `FactConfidence::High` → `ENTRY_SCORE_HIGH` (1.0)
- `FactConfidence::Med` → `ENTRY_SCORE_MED` (0.6)
- `FactConfidence::Low` → `ENTRY_SCORE_LOW` (0.3)

Note: the enum variant is `Med`, not `Medium`.

### `src/crates/assembly/core/src/service/agent_memory/memory_db_tests.rs`

Added 7 tests (6 spec-required + 1 before/after comparison):
1. `unmatched_topic_fact_stays_retrievable` — §5.1: unmatched fact has `score > 0` and `tw_norm == 1.0/5.0`.
2. `weight_cap_preserves_resolution` — §5.2: raw 5.0 → `tw_norm 1.0`; raw 3.0 → `tw_norm 0.6`.
3. `topic_weight_orders_ranking` — §5.3: equal bm25/recency, higher weight ranks first.
4. `confidence_orders_ranking` — §5.4: equal topic weight, High > Med > Low.
5. `topic_dominance_outranks_confidence` — §5.5: larger weight (Low conf) outranks smaller (High conf), ratio constructed from `TOPIC_DOMINANCE_RATIO`.
6. `lowest_combination_not_dropped` — §5.6: lowest combination (tw fallback + Low conf) still present.
7. `before_after_ranking_comparison` — §6.1: before/after top-5 comparison (see below).

## §3.2 fallback (highest-risk detail)

When no keyword matches, `keyword_weight` stays at the `fold(1.0, f64::max)` base value of `1.0` (the unboosted floor). Dividing by `5.0` yields `tw_norm = 0.2`, never `0.0`. The `.max(1.0 / 5.0)` is a defensive clamp expressing the fallback via the same `/5.0` normalization of the `1.0` floor — no bare `0.2` literal. This mirrors today's `fold(1.0, f64::max)` starting value.

## §6.1 Before/after top-5 ranking comparison

Corpus (6 facts, all share query token "corpus", identical `created_at` so `recency_boost` is equal):

| id | raw_w | tw_norm | confidence | entry_score | old_mult | new two_layer |
|----|-------|---------|------------|-------------|----------|---------------|
| fA | 5.0   | 1.0     | Low        | 0.3         | 5.0      | 1.0×(0.6+0.4×0.3)=0.72 |
| fB | 4.0   | 0.8     | High       | 1.0         | 4.0      | 0.8×1.0=0.80 |
| fC | 3.0   | 0.6     | Med        | 0.6         | 3.0      | 0.6×0.84=0.504 |
| fD | 2.0   | 0.4     | High       | 1.0         | 2.0      | 0.4×1.0=0.40 |
| fE | 1.0   | 0.2     | High       | 1.0         | 1.0      | 0.2×1.0=0.20 |
| fF | 1.0   | 0.2     | Low        | 0.3         | 1.0      | 0.2×0.72=0.144 |

Raw test output (old sort = OLD formula `bm25_pos * keyword_weight * recency_boost`; new_s = NEW `bm25_pos * two_layer * recency_boost`):

```
=== §6.1 top-5 (old sort) | r|id|bm25|kw_w|tw_n|entry|old_s|new_s ===
1|fA|-0.0000|5.0000|1.0000|0.3000|0.0000050|0.0000008
2|fB|-0.0000|4.0000|0.8000|1.0000|0.0000040|0.0000007
3|fC|-0.0000|3.0000|0.6000|0.6000|0.0000030|0.0000005
4|fD|-0.0000|2.0000|0.4000|1.0000|0.0000020|0.0000004
5|fE|-0.0000|1.0000|0.2000|1.0000|0.0000010|0.0000002
old5:["fA", "fB", "fC", "fD", "fE"] new5:["fB", "fA", "fC", "fD", "fE"] fell:[]
```

### Which facts changed rank

- **fA and fB swapped positions**: In the OLD formula, fA ranked #1 (highest `keyword_weight` 5.0). In the NEW formula, fB ranks #1 because `two_layer(fB) = 0.80 > two_layer(fA) = 0.72`. This is correct: `tw_high/tw_low = 1.0/0.8 = 1.25 < TOPIC_DOMINANCE_RATIO (1.667)`, so entry score can flip their order. fB's High confidence (entry=1.0) overcomes fA's weight advantage.
- **fC, fD, fE, fF**: unchanged in relative order.
- **No fact fell out of the new top-5**: `fell:[]`. fF (lowest two_layer=0.144) is rank 6 in both orderings (excluded from top-5 by truncation, not by dropping).

## Verification (complete raw output)

### 1. `cargo test -p northhing-agentic-growth` (must stay 139)

```
running 139 tests
test error::tests::error_display_includes_context ... ok
test negation::tests::case_insensitive_english ... ok
[... 137 more tests, all ok ...]
test topics::score::tests::sanitize_overflow ... ok

test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0.00s
```

139 tests, unchanged. No crate code was added.

### 2. `cargo check -p northhing-core --features product-full` (warning baseline 19)

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.63s
```

19 warnings, unchanged. No new warnings introduced.

### 3. `cargo test -p northhing-core --features product-full memory_db` (was 21)

```
running 28 tests
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::memory_db::tests::unmatched_topic_fact_stays_retrievable ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::confidence_orders_ranking ... ok
test service::agent_memory::memory_db::tests::lowest_combination_not_dropped ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::topic_weight_orders_ranking ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::weight_cap_preserves_resolution ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::topic_dominance_outranks_confidence ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok
test service::agent_memory::memory_db::tests::before_after_ranking_comparison ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 1161 filtered out; finished in 0.26s
```

28 tests (was 21, +7 new). All pass.

### 4. `cargo test -p northhing-core --features product-full auto_memory`

```
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1182 filtered out; finished in 0.11s
```

7 tests, unchanged.

### 5. `cargo test -p northhing-core --features product-full growth_adapter` (was 30)

```
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 1159 filtered out; finished in 0.31s
```

30 tests, unchanged.

### 6. `node scripts/check-core-boundaries.mjs`

```
Core boundary check passed.
```

Exit 0.

### 7. Line counts

```
memory_db.rs:       943 lines  (was 918, +25 net — at the <=25 budget)
memory_db_tests.rs: 799 lines  (was 692, +107 — under 800)
```

## Ambiguities judged

1. **Boost count semantics**: `boost_keyword` inserts with `weight=1.0` on first call, then `(weight+1.0).min(5.0)` on subsequent calls. So N boosts yield `weight = min(N, 5.0)`, not `min(N+1, 5.0)`. Tests were calibrated accordingly (raw 5.0 = 5 boosts, raw 3.0 = 3 boosts, raw 2.0 = 2 boosts).

2. **`entry_score_for_confidence(&fact.confidence)` vs `&confidence_enum`**: The `confidence_enum` local is moved into the `Fact` struct at line 542 (before the scoring block). Using `&fact.confidence` after construction avoids the borrow-after-move without cloning.

3. **§6.1 assertion**: fA (tw=1.0, Low) does NOT rank #1 in the new formula because `1.0/0.8 = 1.25 < TOPIC_DOMINANCE_RATIO`. fB (tw=0.8, High) correctly outranks fA (`two_layer` 0.80 > 0.72). This is the expected behavior of the two-layer design — entry score can flip order when the weight ratio is below the dominance threshold.

## Commit

`fd61f5e` on `feat/growth-core-0804` (base `4f7ba93`), message: `feat(growth): wire two-layer retrieval score into fact ranking (T6b)`. `git status` clean.

DONE
