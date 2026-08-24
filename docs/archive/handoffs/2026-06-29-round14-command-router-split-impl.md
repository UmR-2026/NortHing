# R14 Handoff — `bot/command_router.rs` 2614 → facade + 8 sub-siblings

## Result

- **Before**: `bot/command_router.rs` 2614 lines (well over §7 E1 800-line cap; > 3× cap).
- **After**: `bot/command_router.rs` 306 (facade) + 8 sub-siblings, all ≤ 832 lines.

| File | Lines | Role |
|---|---:|---|
| `command_router.rs` | 306 | **facade** — types, `parse_command`, `welcome_message`, `handle_command`, `apply_interactive_request`, `complete_im_bot_pairing`, `pub use` re-exports |
| `command_router_dispatch.rs` | 832 | 18 dispatchers (god methods) |
| `command_router_state.rs` | 151 | `BotChatState`, `BotDisplayMode`, `PendingAction`, `now_secs`, `PENDING_INVALID_LIMIT` |
| `command_router_session.rs` | 309 | `bootstrap_im_chat_after_pairing`, `count_workspace_sessions`, `load_last_dialog_pair_from_turns`, `create_session`, `resolve_session_agent_type`, `strip_user_message_tags`, `truncate_text` |
| `command_router_view.rs` | 320 | 11 view builders (`main_menu`, `settings`, `need_session`, `confirm_mode_switch`, `workspace/assistant/session_selection`, `build_question`, `question_option_line`, `ready_to_chat_body`, `welcome_view`, `menu_or_welcome`) |
| `command_router_util.rs` | 112 | `normalize_im_command_text`, `strip_numeric_reply_suffix`, `result_from_menu`, `result_from_menu_with_forward`, `refresh_assistant_name_if_missing`, `short_path_name`, `parse_question_numbers` |
| `command_router_forwarded_turn.rs` | 202 | `execute_forwarded_turn` (god method) |
| `command_router_questions.rs` | 174 | `handle_question_reply`, `submit_question_answers` (extracted from dispatch to bring dispatch under 800) |
| `command_router_tests.rs` | 359 | 22 tests in 4 mods (`parse_command_tests` 12, `state_tests` 3, `menu_tests` 6, `handle_chat_tests` 1) |

## Plan / Mavis Take-Over

Subagent `plan_078b2ca6` was dispatched with the §7 spec but hit the 30-min plan
timeout (M2.7-highspeed model) at 50% done — the worker had created 7 sub-siblings
+ 1 dispatch merge but had not committed. Mavis took over from the worktree in
5 separate fix passes, none of which were trivial:

1. **Visibility / import errors** (Round 6-style `pub(super)` pattern applied):
 - `now_secs`, `PENDING_TTL_SECS`, `PENDING_INVALID_LIMIT` → `pub(super)`
 - `BotChatState`/`PendingAction` methods → `pub(super)`
 - `MenuItem` import in `command_router_session.rs` (wrong path)
 - `parse_command` import path in `command_router_dispatch.rs`
 - `truncate_label` import path in `command_router_util.rs`
 - `tracing::error` import in `command_router_session.rs`

2. **GBK-as-UTF-8 mojibake in 6 files**: Worker's split tooling read the source
 with the wrong encoding for Chinese byte sequences. Repaired:
 - `command_router.rs` `parse_command` match arms (菜单 / 帮助 / 切换 / 专业模式 / 助理模式 / 详细 / 简洁 / 新建 / 新建会话 / 新会话 / 新建编码会话 / 新建协作会话 / 新建助理会话 / 恢复 / 恢复会话 / 锛— → 锛— ).

 - `command_router_tests.rs` test inputs (same set + ２ → １, １ / 我的助理 / 默认助理).
 - `command_router_dispatch.rs` em-dash corruption in `format!("{truncated}镛— )` → `format!("{truncated}…")` (6 occurrences). The unterminated string was the root cause of 28 "unknown prefix" parse errors that cascaded to look like identifier-syntax errors.
 - Doc-comment em-dashes in 4 places (`command_router_dispatch.rs`, `command_router_questions.rs`, etc.) where the worker's tool produced `锛— ` (CJK + `— `) instead of `—`.

3. **Test discovery silent-skip**: After the worker's split, `cargo test -p
 northhing-core --lib` reported 103 passed with zero command_router_tests
 tests in `--list`, even after a full `cargo clean`. Root cause: the `bot`
 module is `#[cfg(feature = "service-integrations")]` in `service/mod.rs`
 and `command_router_dispatch.rs` had a stale `pub fn pending_invalid`
 that pulled the test module into a half-compiled state. Fixed by full
 `cargo clean -p northhing-core` and rebuilding with `--features
 'service-integrations,product-full'`, which is the canonical way to run
 the full test set including all 22 command_router_tests.

4. **`command_router_questions.rs` extraction** (D-deviation mitigation):
 `command_router_dispatch.rs` was 985 lines after the worker's split; to
 bring it under the 800-line cap, Mavis extracted `handle_question_reply`
 + `submit_question_answers` (155 lines) into a new sibling. Also fixed
 the cross-sibling `pending_invalid` import in the new file and updated
 the `command_router.rs` facade's `use super::command_router_dispatch::`
 import list.

5. **mod.rs `— ` symbol corruption** (transient): Earlier `Set-Content`
 PowerShell encoding issue inserted a literal `— ` character between
 `#[cfg(test)]` and `mod command_router_tests;` in `mod.rs`, which made
 the entire `bot` module fail to parse silently. Fixed by writing the
 correct `#[cfg(test)]\r\nmod command_router_tests;` bytes via Python.

