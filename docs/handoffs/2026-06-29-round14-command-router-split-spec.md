# Round 14 Spec: command_router.rs 2614 → facade + 6 sub-siblings (critical #4 god object)

> **目标**: `src/crates/assembly/core/src/service/remote_connect/bot/command_router.rs` 2614 行 → `command_router.rs` facade (~150-200) + 6 sub-siblings（每 sibling ≤ 800 行）。
> **Pattern**: R5-R13b god-object sub-domain split（free fns + BotChatState impl blocks, R9/R13b `pub(super)` standard）。
> **Trigger**: 4th largest god object in project (post-R13b critical list); Kimi P0 in R13b review; QClaw accepted as next critical.
> **Sister round**: R13b already split `remote_ssh/manager.rs` 2303 → 196 (same 7-sibling pattern).

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/assembly/core/src/service/remote_connect/bot/command_router.rs` | wc -l |
| 行数 | **2614** | `wc -l` |
| Mod re-exports | `bot/mod.rs` 16: `pub use command_router::{BotChatState, ForwardRequest, ForwardedTurnResult, HandleResult};` | mod.rs |
| 公开 pub types | 12: `BotDisplayMode`, `BotChatState`, `PendingAction`, `BotQuestionOption`, `BotQuestion`, `BotActionStyle`, `BotAction`, `HandleResult`, `BotInteractiveRequest`, `BotInteractionHandler`, `BotMessageSender`, `ForwardRequest`, `ForwardedTurnResult`, `BotCommand` | Read |
| 公开 pub fns | 8: `parse_command`, `welcome_message`, `handle_command`, `execute_forwarded_turn`, `bootstrap_im_chat_after_pairing`, `complete_im_bot_pairing`, `apply_interactive_request` + re-export `current_bot_language`, `BotLanguage` | mod.rs + Read |
| Cross-crate callers | 3 files: `feishu.rs` (lines 16, 1551, 1583), `telegram.rs` (lines 14, 687), `weixin.rs` (lines 23, 2004) | git grep |
| 私有 async fns | 22 (dispatch, start_switch, start_resume, create_session, route_pending, handle_question_reply, etc.) | Read |
| 私有 sync fns | 18 (parse, view builders, helpers) | Read |
| God methods (≥80 lines) | 6: `dispatch` (~80), `start_resume` (~125), `create_session` (~145), `route_pending` (~125), `handle_question_reply` (~125), `execute_forwarded_turn` (~190) | Read |
| Test mods | 4: `parse_command_tests` (lines 2278-2410, 12 tests), `state_tests` (2412-2456, 3 tests), `menu_tests` (2458-2572, 6 tests), `handle_chat_tests` (2574-2614, 1 test) | Read |

### 1.1 内部依赖图（impl block / free fn → 调用）

| Source | Calls |
|---|---|
| `dispatch` (716) | `main_menu_view` (view), `pending_invalid` (dispatch), `welcome_view` (view), `refresh_assistant_name_if_missing` (util), `handle_cancel_task` (dispatch), `handle_number` (dispatch), `switch_mode` (dispatch), `set_verbose` (dispatch), `start_switch` (dispatch), `new_session_for_mode` (dispatch), `guarded_new` (dispatch), `start_resume` (dispatch), `handle_chat` (dispatch), `menu_or_welcome` (util) |
| `start_switch` (864) | `get_global_workspace_service`, `workspace_selection_view` (view), `assistant_selection_view` (view), `BotChatState::set_pending` (state) |
| `start_resume` (1122) | `PersistenceManager`, `PathManager`, `need_session_view` (view), `MenuItem`, `BotChatState::set_pending` (state) |
| `create_session` (1383) | `get_global_coordinator`, `get_global_workspace_service`, `CoreServiceAgentRuntime`, `build_remote_session_create_request`, `short_path_name` (util) |
| `route_pending` (1572) | `select_workspace` (dispatch), `select_assistant` (dispatch), `select_session` (dispatch), `handle_question_reply` (dispatch), `dispatch` (self-recursive via `Box::pin`), `menu_or_welcome` (util), `pending_invalid` (util) |
| `handle_question_reply` (1830) | `pending_invalid` (util), `build_question_view` (view), `submit_question_answers` (util) |
| `handle_chat` (2006) | `route_pending` (dispatch), `need_session_view` (view), `resolve_session_agent_type` (session) |
| `execute_forwarded_turn` (2072) | `get_or_init_global_dispatcher`, `RemoteConnectSubmissionSource`, `build_question_view` (view), `truncate_at_char_boundary` (util) |

### 1.2 Cross-crate caller detail

| File | Line | Usage |
|---|---|---|
| `feishu.rs` | 16 | `use super::command_router::{...}` — type imports |
| `feishu.rs` | 1551 | `super::command_router::apply_interactive_request(state, &interaction);` |
| `feishu.rs` | 1583 | `use crate::service::remote_connect::bot::command_router::BotLanguage;` (in test) |
| `telegram.rs` | 14 | `use super::command_router::{...}` — type imports |
| `telegram.rs` | 687 | `super::command_router::apply_interactive_request(state, &interaction);` |
| `weixin.rs` | 23 | `use super::command_router::{...}` — type imports |
| `weixin.rs` | 2004 | `super::command_router::apply_interactive_request(state, &interaction);` |

**All callers use `super::command_router::` or `crate::service::remote_connect::bot::command_router::` (re-export paths). Facade MUST re-export every pub item to keep callers compiling.**

### 1.3 R5-R13b lessons

| 错误类 | Round hit | R14 防御 |
|---|---|---|
| Cargo.lock drift (rmcp 1.7→1.8) | R6 | Plan preflight baseline cargo check |
| cargo check stop-at-first-error | R6 | Worker 报"0 NEW errors"必须每个 crate 都跑过 |
| M3 model 慢 (39min silence) | R6 | Plan 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Worker 漏 test attribute | R9b | Worker 拆 test 必保留 `#[test]`/`#[tokio::test]` attribute |
| mod.rs 漏 `pub mod` | R3b | 每个新 sibling 必须在 mod.rs 加 `pub mod` |
| Spec 不列 struct owner → worker 拆错 | R11a | R14 spec §2.2 显式 struct/fn mapping |
| Worker 没报告行数 → D-deviation | R11a | R14 spec §3 强制 "报告当前 sibling 行数" after each cargo check |
| Cross-reference paths 错 | R11b | R14 spec 列出哪些 type 在 facade vs sibling + §6 re-export table |
| Pre-existing unwrap | R11b (16 unwrap in R13b) | 区分 pre-existing vs new, 不"修复" pre-existing |
| Python split script self-overwrite | R8 (30+ min wasted) | R14 spec §4: split scripts MUST source from git HEAD |
| Test path cross-import | R13b (sftp_mkdir_all_prefixes) | R14 spec §5: tests move to own file with `use super::*` only |
| Cross-sibling visibility cascade | R8 (6 rounds) | R14 spec §2.3: explicit `pub(super)` for cross-sibling fields |
| facade `mod.rs` re-export must be pub | R13b | R14 spec §6: re-export table verified by cargo check |
| Inline body for method-name collision | R8 (R8 execute_dialog_turn_impl) | R14 spec: in facade, public `execute_dialog_turn` inlined (here: not needed, no collision in command_router) |

