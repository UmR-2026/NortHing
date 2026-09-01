# W5-2 Implementation Report: Event Channel Tiering (F2)

## 1. What was Implemented

- **File & Lines:** `src/apps/desktop/src/ui_dioxus/api.rs:188-294`, tests at `src/apps/desktop/src/ui_dioxus/api.rs:452-595`
- **Mechanism Choice:**
  - **Selected Mechanism:** Unified FIFO Stream with Atomic Lossy Accounting.
    - Underlying channel: `tokio::sync::mpsc::unbounded_channel::<KernelEventDto>()`.
    - Lossy backpressure: `Arc<AtomicUsize>` tracks pending `KernelEventDto::TextChunk` instances in the queue, bounded by `MAX_PENDING_TEXT_CHUNKS = 256` via a lock-free CAS loop.
    - Guaranteed control delivery: All other event variants (`TurnState`, `ToolCall`, `TurnPhase`, `Banner`, `Error`) bypass the text chunk count check and are sent unconditionally via `tx.send(dto)`.
    - Consumer wrapping: `EventReceiver` decrements the atomic counter upon receiving `TextChunk` events and exposes `pub async fn recv(&mut self) -> Option<KernelEventDto>`.
    - Isolation & Testing: `create_event_bridge()` exports `(callback, EventReceiver)` for unit testing without requiring an initialized global kernel facade.
- **Mechanism Trade-off & Cost Analysis (裁定说明):**
  - *Why not dual channels with `tokio::select!`?* Dual channels (`control_rx` + `data_rx`) suffer from race conditions and priority inversion. If `TurnState::Completed` arrives on the control channel while lagging `TextChunk`s remain in the data channel, prioritizing control events commits the assistant draft prematurely, leaving subsequently drained `TextChunk`s orphaned.
  - *Why Unified FIFO + Atomic Cap?* Guarantees strict chronological FIFO ordering across all events. Text chunks are dropped if and only if the UI consumer has fallen behind by 256 chunks; state transitions (`TurnState::Completed/Failed/Cancelled`) and approval cards (`ToolCall(AwaitingConfirmation)`) are never dropped.
  - *Cost:* Bounded memory footprint (max 256 text chunks in queue buffer; control events are minimal per turn ~1-2 items).

## 2. 复用侦察 (Reconnaissance & Reuse)

- **Channel Primitives:** Reused standard `tokio::sync::mpsc::unbounded_channel` and `std::sync::atomic::AtomicUsize`.
- **Consumer Interface:** Reused exact `rx.recv().await` signature so `src/apps/desktop/src/ui_dioxus/app.rs:170-268` consumer loop requires zero syntactic changes and preserves its streaming reset semantics.

## 3. Verification Commands & Outputs

### 3.1 `cargo check -p northhing`
```text
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.19s
```

### 3.2 Focused Unit Tests (`ui_dioxus::api`)
Command:
`& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --bin northhing -- ui_dioxus::api`

Output:
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 21.30s
     Running unittests src\main.rs (target\debug\deps\northhing-b25540c259ba06d3.exe)

running 9 tests
test ui_dioxus::api::tests::test_pick_room_session_empty_groups_returns_none ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_hit ... ok
test ui_dioxus::api::tests::test_pick_room_session_preferred_miss_returns_none ... ok
test ui_dioxus::api::tests::test_pick_room_session_no_preferred_picks_first_non_empty ... ok
test ui_dioxus::api::tests::test_event_channel_returns_receiver ... ok
test ui_dioxus::api::tests::test_tiered_event_channel_drain_refills_budget ... ok
test ui_dioxus::api::tests::test_tiered_event_channel_text_chunk_lossy_control_guaranteed ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 98 filtered out; finished in 0.02s
```

### 3.3 Full Desktop Test Suite (`cargo test -p northhing`)
Command:
`& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing`

Output:
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.13s
     Running unittests src\lib.rs (target\debug\deps\northhing-975f8423d7ff303b.exe)

running 107 tests
...
test result: ok. 107 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

## 4. Compile Errors and Layer Resolution

- `E0369`: `binary operation == cannot be applied to type TurnStateKind` in `api.rs` test assertions -> Fixed at **Layer 1 (Language Mechanics)** by using pattern matching `matches!(state, TurnStateKind::Completed)` rather than `assert_eq!`.

## 5. Self-Review Findings & Concerns

- **Spec 1:** Verified. Non-`TextChunk` events (`TurnState`, `ToolCall`, etc.) use the guaranteed delivery path in `api.rs:258-260`.
- **Spec 2:** Verified. `test_tiered_event_channel_text_chunk_lossy_control_guaranteed` fills 356 chunks into the 256-cap channel and asserts `TurnState::Completed` and `ToolCall` arrive intact after draining exactly 256 chunks.
- **Spec 3:** Verified. `app.rs:158-253` consumer semantics are preserved.
- **Global Constraints:** Zero modifications outside `src/apps/desktop`. Exact 1 commit created (`87cb1f4`). No `.superpowers/` files touched by git.
