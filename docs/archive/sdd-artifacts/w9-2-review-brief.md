# W9-2 Review Brief

## Position
`E:\agent-project\NortHing` (repo root)

## Diff
`git diff --stat c3adbef..c80227b` — 9 files, +512/-5:
- `src/apps/desktop/src/ui_dioxus/api.rs` (+17)
- `src/apps/desktop/src/ui_dioxus/mod.rs` (+1)
- `src/apps/desktop/src/ui_dioxus/pages_memory.rs` (+347)
- `src/apps/desktop/src/ui_dioxus/registry.rs` (+8)
- `src/apps/desktop/src/ui_dioxus/windows.rs` (+13)
- `src/crates/assembly/core/src/kernel_facade/memory.rs` (+82/-1)
- `src/crates/assembly/core/src/kernel_facade/tests.rs` (+20/-1)
- `src/crates/contracts/kernel-api/src/lib.rs` (+2/-1)
- `src/crates/contracts/kernel-api/src/memory.rs` (+27)

## Task Brief
`.superpowers/sdd/w9-2-memory-panel-brief.md`

## Spec Summary (from brief)
1. **Contract**: FactDto (snake_case serde, snake_case fields) + KernelMemoryApi::list_facts(Option<&str>) + search_facts(&str, Option<&str>, Option<u32>)
2. **Facade**: passthrough to MemoryDb via auto_memory path; no new global state
3. **Desktop API**: list_facts/search_facts wrappers (≤40 lines)
4. **UI**: pages_memory.rs — read-only list, search, JSONL export to `<config_dir>/northhing/exports/memory-<ts>.jsonl`, empty/error states, Chinese UI
5. **Philosophy**: ZERO edit/delete/forget UI — read-only (TH-3 principle 4)
6. **Constraints**: contracts behavior-light; no .superpowers/ in commit; rot-budget no ceiling raise; new files <800 lines

## Cross-task interfaces
- None — standalone module following existing ModuleAppProps pattern
- Depends on existing `api.rs` facade, `registry.rs` module registration, `use_page_shell`

## Resolved ambiguities
- Export path fixed (no rfd dialog, as rfd was removed in W4-1) — confirmed brief §4
- MemoryDb opened per-call (no global handle) — confirmed brief §2 "禁止新造全局句柄"
- Self-window trigger via windows.rs button (existing pattern for other modules)

## Report path
`.superpowers/sdd/w9-2-review.md`
