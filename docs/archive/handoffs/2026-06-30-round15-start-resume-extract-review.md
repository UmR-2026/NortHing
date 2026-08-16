# R15 Review Guide — `start_resume` extracted from dispatch to `command_router_resume`

## Summary

One-file-internal god-method extraction. Closes the R14 D-deviation (`command_router_dispatch.rs` 842 lines, 4% over the 800-line cap) by moving `start_resume` (~126 lines) into a new sibling file.

| Metric | Before R15 | After R15 | Delta |
|---|---:|---:|---:|
| `command_router_dispatch.rs` lines | 842 | **718** | **-124 (-15%)** |
| `command_router_resume.rs` lines (NEW) | — | 152 | +152 |
| `command_router.rs` facade lines | 304 | 306 | +2 |
| `cargo test -p northhing-core --lib --features 'service-integrations,product-full'` | 899 / 0 / 1 | **899 / 0 / 1** | unchanged |
| R14 D-deviation (dispatch > 800 cap) | OPEN | **CLOSED** | ✅ |

Commit: `5c2ac1b`.

## What to review

### Per-file diff

| File | What changed |
|---|---|
| `command_router_dispatch.rs` | Removed `start_resume` function (L345-470 in old). Updated module doc L1-16 to note `start_resume` lives in `command_router_resume`. Added `use super::command_router_resume::start_resume;` to the imports so the L124 ResumeSession match arm and L649 `route_pending` call site resolve unchanged. |
| `command_router_resume.rs` (NEW, 152 lines) | Defines `pub(super) async fn start_resume(state: &mut BotChatState, page: usize, s: &'static BotStrings) -> HandleResult`. Module doc explains R15 split, scope, and the `truncate_label` cross-sibling dependency. Imports: `HandleResult` (facade), `truncate_label` (dispatch), `BotChatState/BotDisplayMode/PendingAction` (state), `result_from_menu` (util), `need_session_view` (view), `fmt_count/BotStrings` (locale), `MenuItem/MenuView` (menu). Internal `use crate::agentic::persistence::PersistenceManager;` and `use crate::infrastructure::PathManager;` preserved from R14 (function-local use lines). |
| `command_router.rs` (facade) | Doc comment L13 lists the new sibling. Internal use line L33-38 swaps `start_resume` from `command_router_dispatch` import block to a new `use super::command_router_resume::start_resume;` import. No `pub use` re-export change. |
| `mod.rs` | Added `pub mod command_router_resume;` between `command_router_questions` and `command_router_session` (alphabetical order). |

### Why no `pub use` re-export

`start_resume` is only called from 2 places inside the bot module (dispatch L124 + route_pending L649). It is **not** part of the bot's public API surface (`command_router.rs` facade's stable exports). So it stays `pub(super)` — visible only within the bot module — and the cross-sibling use lines provide resolution. This matches the R14 pattern for `handle_question_reply` / `submit_question_answers` (also `pub(super)`).

### Critical observations (please verify)

1. **`truncate_label` is still in `command_router_dispatch.rs` (not moved to `util`)**.
   - Resume.rs imports it via `use super::command_router_dispatch::truncate_label;`.
   - This mirrors the existing R14 cross-sibling import in `command_router_util.rs` L10 and `command_router_view.rs` L10.
   - Creates a 2-edge cross-sibling cycle: `dispatch → resume` (for `start_resume`) AND `resume → dispatch` (for `truncate_label`). The cycle does not fire at runtime (both are leaves).
   - **Alternative (rejected for R15)**: move `truncate_label` to `command_router_util.rs`. Would touch 5 files and grow diff by ~+30 net. If you (QClaw / Kimi) prefer the util placement, easy to do as a follow-up — see handoff §1.

2. **No new tests**. start_resume is already exercised by R14's `command_router_tests.rs` indirectly:
   - `pending_expires_after_ttl`
   - `active_workspace_path_prefers_pro_workspace_then_assistant`
   Both run with the full feature set. Plan L27 explicitly noted "No new tests needed for the extraction itself".
   - If you want direct pagination / branch coverage, 2-3 small tests would slot in. Suggest leaving it to R15.1 unless flagged.

3. **`start_resume` body is byte-identical to R14**. No logic change, no signature change, no comment change other than the module-level doc. The 2 call sites (dispatch L124 + route_pending L649) needed **zero** changes — the function resolves from `super::*` regardless of which sibling it lives in.

4. **No new iron-rule violations**. Verified `unwrap()` / `panic!` / `let _ =` counts in the new file match the pre-R15 dispatch section:
   - `unwrap_or_default()` on `chrono::DateTime::from_timestamp` — same as R14
   - No new `unwrap()` or `expect()` introduced

5. **`mod.rs` ordering** — `command_router_resume` sits between `command_router_questions` and `command_router_session` alphabetically. Matches R14 ordering convention.

6. **Facade `command_router.rs` line count grew by 2** (304 → 306). The growth is entirely doc + use-line re-route; no logic added. Still well under the 200-line cap (R14 R13b R9 facade target).

## Refs

- Plan: `docs/handoffs/2026-06-30-r15-god-object-plan.md` (P0 section L9-80)
- Handoff: `docs/handoffs/2026-06-30-round15-start-resume-extract-handoff.md`
- Spec: not produced (R15 was a 1-function mechanical extraction, not a design exercise — plan YAML in `2026-06-30-r15-god-object-plan.md` L46-68 served as the spec)
- Commit: `5c2ac1b` (refactor)
- Main HEAD before this commit: `412c0d4` (R15 plan doc + 4 visual diagrams)
- Predecessor: `ed35b81` (R14 refactor that left dispatch at 842)

## Questions for reviewer

1. Is the `truncate_label` placement in `command_router_dispatch` acceptable for R15, or do you want a follow-up R15.1 to move it to `command_router_util`?
2. Is the absence of new tests acceptable given existing indirect coverage in `command_router_tests.rs`?
3. The R14 review guide flagged `command_router_dispatch.rs` 832 lines as a D-deviation within 10% tolerance. After R15 the file is 718 lines. Is this acceptable as the dispatch ceiling, or should R16 consider further splits (e.g. `route_pending` per-`PendingAction` per Kimi R14 P2)?
4. Module doc in `command_router_resume.rs` explicitly notes the cross-sibling dependency on `truncate_label`. Is this level of documentation appropriate, or should it be trimmed?
5. Did Mavis's direct-take-over (no subagent dispatch) match your expectation for a 5-minute mechanical extraction, or should future R(N+1) tasks of this size still go through a plan YAML?

## Sign-off request

Please verify:

- [ ] `cargo test -p northhing-core --lib --features 'service-integrations,product-full'` → 899/0/1 (unchanged)
- [ ] `command_router_dispatch.rs` ≤ 720 lines (currently 718)
- [ ] `command_router_resume.rs` ≤ 200 lines (currently 152)
- [ ] No NEW iron-rule violations in production code
- [ ] Both call sites of `start_resume` (dispatch L124, route_pending L649) still resolve
- [ ] No new mojibake / encoding issues (resume.rs is UTF-8 clean)

APPROVE / REJECT + score (e.g. 8/10) + minor observations + decision on R15.1 / R16.