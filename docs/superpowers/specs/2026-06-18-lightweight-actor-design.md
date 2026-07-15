# Lightweight Actor & One-shot Dispatch Design

> **Status:** Approved (brainstorming session, 2026-06-18)
> **For implementers:** This is a design spec. After it is approved by the user, invoke `superpowers:writing-plans` to convert this into an implementation plan.

## Goal

Reduce the cost of two specific execution patterns that today require spinning up the full `ExecutionEngine → RoundExecutor → StreamProcessor → ToolPipeline` stack:

1. **Skill actors** — skills that need to run asynchronously in the background (timers, watchers, pollers) **without any LLM call**.
2. **One-shot subagents** — `Task` tool invocations whose real work is "1 LLM call → pick 1 tool → execute it → return" with no further looping.

Today both patterns go through `ConversationCoordinator::execute_hidden_subagent_internal` (`src/crates/assembly/core/src/agentic/coordination/coordinator.rs:4172-5025`), which constructs a hidden session, registers a cancel token, and spawns a full `ExecutionEngine::execute_dialog_turn`. The per-subagent state is already small (one Session + one CancellationToken + one timeout handle + initial messages), but the **control-flow path** is heavy: it instantiates an `ExecutionContext`, an `ExecutionContextVars`, a per-round `RoundContext`, runs a `StreamProcessor`, and goes through the multi-round loop machinery even when the work is single-shot.

## Non-goals

- Refactoring `ExecutionEngine` or the multi-turn loop.
- Touching `ConversationCoordinator` for subagent types that already work (`Explore`, `Plan`, `FileFinder`, `DeepResearch`, `DeepReview`, `general-purpose`).
- Implementing IPC adapters now (only the trait + a stub + a const flag).
- Distributed actors or multi-machine execution.
- Actor-to-actor channels or persistent actor state across restarts.

## Architecture overview

Two parallel, **independent** execution surfaces replace the heavy path for the two target patterns. They share a trivial common utility (`CancelHandle`, `TimeoutRegistry`, `TelemetrySink`) but do **not** share a trait or a crate.

```
                  ┌──────────────────────────────────┐
                  │  existing Task tool             │
                  │  (full ExecutionEngine loop)    │
                  │  for multi-turn subagents       │
                  └──────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   ┌──────────────────────┐       ┌──────────────────────────┐
   │  SkillRuntime        │       │  ToolDispatcher          │
   │  + async mode        │       │  (new trait)             │
   │                      │       │                          │
   │  actor = skill       │       │  one-shot subagent:      │
   │  running async       │       │  1 LLM call + 1 tool     │
   │  (no LLM)            │       │  single round            │
   └──────────────────────┘       └──────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              ▼
                  ┌───────────────────────┐
                  │  shared utility:      │
                  │  CancelHandle         │
                  │  TimeoutRegistry      │
                  │  TelemetrySink        │
                  └───────────────────────┘
                              │
                              ▼  (const flag USE_ACTOR_IPC, default false)
                  ┌───────────────────────┐
                  │  IpcAdapter (stub)    │
                  └───────────────────────┘
```

**Why two parallel surfaces (option A over the alternatives)?**

Option B ("shared kernel with two frontends") and option C ("one trait, two impls") were considered and rejected at the brainstorming step. The user picked option A because:

- The two paths have very different semantics: actors are persistent and pull-based; one-shot dispatch is single-call and push-based.
- Forcing them behind one trait would require a "mode" enum, which adds friction at every call site.
- The two paths reuse **third-party types** that already exist in the codebase (`tokio_util::sync::CancellationToken`, `std::time::Duration`, the existing `Arc<dyn TelemetrySink>` trait). They do **not** share a new crate or a new trait defined by this spec. If duplication of *behavior* (timeout enforcement, cancel propagation, telemetry hooks) grows past ~100 lines of repeated code, refactor later behind a `DispatchKernel` (option B) without changing call sites.

## Trait definitions

### SkillActor (lives in `crates/services/services-core/src/skill_runtime/async_mode.rs`)

```rust
#[async_trait]
pub trait SkillActor: Send + Sync {
    fn id(&self) -> &str;
    fn skill_name(&self) -> &str;
    /// Called by ActorRuntime on a schedule or event.
    /// Returning `Ok(None)` is a silent tick.
    async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>>;
}

pub struct ActorContext {
    pub tool_dispatcher: Arc<dyn ToolDispatcher>,
    pub cancel: CancellationToken,
    pub telemetry: Arc<dyn TelemetrySink>,
}

pub enum ActorOutput {
    Silent,
    Event(ActorEvent),       // flows to main session via existing event bus
    Error(ActorError),
}
```

Invariants:

1. **`SkillActor::tick` must not call any LLM directly.** If a skill needs LLM to function, it is not an actor — either promote to a full subagent (existing Task tool path) or have the actor call `ctx.tool_dispatcher.dispatch_once(...)` for a single LLM call. Multi-round LLM loops are not allowed in an actor.
2. **`SkillActor::tick` must be cancel-aware.** It must observe `ctx.cancel` on every blocking call (sleep, IO, sub-dispatch). Failing to do so is a bug; the runtime will warn-log on join if an actor survived its cancel token.
3. **Actor state is in-memory.** Restart loses state. Skill registration is persistent; actor instance is not.

### ToolDispatcher (lives in `crates/agent-dispatch/src/dispatcher.rs`)

```rust
#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    /// Resolve a prompt via one LLM call, then invoke the chosen tool exactly once.
    /// Returns the tool's output. Does NOT loop, does NOT retry, does NOT pick a
    /// second tool even if the first tool's output would benefit from one.
    async fn dispatch_once(&self, req: DispatchRequest) -> Result<DispatchOutput>;

    fn available_tools(&self) -> &[ToolDescriptor];
}

pub struct DispatchRequest {
    pub prompt: String,
    pub model: ModelSpec,
    pub cancel: CancellationToken,
    pub timeout: Duration,
    pub parent_session_id: SessionId,
}

pub enum DispatchOutput {
    ToolResult { tool_id: String, output: Value },
    NoToolMatched { reason: String },
    Cancelled,
    Timeout,
}
```

Invariants:

1. **One LLM call. No exceptions.** If the LLM produces a plan that needs multiple steps, return `NoToolMatched` and let the caller fall back to the full subagent path.
2. **Cancel and timeout are honored at every await point.** The dispatcher must observe `req.cancel` before the LLM call, between the LLM call and the tool call, and during the tool call.
3. **The chosen tool runs through the shared `ToolPipeline`.** No tool execution bypass — this preserves the 8-state machine, the confirmation policy, and the per-tool cancel registry.

### ActorRuntime (lives in `crates/agent-dispatch/src/runtime.rs`)

```rust
pub struct ActorRuntime {
    actors: Arc<DashMap<String, ActorHandle>>,
    cancel: CancellationToken,
    use_ipc: bool,  // driven by const flag USE_ACTOR_IPC
}

impl ActorRuntime {
    pub fn new(parent_cancel: CancellationToken) -> Self;
    pub async fn spawn_actor(&self, actor: Box<dyn SkillActor>) -> ActorHandle;
    pub fn cancel_actor(&self, id: &str);
    pub async fn join_actor(&self, id: &str) -> Result<ActorOutput>;
    pub fn list_actors(&self) -> Vec<ActorSummary>;
}
```

Invariants:

1. **`parent_cancel` is the root.** Every actor gets `parent_cancel.child_token()`; cancelling the parent cancels every actor.
2. **`spawn_actor` returns immediately.** The actual `tick` loop runs on a `tokio::spawn`-ed task. The `ActorHandle` is just an id and a `JoinHandle`/`AbortHandle` pair.
3. **`use_ipc = false` (default)** → use `TokioAdapter` (in-process). `use_ipc = true` → dispatch to `IpcAdapter` (stub for now; will route through Phase A3's `northhing internal actor` later).

## Const flags

Two flags in `crates/agent-dispatch/src/lib.rs`:

```rust
pub const USE_ACTOR_IPC: bool = false;
pub const USE_DISPATCHER_IPC: bool = false;
```

When `false` (the default), the runtime uses the in-process adapter. When `true`, it constructs the IPC adapter and routes through it. Both adapters implement the same internal `SpawnAdapter` trait so call sites do not see the difference.

## Data flow

### Actor scenario (e.g. "watchdog skill polls git status every 5 min")

1. Skill registration at app startup: `skill_registry.register_async("git-watchdog", Box::new(WatchdogActor::new()))`
2. App startup creates `ActorRuntime::new(parent_cancel.child_token())` and calls `runtime.spawn_actor(handle)` for each async skill.
3. `WatchdogActor::tick`:
   - `tokio::select! { _ = tokio::time::sleep(5min) => {}, _ = ctx.cancel.cancelled() => return Ok(Some(Cancelled)) }`
   - `let out = ctx.tool_dispatcher.dispatch_once(DispatchRequest { prompt: "git status", model: ..., cancel: ctx.cancel.clone(), timeout: 30s, parent_session_id: ... }).await?`
   - If `out` indicates dirty repo, return `Ok(Some(ActorOutput::Event(ActorEvent::Notification("git is dirty"))))`.
4. `ActorOutput::Event` flows to the main session via the existing `Arc<EventRouter>` (no new channel).

### One-shot subagent scenario

1. `Task` tool receives `subagent_type` that matches a registered one-shot type (e.g. `quick-lookup`).
2. `task_tool.rs` routes to `ToolDispatcher::dispatch_once` instead of `coordinator.execute_subagent`.
3. `ToolDispatcher::dispatch_once`:
   - One LLM call → model picks tool_id + arguments.
   - Single tool execution via the shared `ToolPipeline` (same `cancel_dialog_turn_tools`, same 8-state machine, same confirmation policy).
   - Return `DispatchOutput::ToolResult`.
4. The `Task` tool returns the tool output to the parent session as a normal assistant message.

## Tests

Required unit tests (each one in `crates/agent-dispatch/tests/` or `crates/services/services-core/src/skill_runtime/tests/`):

```rust
#[tokio::test]
async fn watchdog_actor_emits_event_on_dirty_repo();

#[tokio::test]
async fn watchdog_actor_silent_on_clean_repo();

#[tokio::test]
async fn actor_respects_parent_cancel();

#[tokio::test]
async fn actor_respects_tick_timeout();

#[tokio::test]
async fn one_shot_dispatch_routes_to_correct_tool();

#[tokio::test]
async fn one_shot_dispatch_returns_no_tool_on_multi_step_prompt();

#[tokio::test]
async fn one_shot_dispatch_honors_cancel_before_llm_call();

#[tokio::test]
async fn one_shot_dispatch_honors_timeout_during_llm_call();

#[tokio::test]
async fn tool_pipeline_receives_normal_confirmation_request_for_one_shot();

#[tokio::test]
async fn actor_runtime_spawns_in_tokio_by_default();

#[tokio::test]
async fn actor_runtime_routes_through_ipc_when_flag_enabled();
```

Each test uses `mockall` or hand-written fakes for `ToolDispatcher`, `SkillActor`, `TelemetrySink`. No real network, no real LLM.

## Rollback plan

Each piece is gated by a const flag:

- `USE_LIGHTWEIGHT_ACTOR: bool = false;` — when `false`, `SkillRuntime` ignores `register_async` calls (no actor spawned). Equivalent to no change.
- `USE_ONESHOT_DISPATCHER: bool = false;` — when `false`, `Task` tool routes every subagent type through the existing `coordinator.execute_subagent` path.

If a regression appears, flip the relevant flag to `false`, commit, no rebuild required.

## Files added or modified

| Path | Change |
|---|---|
| `crates/agent-dispatch/Cargo.toml` | NEW crate (depends on `tokio`, `async-trait`, `dashmap`, `tracing`) |
| `crates/agent-dispatch/src/lib.rs` | NEW: const flags + crate-level docs |
| `crates/agent-dispatch/src/dispatcher.rs` | NEW: `ToolDispatcher` trait + types |
| `crates/agent-dispatch/src/runtime.rs` | NEW: `ActorRuntime` |
| `crates/agent-dispatch/src/spawn/tokio_adapter.rs` | NEW: in-process adapter (default) |
| `crates/agent-dispatch/src/spawn/ipc_adapter.rs` | NEW: stub IPC adapter (`unimplemented!()`-gated by flag) |
| `crates/agent-dispatch/src/telemetry.rs` | NEW: `TelemetrySink` trait |
| `crates/contracts/runtime-ports/src/lightweight_task.rs` | NEW: re-export port traits |
| `crates/services/services-core/src/skill_runtime/async_mode.rs` | NEW: `SkillActor` trait + `register_async` |
| `crates/services/services-core/src/skill_runtime/runtime.rs` | MODIFY: `register_async` method (gated by `USE_LIGHTWEIGHT_ACTOR`) |
| `crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | MODIFY: route one-shot subagent types to `ToolDispatcher` (gated by `USE_ONESHOT_DISPATCHER`) |

Files explicitly **not** touched:

- `crates/assembly/core/src/agentic/coordination/coordinator.rs` (existing subagent path preserved)
- `crates/assembly/core/src/agentic/execution/execution_engine.rs` (multi-turn loop preserved)
- `crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline.rs` (shared, unchanged)
- `crates/execution/tool-execution/src/pipeline.rs` (8-state machine, unchanged)

## Out of scope

- IPC adapter implementation (only the trait + stub).
- Actor-to-actor channels.
- Actor persistence across restarts.
- Refactoring the multi-turn `ExecutionEngine`.
- Changing existing subagent types (`Explore`, `Plan`, `general-purpose`, etc.).

## Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-06-18 | Two parallel surfaces (option A) | User picked A over B (shared kernel) and C (single trait) |
| 2026-06-18 | `SkillActor::tick` must not call LLM | Keeps the actor model pull-based and predictable |
| 2026-06-18 | `ToolDispatcher` is single-shot only | Multi-step work stays on the existing path; one-shot is for lookups |
| 2026-06-18 | IPC adapter is stub + const flag | Future-proof without committing to a protocol now |
| 2026-06-18 | Do not touch `coordinator.rs` or `execution_engine.rs` | Bounds the change; existing subagents keep working |

## Related

- Parent plan: `docs/superpowers/plans/2026-06-18-northhing-rebuild.md` (Phase A6 multi-session)
- v3 subagent code map (produced during brainstorming): see exploration notes inline above and the cited files in this spec
- Skill system: `crates/services/services-core/src/skill_runtime/`
- Existing subagent path: `crates/assembly/core/src/agentic/coordination/coordinator.rs:4172-5025`
