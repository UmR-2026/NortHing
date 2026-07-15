# Skills Domain — Reference

> Skill / Registry / Loader code mirrors. Read [`SIGNATURES.md`](./SIGNATURES.md) first,
> then [`NOTES.md`](./NOTES.md) for "do NOT copy" warnings, then the specific
> `NN-*.rs` mirror.

## What this domain contains

The northhing skill system is **not** a trait-based plugin loader. It is a
registry-of-markdown-files that:

1. Scans `SKILL.md` files across well-known slots (user-level, project-level,
   northhing's own user dir, `.system` built-in dir).
2. Parses YAML front-matter (`name`, `description`).
3. Resolves shadowing by `name + priority`.
4. Applies a **mode-aware enablement policy** (which skills are on by default
   in which `SkillModeId`).
5. On demand, **filters per-turn** by keyword overlap with the user's prompt
   (resolver v2, weighted Jaccard).

## File ordering — read in this sequence

| # | File | Why |
|---|---|---|
| 01 | [`01-skill-types.rs`](./01-skill-types.rs) | `SkillInfo`, `SkillData`, `ModeSkillInfo`, `ModeSkillStateReason` — every other file builds on these. |
| 02 | [`02-skill-catalog.rs`](./02-skill-catalog.rs) | The 24 built-in skills + group taxonomy. Touch this only when adding/removing built-ins. |
| 03 | [`03-skill-policy.rs`](./03-skill-policy.rs) | Mode-by-mode enablement table. Pure functions, no I/O. |
| 04 | [`04-skill-resolver-v1.rs`](./04-skill-resolver-v1.rs) | The "should this skill be enabled in this mode?" boolean gate. **Different from v2.** |
| 05 | [`05-skill-resolver-v2.rs`](./05-skill-resolver-v2.rs) | **★ Headline algorithm.** Weighted Jaccard: name 2×, description 1×. The on-demand resolver. |
| 06 | [`06-skill-builtin-installer.rs`](./06-skill-builtin-installer.rs) | How built-in skills are embedded at compile-time and synced to `<user_skills>/.system/`. |
| 08 | [`08-registry-full.rs`](./08-registry-full.rs) | `SkillRegistry` (1050 lines) — full mirror. Public methods at 286-1015. |
| 10 | [`10-skill-tool-full.rs`](./10-skill-tool-full.rs) | `SkillTool` (632 lines) — full mirror. Stable-key routing at 251-269. |
| 11 | [`11-skill-agent-snapshot-full.rs`](./11-skill-agent-snapshot-full.rs) | Snapshot adapter (693 lines) — full mirror. **★ The fallback chain at 467-541.** |

## Selection guide — which file to copy when you need to

| You're trying to… | Start with |
|---|---|
| Add a new built-in skill | 02 (catalog) + 03 (policy default) + 06 (install path) |
| Add a new mode | 03 (policy table) + 04 (default-effect lookup) |
| Change relevance scoring | 05 (resolver v2) — only file with the algorithm |
| Add a new override source (e.g. team-level) | 04 (resolver v1) — that's the override layer |
| Change how skills are discovered at scan time | (registry.rs in src — too large to mirror; read directly) |
| Add a new consumer of the skill system | 04 + 05 (the two public entry points) |

## Public API surface (consumer entry points)

```rust
use northhing_core::agentic::tools::implementations::skills::*;

// Read all skills available in this workspace
let skills: Vec<SkillInfo> = get_skill_registry()
    .get_all_skills_for_workspace(workspace_root);

// Filter by mode
let mode_skills: Vec<ModeSkillInfo> = get_skill_registry()
    .get_mode_skill_infos_for_workspace(workspace_root, mode_id);

// On-demand: top-K most relevant for a prompt
let resolved: Vec<ResolvedSkill> = resolve_for_prompt(prompt, &skills);

// Load the actual content of one skill
let data: SkillData = get_skill_registry()
    .find_and_load_skill_for_workspace(name, workspace_root, agent_type)?;
```

## What lives OUTSIDE this mirror

`src/crates/assembly/core/src/agentic/tools/implementations/skills/registry.rs`
(1050 lines) is **not** mirrored here — too large and most of it is slot-table
boilerplate. Read the source file directly when extending the registry.
