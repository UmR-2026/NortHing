# Review P0a — consult-room frontend line (kernel facade bridge)

> Reviewer: judge-m3 (MiniMax-M3)
> Scope: `task-p0a-bridge-brief.md` vs `4889d22` on base `9bba819`
> Package: `.superpowers/sdd/reviews/p0a-bridge/{diff.patch,diffstat.txt,report.md}`
> Sources: actual code read via `git diff 9bba819..4889d22 -- <files>` (the on-disk
> `diff.patch` only contains the brief + report metadata; the four code files
> had to be re-extracted from git).

## Per-constraint verdict (verbatim from brief)

### 1. `KernelToolsApi` 新增 `respond_to_tool_confirmation(&self, tool_id: &str, approved: bool, reason: Option<String>) -> Result<(), KernelError>` — **PASS**

Evidence: `src/crates/contracts/kernel-api/src/tools.rs:94-106`

```rust
async fn respond_to_tool_confirmation(
    &self,
    tool_id: &str,
    approved: bool,
    reason: Option<String>,
) -> Result<(), KernelError>;
```

Signature matches the brief byte-for-byte, including the `reason: Option<String>`
parameter and async-ness. Doc comment matches brief wording.

### 2. facade 路由 `coordinator.confirm_tool/reject_tool` — **PASS**

Evidence: `src/crates/assembly/core/src/kernel_facade/tools.rs:56-67`

```rust
if approved {
    coordinator.confirm_tool(tool_id, None).await
} else {
    coordinator
        .reject_tool(tool_id, reason.unwrap_or_default())
        .await
}
```

Anchor points verified live:
- `coordinator_session.rs:219` — `pub async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> NortHingResult<()>` ✅
- `coordinator_session.rs:224` — `pub async fn reject_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()>` ✅

The coordinator helper itself is `self.coordinator()?` from `kernel_facade/mod.rs:51-57`. ✅

### 3. 未初始化错误体例 = `KernelError::Runtime`（对齐 events.rs:49），不是 Internal — **PASS**

Evidence: `src/crates/assembly/core/src/kernel_facade/tools.rs:51-58`

```rust
let coordinator = match self.coordinator() {
    Ok(c) => c,
    Err(e) => {
        return Err(KernelError::Runtime(format!(
            "kernel facade not initialized — init_core not called: {e}"
        )));
    }
};
```

The underlying `coordinator()` helper at `kernel_facade/mod.rs:55` returns
`KernelError::Internal(...)` — but the implementer correctly *re-wraps* it into
`KernelError::Runtime(...)` to match the contract specified in the brief and the
pattern at `events.rs:49`. Final returned variant is **Runtime** ✅.

Side note (informational, not a finding): the underlying `coordinator()` helper
still returns `Internal` for uninit. Other call sites may still surface
`Internal` — pre-existing inconsistency, out of scope for this batch.

### 4. api.rs 薄封装，不建 AppEvent / event_bus / 子模块 — **PASS**

Evidence: `src/apps/desktop/src/ui_dioxus/api.rs` (116 lines) and
`src/apps/desktop/src/ui_dioxus/mod.rs:26` (`mod api;`).

- 5 free functions: `submit_turn`, `stop_turn`, `list_sessions`, `get_session`,
  `respond_to_tool_confirmation`. No `pub mod` submodules. ✅
- `submit_turn` constructs `TurnInputDto` with the documented defaults
  (`mode: "agentic".into()`, `policy: { allow_subagent: true, max_turns: None }`,
  `source: TriggerSourceDto::User`, `workspace_path: None`)
  — matches brief §3 verbatim. ✅
