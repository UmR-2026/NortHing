# Review — Task T5c (auto_memory guidance: four additions, research report §5.4-C)

You are reviewing ONE commit on branch `feat/growth-core-0804`.

- Worktree (read source from here): `E:\agent-project\northing\.worktrees\growth-core-0804`
- BASE: `9a9fb8a`  HEAD: `2e986ce`
- Diff: `E:\agent-project\northing\.superpowers\sdd\task-t5c-diff.md`
- Spec (the ONLY source of requirements): `E:\agent-project\northing\.superpowers\sdd\task-t5c-brief.md`
- Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t5c-report.md`

Read the brief in full before the diff. All changes are in one file:
`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`.

## Two verdicts required (both must be explicit)

1. **Spec compliance** — does the commit do exactly what the brief §3/§4 requires?
2. **Code quality** — is it correct, honest, and maintainable?

## What this task is

It appends four guidance sections to the agent-facing memory prompt (a `format!` raw string)
and adds tests. It changes no production logic. One binding constraint: the prompt additions
are **verbatim text** given in brief §3 — the implementer's job was transcription, not
authorship.

## Constraints copied verbatim from the plan's Global Constraints

- Rust production files must stay under 800 lines (`auto_memory.rs` is 675 after this change).
- `cargo fmt` is forbidden in this project.
- English-only, no emoji, in code, comments, and prompt text.
- Core tests must run with `--features product-full`.
- Warning policy is warn-only; the core warning baseline is **19** and must not increase.
- `northhing-core`'s `lib.rs:3-4` carries `#![allow(dead_code)]` / `#![allow(unused_imports)]`,
  so an unchanged warning count does NOT prove absence of dead code.

## Highest-value things to check (in priority order)

1. **Verbatim fidelity of the four texts.** Compare each inserted block character by character
   against brief §3. The C3 block is decision **D14** — an adjudicated wording. Check
   specifically: the ASCII hyphen in `see - the system owns that.` (must NOT be an em dash or
   en dash), its exact internal line breaks, and the backticks around `` `# Remembered facts` ``.
   Any silent "improvement", reflow, or Americanized punctuation is a finding.
2. **Section placement and order.** Brief §3 mandates:
   `## When to access memories` (with C1 then C2 appended at its end) → `## Before recommending
   from memory` → `## How to apply memory in your answer` (C4) → `## Auto-captured facts vs.
   your memory files` (C3) → `## Memory and other forms of persistence`. Verify against the
   real file, not just the diff hunks.
3. **Purely additive?** Every pre-existing prompt sentence must survive byte-for-byte. The
   commit shows exactly two deleted lines, both test assertions (see item 4). Confirm no
   prompt prose was edited, reflowed, moved, or dropped.
4. **The two tightened assertions — are they right and non-vacuous?** Context: the C3 prose
   legitimately contains the literal `` `# Remembered facts` ``, which broke two pre-existing
   assertions of the form `!prompt.contains("# Remembered facts")`. They are now
   `!prompt.contains("\n\n# Remembered facts\n\n")`, keyed to the production injection format
   string. Verify by reading the production code that the format string really is
   `format!("\n\n# Remembered facts\n\n{}", items)`, and reason about whether the C3 prose can
   trip the tightened form (it should not). Judge whether these two tests still test what
   their names claim.
   Note for context: a first attempt used `!prompt.contains("- I prefer pnpm")`, which was
   **vacuous** (neither test writes that fact) and was rejected by the orchestrator. If you
   see any remaining assertion that cannot fail, that is Important.
5. **Brace/escaping integrity.** This is a `format!` raw string. Confirm no literal `{` or `}`
   was introduced by the additions, and that the pre-existing `{memory_dir_display}`
   interpolation and the `{{{{...}}}}` frontmatter template are untouched.
6. **The new test's value.** `prompt_includes_all_four_memory_guidance_additions` asserts four
   content substrings plus four ordering relations. Judge whether the chosen substrings are
   distinctive enough to catch a future silent deletion or reword, and whether the ordering
   assertions actually pin the mandated order.

## Already independently verified by the orchestrator — do NOT re-run these

- `cargo test ... auto_memory` = **8**, `prompt_injection` = **4**, `system_prompt` = 21,
  `dream` = 7, `self_cognition` = 19. Core warnings **19**. `check-core-boundaries` exit 0.
- Commit touches exactly one file, 82 insertions / 2 deletions, worktree clean, main checkout
  unpolluted, no temporary proof scaffolding left behind, `auto_memory.rs` = 675 lines.
- The two deleted lines are exactly the two old assertion lines; their comments and assertion
  messages were restored to the original intent.

Review by reading. Do not re-run the suite; the report plus the above is your evidence base.

## Output format

For each finding: **Critical / Important / Minor**, the `file:line`, what is wrong, why it
matters, and the smallest correct fix. If you cannot determine something from the diff alone,
put it under a separate heading **"Cannot verify from diff"** rather than guessing — and do not
assert that a file, symbol, or line does not exist unless you actually opened the path and
confirmed.

⚠️ This task is prompt text. Claims about punctuation, whitespace, or substring matching must
be checked against the actual bytes (open the file / test the substring), not inferred. Do not
report a character-level difference you have not literally compared.

End with the two verdicts, each as `PASS` or `FAIL`, on their own lines:
`SPEC: PASS|FAIL`
`QUALITY: PASS|FAIL`
