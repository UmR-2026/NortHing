# R27b stage summary — `manager_impl.rs` 1234 → facade 10 + 3 sibling (QClaw blocker fix)

> R27b god-object sub-domain split. QClaw R27 review 7.5/10 CONDITIONAL
> APPROVE blocker: `manager_impl.rs` 1234 lines (+434 over 800 cap) became
> new god file after R27. R27b sub-domain splits impl WorkspaceManager by
> method category.

## Result

| Metric | Value |
|---|---|
| manager_impl.rs before | 1234 lines |
| manager.rs after (facade, was 7) | 10 lines |
| workspace_info_impl.rs (NEW) | 487 lines (impl WorkspaceInfo + WorkspaceSummary + WorkspaceManager struct + WorkspaceManagerConfig + impl Default) |
| manager_lifecycle.rs (NEW) | 439 lines (impl WorkspaceManager L487-L900: new/rekey/migrate/open/close/set_active/set_current) |
| manager_accessors.rs (NEW) | 363 lines (impl WorkspaceManager L900-L1233: get/list/search/remove/cleanup/recent/statistics + WorkspaceManagerStatistics) |
| types.rs (existing) | 300 lines (impl WorkspaceIdentity) |
| All 4 sibling < 800 cap | ✅ (largest 487) |
| `cargo check -p northhing-core` | 0 errors |
| `cargo check --workspace` | 0 errors |
| `cargo test -p northhing-core` | 103 passed, 0 failed |
| Consumer crates (5) tests | 102 passed, 0 failed |
| Cargo.lock drift | none |

## Key change: `pub(super)` on WorkspaceManager fields

R27b split `impl WorkspaceManager` into 3 separate `impl` blocks in 3 files
(lifecycle / accessors / info). Rust visibility rules say struct fields are
private to the module where the struct is defined. Original fields were
private (no `pub`):

```rust
pub struct WorkspaceManager {
    workspaces: HashMap<...>,                    // private
    opened_workspace_ids: Vec<String>,          // private
    current_workspace_id: Option<String>,       // private
    recent_workspaces: Vec<String>,             // private
    recent_assistant_workspaces: Vec<String>,   // private
    max_recent_workspaces: usize,               // private
}
```

Splitting into sibling files caused 100+ "field `workspaces` of struct
`WorkspaceManager` is private" errors. Fix: add `pub(super)` to all 6 fields
so they're visible to sibling files within the `workspace` module.

```rust
pub struct WorkspaceManager {
    pub(super) workspaces: HashMap<...>,
    pub(super) opened_workspace_ids: Vec<String>,
    pub(super) current_workspace_id: Option<String>,
    pub(super) recent_workspaces: Vec<String>,
    pub(super) recent_assistant_workspaces: Vec<String>,
    pub(super) max_recent_workspaces: usize,
}
```

`pub(super)` keeps fields private to the crate — they're not exposed in
the public API. The struct itself (`pub struct WorkspaceManager`) is still
public; only the fields are now `pub(super)` for cross-sibling use.

This is a behavior change (fields were private in R27, now `pub(super)`).
Acceptable for the cross-sibling split goal.

## Range extraction lessons (R26 + R27 + R27b all hit similar issues)

1. **Off-by-one end**: Python `lines[start:end+1]` is INCLUSIVE end. So `lines[0:296]` = items 0-295 = L1-L296 (296 items). Easy to miscount by 1.

2. **Last `}` missing**: R27b manager_accessors range `lines[899:1233]` was off by 1 — original has 1234 lines, so range end exclusive 1233 misses the LAST `}` at lines[1233] (0-indexed). Fix: `lines[899:1234]`.

3. **impl block needs `impl X {` opening AND `}` closing**: When splitting a single `impl WorkspaceManager { ... }` block across multiple files, each file needs:
   - `impl WorkspaceManager {` opening (manually added if not in content)
   - `}` closing (manually added if not in content)

   R27b: manager_lifecycle.rs had `impl WorkspaceManager {` in content (L487) but missing `}` close (which is at L1222 in original, far past content end). Fixed by appending `}\n` to content. manager_accessors.rs had NO `impl WorkspaceManager {` opening (content starts at L900 mid-block). Fixed by prepending `impl WorkspaceManager {\n`.

4. **Reading from facade overwrites source**: R27b's first attempt read from manager_impl.rs which was already 8-line facade from R27. Caused empty content. Fix: `git checkout` first.

## Cross-sibling use

- `workspace_info_impl.rs`: `use super::types::*;` (uses types sibling: WorkspaceType, WorkspaceStatus, etc.)
- `manager_lifecycle.rs`: `use super::types::*; use super::workspace_info_impl::*;` (uses WorkspaceManager struct)
- `manager_accessors.rs`: `use super::types::*; use super::workspace_info_impl::*; use super::manager_lifecycle::*;` (uses everything)

## Cross-crate consumer verification (R19 lesson)

Same 5 consumer crates pass:
- `northhing-services-integrations` (99 tests)
- `northhing-runtime-services` (3 tests)
- `northhing-agent-runtime` (0 tests, 0 errors)
- `northhing-agent-tools` (0 tests, 0 errors)
- `northhing-product-capabilities` (0 tests, 0 errors)

## Refs

- R27 stage summary: `docs/handoffs/2026-07-02-r27-stage-summary.md`
- R27 review report (QClaw 7.5/10 CONDITIONAL): `docs/reviews/round27-qclaw-review.md`
- R27 review report (Kimi 9.2/10 APPROVE): user verbal
- R26 interface crate pattern: `docs/handoffs/2026-07-02-r26-stage-summary.md`
- R23 sub-domain split pattern: `docs/handoffs/2026-07-02-r23-stage-summary.md`