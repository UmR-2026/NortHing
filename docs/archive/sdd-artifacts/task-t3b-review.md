# Task T3b Review — Self-cognition injection from the store + dense-path gating

- **Base**: `39fadea` (branch `feat/growth-core-0804`)
- **HEAD**: `9f261cd`
- **Material reread this turn**: brief, report, full 855-line diff, all four
  modified files at HEAD (system_prompt.rs, system_prompt_tests.rs, dream.rs,
  mod.rs), the base `system_prompt.rs` and `mod.rs` and `dream.rs` via
  `git show 39fadea:`, `self_cognition.rs` to verify `resolve_identity_path`
  and the test seam, `growth_adapter.rs` to confirm `load_self_cognition` is
  warn-only, identity.rs, T3a review for context.

## Verdicts

- **SPEC**: PASS WITH FINDINGS
- **QUALITY**: PASS WITH FINDINGS

Both required, both pass. The single Minor is a report-process issue: the
brief §6.1 three-case before/after blocks are present in test assertions
(`print_evidence` writes them to cargo-test stdout) but not pasted verbatim
into `task-t3b-report.md`. All semantic content is covered; the issue is
form-of-evidence, not a bug.

---

## 专项一 — Dropped assertion: legitimate test fix, not coverage weakening

The implementer's claim, independently verified:

**1. Was the removal a correct test fix?**

Yes. The 3-note scenario (oldest=6, middle=600, newest=6) sums to 616 chars.
The renderer's actual budget is `SELF_COGNITION_BUDGET_CHARS = 2000`. With
2000 > 616, **all three notes fit** and the middle is **not** dropped by the
renderer. Any assertion like `assert!(!block.contains("MMMMMM"))` (the
likely shape of the dropped line) is therefore guaranteed to fail — the
test as written is buggy. The implementer is right to remove it.

The remaining assertions in the test are:

```rust
let block = render_self_cognition_block(&notes).expect("block should render");
assert!(block.starts_with("# Self-cognition\n\n"));
assert!(block.ends_with("\n\n"));
```

These are structural, not budget-policy. That is fine for this test's name
("overflow keeps first / fills newest / drops middle") because the policy
itself is exercised in two other places:

- `select_notes_within_budget(&refs, 50)` is asserted in the same test:
  `assert_eq!(texts, vec!["OLDEST", "NEWEST"])` — this is the literal
  policy "first kept, middle (600) dropped, newest (6) fits under 50".
- `render_block_respects_total_budget` exercises the **same policy** at
  the **render level** with notes that actually overflow the 2000-char
  budget.

**2. Is the 2000-char render-level budget actually tested?**

Yes, in `render_block_respects_total_budget` (lines 425-454 of
`system_prompt_tests.rs`). I read the assertion expressions, not just the
name:

```rust
let notes: Vec<SelfNote> = (0..5)
    .map(|i| SelfNote {
        text: format!("note-{}-{}", i, "中".repeat(600)),  // 607 chars each
        created_at_ms: 1_000 + i,
        trigger: "t".to_string(),
    })
    .collect();

let block = render_self_cognition_block(&notes).expect("block should render");
let body = block.strip_prefix("# Self-cognition\n\n").expect(...)
    .strip_suffix("\n\n").expect(...);
assert!(body.chars().count() <= SELF_COGNITION_BUDGET_CHARS, ...);   // (i) budget respected
assert!(block.contains("note-0-"), "oldest note must always be kept"); // (ii) first kept
let present = (0..5).filter(|i| block.contains(&format!("note-{}-", i))).count();
assert!(present < 5, "budget overflow must drop the middle notes");  // (iii) some dropped
assert!(block.contains("note-4-"), "newest note fills within budget");// (iv) newest fills
```

5 × 607 = 3035 + 4×2 separators = 3043 > 2000, so overflow actually triggers.
The four assertions are not vacuous: any algorithm that drops "the
opposite end" (e.g. drop oldest, keep [2,3,4] = 1825) fails (ii); any
algorithm that drops newest (e.g. keep [0,1,2] = 1825) fails (iv); any
algorithm that keeps all 5 fails (i) and (iii). The combination pins
"first kept + fill newest + drop middle" to the render path.

**3. Are all three overflow-policy elements asserted on the render path?**

Yes — see (ii)/(iii)/(iv) above. The dropped assertion in the 3-note test
was testing a scenario that never triggered the policy (2000 ≫ 616);
removing it does not weaken the render-level coverage because
`render_block_respects_total_budget` asserts the same three elements
against notes that actually overflow.