- `stop_turn` / `list_sessions` / `get_session` are pure pass-through. ✅
- `respond_to_tool_confirmation` calls the new facade trait method with
  `None` for `reason` (matching brief §3 — Dioxus approval card has no reject
  text input per F3-UI's "诗意<功能" rule). ✅
- `mod.rs:26` registers `mod api;` (not `pub mod`, no re-export), per brief. ✅

### 5. event_channel: callback → mpsc(256) → Receiver — **PASS**

Evidence: `src/apps/desktop/src/ui_dioxus/api.rs:70-91`

```rust
let (tx, rx) = tokio::sync::mpsc::channel(256);  // line 71
...
let callback = Box::new(move |dto: KernelEventDto| {
    let _ = tx.try_send(dto);   // line 74
});
if let Err(e) = kernel_facade().subscribe_events(callback).await { ... }
```

- Buffer = 256 ✅
- Returns `Receiver<KernelEventDto>` ✅
- Conversion is from `subscribe_events` callback model to mpsc ✅

**Caveat**: brief said `blocking_send`; implementation used `try_send`. See
finding **M-1** below.

### 6. contracts 变更带 1 单测（未初始化 → Err）— **PASS**

Evidence: `src/crates/assembly/core/src/kernel_facade/tools.rs:73-90`

```rust
#[tokio::test]
async fn test_respond_to_tool_confirmation_returns_runtime_err_before_init() {
    let facade = super::super::KernelFacade::new();
    let result = facade
        .respond_to_tool_confirmation("tool-123", true, None)
        .await;
    match result {
        Err(KernelError::Runtime(_)) => {}
        Err(other) => panic!("expected KernelError::Runtime before init, got {:?}", other),
        Ok(_) => panic!("respond_to_tool_confirmation should return Err before init"),
    }
}
```

Test passes (verified in report log line 995). Assertion is **strong** —
explicit match on `Err(KernelError::Runtime(_))` with `panic!` on any other
variant including `Ok`. This is the gold standard for "uninitialized → Err"
contracts. ✅

### 7. 不动 event_bridge.rs / Slint / session_mock.rs；api.rs 不依赖 dioxus — **PASS**

Diffstat covers exactly 6 files:
- brief + report (self-tracked metadata) ✅
- `src/apps/desktop/src/ui_dioxus/api.rs` (new) ✅
- `src/apps/desktop/src/ui_dioxus/mod.rs` (1-line `mod api;`) ✅
- `src/crates/assembly/core/src/kernel_facade/tools.rs` (+43) ✅
- `src/crates/contracts/kernel-api/src/tools.rs` (+13) ✅

No touches to `event_bridge.rs`, Slint UI tree, or `session_mock.rs`.

api.rs imports (lines 6-11):
```rust
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi};
use northhing_kernel_api::session::{KernelSessionApi, SessionDto, SessionId, SessionSummaryDto};
use northhing_kernel_api::tools::KernelToolsApi;
use northhing_kernel_api::turn::{KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto};
```

**No `dioxus` / `tauri` / `slint` imports.** The file is a pure async/tokio
bridge — testable in isolation. ✅

## Skeptical checks

### 复用核查（api.rs 是否全部走 kernel_facade()）— **PASS**

Full grep of `src/apps/desktop/src/ui_dioxus/api.rs` for raw subsystem
references:

- `kernel_facade()` — 6 call sites (lines 31, 43, 48, 53, 61, 76) ✅
- `ConversationCoordinator` — 0 matches
- `global_scheduler` — 0 matches
- `DialogScheduler` — 0 matches
- `northhing_core::kernel_facade` — only the import; no `::coordinator` /
  `::events::subscribe` / `::session` direct access

api.rs routes exclusively through the facade. No bypass. ✅

### 无 owner 抽象（是否引入新 trait / struct 包装）— **PASS**

Diff contains no new `pub trait`, no new `pub struct`, no new `pub mod`. The
five api functions are free functions; `event_channel` is one free function.
No `AppEvent`, no `event_bus.rs`, no `EventAdapter` — per brief prohibition. ✅

### "Deviations: None" 诚实性 — **Minor (M-1)**

Brief §3 for `event_channel`:
```rust
//     let _ = tx.blocking_send(dto);  // 满则丢——非 Critical 事件可接受
```

Implementation at `api.rs:74`:
```rust
let _ = tx.try_send(dto);
```

This is a real deviation: `blocking_send` → `try_send`.

**Substantive assessment** (per the reviewer hint): the brief's `blocking_send`
in a callback context risks **deadlocking the kernel event pump** — the callback
is invoked from the kernel's event dispatch loop, and a full channel would
block the dispatcher thread until the Dioxus receiver drains, which it cannot
do because the event pump is blocked. `try_send` returns `Err(TrySendError::Full)`
immediately and the `let _ =` discards it — matching the brief's "满则丢" intent.

The implementer **did** add a code comment explaining the rationale:
`api.rs:69` — "Events that exceed channel capacity are dropped to prevent
stalling kernel event delivery."

**However**, the report's Deviations section reads "None." This is a
report-honesty gap, not a code defect. The deviation is technically sound and
should have been declared.

**Severity rationale**: This is **Minor**, not Important, because:
1. The change is technically correct and aligned with the brief's stated
   semantics ("满则丢").
2. It does not change observable behavior beyond what the brief already
   intended (drop on full).
3. The code comment justifies the choice in-place.

It does **not** warrant a fixer round. Ledger row should note the
under-reporting for future reference.

### 测试有效性 — **Minor (M-2)**

**facade 单测** (`test_respond_to_tool_confirmation_returns_runtime_err_before_init`):
asserts `Err(KernelError::Runtime(_))` with explicit `panic!` on other variants.
Strong ✅.

**api.rs 单测** (`test_api_functions_fail_cleanly_before_init`, lines 99-107):
```rust
let _ = submit_turn("test-session", "hello".into()).await;
let _ = stop_turn(&"test-turn".to_string()).await;
let _ = list_sessions().await;
let _ = get_session(&"test-session".to_string()).await;
let _ = respond_to_tool_confirmation("call-1", true).await;
```

The `let _ =` swallows results. The test only proves "doesn't panic", not
"returns Err". The header comment ("Facade is uninitialized in isolated test
environment, should return Err not panic") describes intent the test does not
verify.

**Severity rationale**: This is **Minor**, not Important, because:
1. The behavioral contract (pre-init → Err) is already strongly asserted by
   the facade test (M-1's coverage).
2. api.rs thin wrappers should propagate the underlying error verbatim; if
   facade returns `Err(KernelError::Runtime)`, api.rs is mechanically
   guaranteed to surface the same.
3. Strengthening to `assert!(matches!(..., Err(KernelError::Runtime(_)))` would
   be one-line per function — easy housekeeping fix.

Worth tracking for future hygiene, not a blocker.

**Bonus check** — `event_channel` test (lines 110-113): creates `rx`, drops it,
no assertions. Acceptable smoke test for "function returns a Receiver".

## Verification commands (re-checked against report output)

| Command | Report status | Assessment |
|---|---|---|
| `cargo check --workspace` | warnings only (40 in northhing bin, 18 in core), Finished 2.56s | ✅ PASS — no errors, only pre-existing dead-code warnings |
| `cargo check -p northhing --features ui-dioxus` | warnings only, Finished 2.00s | ✅ PASS |
| `cargo test -p northhing-core --features product-full kernel_facade` | 36 passed; 0 failed | ✅ PASS — new test `test_respond_to_tool_confirmation_returns_runtime_err_before_init` runs and passes |

## Findings

### Critical
*(none)*

### Important
*(none)*

### Minor

**M-1** — `task-p0a-bridge-report.md` declares `## Deviations from Brief — None.`
while the implementation changed `event_channel`'s send from `blocking_send` to
`try_send` (`api.rs:74` vs brief §3). The change is technically sound (avoids
callback-blocking event pump deadlock, matches brief's "满则丢" intent) and
documented inline, but was not declared in the report's deviations section.
Severity: Minor (no behavioral regression, no code change needed; report
fidelity gap only).

**M-2** — `test_api_functions_fail_cleanly_before_init` (`api.rs:99-107`) uses
`let _ =` to swallow results; it asserts only "no panic" rather than the
documented "should return Err". Severity: Minor (facade test already strongly
covers pre-init Err contract; api.rs wrappers are mechanically transparent).

### Observations (informational, not findings)

- **Dead-code warnings** in `api.rs` (5 functions unused) and other
  `ui_dioxus/` modules are expected — these are bridge functions whose Dioxus
  consumers land in P0b/P0c. Out of scope for this batch.
- **`event_channel` subscription leak**: when the receiver is dropped, the
  kernel's internal subscriber list keeps a dangling entry that silently
  fails. Pre-existing in the brief's design pattern (`std::thread::spawn` +
  no `unsubscribe_events` call). Not introduced by this batch; recommend a
  later `Drop` guard or explicit unsubscribe path when P0b wires
  `use_future` to consume the receiver.
- **Coordinator helper inconsistency**: `kernel_facade/mod.rs:55` returns
  `KernelError::Internal` for uninit; this batch correctly re-wraps to
  `KernelError::Runtime` in the new method, but other call sites using
  `self.coordinator()?` will still surface `Internal`. Out of scope; flag for
  a future consistency pass.

## Final Verdict

**APPROVE**

### SPEC verdict: **PASS**
All 7 constraints satisfied with cited evidence. Trait signature verbatim,
facade routes correctly, error variant aligned with `events.rs:49`, api.rs is
a thin wrapper with no new abstractions, event_channel has correct buffer size
and return type, the contract test strongly asserts the pre-init Err contract,
and no out-of-scope files were touched.

### QUALITY verdict: **PASS with 2 Minor**
- M-1 (report honesty): deviation is technically sound and in-place-commented;
  not a blocker.
- M-2 (api.rs test strength): behavioral contract covered by facade test;
  strengthening is housekeeping, not a blocker.

No Critical or Important findings. No code changes required for this batch.
Recommended follow-ups (tracked in ledger, addressed in future batches):

1. **M-1 follow-up**: amend report's Deviations section to declare the
   `blocking_send → try_send` change with the inline rationale. Can be a
   one-line addendum or addressed during next batch's handoff.
2. **M-2 follow-up**: tighten `test_api_functions_fail_cleanly_before_init`
   to `assert!(matches!(... , Err(KernelError::Runtime(_))))` for each call.
   Five-line patch.
3. **Informational**: consider `Drop` guard on `event_channel` receiver, or
   wire explicit `unsubscribe_events` in P0b's `use_future` cleanup, to
   prevent dangling subscriber registrations.
4. **Informational**: align `coordinator()` helper's uninit error variant to
   `KernelError::Runtime` (or add a separate helper) for consistency across
   all facade entry points.

## Ledger-ready row

```
Task P0a: complete (commits 9bba819..4889d22, review APPROVE with 2 Minor)
- constraints 1-7: PASS
- M-1: deviation report honesty (try_send vs blocking_send); not blocking
- M-2: api.rs pre-init test strength; not blocking (facade test covers)
```

— judge-m3, 2026-08-26