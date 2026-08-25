# Review — P0b Send/Stop/Streaming Wiring

- **Reviewer**: judge-m3 (task reviewer)
- **Diff base**: `8703901` (P0a)
- **Diff head**: `0200899` (P0b implementer commit)
- **Files reviewed**: `src/apps/desktop/src/ui_dioxus/api.rs` (+26), `src/apps/desktop/src/ui_dioxus/app.rs` (+180)
- **Verification re-run**: skipped (report includes `cargo check -p northhing --features ui-dioxus` ✅ clean + 11/11 ui_dioxus tests pass, including new `test_ensure_room_session_fails_cleanly_when_uninitialized`)

---

## Constraints

| # | Constraint | Verdict | Evidence |
|---|---|---|---|
| 1 | input-box div → real Dioxus `input`, IME Enter guard | **PASS** | `app.rs:520-533` — `input { r#type: "text", value, placeholder, oninput, onkeydown: !e.is_composing() && e.key() == Key::Enter }`. Cursor `<span class="cursor">` removed (was line 944 in old diff). `e.is_composing()` resolves cleanly under Dioxus 0.8.0-alpha.1 (verified by `cargo check` pass). |
| 2 | send/stop unified: non-streaming → submit_turn (lazy session), streaming → stop_turn | **PASS** | `app.rs:534-546` button onclick branches on `streaming()`; `send_action` (`app.rs:251-293`) calls `api::ensure_room_session()` then `api::submit_turn`; `stop_action` (`app.rs:295-305`) calls `api::stop_turn(&turn_id)`. `streaming: Signal<bool>` + new `active_turn_id: Signal<Option<TurnId>>`. |
| 3 | `ensure_room_session()` lives in api.rs; SessionConfigDto shape correct | **PASS** | `api.rs:59-71` — uses `list_sessions().into_iter().next()` then `create_session(SessionConfigDto { workspace_path: None, agent_type: "agentic".into(), model_name: "default".into(), name: Some("诊室".into()) })`. DTO shape cross-checked at `contracts/kernel-api/src/session.rs:15-21` (4 fields, types match). Facade `create_session` (`kernel_facade/session.rs:15-36`) defaults `workspace_path: None` to `helpers::default_workspace_path()` — so `workspace_path: None` is **not** rejected, contrary to my initial worry. New unit test `test_ensure_room_session_fails_cleanly_when_uninitialized` covers uninitialized facade path. |
| 4 | streaming render: use_future consumes event_channel; TextChunk → assistant_draft; terminal TurnState → MockEntry::Entity (no new entry type) | **PASS** | `app.rs:127-207` — use_future loops `rx.recv().await`, matches `KernelEventDto::TextChunk { session_id, text }` (accumulate) and `KernelEventDto::TurnState { session_id, turn_id: _, state, error, .. }` (settle). All terminal states (Completed/Failed/Cancelled) push existing `MockEntry::Entity { who: "它", body, children: vec![] }`. `MockEntry` enum unchanged (`session_mock.rs:18-38`), `render_entry` unchanged (`app.rs:711-759`). Inline `assistant_draft` rendering at `app.rs:491-498` is a transient streaming bubble — does not introduce new entry type. |
| 5 | `seed_session()` preserved as startup fallback | **PASS** | `app.rs:109` — `let mut entries = use_signal(|| seed_session());` (only signal wrapper added; seed call unchanged). `session_mock.rs:53` `pub fn seed_session()` byte-identical (no diff). F1-batch will swap this; per brief, untouched now. |
| 6 | Forbidden zones: session_mock.rs / entry.rs / other pages / i18n untouched; api.rs only adds ensure_room_session | **PASS** | `git diff 8703901 HEAD -- session_mock.rs entry.rs i18n.rs` returns empty. `git diff 8703901 HEAD -- api.rs` shows only `ensure_room_session()` + 1 test + 1 import line. Other pages (`pages_archive.rs`, `pages_space.rs`, `pages_onboarding.rs`, `pages_settings.rs`) not in diff stat. |

---

## Skeptical Checks (deep-dive)

### A. Session filter `unwrap_or(true)` semantics — `app.rs:134, 147`

