# R21a: restore.rs revival impl handoff

**Date**: 2026-07-02
**Branch**: `impl/r21a-restore-revival` (worktree `E:/agent-project/northing-impl-r21a-restore-revival`)
**Commit**: `1cbf0b2 refactor(assembly-core): R21a restore.rs revival (12 restore_* methods mod.rs L1426-1570 -> restore.rs)`
**Parent commit**: `1a69a82 docs(spec): R21 dialog_turn/mod.rs 1653 -> facade ~700 + 4 sibling parallel split`
**Spec reference**: `docs/handoffs/2026-07-02-r21-dialog-turn-mod-split-spec.md` §2.1 r21a restore-revival

## Summary

Revived `dialog_turn/restore.rs` (2-line placeholder from R6) by migrating 12 `restore_*` methods from `mod.rs` L1425-1568. The sibling file is now 167 canonical lines (under the 242 QClaw R20 cap). `mod.rs` shrank from 1653 → 1644 canonical lines (delta -9).

## Per-method mapping table

| # | Method name | mod.rs (BEFORE) | restore.rs (AFTER, line) | Visibility | Body preserved verbatim |
|---|---|---|---|---|---|
| 1 | `restore_session` | L1426-1434 | `restore.rs:33` (`restore_session_impl`) | `pub(super) async fn` | yes |
| 2 | `restore_internal_session` | L1436-1444 | `restore.rs:43` (`restore_internal_session_impl`) | `pub(super) async fn` | yes |
| 3 | `restore_session_with_turns` | L1447-1455 | `restore.rs:54` (`restore_session_with_turns_impl`) | `pub(super) async fn` | yes |
| 4 | `restore_internal_session_with_turns` | L1457-1465 | `restore.rs:65` (`restore_internal_session_with_turns_impl`) | `pub(super) async fn` | yes |
| 5 | `restore_session_view` | L1468-1476 | `restore.rs:76` (`restore_session_view_impl`) | `pub(super) async fn` | yes |
| 6 | `restore_session_view_timed` | L1478-1490 | `restore.rs:87` (`restore_session_view_timed_impl`) | `pub(super) async fn` | yes |
| 7 | `restore_session_view_tail` | L1492-1501 | `restore.rs:99` (`restore_session_view_tail_impl`) | `pub(super) async fn` | yes |
| 8 | `restore_session_view_tail_timed` | L1503-1517 | `restore.rs:110` (`restore_session_view_tail_timed_impl`) | `pub(super) async fn` | yes |
| 9 | `restore_internal_session_view` | L1519-1527 | `restore.rs:122` (`restore_internal_session_view_impl`) | `pub(super) async fn` | yes |
| 10 | `restore_internal_session_view_timed` | L1529-1541 | `restore.rs:133` (`restore_internal_session_view_timed_impl`) | `pub(super) async fn` | yes |
| 11 | `restore_internal_session_view_tail` | L1543-1552 | `restore.rs:145` (`restore_internal_session_view_tail_impl`) | `pub(super) async fn` | yes |
| 12 | `restore_internal_session_view_tail_timed` | L1554-1568 | `restore.rs:156` (`restore_internal_session_view_tail_timed_impl`) | `pub(super) async fn` | yes |

## mod.rs L1425-1559 diff summary

12 method bodies replaced with single-line facade delegates. The new body for each method is:

```rust
self.<method_name>_impl(args).await
```

Example:
```rust
pub async fn restore_session(
    &self,
    workspace_path: &Path,
    session_id: &str,
) -> NortHingResult<Session> {
    self.restore_session_impl(workspace_path, session_id).await
}
```