---

## §2 拆分方案（sub-domain split per spec table）

### §2.1 目标文件结构

```
src/crates/assembly/core/src/service/remote_connect/bot/
├── command_router.rs                     NEW ~150-200 (facade: pub types + 8 pub fns + 4 test mods)
├── command_router_state.rs               NEW ~150 (BotChatState + impl + PendingAction + consts + now_secs)
├── command_router_view.rs                NEW ~350 (all 11 view builders)
├── command_router_dispatch.rs            NEW ~400 (16 dispatchers + sub-routines)
├── command_router_session.rs             NEW ~250 (session lifecycle: create + resume + load)
├── command_router_util.rs                NEW ~50 (6 small helpers)
└── command_router_tests.rs               NEW ~400 (4 test mods)
```

### §2.2 struct owner / fn mapping

| Sibling | Owns (line ranges in original) |
|---|---|
| `command_router_state.rs` | L31-33: `PENDING_TTL_SECS`, `PENDING_INVALID_LIMIT` consts; L37-47: `BotDisplayMode` enum + `Default`; L49-133: `BotChatState` struct + impl (`new`, `active_workspace_path`, `set_pending`, `clear_pending`, `pending_expired`); L135-140: `now_secs` helper; L144-170: `PendingAction` enum |
| `command_router.rs` (facade) | L17-27: imports; L174-191: `BotQuestionOption` + `BotQuestion`; L195-239: `BotActionStyle` + `BotAction` + `From<MenuItem>`; L241-276: `HandleResult` + `BotInteractiveRequest` + type aliases + `ForwardRequest` + `ForwardedTurnResult`; L280-411: `BotCommand` enum + `parse_command` (uses `normalize_im_command_text` + `strip_numeric_reply_suffix` from util); L415-417: `welcome_message`; L702-714: `handle_command` entry; L695-698: `apply_interactive_request`; L678-691: `complete_im_bot_pairing` (calls `bootstrap_im_chat_after_pairing`); L2072-2262: `execute_forwarded_turn` (god method — split into 3 phase helpers in `command_router_dispatch.rs`); L2277-2614: 4 test mods |
| `command_router_view.rs` | L444-617: 11 view builders (`welcome_view`, `ready_to_chat_body`, `main_menu_view`, `settings_menu_view`, `need_session_view`, `confirm_mode_switch_view`, `workspace_selection_view`, `assistant_selection_view`, `session_selection_view`, `build_question_view`, `question_option_line`); L798-804: `menu_or_welcome` (since it's a view-router) |
| `command_router_dispatch.rs` | L716-796: `dispatch` (god method — split into 3 phase helpers); L808-845: `switch_mode`, `confirm_then_run`; L847-860: `set_verbose`; L864-911: `start_switch`; L995-1004: `select_workspace`; L1042-1088: `select_assistant`; L1110-1118: `truncate_label`; L1122-1247: `start_resume` (god method); L1249-1277: `select_session`; L1337-1344: `new_session_for_mode`; L1346-1381: `guarded_new`; L1528-1552: `handle_cancel_task`; L1556-1570: `handle_number`; L1572-1692: `route_pending` (god method); L1698-1740: `pending_invalid`; L1830-1956: `handle_question_reply` (god method); L1958-1984: `submit_question_answers`; L2006-2068: `handle_chat`; L2264-2273: `truncate_at_char_boundary` |
| `command_router_session.rs` | L624-675: `bootstrap_im_chat_after_pairing`; L1090-1108: `count_workspace_sessions`; L1279-1321: `load_last_dialog_pair_from_turns`; L1323-1325: `strip_user_message_tags`; L1327-1335: `truncate_text`; L1383-1524: `create_session` (god method — split into 3 phase helpers); L1993-2004: `resolve_session_agent_type` |
| `command_router_util.rs` | L315-325: `normalize_im_command_text`; L327-335: `strip_numeric_reply_suffix`; L421-440: `result_from_menu` + `result_from_menu_with_forward`; L492-510: `refresh_assistant_name_if_missing`; L512-518: `short_path_name` |

**Note**: `refresh_assistant_name_if_missing` uses `crate::service::workspace::get_global_workspace_service` and modifies `BotChatState`. Placed in `command_router_util.rs` since it's a small helper (≤20 lines body).

### §2.3 `pub(super)` visibility

| Item | Visibility | Reason |
|---|---|---|
| `BotChatState` struct fields | `pub(super)` | All siblings need to read/write `current_workspace`, `current_assistant`, `current_session_id`, `display_mode`, `paired`, etc. directly |
| `BotChatState` impl block methods | `pub(super)` for cross-sibling callers; private for siblings-internal | `set_pending` is called from `command_router_dispatch.rs::start_switch` and facade `apply_interactive_request` |
| `PendingAction` enum + variants | `pub` (re-export) | Re-exported by facade for cross-platform adapters |
| `BotCommand` enum | `pub` (re-export) | Adapters consume `parse_command` output |
| Free helpers in `command_router_util.rs` | `pub(super)` | Cross-sibling: `result_from_menu` is called by `command_router_dispatch.rs`, `command_router_session.rs`, `command_router_view.rs` |
| Free helpers in `command_router_view.rs` | `pub(super)` | Cross-sibling: `main_menu_view` is called by `command_router_dispatch.rs::switch_mode`, etc. |
| Free helpers in `command_router_dispatch.rs` | `pub(super)` | Cross-sibling: `pending_invalid` is called by `route_pending` (in same file), but the handle_chat/handle_number/handle_cancel_task fns are only called within dispatch sibling |
| Free helpers in `command_router_session.rs` | `pub(super)` | Cross-sibling: `create_session` is called by `bootstrap_im_chat_after_pairing` (in same file) and `guarded_new` (in dispatch sibling) |
| `HandleResult`, `BotInteractiveRequest`, `BotAction`, `BotActionStyle`, `ForwardRequest`, `ForwardedTurnResult` | `pub` (facade re-export) | Already cross-platform public API |
| `BotLanguage`, `current_bot_language` | `pub` (re-exported in facade) | Cross-crate test usage |
| `now_secs` | `pub(super)` (in state sibling) | Used by tests in `command_router_tests.rs` |

### §2.4 god method split pattern (R7/R12 standard)

For methods ≥ 80 lines, extract phase helpers as private async fns in the same sibling. Helpers live in same file, take `&mut BotChatState` + `s: &'static BotStrings` (same as god method signature), return `HandleResult`.

| God method | Original lines | Split into |
|---|---|---|
| `dispatch` (716-796) | ~80 | `dispatch_menu_or_welcome` + `dispatch_command_or_question` + `dispatch_pending_action` |
| `start_resume` (1122-1247) | ~125 | `prepare_resume_targets` (resolve ws_path by mode) + `fetch_resume_sessions` (load metadata + page) + `render_resume_page` (build body/items + set pending) |
| `create_session` (1383-1524) | ~145 | `prepare_session_creation` (resolve ws_path by agent_type) + `run_session_creation` (call runtime + handle error) + `finalize_session_creation` (set state + render result) |
| `route_pending` (1572-1692) | ~125 | `route_pending_workspace_or_assistant` + `route_pending_session` + `route_pending_question_or_confirm` |
| `handle_question_reply` (1830-1956) | ~125 | `parse_question_indices` + `build_question_pending_replay` + `apply_question_answer` |
| `execute_forwarded_turn` (2072-2262) | ~190 | `prepare_forwarded_turn` (dispatch send_message) + `dispatch_forwarded_turn` (event loop) + `finalize_forwarded_turn` (extract text + truncate) |

`start_switch` (864-911, ~50 lines) — keep as is (under 80 line threshold).
`handle_chat` (2006-2068, ~70 lines) — keep as is (under 80 line threshold).
`handle_command` (702-714, 12 lines entry point) — keep as is (entry point).

### §2.5 facade (command_router.rs) 内容

```rust
//! Shared command router for IM-bot connections (Telegram / Feishu / WeChat) — Round 14 facade.
//!
//! Public API surface (stable, re-exported by `bot/mod.rs`):
//!   - Types: BotChatState, BotCommand, BotAction, BotActionStyle, BotInteractiveRequest,
//!     BotInteractionHandler, BotMessageSender, BotQuestion, BotQuestionOption, BotDisplayMode,
//!     BotLanguage, HandleResult, ForwardRequest, ForwardedTurnResult, PendingAction.
//!   - Functions: parse_command, handle_command, welcome_message, complete_im_bot_pairing,
//!     execute_forwarded_turn, apply_interactive_request, current_bot_language.
//!
//! Sub-domain split:
//!   - command_router_state: BotChatState + PendingAction + TTL consts
//!   - command_router_view: 11 view builders (welcome/menu/settings/select/question)
//!   - command_router_dispatch: 16 dispatchers + sub-routines (god method splits)
//!   - command_router_session: session lifecycle (bootstrap/create/load/resume)
//!   - command_router_util: 6 small helpers
//!   - command_router_tests: 4 test mods

// ── imports ──
use super::locale::BotLanguage; // re-export
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ── sub-sibling re-exports ──
pub use super::command_router_state::{BotChatState, PendingAction, BotDisplayMode};
use super::command_router_state::{PENDING_TTL_SECS, PENDING_INVALID_LIMIT, now_secs};
pub(super) use super::command_router_view::{
    main_menu_view, settings_menu_view, need_session_view, confirm_mode_switch_view,
    workspace_selection_view, assistant_selection_view, session_selection_view,
    build_question_view, question_option_line, ready_to_chat_body, welcome_view, menu_or_welcome,
};
pub(super) use super::command_router_dispatch::{
    dispatch, switch_mode, set_verbose, start_switch, select_workspace, select_assistant,
    start_resume, select_session, new_session_for_mode, guarded_new, handle_cancel_task,
    handle_number, route_pending, pending_invalid, handle_question_reply, submit_question_answers,
    handle_chat, confirm_then_run, truncate_label, truncate_at_char_boundary,
};
pub(super) use super::command_router_session::{
    bootstrap_im_chat_after_pairing, count_workspace_sessions, load_last_dialog_pair_from_turns,
    create_session, resolve_session_agent_type,
};
pub(super) use super::command_router_util::{
    normalize_im_command_text, strip_numeric_reply_suffix, result_from_menu,
    result_from_menu_with_forward, refresh_assistant_name_if_missing, short_path_name,
};

// ── facade types (cross-platform public) ──
pub use super::locale::{current_bot_language, BotLanguage as LocaleBotLanguage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotQuestionOption { pub label: String, #[serde(default)] pub description: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotQuestion { /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotActionStyle { Primary, Default }

#[derive(Debug, Clone)]
pub struct BotAction { pub label: String, pub command: String, pub style: BotActionStyle }

impl BotAction { /* primary, secondary */ }
impl From<MenuItem> for BotAction { /* ... */ }

pub struct HandleResult { /* reply, actions, forward_to_session, menu */ }
#[derive(Debug, Clone)]
pub struct BotInteractiveRequest { /* ... */ }
pub type BotInteractionHandler = /* ... */;
pub type BotMessageSender = /* ... */;
pub struct ForwardRequest { /* ... */ }
pub struct ForwardedTurnResult { /* ... */ }

// ── BotCommand + parse_command ──
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand { /* ... */ }

pub fn parse_command(text: &str) -> BotCommand { /* uses util::normalize + strip_numeric */ }
pub fn welcome_message(language: BotLanguage) -> &'static str { /* ... */ }
pub fn apply_interactive_request(state: &mut BotChatState, req: &BotInteractiveRequest) { /* ... */ }

pub async fn handle_command(
    state: &mut BotChatState,
    cmd: BotCommand,
    images: Vec<super::super::remote_server::ImageAttachment>,
) -> HandleResult { /* ... */ }

pub async fn complete_im_bot_pairing(state: &mut BotChatState) -> HandleResult { /* ... */ }

pub async fn execute_forwarded_turn(
    forward: ForwardRequest,
    interaction_handler: Option<BotInteractionHandler>,
    message_sender: Option<BotMessageSender>,
    verbose_mode: bool,
) -> ForwardedTurnResult { /* god method (split) */ }

// ── tests (moved to command_router_tests.rs) ──
```

### §2.6 `bot/mod.rs` 改动

L16 已有:
```rust
pub use command_router::{BotChatState, ForwardRequest, ForwardedTurnResult, HandleResult};
```

R14 拆完后 L16 不变（facade 仍然 re-export 这 4 个 type）。R14 不改 `bot/mod.rs` — `pub mod command_router;` 仍然指向 facade 文件（sibling 是 facade 的子模块，不在 mod.rs 暴露）。Sibling 通过 `command_router::state::BotChatState` 等路径访问（仅同 crate 内） — 但因为 facade 重新 export 了所有 pub type，外部 caller 不受影响。

### §2.7 公开 API 不变（caller 不需要改）

| Type / fn | Original path | New path (via facade re-export) |
|---|---|---|
| `BotChatState` | `bot::command_router::BotChatState` | `bot::command_router::BotChatState` (re-exported from `command_router_state`) |
| `BotCommand` | `bot::command_router::BotCommand` | unchanged (lives in facade) |
| `BotAction` | `bot::command_router::BotAction` | unchanged (lives in facade) |
| `HandleResult` | `bot::command_router::HandleResult` | unchanged (lives in facade) |
| `ForwardRequest` | `bot::command_router::ForwardRequest` | unchanged (lives in facade) |
| `ForwardedTurnResult` | `bot::command_router::ForwardedTurnResult` | unchanged (lives in facade) |
| `parse_command` | `bot::command_router::parse_command` | unchanged (lives in facade) |
| `handle_command` | `bot::command_router::handle_command` | unchanged (lives in facade) |
| `apply_interactive_request` | `bot::command_router::apply_interactive_request` | unchanged (lives in facade) |
| `welcome_message` | `bot::command_router::welcome_message` | unchanged (lives in facade) |
| `complete_im_bot_pairing` | `bot::command_router::complete_im_bot_pairing` | unchanged (lives in facade) |
| `execute_forwarded_turn` | `bot::command_router::execute_forwarded_turn` | unchanged (lives in facade) |
| `bootstrap_im_chat_after_pairing` | `bot::command_router::bootstrap_im_chat_after_pairing` | unchanged (lives in facade) |
| `BotLanguage` | `bot::command_router::BotLanguage` (re-exported from `locale`) | unchanged |

**No caller migration needed** — every public API stays at the same `bot::command_router::NAME` path. Verified by `git grep 'use.*command_router::' -- ':!src/crates/assembly/core/src/service/remote_connect/bot/'` → all 3 files (`feishu.rs`, `telegram.rs`, `weixin.rs`) continue to compile.

---

## §3 验证命令

```bash
# Build + test
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | grep -c 'error\['
# Expected: 0

cargo test -p northhing-core --features product-full --lib 2>&1 | grep 'test result:'
# Expected: 899 passed; 0 failed; 1 ignored

cargo check --workspace 2>&1 | grep -c 'error\['
# Expected: 0 (pre-existing workspace errors at most 2: cli/agent/core_adapter.rs:121 + cli/modes/chat/run.rs:80)

cargo fmt --check -- src/crates/assembly/core/src/service/remote_connect/bot/
# Expected: 0 diff

# Iron rules
git diff main..HEAD -- src/crates/assembly/core/src/service/remote_connect/bot/command_router*.rs | grep -cE '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# Expected: 0

# File sizes
wc -l src/crates/assembly/core/src/service/remote_connect/bot/command_router*.rs
# Expected: facade <= 200, every sibling <= 800

# Cross-crate callers
git grep -n 'use.*command_router::' -- ':!src/crates/assembly/core/src/service/remote_connect/bot/'
# Expected: 3 files (feishu.rs, telegram.rs, weixin.rs) — every entry preserved by facade re-export
```

### §3.1 Preflight baseline (R6 lesson)

Before any edits, run all 4 commands to capture baseline (catches pre-existing failures that masquerade as "new"):

```bash
# 1. cargo check core
cargo check -p northhing-core --features product-full --lib 2>&1 | tail -5
# 2. cargo test core
cargo test -p northhing-core --features product-full --lib 2>&1 | grep 'test result:'
# 3. cargo check workspace
cargo check --workspace 2>&1 | grep -c 'error\['
# 4. cargo fmt check
cargo fmt --check -- src/crates/assembly/core/src/service/remote_connect/bot/ 2>&1 | wc -l
```

Save outputs to `BASELINE` file in plan workspace. All post-refactor checks must show same or better numbers.

---

## §4 拆分执行计划（Python script + manual god split）

### §4.1 Python script (R8 pattern: source from git HEAD)

Per R8 lesson: split scripts MUST source from `git show HEAD:path` (not from current SRC, which gets overwritten on first run). Single-pass script, atomic file write.

```python
# split_command_router.py — Round 14
# - Reads from git HEAD's command_router.rs (not from current file)
# - Emits 7 files atomically

import subprocess
from pathlib import Path

REPO = Path("E:/agent-project/northing-impl-round14")
TARGET = REPO / "src/crates/assembly/core/src/service/remote_connect/bot/command_router.rs"
OUT_DIR = TARGET.parent

# Read from git HEAD (per R8 lesson)
src = subprocess.check_output(
    ["git", "show", "HEAD:src/crates/assembly/core/src/service/remote_connect/bot/command_router.rs"],
    cwd=str(REPO), text=True
)
lines = src.split("\n")

# Define sibling boundaries (line ranges in original file, 1-indexed, inclusive)
# Each tuple: (filename, start_line, end_line)
SIBLINGS = [
    ("command_router_state.rs",        31,   170),   # consts + BotDisplayMode + BotChatState + PendingAction + now_secs
    ("command_router_util.rs",         315,  518),   # normalize + strip_numeric + result_from_menu + refresh_assistant_name + short_path_name
    ("command_router_view.rs",         444,  617),   # 11 view builders
    ("command_router_dispatch.rs",     716,  1740),  # dispatch + 14 sub-dispatchers + handle_chat + truncate_at_char_boundary
    ("command_router_session.rs",      624,  675),   # bootstrap_im_chat_after_pairing
    ("command_router_session.rs",      1090, 1108),  # count_workspace_sessions (overlaps; need merge)
    ("command_router_session.rs",      1279, 1335),  # load_last_dialog_pair + strip_user_message_tags + truncate_text
    ("command_router_session.rs",      1383, 1524),  # create_session
    ("command_router_session.rs",      1993, 2004),  # resolve_session_agent_type
    ("command_router_dispatch.rs",     2006, 2068),  # handle_chat
    ("command_router_dispatch.rs",     2264, 2273),  # truncate_at_char_boundary
    ("command_router.rs",              2277, 2614),  # 4 test mods
]
```

**Note**: Same-file multi-range will be merged in single output. The plan above is conceptual; actual script will use a sectioning approach: extract every function in the original file by reading its source signature (def/name line), then route to the correct sibling based on a name → file map.

### §4.2 Manual god method split (post-script)

After Python script creates skeleton files with extracted methods, manually split each god method into phase helpers. This is 1-2 hours of editing per god method (6 methods × ~10 min = 60 min total).

### §4.3 Visibility cascade fix (R8 lesson)

After script, run cargo check to identify missing `pub(super)` markers. Each error → add `pub(super)` to the relevant sibling fn. Expected 2-3 rounds.

### §4.4 mod.rs change (only mod.rs is touched in bot/)

`bot/mod.rs` L7 already has `pub mod command_router;` — this points to the facade file. After R14, sibling files like `command_router_state.rs` are sub-modules of `command_router.rs` (via `mod state;` declarations in facade). **No change to `bot/mod.rs` needed.**

---

## §5 Test path

### §5.1 Test relocation

Move 4 test mods to `command_router_tests.rs`:

| Test mod | Lines in original | Tests | Notes |
|---|---|---|---|
| `parse_command_tests` | 2278-2410 | 12 | `use super::*;` will break (no longer in facade) → change to `use crate::service::remote_connect::bot::command_router::*;` for re-exports |
| `state_tests` | 2412-2456 | 3 | Same pattern: import `BotChatState`, `PendingAction`, `now_secs` from `command_router` (re-export from state sibling) |
| `menu_tests` | 2458-2572 | 6 | Import `BotChatState`, `BotLanguage`, `BotDisplayMode`, `main_menu_view` from `command_router` |
| `handle_chat_tests` | 2574-2614 | 1 | Import `BotChatState`, `BotLanguage`, `handle_chat` from `command_router` |

### §5.2 Import path strategy

For tests, use `use crate::service::remote_connect::bot::command_router::*;` to import all facade-re-exported symbols (BotChatState, PendingAction, BotDisplayMode, BotLanguage, main_menu_view, etc.). This keeps tests cross-version resilient.

`now_secs` and `PENDING_TTL_SECS` are `pub(super)` in `command_router_state.rs` — tests in `command_router_tests.rs` cannot import them directly because tests are in a sibling of `command_router_state`, not a sub-module. Solution: tests in `command_router_tests.rs` re-implement or use `std::time::SystemTime` directly, OR we promote `now_secs` to `pub` (acceptable since it's a pure helper).

Per R13b precedent (`sftp_mkdir_all_prefixes` was promoted to `pub` for test access), R14 will:
- Promote `now_secs` to `pub` in `command_router_state.rs` (used by `state_tests` line 2422)
- Tests in `command_router_tests.rs` use `crate::service::remote_connect::bot::command_router::now_secs` (which works if `now_secs` is re-exported by facade, OR we add `pub use command_router_state::now_secs;` in facade)

---

## §6 Re-export path summary (for cross-crate caller verification)

`bot/mod.rs` re-exports (unchanged from current):
```rust
pub use command_router::{BotChatState, ForwardRequest, ForwardedTurnResult, HandleResult};
```

`command_router.rs` (facade) re-exports (new, to preserve cross-sibling calls):
```rust
pub use super::command_router_state::{BotChatState, PendingAction, BotDisplayMode, now_secs};
pub(super) use super::command_router_view::{
    main_menu_view, settings_menu_view, need_session_view, confirm_mode_switch_view,
    workspace_selection_view, assistant_selection_view, session_selection_view,
    build_question_view, question_option_line, ready_to_chat_body, welcome_view, menu_or_welcome,
};
pub(super) use super::command_router_dispatch::{
    dispatch, switch_mode, set_verbose, start_switch, select_workspace, select_assistant,
    start_resume, select_session, new_session_for_mode, guarded_new, handle_cancel_task,
    handle_number, route_pending, pending_invalid, handle_question_reply, submit_question_answers,
    handle_chat, confirm_then_run, truncate_label, truncate_at_char_boundary,
};
pub(super) use super::command_router_session::{
    bootstrap_im_chat_after_pairing, count_workspace_sessions, load_last_dialog_pair_from_turns,
    create_session, resolve_session_agent_type,
};
pub(super) use super::command_router_util::{
    normalize_im_command_text, strip_numeric_reply_suffix, result_from_menu,
    result_from_menu_with_forward, refresh_assistant_name_if_missing, short_path_name,
};
pub use super::locale::{current_bot_language, BotLanguage as LocaleBotLanguage};
```

**External callers** (verified by `git grep`):
- `feishu.rs` 16, 1551, 1583
- `telegram.rs` 14, 687
- `weixin.rs` 23, 2004

All use only public types (`BotChatState`, `BotCommand`, `BotAction`, `HandleResult`, `ForwardRequest`, `ForwardedTurnResult`, `BotInteractiveRequest`, `BotQuestion`, `PendingAction`, `BotDisplayMode`, `BotLanguage`) and public functions (`parse_command`, `apply_interactive_request`, `welcome_message`, `complete_im_bot_pairing`, `handle_command`, `execute_forwarded_turn`, `bootstrap_im_chat_after_pairing`). All of these remain `pub` in the facade.

---

## §7 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| Cross-sibling visibility 缺失 | 高 (R8 lesson) | 中 | §2.3 显式列出所有 `pub(super)` 标记; cargo check 验证 |
| 公开 API 漂移 | 低 | 高 | §2.7 + §6 列出所有 pub items; `git grep` 验证 cross-crate callers |
| God method split 引入 regression | 中 | 高 | Phase helper 顺序 + same signature + test 覆盖 |
| 156 cargo fmt pre-existing diff 干扰 | 高 (R8 lesson) | 低 | 只在 touched files 上 `cargo fmt --check`, 不全 workspace |
| Python script self-overwrite | 中 (R8) | 中 | §4.1: source from `git show HEAD:path`, 不读 SRC |
| 7-file parallel cargo check 慢 | 中 | 中 | Batch all edits, run cargo check ONCE at end (R8 lesson) |

---

## §8 验收标准

| Metric | Target |
|---|---|
| Facade lines | ≤ 200 |
| Largest sibling | ≤ 800 (no D-deviation) |
| God methods remaining ≥ 80 lines | 0 (all 6 split) |
| New `unwrap()` / `panic!` / `unreachable!` in diff | 0 |
| New `let _ = Result` in diff | 0 |
| `cargo test -p northhing-core --features product-full --lib` | 899 passed; 0 failed; 1 ignored |
| `cargo check --workspace` | 0 NEW errors (pre-existing ≤ 2: cli/agent/core_adapter.rs:121 + cli/modes/chat/run.rs:80) |
| `cargo fmt --check` on touched files | 0 diff |
| Cross-crate callers compile | 3 files (feishu.rs, telegram.rs, weixin.rs) |
| 4 test mods preserved | 12 + 3 + 6 + 1 = 22 tests |

---

## §9 Deliverables

1. **Spec doc** (this file): `docs/handoffs/2026-06-29-round14-command-router-split-spec.md` ← this file
2. **Refactor commit**: `refactor(command-router): R14 split - 1 facade + 6 sub-siblings (critical #4 god object)`
3. **Handoff doc**: `docs/handoffs/2026-06-29-round14-command-router-split-impl.md`
4. **Review guide**: `docs/handoffs/2026-06-29-round14-command-router-split-review.md`
5. **Deliverable**: `C:\Users\UmR\.mavis\plans\plan_078b2ca6\outputs\impl-r14-command-router-split\deliverable.md`

---

## §10 Time budget

| Phase | Time |
|---|---|
| Spec doc | 15 min (this file) |
| Preflight baseline | 5 min |
| Python split script + 7 files | 30 min |
| god method split (6 methods) | 60 min |
| Cargo check + test + fmt cycle | 15 min |
| Handoff + review guide | 15 min |
| Commit | 5 min |
| **Total** | **~145 min (~2.5 hours)** |

If model timeout, batch work into checkpoints at: (a) script complete + cargo check 0 errors, (b) all 6 god methods split, (c) full verification clean.
