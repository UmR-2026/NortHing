# Coding Brief — 2026-06-20

> **Audience:** The next coding agent (human or LLM) picking up where
> this session left off. Companion to `HANDOFF.md` (status) and
> `docs/plans/2026-06-19-post-reference-roadmap.md` (roadmap).
>
> **Pair document:** `docs/reviews/2026-06-20-session-review-brief.md`
> covers the same scope from the **reviewer** perspective — they should
> agree on state and verification.

---

## 0. TL;DR for the coder

```text
Branch   : v3-restructure
HEAD     : fa868ae (clean)
Commits  : 111 (on this branch, per git rev-list --count HEAD)
Tests    : 8/8 regression, 20/20 agent-dispatch, 12/12 desktop
Callbacks: 10/10 wired
Warnings : 0
Next     : pick a K.2 candidate from HANDOFF.md §7 / HANDOFF_NEXT_SESSION.md
```

**One thing to remember**: before writing any code, run the
`preflight-skill-check` skill. It will auto-load `reference-library`,
which enforces a 4-step read-first workflow on 4 covered domains.
**This is not optional.**

---

## 1. The 4-step workflow (enforced by `reference-library`)

For every task in the 4 covered domains (skill / actor / session /
checker), you must do this in order:

```
1. Read  .agents/reference/<domain>/README.md       (overview + file ordering)
2. Read  .agents/reference/<domain>/SIGNATURES.md   (function signature card)
3. Read  .agents/reference/<domain>/NOTES.md        (do-NOT-copy warnings)
4. Open  .agents/reference/<domain>/0N-xxx.rs       (specific mirror)
         copy the pattern with header:
         // Pattern source: .agents/reference/<domain>/0N-xxx.rs
```

Domain mapping:

| If your task touches… | Reference domain |
|---|---|
| Slint callback wiring, SessionSummary, ConversationCoordinator | `session/` |
| New subagent session model | `session/` + `actor/` |
| Skill toggle, resolver | `skills/` |
| Actor / dispatcher / SkillActor / ActorRuntime | `actor/` (use `07-impl-plan-task-map.md`) |
| Plan compliance checker | `checker/` |

**Outside the 4 domains**: you still run `preflight-skill-check`, but no
specific mirror applies. Just be defensive — read existing code in the
area before changing.

---

## 2. State you need to know

### What's done
- Phases A–I of `2026-06-19-post-reference-roadmap.md` — all complete
- Tag `v0.1.0` applied at `2813b36`
- 6 commits this session (44→47): split `app_state.rs`, `spawn_one_shot` API,
  InMemoryRelationship fields, regression script bootstrap, plus docs

### What's NOT done (and why)
- **K.2.1–K.2.5**: explicit candidates, see HANDOFF.md §7 / HANDOFF_NEXT_SESSION.md
- **slint 1.16.1 backend-testing**: upstream blocker, K.2.4 deferred
- **Coordinator subagent replacement**: requires `LongRunningSkill` (K.2.3),
  deferred as multi-day refactor

### What's locked (do NOT change without flag flip)
- `USE_LIGHTWEIGHT_ACTOR = false` (agent-dispatch::flags)
- `USE_ONESHOT_DISPATCHER = false`
- `USE_ACTOR_IPC = false`
- `USE_DISPATCHER_IPC = false`
- `USE_SKILL_REGISTRY = true`
- `USE_SOFTWARE_FALLBACK = true`
- `SESSION_TREE_VIEW = true`
- `DEFAULT_MODE_ID = "code"`

Flipping any of these without an integration test is a regression risk.

---

## 3. Architectural map (where to look)

