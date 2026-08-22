# Review — Task T5a (move distiller pure logic into the growth crate)

You are reviewing ONE commit on branch `feat/growth-core-0804`.

- Worktree (read source from here): `E:\agent-project\northing\.worktrees\growth-core-0804`
- BASE: `2e986ce`  HEAD: `71df0dd`
- Diff: `E:\agent-project\northing\.superpowers\sdd\task-t5a-diff.md` (1100 lines)
- Spec (the ONLY source of requirements): `E:\agent-project\northing\.superpowers\sdd\task-t5a-brief.md`
- Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t5a-report.md`
- Crate layer rules: `src/agentic/AGENTS.md`

Read the brief in full before the diff. Files changed: `src/agentic/AGENTS.md`,
`src/agentic/src/distill/{mod,parse,prompt}.rs`, and
`src/crates/assembly/core/src/service/agent_memory/distiller.rs`.

## Two verdicts required (both must be explicit)

1. **Spec compliance** — does the commit implement brief §3 as specified, with §4 equivalence held?
2. **Code quality** — is it correct, honest, and maintainable?

## What this task is

A **behavior-preserving refactor**: the distiller's prompt construction and JSON response
parsing move from the core assembly layer into the pure `northhing-agentic-growth` crate;
core keeps the IO (LLM call, config/client resolution) and becomes a thin adapter that
re-hydrates neutral parsed data into `Fact` values. **No decision semantics may change.**

The move could not be naive: the crate has no `uuid` dependency, cannot see core's `Fact`
types (layer rule), must stay deterministic, and by convention does not log. Brief §2
documents these obstacles and §3 fixes the design (neutral `DistilledFact` types +
`DistillParseOutcome` carrying `parse_error` instead of logging).

## Constraints copied verbatim from the plan / crate AGENTS.md

- The crate may depend only on the contracts layer; it must not depend on assembly, services,
  adapters, or interfaces. `rusqlite` is prohibited. (`src/agentic/AGENTS.md` §1)
- No magic numbers scattered across the crate; all thresholds concentrated in per-module
  constants **and registered in AGENTS.md §4**. (§4)
- Production `.rs` files stay under 800 lines.
- `cargo fmt` is forbidden. English-only, no emoji.
- Core tests run with `--features product-full`. Core warning baseline is **19**.
- `northhing-core`'s `lib.rs:3-4` has `#![allow(dead_code)]` / `#![allow(unused_imports)]`, so
  an unchanged warning count does NOT prove absence of dead code or leftover imports — judge
  that by reading. (This exact trap produced Important finding I-1 in task S-1.)

## Highest-value things to check (in priority order)

1. **The §4 equivalence checklist, row by row.** Ten behaviors must be provably unchanged.
   Verify each against the new code, and against the pre-move code
   (`git show 2e986ce:src/crates/assembly/core/src/service/agent_memory/distiller.rs`):
   the 3-fact cap checked *before* processing each item; `text` trim → skip-if-empty →
   truncate to 300 **chars** not bytes; unknown/missing `type`/`confidence`/`scope` → **skip
   the item**, never default; accepted values `user|feedback|project|reference`,
   `high|med|low`, `workspace|global`; keyword union trimmed, empties dropped,
   **order-preserving dedup with first occurrence winning**; `was_empty_array` true ONLY for a
   valid empty array and **false on parse failure**; the tolerant keywords deserializer;
   `strip_json_fence`; prompt text byte-identical.
2. **Did any "skip" silently become a "default"?** The old code used `_ => continue` for three
   enum fields. If the new parser substitutes a default variant anywhere, that is **Critical**:
   it would fabricate fact metadata the LLM never asserted.
3. **Enum mapping correctness and exhaustiveness.** Core maps three crate enums onto
   `FactType` / `FactConfidence` / `FactScope`. Check every arm maps to the *corresponding*
   variant (a transposed pair — e.g. `High` → `Low` — would pass all existing tests if the
   adapter tests are weak). Brief §3.4 requires **exhaustive `match` with no catch-all arm**;
   confirm there is none.
4. **R-4's Important fix must survive.** `deserialize_keywords` must **never return `Err`**:
   a string, number, bool, object, or an array containing non-string elements must all degrade
   to "no keywords" while the surrounding facts still parse. Regressing this drops every fact
   of the turn. Read the implementation and judge whether any input can make it `Err`.
5. **D15 observability preserved.** The distinction between "LLM explicitly returned `[]`"
   (`debug!`, healthy no-op) and "parse failed" (`warn!`) must still be visible at the same log
   levels. The `warn!` moved from the parser into the adapter via `parse_error` — check the
   adapter actually logs it and that the message still conveys the same thing.
6. **The 13 moved tests.** Brief §5 requires each to survive in the crate with the **same name
   and same input JSON**; assertion *shape* may change (`DistilledFact` vs `Fact`) but not
   *what* is asserted. Check none was dropped, renamed, or weakened (e.g. an assertion on a
   count silently replaced by an `is_empty()` check).
7. **Crate purity.** No `uuid`, no `SystemTime`/clock access, no core types, no new dependency
   in `src/agentic/Cargo.toml`. Doc comments mentioning these words are fine; code is not.
8. **Dead code left in core.** After the move, are there now-unused imports, constants, or
   private helpers still sitting in `distiller.rs`? Remember the warning count cannot tell you.
9. **AGENTS.md §4** should now register the three moved parameters (3 / 300 / 500) with
   meanings, replacing the "will be filled by subsequent tasks" stub.

## Already independently verified by the orchestrator — do NOT re-run these

- Test counts: crate **154** (was 139), core `distill` **31**, `auto_memory` 8,
  `turn_persist` 12, `dream` 7, `memory_db` 28. Core warnings **19**. Boundary checker exit 0.
- Commit touches exactly the 5 files listed above (629 insertions / 345 deletions);
  `src/agentic/Cargo.toml` is **unchanged**; worktree clean; main checkout unpolluted; no
  temporary snapshot/proof scaffolding left behind.
- Line counts: `parse.rs` 389, `prompt.rs` 128, `distiller.rs` 656 → 416.
- `uuid` / `SystemTime` appear in the crate only inside a doc comment describing the host's job.
- No `_ =>` catch-all remains anywhere in `distiller.rs`.
- **Prompt equivalence independently confirmed**: the system raw string is byte-identical
  (2886 chars, case-sensitive equality) between `2e986ce`'s `distiller.rs` and `71df0dd`'s
  `prompt.rs`; the `user_content` construction (`<user_message>` wrap, 500-char `chars()`
  truncation, `\n\n<assistant_reply>` append) is character-for-character the same; and the
  adapter assembles `Message::system(prompt.system)` / `Message::user(prompt.user)` in the
  same order.

Review by reading. Do not re-run the suite; the report plus the above is your evidence base.
Spend your effort on items 1-6, which the orchestrator's checks do NOT cover.

## Output format

For each finding: **Critical / Important / Minor**, the `file:line`, what is wrong, why it
matters, and the smallest correct fix. If you cannot determine something from the diff alone,
put it under a separate heading **"Cannot verify from diff"** rather than guessing — and do not
assert that a file, symbol, or line does not exist unless you actually opened the path and
confirmed.

⚠️ Claims about specific values, enum arms, regex/word boundaries, or character-vs-byte
behavior must be checked against the actual source, not inferred. In two recent reviews,
confident detail-level reasoning turned out to be wrong (a false regex word-boundary claim and
a false "file does not exist" claim). Verify, then assert.

End with the two verdicts, each as `PASS` or `FAIL`, on their own lines:
`SPEC: PASS|FAIL`
`QUALITY: PASS|FAIL`
