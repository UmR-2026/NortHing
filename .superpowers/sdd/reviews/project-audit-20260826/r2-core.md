# R2 — Core Assembly Hot-Spot Audit

- **Area**: `src/crates/assembly/core/src/**` (kernel_facade, agentic/events, agentic/coordination/dialog_turn + scheduler, agentic/session, service/workspace, util/errors)
- **Repo**: `E:\agent-project\northing` · branch `main` @ `74ea164` (clean tree)
- **Auditor**: R2 (core assembly) · read-only · 2026-08-26
- **Method**: hot-spot risk review (silent loss / lock discipline / cancel-timeout / session lifecycle / DTO fidelity / panic paths), not line-by-line. Ledger (`docs/status/tech-debt-ledger.md`) read first; excluded items not re-reported.

---

## Verdict: needs attention

One structural silent-event-loss defect (Critical) plus several robustness/latency gaps. No production panic path found in the audited surface; lock discipline is generally careful (DashMap guards released before `.await`, async locks, poison-recovery on the subscriber callback). The cancel/watchdog/scheduler paths are well-engineered (stale-turn guards, RAII counter/state guards, bounded watchdog, convergence fallback).

---

## Findings (most severe first)

### 1. Critical — Event priority-heap is never drained in the desktop (broadcast-only) deployment; after ~10k events all non-Critical events are dropped permanently

- **file:line**:
  - `src/crates/assembly/core/src/agentic/events/queue.rs:99-105` — enqueue rejects non-Critical when `queue.len() >= max_queue_size`, then **unconditionally** `queue.push(...)`.
  - `queue.rs:36` — `max_queue_size: 10000` (Default).
  - `queue.rs:127-176,192-217` — the only pop APIs are `dequeue_batch` / `dequeue_configured_batch` / `clear_session`.
  - `src/crates/assembly/core/src/agentic/system.rs:34` — desktop path builds the queue with `Default::default()`; `system.rs:87-103` — the only production pump subscribes to the **broadcast** channel, never drains the heap.
  - `src/apps/cli/src/modes/exec.rs:128-129`, `src/apps/cli/src/modes/chat/run.rs:223` — the **only** production callers of `dequeue_batch` are CLI. `EventQueue::clear_session` has **zero** production callers (only unrelated `file_read_state_store.clear_session` at `agentic/session/session_manager_metadata.rs:487`).
  - `src/crates/assembly/core/src/kernel_facade/lifecycle.rs:98` — desktop `init_core` → `init_agentic_system` (so desktop uses this queue, broadcast-only).
  - Swallowing call sites: `agentic/coordination/subagent_orchestrator/so_handlers.rs:268` (`let _ = …enqueue…`), `agentic/coordination/dialog_turn/sub_handle_out.rs:366` (`let _ = …enqueue…`).
- **what**: `enqueue` couples two structures — it pushes every event onto a bounded priority `BinaryHeap` **and** sends to a broadcast channel. Delivery to the desktop UI happens only via the broadcast channel; the heap is drained only by `dequeue_batch`, which only the CLI calls. In the desktop nothing ever pops the heap, so it fills to 10000 and stays full. From then on every non-Critical `enqueue` returns `Err(EventQueueFull)` and is dropped (silently via `let _ =`, or logged by `StreamEventSink`).
- **why it matters**: Per `default_priority` (`contracts/events/src/agentic.rs:689-742`), `TextChunk`, `DialogTurnCompleted`, `TokenUsageUpdated`, `ModelRound*` are **Normal** and tool `Started/Completed/Failed/ConfirmationNeeded` are **High** — all dropped once the heap is saturated. Only `DialogTurnFailed/Cancelled/SystemError` (Critical) still get through. In a long-running desktop session the UI stops receiving streaming text, tool updates, and completion events — it appears frozen while turns keep executing and persisting in the background. The permanently-retained 10k envelopes are also a slow memory leak. This is silent data loss on the primary shipping surface.
- **fix direction**: Decouple the two delivery paths — either (a) don't gate broadcast delivery on the heap cap / don't push to the heap when no heap-consumer is registered, (b) spawn a heap drain in the desktop like the CLI, or (c) make the heap a bounded ring that evicts oldest instead of rejecting new. Add a regression test asserting the desktop delivery path does not reject after 10k enqueues.
- **effort**: M
- **not tracked**: distinct from ledger P2-6 (that was enqueue returning false `Ok` when full; resolved). This is the never-drained-heap gate; not in the ledger.

