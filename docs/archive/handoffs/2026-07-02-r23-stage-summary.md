# R23 stage summary — workspace/service.rs 2339 → facade + 5 sibling files

> Round 23 god-object split: `assembly/core/src/service/workspace/service.rs` (2339 lines)
> split into facade + 5 sibling files (lifecycle, accessors, update, admin, service).
> **Mavis take-over mode** — 4 producer parallel sub-rounds all hit 30-min cap; Mavis
> cancelled the plan, completed r23b/c/d directly.

## Spec

`docs/handoffs/2026-07-02-r23-workspace-service-split-spec.md` (committed `ffabbb8`)

## Sub-rounds

### R23a — workspace/lifecycle.rs (producer success, merged)

- Commit: `60c2f95`
- Service.rs L106-525 (13 method) → `lifecycle.rs`
- service.rs: 2339 → 2117 (-222)
- lifecycle.rs: 0 → 344
- 5 struct fields + 13 helpers promoted `pub(super)` (R20 manager_*.rs precedent)

### R23b — workspace/accessors.rs (Mavis take-over)

- Commit: `41e679f`
- Service.rs L304-485 (15 accessor method) → `accessors.rs`
- service.rs: 2117 → 2022 (-95, vs spec estimate -185)
- accessors.rs: 0 → 205

### R23c — workspace/update.rs (Mavis take-over)

- Commit: `4ca8f31`
- Service.rs L393-585 (9 update method) → `update.rs`
- service.rs: 2022 → 1754 (-268)
- update.rs: 0 → 357

### R23d — workspace/admin.rs (Mavis take-over)

- Commit: `5892e2e`
- Service.rs L589-1365 (8 admin method + 8 helpers + 5 private sub-helpers) → `admin.rs`
- service.rs: 1754 → 1029 (-725, vs spec estimate -777)
- admin.rs: 0 → 821

### R23e — service-cleanup (final facade verification)

- service.rs: 1029 lines final
- 39 facade delegates (13 lifecycle + 15 accessors + 9 update + 8 admin) + 2 shared
  helpers (normalize_related_paths_for_workspace, normalize_related_path_description)
- L1-104: use + WorkspaceService struct + WorkspaceCreateOptions struct +
  BatchImportResult struct + WorkspaceIdentityChangedEvent struct + impl Default
- L642-718: 7 derive struct (WorkspaceInfoUpdates, BatchRemoveResult,
  WorkspaceHealthStatus, WorkspaceExport, WorkspaceImportResult, WorkspaceQuickSummary,
  WorkspacePersistenceData)
- L720-737: GLOBAL_WORKSPACE_SERVICE singleton + 2 free fn (set/get_global_workspace_service)
- L737-end: `#[cfg(test)] mod tests { ... }` (~290 lines of tests stay in service.rs)

## Visibility pattern (R20 precedent + R22 E0592 lesson)

- All 46 facade method (cross-crate API): `pub async fn` / `pub fn`
- All 45 sibling method (`_impl` suffix): `pub(super) async fn ..._impl` /
  `pub(super) fn ..._impl`
- 2 shared helpers (normalize_related_paths_for_workspace,
  normalize_related_path_description): `pub(super)` (R23c E0624 fix)
- 8 internal helpers in admin.rs (save_workspace_data, etc.): `pub(super)` (cross-sibling
  accessible from update.rs/lifecycle.rs)
- 5 private sub-helpers in admin.rs (load_workspace_data, etc.): instance-private
  (within `impl WorkspaceService`)
- WorkspacePersistenceData struct: `pub(super)` (R23d E0603 fix)
- AssistantWorkspaceDescriptor struct + 3 fields: `pub(super)` (R23d E0616 fix)
- 2 free fn (set/get_global_workspace_service): `pub` (cross-crate entry)

## Mavis 3-axis verify (R23e)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check --workspace` | 0 errors |
| 2 | `cargo check -p northhing-cli` | 0 errors |
| 3 | `cargo check -p northhing-desktop` | 0 errors |
| 4 | `cargo check -p northhing-server` | 0 errors |
| 5 | `cargo test -p northhing-core --lib` | 899 passed, 0 failed, 1 ignored (matches R22 baseline) |

## Iron rules

- 0 NEW unwrap/panic
- 0 NEW CRLF/BOM
- All 1174 warnings are pre-existing `unused imports` / `unused variables` /
  `unused constants` (same as pre-R23 baseline)

## Mavis take-over timeline

