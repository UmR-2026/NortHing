# v3 Prompt Loader — Realistic Design (v2)

**Status**: Awaiting user review
**Date**: 2026-06-17
**Author**: Subagent-driven development session (ZCode) with user
**Replaces**: `2026-06-17-v3-prompt-loader-design.md` (v1 draft, deprecated — based on deleted `northhing-memory` crate)
**Spec target**: 6 concrete code changes against actual northhing v3 code

---

## 1. Problem (verified against real code)

northhing's main agent prompt has been **estimated at 30-50K input tokens** for trivial inputs. The earlier "73,882" claim was never measured; the actual cached system prompt is ~46K chars / ~11.5K tokens (without tool manifest, skill/agent listing, or user context).

### Measured components (file sizes)

| Component | chars | tokens (梅4) | File:line |
|---|---|---|---|
| `auto_memory.rs` injected block (cached, every turn) | 18,645 | 4,661 | `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:114-244` |
| `agentic_mode.md` (cached, every turn) | 11,009 | 2,752 | `src/crates/assembly/core/src/agentic/agents/prompts/agentic_mode.md` |
| `plan_mode_first_entry_reminder.md` (cached first turn) | 3,978 | 994 | `src/crates/assembly/core/src/agentic/agents/prompts/plan_mode_first_entry_reminder.md` |
| `multitask_mode_first_entry_reminder.md` (cached first turn) | 6,580 | 1,645 | `src/crates/assembly/core/src/agentic/agents/prompts/multitask_mode_first_entry_reminder.md` |
| `debug_mode_first_entry_reminder.md` (cached first turn) | 6,002 | 1,500 | `src/crates/assembly/core/src/agentic/agents/prompts/debug_mode_first_entry_reminder.md` |
| `agentic_mode_first_entry_reminder.md` | 30 | 7 | `src/crates/assembly/core/src/agentic/agents/prompts/agentic_mode_first_entry_reminder.md` |
| **Subtotal (cached system + first-turn reminders)** | **46,244** | **~11,561** | |

### Unmeasured (estimates)

| Component | est. tokens | Notes |
|---|---|---|
| Skill listing (`<available_skills>` for 22 builtin) | ~800-1,500 | Only `name + description + path` rendered (not full SKILL.md). See `types.rs:60-76` `to_xml_desc`. |
| Agent listing (`<available_agents>` for 23 builtin) | ~2,000-3,000 | Includes `default_tools` for each agent (multiplier on shared_coding_mode_tools). See `task_tool.rs:357-419`. |
| Tool manifest (full JSON for 24 expanded tools) | ~10,000-15,000 | Each tool has 1-3 KB description + JSON schema. See `tool-contracts/framework.rs:381-398`. |
| User context (project layout, memory files, AGENTS.md) | ~500-2,000 | Capped at 200 entries for layout. See `prompt_builder_impl.rs:474-503`. |
| Collapsed tool listing reminder | ~80-120 | Just a 4-line guidance text + 19 short tool names. See `agent-runtime/src/prompt.rs:8-12`. |
| **Total estimate** | **~25,000-35,000** | |

**Conclusion**: 73K is plausible (with all tool manifest + reminders), but **the dominant bloat is the cached system prompt + tool manifest**, not skill/agent listings (the v1 spec incorrectly attributed the bloat to skills/agents).

## 2. Goals

| Goal | Metric | Strategy |
|---|---|---|
| Reduce cached system prompt | ~4,600 tokens (40% of cached) | Move `auto_memory` instructions into a Skill (A) |
| Reduce first-turn reminders | ~2,000-3,000 tokens | Merge plan/multitask/debug reminders into one shared template (F) |
| Reduce agent listing bloat | ~1,500-2,000 tokens | Drop `default_tools` field from listing (B) |
| Reduce user context bloat | ~500-2,000 tokens | Make Project Layout opt-in (D) |
| Reduce skill listing bloat | ~500-1,000 tokens | Consolidate gstack-* skills (E) |
| Reduce reminder bloat | ~80-120 tokens | Drop collapsed tool listing reminder (C) |
| **Total potential** | **~10,000-15,000 tokens** | (33-43% reduction) |

Non-goals (different from v1 spec):
- Building `northhing-memory` crate — it doesn't exist; memory is file-based
- Background MemoryAgent with mpsc channel — there's no LLM distiller; the main agent manages memory via `auto_memory.rs` instructions
- `read_memory` tool — no semantic search needed; the main agent already writes/reads `memory.md` directly
- DB-backed skill/agent indexing — overkill for 22/23 entries; current in-memory registry is fine

## 3. Six concrete changes (A-F)

### Change A: Move `auto_memory.rs` instructions into a Skill

**File:** `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:114-244` (the inline `format!(r#"# auto memory..."#)` block, 18,645 chars)

**Current state**: The 18,645-byte instructional text is injected into every turn's system prompt via the `{AGENT_MEMORY}` placeholder in `prompt_builder_impl.rs:693-713`. This is **the single largest cached block** (~4,661 tokens, ~40% of cached system prompt).

