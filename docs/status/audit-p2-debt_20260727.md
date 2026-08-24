# P2 Tech Debt Audit Report

> **Date**: 2026-07-27
> **Auditor**: Automated audit (subagent)
> **Scope**: All P2 entries in `docs/status/tech-debt-ledger.md`
> **Method**: git log + grep + actual file inspection + boundary checker execution

## Summary

14 P2 entries audited. 7 resolved (verified), 7 active. **1 ledger discrepancy found**: P2-9 is marked `resolved` but has a 1-violation regression introduced after the resolution commit. CI integration (stage 3) remains unaddressed for P2-9.

## Audit Results

| P2-ID | Ledger Status | Actual Status | Needs Update? | Evidence |
|-------|---------------|---------------|---------------|----------|
| P2-1 | active | active (partial improvement) | Yes — release workflow now exists | `cli-package.yml` builds & uploads CLI binary to GitHub Releases (cross-platform matrix). **But**: doctor still has 2 entry points (`acp_cli::print_doctor` + `management::print_doctor`), no actual connection tests added. Release artifact portion = resolved; doctor unification = still active. |
| P2-2 | active | active | No | grep `single.*instance\|lock.*file\|already.*running` in `src/apps/desktop/` returns zero relevant matches. No single-instance lock mechanism exists. |
| P2-3 | active | active | No | `ContextCompressionStarted/Completed/Failed` events defined in `contracts/events/src/agentic.rs:184-207`. Desktop `event_bridge.rs` has zero `Compression` matches. CLI `run.rs` has zero. CLI `tool_cards.rs` has display-only handling (tool card label), not event handling. |
| P2-4 | active | active | No | `CleanupService` struct at `cleanup.rs:49`, `cleanup_all` at `:59`, but **zero instantiation** anywhere in `src/` (grep confirmed — no code creates `CleanupService::new` or calls it). `spawn_cleanup_task` in session_manager handles expired sessions only, not file cleanup. |
| P2-5 | active | active | No | `persist_failed_dialog_turn` at `turn_persist.rs:176` emits `DialogTurnFailed` event and marks turn status as Error, but **does not insert failure reason as system message** in conversation history. Desktop `event_bridge.rs:95` calls `set_session_error` (temporary banner), not persisted to message list. After refresh, failure is invisible. |
| P2-6 | active | active | No | `queue.rs:85-87`: when `queue.len() >= max_queue_size (10000)`, logs `warn!` and `return Ok(event_id)` — silently drops event, returns success. `queue.rs:227`: `StreamEventSink::enqueue` does `let _ = EventQueue::enqueue(...)` — ignores return entirely. Critical events (e.g. `DialogTurnFailed`) can be silently lost. |
| P2-7 | active | active | No | `tests_cancel.rs:7-12` self-documents the assumption: "With the dev environment's missing LLM, the spawned task fails at `init_turn` in microseconds." No fake AI backend injected. Tests still environment-sensitive. |
| P2-8 | resolved | resolved (verified) | No | `kernel_facade/mod.rs` is now **62 lines** (was 2213). Split into 14 files per `b15ad46` + `792ff8d`. ✓ |
| P2-9 | resolved | **nearly resolved — 1 regression** | **Yes** | Ledger says "violations cleared to 0" per `d621b29`. Verified 2026-07-27: `runCoreBoundaryCheck()` produces **1 violation**: `Cargo.toml:1: workspace crate member must use an approved layered path: src/crates/services/debug-log`. This is a regression introduced by commit `6eb6209` (K4a closeout, added `debug-log` crate without registering it in `crate-layout.mjs`). **CI integration (stage 3) not done**: grep of all `.github/workflows/*.yml` for `core.boundaries\|checker.mjs\|boundary.check` returns zero matches. |
| P2-10 | resolved | resolved (verified) | No | 2/2 >1000 files split: `settings.rs` → `settings/` (6 files), `callbacks_settings.rs` → `callbacks_settings/` (6 files). 3/3 >800 files have `// allow-god-file`: `theme.rs` (855L), `callbacks_lifecycle.rs` (832L), `judge_gate/mod.rs` (822L). No unregistered >800 files found in `src/`. ✓ |
| P2-11 | resolved | resolved (verified) | No | `receipt_store.rs` exists (95 lines). Append-only JSONL at `data_dir/judge-gate/consumed_receipts.jsonl`. LazyLock init replays log. Persist on consume/release. ✓ |
| P2-12 | resolved | resolved (verified) | No | `forbidden-rules.mjs:2972-2989` has `read_episodes` and `episodes::store::read` forbidden under `agentic/agents/` and `agentic/execution/`. Structural guard in place. ✓ |
| P2-13 | resolved | resolved (verified) | No | `agentic_mode.md` at `src/crates/assembly/core/src/agentic/agents/prompts/`. Identity section no longer contains "not an IDE / not a coding tool" contradiction. Self-cognition design separated to `docs/archive/design/2026-07-23-self-cognition/first-entry-design.md`. ✓ |
| P2-14 | active (low priority) | active (low priority) | No | `facts.rs:113` `append_facts_dedup` uses exact-text dedup (`contains(&c.text)` at `:773`). Comment at `:594`: "Append with exact-text deduplication". Confidence still always Med, scope still always Workspace. No normalization or similarity-based dedup implemented. |

