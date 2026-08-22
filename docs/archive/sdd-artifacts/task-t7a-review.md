# Review — Task T7a (boundary rules for the permission matrix)

Branch: `feat/growth-core-0804`
BASE: `9f261cd`  HEAD: `9a9fb8a`
Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
Reviewed by reading source files only (no tests re-run; orchestrator pre-verified counts, warnings, and firing spot-checks).

## Verdict summary

- **SPEC: PASS** — the commit does exactly what brief §3 requires: three new rule groups in the right list for the right semantics, the D9 test relocation is a pure move with both assertions preserved, the `allowPaths` exception is minimal and honest, and no production behavior changed.
- **QUALITY: PASS** — symbols are real (verified against source), regexes use word boundaries correctly, the recorded exception for `init_self_cognition_store` accurately describes reality, messages reference the D9 invariant, line numbers in the report match the on-disk file, and the worktree is clean.

## Findings

### Critical
None.

### Important
None.

### Minor

**M1. `forbidden-rules.mjs:2296, 2346` — `\bself_cognition\b` is redundant with the dedicated function/type patterns.**

In groups A and B the `self_cognition` (bare) regex is listed alongside the dedicated function/type patterns (`load_self_cognition`, `append_self_cognition`, `SelfCognitionRow`, etc.). The dedicated patterns already match any line that contains the dedicated identifiers, and the bare `self_cognition` pattern additionally matches those same lines (because `load_self_cognition` contains `self_cognition` as a substring delimited by `_` and a word char — which is a word boundary on both sides). The intended purpose — catching raw SQL like `SELECT ... FROM self_cognition` — is still served because that string has no surrounding identifier prefix/suffix.

**Effect**: any line that trips a dedicated pattern will *also* trip the bare pattern and produce two failures instead of one. Functionally fine (the checker reports both and the offending line is still flagged), but noisy. The orchestrator's firing proof for group A shows this: `dream.rs:362: ...self_cognition table or module by name...` is preceded by the `self_cognition` SQL trigger, but a real `load_self_cognition` call would yield two messages.

**Smallest correct fix**: keep the bare pattern (it catches SQL the dedicated patterns cannot) and accept the redundancy, OR drop it and rely on the per-symbol patterns plus a separate manual check that `dream.rs` / `judge_memory.rs` have no raw SQL against `self_cognition`. I would keep it; the redundancy is a tolerable cost for the raw-SQL coverage. Worth a note in the rule's `message` so a future author understands why both patterns exist, but not a blocker.

**M2. `forbidden-rules.mjs:2253, 2258, 2263, 2278, 2283, 2293, 2298` — "dream/garden" wording in messages refers to a future migration not visible in this commit.**

Group A's messages repeatedly say "the dream/garden pass must never read or write it". T12 (mentioned in brief §2.3) is the task that migrates `dream` into a `garden` pass; in this commit the file is still called `dream.rs` and only `dream` is gated. The wording is consistent with the brief (which uses the same "dream/garden" phrasing in §3.2), so this is documentation drift rather than a code issue. A future tightening could say "the dream sweep (T12 will rename it garden)" — Minor.

**M3. `dream_d9_tests.rs:1-9` — module-level doc references "agentic-growth" module is a separate crate, not this one.**

The file's doc comment is clear and accurate. No action.

## Verification of the highest-value checks (priority order)

### 1. Every regex symbol exists as written (no silent no-op rules)

All 14 symbols cross-checked against the on-disk source:

| Symbol | Defined at |
|---|---|
| `load_self_cognition` | `service/agent_memory/self_cognition.rs:54` (access module); `agentic/growth_adapter.rs:184` (adapter) |
| `append_self_cognition` | `service/agent_memory/self_cognition.rs:86` |
| `count_self_cognition` | `service/agent_memory/self_cognition.rs:129` |
| `migrate_identity_into_self_cognition` | `service/agent_memory/self_cognition.rs:238` |
| `resolve_identity_path` | `service/agent_memory/self_cognition.rs:142` |
| `SelfCognitionRow` | `service/agent_memory/self_cognition.rs:33` |
| `SelfCognitionDbStore` | `agentic/growth_adapter.rs:134` |
| `init_self_cognition_store` | `agentic/growth_adapter.rs:176` |
| `SelfCognitionStore` | `agentic/growth_adapter.rs:37` (imported), defined in `northhing_agentic_growth::ports` |
| `conn_locked` | `service/agent_memory/memory_db.rs:69` |
| `set_blob` | `agentic/growth_adapter.rs:105` (GrowthStateStore trait impl on `JudgeMomStateStore`) |
| `boost_keyword` | `service/agent_memory/memory_db.rs:646` |
| `insert_fact` | `service/agent_memory/memory_db.rs:240` |
| `supersede_fact` | `service/agent_memory/memory_db.rs:805` |

All paths used in the rules resolve to real files (`dream.rs`, `judge_memory.rs`, `prompt_builder/` directory). No misspelled symbol or wrong path that would silently no-op.

### 2. Rule-list placement semantics are correct

Cross-checked against `scripts/core-boundaries/checker.mjs:971-977`:

- Groups A and B are inside `forbiddenContentRules` (exported on line ~3 per the brief). The checker iterates that list and calls `checkForbiddenContent` (line 843), which reads a **single file** at `rule.path` and matches `pattern.regex` line-by-line. Exact-file matching, as required.
- Group C is inside `forbiddenContentUnderRules` (line 2369 in the file, well past the `];` closer of `forbiddenContentRules` on line 2367). The checker iterates that list and calls `checkForbiddenContentUnder` (line 873), which **walks the directory** at `rule.path` and applies `pattern.allowPaths` exclusions per file. Subtree + per-pattern exception, as required.

No accidental swap.

