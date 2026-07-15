# Upstream: Tokio mpsc + Actor Pattern

> **Source:** WebFetch from `https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html` (2026-06-19).
> **Purpose:** Reference for implementing the actor / dispatcher runtime in
> the planned `crates/agent-dispatch/` crate. NOT a verbatim copy of any
> specific file; reconstructed from the module-level docs.

## Why this is here

The northhing lightweight actor design (`.agents/reference/actor/`) is
built on `tokio` (the project already depends on it). Before writing the
runtime, it's worth a sanity check against tokio's documented patterns
for:

- Long-running tasks that pull messages from an `mpsc::Receiver`.
- Bounded vs unbounded channels.
- Closing semantics for graceful shutdown.

The tokio docs are the canonical source for these patterns.

## API surface (from `tokio::sync::mpsc`)

| Item | Kind | Purpose |
|---|---|---|
| `mpsc::channel(buffer)` | function | Bounded (backpressure). |
| `mpsc::unbounded_channel` | function | Unbounded (no backpressure). |
| `Sender<T>` | struct | Async send handle. Cloneable. |
| `Receiver<T>` | struct | Single-consumer async receive. Not cloneable. |
| `Sender::send` | async fn | Awaits capacity then sends. |
| `Sender::try_send` | sync fn | Non-blocking attempt. |
| `Sender::blocking_send` | sync fn | From sync code. |
| `Receiver::recv` | async fn | `Some(msg)` or `None` when closed. |
| `Receiver::close` | sync fn | Prevent further sends; drain. |

## Design notes (verbatim-ish from the module docs)

- **Bounded vs unbounded.** `channel(n)` is preferred — backpressure
  prevents unbounded memory growth. `unbounded_channel` should only be
  used for true fire-and-forget flows (e.g. telemetry).
- **Multiple senders.** `Sender` is `Clone`. `Receiver` is NOT. This is
  the SPSC/MPSC distinction baked into the type.
- **Closing.** "When all `Sender` handles have been dropped, it is no
  longer possible to send values. … Once all senders have been dropped
  and any remaining buffered values have been received, `Receiver::recv`
  returns `None`."
- **Graceful shutdown.** "The receiver first calls `close`, which will
  prevent any further messages. Then the receiver consumes the channel
  to completion, at which point the receiver can be dropped."

## The canonical actor template

```rust
use tokio::sync::mpsc;

struct MyActor {
    receiver: mpsc::Receiver<Msg>,
    // ... actor state
}

impl MyActor {
    fn new(receiver: mpsc::Receiver<Msg>) -> Self {
        Self { receiver /*, ... */ }
    }

    async fn run(mut self) {
        // Drain until all senders are dropped (recv() == None).
        while let Some(msg) = self.receiver.recv().await {
            self.handle(msg).await;
        }
    }

    async fn handle(&mut self, _msg: Msg) {
        // ... per-message logic
    }
}

#[tokio::main]
async fn main() {
    // Bounded channel gives backpressure.
    let (tx, rx) = mpsc::channel::<Msg>(32);

    // Spawn the actor — it owns the Receiver exclusively.
    let actor = MyActor::new(rx);
    tokio::spawn(actor.run());

    // Many producers can clone `tx`.
    let tx2 = tx.clone();
    tokio::spawn(async move { tx2.send(Msg::Ping).await.ok(); });

    // Drop the original sender so the actor can exit when producers are done.
    drop(tx);
}
```

## How this maps to the northhing actor design

The planned `ActorRuntime` (`.agents/reference/actor/03-actor-runtime.rs`)
already follows this template, with two extensions:

1. **Multi-actor registry.** The tokio template is for one actor. The
   northhing runtime needs a `DashMap<String, ActorHandle>` registry
   so many actors can coexist. Use the same `while let Some(msg) = ...`
   loop inside each spawned task.
2. **Cancellation token + per-actor timeout.** The tokio template
   relies on the natural "all senders dropped" exit. The northhing
   design adds a `CancellationToken` so external callers can stop an
   actor before all senders drop. The pattern:
   ```rust
   tokio::select! {
       _ = cancel.cancelled() => { /* exit gracefully */ }
       Some(msg) = receiver.recv() => { self.handle(msg).await; }
   }
   ```

## What this doc is NOT

- Not a tutorial on async Rust or tokio. For that, see
  `https://tokio.rs/tokio/tutorial`.
- Not a replacement for the northhing actor spec at
  `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md`. The
  spec is the source of truth for the design; this doc is just
  reference material for one specific pattern within that design.
