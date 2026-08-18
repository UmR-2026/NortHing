# Task T5b report — Move dream verdict parsing into the growth crate

- Commit: `8b64aa8` (`refactor(growth): move dream verdict parsing into growth crate`)
- Branch: `feat/growth-core-0804` (parent `71df0dd`)
- Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804` (all edits landed here; `git rev-parse --show-toplevel` verified before commit)

## Scope summary

1. `parse_dream_verdicts` (dream.rs:238-265) moved to the crate as a generic, allow-list-parameterized
   `parse_verdicts` at `src/agentic/src/review/verdict.rs`.
2. `strip_json_fence` consolidated into one shared module `src/agentic/src/llm_output.rs`; both
   `distill/parse.rs` and `review/verdict.rs` call it, neither keeps a private copy (verified:
   exactly one `pub fn strip_json_fence` definition in the crate).
3. The 6 verdict-parsing tests moved to the crate (same names; see the one documented deviation
   in §5 below about fixture action vocabulary).
4. Core side adapted: `dream.rs` calls `parse_verdicts(&text, candidates.len(), &["keep", "supersede"])`
   and applies the `Vec<Verdict>` through the extracted private helper `apply_verdicts`
   (behavior-identical inline loop, same `warn!` sites, same counters).

### Decisions taken (documented, not silent)

- **§5 "same input JSON" vs §6/§7 "zero `supersede` (and zero `keep`) literals in the crate" conflict.**
  The two are irreconcilable: three of the six moved tests' fixtures contain `"keep"` and one contains
  `"supersede"` as the action value, and §7's `rg -n "supersede" src/agentic` is a hard, grepable
  "must return nothing". I resolved in favor of the constraint: crate fixtures use a **neutral
  vocabulary** (`"accept"` / `"reject"`) in place of `"keep"` / `"supersede"`. Everything else is
  preserved verbatim: test names, JSON structure, indexes, reasons, fence wrapping, ordering, and
  every §4 behavior being asserted. The allow-list argument at the crate call site in tests likewise
  uses the neutral pair. This keeps the `review` path (the load-bearing rule in `src/agentic/AGENTS.md`
  §3) carrying zero dream-vocabulary literals, which is the stated *reason* for the whole
  parameterization (§2). Fixture mapping for the six moved tests:
  - `parse_valid_json_array_maps_fields`: `"keep"` → `"accept"` (assertions `.0/.1/.2` → `.index/.action/.reason` per the resolved ambiguity)
  - `parse_fence_tolerant`: `"supersede"` → `"reject"` (assertion `[0].1` → `[0].action`)
  - `parse_bad_json_returns_empty`: unchanged
  - `parse_index_out_of_bounds_skipped`: `"keep"` → `"accept"`
  - `parse_unknown_action_skipped`: unchanged (`"maybe"` is not in any allow-list)
  - `parse_reason_truncated`: `"keep"` → `"accept"` (assertion `[0].2` → `[0].reason`)
- **§5 "Keep at least one core-side test proving `dream.rs` still applies verdicts end to end."**
  No such test existed at base (the 6 tests only exercised `parse_dream_verdicts`; the D9 test only
  exercises the payload). To make the apply path testable without duplicating logic, I extracted the
  inline apply loop (`dream.rs:143-197`) into a private `fn apply_verdicts` whose body is byte-for-byte
  the original loop (same `warn!` sites, same `supersede_fact`/`record_fact_review` calls, same
  skip logic) returning the `(scanned, superseded, kept, skipped)` counters; `run_dream_sweep` keeps
  step (h) (set judge state + `info!` log) and now logs the returned counters. New core test
  `apply_verdicts_applies_keep_and_supersede_end_to_end` proves: crate parser → `apply_verdicts` on an
  isolated MemoryDb → fact status change (superseded fact leaves `get_facts(None)` active set) and both
  `fact_reviews` rows recorded with the right actions. This is the reason the `dream` filter count is
  now 2, not 1 (see §7.2).
- **§7 `rg -n "supersede" src/agentic` "must return nothing":** at base the crate already matches 5
  lines (`AGENTS.md:23`, `ports.rs:51/58/170` — the `Superseded` status variant and the
  `supersede_fact` port method, and `topics/competition.rs:404` comment). It is therefore not
  literally satisfiable; the meaningful, enforceable version is "this task adds zero new matches".
  Verified: the post-change grep is byte-identical to the base grep (5 matches, same lines). The
  `review/` and `llm_output/` paths carry zero matches. `ports.rs` keeps `supersede_fact` because it
  is the *storage port* the host implements — the crate declares the port trait, it does not decide
  supersede semantics in any path (§3 of the crate AGENTS.md bans supersede semantics in the
  `garden`/`review` paths, which this task keeps clean).
- **Out-of-scope rulings respected:** candidate selection not moved (stays for T12); no decision
  semantics changed (dream still supersedes; T7a already deferred the core-side `supersede` boundary
  rule to T12); `dream_d9_tests.rs` and the `#[cfg(test)] #[path]` child-module wiring untouched;
  `distill/parse.rs` touched only to swap its private fence stripper for the shared one.
