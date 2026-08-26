# Wave-Level Final Review — 9-Task Audit-Fix Wave

**Base:** `66f08d1` · **Head:** `bbfe1de` · 10 commits · 28 files · +1,020 / −112

---

## Verdict

**CAN MERGE**

No Critical or Important findings. Wave-level seams are clean. The fix is internally consistent and production-safe.

**Finding counts by severity:**
| Severity | Count | Blocking? |
|---|---|---|
| Critical | 0 | No |
| Important | 0 | No |
| Minor | 2 | No |

---

## Findings

### Minor

**M-1** `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:1` — File is 1,009 lines, exceeding the 1,000-line god-file ceiling. No `// allow-god-file` justification comment at the top. The rot budget decreased (1,011 → 1,009) per the ledger, but the house rule requires either a split OR a justification comment once a file crosses 1,000 lines. Neither is present.

**M-2** Ledger calibration — Task I8's row lists the commit range as `8fc51bc..c48e4a9`. `git diff --stat 8fc51bc..c48e4a9` confirms it produces the correct single-file TOPMOST removal, but notationally the base commit (`8fc51bc`) is the I9 docs update, not an I8 ancestor. The range is functionally correct; the entry would be clearer as `c48e4a9` alone (single-commit task) or parent-of-c48e4a9.

---

## Seam-Check Results

### Seam 1 — C1 `enqueue`→`Result` vs I3 `let _ = enqueue()` (sub_handle_out.rs:353)

**Result: Clean.**

The I3 reorder places `let _ = event_queue.enqueue(completed_event.clone(), None).await` inside the spawned task. `DialogTurnCompleted` carries `Normal` default priority (verified in `events/src/agentic.rs:709`). The production desktop initializes the queue with `heap_enabled: false` (lifecycle.rs:100), making `enqueue` unconditionally return `Ok(_)`. The discard is therefore safe in the production path used by the desktop host. The Result return type preserves the rejection contract for CLI/server consumers that run with `heap_enabled: true`. At the wave level this seam holds: C1's fix prevents the stale-heap problem; I3's placement is correct (event enqueued after persistence, before growth finalize). The `let _ =` pattern is already approved by the task-level review.

### Seam 2 — I2 session-state tolerance vs C1 queue semantics

**Result: Clean.**

I2's `list_sessions` tolerance (corrupt → Idle fallback, per-workspace Err isolation) operates on session metadata deserialization. C1's queue rejection operates on enqueue-time heap capacity. These are orthogonal: a corrupted `state.json` doesn't generate or suppress a `DialogTurnCompleted` event, and an event's en queueability doesn't depend on session state. No combined failure mode exists where a poisoned session plus a full queue compounds to lose a Critical event.

### Seam 3 — I4+I5 process cleanup vs I9 runtime helper (shutdown ordering)

**Result: Clean.**

I4+I5's Drop impls (LSP `start_kill`, MCP `spawn_child_process_tree_cleanup`) operate on `std::process::Child` handles. I9's `build_ui_callback_runtime` creates tokio runtimes for UI callback background threads. These are independent resource domains: child-process handles are owned by LSP/MCP structs; tokio runtimes are scoped to individual `std::thread::spawn` closures. No shared state, no ordering dependency at shutdown.

### Seam 4 — I8 drop TOPMOST vs I9 callbacks_lifecycle

**Result: Clean.**

I8 removes `HWND_TOPMOST` from `set_tool_window` in `block_registry.rs`. I9's `build_ui_callback_runtime` is a pure runtime-construction helper that has zero interaction with window styles, Z-order, or positioning. None of the 8 callback sites re-establishes topmost. No risk of callback re-introducing the removed behavior.

### Seam 5 — I3 reorder and the C-4 invariant

**Result: Clean — invariant holds.**

The C-4 invariant ("DialogTurnCompleted emitted only after persistence succeeds") is verified by trace: `persist_completed_dialog_turn` is called at line 305 of `sub_handle_out.rs`, completing with `.await` BEFORE the `(status, Some(completed_event))` tuple is constructed at line 315. The tuple is then forwarded into the spawned task, where the I3 reorder places: (1) `enqueue(completed_event)` → (2) `tx.send(workspace_turn_status)` → (3) `finalize_persisted_turn_in_workspace_if_needed`. Persistence already completed before step (1). The growth finalize (LLM distillation, up to 30 s) is correctly placed after the UI-completion-signaling event so it no longer blocks the completion path. Six listener sites for DialogTurnCompleted were audited by I3's review — none read growth data at completion time. **Confirmed: invariant preserved.**

---

## Calibration Result (Ledger vs Git)

**Calibration: COMPLETE and CORRECT.**

