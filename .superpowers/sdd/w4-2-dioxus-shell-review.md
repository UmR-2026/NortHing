# Dioxus Shell Audit — W4-2

**Date:** 2026-08-28  
**Scope:** `src/apps/desktop/src/ui_dioxus/` (16 modules), `main.rs`, `app_state/{settings,log,turn_runtime}`  
**Context:** Slint shell physically deleted (70bc4e8..0c95aa6). Dioxus is the sole UI layer.

---

## Critical

### F1 · `quit_shell()` calls `process::exit(0)`, bypassing all cleanup

**File:** `app.rs:763-765`

```rust
fn quit_shell() {
    std::process::exit(0);
}
```

Triggered by the room chrome ✕ button (`app.rs:433-436`).

**What:** The room window's close path terminates the process immediately. No Drop impls run — `WindowDropGuard` for all module windows never fires, `ShellWindowManager` retains stale state in-memory, geometry-follow threads are OS-killed, and the worker thread (tokio runtime with core services, cleanup scheduler, MCP servers) never receives the graceful-shutdown signal from `main.rs:98`.

**Why it matters:** The graceful-shutdown path exists and is correct (`shutdown_tx.send(())` → worker exits → `shutdown_mcp_servers()` runs). It is unreachable from the room close button because `process::exit` terminates before `ui_dioxus::launch()` returns. MCP subprocesses, the daily cleanup scheduler, and the debug-log consumer thread all die without cleanup.

**Fix direction:** Replace `process::exit(0)` with a close signal (e.g., set a shutdown flag via a context-provided `watch::Sender` or call `window().close()` on the room, which returns control to `launch()` and lets `main.rs` proceed). Effort: **S** — one function body change.

---

## Important

### F2 · Event channel `try_send` drops events on lag; `TurnState` loss causes streaming hang

**File:** `api.rs:191-193`

```rust
let callback = Box::new(move |dto: KernelEventDto| {
    let _ = tx.try_send(dto);  // silently drops when channel full (cap 256)
});
```

**What:** The Dioxus shell bridges the kernel's 1024-cap broadcast to a `mpsc::channel(256)` via `try_send`. When the UI task is slow (initial render, heavy re-render), the channel fills and events are dropped. The consumer in `app.rs:158-253` handles three event types:

| Event type | Drop impact |
|---|---|
| `TextChunk` | Streaming text gaps (cosmetic — draft accumulates partial text) |
| `ToolCall(AwaitingConfirmation)` | Approval card silently missing (correctness gap) |
| `TurnState::Completed/Failed/Cancelled` | **Streaming flag never resets, draft never committed — UI hangs permanently** |

The `TurnState` loss is the worst case: after `submit_turn` sets `streaming = true`, a dropped `Completed` event means the user sees an infinite streaming indicator and the draft never becomes a chat entry. Recovery requires navigating away and back (re-mounts the component and resets signals).

**Why it matters:** This compounds the kernel broadcast's own lag-drops (audit r2#4 unfix). Two levels of silent dropping, and the Dioxus shell's consumer has no backpressure or gap-detection mechanism.

**Fix direction:** Options: (a) use `tokio::sync::mpsc::Sender::send` (blocking) instead of `try_send` — would backpressure the kernel broadcast callback, which may be undesirable; (b) prioritize `TurnState` events by peeking or using a priority channel; (c) add a heartbeat/heartbeat-miss detection in the consumer to reset `streaming` if no `TurnState` arrives within a timeout. Effort: **M**.

### F3 · Three raw `std::thread` geometry-follow polling loops (r1-desktop #8, unfixed)

**Files:** `windows.rs:136-178` (self), `399-446` (facility), `599-645` (work)

```rust
std::thread::Builder::new()
    .name("self-geometry-follow".into())
    .spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            if rx.has_changed().is_err() { break; }
            // ... SetWindowPos ...
        }
    })
```

**What:** Each module window spawns a dedicated OS thread that polls the geometry watch channel at ~60Hz and calls `SetWindowPos` to reposition. Exit conditions are `rx.has_changed().is_err()` (channel closed = room destroyed) and `IsWindow(hwnd) == 0` (OS window gone).

