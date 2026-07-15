# Actor Domain — "Do NOT Copy Verbatim" Notes

> **Everything in this domain is a design, not an implementation.**
> Most of the "do not copy" warnings below apply to design choices that
> look reasonable in the spec but will cause problems if you extend them
> carelessly.

## ⛔ Do NOT call LLM directly from `SkillActor::tick`

`SkillActor` invariant #1: `tick` MUST NOT call any LLM. If a skill
needs LLM, it is not an actor. Either:

- Promote to a full subagent (existing Task tool path), OR
- Have the actor call `ctx.tool_dispatcher.dispatch_once(...)` for a
  single LLM call.

Multi-round LLM loops are not allowed in an actor. The runtime will
not enforce this; the warning-log on join is the only signal.

## ⛔ Do NOT extend `ToolDispatcher` to support multi-round

`ToolDispatcher` is explicitly for **one-shot** work. If you need a
multi-round LLM → tool → LLM loop, use the existing
`ConversationCoordinator::execute_hidden_subagent_internal` at
`coordinator.rs:4173`. Adding a `multi_round: bool` parameter to
`DispatchRequest` would conflate the two paths and break the design.

## ⛔ Do NOT copy `OnceLock<mpsc::Sender<...>>` for the actor's hot path

`coordinator.rs:518-520` uses `OnceLock` for the scheduler wire
because the scheduler may come up after the coordinator. The actor
design should wire the dispatcher at construction, not lazily. Use
`OnceLock` only for true late-binding cases (rare).

## ⛔ Do NOT use a separate tokio runtime for actors

`app_state.rs` constructs a per-UI-event runtime, but actors are not
UI events. The actor runtime should run on the same `tokio::runtime::Handle`
as the coordinator, so `CancellationToken` works across the boundary.
If you spawn actors on a separate runtime, `cancel.cancel()` from the
coordinator will not propagate.

## ⛔ Do NOT touch `coordinator.rs:4172-5025` to "extend" the existing path

`execute_hidden_subagent_internal` is the heavy multi-turn subagent
path that the actor design explicitly **replaces**. The spec at
`docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` says:

> "Do NOT enable `USE_LIGHTWEIGHT_ACTOR = true` without integration
> testing first."

Same for the other 3 flags. The intended workflow is:
1. Implement the new path.
2. Wire the call site to check the flag.
3. Run integration tests with flag = `false` (existing path) and
   flag = `true` (new path).
4. Only then flip the default to `true`.

## ⚠️ IPC adapter is a STUB

`IpcSpawnAdapter` in the spec is intentionally a stub in Phase 1
(returns `"ipc-stub"`). Do not write code against the IPC surface
before Phase 3 lands.

## ⚠️ The 4 const flags are independent

| Flag | What it controls |
|---|---|
| `USE_LIGHTWEIGHT_ACTOR` | Enables the `SkillActor` runtime. |
| `USE_ONESHOT_DISPATCHER` | Enables the `ToolDispatcher`. |
| `USE_ACTOR_IPC` | Allows actors to spawn in a separate process. |
| `USE_DISPATCHER_IPC` | Allows dispatches to run in a separate process. |

They can be flipped independently. Do NOT add a single "ACTOR_ENABLED"
flag — each path has its own rollout timeline.

## ⚠️ The spec uses option A (two parallel surfaces)

The brainstorming session considered three options:
- Option A: two parallel surfaces (chosen)
- Option B: shared kernel with two frontends
- Option C: one trait, two impls

Option A was chosen because the two paths have very different semantics
and option C would require a "mode" enum at every call site. If you
find yourself wanting to add a mode enum, that's a signal the spec
should be revisited, not patched.

## ⚠️ The spec says "if duplication grows past ~100 lines, refactor"

The two paths share `CancelHandle`, `TimeoutRegistry`, `TelemetrySink`
already. If you find yourself duplicating more than ~100 lines of
behavior, the spec calls for a `DispatchKernel` refactor (option B)
**without changing call sites**. Do not duplicate the duplication.

## ⚠️ `actor_terminated_after_cancel` is the only failure signal

If an actor survives its cancel token (didn't observe `ctx.cancel` on a
blocking call), the runtime warn-logs on join. There is no other
signal. Make sure your actor tests verify cancel propagation; the
runtime will not catch it for you.

## ⚠️ `dispatch_once` timeout vs `DispatchRequest.timeout`

The per-dispatch timeout is enforced by the dispatcher implementation,
not by the runtime. The actor runtime's `default_tick_timeout` is
separate. If you set both, the tighter one wins (per-dispatch is
shorter, so per-dispatch wins).

## ⚠️ The `IpcSpawnAdapter` "stub" return is `"ipc-stub"` (string)

Per the plan, the IPC adapter in Phase 1 returns the literal string
`"ipc-stub"`. Code that consumes the adapter's output must check for
this string and treat it as a no-op. Do not assume the return is a
typed value — it isn't.

## ⚠️ Self-referential pattern in spec (line 854) is a known anti-pattern

The spec uses `unsafe { std::mem::transmute_copy(&handle) }` as a
placeholder for the actor's self-referential `&self` → `&mut self`
problem. This is **explicitly called out** as a "caveat version" that
will be replaced in Task 2.4 by an `AbortHandle` registry. Do not
copy the transmute pattern; use the post-Task-2.4 form when it
ships.

## ✅ Things you SHOULD borrow

- The `tokio_util::sync::CancellationToken` + `tokio::sync::watch`
  plumbing (in `04-coordinator-spawn-pattern.rs` Pattern 2).
- The DashMap-keyed cancel-token registry (in
  `04-coordinator-spawn-pattern.rs` Pattern 3 and
  `05-scheduler-dashmap-pattern.rs`).
- The per-profile `Semaphore` limiter (in
  `04-coordinator-spawn-pattern.rs` Pattern 4) — same shape works
  for limiting actors-per-skill.
- The const-flag pattern (in `06-const-flag-usage.md`). All 4 actor
  flags should use this exact pattern.
- The `tracing` instrumentation style throughout coordinator.rs
  (warn-on-join for unobserved cancels, info-on-spawn, error-on-fail).