| Task | Ledger Range | Git Verification | Status |
|---|---|---|---|
| C1 | 66f08d1..fb98a77 | ✅ 10 commits listed; base=66f08d1 matches `git log` output | ✅ Verified — 补录 correct |
| I1 | fb98a77..64fba6f | ✅ Correct sequential pair | ✅ |
| I2 | 64fba6f..37a71f4 | ✅ | ✅ |
| I4+I5 | 37a71f4..0b195bc | ✅ | ✅ |
| I6 | 0b195bc..593c247 | ✅ | ✅ |
| I7 | 593c247..f550d06 | ✅ | ✅ |
| I9 | f550d06..a8a0b70 | ✅ | ✅ |
| I8 | 8fc51bc..c48e4a9 | ✅ `git diff --stat` confirms single-file TOPMOST removal (block_registry.rs, 3 lines) | ⚠️ Notationally imprecise (M-2) |
| I3 | c48e4a9..bbfe1de | ✅ | ✅ |

The previously corrupted SHAs were correctly repaired: `66f08d1..fb98a1..fb98a77` chain verified against `git log`. The C1 "补录" row is consistent with the actual commit chain. No ledger row contradicts the diff.

---

## Deferred-Item Triage

### r2 #6 W1 subspans — C1 guarantees for affected call sites

**Decision: CONFIRM DEFER — condition already met.**

The original audit noted C1 would make "not worse" guarantees for W1a-3, W1a-2, W1a-4, W1b, W1c, W2, W5, W7. The diff honors this: the broadcast-only mode (`heap_enabled: false`) in the desktop host makes `enqueue` unconditionally return `Ok(_)`, so no previously-OK call site can now regress. For non-desktop hosts (CLI/server), the original behavior is preserved via the default `heap_enabled: true`. The guarantee holds for all listed call sites without additional code changes.

### F5 (process_group + kill_on_drop inconsistent across call sites)

**Decision: CONFIRM DEFER — not made worse.**

I4+I5 addresses only the LSP and MCP spawn sites (the two long-running, production-critical ones). F5's remaining call sites (process_command.rs, command.rs, git/utils.rs, workspace_info_impl.rs, computer_use_actions/utilities.rs) are unchanged. The audit classified F5 as Minor ("Not a hot-spot on the happy path, most call sites are short-lived `output()` calls"). No regression introduced.

### F9 (spawn_child_process_tree_cleanup runtime-per-Drop cost)

**Decision: CONFIRM DEFER — blast radius grows but pattern is sound.**

I4+I5 introduces a second call site for `spawn_child_process_tree_cleanup` (MCP Drop). This extends the Minor pattern (new tokio runtime + thread per Drop) to more code paths. However, the pattern itself is sound (Drop must be synchronous, can't block), and the F9 recommendation to deprecate or inline the helper is a separate refactor. The new usage doesn't introduce correctness risk, only amplifies the performance-micro pattern noted in F9. Acceptable to defer.

### Accumulated Minors (15 across rows)

**Decision: NONE escalate to Important.**

All 15 Minors are report-hygiene items: toolchain pin notation, line-count measurement, subscriber-audit documentation, remove-before-finalize naming, warn! import style, etc. None interact with correctness, and none are cumulative when combined. All remain triage-at-end-of-wave.

---

## House-Rules Spot-Check

| Rule | Status | Evidence |
|---|---|---|
| File-length ceiling (800 review pressure / 1000 allow-god-file) | ⚠️ One violation | `callbacks_lifecycle.rs` 1,009 lines, no `// allow-god-file` comment (M-1) |
| English-only logs | ✅ Pass | All new log calls use English: `tracing::warn!`/`tracing::error!` with English messages |
| No new mutex/timeout/atomic | ✅ Pass | I4+I5 reuses process_manager (CancellationToken reuse confirmed); I1/I2/I6/I7/I9 add no concurrency primitives |
| Rot budget only decreases | ✅ Pass | callbacks_lifecycle.rs 1,011→1,009; no file added that exceeds prior ceilings |
| God-file: new files split or justified | ✅ Pass | No new .rs files added |

---

## Residual Risks Requiring Human Action

1. **Physical-machine UI walkthrough (desktop)** — The ledger flags this as pending: fold-state + drawer + anti-bottom-bounce + Z-order visual confirmation. The HWND_TOPMOST removal (I8) is confirmed correct by judge Z-order analysis, but visual confirmation on a physical multi-monitor setup remains a user-side action item.

2. **F5/F9 deferred** — Both are Minor process-management pattern issues. F5 remains open at 7 call sites; F9's runtime-per-Drop cost is now exercised by two Drop chains (LSP + MCP). Neither is correctness-risk, but both should be scheduled for a future batch.

3. **pages_onboarding.rs 866 lines** — Over the 800-line review-pressure threshold (pre-existing debt from P3a). Not introduced by this wave; tracked for next expansion batch.

---

*Reviewer: independent final reviewer (K3). Per-task review evidence sourced from diff package + ledger; no tests re-run. Cross-task seam analysis is the primary deliverable.*
