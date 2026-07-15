# R27 stage summary — `assembly/core/src/service/workspace/manager.rs` 1505 → facade 8 + 2 sibling

> R27 god-object split **SUCCEEDED** on third take-over attempt.
> manager.rs 1505 → facade 8 + 2 sibling (types 300 + manager_impl 1234).
> All tests green (103 core + 102 consumer crates), 0 errors workspace-wide,
> 0 regressions.

## Result

| Metric | Value |
|---|---|
| manager.rs before | 1505 lines |
| manager.rs after (facade) | 8 lines |
| types.rs (sibling 1) | 300 lines |
| manager_impl.rs (sibling 2) | 1234 lines |
| Total | 1542 (+37 from header use blocks) |
| `cargo check -p northhing-core` | 0 errors |
| `cargo check --workspace` | 0 errors |
| `cargo test -p northhing-core` | 103 passed, 0 failed |
| Consumer crates (5) tests | 102 passed, 0 failed |
| Cargo.lock drift | none |

## Sibling layout (horizontal split)

| File | Lines | Sub-domain |
|---|---|---|
| `types.rs` | 300 | struct/enum + impl Default + impl WorkspaceIdentity + free fn + IDENTITY_FILE_NAME const |
| `manager_impl.rs` | 1234 | impl WorkspaceInfo + WorkspaceSummary struct + WorkspaceManager struct + WorkspaceManagerConfig + impl Default for WorkspaceManagerConfig + impl WorkspaceManager + WorkspaceManagerStatistics struct |

**Horizontal split** chosen over sub-domain split because:
- `impl WorkspaceManager` accesses private fields `workspaces: HashMap<...>` etc.
- Splitting `WorkspaceManager` struct into a different sibling than `impl WorkspaceManager` requires `pub(super)` on every private field (5+ fields) — a behavior change.
- Keeping struct + impl in same sibling preserves original visibility.

## Cross-sibling use

- `types.rs` header: `use super::*;` (mod.rs re-exports via `pub use manager::*;` wildcard — needed to bring in other sibling items)
- `manager_impl.rs` header: same
- mod.rs changed from explicit `pub use manager::{GitInfo, RelatedPath, ...};` to `pub use manager::*;` (wildcard) to re-export `IDENTITY_FILE_NAME` and `WorkspaceWorktreeInfo` which were not in the original explicit list

## Visibility

- `pub(super)` on all impl block `fn`/`async fn` methods (R23 pattern)
- `pub` on `IDENTITY_FILE_NAME` const (was `pub(crate)`) — needed for cross-sibling via `pub use super::types::*;` re-export
- `pub(super) fn default()` was rejected by Rust — `default()` trait method doesn't allow visibility qualifier. Removed `pub(super)` from `impl Default` `fn default()`.

## mod.rs changes

```rust
// Before
pub use manager::{
    GitInfo, RelatedPath, ScanOptions, WorkspaceIdentity, WorkspaceInfo, WorkspaceKind,
    WorkspaceManager, WorkspaceManagerConfig, WorkspaceManagerStatistics, WorkspaceOpenOptions,
    WorkspaceStatistics, WorkspaceStatus, WorkspaceSummary, WorkspaceType,
};
// After
pub use manager::*;
```

This widens the public API but preserves all original re-exports (the listed items are still re-exported). New `IDENTITY_FILE_NAME` and `WorkspaceWorktreeInfo` re-exported.

## Cross-crate consumer verification (R19 lesson applied)

Same 5 consumer crates as R26:
- `northhing-services-integrations` (99 tests)
- `northhing-runtime-services` (3 tests)
- `northhing-agent-runtime` (0 tests)
- `northhing-agent-tools` (0 tests)
- `northhing-product-capabilities` (0 tests)

All 5 compile clean. `pub use manager::*;` wildcard in mod.rs preserves cross-crate API.

## Off-by-one / extraction lessons (R26 lesson applied + new lessons)

1. **f-string `{{` and `}}` literal** in Python: same R26 bug, used raw string in header this time.
2. **Range off-by-one**: my types range (0, 295) was 0-indexed inclusive = L1-L296, but L296 in original is `impl WorkspaceInfo {`. Need (0, 294) to stop at L295. Fixed.
3. **`pub(super)` on `fn default()`**: trait `Default::default()` doesn't allow visibility qualifier. Strip `pub(super)` for `impl Default` blocks.
4. **Stray `//!` at L8**: original manager.rs L1 = `//! Workspace manager.`. After extracting L1-L18 use block, the file still has `//! Workspace manager.` as the first content line. Need to change to `//` (regular comment, not doc comment).
5. **Reading from facade overwrites source**: when re-running extraction, the script reads the (now-facade) manager.rs which has 8 lines, extracting 8 lines of garbage. Need to `git checkout` manager.rs first.
6. **`pub(crate)` items not re-exported via `pub use ...::*`**: only `pub` items are. Changed `pub(crate) const IDENTITY_FILE_NAME` to `pub const`.

## Refs

- R27 strategy: horizontal split (impl+struct in same sibling for private field access) vs R23/R25 sub-domain split
- R26 lesson: f-string `{{` escape
- R23 pattern: `pub(super)` for cross-sibling fn visibility
- R19 lesson: cross-crate consumer verification
- AGENTS.md: `src/crates/assembly/core/AGENTS.md` (workspace/manager.rs is part of `service/`)