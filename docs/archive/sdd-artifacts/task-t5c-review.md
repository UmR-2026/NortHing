# Review — Task T5c

Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
BASE: `9a9fb8a` HEAD: `2e986ce`
Diff: `E:\agent-project\northing\.superpowers\sdd\task-t5c-diff.md`
Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t5c-report.md`

Single file changed: `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`
Diff stat per `git diff --stat`: `82 insertions(+), 2 deletions(-)`.
`git status --short` clean. No files outside the one target file. No `.superpowers/` artifacts.

I worked from the **committed blob** in HEAD (LF line endings) for all byte-level checks, plus
the on-disk worktree (CRLF, autocrlf-normalized) for layout. Both forms agree on content where
relevant — the only difference is `\r\n` ↔ `\n` line terminators, which is a pre-existing
property of the file (entirely CRLF on disk for all 675 lines, entirely LF in the Git blob),
not introduced by this commit.

---

## Highest-value checks (in priority order)

### 1. Verbatim fidelity of the four blocks — PASS

For each block I read brief §3 (the fenced `text` block) into a Python variable and verified
that the substring occurs byte-for-byte in the committed blob:

- **C1** (brief §3 C1, file lines 224–229): matched at UTF-8 offset 14066 in the blob.
- **C2** (brief §3 C2, file line 231): matched at offset 14438.
- **C3** (brief §3 C3, file lines 249–254): matched at offset 15798.
- **C4** (brief §3 C4, file lines 245–247): matched at offset 15440.

### 2. The D14 hyphen — PASS (the highest-priority check)

Brief §3 mandates the ASCII hyphen in `see - the system owns that.` and explicitly warns that
"an ASCII hyphen must not have become an em dash". I located the substring in the raw bytes
and dumped the surrounding bytes:

```
hex: 73 65 65 20 2d 20 74 68 65 20 73 79 73 74 65 6d 20 6f 77 6e 73 20 74 68 61 74
text: see - the system owns that
```

The hyphen byte is `0x2D` (ASCII hyphen-minus). The line wraps with `0x0A` (LF). No em dash
(`0xE2 0x80 0x94`) or en dash (`0xE2 0x80 0x93`) was introduced in any of the four added
blocks. The em dash present in the pre-existing line 222 (`now — and update…`) is unchanged
across BASE→HEAD and is not part of any of the four additions.

### 3. Backticks around `` `# Remembered facts` `` — PASS

The C3 prose writes the phrase as inline code (`` `# Remembered facts` ``). I grepped for
backticks around the phrase: the only occurrences in the committed blob are
- `The \`# Remembered facts\` block` at line 251 (within C3), bytes `23 23 20 41 75 74 …
  60` (`#` `#` space `A` … backtick before/after).
- Test assertion string and the production format string (separately verified).

The backticks differentiate the C3 inline-code reference from the heading that production code
formats as `format!("\n\n# Remembered facts\n\n{}", items)` at line 304.

### 4. Section placement and order — PASS

Per brief §3 "Resulting section order" the final order must be:
`## When to access memories` → `## Before recommending from memory` → `## How to apply memory
in your answer` (C4 new) → `## Auto-captured facts vs. your memory files` (C3 new) →
`## Memory and other forms of persistence`.

Headings in the committed blob at byte offsets:
- `## When to access memories` ............ 13267
- `## Before recommending from memory` ... 14723
- `## How to apply memory in your answer`  15440
- `## Auto-captured facts vs. your memory files` 15798
- `## Memory and other forms of persistence` 16100

Strictly increasing. The newline between `## Before recommending…` (line 233) and its closing
`A memory that summarizes repo state…` paragraph (line 243) is preserved, then `## How to
apply memory in your answer` follows at line 245 — i.e. C4 is placed *after* the Before-
recommending section's last paragraph as the brief specifies. ✓

### 5. Purely additive to the prompt — PASS

I diffed BASE and HEAD at the line level and counted:
- 2 deleted lines, both the old assertion `!prompt.contains("# Remembered facts"),` (lines
  that were in the empty-facts test at line 489 and in the unreadable-file test at line 532
  in BASE).
- 82 insertions, of which **20 are inside the raw string** (the four added blocks), **2 are
  in-place assertion-line replacements** (lines 490, 533 in HEAD) of the same `!prompt.contains`
  calls with the tightened pattern, and **60 are in the new `#[tokio::test]`** at line 542.
- The raw string closing `"#` moved from line 240 (BASE) to line 260 (HEAD): +20 lines, fully
  accounted for by the four additions inside the raw string. 240 + 20 = 260. ✓

Pre-existing prompt sentences around the additions were verified byte-identical between BASE
and HEAD (e.g. the "trust what you observe now — and update…" line at 222 and the
"A memory that summarizes repo state…" line at 243/234).

### 6. Brace / raw-string integrity — PASS

In the additions region (offset 14066 → 16099, between the start of `Some requests cannot…` and
`## Memory and other forms of persistence`) I counted:
- `{` occurrences: 0
- `}` occurrences: 0

The pre-existing brace tokens are at byte-identical offsets in BASE and HEAD:
- `{memory_dir_display}` (line 114 in both) at offset 3502
- `{{memory name}}` (line 202) at offset 12233
- `{{one-line description…}}` (line 203) at offset 12266
- `{{user, feedback, project, reference}}` (line 204) at offset 12368
- `{{memory content…}}` (line 207) at offset 12416