`diff --stat`: `2 files changed, 182 insertions(+), 26 deletions(-)` (restore.rs: +165 added (was 2 lines, now 167). mod.rs: +17 net (43-26 = +17, but the actual total moved 165 lines out and 156 lines back in for facade signatures).

## Iron rules compliance (Kimi Bug 3 protocol)

### Unwrap / panic / unreachable baseline

| Source | unwrap | panic! | unreachable! |
|---|---|---|---|
| `restore.rs` HEAD baseline (2 lines placeholder) | 0 | 0 | 0 |
| `restore.rs` after | 0 | 0 | 0 |
| **delta** | **0** | **0** | **0** |
| `mod.rs` L1426-1568 (12 method bodies, BEFORE) | 0 | 0 | 0 |
| `mod.rs` L1425-1559 (12 facade delegates, AFTER) | 0 | 0 | 0 |

No new unwrap/panic/unreachable introduced.

### BOM / CRLF

| File | BOM | CRLF |
|---|---|---|
| `restore.rs` | ❌ none | ❌ none |
| `mod.rs` | ❌ none | ❌ none |

Confirmed via Python `file.read()` byte inspection.

### Long lines (>120 char)

| File | BEFORE | AFTER | delta |
|---|---|---|---|
| `restore.rs` | 0 | 0 | 0 |
| `mod.rs` (full file) | 5 | 5 | 0 |

No new long lines added.

## Cross-crate consumer verification (R19 lesson — mandatory)

| Cargo check | Result |
|---|---|
| `cargo check -p northhing-core --features product-full --lib --message-format=short` | ✅ 0 errors (28.83s, cached) |
| `cargo check -p northhing-cli --message-format=short` | ✅ 0 errors (2m 26s) |
| `cargo check --workspace --message-format=short` | ✅ 0 errors (5m 35s) — includes `northhing` desktop binary, `northhing-server`, `northhing-relay-server` |

Spec mentions `northhing-desktop` but actual crate name is `northhing` (apps/desktop binary). Workspace check covers it.

## Spec drift / deviation log

### Deviation 1: Rust does NOT allow same-name methods in two inherent impl blocks

**Spec claim (R21a instructions §"Target structure")**:
> CRITICAL: Rust allows 2 `impl ConversationCoordinator` blocks (mod.rs facade + restore.rs sibling) with same method names — inherent dispatch picks the local block first, falls back to inherent methods on `self`. This matches R20a pattern exactly.

**Reality**: Rust explicitly **forbids** this. Compile errors:
- `error[E0592]: duplicate definitions with name 'restore_session'`
- `error[E0034]: multiple applicable items in scope`

12 errors of each kind at the first cargo check attempt.

**Resolution**: Appended `_impl` suffix to all 12 sibling method names per R21 spec §2.4 r21d precedent (which itself documents the same conflict and uses `_impl` / `_inner` for facade delegation). This is the same pattern my sibling agent (r21c) discovered and used `_inner` suffix.

### Deviation 2: Spec line-count estimate

Spec estimated restore.rs ~250 lines. Actual: 167 lines (under 242 QClaw R20 cap). The 12 method bodies are short (3-9 lines each), so total came in below estimate.

### Deviation 3: Crate name spelling

Spec uses `northhing-cli` and `northhing-desktop`. Actual: `northhing-cli` (apps/cli) and `northhing` (apps/desktop binary). Used actual names.

## Visibility rationale

| Layer | Visibility | Reason |
|---|---|---|
| `restore.rs` sibling methods | `pub(super) async fn ..._impl` | Sibling access within assembly-core crate; matches R20 manager_*.rs precedent (`pub(super)` for cross-sibling). |
| `mod.rs` facade methods | `pub async fn ...` | Cross-crate consumers (northhing-cli, northhing, northhing-server) expect `pub` API to remain unchanged. |

## Notes for Mavis squash-merge

- ✅ All 12 methods migrated with bodies preserved verbatim
- ✅ 0 NEW unwrap/panic/unreachable
- ✅ 0 BOM/CRLF contamination
- ✅ 0 long lines added
- ✅ All 3 cargo check commands clean (northhing-core, northhing-cli, workspace)
- ✅ Single atomic commit (1cbf0b2)
- ✅ Spec deviation 1 explicitly disclosed with E0592 evidence
- ✅ Branch ready to merge into main via fast-forward or merge commit

### Merge order per R21 spec §6.1 / §7 E5

Spec recommends merge order: r21d → r21b → r21a → r21c. r21a's edit zone is mod.rs L1426-1570 (overlaps with no other producer's zone per spec §4.1). If merge conflicts arise, Mavis can resolve mechanically since the 4 zones don't overlap.

## File state

```
$ git -C E:/agent-project/northing-impl-r21a-restore-revival log --oneline -2
1cbf0b2 refactor(assembly-core): R21a restore.rs revival (12 restore_* methods mod.rs L1426-1570 -> restore.rs)
1a69a82 docs(spec): R21 dialog_turn/mod.rs 1653 -> facade ~700 + 4 sibling parallel split
```

```
$ git -C E:/agent-project/northing-impl-r21a-restore-revival diff --stat HEAD~1
 src/crates/assembly/core/src/agentic/coordination/dialog_turn/mod.rs      |  43 +++---
 src/crates/assembly/core/src/agentic/coordination/dialog_turn/restore.rs   | 165 +++++++++++++++++++++
 2 files changed, 182 insertions(+), 26 deletions(-)
```

```
$ wc -l restore.rs mod.rs
  167 restore.rs   (was 2)
 1644 mod.rs       (was 1653)
```

## Owner follow-ups

- **Mavis squash-merge**: branch is ready, single atomic commit, no further producer work needed
- **Mavis review-cycle**: spec §6 says user-driven QClaw + Kimi review before squash; this handoff doc follows R20 patterns (Kim Bug 3 protocol, cross-crate verification)
- **Sibling branches**: r21b, r21c, r21d running in parallel; mod.rs edit zones don't overlap with r21a's L1425-1559 range