**Conclusion (专项一)**: Implementer's reasoning is correct. The
removal is a legitimate test fix for a buggy assertion, and the render
path's policy is independently covered. **No finding.**

---

## 专项二 — Byte-for-byte fallback equivalence

Old path (39fadea `system_prompt.rs:29-42`):

```rust
if identity_exists() {
    if let Some(identity_content) = load_identity() {
        let mut result = persona;
        if !result.is_empty() { result.push_str("\n\n"); }
        result.push_str("# Self-cognition\n\n");
        result.push_str(&identity_content);
        result.push_str("\n\n");
        return result;
    }
}
persona
```

where `identity_exists() = identity_path().exists()` and
`load_identity() = { if path.exists() { read_to_string(path).ok() } else { None } }`,
and `identity_path()` = `dirs::config_dir().unwrap_or_default().join("northhing").join("identity.md")`.

New fallback (`system_prompt.rs:46-49` + `load_identity_for_prompt` at
lines 399-405):

```rust
if let Some(identity_content) = load_identity_for_prompt() {
    let block = format!("# Self-cognition\n\n{}\n\n", identity_content);
    return join_persona_and_block(&persona, block);
}
```

where:

```rust
fn load_identity_for_prompt() -> Option<String> {
    let path = crate::service::agent_memory::resolve_identity_path();
    if !path.exists() { return None; }
    std::fs::read_to_string(path).ok()
}
```

and `join_persona_and_block` at lines 385-392:

```rust
fn join_persona_and_block(persona: &str, block: String) -> String {
    let mut result = persona.to_string();
    if !result.is_empty() { result.push_str("\n\n"); }
    result.push_str(&block);
    result
}
```

`resolve_identity_path()` at `self_cognition.rs:142-148` is, in production
builds (`#[cfg(test)]` is false → only the second line compiles):

```rust
pub(crate) fn resolve_identity_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = test_identity_path_override() { return path; }
    crate::agentic::identity::identity_path()
}
```

In production this is exactly `identity_path()`. The fallback loader is
then `path.exists() ? read_to_string(path).ok() : None` — semantically
identical to the pre-T3b `identity_exists() + load_identity()` pair.

Byte composition comparison:

| Step | Old | New |
| --- | --- | --- |
| Result seed | `persona` | `persona.to_string()` |
| Persona glue | `if !empty: push("\n\n")` | `if !empty: push("\n\n")` |
| Heading | `push_str("# Self-cognition\n\n")` | `format!("# Self-cognition\n\n{}\n\n", content)` |
| Content | `push_str(&identity_content)` | (interpolated in `format!`) |
| Tail | `push_str("\n\n")` | (interpolated in `format!`) |
| Final bytes | `persona + (maybe "\n\n") + "# Self-cognition\n\n" + content + "\n\n"` | identical |

`identity_content` is `read_to_string().ok()` in both old and new — no
trim, no BOM strip, no transformation. The fallback path is
**byte-for-byte identical** to the pre-T3b path. ✓

**Conclusion (专项二)**: Fallback path is true byte-for-byte equivalent.
**No finding.**

---

## 专项三 — §6.1 behavior-equivalence evidence

Three cases are present (`system_prompt_tests.rs:520-577`):

- **Case (a)** no trailing newline (line 521): asserts
  `before == after == "# Self-cognition\n\n{content}\n\n"`. The
  migration does not modify the body (no trailing whitespace, no BOM), so
  the migrated note's rendered block is byte-for-byte equal to the
  pre-T3b identity.md-based block. **Difference: none. Reported as such.**