- **Named `Verdict` struct** used as intended; test assertions use field names.

## §4 Behavior checklist (row by row, with evidence)

1. **Malformed JSON → empty vector, no panic, no log.**
   Evidence: `parse_verdicts` in `review/verdict.rs` does `match serde_json::from_str(&cleaned) { Ok(v) => v, Err(_) => return Vec::new() }`. The crate has no `tracing`/logging calls in `verdict.rs` or `llm_output.rs` (checked: no `log`/`tracing` import in either). The dream error-swallowing behavior is deliberately preserved — I did **not** add the distiller-style `parse_error` channel. Test: `parse_bad_json_returns_empty` passes ("not json at all" → empty).

2. **`index >= item_count` → item skipped.**
   Evidence: `if idx >= item_count { continue; }` in `parse_verdicts`. Test: `parse_index_out_of_bounds_skipped` (index 5, item_count 2 → empty) passes.

3. **Missing or unknown `action` → item skipped, never defaulted.**
   Evidence: `let action = match item.action.as_deref() { Some(a) if allowed_actions.contains(&a) => item.action.unwrap_or_default(), _ => continue };` — same shape as the original `Some("keep") | Some("supersede") => ...` / `_ => continue`. No lowercasing, no trimming, verbatim comparison. Tests: `parse_unknown_action_skipped` ("maybe" → empty) and the new `parse_disallowed_action_skipped_allowed_kept` (disallowed "delete" skipped while "accept" in the same payload kept) pass.

4. **`reason` longer than 200 chars (not bytes) → truncated to 200; shorter untouched; absent stays `None`.**
   Evidence: `reason = item.reason.map(|r| if r.chars().count() > MAX_REASON_CHARS { r.chars().take(MAX_REASON_CHARS).collect() } else { r })` — char-based, identical to the original. Test: `parse_reason_truncated` (250 "a" chars → `chars().count() == MAX_REASON_CHARS`) passes. Shorter reason preserved: `parse_valid_json_array_maps_fields` asserts `reason == Some("still valid")`. Absent → `None` by the same `.map(...)` (no defaulting path).