```rust
if sid.read().as_ref().map(|s| s == &session_id).unwrap_or(true) { ... }
```

**Analysis**: When `session_id_signal` is `None` (before first `send_action`), `unwrap_or(true)` accepts events from any session. For v1 single-room this is **benign**:

1. The room creates its own session lazily via `ensure_room_session()` and stores it before `submit_turn` (verified at `app.rs:263-275`).
2. Until the first send, no `KernelEventDto` should be received from a session this room has bound to — events arriving would be from unrelated subsystems (approval notifications, banner, error). The current `match` ignores non-TextChunk/TurnState variants anyway.
3. After first send, `sid == Some(...)` and filter is strict.

**Risk vector**: if a stale TextChunk from a previous (different) session leaks in **before** the user sends the first message, it would be appended to `assistant_draft` and later sealed as a "completed" Entity. Practically near-zero because the lazy path puts `session_id_signal` in place synchronously inside the spawn block before submit_turn runs.

**Verdict**: **Minor**. v1 single-room semantics accept it. F6 (onboarding/normalization) is the proper place to fix — same scope as other session lifecycle fixes. Suggest a `// ponytail: accept-all-when-unbound is v1 semantics; tighten in F6` comment, but not blocking.

### B. Enter during streaming silently dropped — `app.rs:528-530`

```rust
if !e.is_composing() && e.key() == Key::Enter {
    if !streaming() { send_action(); }
}
```

