# Review Brief — 2026-06-20 Session

> **Purpose**: Single-page entry point for a review agent (human or LLM) to
> understand what shipped this session, what to inspect, and what to verify.
>
> **Branch**: `v3-restructure` @ `5543268`
> **Author of changes**: ZCode session (northhing v3-restructure, 2026-06-20)
> **Reviewer**: TBD
>
> **⚠️ HISTORICAL + CONSOLIDATED (2026-06-20):**
> 1. This brief was written at HEAD `5543268`, before subsequent doc-only
>    commits (`840bd4f`, `e5b83db`, `fa868ae`, `c490151`, `624e12f`, `d3309c6`).
>    Test counts in this brief (8/8 agent-dispatch) reflect the state AT TIME
>    OF WRITING — the actual current count is **20/20** (12 new tests landed
>    in `agent-dispatch` between `5543268` and `d3309c6`).
>    For the up-to-date brief, see the Orchestrator's comprehensive review at
>    `docs/reviews/2026-06-20-northhing-v3-review.md`.
> 2. **Consolidated 2026-06-20**: This brief's full content is reproduced as
>    **Appendix A.1** of `docs/plans/2026-06-19-post-reference-roadmap.md`.
>    Kept as standalone file only for historical traceability. **New model**
>    picking up work should read `HANDOFF.md` first (single entry point) and
>    `docs/plans/2026-06-19-post-reference-roadmap.md` §Appendix A for reviews.

---

## 0. TL;DR for the reviewer

6 commits, 917 insertions / 496 deletions across 15 files. All green.

```text
cargo check -p northhing --lib                 : PASS
cargo test -p northhing-agent-dispatch --lib   : 8/8 PASS
cargo test -p northhing --lib                  : 12/12 PASS
bash scripts/regression-test-desktop.sh        : 8/8 PASS
```

The 6 commits in order:

| # | SHA | One-line |
|---|---|---|
| 1 | `7aa4310` | InMemoryRelationship + parent_dialog_turn_id + parent_turn_index |
| 2 | `0da5130` | ActorRuntime::spawn_one_shot + on_send_message ClosureActor demo |
| 3 | `0830ef0` | HANDOFF bump 42→44 (metadata only) |
| 4 | `c2d2bc8` | **app_state.rs split into 6 submodules** (biggest diff, 557 lines moved) |
| 5 | `5ac37c6` | regression-test-desktop.sh cargo PATH bootstrap |
| 6 | `5543268` | Phase C closeout + Phase K plan + HANDOFF bump 44→46 |

---

## 1. Where to look

### Most impactful change: the `app_state.rs` split (commit `c2d2bc8`)

- 989-line monolith → 6 files in `src/apps/desktop/src/app_state/`
- `create_ui` (callback wiring) stays in `mod.rs` — splitting it would create indirection
- Only pure testable helpers moved: `actor.rs`, `inspector.rs`, `inspector_model_status.rs`, `log.rs`, `sessions.rs`, `skills.rs`
- Visibility chain: each helper is `pub(super)`; `mod.rs` does `use self_submodule::fn`

```bash
git show c2d2bc8 --stat
# 15 files changed, 917 insertions(+), 496 deletions(-)
```

**Specific things to verify**:

1. `cargo check -p northhing` should report 0 errors, 0 new warnings.
2. Each submodule's `pub(super)` items are reachable from `mod.rs` (check `use self::sessions::*` etc.).
3. `create_ui` is byte-equivalent in behavior — same 9 callbacks wired, same property assignments.
4. The 12 desktop tests still pass (they exercise `build_sessions_model` / `build_skills_model` etc. which moved).

### Second biggest: `ActorRuntime::spawn_one_shot` (commit `0da5130`)

- New method on `ActorRuntime` in `src/crates/execution/agent-dispatch/src/runtime.rs`
- Uses `ClosureActor<F: FnMut() + Send + Sync + 'static>` + `#[async_trait]`
- Replaces the previous `spawn_actor` one-shot signature
- Wired into `on_send_message` as a *demo* (constructs a no-op `ClosureActor`, spawns it, awaits telemetry — logs `ActorTicked` event)
- The `USE_LIGHTWEIGHT_ACTOR` flag is still `false` by default, so this is dead-code path

```bash
git show 0da5130 -- src/crates/execution/agent-dispatch/
```

**Specific things to verify**:

1. `ClosureActor` impl is `#[async_trait]` and `Send + Sync` (required for `ActorHandle`).
2. `spawn_one_shot` returns `ActorHandle` (not `JoinHandle`) — consistent with the rest of the API.
3. The 2 new tests in `tests/telemetry_test.rs::closure_actor_tests` exercise both "ticks once" and "captures environment" paths.
4. The demo `on_send_message` body logs `actor_runtime` component events that grep-findable in `.northhing/debug.log`.

### Small but worth checking: InMemoryRelationship extension (commit `7aa4310`)

- Adds `parent_dialog_turn_id: Option<String>` and `parent_turn_index: Option<usize>` to `InMemoryRelationship`
- Disk-load projection in `persistence/manager.rs` populates both fields
- Old serialized JSON without these fields still loads (defaults via `Option::None`)

**Specific things to verify**:

1. The field order in the struct definition matches the order in the disk projection.
2. `#[serde(default, skip_serializing_if = "Option::is_none")]` is present (back-compat).
3. The `persistence/manager.rs` change doesn't break the existing on-disk format (no schema version bump needed).

### Cargo PATH bootstrap (commit `5ac37c6`)