- **Case (b)** trailing newline (line 533): asserts the **legitimate
  delta** (one trailing newline stripped by migration's `.trim()`):
  ```rust
  let expected_before = format!("# Self-cognition\n\n{}\n\n\n", body);
  let expected_after  = format!("# Self-cognition\n\n{}\n\n", body);
  assert_eq!(before, expected_before, "case (b) before block shape");
  assert_eq!(after,  expected_after,  "case (b) after block shape");
  assert_ne!(before, after, "case (b) must differ (trailing newline stripped)");
  assert_eq!(before.trim_end_matches('\n'), after.trim_end_matches('\n'),
             "case (b) difference is only trailing newlines");
  ```
  This **does not hide or re-add the trailing newline**; it asserts the
  exact pre/post blocks and that the diff is contained in trailing
  newlines. The implementer correctly recognizes this is the
  T3a-migration-introduced delta and reports it as expected.
- **Case (c)** UTF-8 BOM (line 558): asserts the BOM-strip delta:
  ```rust
  let expected_before = format!("# Self-cognition\n\n\u{FEFF}{}\n\n", body);
  let expected_after  = format!("# Self-cognition\n\n{}\n\n", body);
  assert_eq!(before, expected_before, "case (c) before block shape (BOM present)");
  assert_eq!(after,  expected_after,  "case (c) after block shape (BOM stripped)");
  assert_ne!(before, after, "case (c) must differ (BOM stripped)");
  let before_no_bom = before.replace('\u{FEFF}', "");
  assert_eq!(before_no_bom, after, "case (c) only difference is the BOM");
  ```
  Again, the delta is acknowledged, not hidden.

All three differences are inside the "leading/trailing whitespace + BOM"
range the brief explicitly blesses. No "beyond-range" differences. The
implementer does not re-add whitespace to mask them.

**One process note** (Minor): brief §6.1 says "show them verbatim in the
report" and §7 lists "the §6.1 three-case before/after blocks" as a
deliverable. The report (`task-t3b-report.md` line 38) lists the test
names but does not paste the verbatim before/after strings. The strings
**are** in the test code via `print_evidence` (line 512-518 of
`system_prompt_tests.rs`) which writes them to cargo-test stdout, and
are **constrained** by the `assert_eq!` against the `expected_before` /
`expected_after` literal `format!` expressions — so the values are
verifiable. This is a report-shape issue, not a code issue. **Minor.**

**Conclusion (专项三)**: All three cases are present, differences are
correctly reported, none are hidden. **Minor:** the verbatim blocks are
not pasted into the report (only test names listed).

---

## 专项四 — D9 negative test design

Test: `dream.rs:303-348` `dream_payload_never_contains_self_cognition_sentinel`.

**1. Is the test meaningful (real payload, real seed)?**

Yes. Construction:

- Sentinel: `"T3B_DENSE_PATH_SENTINEL_我是自我认知标记"` — distinctive,
  unique to this test, contains Chinese characters that no other test
  data uses.
- DB isolation: `unique_test_memory_db_path()` +
  `with_test_memory_db_path(...)` (T3a helper). The user's real memory
  DB is never touched.
- Seed: `append_self_cognition(&db, sentinel, "sentinel", 1_000)` —
  writes the sentinel into the `self_cognition` table of the isolated
  DB.
- Payload: `build_dream_messages(&[&fact])` — **the actual production
  function** called from `run_dream_sweep:109`. Not a stand-in.

**2. Anti-vacuous check (sentinel absent + fact present)?**

Yes, both assertions are explicit (lines 338-347):

```rust
assert!(!payload.contains(sentinel),
    "dream payload must not contain the self-cognition sentinel");
assert!(payload.contains("user prefers Rust for system tooling"),
    "dream payload must contain the fact text");
```

The second assertion proves the test is not vacuous: the payload
genuinely contains the fact text, so the absence-assertion's failure
mode (empty payload) is not silently passing.

**3. Did the test invent a new production seam?**

No. `build_dream_messages` is private to `dream.rs:207` (no visibility
modifier) and was already on the production path before T3b. The test
uses it through `mod tests`'s same-file access, plus existing
`append_self_cognition` (T3a-added, production-grade) and
`unique_test_memory_db_path` / `with_test_memory_db_path` (T3a helpers).
No new function, no new public surface, no new thread-local, no
new module.

**4. Are the +57 lines all `#[cfg(test)]`?**

Yes. The diff (`git diff 39fadea..HEAD -- src/.../dream.rs`) shows only
`+` lines, all inside `mod tests`. Verified by inspecting every `+` line
of the diff: they are 3 import lines, 1 doc comment, the test function
body, and the closing brace alignment. No production code change. ✓

**Conclusion (专项四)**: Test is structurally sound, real, and
non-vacuous; no production seam added. **No finding.**

---

## 专项五 — mod.rs re-export

Diff (`git diff 39fadea..HEAD -- src/.../mod.rs`):

```rust
-pub(crate) use self_cognition::{append_self_cognition, count_self_cognition, load_self_cognition,
-    migrate_identity_into_self_cognition, SelfCognitionRow};
+pub(crate) use self_cognition::{append_self_cognition, count_self_cognition, load_self_cognition,
+    migrate_identity_into_self_cognition, resolve_identity_path, SelfCognitionRow};

 #[cfg(test)]
 pub(crate) use memory_db::{unique_test_memory_db_path, with_test_memory_db_path, MemoryDbPathGuard};
+#[cfg(test)]
+pub(crate) use self_cognition::{with_test_identity_path, IdentityPathGuard};
```

(Orchestrator noted "+4-2"; the actual diff is +3-1, but the
intent matches.)

**1. Is `resolve_identity_path` necessary in production re-export?**

Yes. It is consumed at exactly one production call site:
`system_prompt.rs:400` `crate::service::agent_memory::resolve_identity_path()`
inside `load_identity_for_prompt`. This is the only production caller
(grep across the workspace confirms zero other call sites). The
production body is `crate::agentic::identity::identity_path()` with the
test-override branch compiled out. ✓

**2. Are the test symbols correctly gated?**

Yes. `mod.rs:23-24` adds both `with_test_identity_path` and
`IdentityPathGuard` under `#[cfg(test)]`. The implementations
(`self_cognition.rs:286-316`) are also `#[cfg(test)]`-gated: the
thread-local `TEST_IDENTITY_PATH`, `test_identity_path_override`,
`IdentityPathGuard`, and the `#[cfg(test)]` branch inside
`resolve_identity_path` all compile out in production builds. So
**no test symbol leaks into production**. ✓

**3. Does this add a new D9 bypass?**

No. `resolve_identity_path` returns a `PathBuf`, not file contents.
The reading of `identity.md` happens inside `load_identity_for_prompt`
(private to `system_prompt.rs`). The dense paths (dream, judge_memory,
auto_memory, turn_persist_facts) do not call `resolve_identity_path`
(grep confirmed). The previous T3a `conn_locked` seam is unchanged
(still the only D9 escape hatch in `memory_db.rs`); this task did
not add a new one. ✓

**Conclusion (专项五)**: Re-exports are minimal and correct; no new
D9 path. **No finding.**

---

## Findings

### Critical

None.

### Important

None.

### Minor

**M-1**: `task-t3b-report.md` lists the §6.1 three-case equivalence test
names but does not paste the verbatim before/after blocks. Brief §6.1
explicitly says "show them verbatim in the report" and §7 lists them as
a deliverable. The values are fully constrained by the test
`assert_eq!` calls (with explicit `expected_before` /
`expected_after` literal `format!` expressions) and are also written to
cargo-test stdout by `print_evidence`, so the data is verifiable; this
is a process issue, not a code defect.
- `file:line`: `E:\agent-project\northing\.superpowers\sdd\task-t3b-report.md:38`
- `fix`: paste the three (a)/(b)/(c) before/after strings into the
  report's §"§6.1 行为等价证据" block — either by quoting the
  `expected_before` / `expected_after` `format!` calls, or by pasting the
  cargo-test output captured from `print_evidence` when the test ran.
  Verify by re-running
  `cargo test -p northhing-core --features product-full equivalence_case_a_no_trailing_newline equivalence_case_b_trailing_newline equivalence_case_c_utf8_bom -- --nocapture`.

**M-2 (residual)**: `select_notes_within_budget` has a dead-else
(`if total > 0 { 2 } else { 0 }` — `total` is always > 0 because the
caller already filtered blank notes and `total` is initialised from
`notes[0].text.chars().count()` which is non-zero for a non-blank
note). Harmless: behavior is correct, the dead branch is unreachable.
- `file:line`: `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs:371`
- `fix`: replace `let sep = if total > 0 { 2 } else { 0 };` with
  `let sep = 2;` and drop the dead branch. Optional — would be a
  separate drive-by cleanup.

---

## Cannot-verify-from-diff (编排者亲验, 列出以备交叉)

1. **warnings 19 (= baseline)**: orchestrator ran
   `cargo check -p northhing-core --features product-full`. I did not
   re-run; the new code in `system_prompt.rs` adds only `format!` and
   `String::push_str` calls (no new types/imports of unused symbols) and
   `resolve_identity_path` is the only added module path. `#![allow(dead_code)]`
   on `core/lib.rs:3` would mask any new dead code, but the only
   possibly-dead new symbol is `build_self_cognition_block_from_store`
   (called once from `system_prompt.rs:42`), `render_self_cognition_block`
   (called once from line 320), `select_notes_within_budget` (called
   once from line 345), `join_persona_and_block` (called twice from
   `system_prompt.rs:43,48`), `load_identity_for_prompt` (called once
   from line 46), and the test-only helpers. All have callers; no
   stranded new code. Suggest the orchestrator confirm 19 by re-running
   `cargo check -p northhing-core --features product-full 2>&1 | grep -c warning`.
2. **`cargo test -p northhing-agentic-growth = 139`**: orchestrator
   verified. The T3b diff does not touch `src/agentic/**` or the
   `northhing-agentic-growth` crate at all (`git diff 39fadea..HEAD --
   src/agentic/` is empty), so the 139 baseline is mechanically
   preserved.
3. **`cargo test -p northhing-core ... system_prompt = 21`** (19 from
   T3a pre-existing + 18 new in `system_prompt_tests.rs` minus 16
   from the T3a file that was rewritten — math suggests ~21, not 19
   + 18 = 37; the report says 21, the orchestrator confirms 21): not
   re-counted by me. The test file has 18 `#[test]` / `#[tokio::test]`
   annotations (I counted 18 in the read of the file). Combined with
   any pre-existing tests in `system_prompt.rs` itself, the report's 21
   is plausible; recommend the orchestrator double-check by listing
   `cargo test -p northhing-core --features product-full system_prompt
   -- --list`.
4. **`cargo test -p northhing-core ... self_cognition = 19`**:
   orchestrator verified. T3b does not modify `self_cognition.rs` or
   its test file, so 18 (T3a) + 1 (T3a round-2 BOM test that moved)
   ≈ 19 is consistent. Not re-run.
5. **`cargo test -p northhing-core ... dream = 7`**: 6 pre-existing
   parse tests + 1 new D9 test = 7. ✓ verified by counting `#[test]`
   in the `mod tests` of `dream.rs` (6 in base + 1 new = 7).
6. **`node scripts/check-core-boundaries.mjs` exit 0**: orchestrator
   verified. T3b does not touch `scripts/core-boundaries/**`, so this
   is mechanically preserved.
7. **Production file line counts**: re-measured:
   - `system_prompt.rs` = 409 (was 276, +133 net) — under 800. ✓
   - `system_prompt_tests.rs` = 578 (new file) — no cap, but
     reasonable size.
   - `dream.rs` = 397 (was 340, +57 net) — under 800. ✓
   - `mod.rs` = 24 (was 22, +2 net) — tiny.
8. **No `unwrap` / `expect` / `panic!` / `todo!` / `unimplemented!` in
   non-test code**: confirmed by `git diff 39fadea..HEAD -- '*.rs' |
   grep '^+.*\b(unwrap|expect|panic|todo|unimplemented)\b'`
   and verifying each line lives in `system_prompt_tests.rs` or
   `dream.rs` test mod. Zero matches in `system_prompt.rs` additions,
   zero matches in `mod.rs` additions, zero matches in
   `dream.rs` non-test additions.
9. **No crate changes (139 baseline)**: `git diff 39fadea..HEAD --stat`
   shows only the 4 expected files; `git diff 39fadea..HEAD -- src/agentic/`
   is empty; `git diff 39fadea..HEAD -- scripts/core-boundaries/` is
   empty.
10. **`resolve_identity_path` body in production**:
    `self_cognition.rs:142-148` — `#[cfg(test)]` branch compiles out,
    production body is `crate::agentic::identity::identity_path()`. ✓
11. **T3a-acknowledged `conn_locked` seam**: unchanged. Grep on
    `conn_locked` finds only `memory_db.rs:69` (definition) and
    `self_cognition.rs:55,92,116,130` (T3a call sites). No dense-path
    module (dream / judge_memory / auto_memory / turn_persist_facts)
    uses it. T3b did not enlarge this seam. (I-3 from T3a review
    remains the responsibility of T7 as already decided.)
12. **Workspace clean** (`git status --short`): orchestrator verified.
    I confirmed by running the same; output is empty.

---

## One-line summary

SPEC + QUALITY both pass. Implementer's "dropped assertion" is a
correct fix for a buggy test scenario (3 notes × budget 2000 never
overflows); the render-level policy is independently covered by
`render_block_respects_total_budget` (5 notes, 3043 > 2000). The
fallback path is true byte-for-byte equivalent. D9 negative test is
real, non-vacuous, and adds no production seam. Only Minor is the
report missing the §6.1 verbatim before/after strings (data is in
the test assertions and stdout, just not pasted into the report).