```
src/apps/desktop/src/app_state/
├── mod.rs                              # create_ui (single wiring point) + AppState + run_event_loop + tests
├── actor.rs                            # maybe_construct_actor_runtime (USE_LIGHTWEIGHT_ACTOR gate)
├── inspector.rs                        # build_mcp_status_string (live MCP read)
├── inspector_model_status.rs           # build_model_status_string
├── log.rs                              # log_debug_event (fire-and-forget debug helper)
├── sessions.rs                         # build_sessions_model / build_messages_model + depth walk
└── skills.rs                           # build_skills_model / refresh_skills_ui

src/crates/execution/agent-dispatch/
├── src/
│   ├── actor.rs                        # SkillActor trait + ActorContext / Output / Event / Error / Schedule
│   ├── runtime.rs                      # ActorHandle + ActorRuntime + spawn_one_shot + ClosureActor
│   ├── flags.rs                        # 4 const flags, all false
│   ├── telemetry.rs                    # TelemetrySink trait + NoopTelemetrySink
│   └── spawn/{tokio,ipc}_adapter.rs
└── tests/
    └── telemetry_test.rs               # 8 unit tests (Notify await_join + closure_actor + periodic + signal)

src/crates/contracts/runtime-ports/src/
├── lightweight_task.rs                 # ToolDispatcherPort (stub)
└── mcp.rs                              # McpCatalogReader (rich async)

src/apps/desktop/src/
├── mcp_adapter.rs                      # wraps MCPService, impls McpCatalogReader + McpCatalogPort marker
└── app_state/                          # (see above)
```

### What you'll edit for each K.2 candidate

| K.2.x | Files you'll touch |
|---|---|
| K.2.1 (slint extraction) | `src/apps/desktop/src/app_state/{mod.rs, new: slint_glue.rs}` |
| K.2.2 (coordinator split) | `src/crates/assembly/core/src/agentic/coordinator.rs` |
| K.2.3 (LongRunningSkill) | `src/crates/execution/agent-dispatch/src/{actor.rs, runtime.rs}` + tests |
| K.2.4 (mock display) | NEW crate `crates/test-platforms/slint-noop` + tests |
| K.2.5 (plan closeout) | `docs/plans/2026-06-19-post-reference-roadmap.md` only |

---

## 4. Workflow rules (carry-over from previous sessions)

### MUST
- `preflight-skill-check` before writing code (always — even for tiny edits)
- Read the 4 reference docs in order for 4-domain tasks
- TDD: red → green → commit per step
- One commit per logical change
- Bump `HANDOFF.md` `## 9. Commit Log` + `## 10. Verification` after each commit
- Update `## 0. TL;DR` "Last updated" date when you change HANDOFF.md

### MUST NOT
- Touch `coordinator.rs:4172-5025` (the heavy subagent path) to extend it
- Touch `execution_engine.rs`, `tool_pipeline.rs`
- Add `unsafe` to `app_state/` (slint macros emit internal `unsafe` blocks;
  hand-written `unsafe` is forbidden there)
- Enable any const flag without an integration test
- Modify `Cargo.toml [workspace.dependencies]` (only `[workspace.members]`)
- Modify `.agents/reference/` directly — it's a generated mirror

### SHOULD
- Update plan doc `docs/plans/2026-06-19-post-reference-roadmap.md` with
  any new phases / decisions / deviations
- Run `node scripts/test_reference_skill.cjs` if you edit any skill description
- Run `node scripts/copy_reference.cjs` if you modify any `src/` file that's
  mirrored in `.agents/reference/<domain>/`

---

## 5. Commit message conventions

```
feat(<area>): <description>            # new feature
refactor(<area>): <description>        # code change, no behavior
fix(<area>): <description>             # bug fix
chore(<area>): <description>           # tooling, deps, non-code
docs(<area>): <description>            # docs only
test(<area>): <description>            # tests only
perf(<area>): <description>            # performance

Examples from this session:
  feat(agent-dispatch+desktop): spawn_one_shot + on_send_message demo (A3)
  refactor(desktop): split app_state.rs into 6 submodules (Phase B)
  docs(handoff+plan+review): full session closeout + K.4 section + review brief
```

Commit pattern at end of session:
1. `feat` / `refactor` / `fix` commits (one per logical change)
2. `docs(handoff+plan): bump N→N+M` (update HANDOFF.md `## 9` + plan doc §K)

---

## 6. Verification commands

Run **before claiming done** for any task:

