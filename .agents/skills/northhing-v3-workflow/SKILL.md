---
name: northhing-v3-workflow
description: "Use when working on northhing v3 prompt loading or related Agent architecture changes. Encodes the v3 pattern (const flag + regression test + commit + PROJECT_STATE update) and the worktree workflow. Trigger this when picking up northhing v3 work, planning a new v3 phase, or executing any of the 5 follow-up tasks (Mode prompt ç²¾ç®, Tool manifest éæ, CompressAgent, dead code æ¸ç, GUI build fix)."
---

# northhing v3 Worktree Workflow

This skill encodes the workflow used during northhing v3 prompt loading refactor (v3-restructure branch, 16 commits, ~6,500-9,500 tokens/turn saved). Use it for any follow-up v3 work or related Agent architecture changes.

## When to trigger

- Picking up northhing v3 work (continue from `v3-restructure` branch)
- Planning a new v3 phase (v3.5, v3.6, etc.)
- Executing any of the 5 follow-up tasks listed in `HANDOFF.md` and `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md`:
  - Mode prompt ç²¾ç® (team_mode, deep_research, deep_review, cowork)
  - Tool manifest éæ (24 expanded â?5 core + 19 advanced)
  - å®æ½ CompressAgent / LoopEngineerAgent (P1-9)
  - 16 CLI dead_code warnings æ¸ç (P2-4)
  - GUI mobile-web/dist èµæºé®é¢ä¿®å¤
- Reviewing or extending v3 changes

## Quick reference

**Worktree**: `E:\agent-project\northhing-v3` (branch `v3-restructure`)

**Rust toolchain (CRITICAL)**:
- `rustup MSVC 1.96.0` (`stable-x86_64-pc-windows-msvc`)
- `C:\Users\UmR\.cargo\bin` MUST be first in PATH (rustup shim)
- If `C:\Program Files\Rust stable GNU 1.95\bin` is first, `cargo build` will fail with `dlltool.exe not found`
- Workaround at the start of every shell:
  ```bash
  set "PATH=C:\Users\UmR\.cargo\bin;%PATH%"
  cargo build -p northhing-core
  ```

**Build commands**:
```bash
# Single crates
cargo build -p northhing-core
cargo build -p northhing-agent-runtime
cargo build -p northhing-cli

# Tests
cargo test -p northhing-core --lib     # 821 tests, must pass
cargo test -p northhing-agent-runtime  # multiple suites, must pass

# DO NOT build northhing-desktop (mobile-web/dist missing â?Tauri build script error)
```

## The v3 change pattern (EVERY change)

Every v3 change follows this exact pattern. The pattern is **mandatory** for follow-up work because it provides instant rollback:

### 1. Add a const flag at the top of the file
```rust
/// v3.x <Change letter>: <one-line description>
/// <Longer explanation of what it does and the savings>
/// Rollback: <what to change in the flag>.
const USE_X_FEATURE: bool = true;  // v3.x
```

### 2. Wrap the change in an `if` branch
```rust
if USE_X_FEATURE {
    // v3.x path (e.g. short pointer)
    return X_POINTER.to_string();
} else {
    // Original code (preserved for rollback)
    return X_ORIGINAL.to_string();
}
```

### 3. Add a regression test (in the test module at bottom of file)
- If the file has no test module, create one (`#[cfg(test)] mod tests { ... }`)
- Test the new behavior
- If the change affects an existing test, update that test to expect the new behavior
- This is critical â?every commit in v3 had a test (or test fix-up in the same commit)

### 4. Build + test + commit
```bash
cargo build -p <crate>           # must compile
cargo test -p <crate> --lib       # must pass
git add <files>
git commit -m "<type>(<scope>): v3.x <letter> <description>

<bullet list of what changed and why>
<rollback instructions>
<verification results>"
```

Commit message format: `<type>(<scope>): v3.x <letter> <one-liner>`

### 5. Update PROJECT_STATE.md
Add a "v3.x complete" section with the commit hash, files changed, and tokens saved.

## Branch rules

- All v3 work happens on `v3-restructure` branch in worktree `E:\agent-project\northhing-v3`
- The main repo `E:\agent-project\northhing` is on `main` branch (clean, no remote)
- **DO NOT push** â?no remote configured
- **DO NOT modify main directly** â?all work in v3-restructure
- For follow-up tasks (5 candidates in HANDOFF.md), create a sub-branch off v3-restructure if you want isolation:
  ```bash
  cd E:/agent-project/northhing-v3
  git worktree add -b v3.5-mode-prompts ..\northhing-v3-mode-prompts v3-restructure
  ```
  See `using-git-worktrees` skill for details.

## One-line rollback (v3 changes)

```bash
# v3.0 (C + B)
# prompt.rs:  const DISABLE_COLLAPSED_TOOL_LISTING_REMINDER: bool = false;
# task_tool.rs: const DROP_AGENT_DEFAULT_TOOLS_IN_LISTING: bool = false;

# v3.1 (E)
# skill_agent_snapshot.rs: const COLLAPSE_GSTACK_SKILLS_IN_LISTING: bool = false;

# v3.2 (A)
# auto_memory.rs: const USE_MEMORY_SKILL_POINTER: bool = false;

# v3.3 (D)
# agents.rs: const INCLUDE_PROJECT_LAYOUT_BY_DEFAULT: bool = true;
```

## What to read first

When picking up work, read in this order:

1. `HANDOFF.md` (in worktree root) â?5 min, full picture
2. `docs/PROJECT_STATE.md` â?current state + follow-up tasks
3. `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md` â?design intent
4. `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md` â?task breakdown + post-v3 candidates
5. `docs/CODE_REVIEW.md` â?original review + status updates