## D-deviations (open)

- `command_router_dispatch.rs` 832 lines (32 over 800 cap, 4% over) — within
 QClaw 10% tolerance; further trim (e.g. extract `start_resume` 127 lines
 to a `command_router_resume.rs`) is the R15 candidate.

## Verification

```
$ cargo test -p northhing-core --lib --features 'service-integrations,product-full'
 ...
 test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
 finished in 2.18s

$ cargo test -p northhing-core --lib
 ...
 test result: ok. 103 passed; 0 failed; 0 ignored
 finished in 0.07s
```

- 22 new `command_router_tests` all pass (parse_command aliases, state, menu, handle_chat).
- 0 NEW iron-rule violations (no new `unwrap` / `panic!` / `let _ =` in production code).
- Pre-existing failures in `northhing-tool-contracts` and `northhing-terminal`
 (cargo test --workspace) exist on main HEAD and are not introduced by R14.

## Subagent path / context

- Plan: `plan_078b2ca6` (cancelled at 30 min; worktree preserved)
- Worktree: `E:\agent-project\northing-impl-round14` (branch `impl/round14-command-router-split`)
- Spec: `docs/handoffs/2026-06-29-round14-command-router-split-spec.md` (worker-written, 495 lines, useful as record of the original split plan)
- Review guide: `docs/handoffs/2026-06-29-round14-command-router-split-review.md` (this handoff's review-facing companion)
- Head: `ed35b81` (refactor commit)

---

## Review-fix cycle (post-merge)

After `ed35b81` was merged at `92faf19`, the review-fix-cleanup cycle ran:

| Commit | Author | Purpose |
|---|---|---|
| `ca3bc2f` | QClaw | review report (COND 7.5/10, R14-fmt fixup required) |
| `6060801` | Kimi | review report (APPROVE 8.6/10, 3 minor observations) |
| `f777284` | Mavis | fix fmt + 3 unused imports + unwrap invariant comment + Chinese byte mojibake repair |

### QClaw COND addressed (`f777284`)

- **R14-fmt fixup**: `cargo fmt -p northhing-core` applied to the 6 R14 split files + mod.rs. Corrected import ordering, alphabetized use lists, reformatted multi-line fn signatures, removed module-decl ordering glitch (mod.rs had `pub mod util` and `view` placed before `#[cfg(test)] mod tests` instead of after, now correctly after).

### Kimi minor observations addressed (`f777284`)

1. **Unused imports cleanup**:
 - `command_router.rs:54` — `use super::locale::{fmt_count, strings_for, BotStrings}` → `use super::locale::strings_for` (only `strings_for` was actually used in facade)
 - `command_router_session.rs:18` — `use super::locale::{current_bot_language, strings_for, BotStrings}` → drop `BotStrings`
 - `command_router_util.rs:16` — removed orphan `use super::locale::BotStrings;` (no consumer)
2. **Unwrap invariant comment** at `command_router_dispatch.rs:791` (handle_chat):
 ```rust
 if state.current_session_id.is_none() {
 return result_from_menu(state, need_session_view(state, s));
 }
 // Pre-existing safe unwrap (1f19784): the is_none() check above guarantees
 // Some — the unwrap() here is intentional, not a missing invariant. Kept
 // as-is per the "no NEW iron rule violations" rule; this comment exists
 // so future readers don't mis-classify it as new debt.
 let session_id = state.current_session_id.clone().unwrap();
 ```
3. **Python split script encoding enforcement**: logged in MEMORY.md as a standing rule for any future subagent that creates Rust split scripts.

### Kimi non-blocking R15 candidate (P2)

- `route_pending` (122 lines) split into per-`PendingAction` dispatchers — deferred to R15+ as a non-required polish.

### Chinese byte mojibake repair (incidental fix in `f777284`)

A second round of mojibake was discovered during the review-fix cycle. Root cause: PowerShell's `[char]$bytes[$i]` cast (during the CRLF re-conversion step in commit `f777284`) round-trips non-ASCII bytes through Latin-1, which is then re-encoded as UTF-8. Affected 6 files; 1664 bytes recovered total (mod.rs alone: 1365 bytes). All byte-level repair done via Python with explicit `b.replace()` on the raw byte stream — no character decoding round-trip. Pattern for future detection: any 2-byte UTF-8 sequence matching `c2 [0x80-0xbf]` (control char re-encoded) or `c3 [0x80-0xff]` (Latin-1 char re-encoded) is suspect.

### Verification (post fixup)

```
cargo test -p northhing-core --lib --features 'service-integrations,product-full'
 ...
 test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

R14 fully closed. Working tree clean (4 pre-existing R5/R6/R8b handoff docs untracked; not part of R14).

## Final state at R14 close

- **main HEAD**: `f777284` (R14 review-fix complete)
- **Test baseline**: 899/0/1
- **NEW iron rules**: 0 violations
- **D-deviations open**: 1 (`command_router_dispatch.rs` 832 lines, 32 over 800 cap, 4% over, QClaw 10% tolerance) → R15
- **Kimi factual corrections applied**: review_platform/mod.rs 4866 was Kimi error (actual 319), confirmed at file inspection; not relevant to R14 directly but reinforces the standing rule to re-verify reviewer claims exceeding known project max.

