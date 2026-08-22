# Review request — T5b: dream verdict parsing moved into the growth crate

## What to review

Diff: `E:\agent-project\northing\.superpowers\sdd\task-t5b-diff.md` (base `71df0dd` → HEAD `8b64aa8`, 7 files)
Implementer's report: `E:\agent-project\northing\.superpowers\sdd\task-t5b-report.md`
Task spec the implementer was given: `E:\agent-project\northing\.superpowers\sdd\task-t5b-brief.md`
Repo for reading full files: `E:\agent-project\northing\.worktrees\growth-core-0804`

This is a **behavior-preserving refactor**. A pure function that parsed an LLM's JSON verdict
array was moved from `service/agent_memory/dream.rs` into the growth crate at
`src/agentic/src/review/verdict.rs`, made generic over the allowed action names, and a duplicated
`strip_json_fence` helper was consolidated into a new shared module `src/agentic/src/llm_output.rs`.

You must deliver **two independent verdicts**: SPEC compliance and CODE QUALITY. Both are
required; neither substitutes for the other.

## Already independently verified by the orchestrator — do not re-run

- crate tests 154 → **165**; core `dream` 7 → **2**; `distill` **31**, `auto_memory` **8**,
  `memory_db` **28**, `self_cognition` **19**, `turn_persist` **12**, `prompt_injection` **4**,
  `growth_adapter` **30**, `system_prompt` **21** — all unchanged from base
- `cargo check -p northhing-core --features product-full` — **19** warnings, matching baseline
- `node scripts/check-core-boundaries.mjs` — exit 0
- `src/agentic/Cargo.toml` unchanged; line counts all well under 800
- `git diff -w` on `dream.rs` shows the verdict-application loop **body** is byte-identical to
  the pre-move version apart from the destructuring pattern on the `for` line

Spend your effort on semantics, not on re-running the suite. Treat the test run as settled.

## Priority 1 — The fixture-vocabulary substitution (the one real judgment call)

The brief contained a **contradiction that was the orchestrator's fault**: §5 demanded the 6
moved tests keep "the same input JSON", while §6/§7 demanded "zero `keep`/`supersede` literals in
the crate". Three of the moved fixtures contain exactly those action strings, so both cannot
hold. The implementer resolved it by rewriting the crate-side fixtures to use neutral action
names (`accept`/`reject`) and reported the deviation openly.

Judge this honestly and independently:

1. Do the rewritten crate tests still exercise **the same code paths and the same behaviors** as
   the originals — index mapping, out-of-bounds skip, unknown-action skip, reason truncation,
   fence tolerance, malformed-JSON-to-empty, output ordering? Compare fixture by fixture against
   the deleted originals visible in the diff.
2. Is any behavior now **untested** that was tested before? Specifically: is the real dream
   vocabulary (`keep` / `supersede`) still covered anywhere? Look at the new core-side test
   `apply_verdicts_applies_keep_and_supersede_end_to_end` in `dream.rs` and say whether it
   genuinely closes that gap or merely appears to.
3. Was the substitution actually necessary, or could both constraints have been satisfied (for
   example by keeping the original fixtures and scoping the "zero literals" rule to non-test
   code)? State which you would have chosen and why — this feeds a real decision about whether
   the plan's own constraint needs amending.

## Priority 2 — Parameterization correctness

The whole point of this task is that the crate must not know the dream action vocabulary
(`src/agentic/AGENTS.md` §3 forbids supersede semantics in the `garden`/`review` paths).

- Does `parse_verdicts(json, item_count, allowed_actions)` treat an action **not** in the
  allow-list identically to how the old hard-coded `_ => continue` treated an unknown action?
- Is the action compared **verbatim**? The old code did no trimming, lowercasing, or
  normalization. Confirm none was silently introduced.
- Is a **missing** `action` field still skipped rather than defaulted? Check that no `Default`
  impl or `unwrap_or_default()` can turn a missing action into an empty-string action that then
  fails the allow-list check for a different reason than before (same outcome, different path —
  say so if that is what happens).
