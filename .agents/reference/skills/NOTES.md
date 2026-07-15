# Skills Domain — "Do NOT Copy Verbatim" Notes

> **Read this before you copy from `0N-*.rs`.** This file lists the patterns
> that look reusable but are actually legacy, bug-prone, or stub-only.
> Copying them will reintroduce known problems.

## ⛔ Do NOT copy `render_full_skill_listing_body` (v3 fallback)

`skill_agent_snapshot.rs:467` — the v3-style "render all skills in full" path.

It still exists because `USE_SKILL_REGISTRY = true` (line 27) is the coarse
rollback switch and `render_resolved_skill_listing_body` (line 486) falls back
to it when:
- no user prompt provided, OR
- prompt is whitespace-only, OR
- `resolve_for_prompt` returned an empty Vec (no skill hit `MIN_RELEVANCE_SCORE`)

**Do not** use it as a model for new code. The whole point of the v2 resolver
is to avoid rendering the full list. If your new code is "always render full
list", you are re-introducing the v3 problem.

## ⛔ Do NOT confuse `resolver.rs` (v1) with `resolver_v2.rs` (v2)

The "v1/v2" naming is **retconned**. The two files solve different problems:

| File | Problem | Question it answers |
|---|---|---|
| `resolver.rs` (v1) | Availability | "Should this skill be enabled in this mode— " — boolean gate |
| `resolver_v2.rs` (v2) | Relevance | "Given a prompt, which enabled skills are most relevant— " — ranking |

**Do not** collapse them in new code. The "v2" suffix refers to replacing
v3-style "render everything" prompting, **not** replacing `resolver.rs`.

## ⛔ Do NOT modify the legacy cleanup list without coordinating

`builtin.rs:27` — `LEGACY_BUILTIN_SKILL_DIR_NAMES` is a hard-coded list of
**14 Superpowers-era skill directory names** that were bundled in 2026-04 then
removed. `cleanup_legacy_builtin_dirs` will **delete** any of these (plus
`SUPERPOWERS_LICENSE.txt` at `builtin.rs:45`) on first install after upgrade.

If you copy the install path but drop this list, you will leak artifacts on
user systems forever. If you copy it but add new entries, you will silently
delete user data on upgrade.

## ⛔ Do NOT call `resolve_for_prompt` without a fallback

`render_resolved_skill_listing_body` (`skill_agent_snapshot.rs:486`) is the
**only safe wrapper**. It guarantees the agent always sees *some* skill list
(never empty). Calling `resolve_for_prompt` directly and rendering the result
without a fallback will produce an empty listing in edge cases (no prompt,
no matches), which the agent interprets as "no skills exist" — a worse UX
than the full list.

## ⛔ Do NOT use `render_resolved_skill_listing_body` if you need a non-default cap

`render_resolved_skill_listing_body` hard-codes `RESOLVED_SKILLS_MAX = 5` and
ignores the `with_max` variant. If you need a different cap, build `SkillInfo`
yourself and call `resolve_for_prompt_with_max`. Do not edit the wrapper to
add a max parameter — it has other callers.

## ⛔ Do NOT treat `to_xml_desc()`'s `<location>` field as the `SkillLocation` enum

`SkillInfo::to_xml_desc()` at `types.rs:60` uses `<location>{}</location>`
with the **filesystem path** (not the `SkillLocation` enum). This is almost
certainly a long-standing naming bug, but every consumer (prompt renderer,
`Skill` tool description) has been wired to read `path` from this field, so
"fixing" it would break callers. Leave it alone.

## ⛔ Do NOT duplicate `OnceLock<mpsc::Sender<...>>` injection pattern

`SkillRegistry` uses `OnceLock` for its scan dependencies — fine because
scanning is one-shot at startup. The actor design (see
`.agents/reference/actor/`) **should not** copy this for hot-path
messaging; wire at construction instead.

## ⚠️ Synthetic `SkillInfo` in `render_resolved_skill_listing_body`

`skill_agent_snapshot.rs:500-515` builds a synthetic `SkillInfo` from
`SkillSnapshotEntry` with hard-coded `level = User`, `source_slot = ""`,
`dir_name = name`, `is_builtin = false`. The resolver only uses `name` and
`description`, so the synthetic fields are functionally inert — but they
are still constructed on every call. A future refactor can slim this down
without changing behavior. **Do not** rely on the synthetic fields
elsewhere; they are not real registry data.

## ⚠️ `disabled_by_mode: bool` is a backward-compat inverse

`ModeSkillInfo::disabled_by_mode` at `types.rs:103` is "the inverse of
`effective_enabled`". Old API consumers may read this. New code should use
`effective_enabled` directly. The field is kept for the v3 wire format.

## ⚠️ `SkillTool::call_impl` has two syntactically identical routing arms

`sources: skill_tool.rs:251-269` (workspace variant) repeats the same
`if use_stable_key { find_and_load_skill_by_key_for_workspace } else
{ find_and_load_skill_for_workspace }` pattern that the `is_remote()` arm
at lines 205-249 already implements. Cosmetic duplication; no behavior bug.
Do not "consolidate" them with a macro without first running the full
`tools/implementations/skill_tool.rs` test suite.

## ⚠️ `refresh_for_workspace` is a no-op alias

`registry.rs:737` — currently a no-op that just calls `refresh()`. Used to
do something. If you add workspace-scoped caching here, it is a real
behavior change. Mention it in the commit message.

## ⚠️ `scan_skill_candidates_for_workspace` calls `ensure_builtin_skills_installed()` on every scan

`registry.rs:460` — the installer is fast (short-circuits on hash match) but
takes an OS file lock. If you call `get_all_skills_for_workspace` in a hot
path, profile before adding a "skip install" flag.

## ⚠️ `NORTHHING_SYSTEM_SLOT = "northhing-system"` exclusion

`registry.rs:405-407` — installer writes its bundled skills to
`<user_skills>/.system/` but the registry also scans `<user_skills>/` (the
`northhing` user slot, `registry.rs:341-352`). The `.system` directory is
explicitly skipped during that scan; without this guard the registry would
double-count built-ins.

## ⚠️ No desktop/server consumer exists

`apps/cli/src/modes/chat.rs` is the **only** consumer in `apps/`. There is
no `apps/desktop` or `apps/server` integration of the skill system. If you
add a desktop binding, you are writing the first one — there is no
precedent in this repo.

## ✅ Things you SHOULD copy

- `Resolver v2`'s algorithm body (file 05) — clean, tested, and the
 weighted-Jaccard implementation is good to reuse for any future
 "prompt → ranked list" feature.
- `SkillData::from_markdown` (file 01, line 126) — the front-matter parser
 contract. Any new `SKILL.md`-like file should reuse the same
 `name`/`description` required-field shape.
- `SkillLocation` + `SkillInfo` data model (file 01) — this is the public
 data shape exposed to consumers. Don't fork it; extend it.
- `ModeSkillStateReason` enum (file 01) — if you add a new reason, add the
 variant here and update the serializer test, not a new type.
- The mode-policy table pattern (file 03) — declaration-ordered with
 "later wins" semantics. Use this for any future per-mode default table.