No escaping was needed because no literal `{` or `}` exists in the four additions, and none
was introduced.

### 7. The two tightened assertions — non-vacuous, correctly targeted

Both modified assertions now use the exact rendering format produced by line 304:

```rust
format!("\n\n# Remembered facts\n\n{}", items)
```

Pattern: `"\n\n# Remembered facts\n\n"`.

- C3 prose contains `# Remembered facts` surrounded by **backticks** (inline-code form), not
  by `\n\n` newlines — so the tightened pattern cannot match the prose.
- The only places this full pattern appears in the blob are: line 304 (production format
  string literal), lines 490 and 533 (the new assertion strings themselves), and lines 467,
  468, 491, 512, 534 (other test fixtures/strings that include the substring without the
  `\n\n` flanking — verified by `re.findall` returning zero source matches outside these test
  regions).

The implementer's report provides empirical non-vacuity proof: each tightened assertion was
temporarily inverted to `prompt.contains("\n\n# Remembered facts\n\n")` and observed to fail,
then reverted. Both fail messages have specific line numbers and reasons (`PROOF: inverted
assertion to demonstrate non-vacuity`). I did not re-run these (orchestrator report is the
evidence) but the test logic is sound: at runtime, the assertion becomes
  - true ⟺ the production `# Remembered facts` block is NOT in the rendered prompt
  - i.e. precise behavior of `format!("…{}", items) → String::new()` when `selected.is_empty()`.

### 8. The new test's value — non-vacuous, distinctive substrings, correct ordering

`prompt_includes_all_four_memory_guidance_additions` (lines 542–600) does two things:

(a) **Content presence** via the four substrings the brief §4 explicitly approves:
   - `"a single-line command or a one-off shell invocation"` (C1)
   - `"at most 4-6 memory reads or searches"` (C2)
   - `"You are not expected to deduplicate against facts you cannot"` (C3)
   - `"Do not narrate the retrieval"` (C4)

   Each of those is a multi-word phrase unique to its block — removing or paraphrasing the
   corresponding block would break the assertion. Each `assert!` has a message naming the
   guidance item per brief §4.

(b) **Strict ordering** via five-string `prompt.find(...)` `<` relations that encode the
   exact brief §3 final order. Removing C3 or C4 would make `prompt.find("## How to apply…
   ")` or `prompt.find("## Auto-captured facts…")` return `None` → `.expect("heading exists")`
   would panic. Mis-ordering C3 before C4 would trip the third `<` assertion. ✓

The test sets up a workspace and calls `build_workspace_agent_memory_prompt` without
injecting facts, so `facts_section` is empty and the rendered prompt equals `base_prompt`.
The headings all live in `base_prompt`, so find-then-compare is sound.

---

## Constraints (verbatim from the review brief)

- `auto_memory.rs` is **675 lines** (verified by `Get-Content -Encoding UTF8 … .Count` ≈ 675).
  Under 800 ✓.
- `cargo fmt` not run (orchestrator-reported; no diff churn outside the additions).
- English-only, no emoji in added text/code/comments ✓.
- `cargo check -p northhing-core --features product-full` reports **19 warnings** (unchanged
  baseline). The one warning-suppression allowlist in `northhing-core/lib.rs` means
  unchanged-count does not prove absence of dead code; orchestrator's blanket verification
  accepted.
- One file changed; nothing under `.superpowers/` committed. ✓

---

## Findings

### None Critical
### None Important
### Minor

**Minor #1 — `task-t5c-report.md` lacks the §6.1 rendered-prompt paste**

Brief §6.1 explicitly mandates: *"paste, in the report, the **rendered** prompt region: run a
test (or a `#[ignore]`d helper) that prints the rendered prompt from `## When to access
memories` through `## Memory and other forms of persistence`, and paste that block verbatim."*

The implementer's report verifies the format string at line 304 and demonstrates test
non-vacuity, but does not include the rendered-prompt paste. I have independently read the
four additions out of the source blob and confirmed byte-for-byte fidelity against brief §3,
so the *evidence* exists; it just isn't in the report. This is a process gap, not a code
defect, and the brief §6.1 risk it was meant to mitigate (silent brace-escape mangling of the
prompt) is structurally impossible here since (a) the additions introduce zero `{`/`}`, and
(b) the production format string is untouched. A future task that adds prompts with literal
braces would benefit from the report-level evidence.

If a future iteration of the report wants the artifact, a one-liner like

```rust
let p = build_workspace_agent_memory_prompt(&workspace).await.unwrap();
let i = p.find("## When to access memories").unwrap();
let j = p.find("## Memory and other forms of persistence").unwrap();
println!("{}", &p[i..=j+38]);
```

prints the section verbatim for inclusion in the report. Optional, not blocking.

---

## Cannot verify from diff (none)

Every load-bearing claim in this review — verbatim text, ASCII hyphen, backticks, brace
counts, byte offsets of pre-existing interpolations, section ordering, test non-vacuity
targeting, file length, single-file footprint, clean worktree — was verified by opening
paths and inspecting bytes (Python `str.find` + `bytes.find` on `git show HEAD:…`) rather
than inferred from the diff. The orchestrator-provided givens (test counts, warning count,
boundary check exit, line count, file scope) were not re-run per the review brief's
instruction. No residual unknowns.

---

## Verdicts

SPEC: PASS
QUALITY: PASS
