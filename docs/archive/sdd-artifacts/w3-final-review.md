# W3 波次终审报告（a7ac75d..c6f2924）

Reviewer: reviewer/step-explore_reviewer（首次空响应，按 SOP 同 task_id 续派成功）
日期：2026-08-27
结论：**CAN MERGE · 0 Critical · 0 Important · 3 Minor（defer/reject，无阻塞）**

以下为终审 reviewer 原文（ verbatim ）：

---

### Strengths

- **W3-1** went beyond the minimum rollback spec: the implementer correctly identified that rolling back only `sessions` + `session_workspace_index` while leaving `context_store`, `turn_skill_agent_snapshot_store`, and `file_read_state_store` populated would still leak state — and wired all five rollbacks. This is the correct root-cause fix, not the literal spec minimum.
- **W3-3** chose the genuinely better algorithm: incremental `watch`/`unwatch` on the existing `RecommendedWatcher` eliminates both the full-rebuild waste *and* the dual-task window that a JoinHandle-abort design would have left behind. The `Option`-guard dance (drop the guard before `create_watcher()`) is correct and the deadlock narrative holds — the two lock acquisitions are sequential with no nesting.
- **W3-4** `abort_drain_task` closure is clean deduplication: seven early-return sites now each call `abort_drain_task()` instead of inline `handle.abort()`. Tests cover both negative assertions (early-return paths abort) and the positive assertion (normal stream completion does not abort), with the `_keep_sse_tx` lifetime trick correctly preventing false negatives from the channel closing for the wrong reason.
- All logs: English, no emojis, structured key-value fields (`turn_id`, `round_id`, `error`, `message_id`, `dropped_count`, `total_images`, `session_id`).
- All four tasks are crate-scoped; no upward cross-layer dependencies introduced. `cargo check --workspace` clean on every task.

### Cross-task seam check

| Seam | Verdict | Evidence |
|---|---|---|
| W3-1 ↔ W3-2 (session rollback ↔ DTO conversion) | **Verified-clean** | session persistence layer and DTO mapping layer are different crates/traits; `save_session` writes the canonical session struct — DTO conversion is client-side read path. Rollback occurs before the session is visible to any reader, so no race where a partially-rolled-back session could be serialized through dto.rs. |
| W3-1 ↔ W3-4 (session lifecycle ↔ stream processor) | **Verified-clean** | `SessionManager` and `StreamProcessor` live in different execution crates. `StreamProcessor` is scoped to a single stream run; it holds a `Session` by ID string, not a live reference to the `DashMap`. The abort-on-early-return in W3-4 doesn't interact with session creation at all. |
| W3-2 ↔ W3-3 (dto.rs ↔ file watch) | **Verified-clean** | File-disjoint, crate-disjoint. No semantic coupling. |
| W3-2 ↔ W3-4 (dto.rs ↔ stream drain task) | **Verified-clean** | File-disjoint, crate-disjoint. `SseLogCollector` is crate-internal to `agent-stream`; dto.rs is in `assembly/core`. The drain task writes to an `Arc<Mutex<SseLogCollector>>`, which is flushed on error — this is internal to W3-4's scope and not affected by W3-2's DTO changes. |
| W3-3 ↔ W3-4 | **Verified-clean** | File-disjoint, crate-disjoint. No shared state or call paths. |
| Whole-wave log consistency | **Verified-clean** | All four new logs: English, no emoji, structured fields — consistent across tasks. |
| W3-1 caller-visible failure behavior | **Risk noted, low** | `create_session_with_id_and_details` returns the raw persistence error. All 7 callers (including `so_handlers.rs`, `dialog_turn/session.rs`) receive `Err` and do not retry-create — confirmed by the `Session::new_with_id` pattern requiring explicit caller-supplied IDs. No caller silently ignores the `Err` return. Low risk. |
| W3-3 caller-visible failure behavior | **Risk noted, low** | `watch_path` incremental `Err` returns with the same message format as the old full-rebuild path: `"Failed to watch path {}: {}"`. Existing callers that match on the error string will continue to match. `unwatch_path` on unknown paths is now a documented no-op (new test confirms) — previously it called `create_watcher()` which rebuilt the watcher; now it short-circuits. A caller relying on `create_watcher()` side effects from unwatch-unknown would break, but that was a reliance on an implementation accident, not a contract. Low risk. |
| W3-4 SSE channel drain ↔ upstream SSE sender | **Verified-clean** | The drain task reads from `rx` (the unbounded channel SSE data arrives on) and writes to the `Arc<Mutex<SseLogCollector>>`. `abort()` on early return drops the `rx` receive end. The sender end (`sse_tx`) remains open until the test/owner drops it. Tests correctly use `_keep_sse_tx` to hold the sender alive and assert `is_closed()` as a surrogate for "drain task terminated" — this is functionally sound: `abort()` causes the task to be dropped, which drops the `rx` guard, which closes the channel, which causes `is_closed()` on the sender. |

### Minor triage