### 3. Relocated test is not weaker than before

`dream.rs` post-move (`dream.rs:289-340`) keeps the six `parse_*` tests untouched. The D9 test was lifted verbatim into `dream_d9_tests.rs:24-67`. Diffed character-by-character against the pre-move `dream.rs:304-339`:

- Function name: `dream_payload_never_contains_self_cognition_sentinel` — identical.
- Sentinel string: `"T3B_DENSE_PATH_SENTINEL_我是自我认知标记"` — identical (orchestrator pre-verified byte-equality; I re-confirmed by reading both versions).
- Assertion 1: `assert!(!payload.contains(sentinel), "dream payload must not contain the self-cognition sentinel")` — preserved at `dream_d9_tests.rs:58-61`.
- Assertion 2 (anti-vacuity): `assert!(payload.contains("user prefers Rust for system tooling"), "dream payload must contain the fact text")` — preserved at `dream_d9_tests.rs:64-67`.

Test count: pre-move `dream.rs` had 7 `#[test]` functions; post-move `dream.rs::tests` has 6 + `dream::d9_tests` has 1 = 7. Orchestrator confirmed `cargo test ... dream` returns `7 passed`.

The new module is `mod d9_tests`, not `mod tests`, which is required because `dream.rs` already has an inline `mod tests` at `dream.rs:290` (brief §3.1 explicitly calls this out and the implementer followed it).

### 4. `allowPaths` is minimal and honest

- Group C: only one `allowPaths` entry across all 6 patterns, on the `SelfCognitionDbStore` pattern only: `['src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt_tests.rs']`.
- Verified that `SelfCognitionDbStore` actually appears at `system_prompt_tests.rs:37` (`let store = crate::agentic::growth_adapter::SelfCognitionDbStore::new(db);`) — the allowlist is targeted, not a directory wildcard.
- Verified that `system_prompt.rs` (the production file in the same directory) does NOT reference `SelfCognitionDbStore` literally (line 317 stores into `_store` but the type name is not written in the source). The orchestrator's spot-check injecting `SelfCognitionDbStore` into `system_prompt.rs:415` was caught — so the allowlist does not exempt the production file.
- No `allowPaths` is used to whitelist the very file the rule is constraining (the classic foot-gun the brief warns about). Group A targets `dream.rs` and the only `allowPaths` would be `dream_d9_tests.rs`, but the implementer chose a cleaner path: the per-file rule on `dream.rs` does not cover `dream_d9_tests.rs` (different file path), so no `allowPaths` is needed in group A or B at all. This is the better design — the negative test's single exception is "this file is not in scope" rather than "this file is in scope but exempted", which is harder to misread.
- The recorded exception for `init_self_cognition_store` is in the group `reason` text (line 3142) and accurately describes reality: `init_self_cognition_store` is the one-time idempotent identity.md migration trigger called as `let _store = init_self_cognition_store(&db);` at `system_prompt.rs:317`, where the store is constructed and immediately dropped. Its only observable effect is the migration write. The implementation correctly does not ban this symbol under `prompt_builder/`, and the reason text tells a future author why.

### 5. Scope discipline

`git diff --stat 9f261cd 9a9fb8a` shows exactly three files changed:

```
.../rules/source/forbidden-rules.mjs    | 158 +++++++++++++++++++++
.../core/src/service/agent_memory/dream.rs        |  65 +++-------
.../core/src/service/agent_memory/dream_d9_tests.rs  |  68 ++++++++
```

- `checker.mjs`, `self-test.mjs`, `crate-rules.mjs`, `crate-layout.mjs`, `feature-rules.mjs`, `facade-rules.mjs`, `required-rules.mjs` — all untouched (verified by `git diff --name-only`, which returned only the three allowed files).
- No `supersede` rule on `dream.rs` or `judge_memory.rs` (deferred to T12 per brief §2.3). Group C bans `supersede_fact` only under `prompt_builder/`, which the brief §3.4 explicitly listed as a write symbol to gate.
- Production behavior unchanged: the only Rust file with non-test code touched is `dream.rs`, and its non-test lines (production body, 1-287) are byte-identical to pre-move. The change inside `mod tests` is *removal* of the D9 test; the `#[path] mod d9_tests;` declaration at the bottom of the file only exists under `#[cfg(test)]`.
- No production log strings added, removed, or reworded.
- The 800-line constraint is satisfied: `dream.rs` is 348 lines post-move, `dream_d9_tests.rs` is 68 lines. (`forbidden-rules.mjs` is 3177 lines but is JS, not a Rust production file, so the 800-line constraint does not apply.)
- `cargo fmt` was not run (the implementer did not invoke it; I have not re-run anything either).

### 6. Rule messages

All messages name the invariant (D9: self-cognition is agent-exclusive; the prompt path is read-only on growth state), reference the seam being closed (`T3a seam`, `conn_locked escape hatch`), and tell a future author what to do instead of just that they tripped a lint. The `init_self_cognition_store` exception is documented in the `reason` (group C, line 3142), not in any pattern `message`, which is the correct location.

Minor only: see M1, M2 above.

## Cannot verify from diff

The brief asks for "what has already been verified — do not re-run". I did not re-run:

- `cargo test` counts (orchestrator confirmed dream 7, self_cognition 19, system_prompt 21, memory_db 28, auto_memory 7, growth_adapter 30, growth-crate 139).
- The `cargo check` warning baseline of 19 (orchestrator confirmed it stayed at 19).
- The line-by-line firing proof for all 28 patterns (orchestrator ran the in-scope spot-checks for group B and group C, and the report's table documents the full 28/28 with the captured checker output for each). The patterns are well-formed word-boundary regexes against symbols I independently verified exist; the spot-checks are sufficient.

`SPEC: PASS`
`QUALITY: PASS`