**Why it matters:** This is the unfix from audit r1-desktop #8. The 2026-08-14 re-spike proved that `use_future`-based polling causes a busy-spin at ~97% CPU in dioxus 0.8-alpha.1 (entry.rs:180-187). The raw-thread workaround works and exits cleanly on window close, but: (a) 3 dedicated OS threads for positioning is overhead; (b) the 16ms poll interval means up to 16ms lag between room move and module follow; (c) threads are fire-and-forget — if one panics, there's no JoinHandle to observe; (d) the exit-via-IsWindow check is a polling dead-reckoning — between `SetWindowPos` and the next `IsWindow` check, a race is theoretically possible.

**Fix direction:** Long-term requires dioxus event-loop integration (rust/wry event handler on module windows) or tao's `Event::LoopDestroyed` hook. The current workaround is acceptable for the alpha. ponytail: leave as-is until dioxus 0.8 stable exposes proper multi-window event hooks. Effort: **L** (waiting on upstream).

### F4 · Onboarding tests provider connectivity but never persists the provider config

**File:** `pages_onboarding.rs:672-705`

```rust
// Step 3 completion:
if let Err(e) = super::api::store_provider_api_key("onboarding", &key_val).await { ... }
let update_res = crate::app_state::settings::update_app_settings(|s| { ... }).await;
if let Err(e) = northhing_core::kernel_facade::kernel_facade().create_session(session_config).await {
    tracing::warn!("onboarding create_session best-effort error: {e}");
}
```

**What:** The 3-step onboarding flow tests the provider (`test_provider_config`), stores the API key in the OS keyring under account `"onboarding"`, and creates a session with `model_name: "default"`. But it never creates a `ProviderConfigDto` and persists it to the global config. No `KernelSettingsApi::upsert_provider_config` or `upsert_model_config` call exists in the shell.

**Why it matters:** After onboarding completes:
- The keyring has a key under `"onboarding"` that no provider references
- The global config has no providers
- `create_session(model_name: "default")` depends on a `default_provider_id` existing; if none is set, the session creation is a no-op (logged as best-effort warn, per line 700)
- The user arrives at an empty settings page with no providers configured

This is the Dioxus counterpart to the Slint onboarding flow which presumably did persist the provider.

**Fix direction:** After a successful `test_provider_config`, build a `ProviderConfigDto` from the form fields and call `kernel_facade().upsert_provider_config(...)` + `set_default_provider(...)` before `create_session`. Effort: **M** — ~20 lines in the spawn block.

---

## Minor

### F5 · `ModuleAppProps::PartialEq` always returns `true` — fragile prop-diffing hack

**File:** `registry.rs:39-43`

```rust
impl PartialEq for ModuleAppProps {
    fn eq(&self, _other: &Self) -> bool { true }
}
```

**What:** Dioxus's VirtualDom uses `PartialEq` on props to decide whether to re-render. Returning `true` unconditionally means prop changes (e.g., a new `theme_rx` after window re-open) never trigger re-render. The current design works around this via async channels (`watch::Receiver`) and `use_future`, but the hack is undocumented and fragile — any future developer adding a time-sensitive prop will find it silently ignored.

**Fix direction:** Either implement proper `PartialEq` (comparing the fields that matter for re-render), or add a comment explaining the intentional hack. Effort: **S**.

### F6 · `std::sync::Mutex` in entry.rs shared state

**File:** `entry.rs:139-140`

```rust
let room_window_id: Arc<Mutex<Option<WindowId>>> = Arc::new(Mutex::new(None));
let latest_geometry: Arc<Mutex<Geometry>> = Arc::new(Mutex::new(initial_geometry));
```

**What:** These are accessed from the tao event handler (entry.rs:222) and from `use_effect` in `app.rs:261` — different threads. `std::sync::Mutex` blocks the thread during acquisition. Under current code, locks are held briefly and no `.await` occurs inside, so there's no actual deadlock. But the pattern is a footgun — if future code acquires one of these mutexes and then awaits, it blocks a tokio worker thread.

