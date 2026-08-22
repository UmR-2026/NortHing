# Task T5c — auto_memory guidance: four additions (research report §5.4-C)

Base commit: `9a9fb8a` (branch `feat/growth-core-0804`, worktree
`E:\agent-project\northing\.worktrees\growth-core-0804`)

## 1. Scope

Add **four** pieces of guidance to the agent-facing memory prompt in
`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`, plus one test assertion
per addition.

This is a **prompt-text transcription task**. The exact English wording is given verbatim in
§3 — copy it character for character. Do not improve, shorten, rephrase, or translate it.

**Out of scope** (do not touch):
- Any Rust logic. The only code you add is test assertions.
- The `# Remembered facts` retrieval/selection logic (`auto_memory.rs:243-289`),
  `build_query_aware_facts_reminder` (`:291`), `build_workspace_memory_files_context` (`:334`).
- Any existing prompt sentence. **Every line of the current prompt must survive byte-for-byte.**
  This task is purely additive.
- The distiller prompt (`distiller.rs`) — that was §5.4-A and is already done.

## 2. Verified facts about the file (checked at `9a9fb8a`)

- `auto_memory.rs` is **595 lines**. After your change it must stay under 800.
- The prompt lives in one `format!` **raw string** inside
  `build_workspace_agent_memory_prompt` (`:105`): the raw string opens at `:112`
  (`r#"# auto memory`) and closes at `:240` (`"#`).
- Current section order inside that raw string:

  | Line | Heading |
  |---|---|
  | `:112` | `# auto memory` |
  | `:120` | `## Types of memory` |
  | `:184` | `## What NOT to save in memory` |
  | `:194` | `## How to save memories` |
  | `:218` | `## When to access memories` |
  | `:224` | `## Before recommending from memory` |
  | `:236` | `## Memory and other forms of persistence` |

- ⚠️ **Brace escaping trap**: this is a `format!` raw string, so `{` and `}` are format
  syntax. `{memory_dir_display}` at `:114` is a real interpolation. The frontmatter template
  at `:202-207` is written `{{{{memory name}}}}` (four braces) so that it renders as
  `{{memory name}}`. **Do not touch those.** The four texts in §3 contain no braces, so you
  should not need any escaping — but if you ever add a literal brace, it must be doubled.
- Tests are two inline modules: `mod tests` (`:405`) and `mod query_aware_tests` (`:523`).
  Put your new assertions in `mod tests`, following the existing
  `prompt_injection_*` tests (`:429-521`) for how a workspace is set up.

## 3. The four additions — verbatim text

### C1 — read-decision boundary (hard-skip list)

Append to the **end of the `## When to access memories` section**, i.e. after the current
last bullet of that section (`:222`, the "Memory records can become stale over time..."
bullet) and before the `## Before recommending from memory` heading:

```text
Some requests cannot benefit from memory at all. Skip the memory lookup entirely when the message is:
- a question about the current time or date
- a simple translation or a word definition
- a single-line command or a one-off shell invocation
- a pure formatting, renaming, or syntax fix
Answer these directly. Reaching for memory here spends turns and changes nothing.
```

### C2 — read budget

Immediately after the C1 block (same section, C1 first then C2):

```text
Keep the lookup bounded: at most 4-6 memory reads or searches before you move on to the actual work. If what you need has not surfaced by then, proceed with what you have and say plainly what you could not find. An open-ended memory hunt is worse than a partial answer delivered now.
```

### C3 — division of labour with auto-captured facts

Add as a **new section** placed immediately **before** the
`## Memory and other forms of persistence` heading (`:236`). Heading and body:

```text
## Auto-captured facts vs. your memory files

The `# Remembered facts` block is a query-relevant subset maintained by the
system. Treat it as already known and do not copy its content into your own
memory files. You are not expected to deduplicate against facts you cannot
see - the system owns that.
```

The body is a **binding, already-adjudicated wording (decision D14)** — reproduce it exactly,
including the hyphen in `see - the system owns that.` (not an em dash) and the existing line
breaks. Rationale you must not undo: an earlier draft told the agent "when content overlaps,
the file layer wins; do not create a file for a fact that already exists". That instruction is
**impossible to execute** — the injected `# Remembered facts` block is only the top-k subset
relevant to the current query, never the whole store, so the agent cannot know whether a fact
already exists. Unexecutable instructions are worse than none: the model fake-complies.