- Does the allow-list get genuinely consulted, or could a passing test pass with it ignored?

## Priority 3 — The `strip_json_fence` consolidation

Two byte-identical private copies existed (`dream.rs:267-280` and `distill/parse.rs:158-171`).
Now there should be exactly one, in `llm_output.rs`.

- Is the shared implementation behaviorally identical to both deleted copies? Walk the branches:
  bare ```` ``` ````, ```` ```json ````, unfenced, whitespace-only, a string that both starts and
  ends with a fence, and a string containing a fence in the middle.
- Did consolidating it change anything for the **distiller** path? `distill` tests are unchanged
  at 31, but confirm by reading that `distill/parse.rs` now calls the shared function with the
  same argument and uses the result the same way.
- Is `llm_output.rs` the right home, and is `lib.rs` module registration consistent with the
  crate's existing style?

## Priority 4 — The out-of-scope extraction

The implementer extracted the inline verdict-application loop in `dream.rs` into a private
`fn apply_verdicts(...) -> (usize, usize, usize, usize)`. This was **not** requested by the
brief; the implementer says it was needed to make the brief-mandated core-side end-to-end test
possible, since the logic was previously buried in an `async fn` that opens a database.

- Is the extraction genuinely behavior-preserving? Check in particular: the four counters are
  incremented on exactly the same conditions; every `warn!` site survives with the same message
  and trigger; `set_judge_state` and the summary `info!` still run **after** application and in
  the same order; the `skipped` branch still covers the same cases.
- Is the justification sound, or could the test have been written without the extraction?
- Is the doc comment on `apply_verdicts` accurate, or does it overclaim?

## Priority 5 — Crate hygiene (same bar as T5a)

- No new dependency; no `uuid`, no clock, no IO, no logging inside the crate. The established
  convention (`scheduler.rs:27`) is that pure logic does not log and the host reports.
  ⚠️ Note the deliberate asymmetry with T5a: the distiller parser returns a `parse_error` for the
  host to log, but dream's parser **silently** returns an empty vector on malformed JSON, because
  that is what the old code did (`dream.rs:242`, `Err(_) => return Vec::new()`). The brief
  explicitly forbade "improving" dream to match the distiller. Confirm no such improvement crept
  in, and confirm the crate does not log instead.
- Is `MAX_REASON_CHARS` registered in `src/agentic/AGENTS.md` §4 in the same format T5a used?
- Truncation must be by **chars**, not bytes. Verify, and note whether any test uses multi-byte
  input to prove it (if none does, that is a finding worth raising).
- Is `Verdict` a sensible public API for a library crate — field visibility, derives, naming?
  It is now public surface that T9 and T12 are expected to reuse.
- `review/mod.rs` and `review/verdict.rs` doc comments were reworded because the parser is no
  longer judge-mom-specific. Are they factual, and do they avoid claiming that T9/T12 work exists?

## Ground rules

- Everything in the crate and in core must be English-only, no emoji. `cargo fmt` is banned in
  this repo; do not report formatting-only nits as if they were violations.
- Classify every finding as **Critical** / **Important** / **Minor**, each with file and line.
  Only Critical and Important trigger a fix round, so do not inflate severity to get attention —
  and do not deflate a real correctness problem to Minor.
- If a claim cannot be settled from the diff alone, read the full file in the worktree. If it
  still cannot be settled, say so explicitly under a heading "Cannot verify from diff" rather
  than guessing. The orchestrator resolves those items personally.
- Verify assertions about regexes, word boundaries, and string matching by actually reading the
  code rather than reasoning about what a pattern "should" match. A previous review round stated
  a confidently wrong claim about `\b` and underscores.

## Output

Write your review to `E:\agent-project\northing\.superpowers\sdd\task-t5b-review.md`.

Structure it as: the two verdicts up front (SPEC PASS/FAIL, QUALITY PASS/FAIL), then the
Priority 1 judgment in full, then findings by severity, then a "Cannot verify from diff" section
if you need one. Be concise but concrete — cite file and line for every claim.