**Change**: Convert these instructions into a Skill (saved as a new builtin skill at `src/crates/assembly/core/builtin_skills/memory/SKILL.md` or similar). Replace the 18,645-byte inline block with a short pointer (~200-500 chars):
- "Memory is managed via the Skill tool. Load `memory` skill when you need to write or recall user memory."

**Estimated saving**: ~4,000-4,500 tokens/turn (the inline block is replaced with a much shorter pointer).

**Risk**: Medium. The current model writes to memory proactively on every turn (because the instructions tell it to). Moving to a Skill means the model only loads the skill on demand. **User-visible behavior change**: memory writes may be less frequent.

**Mitigation**: Keep a short reminder in the system prompt (~100-200 chars) that says "Periodically load the `memory` skill and update `memory.md` with important user context."

### Change B: Drop `default_tools` from `<available_agents>` listing

**File:** `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs:357-372` (the `<agent type="..."><description>...</description><tools>a, b, c</tools></agent>` rendering)

**Current state**: Each agent in the listing emits its `default_tools` field as a comma-joined list. For `agentic`-class agents with ~24 tools in `shared_coding_mode_tools()` (`agents/mod.rs:75-102`), this multiplies across 5-7 visible sub-agents — ~150-200 tool name repetitions per turn.

**Change**: Drop the `<tools>` line from `task_tool.rs:357-372` `format_agent_descriptions`. Replace with a comment: `<!-- see agent's tool policy via Task dispatch -->`.

**Estimated saving**: ~1,500-2,000 tokens/turn (8-10KB of redundant tool name lists).

**Risk**: Low. The model can discover an agent's tools by either:
- Reading the agent's source file (the `path` field in `<agent>`)
- Using `GetToolSpec` (already in the tool manifest)
- Trial-and-error via `Task` dispatch (it will get an error if it tries a wrong tool)

**Mitigation**: Add a one-line note in the listing: "For sub-agent capabilities, see the agent's source or use `GetToolSpec`."

### Change C: Drop `# Collapsed Tool Listing` reminder

**File:** `src/crates/execution/agent-runtime/src/prompt.rs:8-12` (the 4-line `COLLAPSED_TOOL_LISTING_GUIDANCE` constant), invoked at `:177-187`.

**Current state**: The "Collapsed Tool Listing" reminder adds a 4-line guidance text + a bullet list of 19 collapsed tool names wrapped in `<collapsed_tools>...</collapsed_tools>`. Total ~300-400 chars per turn.

**Change**: Delete the `COLLAPSED_TOOL_LISTING_TITLE` + `COLLAPSED_TOOL_LISTING_GUIDANCE` constants. Emit only the body (or drop the entire reminder).

**Estimated saving**: ~80-120 tokens/turn.

**Risk**: Zero. Collapsed tools are already self-describing — the model sees the tool's stub message ("THIS IS A COLLAPSED TOOL. Before first use, call GetToolSpec(— " at `product_runtime/catalog.rs:583`) in the full tool manifest.

### Change D: Make Project Layout opt-in (not in default User Context)

**File:** `src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs:474-503` (`get_project_layout`, capped at 200 entries) and `agent-runtime/agents.rs:50-56` (`shared_coding_mode_user_context_policy` which includes `UserContextSection::ProjectLayout`).

**Current state**: Project Layout (directory tree, capped at 200 entries) is included in `User Context` for every turn. For non-trivial repos, this is 2-8KB / 500-2,000 tokens, regenerated on every turn.

**Change**: Remove `UserContextSection::ProjectLayout` from `shared_coding_mode_user_context_policy()` in `agent-runtime/agents.rs:50-56`. Keep the `get_project_layout` function for explicit invocation (e.g., a future `get_project_layout` tool or a `UserContextPolicy::with_project_layout()` opt-in).

**Estimated saving**: ~500-2,000 tokens/turn (varies by repo size).

