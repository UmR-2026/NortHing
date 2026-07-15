<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
 Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
 本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# v3 Prompt Loader Implementation Plan (v2 — Realistic)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce agent-app main agent prompt tokens by **7,000-11,000 tokens** (conservative estimate) through 6 concrete code changes (A-F) against the actual v3 codebase. Target: post-v3 prompt ~16-25K tokens (down from unmeasured ~25-35K).

**Architecture:** 6 surgical changes to specific files. No new crates, no new tools, no DBs. Each change is independently shippable and can be rolled back via `git revert`.

**Tech Stack:** Rust (workspace), existing prompt builder, existing tool manifest, no new dependencies.

**Spec:** `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md`
**Replaces:** `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl.md` (v1, deprecated — based on deleted `agent-app-memory`)

**Working directory:** `E:\agent-project\agent-app-v3` (git worktree on `v3-restructure` branch)

---

## File Structure (changes only)

| Phase | Files modified | Files created |
|---|---|---|
| v3.0 | `src/crates/execution/agent-runtime/src/prompt.rs`, `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | — |
| v3.1 | `src/crates/assembly/core/src/agentic/tools/implementations/skill_tool.rs`, `src/crates/assembly/core/builtin_skills/_gstack/` (new parent skill) | `_gstack/SKILL.md` |
| v3.2 | `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`, `src/crates/assembly/core/src/agentic/agents/prompts/agentic_mode.md` (placeholder reduction), `src/crates/assembly/core/builtin_skills/memory/SKILL.md` (new) | `memory/SKILL.md` |
| v3.3 | `src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs`, `src/crates/execution/agent-runtime/src/agents.rs` | — |

**No new crates.** **No new tools.** **No DBs.** All changes are surgical edits to existing files.

---

## Phase v3.0: Quick Wins (0.5 day, ~1,500-2,000 tokens)

**Goal:** Two trivial/low-risk drops that can ship in a single day.

### Task 1: Change C — Drop Collapsed Tool Listing reminder

**Files:**
- Modify: `E:\agent-project\agent-app-v3\src\crates\execution\agent-runtime\src\prompt.rs:8-12` and `:177-187`

- [ ] **Step 1: Read current code**

Open `prompt.rs:1-30` and `:170-200`. Confirm:
- `COLLAPSED_TOOL_LISTING_TITLE` constant exists at line 8-10
- `COLLAPSED_TOOL_LISTING_GUIDANCE` constant exists at line 11-12
- Both are concatenated in `render_collapsed_tool_listing_reminder()` at `:177-187`

- [ ] **Step 2: Write a character-count test (TDD red)**

In `prompt.rs` `mod tests` (or create one), add:

```rust
#[test]
fn render_collapsed_tool_listing_reminder_can_be_disabled() {
 // Test that when DISABLE_COLLAPSED_LISTING is true, the function returns empty
 // or returns just the body without the guidance text.
 // For now, this is a manual test — adjust after Step 4.
}
```

(Actual test logic depends on the implementation chosen in Step 3. Use a simple "function exists" test for now.)

- [ ] **Step 3: Implement the change**

Add a feature flag at the top of `prompt.rs`:

```rust
const DISABLE_COLLAPSED_TOOL_LISTING_REMINDER: bool = true; // v3.0
```

Modify `render_collapsed_tool_listing_reminder()` (`:177-187`) to:

```rust
pub fn render_collapsed_tool_listing_reminder(body: &str) -> String {
 if DISABLE_COLLAPSED_TOOL_LISTING_REMINDER {
 // v3.0: Drop the title + guidance; emit only the body.
 body.to_string()
 } else {
 format!("{}\n{}\n{}", COLLAPSED_TOOL_LISTING_TITLE, COLLAPSED_TOOL_LISTING_GUIDANCE, body)
 }
}
```

(Adjust the function signature to match the actual code. The body itself is ~150-200 bytes; the title + guidance is ~150-200 bytes; we're saving the latter.)

- [ ] **Step 4: Build check**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-agent-runtime 2>&1 | tail -10
```

