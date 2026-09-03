# Startup-Hang — Room First-Render Path Static Trace

Read-only investigation. Repo: `E:\agent-project\northing`. Continuation of
`startup-hang-static-trace.md` (cleared init gate / MCP reconnect / ENV_LOCK /
config pollution / WebView2 version / W15-1 markdown rendering code).

Scope: after clearing all upstream suspects, the hang has moved into the
**first-render path of `room_app_root`**. Last visible stdout line is
`ui_dioxus/i18n: loaded locale zh-CN (325 keys)`. The window skeleton paints
once (知序 / 会话 03 / input bar visible in screenshot), then `tao` stops
pumping events. ~100% single-core CPU the whole time. `.northhing/debug.log`
has zero writes.

**Working-tree state at investigation time** (HEAD = `6cbebbb`):
- Uncommitted `trc()` instrumentation in `main.rs` / `entry.rs` / `app.rs` —
  diagnostic markers added by orchestrator, **not in the running build**;
  running binary has no `trc` prints, hence the "silent" stdout tail after
  i18n load.
- `.superpowers/sdd/reports/startup-hang-static-trace.md` (the prior report)
  and screenshots committed as untracked.

---

## Q1 — What runs between `loaded locale zh-CN (325 keys)` and the next visible stdout line?

### 1.1 Locale load itself

`i18n.rs:44` `LocalePack::load(locale)` is sync — `std::fs::read_to_string`
on `<repo>/src/crates/assembly/core/locales/zh-CN.ftl`. Bytes confirmed via
`Format-Hex`: valid UTF-8 (`E7 AE 80 E4 BD 93` = "简体"), CRLF line endings
(`0D 0A`), no BOM. Parser at `i18n.rs:117` uses `str::lines()` which handles
both `\n` and `\r\n` correctly, so the file parses cleanly to **325 keys**
as observed. **No I/O hang source here.** Runs inside `use_hook` so it
executes exactly once.

### 1.2 Synchronous path inside `room_app_root` (after locale returns)

`app.rs:42-65` — five `use_context::<T>()` reads, one `use_hook` for the
locale pack, and nine `use_signal(...)` initializers (cheap; trivial values:
`true`, `false`, `None`, empty `Vec`, empty `HashSet`, short strings, short
`vec!["#DAD6CF".to_string(), …]`). The one non-trivial initializer is at
`app.rs:65`:

```rust
let mut active_set = use_signal(|| window_manager.subscribe_active().borrow().clone());
```

`window_manager.subscribe_active()` returns a fresh `tokio::sync::watch::Receiver<HashSet<&'static str>>`
(`registry.rs:219`); `.borrow()` returns a `Ref<'_, …>` guard; `.clone()`
clones the empty initial set. The receiver is dropped at end of statement.
Net cost is one atomic read + one empty-set allocation. **Not a hot spot.**

### 1.3 Three `use_future` registrations (closures run sync, async blocks do NOT run yet)

`app.rs:67-91` — **F1** register. Closure is `let mut session_id_signal = …; let mut entries = …; async move { match api::ensure_room_session().await { … } }`.
The closure body itself is just the two `let mut` bindings plus construction
of the async block — both `session_id_signal` and `entries` are passed-by-value
into the block (cloned signals). No I/O executed yet.

`app.rs:93-105` — **F2** register. Closure is `let wm = wm_future.clone(); async move { let mut active_rx = wm.subscribe_active(); loop { if active_rx.changed().await.is_err() { break; } active_set.set(active_r.get_ref().clone()); } }`. **No I/O.**

`app.rs:107-219` — **F3** register. **Important**: the closure BODY runs
synchronously during registration and **calls `api::event_channel()`** at
`app.rs:108`. This call is not async, but it does spawn a side task. See Q2.

### 1.4 The `use_effect` registration (`app.rs:222-237`)

Closure body is empty at register time. Fires once after first render. Calls
`window().id()` (sync tao), `room_window_id_tx.send(Some(…))` (sync watch
send), `window().outer_position()` / `window().outer_size()` (sync tao),
`geometry_tx.send(Geometry { … })` (sync watch send). All four are
non-blocking. **No I/O.**