**DO NOT read** the DEPRECATED v1 docs (5 files, all marked DEPRECATED in their headers):
- `docs/PROMPT_LOADER_ARCHITECTURE.md`
- `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design.md` (without -v2)
- `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl.md` (without -v2)

## Common pitfalls

1. **MinGW vs MSVC**: Always verify `cargo --version` shows 1.96.x MSVC, not 1.95 GNU
2. **northhing-desktop build**: Will fail with `mobile-web/dist doesn't exist`. Don't try to build it.
3. **northhing-memory crate**: Does NOT exist. All references in original docs are stale.
4. **Test assertion updates**: When changing prompt.rs / task_tool.rs, also update the existing test that asserts on the old behavior (e.g. `tool_listing_sections_render_only_present_sections`).
5. **Catalog additions**: Adding a new skill? You need to:
   - Add the file under `builtin_skills/`
   - Add `BuiltinSkillId` variant in `catalog.rs`
   - Add `BuiltinSkillSpec` to `BUILTIN_SKILL_SPECS` array
   - (The directory is auto-discovered via `include_dir!`, but the spec must be registered)

## For new v3 phases

The 5 follow-up candidates in `HANDOFF.md` and plan v2 can each be a new phase following the same pattern:

1. Create a new sub-branch: `git worktree add -b v3.5-<name> ..\northhing-v3-v35-<name> v3-restructure`
2. Plan in `docs/superpowers/specs/YYYY-MM-DD-v35-<name>.md`
3. Implement following the const-flag pattern
4. Update `PROJECT_STATE.md` with new phase
5. (Optional) Merge back to v3-restructure when stable

## v3 Enhancement: Context Engineering & Incremental Implementation

> æ¥æºï¼addyosmani/agent-skills context-engineering + incremental-implementation

### Prompt Loading ä¼åçäºå±ä¸ä¸ææ¨¡å

v3 çæ ¸å¿ç®æ æ¯èç tokens/turnãä»¥ä¸æ¯ context-engineering çäºå±ä¸ä¸ææ¨¡åï¼æå¯?prompt loading ç­ç¥ï¼?
| å±çº§ | åå®¹ | v3 å¯¹åº |
|------|------|---------|
| Rules Files | é¡¹ç®çº¦å®ãç¼ç è§è?| AGENTS.md, northhing-v3-workflow skill |
| Spec/Architecture | å½åä»»å¡çè®¾è®¡ææ¡?| PROJECT_STATE.md, HANDOFF.md |
| Relevant Source | å½åæ¹å¨çæºæä»¶ | åªå è½½å½å?const flag å½±åçæ¨¡å?|
| Error Output | ç¼è¯/æµè¯è¾åº | cargo build/test stderr |
| Conversation History | å½åå¯¹è¯ä¸ä¸æ?| ä¿æç²¾ç®ï¼é¿å?context flooding |

### Selective Include æ¨¡å¼ï¼v3 prompt loading æ ¸å¿ç­ç¥ï¼?
æ¯ä¸ª turn åªå è½½å½åä»»å¡éè¦ç prompt çæ®µï¼?
```
TASK: åæ¢ module X ç?prompt loading å?const flag æ¨¡å¼
RELEVANT FILES: module X æºæä»?+ æµè¯ + ç¸å³ const å®ä¹
PATTERN TO FOLLOW: å·²å®æéæç module Yï¼åèå¶ const flag å®ç°ï¼?CONSTRAINT: const flag é»è®¤ falseï¼ä¿æååå¼å®?```

### Incremental Implementation åå

v3 ç?const flag æ¨¡å¼æ?feature flag ç?Rust ç¼è¯æçæ¬ï¼

| ç»´åº¦ | éç¨ feature flag | v3 const flag |
|------|-------------------|---------------|
| å®ç° | `process.env.FEATURE_X` | `const USE_X: bool = false;` |
| å¼é | è¿è¡æ¶æ£æ?| **é?*ï¼ç¼è¯æå¸¸éä¼ æ­ï¼?|
| åæ¢ | ä¸ééæ°ç¼è¯ | éè¦éæ°ç¼è¯?|
| å®å¨æ?| è¿è¡æ¶å¯å?| ç¼è¯æä¸å¯å |

**Rule 0: Simplicity First** â?æç®åçå®ç°å¾å¾ä¹æç?tokenãä¸è¦è¿åº¦è®¾è®?prompt loading é»è¾ã?
**Rule 0.5: Scope Discipline** â?éææ¶åç°çå¶ä»é®é¢è®°ä¸º "NOTICED BUT NOT TOUCHING"ï¼ä¸æ©å¤§èå´ã?
**Keep It Compilable** â?Rust æ¯?TS æ´ä¸¥æ ¼ï¼æ¯ä¸ªä¸­é´ç¶æé½å¿é¡»è?`cargo check` éè¿ã?
## Related skills in this project

The skills are bundled in `.agents/skills/`:
- `brainstorming/` â?for design discussions
- `writing-plans/` â?for creating implementation plans
- `subagent-driven-development/` â?for executing plans via subagents
- `using-git-worktrees/` â?for the worktree workflow
- `test-driven-development/` â?for TDD
- `verification-before-completion/` â?for pre-commit verification
- `using-superpowers/` â?the entry point

These are the same skills the v3 work used. Use them.

## TL;DR for parallel workers

- Worktree: `E:\agent-project\northhing-v3` (v3-restructure)
- Rust: rustup MSVC 1.96 + `set "PATH=C:\Users\UmR\.cargo\bin;%PATH%"` first
- Pattern: `const FLAG` + if/else + regression test + commit + PROJECT_STATE update
- Tests: 821+ must pass
- Don't: northhing-desktop build, v1 docs, push
- For details: read `HANDOFF.md`
