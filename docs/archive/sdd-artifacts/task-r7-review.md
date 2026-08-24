# Task R-7 Review

## 1. Verdict summary

- **SPEC: FAIL**
- **QUALITY: FAIL**
- **REJECTED**
- Findings: **Critical 1 / Important 3 / Minor 0**

The change is correctly scoped to the facts call site and leaves episode logging and `append_facts_entry` internals unchanged. However, the in-memory-only fail-closed lookup can suppress facts for a valid long-running main turn after session eviction, one public hidden-subagent creation path can have neither checked signal, the required lookup-failure test is absent, and the verification report does not contain the mandated complete raw output and reports the wrong line count.

## 2. Findings

### Critical

#### C1. In-memory lookup failure can suppress a valid main-dialog turn after eviction

- **Location:** `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs:338-353`
- `SessionManager::get_session` is only a `DashMap` lookup (`agentic/session/session_manager_lifecycle.rs:189-191`); it does not consult persistence.
- The cleanup task evicts sessions solely from `last_activity_at` age and does not exclude `SessionState::Processing` (`agentic/session/session_manager_auto_save_cleanup.rs:93-132,170-232`).
- A main turn refreshes `last_activity_at` when it starts/enters processing (`agentic/session/session_persistence/turn_lifecycle.rs:69-80`; `coordination/dialog_turn/sub_handle_out.rs:270-279`), but no periodic touch exists during execution. The watchdog is configurable and defaults to 600 seconds (`sub_handle_out.rs:40-50`); after watchdog timeout, the inner execution/finalization task is explicitly allowed to continue in the background (`sub_handle_out.rs:385-404`). Thus a configured or cancellation-stalled main turn can outlive the one-hour cleanup threshold, be removed from `sessions`, later reach this finalizer, and now lose facts solely because `get_session` returns `None`.
- This is a regression relative to the old unconditional `append_facts_entry` call and matches the requested Critical condition: a legitimate main turn is not guaranteed to remain retrievable from the in-memory cache at finalization time. The new `warn!` makes the loss observable, but does not preserve the memory operation.

**Executable fix:** at `turn_persist.rs:338`, use the in-memory session when present; when absent, resolve the retained workspace path with `effective_session_workspace_path(session_id)` and load persisted `SessionMetadata`, then apply the same parent/creator predicate. Fail closed only when both in-memory and persisted metadata are unavailable. Keep the missing-metadata warning and add a pure test for the fallback classification result. This preserves fail-closed security without treating ordinary cache eviction as proof of a subagent.

### Important

#### I1. The required fail-closed lookup/missing-metadata test is absent

- **Location:** `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs:733-780`
- All seven tests call `is_main_dialog_session` with an already-resolved pair of optional fields. None represents `session_manager.get_session(session_id) == None` and asserts denial.
- In fact, `(None, None)` is asserted as allowed at `turn_persist.rs:738-741`, so the pure predicate cannot distinguish “known main session with absent child markers” from “session/metadata unavailable.” The report acknowledges the missing call-site fail-closed test at `task-r7-report.md:107-113`.
- The report also acknowledges that the base file had no `#[cfg(test)]` module (`task-r7-report.md:95-97`) but continued rather than reporting the brief-mandated `BLOCKED` state.

**Executable fix:** extract a pure helper whose outer `Option` represents lookup/metadata availability, for example `should_distill_facts(Option<SessionSignals>)`; assert `None => false` and `Some { parent: None, created_by: None } => true`. The async call site should only adapt `SessionManager`/persisted metadata into that pure input. Add the test in this file; no full `SessionManager` fixture is needed.

#### I2. One public hidden-subagent creation path can bypass both checked signals

- **Location:** `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs:340-345,374-399`
- Normal fresh/fork dispatch is covered by `created_by = "session-..."` (`subagent_orchestrator/so_dispatch.rs:45,58-65,101-108`), and `/btw` children are covered by the same prefix (`so_handlers.rs:59-68`).
- However, `create_hidden_subagent_session_with_workspace` is a public subagent constructor and forwards caller-supplied `created_by` unchanged (`dialog_turn/coordinator_session.rs:139-154`). A caller may pass `None` or a non-`session-` value. The created `SessionKind::Subagent` initially has `relationship = None` (`agentic/core/session.rs:163-179`; `dialog_turn/session.rs:86-104`).
- The report/comment claim that `parent_session_id` is set in memory for every dispatched/forked child is also inaccurate. `persist_session_lineage` writes persisted metadata only (`agentic/session/session_manager_metadata.rs:408-415`); the in-memory relationship is populated on disk restore (`agentic/persistence/session_subhandlers.rs:225-254`). The normal dispatch remains safe because `created_by` catches it, but the public constructor above can produce a live subagent with both predicate inputs `None`, so its brief is classified as main and distilled.
- No in-repository caller of this public wrapper was found, but it is still an exposed creation path and therefore an incomplete security gate.