- Adds ~15 lines to `scripts/regression-test-desktop.sh`
- Probes `$HOME/.cargo/bin`, `/c/Users/$USER/.cargo/bin`, `C:/msys64/.../bin`, `/usr/local/cargo/bin`
- Prepends to PATH if cargo isn't already there

**Specific things to verify**:

1. Script no longer requires manual `PATH="/c/msys64/mingw64/bin:$PATH"` prefix on Windows.
2. The probing logic doesn't break on macOS/Linux (those paths are simply not found).
3. Regression result unchanged: 8/8 PASS.

---

## 2. What was deliberately NOT done (and why)

| Item | Why deferred |
|---|---|
| `create_ui` mock display test | `slint 1.16.1` doesn't expose `backend-testing` as a public feature. Would require either an upstream slint upgrade or a workspace-level mock `Platform` impl. Documented in HANDOFF §6 + plan §K.2.4. |
| Replacing `ConversationCoordinator::execute_hidden_subagent_internal` with `ActorRuntime` | Multi-day refactor. The current path is multi-turn LLM; `SkillActor::tick` is single-shot. Phase A1 (K.2.3) will introduce `LongRunningSkill` to bridge this. See HANDOFF §6 + plan §K.2.3. |
| `slint::include_modules!()` extraction to `slint_glue.rs` | Pending user direction (K.2.1). Would slim `mod.rs` from 989 → ~700 lines but no functional change. |
| Coordinator subagent path split | Pending user direction (K.2.2). Prep work for A1. |
| SkillActor multi-turn redesign | Pending user direction (K.2.3). The "real" flag-flip milestone. |
| Plan doc top-level TL;DR + status snapshot | **Done this turn** — added at the top of `2026-06-19-post-reference-roadmap.md` and in HANDOFF §0. |

---

## 3. Verification commands (copy-paste ready)

```bash
cd /e/agent-project/northhing

# 1. State
git log --oneline HEAD~6..HEAD
git status

# 2. Full regression
bash scripts/regression-test-desktop.sh

# 3. Test counts
cargo test -p northhing-agent-dispatch --lib 2>&1 | tail -20
cargo test -p northhing --lib 2>&1 | tail -20

# 4. Lints
cargo check -p northhing --lib 2>&1 | tail -10
cargo check -p northhing-agent-dispatch --lib 2>&1 | tail -10

# 5. Confirm no hand-written unsafe in app_state (Phase I.2 invariant)
grep -rn "unsafe" src/apps/desktop/src/app_state/ | head -20

# 6. Confirm const flags still false
grep -E "^\s*pub const (USE_LIGHTWEIGHT_ACTOR|USE_ONESHOT_DISPATCHER|USE_ACTOR_IPC|USE_DISPATCHER_IPC)" \
  src/crates/execution/agent-dispatch/src/flags.rs
```

---

## 4. Questions the reviewer should ask

1. **`c2d2bc8` — does the visibility chain leak?** Check that no helper is exposed beyond what `mod.rs` needs (no `pub` instead of `pub(super)`).
2. **`0da5130` — does `ClosureActor` require `Send + Sync`?** It's spawned via `tokio::spawn` which requires both. Verify the trait bounds on `F`.
3. **`7aa4310` — does the on-disk schema break?** Without a migration step, an old `SessionRelationship` JSON missing `parent_dialog_turn_id` should still load. Run an existing on-disk session through `list_sessions` if possible.
4. **`5ac37c6` — does the PATH probe work on Linux?** The probing logic should be a no-op when `/c/Users/...` doesn't exist (i.e., not on Windows). Verify by reading the script.
5. **Is the `LazyLock<Arc<AppState>>` global initialized exactly once?** Phase I.2 introduced this; verify no double-init or static-cycle issue.
6. **Does the debug-log helper `log_debug_event` leak threads?** Each fire-and-forget spawns a fresh thread. Verify it doesn't accumulate under repeated use (no thread pool exhaustion).

---

## 5. Files changed (full list)

```
HANDOFF.md                                          (M)
docs/plans/2026-06-19-post-reference-roadmap.md     (M)
scripts/regression-test-desktop.sh                  (M)
src/apps/desktop/src/app_state/actor.rs             (A)
src/apps/desktop/src/app_state/inspector.rs         (A)
src/apps/desktop/src/app_state/inspector_model_status.rs  (A)
src/apps/desktop/src/app_state/log.rs               (A)
src/apps/desktop/src/{app_state.rs => app_state/mod.rs}  (R)
src/apps/desktop/src/app_state/sessions.rs          (A)
src/apps/desktop/src/app_state/skills.rs            (A)
src/crates/assembly/core/src/agentic/core/session.rs          (M)
src/crates/assembly/core/src/agentic/persistence/manager.rs   (M)
src/crates/execution/agent-dispatch/Cargo.toml      (M)
src/crates/execution/agent-dispatch/src/runtime.rs  (M)
src/crates/execution/agent-dispatch/tests/telemetry_test.rs  (M)

A = Added, M = Modified, R = Renamed
```

Plus this file (the review brief itself, not yet committed):
```
docs/reviews/2026-06-20-session-review-brief.md     (A — pending)
```

---

## 6. Sign-off criteria

- [ ] All 8 regression checks pass
- [ ] Both test suites green (8 + 12)
- [ ] No new compiler warnings
- [ ] `git status` clean
- [ ] `create_ui` still wires 9 callbacks (verify with `grep "ui.on_" src/apps/desktop/src/app_state/mod.rs | wc -l` → should be 9)
- [ ] 4 const flags in `agent-dispatch::flags` still all `false`
- [ ] HANDOFF.md §0 TL;DR matches reality
- [ ] Plan doc §TL;DR (added at top) matches reality