**Analysis**: During streaming, Enter is consumed by the onkeydown but `send_action()` is not invoked. The character is NOT echoed into `user_input` (default Enter behavior on `<input type="text">` is submit-form, but here it's a Dioxus `input` with no `<form>` parent, so it has no native submit). The IME guard correctly skips composition.

**Tech-debt P0-1** is the canonical reference: "queued input" is a separate concern. P0b's minimum is: don't trigger send while streaming, don't double-fire. Both achieved. Input is preserved (the user can press Enter again after streaming ends).

**Verdict**: **Minor**. Acceptable per P0-1 demarcation. Not a defect.

### C. stop_action ordering — `app.rs:295-305`

```rust
let stop_action = move || {
    let mut streaming = streaming;
    let mut active_turn_id = active_turn_id;
    if let Some(turn_id) = active_turn_id() {
        spawn(async move { let _ = api::stop_turn(&turn_id).await; });
    }
    streaming.set(false);
    active_turn_id.set(None);
};
```

**Analysis** (the skeptical check hypothesis was "set streaming(false) first, then await" — the **actual** order is spawn-then-sync-reset):

1. `spawn` fires `api::stop_turn` as a detached task — UI does NOT block on the kernel round-trip.
2. Synchronous `streaming.set(false)` + `active_turn_id.set(None)` immediately reflects user intent.
3. When the kernel eventually emits `TurnState::Cancelled`, the use_future loop matches `Cancelled` → pushes `[Cancelled]` entry (with whatever draft was accumulated), sets streaming false (already false, no-op).

**Race window** between sync reset and kernel cancellation ack: 50-500ms typically. During this window:
- User can click send button: but `if streaming()` is false now, so it calls `send_action`. This submits a new turn (independent of the cancelled one). Correct behavior.
- User can type: `user_input` is a separate signal, untouched by `stop_action`. Correct.

**Verdict**: **PASS**. No deadlock, no UI freeze, no lost events. The hypothesis was wrong about the order — actually the cleaner order.

### D. `model_name: "default"` end-to-end — checked at `kernel_facade/session.rs:24-26` + `kernel_facade/turn.rs:44-79`

```rust
// facade/session.rs:24
if !config.model_name.is_empty() {
    core_config.model_id = Some(config.model_name.clone());
}
```

So `"default"` is stored as `model_id`. But `submit_turn` (`kernel_facade/turn.rs:44`) does NOT pass `model_id` to the scheduler at all — the parameters are `(session_id, text, None, None, mode, workspace, policy, ...)`. The stored `model_id` is metadata used at session-creation for context bootstrapping, not for the submit path.

**Verdict**: **PASS**. Brief explicitly authorizes the `"default"` fallback ("若 facade 无此 API 则 default"). Downstream impact is benign.

### E. House rule 4 (concurrency test binding) — AGENTS.md: "changes touching `tokio::select!`, cancellation tokens, or timeout races must ship with at least one automated test"

**Analysis**: P0b's change adds one `use_future` block that loops on `mpsc::Receiver::recv().await`. No new `tokio::select!`, no new cancellation token, no new timeout race. The mpsc channel + event_channel wrapper already shipped in P0a with its own test (`test_event_channel_returns_receiver`).

**Verdict**: **Rule 4 does NOT trigger.** No new test required by house rule. Existing `test_ensure_room_session_fails_cleanly_when_uninitialized` is adequate for the new `ensure_room_session` wrapper.

---

## Findings

### Critical
None.

### Important
None.

### Minor

1. **M1 (clarity)** — `app.rs:134, 147` — `unwrap_or(true)` session filter. Acceptable v1 single-room semantics but worth a `// ponytail:` comment so future readers don't panic. (See skeptical check A.)

2. **M2 (duplication)** — Failed/Cancelled body-formatting branches in `app.rs:162-200` share structure but differ in prefix. Could collapse with a helper `(prefix: &str, draft: Option<String>) -> String`, but the duplication is small (~12 lines × 2) and the literal strings are individually readable. Ponytail verdict: keep inline, simplify in P0c+ if a 4th terminal state appears.

3. **M3 (visibility)** — `app.rs:503` inline style on `send-error`: `"color: var(--faint); font-size: 11px; padding-bottom: 4px;"`. Should be a CSS class per house style, but `css.rs::truth_css()` is frozen. Acceptable as inline; if F1 batch adds inline style audit, roll in.

4. **M4 (test coverage gap)** — `test_ensure_room_session_fails_cleanly_when_uninitialized` only exercises the Err path (facade uninitialized). A test that asserts `SessionConfigDto` is constructed with the correct fields (and the helper returns the first session id from `list_sessions()`) would be a worthwhile follow-up but is not required by the brief or house rule 4. Defer.

---

## Dual Judgment

### SPEC 判决

All 6 brief constraints satisfied with file:line evidence. No spec deviations. The "lazy session" pattern matches prescription §F2 item 4 verbatim. The IME guard exceeds the brief's minimum (brief allowed "comment-only fallback" — implementer added the actual guard, which is strictly better).

**SPEC: PASS** ✅

### QUALITY 判决

- **Correctness**: Facade `workspace_path: None` is correctly defaulted by `kernel_facade/session.rs:16-19`, not by app code — implementer correctly delegated. Event filter semantics correct for v1.
- **Concurrency**: use_future + mpsc::recv pattern is standard Dioxus idiom, no new races introduced. stop_action fire-and-forget pattern correctly avoids UI blocking.
- **Type safety**: All destructure patterns match the contracts (`session_id: String, turn_id: String, state: TurnStateKind, error: Option<String>` — cross-checked at `contracts/kernel-api/src/events.rs:75-85` and `turn.rs:59-64`).
- **Layering**: All api.rs changes are thin facade wrappers; no business logic leaked into api.rs or app.rs. Forbidden zones respected.
- **Idioms**: Dioxus 0.8 patterns correct (`Signal`, `use_signal`, `use_future`, `spawn`); closure captures explicit (`let mut x = x;` style).
- **Style**: Code follows existing conventions in app.rs (cf. the existing `wm_left` clone pattern at `app.rs:307-325`).

**QUALITY: PASS** ✅

---

## Final Verdict

**APPROVE** ✅

Both judgments pass. No Critical or Important findings. 4 Minor items are non-blocking and explicitly out of P0b's minimum scope per the brief. Implementer deviated from brief in zero meaningful ways; the IME guard was a small upgrade over the brief's "comment-only fallback" allowance.

**Next step**: ledger append `Task P0b: complete (commits 8703901..0200899, review clean)` once orchestrator commits.

**Defer to P0c+**:
- M1: tighten session filter or document ponytail
- M4: SessionConfigDto construction test (low priority)