Expected: `Finished` line. If the function signature doesn't match, read the actual `render_collapsed_tool_listing_reminder` and adjust the call.

- [ ] **Step 5: Run prompt tests**

```bash
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-agent-runtime 2>&1 | tail -10
```

Expected: All tests pass.

- [ ] **Step 6: Manual verify**

The change is in a `const` flag, so the **simplest manual verification is `git diff`** to see what changed. The actual prompt rendering is internal to the runtime and would only show in a real dialog turn. For v3.0 acceptance, the build + tests passing is sufficient.

- [ ] **Step 7: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/execution/agent-runtime/src/prompt.rs && git commit -m "feat(runtime): v3.0 C drop collapsed tool listing reminder"
```

### Task 2: Change B — Drop `default_tools` from agent listing

**Files:**
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\tools\implementations\task_tool.rs:357-372` (`format_agent_descriptions`)

- [ ] **Step 1: Read current code**

Open `task_tool.rs:350-420`. Confirm:
- `format_agent_descriptions(agents)` exists
- Each agent's XML has `<agent type="..."><description>...</description><tools>a, b, c</tools></agent>`
- `default_tools` is a `Vec<String>` joined with ", "

- [ ] **Step 2: Locate the rendering**

Search for `&lt;tools&gt;` or `<tools>` in `task_tool.rs` to find the exact line. The function likely uses a `format!` macro or `String` concatenation. The plan assumes it's at `:357-372`; adjust to actual line if different.

- [ ] **Step 3: Add a feature flag and modify**

At the top of `task_tool.rs`:

```rust
const DROP_AGENT_DEFAULT_TOOLS_IN_LISTING: bool = true; // v3.0
```

Replace the `<tools>` line in `format_agent_descriptions` with:

```rust
let tools_line = if DROP_AGENT_DEFAULT_TOOLS_IN_LISTING {
 String::new() // v3.0: drop the tools field; model discovers via GetToolSpec
} else {
 format!("<tools>{}</tools>", agent.default_tools.join(", "))
};
```

Then change the format string to include `tools_line` instead of the hard-coded `<tools>...</tools>`.

- [ ] **Step 4: Build check**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-assembly-core 2>&1 | tail -10
```

Expected: `Finished` line.

- [ ] **Step 5: Run tests**

```bash
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-assembly-core 2>&1 | tail -20
```

Expected: All tests pass. If a test asserts on the `<tools>` line, update the assertion to expect empty.

- [ ] **Step 6: Add a unit test**

In `task_tool.rs` (or a test file), add:

```rust
#[test]
fn format_agent_descriptions_drops_default_tools_in_v3() {
 // Construct a fake agent
 let agent = AgentSnapshotEntry {
 id: "test".to_string(),
 description: "test agent".to_string(),
 default_tools: vec!["a".to_string(), "b".to_string()],
 };
 let xml = format_agent_descriptions(&[agent]);
 assert!(!xml.contains("<tools>a, b</tools>"), "default_tools should be dropped in v3: {}", xml);
 assert!(xml.contains("test"), "agent id should still be present");
}
```

- [ ] **Step 7: Run the new test**

```bash
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-assembly-core format_agent_descriptions_drops_default_tools 2>&1 | tail -10
```

Expected: 1 test passes.

- [ ] **Step 8: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs && git commit -m "feat(core): v3.0 B drop default_tools from agent listing"
```

### Task 3: v3.0 Acceptance

- [ ] **Step 1: Run full test suite**

```bash
cd E:/agent-project/agent-app-v3 && cargo test --workspace 2>&1 | tail -20
```

Expected: All tests pass.

- [ ] **Step 2: Verify file changes**

```bash
cd E:/agent-project/agent-app-v3 && git log --oneline -3 && git diff HEAD~2 --stat
```

Expected: 2 commits, each touching exactly one file.

- [ ] **Step 3: Update PROJECT_STATE.md**

