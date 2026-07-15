# Actor Domain — Reference

> Lightweight Actor / One-shot Dispatcher code mirrors.
> Read [`SIGNATURES.md`](./SIGNATURES.md) first, then
> [`NOTES.md`](./NOTES.md) for "do NOT copy" warnings.

## Status

**Designed only — NOT IMPLEMENTED.** The spec and plan are approved
(2026-06-18) but no Rust crate has been scaffolded. The
mirror files in this directory are extracted from the design
documents plus patterns borrowed from the existing coordinator and
scheduler.

When the implementation lands, replace these design-doc extracts
with full source mirrors and bump the `Last synced` field in each
file header.

## File ordering

| # | File | Source | Status |
|---|---|---|---|
| 01 | [`01-skill-actor-trait.rs`](./01-skill-actor-trait.rs) | spec lines 70-99 | Design |
| 02 | [`02-tool-dispatcher-trait.rs`](./02-tool-dispatcher-trait.rs) | spec lines 101-135 | Design |
| 03 | [`03-actor-runtime.rs`](./03-actor-runtime.rs) | spec lines 137-169 | Design |
| 04 | [`04-coordinator-spawn-pattern.rs`](./04-coordinator-spawn-pattern.rs) | `coordinator.rs:1-100, 301-5316` | Existing |
| 05 | [`05-scheduler-dashmap-pattern.rs`](./05-scheduler-dashmap-pattern.rs) | `scheduler.rs:100-220` | Existing |
| 06 | [`06-const-flag-usage.md`](./06-const-flag-usage.md) | project convention | Pattern |

## Selection guide

| You need to… | Start with |
|---|---|
| Implement a new `SkillActor` | 01 (trait) + 04 (spawn pattern) + 06 (flag) |
| Add a new one-shot dispatch | 02 (trait) + 04 + 06 |
| Wire the actor runtime into startup | 03 (runtime shape) + 04 (OnceLock lessons — see NOTES) |
| Add a per-actor state registry | 05 (DashMap pattern) |
| Decide on a flag flip | 06 (the project's standard process) |

## Why two parallel surfaces (per the spec)

The spec chose two traits (SkillActor + ToolDispatcher) over a single
trait with a "mode" enum because:

1. **Semantics differ.** Actors are persistent and pull-based; one-shot
   dispatches are single-call and push-based.
2. **Forcing them behind one trait** would require a "mode" enum at
   every call site — adds friction.
3. **Both already use the same third-party types** (`CancellationToken`,
   `Duration`, `Arc<dyn TelemetrySink>`). No new shared crate needed.

If duplication of *behavior* (timeout enforcement, cancel propagation,
telemetry hooks) grows past ~100 lines, refactor later behind a
`DispatchKernel` (per the spec) without changing call sites.

## Public API surface (planned, not yet implemented)

```rust
// In the (planned) crates/agent-dispatch crate:

use agent_dispatch::{ActorRuntime, ToolDispatcher, SkillActor, ActorContext};

// At startup:
let runtime = Arc::new(ActorRuntime::new(dispatcher.clone(), telemetry.clone()));

// To register a periodic actor:
runtime.spawn_actor(
    Box::new(MyPollingActor::new()),
    ActorSchedule::Periodic(Duration::from_secs(60)),
);

// For a one-shot subagent (via the dispatcher, not the actor):
let result = dispatcher.dispatch_once(DispatchRequest {
    dispatch_id: "...".to_string(),
    user_prompt: "Find all .rs files in src/".to_string(),
    prepended_context: vec![],
    tool_allowlist: vec!["file_search".to_string()],
    timeout: Duration::from_secs(30),
    cancel: CancellationToken::new(),
    telemetry: telemetry.clone(),
}).await;
```
