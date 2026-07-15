# Skills Domain — Signatures

> One-page function signature card. Find the right function here, then open
> the corresponding `NN-*.rs` mirror for the full body.

## Resolver v2 — on-demand relevance (the headline algorithm)

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/resolver_v2.rs`

| Function | Line | Signature | Purpose |
|---|---|---|---|
| `resolve_for_prompt` | 44 | `fn(prompt: &str, skills: &[SkillInfo]) -> Vec<ResolvedSkill>` | Top-`RESOLVED_SKILLS_MAX` (5) skills by weighted Jaccard. |
| `resolve_for_prompt_with_max` | 49 | `fn(prompt: &str, skills: &[SkillInfo], max_results: usize) -> Vec<ResolvedSkill>` | Same, caller picks the cap. |
| `score_skill` | 91 | `fn(prompt_keywords: &HashSet<String>, skill: &SkillInfo) -> f64` | Per-skill score, weighted (name 2×, desc 1×). |
| `tokenize` | 141 | `fn(text: &str) -> HashSet<String>` | Alphanumeric split, lowercased, stop-words removed, min length 2. |

**Constants:**
- `RESOLVED_SKILLS_MAX: usize = 5` (line 18)
- `MIN_RELEVANCE_SCORE: f64 = 0.05` (line 22)
- `STOP_WORDS: &[&str]` (line 142) — ~80 common English words + domain fillers

## Resolver v1 — availability gate (different from v2!)

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/resolver.rs`

| Function | Line | Signature | Purpose |
|---|---|---|---|
| `resolve_skill_state_for_mode` | 12 | `fn(skill: &SkillCandidate, mode_id: SkillModeId) -> ModeSkillState` | Boolean gate: enabled or not in this mode, with reason. |
| `resolve_skill_default_enabled_for_mode` | — | `fn(dir_name: &str, mode_id: SkillModeId) -> Option<bool>` | Built-in default per mode. |

> The "v1" / "v2" naming is **about different problems**:
> - v1 = "should this skill be enabled in this mode?" (gate)
> - v2 = "given a prompt, which enabled skills are most relevant?" (ranking)
> Do not collapse them in new code.

## Catalog — built-in skills

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/catalog.rs`

| Item | Line | Purpose |
|---|---|---|
| `BuiltinSkillId` | 8 | 24-variant enum of all built-in skills. |
| `BuiltinSkillGroup` | 36 | `Office` / `Meta` / `ComputerUse` / `Gstack` taxonomy. |
| `BuiltinSkillSpec` | 55 | `(id, group)` pair. |
| `BUILTIN_SKILL_SPECS` | — | Static slice: `dir_name -> (id, group)`. |
| `builtin_skill_spec(dir_name)` | 184 | Lookup by dir_name. |
| `builtin_skill_group_key(dir_name)` | 194 | Returns the group key string. |
| `builtin_skill_dir_names()` | 198 (impl) | All dir names. |

## Policy — mode-by-mode enablement table

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/policy.rs`

| Item | Line | Purpose |
|---|---|---|
| `SkillModeId` | — | Enum of all modes (`agentic`, `Cowork`, `Team`, `DeepResearch`, `Claw`, `coding_shared`, `Other`). |
| `ModeSkillPolicy` | 64 | The table of rules per mode. |
| `SkillPolicyRule` | 62 | Single rule (matches a skill, declares default effect). |
| `PolicyEffect` | 46 | `Enable` / `Disable` / `Default`. |
| `policy_for_mode(mode_id)` | — | Returns the policy table for a mode. |
| `resolve_builtin_default_enabled(dir_name, mode_id)` | 165 | `Option<bool>` — `None` for unknown. |
| `resolve_builtin_default_effect(spec, mode_id)` | — | Per-rule effect, declaration-ordered (later wins). |