### C4 — application discipline

Add as a **new section** immediately **after** the end of the
`## Before recommending from memory` section (i.e. after `:234`, the "A memory that
summarizes repo state..." paragraph) and before the C3 section you added:

```text
## How to apply memory in your answer

Use memory only when it changes the substance of your answer - a different recommendation, a different default, or a caveat you would otherwise have missed. Do not narrate the retrieval: no "I remember that...", "according to my memory...", or "as I noted earlier". Let the recalled context shape the answer silently.
```

### Resulting section order (verify this at the end)

```
## When to access memories        <- C1 then C2 appended at its end
## Before recommending from memory
## How to apply memory in your answer          <- C4 (new)
## Auto-captured facts vs. your memory files   <- C3 (new)
## Memory and other forms of persistence
```

## 4. Tests

Add **four** assertions to `mod tests` (`:405`). One per addition, each keyed on a distinctive
substring so a future reword cannot silently delete the guidance:

- C1: `"a single-line command or a one-off shell invocation"`
- C2: `"at most 4-6 memory reads or searches"`
- C3: `"You are not expected to deduplicate against facts you cannot"`
- C4: `"Do not narrate the retrieval"`

You may put all four in one new `#[tokio::test]` (e.g. asserting on the output of
`build_workspace_agent_memory_prompt`) or in four separate tests — your call, but each
assertion needs a message saying which guidance item is missing. Follow the existing
`prompt_injection_*` tests for workspace setup.

Also assert the section order from §3 holds, by comparing the byte offsets of the four
headings (`## Before recommending from memory`, `## How to apply memory in your answer`,
`## Auto-captured facts vs. your memory files`, `## Memory and other forms of persistence`)
in the rendered prompt. A misplaced section is the most likely silent error in this task.

## 5. Constraints

- **Purely additive to the prompt.** No existing sentence may be edited, moved, or deleted.
- Do not run `cargo fmt`.
- English-only, no emoji, in prompt text, code, and comments.
- Do not add or reword any production log string.
- `auto_memory.rs` must stay under 800 lines.
- No new files. No changes outside `auto_memory.rs`.

## 6. Verification (paste **complete raw stdout+stderr**, not excerpts)

Prefix for cargo: `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo test -p northhing-core --features product-full auto_memory` — 7 now, must be 7 + your new tests
2. `cargo test -p northhing-core --features product-full prompt_injection` — **4** now, must stay 4 and pass
3. `cargo check -p northhing-core --features product-full` — warning baseline **19**, must not increase
4. `node scripts/check-core-boundaries.mjs` — exit 0
5. Line count of `auto_memory.rs` via `(Get-Content -LiteralPath <path> -Encoding UTF8).Count`

### 6.1 Prove the additions survived escaping (mandatory evidence)

`format!` brace mistakes and raw-string mishaps do not always fail the build — they can
mangle the rendered text. So paste, in the report, the **rendered** prompt region: run a test
(or a `#[ignore]`d helper) that prints the rendered prompt from
`## When to access memories` through `## Memory and other forms of persistence`, and paste
that block verbatim. The reviewer must be able to read the four additions as the agent will
actually see them, with the section order visible.

Also confirm explicitly: no `{` or `}` was added to the raw string, and `:114`'s
`{memory_dir_display}` plus `:202-207`'s `{{{{...}}}}` are untouched.

## 7. Deliverables

- **One commit on `feat/growth-core-0804`** (you MUST commit; leaving work uncommitted is a
  failed task). Message prefixed `feat(memory): `.
- **All source edits must land in the worktree** `E:\agent-project\northing\.worktrees\growth-core-0804`,
  never in the main checkout `E:\agent-project\northing`.
- `git status --short` clean before you finish. Do not commit anything under `.superpowers/`.
- Report to **`E:\agent-project\northing\.superpowers\sdd\task-t5c-report.md`** (main repo
  path, NOT the worktree's `.superpowers/`): what you inserted where with `file:line`, the
  §6.1 rendered block, full verification output, line count, and anything ambiguous.
- End with a status line: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, or `BLOCKED`.
