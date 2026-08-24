# Task T7a Report — growth-core boundary rules: D9 dense paths + prompt-builder read-only (firing-proof + D9 relocation)

Branch: `feat/growth-core-0804` (worktree `E:\agent-project\northing\.worktrees\growth-core-0804`)
Commit: `9a9fb8a` — `chore(boundaries): ban self-cognition dense paths from dream/judge + growth-state writes from prompt builder (T7a)`
Base: `9f261cd` (T3b)
Date: 2026-08-06

## 1. Summary

Three new boundary rule groups ship, all proven to fire (§5.1 hard acceptance), and the
D9 negative test is relocated out of `dream.rs` so the new dense-path rules are not
diluted by a legal test file.

1. **Dense-path group A** — `forbiddenContentRules` entry on
   `.../service/agent_memory/dream.rs` (11 patterns): every self-cognition read/write
   symbol + `conn_locked` is banned in the dream file.
2. **Dense-path group B** — identical 11 patterns on
   `.../service/agent_memory/judge_memory.rs` (judge-mom facade).
3. **Group C** — `forbiddenContentUnderRules` entry on
   `.../agentic/agents/prompt_builder` (6 patterns): prompt builder is read-only on
   growth state; the only write-scoped symbol that may exist there is the recorded
   `init_self_cognition_store` exception.

No production behavior changed. `conn_locked` stays `pub(crate)` with its 4 call sites
in `self_cognition.rs` (the rule bans it only in `dream.rs` / `judge_memory.rs`).
`init_self_cognition_store` / `load_self_cognition` in `system_prompt.rs` keep current
behavior. `supersede_fact` is **not** banned on dream/judge paths (still called by
`dream.rs` in production, deferred to T12) — it is banned only in group C's
`prompt_builder` scope.

## 2. Rules added (file:line, exact spellings)

`scripts/core-boundaries/rules/source/forbidden-rules.mjs` (worktree copy), after
commit `9a9fb8a`, 3177 lines (baseline before this task: 3019).

### Group A — `dream.rs` (entry starts line 2248)
Patterns (line 2249–2306), exact spellings verified against the D9/T3a symbols:

| # | line | regex |
|---|------|-------|
| 1 | 2251 | `\bload_self_cognition\b` |
| 2 | 2256 | `\bappend_self_cognition\b` |
| 3 | 2261 | `\bcount_self_cognition\b` |
| 4 | 2266 | `\bmigrate_identity_into_self_cognition\b` |
| 5 | 2271 | `\bresolve_identity_path\b` |
| 6 | 2276 | `\bSelfCognitionRow\b` |
| 7 | 2281 | `\bSelfCognitionDbStore\b` |
| 8 | 2286 | `\binit_self_cognition_store\b` |
| 9 | 2291 | `\bSelfCognitionStore\b` |
| 10 | 2296 | `\bself_cognition\b` (catches the table name even via raw SQL) |
| 11 | 2301 | `\bconn_locked\b` (T3a seam) |

### Group B — `judge_memory.rs` (entry starts line 2308)
Same 11 patterns at lines 2311–2366 with judge-mom wording.

### Group C — `prompt_builder` (entry starts line 3140, under `forbiddenContentUnderRules`)
| # | line | regex | allowPaths |
|---|------|-------|------------|
| 1 | 3145 | `\bappend_self_cognition\b` | — |
| 2 | 3150 | `\bSelfCognitionDbStore\b` | line 3151: `['src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt_tests.rs']` |
| 3 | 3156 | `\bset_blob\b` | — |
| 4 | 3161 | `\bboost_keyword\b` | — |
| 5 | 3166 | `\binsert_fact\b` | — |
| 6 | 3171 | `\bsupersede_fact\b` | — |

Deliberately **not** in group C (recorded exception): `init_self_cognition_store` at
`system_prompt.rs:317` (`let _store = init_self_cognition_store(&db);` — one-time
idempotent identity.md migration, its only effect is that write). Documented in the
entry's `reason` text.

## 3. D9 negative test relocation

