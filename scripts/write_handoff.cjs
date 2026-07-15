// write_handoff.cjs — write HANDOFF_NEXT_SESSION.md to avoid GBK mojibake
const fs = require('fs');
const path = require('path');
const content = `# Handoff to Next Session (2026-06-19)

> **For next agent:** Read this first. Context below is everything
> you need to know without reading the long HANDOFF.md or PROJECT_STATE.md.

## TL;DR

Two sessions ago we shipped A0-A8 of the v3 restructure. Last session
we established a **4-domain code reference library** at
\`.agents/reference/\` plus a ZCode skill \`reference-library\` that
auto-loads on any 4-domain task. **The next session's job is to
execute the plan at \`docs/plans/2026-06-19-post-reference-roadmap.md\`.**

| Aspect | Value |
|---|---|
| **Branch** | \`v3-restructure\` (worktree at \`E:\\agent-project\\northhing\`) |
| **HEAD** | \`4db07de\` (clean working tree) |
| **Total commits on branch** | 26 |
| **Tag** | \`v0.1.0\` (at commit \`2813b36\`, A6 commit) |
| **Reference library files** | 46 (skills/actor/session/checker + _upstream + README) |
| **Skill matchability test** | 12/12 PASS (run via \`node scripts/test_reference_skill.cjs\`) |
| **Plan for next session** | \`docs/plans/2026-06-19-post-reference-roadmap.md\` (Phase A -> B -> C) |

## Three things the next session MUST do

1. **Read the plan first.**
   \`docs/plans/2026-06-19-post-reference-roadmap.md\` is the executable
   playbook. It has 3 phases (A: 0.5d, B: 1-2d, C: 1-2d) with checkboxes.

2. **Run \`preflight-skill-check\` BEFORE writing code.** This is the
   auto-loaded meta-skill. Once it loads, it will surface
   \`reference-library\` and the 4-step workflow (README -> SIGNATURES ->
   NOTES -> mirror).

3. **Follow the 4-step workflow inside the skill.** For every task:
   - Open \`.agents/reference/<domain>/README.md\`
   - Open \`.agents/reference/<domain>/SIGNATURES.md\`
   - Open \`.agents/reference/<domain>/NOTES.md\` (read warnings)
   - Open the specific \`NN-*.rs\` and copy the pattern with header:
     \`// Pattern source: .agents/reference/<domain>/0N-xxx.rs\`

## What the plan does

### Phase A - Close the A6 GUI wiring gap (0.5 day)
- 4 unwired Slint callbacks: \`toggle-skill\`, \`load-more-messages\`, \`refresh-sessions\`, \`refresh-messages\`.
- \`SESSION_TREE_VIEW = true\` is on, but sidebar is flat (real implementation is Phase C).
- 5 tasks (A.1-A.5). One commit at the end.

### Phase B - Track B Phase 1: Lightweight Actor Skeleton (1-2 days)
- 5 tasks (B.1-B.8) implementing impl-plan Tasks 1.1-1.5.
- Creates \`crates/agent-dispatch/\` with const flags all defaulting to \`false\`.
- Reference: \`.agents/reference/actor/07-impl-plan-task-map.md\` (17-task map).

### Phase C - Subagent tree sidebar + Inspector live data (1-2 days)
- 7 tasks (C.1-C.7).
- Real \`parent_session_id\` on \`SessionSummary\`.
- Real \`model-status\` in Inspector (3 A5 providers).
- Depends on Phase B for real subagent sessions to draw.

## Critical context for next session

### Project state
- **Working tree:** clean. 26 commits, HEAD = \`4db07de\`.
- **Branch:** \`v3-restructure\`. Don't merge to main yet - that's a separate plan.
- **Toolchain caveat (unchanged from previous sessions):**
  \`set PATH=C:\\Users\\UmR\\.cargo\\bin;C:\\Users\\UmR\\.rustup\\toolchains\\stable-x86_64-pc-windows-msvc\\bin;%PATH%\` before any cargo command.
  GNU toolchain ahead of MSVC breaks \`getrandom\`/\`aws-lc-rs\` with \`dlltool.exe not found\`.
- **\`.gitignore\` has \`nul\`** (Windows phantom file - PowerShell redirection side effect).

### Reference library
- 46 files in \`.agents/reference/\`. 4 domains x 3 (README/SIGNATURES/NOTES) + mirrors + upstream.
- Skill at \`.agents/skills/reference-library/SKILL.md\`.
- Mirror script: \`scripts/copy_reference.cjs\` (re-run after \`src/\` changes).
- Matchability test: \`scripts/test_reference_skill.cjs\` (12/12 PASS).

### Skill system
- Project-local skills: \`.agents/skills/<name>/SKILL.md\` (28 skills, includes the new \`reference-library\`).
- User-global: \`C:\\Users\\UmR\\.agents\\skills/preflight-skill-check/\`.
- All skills use frontmatter \`name\` + \`description\`. The description is what \`preflight-skill-check\` matches on.

### ZCode CLI limitation (carried forward)
- \`Agent\` tool only supports \`Explore\` subagent_type (read-only search).
- No implementer / spec-reviewer / code-quality-reviewer subagent.
- **Practical workaround:** use \`superpowers:executing-plans\` (inline) or do implementation in main session with manual review.

### Key design decisions (locked in)
1. GUI: Slint + Material (not Tauri+React).
2. CLI: hidden \`northhing internal ...\` with token gate.
3. Const-flag pattern: every behavior change ships behind a \`const FLAG: bool\`.
4. LLM providers: 3 (Anthropic, Gemini, OpenAI-compatible) - module-based, NO \`ModelClient\` trait (HANDOFF wording was aspirational; see \`_upstream/northhing-a5-providers.md\`).
5. Lightweight actor: 4 const flags, all default \`false\`. See \`_upstream/tokio-actor-pattern.md\` for tokio mpsc actor pattern.

### User preferences (carried forward)
- Const-flag pattern for any behavior change (one-line rollback).
- TDD red->green->commit per step.
- Verify against actual code, don't trust review line numbers.
- PowerShell conventions (use \`Get-ChildItem\`, not \`ls\`).
- Windows-first; cross-platform later.
- Aesthetic priority: modern Material > native.

## What NOT to do

- **Do NOT skip the 4-step reference workflow.** It's enforced by the skill.
- **Do NOT touch \`coordinator.rs:4172-5025\`** to "extend" the existing subagent path. The actor design replaces it.
- **Do NOT touch \`crates/assembly/core/src/agentic/execution/execution_engine.rs\`**.
- **Do NOT enable \`USE_LIGHTWEIGHT_ACTOR = true\`** without integration tests first.
- **Do NOT enable \`USE_ONESHOT_DISPATCHER = true\`** without integration tests first.
- **Do NOT extend \`ToolDispatcher\` to support multi-round loops.** Multi-round goes through \`ConversationCoordinator::execute_hidden_subagent_internal\` (coordinator.rs:4173).
- **Do NOT duplicate \`SessionState\` / \`ProcessingPhase\` definitions.** They live at \`core/state.rs:13-34\` only.
- **Do NOT add a 6th \`start_dialog_turn_*\` facade.** Use one parameterized entry instead.
- **Do NOT modify \`Cargo.toml [workspace.dependencies]\`** in Phase B.2 - only edit \`[workspace.members]\`.

## Files most likely to need attention next session

- \`src/apps/desktop/src/app_state.rs\` - add 4 callback wirings (Phase A.1-A.3).
- \`src/apps/desktop/src/ui/main.slint\` - already declares 9 callbacks; nothing to add.
- \`src/apps/desktop/src/ui/views/SidebarView.slint\` - Phase C.2 nested rendering.
- \`src/apps/desktop/src/ui/views/InspectorView.slint\` - Phase C.3/C.4/C.5 live data.
- \`crates/agent-dispatch/\` - new crate (Phase B.2-B.5).
- \`Cargo.toml\` - workspace member add (Phase B.2).
- \`HANDOFF.md\` - bump total commits at end of each phase.
- \`docs/PROJECT_STATE.md\` - track status updates (some GBK mojibake in this file is unrelated).

## Risk log carried forward

| Risk | Mitigation |
|---|---|
| PATH order: GNU before MSVC -> dlltool.exe not found | Always prepend MSVC path |
| PowerShell \`-Command\` with \`$variable\` in single quotes -> parser error | Use script file or \`-File\` instead of inline |
| Windows phantom \`nul\` file blocks \`git add -A\` | Already in \`.gitignore\` |
| AskUserQuestion schema: max 4 questions, max 4 options | Split into multiple calls |
| ZCode \`Agent\` tool lacks implementer subagent | Use inline execution |
| Const flag flips must not break existing tests | Each phase ends with \`bash scripts/regression-test-desktop.sh\` |
| Reference library drifts from \`src/\` | Run \`node scripts/copy_reference.cjs\` after any \`src/\` change |
| Skill description becomes stale | Run \`node scripts/test_reference_skill.cjs\` after editing the description |

## End-of-session snapshot

\`\`\`
$ git log --oneline -5
4db07de docs(reference): enrich skill description + impl-plan task map + A5 actual shape + 12/12 match test
442028e docs(reference): establish .agents/reference/ + reference-library skill (4 domains)
a6e1df8 docs: fix GBK mojibake in Chinese docs and add CODE_REVIEW.md
9d8edac docs: regenerate clean HANDOFF.md (fix GBK mojibake corruption)
2813b36 A6: Multi-session UI - Slint sidebar session management wired to core

$ git status
On branch v3-restructure
nothing to commit, working tree clean

$ node scripts/test_reference_skill.cjs
12/12 prompts would trigger the skill.

$ find .agents/reference -type f | wc -l
46
\`\`\`

## Quick start for next session

\`\`\`bash
# 1. Read the plan
cat docs/plans/2026-06-19-post-reference-roadmap.md

# 2. Confirm the test still passes
node scripts/test_reference_skill.cjs

# 3. Skim the reference library
ls .agents/reference/
ls .agents/reference/skills/ .agents/reference/actor/ .agents/reference/session/ .agents/reference/checker/

# 4. Start Phase A.1: wire toggle-skill callback
#    - Read .agents/reference/skills/SIGNATURES.md first
#    - Edit src/apps/desktop/src/app_state.rs
#    - cargo build -p northhing-desktop
#    - Commit
\`\`\`

Ready for next session.
`;
const dst = path.join(__dirname, '..', 'docs', 'HANDOFF_NEXT_SESSION.md');
fs.writeFileSync(dst, content, 'utf8');
console.log('wrote', dst, fs.statSync(dst).size, 'bytes');