**Risk**: Medium. Some flows may rely on the layout being there (e.g., agents that don't use `LS`/`Glob` first). Test in non-trivial repos before merge.

**Mitigation**: Add a one-line note in `User Context` block: "Project layout available on demand via `LS`/`Glob`."

### Change E: Consolidate `gstack-*` skills into one entry

**File:** `src/crates/assembly/core/builtin_skills/` (13 directories starting with `gstack-`): `gstack-autoplan, gstack-cso, gstack-design-consultation, gstack-design-review, gstack-document-release, gstack-investigate, gstack-office-hours, gstack-plan-ceo-review, gstack-plan-design-review, gstack-plan-eng-review, gstack-qa, gstack-qa-only, gstack-retro, gstack-review, gstack-ship`.

**Current state**: All 13+ gstack skills are listed individually in `<available_skills>` for any workspace where they're enabled. Each takes ~200-300 chars in the listing.

**Change**: Either:
- (a) Move the 13 skills into one nested `gstack` skill that internally dispatches via a sub-command (`/plan`, `/review`, etc.), or
- (b) Add a per-mode filter in `load_skill_entries` (`skill_agent_snapshot.rs:280-316`) so only the 2-3 most relevant gstack skills are listed by default.

**Recommended**: (b) — safer, less invasive. Add a new field `gstack_default_skill` to mode policy that picks which 2-3 gstack skills are listed for that mode.

**Estimated saving**: ~500-1,000 tokens/turn (cuts the 13 gstack skills down to 2-3 per turn).

**Risk**: Low. The full skill set is still available — just not all listed at once. User can still load any specific skill via `Skill` tool.

### Change F: Merge plan/multitask/debug `*_first_entry_reminder.md` into one shared template

**Files:**
- `src/crates/assembly/core/src/agentic/agents/prompts/plan_mode_first_entry_reminder.md` (3,978 chars)
- `src/crates/assembly/core/src/agentic/agents/prompts/multitask_mode_first_entry_reminder.md` (6,580 chars)
- `src/crates/assembly/core/src/agentic/agents/prompts/debug_mode_first_entry_reminder.md` (6,002 chars)

**Current state**: Three separate reminder templates, each injected on the first turn of the respective mode. Total ~16,560 chars / 4,140 tokens for the first turn of any of these modes.

**Change**: Audit the three templates for overlap. Most likely they share 60-80% of content (e.g., "this is the first turn of mode X" boilerplate). Extract the shared parts into one `_mode_first_entry_shared.md` (~2,000 chars) and keep only the mode-specific bits (~1,000 chars each).

**Estimated saving**: ~2,000-3,000 tokens **on the first turn only** of plan/multitask/debug modes (subsequent turns don't see this).

**Risk**: Low. Each mode's first-entry reminder is mode-specific enough that the model can tell them apart. The shared boilerplate is genuinely shared.

**Mitigation**: A/B test by running the same query in all three modes before/after; compare model output.

## 4. Total estimated savings

| Change | Saving (tokens/turn) | Difficulty | Risk | Net (subtract risk budget) |
|---|---|---|---|---|
| A. Move auto_memory to Skill | ~4,000-4,500 | Medium | Medium | ~3,000-3,500 |
| B. Drop `default_tools` from agent listing | ~1,500-2,000 | Low | Low | ~1,500-2,000 |
| C. Drop collapsed tool listing reminder | ~80-120 | Trivial | Zero | ~80-120 |
| D. Project Layout opt-in | ~500-2,000 | Medium | Medium | ~300-1,500 |
| E. Consolidate gstack-* skills | ~500-1,000 | Medium | Low | ~500-1,000 |
| F. Merge mode first-entry reminders | ~2,000-3,000 (first turn only) | Low | Low | ~2,000-3,000 |
| **Total (best case)** | **~9,000-12,500** | | | **~7,500-11,000** |
| **Conservative total** | | | | **~7,000-9,000** |

If the unmeasured total is 25-35K, **post-v3 total would be ~16-25K** (43-50% reduction).

This is less aggressive than v1 spec's 73K — 2K claim, but **honest and achievable**.

## 5. Implementation order

| Phase | Days | Changes | Cumulative saving |
|---|---|---|---|
| **v3.0** (Quick wins) | 0.5 day | C (trivial drop) + B (low-risk drop) | ~1,500-2,000 tokens |
| **v3.1** (Low-risk restructure) | 1-2 days | E (gstack consolidation) + F (reminder merge) | ~4,000-6,000 tokens |
| **v3.2** (Behavior-changing) | 2-3 days | A (auto_memory — Skill) | ~7,000-9,500 tokens |
| **v3.3** (Opt-in feature) | 1-2 days | D (Project Layout opt-in) | ~7,500-11,000 tokens |
| **Total** | **5-8 days** | All 6 changes | **~7,500-11,000 tokens saved** |

**v3.0 alone is 1,500-2,000 tokens in 0.5 day** — recommended first step regardless of broader commitment.

## 6. Open questions (for follow-up)

1. **Memory write frequency trade-off (Change A)**: User may notice that the model writes to memory less often. Is this acceptable, or do we need a hook in the agent loop to remind the model to write— 2. **Project Layout opt-in (Change D)**: Should we add a `get_project_layout` tool so the model can fetch the layout on demand, or rely on `LS`/`Glob`— 3. **gstack consolidation (Change E)**: Are all 13 gstack skills actively used, or are some legacy— If legacy, they could be deleted entirely.
4. **Tool manifest bloat (out of scope for v3)**: The 10-15K-token tool manifest is the largest unmeasured contributor. Future work could split tools into "core" (always listed) and "advanced" (on-demand via `GetToolSpec`).
5. **Mode prompt templates (out of scope)**: `team_mode.md` (19K chars), `deep_research_agent.md` (24K), `deep_review_agent.md` (24K) are large but mode-specific. Could be reduced via content audit.

## 7. Decision log

| # | Decision | Rationale |
|---|---|---|
| 1 | Scope to 6 changes A-F | Each is independently shippable; A alone is the biggest win |
| 2 | v3.0 = C + B only | Trivial or low-risk drops, fast wins |
| 3 | Defer Change A to v3.2 | Behavior-changing; needs user acceptance testing |
| 4 | Skip all v1 spec features | northhing-memory / read_memory / embedding don't apply; memory is file-based |
| 5 | Conservative savings estimate | Don't over-promise; ~7-11K tokens is realistic |