### 2. Important — one corrupt `session_state.json` poisons the whole workspace session list; `list_sessions_all_workspaces` fails atomically across all workspaces

- **file:line**:
  - `src/crates/assembly/core/src/agentic/persistence/session_subhandlers.rs:303-307` — `list_sessions` does `load_stored_session_state(workspace_path, &id).await?` per session (propagates error).
  - `src/crates/services/services-core/src/json_store.rs:113-116` — `read_optional` returns `Err(Deserialize)` on a parse failure (not `Ok(None)`).
  - `src/crates/assembly/core/src/kernel_facade/session.rs:90-96` — `list_sessions_all_workspaces` maps any per-workspace error to a whole-call `Err` (`?` inside the loop).
- **what**: The runtime-state enrichment in `list_sessions` is fail-fast. A single unreadable/corrupt/schema-drifted `session_state.json` makes the entire workspace listing error, and the cross-workspace aggregator turns that into a total failure for every workspace.
- **why it matters**: The consult-room startup flow `ensure_room_session` now depends on `list_sessions_all_workspaces()` (ledger P2-22 resolution). One bad state file (disk error, partial write from an external cause, or a future non-backward-compatible schema change) makes room session resolution fail entirely — the app can't list or open sessions in any workspace. State enrichment is cosmetic (defaults to `Idle`); it should not be load-bearing.
- **fix direction**: In `list_sessions`, treat a failed state load as `None` → `SessionState::Idle` with a `warn!` (skip-and-continue), matching the existing `.unwrap_or(SessionState::Idle)` intent. Optionally make `list_sessions_all_workspaces` tolerate a single failing workspace (return its group empty + warn) instead of aborting the whole listing.
- **effort**: S

### 3. Important — turn-completed event and watchdog signal are delayed by inline growth/memory LLM distillation (up to ~15–30s)

- **file:line**:
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs:352` — `finalize_persisted_turn_in_workspace_if_needed(...)` is awaited **before** `:365-367` enqueues `DialogTurnCompleted` and `:368` fires the watchdog oneshot `tx.send`.
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs:312,326` — finalize runs `append_episode_log_entry` + `append_facts_entry`.
  - `turn_persist.rs:472` — `append_facts_entry` calls `distill_facts_with_llm(...)`; `turn_persist.rs:561` calls `run_dream_sweep(...)`.
  - `src/crates/assembly/core/src/service/agent_memory/distiller.rs:27` — `DISTILL_TIMEOUT_SECS = 15`; `dream.rs:24` — `DREAM_LLM_TIMEOUT_SECS = 15` (24h-gated at `dream.rs:20`).
- **what**: Growth/memory LLM work (fact distillation + dream sweep) runs inline in the turn-finalize task, ahead of enqueuing the terminal `DialogTurnCompleted` event and ahead of the watchdog's completion signal.
- **why it matters**: The turn is already persisted and the scheduler already notified (that happens earlier in `persist_completed_dialog_turn`, so queued-message dispatch is unaffected), but the UI's completion event is held back up to ~15s (distiller) + up to ~15s (dream sweep when the 24h gate opens) — the UI shows "generating" after the turn has actually finished, and the watchdog oneshot (`sub_handle_out.rs:372-392`) is delayed by the same window. Bounded (both calls are timeout-capped and warn-only), so no hang — but unnecessary terminal-event latency coupling growth work into the turn lifecycle.
- **fix direction**: Enqueue `DialogTurnCompleted` and fire the watchdog oneshot **before** the growth hooks, or spawn `append_episode_log_entry`/`append_facts_entry` onto a separate task so they don't sit on the completion path.
- **effort**: S

### 4. Important — kernel-facade subscriber delivery rides a 1024-buffer broadcast; lag drops events warn-only, and one slow subscriber stalls the shared sequential pump

- **file:line**:
  - `src/crates/assembly/core/src/agentic/events/queue.rs:23` — `EVENT_BROADCAST_BUFFER = 1024`.
  - `src/crates/assembly/core/src/agentic/system.rs:97-99` — pump handles `RecvError::Lagged(skipped)` with a `warn!` and continues (events skipped are lost).
  - `src/crates/assembly/core/src/agentic/events/router.rs:61-84` — `route` dispatches to subscribers **sequentially**, awaiting each `on_event`; one slow subscriber blocks the pump.
  - `src/crates/assembly/core/src/kernel_facade/events.rs:13,17-26` — the desktop UI subscriber invokes a synchronous `Fn` callback under a `std::sync::Mutex` inside `on_event`.
  - Subscribers confirmed on this path: `src/apps/desktop/src/app_state/event_bridge.rs:339,352`, `src/apps/desktop/src/ui_dioxus/api.rs:194` (both via `kernel_facade().subscribe_events`).