### 1.5 `rsx!` macro + first Dioxus render

`app.rs:328-598`. Builds a non-trivial DOM (~120 nodes visible: chat-flow,
chrome buttons, gems, room-head, room-input, brand-inline SVG, chronicle
bar, etc.) and three `<style>` blocks (TRUTH_CSS + OVERLAY_CSS +
CHAT_MD_CSS, all `include_str!` byte strings). The CSS payload is included
at compile time so it costs zero I/O; it does cost memory for the `String`
copies inserted via `dangerous_inner_html`. **No hang candidate.**

After the rsx! block returns, Dioxus schedules the first render. The render
paints the WebView (screenshot evidence: 知序 / 会话 03 / input bar all
visible), then the `use_effect` fires, then the three `use_future` tasks
become eligible to be polled.

### 1.6 Implicit-spawn side task from F3 registration — **the only side-effect during render** (`app.rs:107-219`)

```rust
use_future(move || {
    let mut rx = api::event_channel();   // <-- sync, but spawns a tokio task
    let sid = session_id_signal;
    let session_allow_list = session_allow_list;
    async move {
        while let Some(dto) = rx.recv().await {
            …match dto { TextChunk | ToolCall | TurnState }…
        }
    }
});
```

`api::event_channel()` is `api_events.rs:93-112`. Step-by-step:

1. `create_event_bridge()` (sync) — creates the unbounded mpsc and the
   callback wrapper.
2. Builds an async `subscribe_task = async { kernel_facade().subscribe_events(callback).await }`.
3. `tokio::runtime::Handle::try_current()` — **succeeds on main_rt** because
   `main_rt.block_on(async { ui_dioxus::launch(...) })` runs the launch
   inside main_rt, and the current thread IS a main_rt worker (the one
   calling `block_on` is the one running `event_loop.run(...)`).
4. `handle.spawn(subscribe_task)` — schedules on main_rt.
5. Returns `rx`.

The closure body is sync; the spawn is fire-and-forget. **No I/O during
`room_app_root` registration itself.** But this is the first place where a
   background task is created that consumes cycles and may emit events that
   later re-trigger renders. See Q3.

### 1.7 Net synchronous cost between locale load and first render

Roughly: ~120 RSX nodes + three `dangerous_inner_html` style inserts + nine
trivial `use_signal` initializers + four use_future closures + one
`api::event_channel()` that spawns one background task. **Nothing here is
likely to busy-spin by itself**, but the spawn from §1.6 is a control-flow
trigger that hands work off to main_rt and the kernel event bus.

---

## Q2 — Does `api::event_channel()` or the `subscribe_task` it spawns spin?

### 2.1 The `subscribe_task` itself

```rust
let subscribe_task = async move {
    if let Err(e) = kernel_facade().subscribe_events(callback).await {
        tracing::warn!("ui_dioxus::api::event_channel subscribe failed: {e}");
    }
};
```

`kernel_facade().subscribe_events(callback)` (`kernel_facade/events.rs:41-60`)
is `async fn` but its body has no `.await` after `coordinator()`: it builds a
`KernelEventSubscriber { callback: Arc::new(Mutex::new(callback)) }` and
calls `coordinator.subscribe_internal(id.clone(), subscriber)`, which is
sync. Returns `Ok(id)` immediately. **One-shot, fast, not a spin source.**

### 2.2 The F3 main loop

```rust
while let Some(dto) = rx.recv().await {
    … match dto { … }
}
```

`rx` wraps a `tokio::sync::mpsc::UnboundedReceiver<KernelEventDto>`. Idle
behavior parks on the receiver. **No spin unless events flood.** See Q3.

### 2.3 What F3 does when it receives a `TextChunk`

`app.rs:114-120`:

```rust
KernelEventDto::TextChunk { session_id, text } => {
    if sid.read().as_ref().map(|s| s == &session_id).unwrap_or(true) {
        let mut d = assistant_draft.write();
        let cur = d.get_or_insert_with(String::new);
        cur.push_str(&text);
    }
}
```

