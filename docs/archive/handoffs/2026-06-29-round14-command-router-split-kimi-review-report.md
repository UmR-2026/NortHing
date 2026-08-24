# Round 14 `command_router.rs` Split — Review Report (Kimi)

> **Reviewer**: Kimi
> **Date**: 2026-06-30
> **Commit**: `92faf19` (R14 merge) + `ca3bc2f` (QClaw review report)
> **Base**: `1f19784` (R13b review)
> **Verdict**: ✅ **APPROVE 8.6/10** with minor observations

---

## 1. Answers to review-guide questions

| # | Question | Answer |
|---|----------|--------|
| 1 | Is 832-line `command_router_dispatch.rs` acceptable as a one-round D-deviation— | **可接受** (4% over 800 cap, within QClaw 10% tolerance). R15 不必预提取 `start_resume` |
| 2 | Is the 22-test count in `command_router_tests.rs` sufficient coverage— | **足够** — 覆盖 parse_command (12) + state (3) + menu (6) + handle_chat (1) |
| 3 | Should `route_pending` (122 lines) be split into per-`PendingAction` dispatchers— | **R15+ P2 候选**，不阻塞 R14 merge |
| 4 | Should `execute_forwarded_turn` re-export be kept, or update IM adapters to import directly— | **保持 re-export** — 零迁移成本，结构稳定后再考虑迁移 |
| 5 | Any concern about the corrupted pattern (镛— → …") in git history— | **无担忧** — HEAD 清洁，历史中间 commit 有修复是正常协作 |

---

## 2. Verdict

**APPROVE 8.6/10**

R14 是一次高质量的 god-object 拆分。Bot command router 2614 行拆为 306 行 facade + 8 个职责单一的 sub-sibling，结构清晰，pub(super) 模式一致，外部调用方 0 迁移成本。

Mavis take-over 处理得当：
- 5-pass fix (visibility, GBK encoding, mod.rs corruption, questions extraction, facade import reconciliation) — 每一步都对应明确的 root cause
- Chinese byte 修复覆盖 6 个文件，0 mojibake 残留
- 22 个新测试全部通过

唯一小遗憾是 dispatch.rs 仍超 800 32 行（4%），但 R15 已经计划提取 `start_resume`，这是可控的。

---

## 3. Minor observations (non-blocking)

1. **dispatch.rs:791 处的 `unwrap()`**: `let session_id = state.current_session_id.clone().unwrap();` — 虽然上面 L787 已经 early return 保证了 unwrap 不会 panic，但建议加一行注释说明 invariant，便于 reviewer 不会误判为新引入的 iron rule violation。
 - **Status**: pre-existing on main HEAD 1f19784 (R14 之前就存在). QClaw 也独立提出.

2. **Facade `command_router.rs:54` 未使用的 imports**: `use super::locale::{fmt_count, strings_for, BotStrings};` 中 `BotStrings` 和 `fmt_count` 在 facade 内未使用（strings_for 在 create_handle / complete_im_bot_pairing 中用到了）。
 - **Fix**: 改成 `use super::locale::strings_for;` 或保持现有但加 `#[allow(unused_imports)]`。

3. **`command_router_session.rs:18` 未使用 `BotStrings`**: 类似情况，session module 导入了但未直接用。
 - **Fix**: 改为 `use super::locale::{current_bot_language, strings_for};`

4. **`command_router_util.rs:16` 未使用 `BotStrings`**: util module 同样有未使用 import。
 - **Fix**: 删掉未使用行。

5. **R15+ P2 (延期处理)**: `route_pending` (122 lines) 拆分为 per-PendingAction dispatchers，可以改善 dispatch.rs 的 dispatch 表清晰度。

6. **未来 subagent 拆分脚本规范**: Python 拆分脚本（如 R14 worker 的 `split_command_router.py`）必须强制 `encoding='utf-8'`，避免 GBK-as-UTF-8 乱码。已写入 MEMORY 作为 standing rule。

---

## 4. Approval conditions

无 blocking condition。R14 已经合并到 main (`92faf19`)，可以作为 R15 的 baseline 继续推进。

## 5. R15 candidate priority (informational)

1. `command_router_dispatch.rs` start_resume 提取 (832 → ~705 lines) — QClaw 明确要求
2. `control_hub_tool.rs` 2526 (Kimi P1 critical list)
3. `acp/client/manager.rs` 2519
4. `exec.rs` 2488
5. `runtime-ports/src/lib.rs` 2460