- **what**: Desktop UI events flow `enqueue → broadcast_tx → pump(system.rs) → EventRouter.route → KernelEventSubscriber → UI callback`. If the pump falls behind by >1024 events (a burst of fast `TextChunk`s, or a slow/blocking UI callback under the Mutex), the broadcast overruns and events are dropped with only a warning.
- **why it matters**: Dropped `TextChunk`/`TurnState` events stall or corrupt the UI stream. The sequential dispatch means a single misbehaving subscriber degrades delivery for all subscribers. This is conditional (requires the pump to lag), so lower severity than Finding 1, but it is a second independent silent-event-loss path on the same delivery chain. The team has already observed this lag class (`apps/desktop/src/app_state/streaming_lifecycle.rs:5-19` documents `EVENT_BROADCAST_BUFFER=1024` lag affecting the stop button).
- **fix direction**: Raise the buffer and/or make per-subscriber delivery independent (spawn per subscriber or use per-subscriber bounded channels with explicit overflow policy); ensure the UI callback never blocks the pump (it already posts to the Slint event loop — keep it that way).
- **effort**: M

### 5. Minor — session create: in-memory insert is not rolled back when persistence fails (ghost session occupies a slot)

- **file:line**: `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs:166-182` — `sessions.insert(...)` and index insert happen, then `save_session(...).await?` can return `Err` with no rollback of the in-memory inserts.
- **what**: If the initial persist fails, the caller gets `Err` but the session remains in the in-memory map and `session_workspace_index`, consuming one of `max_active_sessions` slots.
- **why it matters**: Repeated persist failures leak in-memory session slots and can eventually trip the `max_active_sessions` guard (`:149-154`) for a session that was never successfully created. Low likelihood (persist failures are rare), hence Minor.
- **fix direction**: On persist failure, remove the just-inserted in-memory entries before returning `Err` (or persist first, then insert into memory).
- **effort**: S

### 6. Minor — `prepare_turn` history-restore failure is logged at `debug!` and the turn proceeds with partial context

- **file:line**: `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_in.rs:172-177` — restore `Err(e) => debug!("Failed to restore session history (may be new session)…")`, then continues.
- **what**: When the context is cold and a restore is attempted (`:141-161`), a restore failure is swallowed at debug level and the turn runs without the restored history.
- **why it matters**: For a session that genuinely has persisted turns (the `:124` branch: has `dialog_turn_ids` but few messages), a transient IO/parse failure silently degrades the model's context for that turn — the user sees a worse answer with no signal. Legitimate for brand-new sessions, but the same debug path covers real data-unavailability. No permanent loss (old turns stay on disk), hence Minor.
- **fix direction**: Distinguish "new session" from "restore failed for a session with turns"; emit a `warn!` (or a SystemError/banner) when restore fails for a session known to have history.
- **effort**: S

### 7. Minor — `metadata_to_message_dto` silently nulls a compression payload that fails to serialize

- **file:line**: `src/crates/assembly/core/src/kernel_facade/dto.rs:72-74` — `serde_json::to_value(p).unwrap_or(serde_json::Value::Null)`.
- **what**: If `compression_payload` fails to serialize, the DTO carries `Null` with no log.
- **why it matters**: Silent fidelity loss on message metadata; a serialization failure here would be invisible. `CompressionPayload` is normally serializable, so likelihood is low — Minor, but a `warn!` on the `Err` arm would make it observable.
- **fix direction**: Log a warning on the `Err` arm instead of a bare `unwrap_or(Null)`.
- **effort**: S

### 8. Minor — `message_to_dto` silently drops multimodal images that lack an `image_path`

- **file:line**: `src/crates/assembly/core/src/kernel_facade/dto.rs:23-26` — `images.iter().filter_map(|img| img.image_path.clone())`.
- **what**: Multimodal images that carry only a `data_url` (no `image_path`) are filtered out of the DTO silently.
- **why it matters**: The frozen `MessageContentDto::Multimodal.images` is a `Vec<String>` of paths, so data-URL-only images can't be represented — but the silent `filter_map` means history rendering loses those attachments without trace. Relevant to remote-image flows where `data_url` is the primary carrier (see `scheduler_turn/turn_submit.rs:392-399` accepting `dataUrl`-only attachments). Minor (schema constraint), worth a comment or a placeholder so loss is deliberate and visible.
- **fix direction**: Document the path-only contract at the filter site, or surface a marker for path-less images rather than dropping them silently.
- **effort**: S