**Executable fix:** within the one-file gate, retain the required parent/creator checks and additionally fail closed for `SessionKind::Subagent` (and `EphemeralChild` if the early persistence return is ever changed), passing this child-kind fact into the pure predicate and adding a regression test for “subagent kind + no parent + no creator.” Alternatively, if the signal-only design must remain absolute, amend the one-file scope and make the public hidden-subagent constructor always set one of the approved signals.

#### I3. Required verification evidence is incomplete, and the reported line count is wrong

- **Location:** `E:\agent-project\northing\.superpowers\sdd\task-r7-report.md:33-87`
- The report gives selected summary lines rather than complete raw stdout/stderr for the six mandated commands: cargo check contains only the final warning summary; the focused tests contain selected test/result lines; growth and memory tests contain only final result lines.
- The report says `turn_persist.rs` has **708** lines (`task-r7-report.md:85-87`), while the reviewed file has **781** lines. It still satisfies `< 800`, but the required actual value was not reported.
- Per review discipline, the reviewer did not rerun the six commands, so the claimed warning/test totals cannot be independently recovered from the supplied evidence.

**Executable fix:** rerun the exact six brief commands with the required PATH prefix, capture complete stdout and stderr verbatim in the report, explicitly report warning count 19/no new warnings if confirmed, and replace the line count with the command’s actual result (currently 781). Do not run `cargo fmt`.

### Minor

None.

## 3. Constraints checklist

| # | Result | Evidence / conclusion |
|---|---|---|
| 1 | **PASS** | `git diff --name-only 27c9738 e62a8a3` lists only `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`; no `.superpowers/`, schema, SQL, or other committed file. One commit is present. |
| 2 | **PASS** | Gate surrounds only the `append_facts_entry` call at `turn_persist.rs:355-365`. `append_episode_log_entry` call remains unconditional at `:312-322`; its implementation is unchanged by the diff. |
| 3 | **PASS, signal coverage incomplete** | Predicate uses `parent_session_id` and `created_by`, not `agent_type` (`:338-345,387-399`). I2 shows an uncovered subagent creation path. |
| 4 | **FAIL** | A total in-memory lookup miss denies as required (`:346-353`), but a present hidden-subagent session with missing approved metadata can be allowed (`None, None`) through I2. C1 also shows that treating cache absence as authoritative can suppress a valid main turn. |
| 5 | **PASS** | New non-test code contains no `.unwrap()`, `.expect()`, or `panic!`; errors do not propagate. Missing session logs and returns `false`. |
| 6 | **PASS** | `append_facts_entry` body, including distillation, JSONL/DB writes, reviews, and topic boost, is unchanged; only whether it is called changed. |
| 7 | **PASS** | Unused parameter remains `_agent_type` at `turn_persist.rs:496`. |
| 8 | **PASS by static diff; command execution not provable** | Added logs/comments are English-only and emoji-free; no unrelated formatting changes are present. Whether `cargo fmt` was actually invoked cannot be established from a diff. |
| 9 | **FAIL (reporting); code threshold passes** | Actual file length is **781** lines, below 800. Report incorrectly states 708. |
| 10 | **FAIL** | Main allow, parent reject, creator-prefix reject, and other-creator allow are tested. Lookup/metadata-unavailable fail-closed rejection is not tested. |
| 11 | **FAIL** | Supplied report does not contain complete raw output; line-count evidence is incorrect. Claimed 19 warnings, growth_adapter 25, memory_db 21, and boundary exit 0 therefore remain report assertions rather than complete supplied evidence. |

## 4. Special review: can fail-closed disable main-dialog memory?

### Evidence chain

1. `SessionManager.sessions` is an in-memory `DashMap` (`agentic/session/session_manager.rs:99-109`).
2. `get_session` reads only that map (`session_manager_lifecycle.rs:189-191`). There is no persistence fallback in the new gate.
3. A cold/evicted main session is normally restored before a new turn starts (`coordination/dialog_turn/sub_handle_in.rs:35-51`), and restore reinserts it into the map (`agentic/session/restore_apply.rs:361-364`).
4. Turn start/processing and successful state convergence refresh `last_activity_at` (`session_persistence/turn_lifecycle.rs:69-80`; `session_manager_lifecycle.rs:226-273`), so ordinary short turns normally retain the session through finalization.
5. The cleanup task nevertheless evicts by age alone, without checking `Processing` or active-turn registration (`session_manager_auto_save_cleanup.rs:93-132,170-232`). A long/stalled main execution can therefore disappear from the map while its spawned finalization task remains alive.
6. `finalize_persisted_turn_in_workspace_if_needed` does not restore or read persisted metadata. `None` at `turn_persist.rs:346-353` logs a warning and suppresses facts.

