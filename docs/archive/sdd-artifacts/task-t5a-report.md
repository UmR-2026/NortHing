# Task T5a — Move distiller pure logic into the growth crate

## Status

DONE

## Commit

- Branch: `feat/growth-core-0804`
- Commit: `71df0dd` — `refactor(growth): move distiller prompt/parse pure logic into growth crate`
- Base: `2e986ce`
- Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`

## Files changed (5 files, +629 / -345)

| File | Change |
|---|---|
| `src/agentic/src/distill/parse.rs` | Full implementation: pure JSON parser for distilled facts |
| `src/agentic/src/distill/prompt.rs` | Full implementation: `build_distill_prompt` + `MAX_ASSISTANT_TEXT_CHARS` |
| `src/agentic/src/distill/mod.rs` | Doc updated ("Filled by task G1-T5." removed) |
| `src/crates/assembly/core/src/service/agent_memory/distiller.rs` | Core is now a thin IO adapter over the crate |
| `src/agentic/AGENTS.md` | §4 parameter registry filled (was the "subsequent tasks" stub) |

## What moved into the crate

- `parse_distilled_facts(json: &str) -> DistillParseOutcome` — outcome carries
  `facts: Vec<DistilledFact>`, `keywords: Vec<String>`, `was_empty_array: bool`,
  `parse_error: Option<String>`.
- Types `DistilledFact`, `DistilledFactType`, `DistilledConfidence`,
  `DistilledScope` (neutral, no id/timestamp/provenance).
- Private `strip_json_fence`, `RawDistilledFact`, and the R-4 tolerant
  `deserialize_keywords` (never returns Err; malformed `keywords` degrades to
  "no keywords" while facts still parse).
- Constants `MAX_DISTILL_FACTS=3`, `MAX_FACT_TEXT_CHARS=300` (parse.rs),
  `MAX_ASSISTANT_TEXT_CHARS=500` (prompt.rs, pub).
- `build_distill_prompt(user_input, last_assistant_text: Option<&str>) -> DistillPrompt {system, user}`.
  System text byte-copied from the old `build_distillation_messages`.

## What stays in core (host IO, out of scope)

- `distill_facts_with_llm` flow unchanged (calls `resolve_distiller_client`).
- Adapter `parse_distilled_facts(json, session_id, turn_id)` — hydrates crate
  `DistilledFact` → host `Fact`: fresh `uuid`, `created_at`, `provenance`,
  `schema_version: 1`; logs `warn!` when `outcome.parse_error.is_some()` (D15
  no-op / fallback semantics identical: parse failure → empty + `was_empty=false`;
  explicit `[]` → empty + `was_empty=true`).
- `MIN_USER_INPUT_CHARS=20`, `DISTILL_TIMEOUT_SECS=15`.

## Proof: prompt output byte-identical

- Before snapshot captured pre-move via a temporary test on the OLD
  `build_distillation_messages` path → `distill-prompt-before.txt` (6591 bytes).
- After snapshot emitted via `build_distill_prompt` with the same 2 cases and
  same output format → `distill-prompt-after.txt` (6591 bytes).
- Explicit equality check (not eyeballing):
  `SequenceEqual(before_bytes, after_bytes)` → **byte-identical: True**.
  (Snapshots live in `C:\Users\UmR\AppData\Local\Temp\opencode\`; both temp tests
  removed before commit.)
- Cases: "short-no-asst" (no assistant reply) and "long-with-asst"
  (600-char assistant reply → 500-char truncation).

## Test results

| Suite | Before | After |
|---|---|---|
| `cargo test -p northhing-agentic-growth` | 139 passed | **154 passed** (13 parse + 2 prompt new; 139 baseline intact) |
| core `distill` filter | 37 passed | **31 passed** (13 parse tests moved to crate; 24 remaining + 7 new adapter tests) |
| core `auto_memory` filter | 8 passed | **8 passed** |
| core `turn_persist` filter | 12 passed | **12 passed** |
| core warnings | 19 | **19** (unchanged) |
| `node scripts/check-core-boundaries.mjs` | pass | **pass** |

New adapter tests (distiller.rs): `adapter_parse_hydrates_fact_fields`,
`adapter_parse_assigns_unique_ids_per_fact`, `adapter_parse_maps_all_enum_variants`,
`adapter_parse_maps_reference_variant`, `adapter_parse_failure_returns_empty_not_empty_array`,
`adapter_parse_empty_array_flags_noop`, `adapter_parse_propagates_keywords`.

## R-4 compliance

`deserialize_keywords` moved intact into `parse.rs` — still tolerant of string /
number / object / array-with-non-strings, never returns `Err`. Verified by crate
tests `parse_keywords_wrong_type_*` (all pass) and adapter test
`adapter_parse_propagates_keywords`.

## Constraints honored

- One commit, message prefixed `refactor(growth): `, on `feat/growth-core-0804`.
- `cargo fmt` not run; `dream.rs` untouched (T5b scope).
- All crate files LF, no BOM; core file kept LF, no BOM (encoding verified).
- Report written to MAIN repo `.superpowers/` (worktree `.superpowers/` untouched, not committed).

## Concerns

- Minor (deferred to final triage): crate parse tests overlap conceptually with
  the 7 core adapter tests (boundary re-verification by design).
