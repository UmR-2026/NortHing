# Const-Flag Usage Pattern

> Source: this is the project's **standard pattern** for any behavior change.
> Adapted from `A4 Skill system v2` (`USE_SKILL_REGISTRY` at
> `skill_agent_snapshot.rs:27`) and called out in
> `HANDOFF.md` / `PROJECT_STATE.md`.

## Why a const flag?

The project requires every behavior change to ship behind a
`const FLAG: bool = true;` gate. This lets you:

1. Roll back with a one-line `const FLAG: bool = false;` + commit.
2. Ship a "dark launch" — flag on in code, but only the call site
   checks the flag, so the path is exercised in tests without
   affecting production.
3. A/B test two implementations by toggling the flag in a feature
   branch.

## The pattern

```rust
// In the file that defines the new behavior:
pub const USE_NEW_BEHAVIOR: bool = true;

// In the call site:
if USE_NEW_BEHAVIOR {
    new_behavior(...).await
} else {
    legacy_behavior(...).await
}
```

That's it. The flag is a `const`, not a `static` or a config value —
this is a deliberate choice. It means:

- No runtime configuration overhead.
- The flag value is baked at compile time; the dead branch is
  eliminated by the compiler.
- The flag is grep-able across the codebase: `git grep "USE_NEW_BEHAVIOR"`.
- The flag can only be changed by editing the source and rebuilding.

## Rules of the road

1. **Default the flag to `true`** for new behaviors that have been
   validated. Default to `false` for behaviors that are still being
   rolled out (e.g. `USE_LIGHTWEIGHT_ACTOR`, `USE_ONESHOT_DISPATCHER`).

2. **Name flags in SCREAMING_SNAKE_CASE** with a `USE_` prefix.

3. **Co-locate the flag with the new behavior**, not in a central
   `flags.rs`. This makes the flag easy to find and roll back.

4. **Pair every flag flip with a regression test** that exercises
   both the new and legacy paths (if the legacy path still exists).

5. **Pair every flag flip with a `PROJECT_STATE.md` update** that
   records the new state and links to the next flip planned.

6. **Do not introduce `static FLAG: AtomicBool`** — the project
   deliberately uses `const`. If you need a runtime toggle, propose
   it as a separate design change.

## Examples in the codebase

| Flag | File | Default | Rollback cost |
|---|---|---|---|
| `USE_SKILL_REGISTRY` | `skill_agent_snapshot.rs:27` | `true` | One line. |
| `USE_SLINT_SHELL` | (per A1 implementation) | `true` | One line. |
| `USE_SOFTWARE_FALLBACK` | (per A1 implementation) | `true` | One line. |
| `USE_LIGHTWEIGHT_ACTOR` | (planned) | `false` | One line. |
| `USE_ONESHOT_DISPATCHER` | (planned) | `false` | One line. |
| `USE_ACTOR_IPC` | (planned) | `false` | One line. |
| `USE_DISPATCHER_IPC` | (planned) | `false` | One line. |

## What this means for the actor design

When implementing the actor / dispatcher surface, **every** new
behavior should ship behind a const flag. The 4 planned flags are:

- `USE_LIGHTWEIGHT_ACTOR` — enable the `SkillActor` runtime.
- `USE_ONESHOT_DISPATCHER` — enable the `ToolDispatcher` for one-shot subagents.
- `USE_ACTOR_IPC` — allow actors to spawn in a separate process.
- `USE_DISPATCHER_IPC` — allow dispatches to run in a separate process.

All default to `false`. Flip to `true` only after the corresponding
phase has been validated by an integration test.
