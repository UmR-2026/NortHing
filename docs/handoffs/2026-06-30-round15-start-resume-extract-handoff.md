# R15 Handoff — `start_resume` extraction

## What landed

| File | Lines (before) | Lines (after) | Delta | Notes |
|---|---:|---:|---:|---|
| `command_router_dispatch.rs` | 842 | 718 | **-124 (-15%)** | Below 800-line cap; R14 D-deviation closed |
| `command_router_resume.rs` (NEW) | — | 152 | +152 | Owns `start_resume` god method |
| `command_router.rs` (facade) | 304 | 306 | +2 | doc comment + use-line re-route |
| `mod.rs` | (unchanged) | (unchanged) | +1 | `pub mod command_router_resume;` declared between questions and session (alphabetical) |

Commit: `5c2ac1b` (`refactor(remote-connect): R15 extract start_resume to command_router_resume`).

## Why Mavis did this directly (no subagent)

`start_resume` is a single 126-line function with 2 callers (`dispatch` ResumeSession arm + `route_pending` SelectSession handler). The whole move is mechanical:

1. Move function body verbatim into the new sibling.
2. Add the imports the function needs (same imports `dispatch` already had).
3. In `dispatch`, replace the function body with `use super::command_router_resume::start_resume;` and remove the `start_resume` token from the `command_router_dispatch` use list.
4. In `command_router.rs` facade, swap the use source from dispatch to resume.
5. Declare the new module in `mod.rs`.

There is no design decision to defer to a subagent — the plan YAML template in `docs/handoffs/2026-06-30-r15-god-object-plan.md` L46-68 already nails down the 6 imports, the `pub(super)` rule, and the verification steps.

A worker subagent would burn 30 minutes of plan-time on what is in practice ~5 minutes of mechanical editing. Per plan L286: "1 个文件内部的小手术, 不需要调 subagent — Mavis 自己 5 分钟搞定 (R14 questions.rs extraction 验证过)".

## Key design decisions

### 1. `truncate_label` stays in `command_router_dispatch`

`truncate_label` is a pure 8-line helper used by 3 places:

- `command_router_util.rs` L10 (existing cross-sibling import)
- `command_router_view.rs` L10, 191, 215, 241, 312 (existing cross-sibling import + 4 call sites)
- `command_router_dispatch.rs` L444 (start_resume's own call site)

Two options were considered:

- **A.** Move `truncate_label` to `command_router_util.rs` (cleanest layer — it's a pure utility). Touches 5 files (util + view + dispatch + facade + this new resume.rs).
- **B.** Leave `truncate_label` in dispatch; resume.rs imports it cross-sibling like util/view already do. Touches 1 file (this new resume.rs + the dispatch.rs use list).

Chose **B** because:

1. It mirrors the existing R14 pattern: `command_router_util.rs` L10 already does `use super::command_router_dispatch::truncate_label;`. We are not introducing new cross-sibling import topology — we are reusing the established one.
2. R15's scope is "close the R14 D-deviation", not "rebuild the R14 sibling layer". Moving `truncate_label` would broaden the diff from -121 net to ~+30 net and risk regression.
3. A `truncate_label` cleanup round (option A) is a cheap standalone refactor; if QClaw / Kimi flag it as a code-smell, it slots naturally into R15.1.

### 2. `start_resume` signature is byte-identical to R14

Same `pub(super) async fn start_resume(state: &mut BotChatState, page: usize, s: &'static BotStrings) -> HandleResult`. Both callers (`dispatch` L124, `route_pending` L649) need zero changes — the function still resolves from `super::*` regardless of which sibling it lives in.

### 3. `truncate_label` cross-sibling import direction is `resume → dispatch`, not `dispatch → resume`

Resume is the new file; it imports `truncate_label` from `command_router_dispatch`. This is **not** a circular import — `command_router_dispatch` does **not** import from `command_router_resume` directly; it imports `start_resume` via `use super::command_router_resume::start_resume;` (sibling-to-sibling same direction).

Wait — let me re-check that. dispatch.rs L29:

```rust
use super::command_router_resume::start_resume;
```

Yes, dispatch DOES import from resume (to make `start_resume` callable in the dispatch match arm). So both directions exist: dispatch → resume (for `start_resume`) and resume → dispatch (for `truncate_label`). That's a 2-edge cross-sibling import cycle, but the cycle does not actually fire at runtime — both are leaf functions, neither calls back into the other inside the same call chain.

If QClaw / Kimi flag this as a smell, the cleanup is option A (move truncate_label to util).

## Verification

```text
$ cargo test -p northhing-core --lib --features 'service-integrations,product-full'
test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.13s
```

Matches R14 baseline (899/0/1) exactly. No new tests were added — start_resume was already exercised via `command_router_tests.rs::pending_expires_after_ttl` and `active_workspace_path_prefers_pro_workspace_then_assistant` (per plan L27).

`cargo check -p northhing-core --tests --features 'service-integrations,product-full'` finishes clean (only pre-existing dead-code warnings unrelated to this commit).

## What did NOT land

- **P2-1 `route_pending` split** — deferred per plan L113. Kimi R14 P2, no rush.
- **`truncate_label` move to util.rs** — design alternative A above, deferred unless reviewer flags it.
- **New tests for `command_router_resume.rs`** — plan L27 says "no new tests needed for the extraction itself". If reviewer wants direct coverage, 2-3 tests for pagination + workspace-resolution branches would slot in cheaply.
- **cron self-reminder** — plan L72 calls for `r15-monitor` cron at 25-min interval. Not needed since this commit was done by Mavis in 5 minutes (no 30-min subagent cycle to monitor).

## Refs

- Plan: `docs/handoffs/2026-06-30-r15-god-object-plan.md` (P0 section L9-80)
- Review guide: `docs/handoffs/2026-06-30-round15-start-resume-extract-review.md`
- Commit: `5c2ac1b` (refactor)
- Main HEAD before this commit: `412c0d4` (R15 plan doc + 4 visual diagrams)