Append to "Known issues / progress" section:

```markdown
- v3.0 complete (2026-06-17): C (collapsed tool listing) + B (default_tools in agent listing) shipped. ~1,500-2,000 tokens saved.
```

- [ ] **Step 4: Commit docs**

```bash
cd E:/agent-project/agent-app-v3 && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.0 complete in PROJECT_STATE"
```

---

## Phase v3.1: Low-Risk Restructure (1-2 days, ~2,500-4,000 additional tokens)

**Goal:** Consolidate gstack skills + merge first-entry reminders.

### Task 4: Change E — Consolidate gstack skills

**Files:**
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\tools\implementations\skill_tool.rs` (or `skill_agent_snapshot.rs:280-316` `load_skill_entries`)
- Create: `E:\agent-project\agent-app-v3\src\crates\assembly\core\builtin_skills\_gstack\SKILL.md` (a parent skill that lists the 13 gstack sub-skills)
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\builtin_skills\find-skills\SKILL.md` (update if it references gstack skills)

**Note:** If 13 gstack skills have overlapping content, consolidation may also involve merging their SKILL.md files. For v3.1, the goal is **listing size reduction** only — full content merging is out of scope.

- [ ] **Step 1: Audit gstack skill names**

```bash
cd E:/agent-project/agent-app-v3 && ls src/crates/assembly/core/builtin_skills/ | grep "^gstack"
```

Expected output: ~13 directories starting with `gstack-`. Record them for the parent skill SKILL.md.

- [ ] **Step 2: Create parent skill**

Create `src/crates/assembly/core/builtin_skills/_gstack/SKILL.md` (note underscore prefix to sort first; the leading underscore is a convention to mark meta-skills):

```markdown
# GStack Skill Bundle

This skill is a meta-skill that lists the available gstack skills. Load this skill to discover which gstack skill fits your task.

## Available gstack skills

- `gstack-autoplan` — auto-planning
- `gstack-cso` — Chief Strategy Officer perspective
- `gstack-design-consultation` — design consultation
- `gstack-design-review` — design review
- `gstack-document-release` — document release workflow
- `gstack-investigate` — investigation
- `gstack-office-hours` — office hours
- `gstack-plan-ceo-review` — plan CEO review
- `gstack-plan-design-review` — plan design review
- `gstack-plan-eng-review` — plan engineering review
- `gstack-qa` — QA workflow
- `gstack-qa-only` — QA-only mode
- `gstack-retro` — retrospective
- `gstack-review` — general review
- `gstack-ship` — ship workflow

To use a gstack skill, load it directly via the `Skill` tool with the skill name.
```

- [ ] **Step 3: Add a mode policy filter**

In `skill_agent_snapshot.rs:280-316` `load_skill_entries`, add a filter for gstack skills. The exact code depends on the existing filter logic; pseudocode:

```rust
let effective_skills: Vec<_> = all_skills
 .into_iter()
 .filter(|s| {
 if s.id.starts_with("gstack-") {
 // v3.1: Only show the _gstack meta-skill, not the 13 individual gstack skills
 s.id == "_gstack"
 } else {
 true
 }
 })
 .collect();
```