## Types — the data shapes

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/types.rs`

| Type | Line | Purpose |
|---|---|---|
| `SkillLocation` | 10 | `User` / `Project` — slot location. |
| `SkillInfo` | 29 | Full registry metadata; has `to_xml_desc()` at line 60. |
| `ModeSkillStateReason` | 82 | The 7-variant reason for current state. |
| `ModeSkillInfo` | 95 | `SkillInfo` + mode annotations + `state_reason`. |
| `SkillData` | 113 | Loaded content (for execution). |
| `SkillData::from_markdown` | 126 | Parses SKILL.md front-matter, requires `name` and `description`. |

## Built-in installer

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/builtin.rs`

| Item | Line | Purpose |
|---|---|---|
| `BUILTIN_SKILLS_BUNDLE_HASH` | — | SHA-256 of the embedded `builtin_skills/` tree, set at build time. |
| `LEGACY_BUILTIN_SKILL_DIR_NAMES` | 27 | 14 Superpowers-era skill dir names to delete on upgrade. |
| `LEGACY_BUILTIN_ROOT_FILES` | 45 | `["SUPERPOWERS_LICENSE.txt"]` to delete. |
| `ensure_builtin_skills_installed()` | 220 | Syncs embedded builtins to `<user_skills>/.system/`. Short-circuits on hash match. |

## Snapshot adapter (consumer of registry + resolver)

Source: `src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs`
*(693 lines, not mirrored — read directly when extending)*

| Item | Line | Purpose |
|---|---|---|
| `USE_SKILL_REGISTRY` | 27 | `bool = true` — coarse rollback switch. |
| `SkillSnapshotEntry` | 30 | One skill in the snapshot. |
| `AgentSnapshotEntry` | 64 | One agent in the snapshot. |
| `TurnSkillAgentSnapshot` | 82 | Per-turn immutable snapshot. |
| `SkillAgentDiff` | 96 | Diff between two snapshots. |
| `resolve_skill_agent_snapshot(...)` | 190 | Main entry point. |
| `diff_skill_agent_snapshot(prev, cur)` | 368 | Produce a `SkillAgentDiff`. |
| `render_full_skill_listing_body(skills)` | 467 | **v3 fallback path — see NOTES.md.** |
| `render_resolved_skill_listing_body(skills, user_prompt)` | 486 | **The current path.** Falls back to full-list on no-prompt / no-match. |
| `build_skill_agent_tool_listing_sections_from_snapshot(snapshot)` | 557 | Build the prompt sections. |
| `build_embedded_user_context_reminder(...)` | 445 | User-context reminder section. |

## Skill tool (consumer of registry, exposed as a `Tool`)

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skill_tool.rs`
*(632 lines, not mirrored — read directly when extending)*

| Item | Line | Purpose |
|---|---|---|
| `SkillTool` struct | 18 | Implements `Tool` trait, tool name `"Skill"`. |
| `SkillTool::new()` | 21 | Constructor. |
| `SkillTool::resolved_skills_xml_for_context(...)` | 46 | `<available_skills>` XML for tool description. |
| `SkillTool::build_available_skills_context_section(...)` | 89 | Build prompt context section. |

`call_impl` dispatches on whether the command is a stable key
(`user::slot::dir`, three `::` segments) and routes to:
`find_and_load_skill_*` (local + remote variants) or
`find_and_load_skill_by_key_*` (stable key variants).

## Module facade

Source: `src/crates/assembly/core/src/agentic/tools/implementations/skills/mod.rs`

```rust
pub use registry::SkillRegistry;
pub use resolver_v2::{resolve_for_prompt, resolve_for_prompt_with_max, ResolvedSkill};
pub use types::{SkillData, SkillInfo, SkillLocation, ModeSkillInfo, ModeSkillStateReason};

pub fn get_skill_registry() -> &'static SkillRegistry;  // line 19
```

## Re-export

```rust
// tools/implementations/mod.rs:81
pub use skill_tool::SkillTool;

// Final consumer path
use northhing_core::agentic::tools::implementations::SkillTool;
```