**Fix direction:** Replace with `tokio::sync::watch` for `room_window_id` (it's a single-writer, multi-reader pattern) or use `dioxus::signals::global` for cross-component state. Effort: **S**.

### F7 · Settings page has no provider edit UI

**File:** `pages_settings.rs:444-488`

**What:** The Dioxus settings page lists providers and allows setting a default (click → `set_default_provider`), but has no edit form. Users cannot modify an existing provider's name, URL, model, or API key from the Dioxus shell. The Slint-side edit flow (which had the "silently erase API key" bug, fixed via `resolve_edit_api_key` in `sync.rs`) is now deleted. The `resolve_edit_api_key` function and `resolve_effective_api_key` in `app_state/settings/sync.rs` are dead code (`#[allow(dead_code)]`) — they're never called from the Dioxus shell.

**Why it matters:** Not a bug — a missing capability. Users who need to edit a provider must use the CLI or another surface. The fix for the Slint bug (r1#1) does not carry over because the edit UI doesn't exist in the Dioxus shell.

**Fix direction:** Add an edit-mode form to `pages_settings.rs` that reuses `resolve_edit_api_key` for key-preservation-on-clear. Effort: **L** (form + save path + keyring integration).

---

## Checked and Cleared

| Area | Finding |
|---|---|
| **Window lifecycle close ordering** | `WindowDropGuard` runs on VirtualDom drop → `notify_closed_with_gen` → manager state cleaned up. Geometry-follow threads exit via `IsWindow`/channel-close checks. Exception: F1 (`quit_shell` → `process::exit`). |
| **Event chain lag propagation** | Consumer processes events sequentially in a single `use_future` loop. No stall propagation to the kernel (the mpsc drops events rather than blocking). The risk is单向: Dioxus loses events, not the kernel. Documented in F2. |
| **State management (state.rs, registry.rs)** | No `std::sync::Mutex` held across `.await`. `ShellWindowManager` acquires the mutex briefly (no await inside). `GlobalTheme` uses `watch::Sender` (no mutex). No leaked tasks: `use_future` loops break on channel-close, which happens when the component unmounts. |
| **Entry/app startup ordering** | Core init failure is handled: `runtime.block_on(initialize_core_services())` → `process::exit(1)` on failure (line 82). Shell only launches after worker thread starts. `ensure_room_session` retries on send. UI shows seed session + banner when core is unavailable. Documented in F9 note. |
| **`session_mock.rs` in production** | Correctly compiled into production — provides `seed_session()` (initial mock data), `messages_to_entries()` (kernel message → UI entry converter). Not dev-only; not gated; serves as the production message-mapping layer. |
| **i18n consistency** | `LocalePack::load` reads `.ftl` files from `src/crates/assembly/core/locales/`. Falls back to key names on missing/corrupt files. `DEFAULT_LOCALE = "zh-CN"`. All chrome/aria text routes through `locale.t()`. Some inline Chinese literals exist (e.g., windows.rs:321 "还宽，慢慢来") — consistent with i18n-frozen policy of hardcoded Chinese, but stylistically inconsistent with the `locale.t()` pattern. |

---

## Sample Coverage Note

**Deep-read (full content):** All 16 `ui_dioxus/` modules, `main.rs`, `app_state/settings/{mod,types,keyring,io,sync,integrity}.rs`, `app_state/log.rs`, `app_state/turn_runtime.rs`.

**Skimmed:** `app_state/settings/io.rs` test module (218 lines of test scaffolding — patterns verified, individual test cases not line-by-line reviewed). `css.rs` OVERLAY_CSS section for E3-S6 settings window (727-746 line block, single-line rules, structural pattern verified).

---

## Effort Summary

| Finding | Severity | Effort |
|---|---|---|
| F1 `process::exit(0)` | **Critical** | S |
| F2 Event channel lag drops | **Important** | M |
| F3 Geometry-follow threads | **Important** | L |
| F4 Onboarding missing provider persist | **Important** | M |
| F5 `PartialEq` hack | Minor | S |
| F6 `std::sync::Mutex` shared state | Minor | S |
| F7 No provider edit UI | Minor | L |