### Conclusion

**Risk confirmed: not every valid main turn is guaranteed to be retrievable at finalization; Critical.** Normal short-turn and post-restart submission paths are protected by restore/touch sequencing, so this is not evidence that all ordinary main turns immediately lose memory. It is nevertheless a real supported long/stalled-turn path, and the gate converts cache absence into permanent facts loss for that turn. The missing-session `warn!` provides observability but does not remove the regression.

### Mitigation

Use persisted `SessionMetadata` as a fallback when the in-memory entry is absent, then apply the same security predicate. Preserve fail-closed only when neither source can establish metadata. This also allows a pure outer-`Option` classifier test for the unavailable case.

## 5. Special review: subagent creation paths and signal bypass

### Covered paths

- Fresh Task/subagent dispatch: `so_dispatch.rs:45,58-65` sets `created_by = "session-<parent>"`.
- Fork dispatch: `so_dispatch.rs:45,101-108` uses the same creator marker.
- Spawn lifecycle: `so_lifecycle/spawn.rs:126-152` creates `SessionKind::Subagent` and persists lineage.
- `/btw` hidden child: `so_handlers.rs:59-68` sets the creator marker even though it does not set an in-memory relationship.
- SessionControl/SessionMessage-created agent sessions: their creator helpers generate `session-<source>` (`session_control_tool.rs:67-73`; `session_message_tool/sm_resolve.rs:127-150`), so their non-human forwarded inputs are rejected by this gate.
- Restore path: persisted `relationship` and `created_by` are projected into the in-memory `Session` (`persistence/session_subhandlers.rs:203-254`).

### Bypass

`create_hidden_subagent_session_with_workspace` (`dialog_turn/coordinator_session.rs:139-154`) can create a `SessionKind::Subagent` while forwarding `created_by = None` and without setting an in-memory relationship. If a turn is submitted to that session, the current predicate sees `(None, None)` and allows distillation.

### Conclusion

**Bypass exists: Important.** No current in-repository call site was found, but the public constructor itself does not uphold either signal invariant. The implementation/report’s statement that `parent_session_id` is set in memory for every dispatched/forked subagent is not supported by `persist_session_lineage`.

## 6. Other requested risk conclusions

### `created_by.starts_with("session-")` false-positive surface

`resolve_agent_session_create_created_by` accepts arbitrary non-empty `created_by`/`createdBy` strings from runtime metadata (`coordination/subagent_ports.rs:24-33,66-77`). Therefore an external caller could label a genuinely user-facing Standard session with a `session-` prefix and suppress its facts. Current in-repository producers either use `None` for ordinary main-session creation (`dialog_turn/coordinator_session.rs:20-76`) or intentionally use `session-<source>` for agent-created/cross-session work, so no current legitimate producer collision was found. Static false-positive possibility exists, but observed repository usage makes it low-probability and conservative; no separate finding beyond the signal-design caveat.

### T6a / topic boost and decay

Confirmed: `growth_adapter::boost_turn_topics` remains inside `append_facts_entry` at `turn_persist.rs:553-561`; no boost or decay was moved outside the gate. A rejected child turn performs neither boost nor paired decay, as required.

### Observability and level

- Known non-main sessions: one `debug!` per rejected turn at `turn_persist.rs:366-371`, appropriate for a frequent expected path and unlikely to spam production warning logs.
- Missing in-memory session: `warn!` at `:346-351`, appropriate because it indicates degraded functionality. It is followed by the generic debug line, which is redundant but harmless.
- Logs/comments are English-only and have no emoji.

## 7. Cannot verify from diff / supplied evidence

1. The actual success, complete diagnostics, and exact warning/test totals of the six required commands: the reviewer intentionally did not rerun them, and the report contains only excerpts.
2. Whether `cargo fmt` was invoked: the diff shows no unrelated formatting, but command execution history is not derivable from the diff.
3. Runtime frequency of the long/stalled-turn eviction path and external use of the public hidden-subagent constructor.
4. Values supplied by out-of-repository callers through free-form `createdBy` metadata; only in-repository producers were statically checked.