## Discrepancies Detail

### P2-9: Ledger says resolved, but 1 regression violation exists

**Ledger status**: `resolved` (updated at HEAD `36ba7f8`, 2026-07-27)
**Actual status**: 1 violation remains

**Root cause**: Commit `6eb6209` (K4a closeout — debug-log crate) added `src/crates/services/debug-log` as a workspace member, but did not register it in `scripts/core-boundaries/rules/crate-layout.mjs`. The crate-layout rules only list `services-core`, `services-integrations`, and `terminal` under the `services` layer.

**Fix needed**: Add `{ crateName: 'debug-log', layer: 'services', path: 'src/crates/services/debug-log' }` to `crateLayoutLayerNames` in `crate-layout.mjs`.

**Additional gap**: Stage 3 (CI integration) explicitly noted as "not yet done" in the ledger itself. The checker is not referenced in any workflow file (`ci.yml`, `nightly.yml`, `release-please.yml`, `cli-package.yml`, `desktop-package.yml`).

### P2-1: Ledger says active, but release artifact portion is now resolved

The ledger status is `active` with note "(CLI is frozen surface)". However, a full CLI release workflow (`cli-package.yml`) now exists with cross-platform matrix builds, SHA256 checksums, and GitHub Release upload. The **doctor unification** portion remains active (2 entry points, no connection tests).

**Suggested ledger update**: Split P2-1 into two sub-items:
- P2-1a (release artifact): **resolved** — `cli-package.yml` exists
- P2-1b (doctor unification): **active** — 2 entry points remain

## CI Integration Status

| Check | In CI? | Evidence |
|-------|--------|----------|
| Boundary checker (`checker.mjs`) | **No** | Zero matches for `core.boundaries\|checker.mjs\|boundary.check` across all 5 workflow files |
| Self-test (`self-test.mjs`) | **No** | Not in any workflow or `package.json` scripts |
| CLI package build | Yes | `cli-package.yml` — triggered on release published + manual dispatch |
| Desktop package build | Yes | `desktop-package.yml` — same trigger pattern |
| Nightly | Yes | `nightly.yml` — desktop + CLI builds |

**Recommendation**: Add boundary checker to `ci.yml` as a required check. The checker function `runCoreBoundaryCheck()` must be invoked (currently exported but not called when running `node scripts/core-boundaries/checker.mjs` directly — the file has no entry-point invocation).

## Verification Commands

```powershell
# Boundary checker (must call exported function — direct node run produces no output)
Set-Location E:\agent-project\northing
node -e "import('./scripts/core-boundaries/checker.mjs').then(m => m.runCoreBoundaryCheck())"
# Output: "Core boundary check failed.\nCargo.toml:1: workspace crate member must use an approved layered path: src/crates/services/debug-log"

# God file scan
Get-ChildItem -Path src/ -Recurse -Filter *.rs | ForEach-Object { $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines; if ($lines -gt 800) { [PSCustomObject]@{File=$_.FullName; Lines=$lines} } } | Sort-Object Lines -Descending
# Output: 3 files, all with allow-god-file comments

# CI workflow scan
Get-ChildItem -Path .github/workflows/ -Filter *.yml | ForEach-Object { Select-String -Path $_.FullName -Pattern "core.boundaries|checker\.mjs" }
# Output: (none)
```

## Conclusion

- **7/14 P2 items resolved** (P2-8, P2-10, P2-11, P2-12, P2-13 verified clean; P2-1 release artifact resolved but doctor unification active)
- **7/14 P2 items active** (P2-1 doctor, P2-2, P2-3, P2-4, P2-5, P2-6, P2-7, P2-14)
- **1 discrepancy**: P2-9 marked resolved but has 1 regression violation (`debug-log` crate not in layout rules)
- **CI gap**: Boundary checker not wired into any CI workflow; checker script lacks entry-point invocation when run directly