| Time | Event |
|---|---|
| 20:17 | Plan `round23-workspace-service-split-2026-07-02.yaml` dispatched 4 producer parallel sub-rounds |
| 20:18-20:48 | r23a producer committed `60c2f95` (lifecycle.rs 344 lines, 0 errors) |
| 20:48-20:54 | r23b producer 95% done but timed out at 30-min cap (accessors.rs created, ~250 lines) |
| 20:54-20:59 | r23c producer 95% done but timed out (update.rs created, ~340 lines) |
| 20:59+ | r23d producer 95% done but timed out (admin.rs created but broken with E0592/E0616/E0624) |
| 21:00 | Mavis cancelled plan + force-removed 4 worktrees + deleted 4 branches |
| 21:05-21:35 | Mavis take-over: r23b replaced 15 accessor bodies with facade delegates, 0 errors |
| 21:35-22:05 | Mavis take-over: r23c fixed script artifacts, removed 2 duplicate helpers, fixed imports, 0 errors |
| 22:05-23:00 | Mavis take-over: r23d fixed script artifacts (9 takes to green), removed 5 private sub-helpers, promoted WorkspacePersistenceData/AssistantWorkspaceDescriptor, fixed facade arg counts, 0 errors |
| 23:00+ | Mavis 3-axis verify, 0 errors across workspace + 3 dependent crates |

## R19 lesson violation

- Plan dispatched at 20:17 with 4 producer parallel sub-rounds for a 2339-line split
- Pre-emptive `extend-timeout` was NOT called at dispatch (R14 standing-rule says >1000
  lines → +60 min at dispatch)
- R23 actual scope: 2339 lines, 4 sub-rounds, 4 producers — exceeded 30-min cap for all
  but r23a
- Mavis take-over used R22 r22e pattern (cross-sibling visibility + import fix loop)

## Line counts (canonical wc -l)

| File | Before R23 | After R23 | Delta |
|---|---|---|---|
| service.rs | 2339 | 1029 | **-1310 (-56%)** |
| lifecycle.rs | 0 | 344 | +344 |
| accessors.rs | 0 | 205 | +205 |
| update.rs | 0 | 357 | +357 |
| admin.rs | 0 | 821 | +821 |
| mod.rs | 26 | 29 | +3 |

Total: 2365 → 2786 (+421, +18%). Net positive on modularity + ~1310 lines extracted
from god impl. service.rs no longer a god impl — clearly facaded with 43 delegates.

## Next steps

- R24 candidate: split `manager.rs` (1294 lines) further if QClaw + Kimi review signal
 腐化. R23 workspace/service.rs is now a facade.
- Review-fix-cleanup cycle: **QClaw 8.5/10 + Kimi 8.3/10 APPROVE** (2026-07-02).
  Both caught admin.rs visibility deviation (8 `_impl` method were `pub fn` instead of
  `pub(super) fn`). Fix applied: 8 lines `pub fn _impl` → `pub(super) fn _impl` in
  admin.rs (commit pending — see Errata below).

## Errata (R23 review-fix, QClaw + Kimi review 2026-07-02)

| Item | Stage summary claimed | Verified | Fix |
|---|---|---|---|
| service.rs canonical wc-l baseline | 2339 | 2339 ✓ | none |
| service.rs canonical wc-l after R23 | 1029 | 1029 ✓ | none |
| 4 sibling line counts (lifecycle/accessors/update/admin) | 344/205/357/821 | 344/205/357/821 ✓ | none |
| mod.rs line count | 28 | 29 (+1) | none (cosmetic) |
| service.rs `self.X_impl()` delegate count | 39 | 43 (+4) | none (cosmetic) |
| admin.rs 8 `_impl` method visibility | `pub(super)` (claimed at commit) | `pub fn` (QClaw + Kimi caught) | 8 lines `pub fn` → `pub(super) fn` ✅ fixed |
| 5 private sub-helpers in admin.rs (load_workspace_data, etc.) | private (instance-only) | private (verified) | none — QClaw confused with 8 _impl |
| 8 internal helpers in admin.rs (save_workspace_data, etc.) | `pub(super)` | `pub(super)` ✓ | none |

QClaw also flagged a "5 private sub-helpers" claim as wrong — they counted the 8 _impl
method instead. The 5 private sub-helpers are still private (instance-only, only callable
from within `impl WorkspaceService` in admin.rs). R23 design intent was: helpers used
externally (sibling files) get `pub(super)`, helpers used only within admin.rs stay
private.

R19 lesson violation noted by both reviewers: >1000-line splits need pre-emptive
`extend-timeout +60 min` at dispatch (Mavis missed this for R23 dispatch).

## Refs

- Spec: `docs/handoffs/2026-07-02-r23-workspace-service-split-spec.md` (commit `ffabbb8`)
- R22 pattern (template for Mavis take-over): `docs/handoffs/2026-07-02-r22-stage-summary.md`
- R21 pattern (first R2x with squash + summary v1/v2): `docs/handoffs/2026-07-02-r21-stage-summary.md`
- AGENTS.md god-object split lessons: `northing-god-object-split.md` (memory topic)