5. **Fence tolerance identical for ```` ``` ````, ```` ```json ````, unfenced.**
   Evidence: single shared `strip_json_fence` in `llm_output.rs` — the body was moved verbatim from the two identical private copies (trim → strip leading ```` ``` ```` → optional `json` → `trim_start` → strip trailing ```` ``` ```` → `trim`). Test: `parse_fence_tolerant` (```` ```json ````) passes; new `llm_output.rs` tests cover plain fence and unfenced input. The `distill` suite (31 tests, unchanged) also still exercises the shared stripper via `parse_json_fence_wrap`.

6. **Output order follows input order, with skipped items simply absent.**
   Evidence: `parse_verdicts` iterates `raw` in order and pushes only kept items. Test: `parse_allow_list_changes_results` asserts the kept item keeps its input index/order; `parse_disallowed_action_skipped_allowed_kept` asserts the kept item is `index 0` of the payload.

## §5 Tests

- 6 tests moved to the crate with the same names and same assertion semantics (field-based instead of `.0/.1/.2`): `parse_valid_json_array_maps_fields`, `parse_fence_tolerant`, `parse_bad_json_returns_empty`, `parse_index_out_of_bounds_skipped`, `parse_unknown_action_skipped`, `parse_reason_truncated`.
- New in the crate:
  - `parse_disallowed_action_skipped_allowed_kept` — an action not in `allowed_actions` is skipped while an allowed one in the same payload is kept.
  - `parse_allow_list_changes_results` — the same payload with two different allow-lists yields different results (proves the allow-list is consulted).
  - `strip_json_fence_unfenced_passes_through` / `strip_json_fence_plain_fence_removed` / `strip_json_fence_flagged_fence_removed` — fence tests in the new home (no direct `strip_json_fence` test existed before).
- Name collisions (expected per brief): `distill::parse::tests::parse_valid_json_array_maps_fields` and `parse_bad_json_returns_empty` now coexist with the same-named tests in `review::verdict::tests` — different modules, both compile and run (crate suite shows both modules' versions among the 165).
- Core side end-to-end test kept/added: `apply_verdicts_applies_keep_and_supersede_end_to_end` (see "Decisions" above).
- `dream` filter: **7 → 2** (6 parse tests moved to the crate; 1 new end-to-end apply test added; D9 negative test retained).

## §6 Constraints

- `src/agentic/Cargo.toml` unchanged — the crate gains no dependency. (Verified via `git diff HEAD -- src/agentic/Cargo.toml` = empty; the file is not in the commit.)
- **Zero `supersede`/`keep` literals added by this task.** Grep output below is byte-identical to the base-commit grep (5 `supersede` matches, all pre-existing in `AGENTS.md` / `ports.rs` / `topics/competition.rs`; 9 English-word `keep` matches in `.rs` files, all pre-existing in `scheduler.rs` / `topics/competition.rs` / `topics/extract.rs`). `review/` and `llm_output/` contain none.
- `MAX_REASON_CHARS` registered in `src/agentic/AGENTS.md` §4 (row below), format matching T5a's rows.
- `cargo fmt` not run. English-only. No emoji. All touched files < 800 lines (line counts below).
- Warn-only semantics preserved: no log site added or removed; the two `warn!` sites in the apply loop moved verbatim into `apply_verdicts`; the `info!` sweep summary stays in `run_dream_sweep` unchanged.

### §6 grep output

`rg -n "supersede" src/agentic` (identical before and after this task; 5 pre-existing matches):

```
src/agentic\AGENTS.md:23:negation only). Any supersede semantics appearing in the `garden` or `review`
src/agentic\src\ports.rs:51:            Self::Superseded => "superseded",
src/agentic\src\ports.rs:58:            "superseded" => Some(Self::Superseded),
src/agentic\src\ports.rs:170:    fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> GrowthResult<()>;
src/agentic\src\topics\competition.rs:404:    // deletion. No retire/supersede/deactivate function exists in this module.
```

`rg -n "keep" src/agentic --glob "*.rs"` (identical before and after; 9 pre-existing English-word matches):

```
src/agentic\src\scheduler.rs:28://! every subsequent turn once paused, because `turns` keeps growing while
src/agentic\src\scheduler.rs:44://!   [`DistillTransition::Resumed`]. A missed probe keeps the brake on;
src/agentic\src\topics\competition.rs:308:    // Invariant 1 (sum conservation): 10 deterministic boosts keep sum == 1.0
src/agentic\src\topics\extract.rs:133:    // is long enough, keep the original (e.g. `c++` -> keep `c++`).
src/agentic\src\topics\extract.rs:276:///    keeping the first occurrence order.
src/agentic\src\topics\extract.rs:302:            // Dedup: keep first-occurrence order
src/agentic\src\topics\extract.rs:336:///    keeping the first occurrence order.
src/agentic\src\topics\extract.rs:351:            // Dedup: keep first-occurrence order (post-normalization).
src/agentic\src\topics\extract.rs:388:    fn pure_cjk_keeps_contiguous_run_as_one_topic() {
```

Delta from this task: **0 new matches for either pattern.** (`rg -n "fn strip_json_fence" src/agentic` → exactly one definition, at `llm_output.rs:12`.)

### AGENTS.md §4 row added

```
| `review::verdict::MAX_REASON_CHARS` | 200 | max chars per verdict reason |
```

(Preamble sentence also extended: "verdict parsing parameters live in `review/verdict.rs`.")

## §7 Verification (complete raw output)

Prefix used for all cargo commands: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 7.1 `cargo test -p northhing-agentic-growth` — **165** (was 154; +11 = 6 moved + 2 new parameterization tests + 3 fence tests)

Full raw stdout+stderr:

```
    Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 165 tests
test distill::parse::tests::parse_empty_array_returns_empty ... ok
test distill::parse::tests::parse_json_fence_wrap ... ok
test distill::parse::tests::parse_four_items_truncates_to_three ... ok
test distill::parse::tests::parse_bad_json_returns_empty ... ok
test distill::parse::tests::parse_keywords_array_with_non_string_elements_facts_intact ... ok
test distill::parse::tests::parse_keywords_wrong_type_object_ignored_facts_intact ... ok
test distill::parse::tests::parse_keywords_wrong_type_ignored_facts_intact ... ok
test distill::parse::tests::parse_keywords_wrong_type_number_ignored_facts_intact ... ok
test distill::parse::tests::parse_keywords_union_dedup_across_items ... ok
test distill::parse::tests::parse_legacy_json_without_keywords_still_works ... ok
test distill::parse::tests::parse_text_over_300_chars_truncated ... ok
test distill::parse::tests::parse_unknown_fact_type_skipped_valid_kept ... ok
test llm_output::tests::strip_json_fence_plain_fence_removed ... ok
test distill::prompt::tests::build_distill_prompt_wraps_user_input ... ok
test distill::prompt::tests::build_distill_prompt_truncates_assistant_text_to_500 ... ok
test error::tests::error_display_includes_context ... ok
test llm_output::tests::strip_json_fence_flagged_fence_removed ... ok
test distill::parse::tests::parse_valid_json_array_maps_fields ... ok
test llm_output::tests::strip_json_fence_unfenced_passes_through ... ok
test negation::tests::case_insensitive_english ... ok
test negation::tests::english_stop_remembering ... ok
test negation::tests::chinese_preference_replaced ... ok
test negation::tests::chinese_stop_remembering ... ok
test negation::tests::english_fact_is_wrong ... ok
test negation::tests::english_preference_replaced ... ok
test negation::tests::chinese_fact_is_wrong ... ok
test negation::tests::no_hit_empty_or_whitespace ... ok
test negation::tests::no_hit_false_friend_ji ... ok
test negation::tests::no_hit_not_great ... ok
test negation::tests::no_hit_vague_negative_chinese ... ok
test negation::tests::parse_duplicates_deduped ... ok
test negation::tests::parse_malformed_returns_empty ... ok
test negation::tests::parse_negative_float_string_dropped ... ok
test negation::tests::parse_out_of_range_dropped ... ok
test negation::tests::parse_simple_valid ... ok
test negation::tests::parse_with_json_fence ... ok
test negation::tests::parse_with_surrounding_prose ... ok
test negation::tests::parse_zero_candidates_always_empty ... ok
test negation::tests::prompt_candidates_numbered_without_fact_id ... ok
test negation::tests::priority_fact_is_wrong_over_preference ... ok
test negation::tests::prompt_contains_user_message_tags_and_original_text ... ok
test negation::tests::prompt_empty_candidates_does_not_panic ... ok
test negation::tests::same_kind_earliest_phrase_wins ... ok
test negation::tests::target_hint_capped_at_60_chars ... ok
test negation::tests::target_hint_extracted ... ok
test negation::tests::target_hint_none_when_nothing_after ... ok
test ports::tests::test_fact_status_round_trip ... ok
test ports::tests::test_fact_type_round_trip ... ok
test ports::tests::test_fake_clock ... ok
test ports::tests::test_object_safety ... ok
test ports::tests::test_reviewer_round_trip ... ok
test review::verdict::tests::parse_allow_list_changes_results ... ok
test review::verdict::tests::parse_bad_json_returns_empty ... ok
test review::verdict::tests::parse_disallowed_action_skipped_allowed_kept ... ok
test review::verdict::tests::parse_fence_tolerant ... ok
test review::verdict::tests::parse_index_out_of_bounds_skipped ... ok
test review::verdict::tests::parse_reason_truncated ... ok
test review::verdict::tests::parse_unknown_action_skipped ... ok
test review::verdict::tests::parse_valid_json_array_maps_fields ... ok
test scheduler::tests::after_garden_sweep_gate_is_closed ... ok
test scheduler::tests::all_wake_phrases_match ... ok
test scheduler::tests::auto_pause_event_fires_only_once ... ok
test scheduler::tests::bare_negators_do_not_trigger_wake ... ok
test scheduler::tests::below_auto_pause_threshold_no_event ... ok
test scheduler::tests::decide_turn_both_closed ... ok
test scheduler::tests::decide_turn_both_gates_open ... ok
test scheduler::tests::decide_turn_distill_open_garden_not_due ... ok
test scheduler::tests::decide_turn_distill_paused_garden_open ... ok
test scheduler::tests::garden_sweep_both_zero_returns_false ... ok
test scheduler::tests::garden_sweep_clock_backwards_returns_false ... ok
test scheduler::tests::garden_sweep_exact_interval_returns_true ... ok
test scheduler::tests::garden_sweep_from_zero_to_interval_returns_true ... ok
test scheduler::tests::garden_sweep_one_ms_below_interval_returns_false ... ok
test scheduler::tests::has_hit_turns_does_not_pause ... ok
test scheduler::tests::hit_turns_increments_only_on_produced_facts ... ok
test scheduler::tests::old_blob_without_paused_at_turns_deserialises ... ok
test scheduler::tests::paused_state_still_increments_turns ... ok
test scheduler::tests::old_growth_state_blob_without_paused_at_turns_round_trips ... ok
test scheduler::tests::probe_hit_resumes_and_resets_window ... ok
test scheduler::tests::probe_miss_stays_paused_and_next_window_arrives ... ok
test scheduler::tests::probe_resume_event_fires_once ... ok
test scheduler::tests::probe_window_first_probe_at_anchor_plus_n ... ok
test scheduler::tests::resume_event_fires_only_on_transition ... ok
test scheduler::tests::saturating_add_at_max_does_not_panic ... ok
test scheduler::tests::should_distill_returns_false_when_paused ... ok
test scheduler::tests::should_distill_returns_true_when_not_paused ... ok
test scheduler::tests::triggers_auto_pause_at_twenty ... ok
test scheduler::tests::wake_phrase_resumes_resets_window_and_distils ... ok
test state::tests::test_bad_json ... ok
test state::tests::test_migration_all_legacy_present ... ok
test state::tests::test_blob_exists_and_valid ... ok
test state::tests::test_migration_dirty_legacy_keys ... ok
test state::tests::test_migration_idempotent ... ok
test state::tests::test_migration_no_legacy_keys ... ok
test state::tests::test_migration_port_error_on_legacy ... ok
test state::tests::test_port_error_load ... ok
test state::tests::test_port_error_save ... ok
test state::tests::test_unknown_schema_version ... ok
test topics::competition::tests::all_zero_weights_split_equally ... ok
test topics::competition::tests::boost_clamp_and_negative_noop ... ok
test topics::competition::tests::boost_inserts_new_topic ... ok
test topics::competition::tests::boost_rise_causes_fall ... ok
test topics::competition::tests::duplicate_topic_boost_and_health ... ok
test topics::competition::tests::empty_group_handling ... ok
test topics::competition::tests::health_healthy_group ... ok
test topics::competition::tests::health_out_of_range ... ok
test topics::competition::tests::health_sum_drift ... ok
test topics::competition::tests::nan_and_negative_treated_as_zero ... ok
test topics::competition::tests::no_member_removed_by_boost ... ok
test topics::competition::tests::revive_already_above_returns_none ... ok
test topics::competition::tests::single_member_group ... ok
test topics::competition::tests::sum_conservation_over_many_boosts ... ok
test topics::competition::tests::suppressed_member_can_revive ... ok
test topics::competition::tests::revive_extreme_group_returns ... ok
test topics::competition::tests::suppression_both_below ... ok
test topics::competition::tests::suppression_boundary_strict_less_than ... ok
test topics::competition::tests::suppression_raw_high_stays_active ... ok
test topics::competition::tests::suppression_share_high_stays_active ... ok
test topics::competition::tests::zero_share_can_rise ... ok
test topics::extract::tests::ascii_case_normalized_to_lowercase ... ok
test topics::extract::tests::ascii_connector_chars_survive_inside_tokens ... ok
test topics::extract::tests::ascii_stopwords_are_filtered ... ok
test topics::extract::tests::at_most_max_topics_returned ... ok
test topics::extract::tests::cjk_stopwords_are_filtered ... ok
test topics::extract::tests::connector_chars_stripped_from_ends ... ok
test topics::extract::tests::duplicate_tokens_are_deduplicated ... ok
test topics::extract::tests::empty_input_yields_empty_result ... ok
test topics::extract::tests::long_cjk_topic_is_truncated_by_char_count ... ok
test topics::extract::tests::mixed_cjk_ascii_contains_both_kinds ... ok
test topics::extract::tests::normalize_candidates_caps_at_max_topics ... ok
test topics::extract::tests::normalize_candidates_dedups_case_variants ... ok
test topics::extract::tests::normalize_candidates_discards_empty_whitespace_control ... ok
test topics::extract::tests::normalize_candidates_discards_short_ascii ... ok
test topics::extract::tests::normalize_candidates_discards_short_cjk ... ok
test topics::extract::tests::normalize_candidates_matches_extract_topics_key_space ... ok
test topics::extract::tests::normalize_candidates_preserves_connectors ... ok
test topics::extract::tests::normalize_candidates_truncates_long_candidate ... ok
test topics::extract::tests::only_punctuation_yields_empty_result ... ok
test topics::extract::tests::only_whitespace_yields_empty_result ... ok
test topics::extract::tests::pure_ascii_filters_stopwords_and_short_tokens ... ok
test topics::extract::tests::pure_cjk_keeps_contiguous_run_as_one_topic ... ok
test topics::extract::tests::pure_digit_tokens_are_filtered ... ok
test topics::extract::tests::same_input_produces_same_output_twice ... ok
test topics::extract::tests::short_words_and_single_chars_yield_empty ... ok
test topics::score::tests::best_weight_all_nan ... ok
test topics::score::tests::best_weight_empty ... ok
test topics::score::tests::best_weight_ignores_nan ... ok
test topics::score::tests::dominance_property_loop ... ok
test topics::score::tests::dominance_tw055_es0_loses_to_tw05_es1 ... ok
test topics::score::tests::dominance_tw09_es0_beats_tw05_es1 ... ok
test topics::score::tests::rank_below_floor_dropped ... ok
test topics::score::tests::rank_descending_score ... ok
test topics::score::tests::rank_empty ... ok
test topics::score::tests::rank_nan_candidate_no_panic ... ok
test topics::score::tests::rank_tie_different_tw ... ok
test topics::score::tests::rank_tie_same_score_different_id ... ok
test topics::score::tests::retrieval_floor_only ... ok
test topics::score::tests::retrieval_upper_bound ... ok
test topics::score::tests::retrieval_zero_tw ... ok
test topics::score::tests::sanitize_infinity ... ok
test topics::score::tests::sanitize_mid ... ok
test topics::score::tests::sanitize_nan ... ok
test topics::score::tests::sanitize_neg_infinity ... ok
test topics::score::tests::sanitize_negative ... ok
test topics::score::tests::sanitize_overflow ... ok

test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 7.2 `cargo test -p northhing-core --features product-full dream` — **2** (was 7; 6 parse tests moved to the crate, 1 new end-to-end apply test added, D9 negative test retained)

```
running 2 tests
test service::agent_memory::dream::d9_tests::dream_payload_never_contains_self_cognition_sentinel ... ok
test service::agent_memory::dream::tests::apply_verdicts_applies_keep_and_supersede_end_to_end ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1214 filtered out; finished in 0.09s
```

### 7.3 `cargo test -p northhing-core --features product-full distill` — **31**, unchanged

```
running 31 tests
test agentic::coordination::dialog_turn::turn_persist_facts::tests::none_signals_denies_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist_facts::tests::standard_empty_creator_without_parent_allows_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist_facts::tests::standard_no_parent_no_creator_allows_distillation ... ok
test agentic::coordination::dialog_turn::turn_persist_facts::tests::standard_non_session_creator_without_parent_allows_distillation ... ok
test agentic::episodes::distill::tests::error_message_truncation ... ok
test agentic::episodes::distill::tests::distill_with_no_tools ... ok
test service::agent_memory::distiller::tests::adapter_parse_empty_array_flags_noop ... ok
test service::agent_memory::distiller::tests::adapter_parse_assigns_unique_ids_per_fact ... ok
test service::agent_memory::distiller::tests::adapter_parse_failure_returns_empty_not_empty_array ... ok
test agentic::episodes::distill::tests::distill_with_tools ... ok
test service::agent_memory::distiller::tests::adapter_parse_hydrates_fact_fields ... ok
test agentic::episodes::distill::tests::distill_with_failure_no_repair ... ok
test agentic::episodes::distill::tests::distill_with_repair_across_rounds ... ok
test agentic::episodes::distill::tests::repair_content_from_input_when_no_result_for_assistant ... ok
test service::agent_memory::distiller::tests::adapter_parse_maps_all_enum_variants ... ok
test service::agent_memory::distiller::tests::adapter_parse_maps_reference_variant ... ok
test service::agent_memory::distiller::tests::adapter_parse_propagates_keywords ... ok
test service::agent_memory::facts::tests::distill_facts_no_keyword_returns_empty ... ok
test service::agent_memory::facts::tests::distill_facts_truncates_long_sentence ... ok
test service::agent_memory::facts::tests::distill_facts_with_cjk_period ... ok
test service::agent_memory::facts::tests::distill_facts_multiple_sentences_with_keyword ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_chinese ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_always ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_remember ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_true_on_unpaused_db ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_false_when_paused ... ok
test agentic::growth_adapter::tests::finish_distill_turn_continues_counting_while_paused ... ok
test agentic::growth_adapter::tests::finish_distill_turn_triggers_pause_at_threshold_and_persists ... ok
test agentic::growth_adapter::tests::finish_distill_turn_uses_migrated_legacy_counts ... ok
test agentic::growth_adapter::tests::finish_distill_turn_does_not_rewrite_legacy_keys ... ok
test agentic::growth_adapter::tests::finish_distill_turn_with_facts_increments_hits_and_no_pause ... ok

test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 1185 filtered out; finished in 0.11s
```

### 7.4 `cargo check -p northhing-core --features product-full` — warning baseline **19**, unchanged

The compiler summary line (authoritative): `warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)` — identical before and after.

Full warning list after (all pre-existing; first one is `hidden_glob_reexports` at `agentic/session/mod.rs:13`, present at base too — at base it was captured by PowerShell as a stderr error-record line `cargo : warning: private item shadows public glob re-export` so it did not match a `^warning: ` text filter, but it is present in the base log and counted in the base "19"):

```
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:300:9
warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:66:13
warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:37:13
warning: unused variable: `event_system`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:305:9
warning: unused variable: `tool_use_id`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_signal.rs:72:9
warning: unused variable: `port`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:41:13
warning: unused variable: `workspace_turn_status`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:386:17
warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:70:13
warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:290:36
warning: unused variable: `last_mentioned_at`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:345:80
warning: unused variable: `at_ms`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:805:85
warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
```

Exit code: 0.

### 7.5 `node scripts/check-core-boundaries.mjs` — exit **0**

```
Core boundary check passed.
```

(Exit code 0 confirmed.)

### 7.6 Line counts of every file touched or created

```
   48  src/agentic/src/llm_output.rs                     (created)
  161  src/agentic/src/review/verdict.rs
   25  src/agentic/src/lib.rs
    7  src/agentic/src/review/mod.rs
  374  src/agentic/src/distill/parse.rs
   46  src/agentic/AGENTS.md
  335  src/crates/assembly/core/src/service/agent_memory/dream.rs
```

All < 800 lines. `src/agentic/Cargo.toml` untouched (not in the commit).

### 7.7 `rg -n "supersede" src/agentic` — see §6 above; byte-identical to base (5 pre-existing matches), zero new

## Anything ambiguous

- The §5 "same input JSON" vs §6/§7 zero-literal conflict and the §7 grep "must return nothing"
  phrasing (impossible literally at base) — resolution documented at the top of this report and in §6.
- The apply-loop extraction into `apply_verdicts` was the minimal change needed to satisfy §5's
  required core-side end-to-end test; the loop body is unchanged and stays in `dream.rs`.

DONE
