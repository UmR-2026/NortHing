# Actor Implementation Plan — Task Map

> **Source:** `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`
> (17 tasks across 4 phases, last updated 2026-06-19).
> **Purpose:** For each task, point to the reference files in
> `.agents/reference/actor/` that the implementer should consult
> before, during, and after the task. This makes the "copy from
> reference" workflow traceable task-by-task.

## How to use this map

When you start a task, do this:

1. Open the task's section in the impl plan.
2. Read the **"Read first"** column in the table below.
3. **Copy** the relevant pattern (with the
   `// Pattern source: .agents/reference/actor/NN-xxx.rs` header).
4. When the task is done, **update** any reference file the task
   materially changed. The mirror must stay in sync.

## Map

### Phase 1 — Skeleton + const flags (5 tasks)

| Task | Title | Read first | Reference file(s) to copy from | Mirror files to update after task |
|---|---|---|---|---|
| 1.1 | Create `agent-dispatch` crate manifest | `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` (lines 137-169) | `actor/03-actor-runtime.rs` (runtime shape) | none (skeleton task) |
| 1.2 | Add const flags + telemetry trait | `actor/06-const-flag-usage.md` | `actor/03-actor-runtime.rs` (the 4 const flag declarations) | `actor/06-const-flag-usage.md` (add new flags if introduced) |
| 1.3 | Add the contract port | `.agents/reference/actor/02-tool-dispatcher-trait.rs` | `actor/02-tool-dispatcher-trait.rs` (DispatchRequest/Output) | none (port only) |
| 1.4 | Add stub IPC adapter | `.agents/reference/_upstream/tokio-actor-pattern.md` (closing semantics) | `actor/03-actor-runtime.rs` (IpcAdapter comment block) | none (stub) |
| 1.5 | Verify the full workspace still compiles | n/a — just `cargo check` | n/a | none (verification only) |

### Phase 2 — Core types (6 tasks)

| Task | Title | Read first | Reference file(s) to copy from | Mirror files to update after task |
|---|---|---|---|---|
| 2.1 | SkillActor trait + async_mode module | `actor/01-skill-actor-trait.rs` | `actor/01-skill-actor-trait.rs` (the full trait) | **Replace** `actor/01-skill-actor-trait.rs` with the real impl (bump `Last synced`). |
| 2.2 | ToolDispatcher trait + DispatchRequest/Output | `actor/02-tool-dispatcher-trait.rs` | `actor/02-tool-dispatcher-trait.rs` (the full trait) | **Replace** `actor/02-tool-dispatcher-trait.rs`. |
| 2.3 | ActorHandle + ActorRuntime | `actor/03-actor-runtime.rs` + `actor/04-coordinator-spawn-pattern.rs` (Pattern 1, 3) | `actor/04-coordinator-spawn-pattern.rs` (mpsc + DashMap) | **Replace** `actor/03-actor-runtime.rs`. |
| 2.4 | Replace the unsafe transmute with Notify-based cleanup | `actor/NOTES.md` (⚠️ #9 self-referential pattern) | `actor/04-coordinator-spawn-pattern.rs` (Pattern 1: OnceLock → direct wire) | `actor/NOTES.md` — remove the "transmute placeholder" warning. |
| 2.5 | Wire SkillRuntime::register_async behind the flag | `actor/06-const-flag-usage.md` (co-location rule) | `actor/03-actor-runtime.rs` (where the flag is checked) | `actor/06-const-flag-usage.md` (add new flag if introduced). |
| 2.6 | Phase 2 verification | n/a — `cargo test` | n/a | none |

### Phase 3 — Integration (4 tasks)

| Task | Title | Read first | Reference file(s) to copy from | Mirror files to update after task |
|---|---|---|---|---|
| 3.1 | PipelineDispatcher (real ToolDispatcher over shared pipeline) | `.agents/reference/session/01-conversation-coordinator.rs` (the 6 entry points) | `session/01-conversation-coordinator.rs` (entry-point shape) | none (production impl, the trait definitions in `actor/02-*.rs` are still the source of truth) |
| 3.2 | task_tool one-shot routing (flag-gated, default off) | `actor/06-const-flag-usage.md` + `actor/NOTES.md` ⛔ #2 (don't extend ToolDispatcher) | `session/06-app-state-slint-wiring.rs` (callback wiring pattern) | none |
| 3.3 | End-to-end flag-off regression test | `actor/05-scheduler-dashmap-pattern.rs` (test idioms) | `actor/05-scheduler-dashmap-pattern.rs` | none |
| 3.4 | Phase 3 verification | n/a | n/a | none |

### Phase 4 — Documentation + tag (3 tasks)

| Task | Title | Read first | Reference file(s) to copy from | Mirror files to update after task |
|---|---|---|---|---|
| 4.1 | Document the design for the next maintainer | `actor/README.md` (overview) | `actor/README.md` | **Update** `actor/README.md` with any new public API additions. |
| 4.2 | Update HANDOFF.md | `.agents/reference/README.md` (totals table) | n/a (HANDOFF is not in the reference library) | n/a (HANDOFF lives at project root) |
| 4.3 | Final verification + tag | n/a | n/a | none |

## Total: 18 task × reference-pair rows = 18 touchpoints

If a task is **in scope** for you, you have at most 3 things to do:

1. **Read** the reference file(s) in the "Read first" column.
2. **Copy** the pattern from the "Reference file(s) to copy from" column.
3. **Update** the mirror in the "Mirror files to update after task" column,
   bumping the `Last synced` SHA in the file header.

## What the implementer MUST NOT do

These are the cardinal sins, repeated from `actor/NOTES.md` for emphasis:

1. **Do NOT call LLM directly from `SkillActor::tick`.**
   Use `ctx.tool_dispatcher.dispatch_once(...)`.
2. **Do NOT extend `ToolDispatcher` to support multi-round.**
   Multi-round loops go through `ConversationCoordinator::execute_hidden_subagent_internal` (coordinator.rs:4173).
3. **Do NOT use `OnceLock<mpsc::Sender>` for the hot path.**
   Wire at construction. Use `OnceLock` only for true late-binding.
4. **Do NOT use a separate tokio runtime for actors.**
   Use the same `tokio::runtime::Handle` as the coordinator.
5. **Do NOT touch `coordinator.rs:4172-5025` to "extend" the existing path.**
   The new actor design **replaces** it via flag-gated routing.

If any of these would be violated by a task's plan, surface it in the
plan review **before** starting the task.

## Cross-references to other domains

When implementing Phase 3.1 (PipelineDispatcher) or Phase 3.2
(task_tool routing), you'll need to consult **other domains** in the
reference library:

| If the task touches… | Also read |
|---|---|
| The tool pipeline | `.agents/reference/skills/10-skill-tool-full.rs` (a `Tool` trait impl, same shape) |
| The session state machine | `.agents/reference/session/04-session-state.rs` |
| Slint-side callback wiring | `.agents/reference/session/06-app-state-slint-wiring.rs` |
| Const flag flipping | `.agents/reference/actor/06-const-flag-usage.md` |