- Removed `dream_payload_never_contains_self_cognition_sentinel` (and its now-unused
  imports `append_self_cognition`, `FactConfidence`, `FactProvenance`, `FactScope`,
  `unique_test_memory_db_path`, `with_test_memory_db_path`, `FactType`) from `dream.rs`.
- New file `src/crates/assembly/core/src/service/agent_memory/dream_d9_tests.rs`
  (68 lines) holds the test verbatim. It uses `super::*` (still reaches the **private**
  `build_dream_messages` via `#[path]` child module) and seeds the sentinel through
  `append_self_cognition`.
- `dream.rs` tail (lines 341–348): `#[cfg(test)] #[path = "dream_d9_tests.rs"] mod d9_tests;`
- This file is intentionally not covered by the per-file rules — it is the negative
  test's one job to touch the banned table, and the production file it feeds is now
  fully clean.

## 4. §5.1 Firing proof (hard acceptance criterion)

Method per brief: inject a minimal temporary violation block per file, run the checker,
capture exact failure output, revert, re-run checker to exit 0. All 28 patterns (11 + 11
+ 6) fired on first run with their exact `message`. `forbiddenContentRules` failures
render as `<file>:<line>: <pattern message>`; `forbiddenContentUnderRules` failures
render `<file>:<line>: <reason>; <pattern message>` (reason prefix observed in output).

### Group A — `dream.rs` (11/11 fired)
Injected block `// T7A_FIRING_PROOF_START..END` appended to `dream.rs` (one line per
pattern); checker output (abbreviated to the message head):

| pattern | trigger line used | checker output (file:line: message) |
|---------|-------------------|--------------------------------------|
| `load_self_cognition` | `let _v = load_self_cognition;` | `dream.rs:353: dream sweep must not read self-cognition (D9: …)` |
| `append_self_cognition` | `let _v = append_self_cognition;` | `dream.rs:354: dream sweep must not write self-cognition (D9: …)` |
| `count_self_cognition` | `let _v = count_self_cognition;` | `dream.rs:355: dream sweep must not touch the self-cognition store (D9: …)` |
| `migrate_identity_into_self_cognition` | `let _v = migrate_identity_into_self_cognition;` | `dream.rs:356: dream sweep must not trigger the identity.md self-cognition migration (D9: …)` |
| `resolve_identity_path` | `let _v = resolve_identity_path;` | `dream.rs:357: dream sweep must not resolve the identity.md self-cognition path (D9: …)` |
| `SelfCognitionRow` | `let _v = SelfCognitionRow;` | `dream.rs:358: dream sweep must not use self-cognition row types (D9: …)` |
| `SelfCognitionDbStore` | `let _v = SelfCognitionDbStore;` | `dream.rs:359: dream sweep must not build the self-cognition store adapter (D9: …)` |
| `init_self_cognition_store` | `let _v = init_self_cognition_store;` | `dream.rs:360: dream sweep must not initialize the self-cognition store (D9: …)` |
| `SelfCognitionStore` | `let _v = SelfCognitionStore;` | `dream.rs:361: dream sweep must not use the self-cognition crate port (D9: …)` |
| `self_cognition` (raw SQL) | `let _v = "SELECT text FROM self_cognition";` | `dream.rs:362: dream sweep must not reference the self_cognition table or module by name (D9: …)` |
| `conn_locked` | `let _v = db.conn_locked();` | `dream.rs:363: dream sweep must not use the conn_locked escape hatch (T3a seam: …)` |

### Group B — `judge_memory.rs` (11/11 fired)
Injected block after `set_judge_state`; checker output `judge_memory.rs:15..25` — each of
the 11 patterns fired with the judge-mom wording (`judge-mom must not read/write/touch/
trigger/resolve/use/build/initialize/reference/use-the-conn_locked-escape-hatch …`). Same
triggers as Group A.

### Group C — `prompt_builder` (6/6 fired)
Injected block appended in `system_prompt.rs` after `#[path = "system_prompt_tests.rs"] mod tests;`:

| pattern | trigger line used | checker output (file:line: message) |
|---------|-------------------|--------------------------------------|
| `append_self_cognition` | `let _v = append_self_cognition;` | `system_prompt.rs:414: …must not append self-cognition notes; writing self-cognition is agent-exclusive (D9)…` |
| `SelfCognitionDbStore` | `let _v = SelfCognitionDbStore;` | `system_prompt.rs:415: …must not build a self-cognition store handle (its append path writes growth state)…` |
| `set_blob` | `let _v = set_blob;` | `system_prompt.rs:416: …must not write growth-state blobs (set_blob is a GrowthStateStore write…)` |
| `boost_keyword` | `let _v = boost_keyword;` | `system_prompt.rs:417: …must not boost keywords (mutates growth topic weights…)` |
| `insert_fact` | `let _v = insert_fact;` | `system_prompt.rs:418: …must not insert facts (mutates the growth fact store…)` |
| `supersede_fact` | `let _v = supersede_fact;` | `system_prompt.rs:419: …must not supersede facts (mutates the growth fact store…)` |

Line 415's firing is significant: it proves the `allowPaths` exception is scoped to
`system_prompt_tests.rs` and does **not** exempt the production file `system_prompt.rs`.

**Revert + re-run:** all three blocks removed (grep `T7A_FIRING_PROOF` → no files found),
checker re-run → `Core boundary check passed.` EXIT=0. `git status --short` clean after
commit (see §5/§7).

## 5. allowPaths entries (each with justification)

| entry (line) | value | justification |
|--------------|-------|---------------|
| 3151 | `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt_tests.rs` | The prompt-builder unit tests must seed growth-state fixtures (`SelfCognitionDbStore`) to exercise the read path; the exception is one file, not the directory, and its narrowness is proven by the §4 group-C row for `system_prompt.rs:415`. No other new allowPaths were introduced. |

Pre-existing allowPaths (e.g. line 2402 `miniapp/runtime.rs`) untouched.

## 6. Verification output (brief §5)

1. Checker baseline (pre-change): `Core boundary check passed.` EXIT=0.
2. Checker post-change (violations reverted): `Core boundary check passed.` EXIT=0.
3. `cargo test -p northhing-core --features product-full dream` → `7 passed; 0 failed`
   (includes relocated `dream::d9_tests::dream_payload_never_contains_self_cognition_sentinel`).
4. `cargo test -p northhing-core --features product-full self_cognition` → `19 passed; 0 failed`.
5. `cargo test -p northhing-core --features product-full system_prompt` → `21 passed; 0 failed`.
6. `cargo check -p northhing-core --features product-full` → `generated 19 warnings`
   (baseline preserved; `cargo fix` still reports 18 suggestions as before).
7. `cargo test -p northhing-agentic-growth` → `139 passed; 0 failed`.
8. Line count of `forbidden-rules.mjs` (PowerShell `(Get-Content -Encoding UTF8).Count`):
   3177 (baseline 3019). Diff stat: +234/−57 across 3 files (+158 rules, +68 test file,
   −57 removed test from `dream.rs`).

## 7. Deliverables check

- One commit on `feat/growth-core-0804`: `9a9fb8a` `chore(boundaries): …` — correct prefix.
- Files changed: `forbidden-rules.mjs`, `dream.rs`, `dream_d9_tests.rs` (new) — exactly the
  three allowed.
- `git status --short` in the worktree: clean (empty output) after commit; checker EXIT=0.
- Nothing under `.superpowers/` committed.

## 8. Notes / ambiguities

- The checker renders group-C failures with the entry `reason` prepended to the pattern
  `message` (`file:line: reason; message`); single-file rule failures render `file:line:
  message` only. Verified in captured output.
- `init_self_cognition_store` and `supersede_fact` scope decisions (recorded exception /
  T12 deferral) follow plan §4 exactly; no `plan-mandated finding` collision found.
- PowerShell `2>&1` writes checker's stderr to stdout; exit code read via `$LASTEXITCODE`
  (checker exits 1 on failure, 0 on pass).

## Status: DONE