Writing to `assistant_draft` (a Signal) marks the room scope dirty. Dioxus
then schedules a re-render. If TextChunks arrive at high frequency, this
**renders repeatedly**. With 325-key zh-CN locale, each render of the
chat-flow goes through `render_entries(...)` (`app.rs:725-737`) and
`render_entry` (`app.rs:739-783`) — for each entry, `render_markdown(body)`
invokes `pulldown_cmark::Parser` on every body.

`pulldown_cmark` is CPU work. If a flood of TextChunks (each one separate
event) arrives after first render, the room re-renders for every chunk,
parsing markdown for every message body on every render. **This is a real
candidate for 100% single-core CPU** — but only IF events are flowing.

But: user said "smoke-echo removed" and "episodes 164 dirs quarantined".
W15-1 rollback binary hangs identically. So either MCP is still emitting
events, OR the events come from somewhere else. **Candidate, but data-
dependent.**

### 2.4 What F3 does when it receives a `ToolCall { AwaitingConfirmation }`

`app.rs:121-164` — calls `api::respond_to_tool_confirmation(...)` (async,
awaits kernel), then pushes a `MockEntry::Approval {…}` into `entries`,
which also marks the room scope dirty. Again, this only fires if events
flow. **Same data-dependence as Q2.3.**

### 2.5 The F3 loop itself

`rx` is an unbounded mpsc. The recv() awaits if empty. There is no busy-
loop in F3 itself. **The loop body only runs when an event arrives.**

---

## Q3 — Could the kernel be emitting events that drive a render-storm feedback loop?

This is the **highest-ranked data-dependent candidate** for the 100% CPU +
silent-stdout + tao-starve symptoms.

### 3.1 The tao waker chain

`dioxus-desktop-0.8.0-alpha.1/waker.rs` shows the VirtualDom's waker is
`tao_waker(proxy, id)` which calls `proxy.send_event(UserWindowEvent::Poll(id))`
on every wake. The tao event loop's `launch.rs:39-40` handles `Poll(id)` by
calling `app.poll_vdom(id)` → `view.poll_vdom()` (`webview.rs:533-585`) which:

1. Polls `self.dom.wait_for_work()` once.
2. If `Ready`, calls `self.dom.render_immediate(f)` which drains every dirty
   task + every dirty scope in a while loop (no idle wait between drains).

If a render produces dirty scopes, the scheduler runs them; if a render
re-marks the scope dirty (e.g., a Signal `set` inside the render that
schedules another `set`), this loops until stable.

### 3.2 The render storm condition

For F3 to drive a render storm, the kernel must be emitting events at a
rate that **outpaces one render**. With `pulldown-cmark` parsing markdown
bodies on every render, a single render is bounded but non-trivial
(~ms-to-tens-of-ms per message body). If events arrive faster than renders
complete, the scheduler always finds dirty work and `wait_for_work` never
needs to `wait_for_event().await` — it just loops.

**Suspect data flips between 02:12 (healthy) and 23:02 (hang):**

1. **MCP server backlog** — after `smoke-echo` was removed, any pending
   events from the OS keychain re-init or reconnect attempts that were
   sitting in a kernel subscriber queue might now finally flush into the
   F3 mpsc channel. Single-instance startup, no backpressure.
2. **Kernel-side buffered events** — `coordinator.subscribe_internal`
   just appends a subscriber; it does NOT replay missed events. But
   per-session `KernelEventSubscriber::on_event` (`events.rs:31-37`) could
   be invoked from a long-lived queue when the subscriber registers late
   (subject to kernel event-bus semantics, not visible in this static
   trace).