| Task | Item | Verdict | Reason |
|---|---|---|---|
| W3-1 ① | `create_session_with_id` wrapper paths' key-uniqueness not enumerated | **Defer** | Pre-existing contract question. The current diff fixes the immediate bug correctly. The wrapper paths (`with_creator`, `with_details`) all funnel through `create_session_with_id_and_details` where the rollback now lives — so the fix is structurally inherited by all wrappers without needing per-wrapper enumeration. |
| W3-1 ② | New test doesn't assert the `warn!` log | **Defer** | Observability coverage, not a correctness gap. The structural assertion (all five stores cleaned, sessions map empty) already verifies the fix's core behavior. A warn! assertion is a nice-to-have for the backlog. |
| W3-1 ③ | Error category not pinned with `matches!` | **Reject** | This is a different finding from F2 (which was about `AiClient` error being surfaced as generic). Here the raw persistence error is returned directly — no recategorization occurs, so `matches!` pinning is not applicable. |
| W3-2 ① | rustfmt reflowed three sibling function signatures | **Reject** | Tool-driven formatting change, function bodies untouched. Not a defect. |
| W3-2 ② | `message_to_dto` restructured to let-bindings | **Reject** | Readability-neutral restructuring; same control flow, same data. The new form actually improves clarity by separating role/content/metadata construction. |
| W3-3 ① | `cargo check --workspace` output in report lacks warning section | **Reject** | The actual output had zero warnings. No warning section was needed. |
| W3-3 ② | Three new `unwrap()` in tests | **Reject** | All three are on test setup paths (`dir_a.path().to_str().unwrap()`, `std::fs::write(...).expect(...)`). These panic on setup failure, not on logic — appropriate for test scaffolding. |
| W3-3 ③ | Same-path re-watch idempotency and incremental-failure inconsistency untested | **Defer** | Legitimate behavioral gap. Re-watching the same path through `notify::Watcher::watch` is library-defined to be idempotent (the notify crate handles duplicate watches silently), but this is not directly probed. If a caller relies on re-watch returning `Err`, the new behavior silently succeeds — a subtle contract shift. Add to backlog. |
| W3-3 ④ | Spec-3 single-task property has no direct probe | **Accept ledger explanation** | The ledger already closes this: Spec-3 (≤1 background task) holds structurally because `create_watcher()` is the sole `spawn_blocking` site and is only called when `watcher` is `None`. The 50ms poll-tick window during which the old task lingers after `watcher = None` is bounded and was explicitly accepted as non-leak by the ledger. No finding. |
| W3-4 ① | Tests infer abort via 15ms `sleep` + `is_closed` instead of `is_finished`/`is_aborted` | **Defer** | Functionally equivalent: `abort()` on a `tokio::spawn` handle causes the task to be dropped, which drops `rx`, which closes the channel, which causes `sse_tx.is_closed()` to return `true`. The 15ms sleep is shorter than the notify poll tick and is sufficient. A cleaner test would call `.await` on the `JoinHandle` or check `is_finished()` after a zero-delay yield, but the current approach is sound. Worth a follow-up for robustness under high load. |
| W3-4 ② | Report line-count drift (639 vs 644) | **Reject** | Trivial report artifact. No code or behavioral relevance. |
| W3-4 ③ | `Fn` vs `FnOnce` closure style | **Reject** | The `abort_drain_task` closure captures `sse_drain_task` by reference (`if let Some(ref handle)`), making it `Fn` (callable multiple times). Changing to `FnOnce` would require moving ownership into the closure, which would then need to be rebuilt per early-return site — net code increase with no benefit. The current style is correct. |

### Issues

#### Critical (Must Fix)
*(none)*

#### Important (Should Fix)
*(none)*

#### Minor (Nice to Have)

- W3-3 ③ — **Re-watch idempotency contract gap**: `watch_path` now silently succeeds when called with a path that is already watched (because `watcher.watch()` is idempotent in notify), whereas the old code would rebuild the watcher (also succeeding). Existing callers matching on Err-messages for duplicate paths will silently succeed. No caller currently does this, but the contract shift is worth documenting in the `watch_path` doc comment. (`file_watch/service.rs:139`)

- W3-1 ② — **Test should cover `warn!` log**: The new rollback test asserts structural cleanup but doesn't assert the `warn!` is emitted. The log is the user-visible signal for diagnosing persistence failures in production. (`session_manager_lifecycle_tests_rollback_delete.rs:798-850`)

- W3-4 ① — **Test robustness**: Replace the `sleep(Duration::from_millis(15))` + `is_closed()` pattern with `handle.is_finished().await` or a `tokio::task::yield_now().await` followed by `is_finished()`. The sleep-based approach works under normal load but has a theoretical race under thread starvation. (`lib.rs:586` and `lib.rs:617`)

### Assessment

**Wave verdict:** **CAN MERGE** · 0 Critical · 0 Important · 3 Minor (defer/reject)

**Reasoning:** All four tasks were implemented correctly with full test coverage and clean workspace verification. The cross-task seam analysis confirms no semantic interference between the four changes — they are file-disjoint and operating at different abstraction layers. The three deferred Minor items are observability polish or latent contract documentation gaps, not correctness defects, and do not block wave closure.
