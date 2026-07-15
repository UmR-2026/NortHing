# Actor Domain — Signatures

> One-page signature card. **DESIGN ONLY** — no Rust implementation as of
> 2026-06-19. The signatures below are from the spec; expect them to
> drift slightly when the implementation lands.

## `SkillActor` trait

Source: `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` lines 70-99

```rust
#[async_trait]
pub trait SkillActor: Send + Sync {
    fn id(&self) -> &str;
    fn skill_name(&self) -> &str;
    async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>, ActorError>;
}

pub struct ActorContext {
    pub tool_dispatcher: Arc<dyn ToolDispatcher>,
    pub cancel: CancellationToken,
    pub telemetry: Arc<dyn TelemetrySink>,
}

pub enum ActorOutput {
    Silent,
    Event(ActorEvent),
    Error(ActorError),
}
```

**Invariants (from spec):**
1. `tick` must NOT call LLM directly. If a skill needs LLM, it's not an actor.
2. `tick` must observe `ctx.cancel` on every blocking call.
3. Actor state is in-memory; restart loses state. Registration is persistent.
4. Per-tick timeout enforced by runtime (default 30s).

## `ToolDispatcher` trait

Source: spec lines 101-135

```rust
#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    async fn dispatch_once(&self, req: DispatchRequest) -> DispatchOutput;
}

pub struct DispatchRequest {
    pub dispatch_id: String,
    pub user_prompt: String,
    pub prepended_context: Vec<String>,
    pub tool_allowlist: Vec<String>,
    pub timeout: Duration,
    pub cancel: CancellationToken,
    pub telemetry: Arc<dyn TelemetrySink>,
}

pub enum DispatchOutput {
    ToolResult(ToolResult),
    NoToolMatched { reason: String },
    Cancelled,
    Timeout,
}
```

**Note:** This is for **one-shot** work only. Multi-round loops go through
`ConversationCoordinator::execute_hidden_subagent_internal` (coordinator.rs:4173).

## `ActorRuntime`

Source: spec lines 137-169

```rust
pub struct ActorRuntime {
    actors: DashMap<String, ActorHandle>,
    dispatcher: Arc<dyn ToolDispatcher>,
    telemetry: Arc<dyn TelemetrySink>,
    default_tick_timeout: Duration,
}

impl ActorRuntime {
    pub fn new(dispatcher: Arc<dyn ToolDispatcher>, telemetry: Arc<dyn TelemetrySink>) -> Self;
    pub fn spawn_actor(&self, actor: Box<dyn SkillActor>, schedule: ActorSchedule) -> ActorHandle;
    pub fn stop_actor(&self, id: &str);
    pub fn stop_all(&self);
}

pub struct ActorHandle {
    pub id: String,
    pub fn stop(&self);       // signals cancel
    pub async fn await_join(self) -> Result<(), JoinError>;
}

pub enum ActorSchedule {
    Periodic(Duration),
    OnSignal(mpsc::Receiver<ActorTrigger>),
    Cron(String),
    OneShot,
}
```

## Const flags (planned, not yet declared)

| Flag | Default | When to flip to `true` |
|---|---|---|
| `USE_LIGHTWEIGHT_ACTOR` | `false` | After Phase 2 of impl plan passes integration test. |
| `USE_ONESHOT_DISPATCHER` | `false` | After Phase 1 of impl plan passes integration test. |
| `USE_ACTOR_IPC` | `false` | After Phase 3 of impl plan lands. |
| `USE_DISPATCHER_IPC` | `false` | After Phase 3 of impl plan lands. |

See [`06-const-flag-usage.md`](./06-const-flag-usage.md) for the standard
flip process.

## Existing patterns to borrow

From [`04-coordinator-spawn-pattern.rs`](./04-coordinator-spawn-pattern.rs):

| Pattern | Source | Use case |
|---|---|---|
| mpsc back-channel (with `OnceLock` wiring) | coordinator.rs:518-528 | Scheduler → coordinator notifications. **See NOTES — don't copy for actor registry.** |
| Spawned turn with cancel + timeout | coordinator.rs:301, 360, 431-502 | Per-actor spawn. |
| DashMap-keyed cancel registry | coordinator.rs:511 | Per-actor cancellation by id. |
| Semaphore-based concurrency limiter | coordinator.rs:512 | Per-profile actor limits. |
| `tokio::spawn` on current runtime | coordinator.rs:5316 | Avoid separate runtime for actors. |

From [`05-scheduler-dashmap-pattern.rs`](./05-scheduler-dashmap-pattern.rs):

| Pattern | Source | Use case |
|---|---|---|
| `ActiveDialogTurnStore` | scheduler.rs:101-135 | Per-session turn registry. |
| `DialogReplySuppressionSet` | scheduler.rs:137-158 | Per-(session, turn) composite-key flags. |
| `SessionAbortFlags` | scheduler.rs:160-177 | Per-session single-key flags. |
| `DialogTurnQueue` | scheduler.rs:209-220+ | Per-session priority queue. |
