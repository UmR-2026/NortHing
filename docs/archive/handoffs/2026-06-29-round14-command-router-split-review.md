# R14 Review Guide — `bot/command_router.rs` 2614 → facade + 8 sub-siblings

## What to review

| File | Lines | Notes |
|---|---:|---|
| `src/.../bot/command_router.rs` | 306 | facade; types + `parse_command` + `welcome_message` + `handle_command` + `apply_interactive_request` + `complete_im_bot_pairing` + re-exports |
| `src/.../bot/command_router_dispatch.rs` | 832 | 18 dispatchers (god methods) — **D-deviation**: 32 over 800 cap (4%, within 10% tolerance) |
| `src/.../bot/command_router_state.rs` | 151 | `BotChatState`, `BotDisplayMode`, `PendingAction`, `now_secs`, `PENDING_INVALID_LIMIT` |
| `src/.../bot/command_router_session.rs` | 309 | session creation, resume-pair loading, IM bootstrap |
| `src/.../bot/command_router_view.rs` | 320 | 11 view builders |
| `src/.../bot/command_router_util.rs` | 112 | shared helpers |
| `src/.../bot/command_router_forwarded_turn.rs` | 202 | `execute_forwarded_turn` (god method) |
| `src/.../bot/command_router_questions.rs` | 174 | `handle_question_reply` + `submit_question_answers` (extracted from dispatch) |
| `src/.../bot/command_router_tests.rs` | 359 | 22 tests in 4 mods |
| `src/.../bot/mod.rs` | (modified) | declares 8 sub-siblings + tests mod |

## Critical observations (please verify)

1. **`command_router_dispatch.rs` is 832 lines (32 over 800 cap, 4% over)**.
   - Largest dispatcher is `start_resume` (L345-468, 127 lines).
   - QClaw 10% tolerance applies; R15 should split `start_resume` out to bring dispatch ≤ 800.

2. **Chinese byte corruption repairs in 6 files**: The R14 worker's split
   tooling (Python `split_command_router.py` script) read source files
   with the wrong encoding for non-ASCII bytes, producing GBK-as-UTF-8
   mojibake. Mavis identified and repaired all 6 affected files. Most
   critical repair: `command_router_dispatch.rs` had `format!("{truncated}镛?)`
   with an unterminated string that caused 28 cascading parse errors.
   After repair: `format!("{truncated}…")`.

3. **`command_router_questions.rs` was extracted by Mavis** (not the worker)
   to bring `command_router_dispatch.rs` from 985 → 832 lines. The
   extraction needed:
   - `command_router.rs` facade: removed `handle_question_reply` /
     `submit_question_answers` from the `command_router_dispatch` import
     list and added a `use super::command_router_questions::{...}` import.
   - `command_router_questions.rs`: imports `pending_invalid` from
     `command_router_dispatch` (since the questions handlers call it for
     invalid-reply recovery).
   - `mod.rs`: added `pub mod command_router_questions;`.

4. **`pub use` re-export of `execute_forwarded_turn`** in the facade
   preserves the IM adapter import path (feishu.rs / telegram.rs / weixin.rs
   do `use super::command_router::execute_forwarded_turn`). No caller
   migration cost.

5. **`pub(super)` is the standard for sibling struct field access** (R9 +
   R13b Kimi confirmed). Used on `BotChatState` fields, `PENDING_TTL_SECS`,
   `PENDING_INVALID_LIMIT`, `now_secs`, and all cross-sibling functions.

6. **Test binary needs `--features 'service-integrations,product-full'` to
   see the 22 new command_router_tests**. Default-feature build still
   passes 103 tests but excludes the bot module entirely (bot is
   `#[cfg(feature = "service-integrations")]` in `service/mod.rs`).
   This is upstream behavior, not a regression.

7. **Mavis take-over from 30-min subagent timeout**: Worker
   `plan_078b2ca6` hit the engine-capped 30-min plan timeout at 50% done.
   For future R(N+1) dispatcher subagent tasks > 2000 lines, Mavis should
   immediately call `mavis team plan extend-timeout --minutes 60` after
   dispatch (lessons in MEMORY.md).

## Refs

- Plan YAML: `plan_078b2ca6` (cancelled)
- Worktree: `E:\agent-project\northing-impl-round14` (branch `impl/round14-command-router-split`)
- Spec: `docs/handoffs/2026-06-29-round14-command-router-split-spec.md` (495 lines, worker-written)
- Impl handoff: `docs/handoffs/2026-06-29-round14-command-router-split-impl.md`
- Commit: `ed35b81` (refactor)
- Main HEAD before merge: `1f19784`

## Questions for reviewer

1. Is the 832-line `command_router_dispatch.rs` acceptable as a one-round
   D-deviation, or should R15 already pre-extract `start_resume`?
2. Is the 22-test count in `command_router_tests.rs` sufficient coverage
   for the new split? (12 parse_command + 3 state + 6 menu + 1 handle_chat)
3. The dispatch.rs has `route_pending` as the largest god method (122 lines).
   Should it be split into per-`PendingAction` dispatchers (R15+ candidate)?
4. Is keeping the `command_router.rs` facade's re-exports of
   `execute_forwarded_turn` the right pattern, or should feishu/telegram/weixin
   be updated to import directly from `command_router_forwarded_turn`?
5. The 4 doc-comment em-dash corruptions (`…"` → `—`) were repaired in
   place; the worker had introduced these via the encoding bug. Any
   concern about leaving the corrupted pattern in the git history?

## Sign-off request

Please verify:
- All 22 new tests pass under `--features 'service-integrations,product-full'`
- No NEW iron-rule violations (unwrap / panic! / let _ =) in production code
- `command_router_dispatch.rs` line count (832) is acceptable
- Pattern parity with R9 / R13b sibling splits

APPROVE / REJECT + score (e.g. 8/10) + minor observations + decision on R15.