(Adjust based on the actual filter logic. The goal is: in any mode's default listing, replace the 13 gstack skills with the single `_gstack` meta-skill.)

- [ ] **Step 4: Build + test**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-assembly-core 2>&1 | tail -10
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-assembly-core 2>&1 | tail -20
```

Expected: All tests pass.

- [ ] **Step 5: Add a test**

```rust
#[test]
fn gstack_skills_consolidated_in_listing() {
 let snapshot = resolve_skill_agent_snapshot(...);
 let gstack_count = snapshot.skills.iter().filter(|s| s.id.starts_with("gstack-")).count();
 assert_eq!(gstack_count, 0, "individual gstack skills should be filtered out, only _gstack should appear");
 let has_meta = snapshot.skills.iter().any(|s| s.id == "_gstack");
 assert!(has_meta, "_gstack meta-skill should be in the listing");
}
```

- [ ] **Step 6: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/assembly/core/builtin_skills/_gstack/ src/crates/assembly/core/src/agentic/ && git commit -m "feat(core): v3.1 E consolidate gstack skills into _gstack meta-skill"
```

### Task 5: Change F — Merge first-entry reminders

**Files:**
- Create: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\agents\prompts\_mode_first_entry_shared.md` (~2,000 chars; shared boilerplate)
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\agents\prompts\plan_mode_first_entry_reminder.md` (slim down to ~1,000 chars; reference `_mode_first_entry_shared.md`)
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\agents\prompts\multitask_mode_first_entry_reminder.md` (same)
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\agents\prompts\debug_mode_first_entry_reminder.md` (same)

- [ ] **Step 1: Read all 3 reminder files**

```bash
cd E:/agent-project/agent-app-v3 && wc -l src/crates/assembly/core/src/agentic/agents/prompts/{plan,multitask,debug}_mode_first_entry_reminder.md
```

- [ ] **Step 2: Identify shared content**

Manually read all 3 files. Identify the 60-80% of content that's the same across all three (e.g., "this is the first turn of a new mode" boilerplate, "you have access to the following tools", etc.). Extract that into `_mode_first_entry_shared.md`.

- [ ] **Step 3: Create shared file**

```markdown
# First-turn Reminder (Shared)

This is the first turn in this mode. Some additional guidance applies for the first turn only.

[Shared boilerplate extracted from the 3 mode reminders]
```

- [ ] **Step 4: Slim each mode reminder**

For each of plan/multitask/debug, keep only the mode-specific content. Reference the shared file via a comment:

```markdown
<!-- See _mode_first_entry_shared.md for the shared first-turn boilerplate -->

# Plan Mode — First Turn

[Plan-mode-specific content only]
```

- [ ] **Step 5: Update build.rs (if reminders are embedded)**

If the prompts are embedded at build time via `build.rs`, ensure the new `_mode_first_entry_shared.md` is also embedded. Search for the existing reminder embedding code in `build.rs` and add the new file.

- [ ] **Step 6: Build + test**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-assembly-core 2>&1 | tail -10
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-assembly-core 2>&1 | tail -20
```

- [ ] **Step 7: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/assembly/core/src/agentic/agents/prompts/ && git commit -m "feat(core): v3.1 F merge first-entry reminders into shared template"
```

### Task 6: v3.1 Acceptance

- [ ] **Step 1: Run full test suite**

```bash
cd E:/agent-project/agent-app-v3 && cargo test --workspace 2>&1 | tail -20
```

- [ ] **Step 2: Update PROJECT_STATE.md**

```markdown
- v3.1 complete (2026-06-17): E (gstack consolidation) + F (first-entry reminder merge). Cumulative: ~4,000-6,000 tokens saved.
```

- [ ] **Step 3: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.1 complete"
```

---

## Phase v3.2: Behavior-Changing (2-3 days, ~3,000-3,500 additional tokens)

**Goal:** Move `auto_memory.rs` instructions into a Skill.

### Task 7: Create `memory` Skill

**Files:**
- Create: `E:\agent-project\agent-app-v3\src\crates\assembly\core\builtin_skills\memory\SKILL.md`

- [ ] **Step 1: Read `auto_memory.rs:114-244`**

Open `auto_memory.rs:100-250` and read the full instructional text. This becomes the basis for the new skill's SKILL.md.

- [ ] **Step 2: Create the skill**

Create `src/crates/assembly/core/builtin_skills/memory/SKILL.md` with the contents of the 18,645-byte block (or a polished version). Make sure the skill name is `memory`.

- [ ] **Step 3: Verify the skill is auto-registered**

The skill registration code (likely in `skills/builtin.rs`) should auto-discover the new directory. Verify by:

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-assembly-core 2>&1 | tail -5
```

If the skill isn't auto-discovered, manually register it in `builtin.rs`.

- [ ] **Step 4: Commit (skill only)**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/assembly/core/builtin_skills/memory/ && git commit -m "feat(core): v3.2 add memory skill (content extracted from auto_memory.rs)"
```

### Task 8: Reduce `auto_memory.rs` to a short pointer

**Files:**
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\service\agent_memory\auto_memory.rs:114-244`
- Modify: `E:\agent-project\agent-app-v3\src\crates\assembly\core\src\agentic\agents\prompt_builder\prompt_builder_impl.rs:693-713` (the `{AGENT_MEMORY}` placeholder fill)

- [ ] **Step 1: Add a feature flag**

In `auto_memory.rs`:

```rust
const USE_MEMORY_SKILL_POINTER: bool = true; // v3.2
```

- [ ] **Step 2: Replace the inline block**

Modify `build_workspace_agent_memory_prompt` (`:106-245`) to:

```rust
pub fn build_workspace_agent_memory_prompt() -> String {
 if USE_MEMORY_SKILL_POINTER {
 // v3.2: Replace 18,645-byte inline block with a short pointer
 MEMORY_SKILL_POINTER.to_string()
 } else {
 // Original full instructions
 MEMORY_FULL_INSTRUCTIONS.to_string()
 }
}

const MEMORY_SKILL_POINTER: &str = r#"
Memory is managed via the `memory` skill. Load it via the `Skill` tool when you need to:
- Write important user context to memory
- Recall past user preferences or decisions
- Update the memory index (`<workspace>/.agent-app/memory/memory.md`)

Periodically (every 5-10 turns), load the `memory` skill and update `memory.md` with important context from the conversation.
"#;

const MEMORY_FULL_INSTRUCTIONS: &str = r#"
// (existing 18,645-byte block, moved here unchanged)
"#;
```

(Cut the 18,645-byte block from its current location and paste it into `MEMORY_FULL_INSTRUCTIONS`. The compile-time size of the binary doesn't change, but the **runtime prompt** does — only `MEMORY_SKILL_POINTER` is rendered into the prompt.)

- [ ] **Step 3: Update the call site**

In `prompt_builder_impl.rs:693-713`, the `{AGENT_MEMORY}` placeholder is filled by `build_workspace_agent_memory_prompt()`. With the feature flag, this now returns the short pointer. **No code change needed** in `prompt_builder_impl.rs` — the API is the same.

- [ ] **Step 4: Build + test**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-assembly-core 2>&1 | tail -10
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-assembly-core 2>&1 | tail -20
```

- [ ] **Step 5: Add a test for the pointer**

```rust
#[test]
fn memory_skill_pointer_is_short() {
 let prompt = build_workspace_agent_memory_prompt();
 assert!(prompt.len() < 1000, "v3.2 pointer should be <1K chars, got {}", prompt.len());
 assert!(prompt.contains("memory skill"), "pointer should reference the memory skill");
}
```

- [ ] **Step 6: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/assembly/core/src/service/agent_memory/auto_memory.rs && git commit -m "feat(memory): v3.2 A replace inline 18KB block with short skill pointer"
```

### Task 9: v3.2 Acceptance

- [ ] **Step 1: Run full test suite**

```bash
cd E:/agent-project/agent-app-v3 && cargo test --workspace 2>&1 | tail -20
```

- [ ] **Step 2: Manual end-to-end test (memory writes)**

Start the CLI and have a multi-turn conversation. Verify the model still writes to `memory.md` (just less frequently).

- [ ] **Step 3: Update PROJECT_STATE.md**

```markdown
- v3.2 complete (2026-06-17): A (auto_memory — memory skill). Cumulative: ~7,000-9,500 tokens saved. Behavior change: model writes to memory less frequently.
```

- [ ] **Step 4: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.2 complete"
```

---

## Phase v3.3: Opt-In Feature (1-2 days, ~300-1,500 additional tokens)

**Goal:** Make Project Layout opt-in.

### Task 10: Change D — Remove ProjectLayout from default User Context

**Files:**
- Modify: `E:\agent-project\agent-app-v3\src\crates\execution\agent-runtime\src\agents.rs:50-56` (`shared_coding_mode_user_context_policy`)
- (Optional) Add a `get_project_layout` tool

- [ ] **Step 1: Read the current policy**

Open `agent-runtime/agents.rs:50-56` and confirm `UserContextSection::ProjectLayout` is in the default policy.

- [ ] **Step 2: Remove ProjectLayout from the default**

```rust
// Before (in shared_coding_mode_user_context_policy):
UserContextPolicy::default()
 .with_section(UserContextSection::ProjectLayout)
 .with_section(UserContextSection::WorkspaceContext)
 // ...

// After (v3.3):
UserContextPolicy::default()
 // ProjectLayout removed; available on demand via LS/Glob
 .with_section(UserContextSection::WorkspaceContext)
 // ...
```

(Adjust the actual API based on `UserContextPolicy`'s real methods.)

- [ ] **Step 3: Add a feature flag (for quick rollback)**

At the top of `agents.rs`:

```rust
const INCLUDE_PROJECT_LAYOUT_BY_DEFAULT: bool = false; // v3.3
```

Wrap the policy modification:

```rust
let mut policy = UserContextPolicy::default()
 .with_section(UserContextSection::WorkspaceContext)
 // ...;
if INCLUDE_PROJECT_LAYOUT_BY_DEFAULT {
 policy = policy.with_section(UserContextSection::ProjectLayout);
}
```

- [ ] **Step 4: Build + test**

```bash
cd E:/agent-project/agent-app-v3 && cargo build -p agent-app-agent-runtime 2>&1 | tail -10
cd E:/agent-project/agent-app-v3 && cargo test -p agent-app-agent-runtime 2>&1 | tail -20
```

- [ ] **Step 5: Add a test**

```rust
#[test]
fn project_layout_excluded_by_default_in_v3() {
 let policy = shared_coding_mode_user_context_policy();
 assert!(!policy.includes(UserContextSection::ProjectLayout), "ProjectLayout should be excluded by default in v3.3");
}
```

- [ ] **Step 6: (Optional) Add a `get_project_layout` tool**

If a future need arises, add a tool that returns the project layout on demand. For v3.3, the model can use `LS`/`Glob` instead.

- [ ] **Step 7: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add src/crates/execution/agent-runtime/src/agents.rs && git commit -m "feat(runtime): v3.3 D exclude ProjectLayout from default user context"
```

### Task 11: v3.3 Acceptance

- [ ] **Step 1: Run full test suite**

```bash
cd E:/agent-project/agent-app-v3 && cargo test --workspace 2>&1 | tail -20
```

- [ ] **Step 2: Update PROJECT_STATE.md**

```markdown
- v3.3 complete (2026-06-17): D (ProjectLayout opt-in). Cumulative: ~7,500-11,000 tokens saved.
```

- [ ] **Step 3: Commit**

```bash
cd E:/agent-project/agent-app-v3 && git add docs/PROJECT_STATE.md && git commit -m "docs: mark v3.3 complete"
```

---

## Self-Review

**1. Spec coverage:**

| Spec section | Implemented in |
|---|---|
| §1 Measured components | Background only; this plan references the measurements |
| §2 Goals (savings targets) | All 6 changes A-F have estimated savings |
| §3 Six concrete changes | All 6: A (Tasks 7-9), B (Task 2), C (Task 1), D (Task 10), E (Task 4), F (Task 5) |
| §4 Total estimated savings | ~7,500-11,000 tokens, achievable via the 11 tasks |
| §5 Implementation order | 4 phases: v3.0 (0.5d), v3.1 (1-2d), v3.2 (2-3d), v3.3 (1-2d) |
| §6 Open questions | Not blocking; documented in spec for follow-up |
| §7 Decision log | Reflected in phase ordering |

**2. Placeholder scan:**

- No "TBD" / "TODO" / "待定" in task descriptions
- All `(adjust based on actual...)` notes are flagged in spec for the implementer to verify at runtime

**3. Type consistency:**

- `UserContextSection::ProjectLayout` — verify this enum variant exists in `prompt_builder_impl.rs`
- `UserContextPolicy::with_section` — verify method name
- `SkillInfo` field `id` / `name` / `description` / `path` — used consistently (from `types.rs:60-76`)
- `format_agent_descriptions` — verify function name in `task_tool.rs`
- `render_collapsed_tool_listing_reminder` — verify function name in `prompt.rs`

**4. Scope check:**

- Each task is a single-file or paired-files edit
- Each phase is independently shippable
- v3.0 alone is 1,500-2,000 tokens in 0.5 day
- Total scope: 11 tasks across 4 phases, 5-8 working days

**5. Risk management:**

- Every change is behind a `const` flag for instant rollback
- Every change has a test
- v3.0 (C + B) is trivial/low-risk; recommended as first step

---

**Plan complete and saved to `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md`.**

**Two execution options:**

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach— **

---

## Post-v3 Candidates (Not in v3.0-v3.4 scope, but worth doing)

After v3.0-v3.4 (5 phases, 11 tasks) shipped, ~6,500-9,500 tokens/turn saved. Additional candidates identified during implementation, **not in current spec/plan scope**:

### Candidate A: Mode prompt 精简 (v3.5— )
- **Files**: `team_mode.md` (19K), `deep_research_agent.md` (23K), `deep_review_agent.md` (24K), `cowork_mode.md` (14K)
- **Approach**: 类似 v3.2 Change A — 提取内容— Skill, prompt 留短 pointer
- **Saving**: ~4,000-12,000 tokens/turn
- **Difficulty**: — (每个 prompt — 1 — const + — 1 — skill)
- **Risk**: — (— prompt 独立— ### Candidate B: Tool manifest 重构 (v3.6— )
- **Files**: `tool-contracts/framework.rs`, `tools/registry.rs`, `tools/product_runtime/catalog.rs`, `tools/product_runtime/get_tool_spec_tool.rs`
- **Approach**: 24 expanded tools — 5 core (always listed) + 19 advanced (via GetToolSpec on demand)
- **Saving**: ~5,000-10,000 tokens/turn
- **Difficulty**: — (— 24 — tool 的发现机— **Risk**: — (影响所— tool)

### Candidate C: 实施 CompressAgent / LoopEngineerAgent (P1-9)
- **Files**: 新建 `subagents/compress.rs`, `subagents/loop_engineer.rs`, 2 — prompt, 注册— catalog
- **Approach**: 从零创建 (之前不存— . 应该作为 advisor-only (类似 reviewer)
- **Saving**: 不省 token, 但补— sub-agent 生— **Difficulty**: — **Risk**: — ### Candidate D: 16 CLI dead_code warnings (P2-4)
- **Files**: `agent-app-cli` crate
- **Approach**: — `cargo build -p agent-app-cli 2>&1 | grep "never used"`, 逐个— **Saving**: 0 token, 干净
- **Difficulty**: — **Risk**: 0 (— *先确— *不是 future stub)

### Candidate E: GUI mobile-web/dist 资源问题
- **Files**: `apps/desktop/tauri.conf.json`, `pnpm` build pipeline
- **Approach**: — `pnpm run build:web && pnpm run prepare:mobile-web`, 或改 tauri.conf.json 跳过
- **Saving**: 不省 token, 解锁 desktop build
- **Difficulty**: — **Risk**: — (但要先确— frontend 实际能用)

### Parallel execution

A-E 都可— *独立并行**执行 (前提: 不同时改同一文件). 每个沿用 v3 模式:
- `const` flag + regression test + commit + PROJECT_STATE 更新
- 一行回— (— flag)
- 不破坏现— 821+ tests

See `HANDOFF.md` (in worktree root) for the full handoff document.