3. **`TextChunk` lossy buffer cap** (`api_events.rs:14,
   `MAX_PENDING_TEXT_CHUNKS = 256`) only applies BEFORE the channel.
   Once in the F3 mpsc, the channel is unbounded — a backlog from a prior
   session that the room re-subscribes to could arrive in a burst.

**What to verify at runtime**: a `trc("F3:evt")` print per event would show
event rate. If F3 sees >100 events before any print after i18n load, the
render-storm hypothesis is confirmed.

---

## Q4 — Ranked suspect list (suspects that can produce 100% single-core CPU + tao starvation)

### Suspect #1 — F3 event-driven render storm (HIGHEST, data-dependent)

- **Mechanism**: unbounded mpsc → markdown re-render feedback loop
- **File:line**: `apps/desktop/src/ui_dioxus/app.rs:107-219` (F3 main loop),
  `app.rs:114-120` (TextChunk path that writes `assistant_draft`),
  `apps/desktop/src/ui_dioxus/markdown_render.rs:497` (render_markdown
  re-parses per render).
- **Spin/block shape**: spin (CPU-bound `pulldown_cmark::Parser` running in
  a hot loop because the scheduler always finds dirty work).
- **Data flip**: presence of an event source that emits >render-rate
  TextChunk / TurnState / ToolCall events to F3 after the room subscribes.
  After `smoke-echo` cleanup the MCP event stream shape may have changed in
  a way that drives more events through F3 than before.
- **Why not seen at 02:12**: MCP event volume / kernel state may have
  changed; `app.rs:114-120`'s "always-set-on-text-chunk" filter is
  `unwrap_or(true)` (app.rs:115) — if `session_id_signal` is `None` (the
  F1 task hasn't completed yet), EVERY TextChunk is treated as for-this-
  session, multiplying render triggers.

### Suspect #2 — `use_signal(|| window_manager.subscribe_active().borrow().clone())` re-subscribe on every render

- **Mechanism**: each render creates+drops a `tokio::sync::watch::Receiver`,
  which takes a shared RwLock read guard. Under heavy re-render (Suspect #1),
  this creates GC-like pressure on the watch channel's internal state.
- **File:line**: `apps/desktop/src/ui_dioxus/app.rs:65`.
- **Spin/block shape**: mild CPU allocation; not 100% on its own but
  amplifies Suspect #1.
- **Data flip**: only manifests if renders repeat; same dependency as #1.
- **Why not seen at 02:12**: same.

### Suspect #3 — `Geometry` channel re-broadcast from `WindowEvent::Moved` storm

- **Mechanism**: the custom tao event handler at `entry.rs:243-248` calls
  `geometry_tx.send_modify(...)` on every `Moved` / `Resized`. Each sends a
  notification to the single `GeometryRxArc` receiver (the one cloned into
  module windows on spawn). When NO module windows are mounted, the only
  "receiver" is the dropped sender side — but `send_modify` itself is
  cheap.
- **File:line**: `apps/desktop/src/ui_dioxus/entry.rs:243-253`.
- **Spin/block shape**: only spin if a `Moved` storm (e.g., DPI-change
  animation, snap-drag) hits the window.
- **Data flip**: window position state on Windows can flip the rate
  of `Moved` events (e.g., maximized → restored, DPI change, accessibility
  tools). A continuous `Moved` flood at 1000+ events/s would burn CPU but
  not "tao-stop" — tao is the source of these events, so it can't stop
  emitting them while busy.
- **Likely NOT the root cause** — the symptom is tao stopping, not tao
  flooding.

### Suspect #4 — `LocalePack::load` re-reading the same `zh-CN.ftl` on every render

- **Mechanism**: `use_hook` should run the closure ONCE per component
  lifetime. If dioxus-hooks 0.8.0-alpha.1 has a bug where `use_hook` re-
  invokes the closure on re-render, every render would re-read the locale
  file from disk (sync I/O, blocks render thread).
- **File:line**: `apps/desktop/src/ui_dioxus/app.rs:48` + `i18n.rs:44`.
- **Spin/block shape**: depends. Sync I/O would block the render thread
  but not spin CPU. If re-invoked many times rapidly (feedback), would
  burn CPU and disk.
- **Data flip**: file system state (file size, AV scanner activity on
  `zh-CN.ftl`) would change the cost.
- **Likely NOT the root cause** — `use_hook` is one of the most-tested
  Dioxus primitives; if it re-invoked, the entire ecosystem would be broken.
  But worth a sanity check.

### Suspect #5 — `watch::Receiver::changed()` polling under heavy churn

- **Mechanism**: F2's `loop { if active_rx.changed().await.is_err() { break; } active_set.set(active_rx.borrow().clone()); }`.
  If `ShellWindowManager::active_tx` is being hammered (every micro-task
  re-publishing), the changed() returns Ready immediately, the loop spins
  without yielding.
- **File:line**: `apps/desktop/src/ui_dioxus/app.rs:94-105`.
- **Spin/block shape**: spin if `active_tx` is updated continuously.
- **Data flip**: only happens if something is calling `mark_opening` /
  `mark_closing_target` repeatedly. The `entry.rs:235-240` CloseRequested
  handler calls `mark_all_closing_targets` once per close. The
  `mark_opening` callers are gem-button onclick handlers in app.rs — those
  fire on user clicks, not at startup.
- **Why probably NOT the cause**: nothing pushes `active_tx` at startup; F2
  parks on `changed()` indefinitely until first push.

### Suspect #6 — Dioxus scheduler micro-spin during `wait_for_work`

- **Mechanism**: `dioxus-core/virtual_dom.rs:445-461` `wait_for_work`
  loops `process_events() + has_dirty_scopes check + wait_for_event().await`.
  If `wait_for_event` returns Ready immediately (because something keeps
  sending `SchedulerMsg::TaskNotified` to `self.rx`), the loop never
  awaits and burns CPU.
- **File:line**: `dioxus-core-0.8.0-alpha.1/src/virtual_dom.rs:445-461` +
  `tasks.rs:288` (`handle_task_wakeup` poll path).
- **Spin/block shape**: spin.
- **Data flip**: only if a future re-wakes itself via `cx.waker().wake_by_ref()`
  in a tight loop. None of F1/F2/F3 do this.
- **Likely NOT the root cause** unless there's a future that yields and
  immediately wakes itself in the same poll iteration. The fact that
  the documented busy-spin poison shape is "ANY sleeping use_future"
  (`entry.rs:180-188`) suggests this COULD trigger on any awaitable in
  this Dioxus version — but the user said at 02:12 it didn't busy-spin,
  which means whatever triggers it is data-dependent (see #1).

### Suspect #7 — `subscribe_events` callback latency inside the kernel

- **Mechanism**: `coordinator.subscribe_internal(id, subscriber)` (sync) is
  called once at F3 spawn. The callback is `Arc<Mutex<Box<dyn Fn>>>` — every
  event acquires the mutex. If kernel emits events faster than the
  callback can drain, the mutex serializes them, but no spin (lock is
  short-lived).
- **File:line**: `apps/desktop/src/ui_dioxus/api_events.rs:54-77` +
  `kernel_facade/events.rs:13-27`.
- **Spin/block shape**: not on its own.
- **Data flip**: high event volume.

### Suspect #8 — tokio Mutex contention (`ROOM_SESSION_CACHE`)

- **File:line**: `apps/desktop/src/ui_dioxus/api.rs:117,122`.
- **Spin/block shape**: only spin if a `lock().await` is held while the
  task is woken repeatedly. None of F1's body wakes itself.
- **Data flip**: irrelevant — F1 acquires once, holds across
  `load_app_settings` + `list_sessions_all_workspaces`, then drops. Single
  holder, no contention possible at startup.
- **NOT the root cause**.

---

## Q5 — Data flips between 02:12 (healthy) and 23:02 (hang)

State observed on this machine now:

| Path | Content | Risk for first-render |
|---|---|---|
| `~/.northhing/config/app.json` | 34 lines, 784 bytes. 1 provider, 1 workspace (`C:\northhing-test`), **0 MCP servers**. | Low (keyring migration is a no-op) |
| `~/.northhing/projects/<slug>/` | **3999 stale workspace dirs** (all `c--users-umr-appdata-local-temp-northhing-session-restore-test-*` from prior test runs). | **NOT loaded** — `load_workspace_history_only` only reads `workspace_data.json`, not this dir |
| `~/.northhing/personal_assistant/workspace/` | Empty | Low |
| `~/.northhing/relay/api_key` | 44 bytes | Low |
| `C:\Users\UmR\AppData\Roaming\northhing\config\app.json` | **3653 lines** (Dioxus UI settings, not loaded by `load_app_settings`) | Low for the room first-render path |
| `C:\Users\UmR\AppData\Roaming\northhing\data\workspace_data.json` | **3 workspaces** (only the ones loaded into the manager at startup). | `list_sessions_all_workspaces` iterates 3 + default = ~4 disk reads |
| `C:\Users\UmR\AppData\Roaming\northhing\data/backups/` | (subdir) | Could be very large; only touched on `save_json` |
| `C:\Users\UmR\AppData\Roaming\northhing\episodes/` | 3 small dirs | Not loaded at startup |
| `C:\Users\UmR\AppData\Roaming\northhing\episodes-quarantine-20260903/` | **164 dirs** (from user cleanup) | Not loaded at startup |
| WebView2 user data (`%LOCALAPPDATA%\northhing-dioxus-dev\webview_data`) | (user reset) | Could be in an inconsistent state — wry might spin trying to migrate/clean it |

**Key state-dependent variables the first-render path reads:**

1. **`workspace_data.json`** size and `workspaces` count → drives
   `list_sessions_all_workspaces` iteration count. **Stable: 3 workspaces.
   Not a multi-second cost.**
2. **OS keyring** (`PRODUCTION_KEYRING` via `keyring` crate) — sync Windows
   Credential Manager calls in `keyring_migrate_mcp_servers` at
   `io.rs:63-85`. With 0 MCP servers in `app.json`, **zero calls**. Not the
   cause.
3. **`zh-CN.ftl`** size and parse cost → 325 keys, sub-millisecond.
4. **MCP server event volume** → drives F3 render storm. **HIGHLY
   variable, the most plausible trigger.**
5. **WebView2 user data state** → affects `wry::WebViewBuilder::build()`
   at `WebviewInstance::new` in `webview.rs:499`. If the WebView2 user
   data dir is in a corrupted state, wry may spin trying to initialize.
   The user reset this dir, so state is fresh; but a fresh dir requires
   initialization which can include file I/O.

---

## Q6 — Documented busy-spin poison shape (from prior report `entry.rs:180-188`)

> "ANY sleeping use_future in the room window — including a bare
> `loop { sleep(100ms).await }` with no window()/send calls — makes one
> background thread busy-spin at ~97% single-core CPU on dioxus
> 0.8.0-alpha.1. The polling shape itself is the poison, so geometry
> publishing must not use a future at all."

**Important re-read**: the prior fix removed the *geometry polling* use_future
(because it was sleeping with a `sleep`). The three remaining use_futures
**do not sleep** — they await primitives (tokio mutex, watch::Receiver,
mpsc::Receiver). If the busy-spin shape applies to ANY awaiting use_future
in dioxus 0.8-alpha.1, then F1/F2/F3 themselves could be the spin source,
**independent** of any data. But that contradicts the "healthy at 02:12"
baseline.

The most likely reconciliation: **the busy-spin is per-awaitable type**.
If a specific await primitive (`watch::Receiver::changed()`,
`mpsc::UnboundedReceiver::recv()`, `tokio::sync::Mutex::lock()`) yields
"correctly" in the Dioxus scheduler on this Dioxus version, no spin. If it
yields "incorrectly" (e.g., the waker chain re-wakes immediately), spin.

The `watch::Receiver::changed()` in F2 would not busy-spin unless something
keeps sending. The `mpsc::UnboundedReceiver::recv()` in F3 would not
busy-spin unless the channel has events. The `tokio::sync::Mutex::lock()` in
F1 would not busy-spin unless the lock is held elsewhere — it's not.

**Conclusion: the data flip most likely operates on Suspect #1 (F3 event
flood).** Test that hypothesis first.

---

## Q7 — Recommended runtime probes (without code changes)

The user did not commit `trc` instrumentation. Before adding more, ask
the user to:

1. **Sample `trc("F3:evt")`** count between `R:after-f3` and `R:pre-rsx` (or
   the equivalent point in HEAD without instrumentation). If F3 sees
   >100 events in 1 second after first render, **Suspect #1 confirmed**.

2. **Disable F3 at startup** as a binary test: temporarily comment out the
   `use_future(move || { let mut rx = api::event_channel(); … })` block.
   If the hang disappears, **Suspect #1 confirmed** at the receive loop
   level (not the inner match block).

3. **Disable `render_markdown`** temporarily (revert to plain text) and
   see if the CPU burn reduces. If yes, **markdown re-parsing is the cost
   amplifier, but the trigger is still the event flood** (without events,
   no re-render, no re-parse).

4. **Check `Self::E:mount-end`** trace: if `use_effect` never prints after
   first render, **the tao event loop is fully blocked** before any
   geometry move fires. That would imply Suspect #6 (Dioxus scheduler
   spin) rather than Suspect #1 (F3-driven).

5. **Capture a `target\debug\northhing.exe` process dump** (procdump or
   WinDbg `.dump`) during the hang to see which thread is at 100% CPU
   and what it's spinning on. This is the single highest-value diagnostic
   the user can run — it tells us whether the spin is in `room_app_root`'s
   render path, a Dioxus scheduler, a std::thread spawned by an inner/
   outer window, or the WebView2 wry process.

---

## Q8 — Likely-blind-spot items not yet investigated

- **Inner/outer window `use_hook` `std::thread::Builder::spawn(loop { sleep(16ms) })`** (`windows/self_app.rs:56-99`, `windows/work.rs:56-99`):
  these spawn a thread that wakes every 16 ms. **They do NOT spawn at
  startup** because the inner/outer windows are only mounted on gem click
  (`app.rs:419,431,443,575,590`). But if a previous run left a stuck
  module window alive in `ShellWindowManager` state (e.g., registry
  corruption between sessions), a re-mount on next launch could spawn
  the thread unexpectedly. Worth checking `~/.northhing/data/workspace_data.json`
  for stale `module_windows` entries. (None observed today, but worth a
  spot check on next hang.)
- **`window().drag()` on `mousedown`** at `app.rs:399, 454` — if the user's
  mouse pointer was already inside the room window at startup, a
  `mousedown` on a drag-handler element during the first 100ms could
  enter a drag loop. But this requires user input.
- **`render::DioxusDocument::create_scope`** doing wry-side work — wry
  builds the WebView which may take time on first launch (this is the
  ONE actual heavy I/O in the first render). On Windows, WebView2 init
  can take 1-3 seconds on first run; subsequent runs hit a cache. The
  user "reset" WebView2 data, so this is a fresh init. Worth profiling.
- **MCP manager `initialize_all` still runs in background** (`lifecycle.rs:140-144`).
  Even with smoke-echo removed, the MCP service still attempts to start
  any servers in `MCPService`'s config. If `app.json` has 0 MCP servers
  but a stale config file in `data/mcp-servers.json` has entries, those
  would attempt to spawn and could flood events. Worth checking
  `data/mcp-servers.json` if it exists.

---

## Status

**DONE (read-only static trace).** Top suspect is **Suspect #1 (F3 event
flood driving render storm)**. Suspect #6 (Dioxus scheduler spin) is the
runner-up if Q7 #1 (F3 event count) shows low volume. The change since
02:12 that most plausibly explains a data-dependent trigger is **the
removal of `smoke-echo` MCP server** — but the cleanup left the MCP event
bus in a state where some other server (or the kernel's own internal
events) is flooding F3.

Recommend the user run **Q7 #5 (process dump)** as the single highest-
value diagnostic. With the stack of the spinning thread in hand, the
suspect can be narrowed to one of the eight in minutes.