```bash
cd /e/agent-project/northhing

# Quick sanity (lib + tests, both must be 0 warnings)
cargo check -p northhing --lib 2>&1 | tail -10
cargo check -p northhing --tests 2>&1 | tail -10

# Full regression
bash scripts/regression-test-desktop.sh

# Test counts (sources of truth)
cargo test -p northhing-agent-dispatch --lib 2>&1 | tail -10   # expect 20/20
cargo test -p northhing --lib 2>&1 | tail -10                  # expect 12/12

# State confirmation
git status              # must be clean
git log --oneline -5    # confirm new commits
```

**Evidence before assertions**: do not claim "tests pass" without
showing the tail of the test output. Do not claim "regression green"
without showing the `8/8 PASS` line.

---

## 7. Cross-references

### Read these before coding
- `HANDOFF.md` (project status, 257 lines)
- `docs/plans/2026-06-19-post-reference-roadmap.md` (roadmap, 916 lines)
- `.agents/reference/<domain>/README.md` + `SIGNATURES.md` + `NOTES.md`
- `.agents/skills/reference-library/SKILL.md` (workflow)
- `.agents/skills/writing-plans/SKILL.md` (if designing a new plan)
- `.agents/skills/test-driven-development/SKILL.md` (if writing tests)

### Update these after coding
- `HANDOFF.md` — `## 0. TL;DR`, `## 9. Commit Log`, `## 10. Verification`
- `docs/plans/2026-06-19-post-reference-roadmap.md` — append to §K or new phase
- `docs/HANDOFF_NEXT_SESSION.md` — only if you want to leave a different
  briefing than the current one (otherwise the existing one is fresh)
- `docs/reviews/` — only if there's something new worth reviewing

### Companion documents
- `docs/reviews/2026-06-20-session-review-brief.md` — reviewer's view
- `docs/handoffs/2026-06-20-coding-brief.md` — **this document**
- `docs/architecture/deep-review.md` — architecture reference for DeepReview feature
- `docs/notes/plan-compliance-checker.md` — checker notes
- `docs/notes/preflight-skill-check.md` — preflight notes

---

## 8. Quick-start for the very next task

If you're picking up **K.2.1** (slint extraction, recommended start):

```bash
cd /e/agent-project/northhing

# 1. Confirm state
git status   # clean
git log --oneline -3   # 840bd4f at HEAD

# 2. Read the relevant reference (none of 4 domains — slint glue is internal)
#    but skim HANDOFF.md §3 (Submodule Split) for the design decision context

# 3. Create the new module
touch src/apps/desktop/src/app_state/slint_glue.rs

# 4. In slint_glue.rs:
#    slint::include_modules!();
#    pub use self::{AppWindow, ...Item};

# 5. Update mod.rs:
#    mod slint_glue;
#    pub use slint_glue::{AppWindow, ...};
#    // remove the inline slint::include_modules!() call

# 6. Verify
cargo check -p northhing --lib
cargo test -p northhing --lib

# 7. Commit
git add src/apps/desktop/src/app_state/
git commit -m "refactor(desktop): extract slint::include_modules!() to slint_glue.rs (K.2.1)"

# 8. Update HANDOFF.md
#    - §9 commit log: add this commit
#    - §10 verification: confirm 12/12 desktop tests
#    - §3 (Submodule Split): add slint_glue.rs row to the table
git add HANDOFF.md
git commit -m "docs(handoff): record K.2.1 slint_glue extraction"

# 9. Run full regression
bash scripts/regression-test-desktop.sh

# 10. Done. Hand off.
```

---

## 9. If you get stuck

1. **Compile error you don't recognize**: `cargo clean -p <crate>` then retry.
2. **Test count regressed**: `git log --oneline -10` to see what changed; revert if needed.
3. **Skill not auto-loading**: check `.agents/skills/<name>/SKILL.md` frontmatter `description`
   — keywords must match what `preflight-skill-check` scans for.
4. **Reference mirror is stale**: run `node scripts/copy_reference.cjs`.
5. **Reference domain doesn't cover your task**: that's fine — `preflight-skill-check`
   still applies, just no specific mirror to read.
6. **Don't know which file owns X**: `git log --oneline -- <path>` shows the history.

For issues not covered here, document them in a new section under
`docs/notes/` and bump this brief.