---

## Notes on things checked and cleared (not findings)

- **Outer event catch-all** `kernel_facade/events.rs:296,298` (`_ => vec![]`): maps onto the deliberately **FROZEN** minimal `KernelEventDto` (`contracts/kernel-api/src/events.rs:68-102` — only TextChunk/TurnState/ToolCall/TurnPhase/Banner/Error). Unmapped `AgenticEvent`s have no DTO variant by design; richer data is available via `get_session_metadata`. Not a mapping bug. The `ToolEventData::Confirmed/Rejected` variants dropped at `:296` are never constructed in production (dead variants; only matched in `interfaces/acp/src/runtime/events.rs:130-133`), so nothing real is lost.
- **Lock discipline**: no `std::sync::Mutex`/`RwLock` held across `.await` in the audited area; `DashMap` guards are explicitly released before `.await` (`session_manager_lifecycle.rs:196-207`); workspace manager uses `tokio::sync::RwLock` with short critical sections (`service/workspace/accessors.rs:64-67`); the subscriber callback Mutex is poison-recovered and not held across an await (`kernel_facade/events.rs:18-26`).
- **Cancel/timeout**: `turn_cancel.rs` has stale-turn guards (`:164-195`), a capped 1500ms drain wait with a convergence fallback (`:212-238`); the watchdog (`sub_handle_out.rs:42-48,371-392`) is bounded (default 600s, env-overridable) and races a oneshot; RAII `ActiveTurnRegistration`/`SessionExecutionGuard` keep the active-turn counter balanced on early-return and success paths. Cancel-token cleanup happens inside `execute_dialog_turn` (`execution/execution_engine.rs:121-126`).
- **Panic paths**: the only `panic!` under `kernel_facade/` are in a `#[cfg(test)]` module (`kernel_facade/tools.rs:91-92`). No `unwrap`/`expect`/indexing found on production paths in the audited files.
- **DTO fidelity**: `summary_to_dto`/`session_to_dto`/`metadata_to_dto` are faithful to their (frozen-minimal) DTO shapes; full metadata is exposed via `SessionMetadataDto`.

---

## Sample coverage note

**Deep-read (full)**: `kernel_facade/{events,dto,session,turn,helpers,lifecycle,mod}.rs`; `agentic/events/{queue,router,types,mod}.rs`; `agentic/system.rs`; `agentic/coordination/dialog_turn/{turn_cancel,coordinator_cancel,sub_handle_out,sub_handle_in,coordinator_session,turn_persist}.rs`; `agentic/coordination/scheduler/{mod,scheduler_types,scheduler_lifecycle}.rs` + `scheduler_turn/turn_submit.rs`; `agentic/session/{session_manager_workspace_path,session_manager_persistence_predicate,facade}.rs`; `service/workspace/{manager,manager_lifecycle,facade}.rs`; `util/errors.rs`.

**Partial/targeted**: `agentic/coordination/coordinator.rs` (CancelTokenGuard, ActiveSubagentExecution, SubagentExecutionScope); `agentic/coordination/dialog_turn/sub_handle_out.rs` watchdog region; `agentic/session/session_manager_lifecycle.rs` (create/update-state/list); `agentic/persistence/{session_subhandlers,metadata_subhandlers,paths_utilities}.rs`; `service/workspace/{accessors,service_state}.rs`; `contracts/events/src/agentic.rs` + `contracts/kernel-api/{events,session}.rs`; `services-core/json_store.rs`; `execution/execution_engine.rs`.

**Skimmed only (other rounds own)**: growth/memory internals (`service/agent_memory/{distiller,dream}.rs` — read only to bound the timeouts in Finding 3), `service/memory*`, `service/snapshot` (not opened).

**Cross-checked (consumer side, outside area but needed for Finding 1)**: `src/apps/cli/src/modes/{exec.rs,chat/run.rs}` (heap drainers), `src/apps/desktop/src/{main.rs,app_state/streaming_lifecycle.rs,app_state/event_bridge.rs,ui_dioxus/api.rs}` (no heap drain; broadcast/kernel-facade subscribers).

**Not opened** (out of area / low yield for this mandate): `kernel_facade/{settings,memory,tools,agents,platform,usage}.rs` beyond a panic grep, `function_agents/`, `infrastructure/`, `product_runtime/`, `service_agent_runtime/`.
