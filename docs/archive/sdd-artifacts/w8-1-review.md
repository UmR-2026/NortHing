# W8-1 Review Judgment — input.rs god-file 拆分

- Commit: `3337c73`（3ab2330..3337c73）
- Reviewer: judge-m3（独立验收）
- Spec 来源：`w8-1-input-split-brief.md` §Spec 6 条 + §Global Constraints
- Diff 包：`w8-1-review-package.diff`（7 文件 +894/-812；input.rs 802 → input/ 5 文件：mod 46 + bridge 11 + non_key 169 + key_actions 235 + key_popups 428）

## 判决

**Approved** — SPEC: pass · QUALITY: pass · C/I/M = 0/0/1

## SPEC 对照

| § | 条目 | 结果 | 证据 |
|---|---|---|---|
| 1 | 目录拆分（mod.rs + key_popups.rs + key_actions.rs + bridge.rs，可按实际拆分 non_key.rs） | ✓ | 5 文件齐全；mod.rs 持有原 `handle_key_event` 签名（pub(crate)），其余 helper 用 pub(super) 在 impl ChatMode 上注册；non_key.rs 收纳 `apply_exit_reason` 与 `handle_non_key_event` |
| 2 | bridge 提取 7 处（L121/135/156/181/444/504/606） | ✓ | 报告 §4.1 列 7 处；抽查 L121 (AllowOnce)、L156 (Reject)、L444 (Ctrl+C cancel)、L504 (Enter send_message)、L606 (Esc cancel) — 5/7 处亲自核对 diff，闭包体逐字符等价 |
| 3 | handle_key_event 拦截层与分支顺序 | ✓ | 7 层顺序（permission → question → global popup nav → info → command palette → 10 个 specific popup → catch-all action）完整；catch-all 在 key_actions.rs 行 1085 `_ => {}` 保留 |
| 4 | 行为零变化 + 8 参数保留 + 不下沉 dispatch trait | ✓ | `apply_exit_reason` 签名 8 参数逐位一致（non_key.rs:14）；新出现的 helper 仅 `handle_permission_prompt_key/handle_question_prompt_key/handle_popup_key/handle_key_action`（pub(super)，均为纯转发）；`ChatMode` trait dispatch 未引入 |
| 5 | manifest 处置：`god_file:src/apps/cli/src/modes/chat/input.rs` 条目删除 | ✓ | diff 中 rot-budget.json 仅 input.rs 条目消失，ceiling 数字无任何改动 |
| 6 | 验证命令 + report | ✓ | 报告 §5 含 `cargo check -p northhing-cli`（0 error）、`cargo test -p northhing-cli`（38 passed）、`node scripts/verify-rot-budget.mjs`（5 grep + 3 dir + 7 god-file）三条命令完整输出；输出与 diff 内容对得上 |

## QUALITY（独立判断）

### 复用核查
- `bridge<F, T>` helper 7 处调用，且签名通用 `where F: Future<Output = T>`，未针对任一特定调用定制（无 over-spec）—— 抽取得当
- `pub(in crate::modes::chat)` 替代原 `pub(super)`，对 `chat::run` 调用点完全等价（见 run.rs:454/479/497 仍能解析）

### 无 owner 抽象
- 新建 4 个 `impl ChatMode { pub(super) fn ... }` 块，全部方法仍挂在原 owner（ChatMode）上 —— 零新 struct / trait 引入

### 预算闸（manifest）
- rot-budget.json 中 input.rs 条目已删除；无新 ceiling 数字；无 >800 行新文件（最大 key_popups.rs 428）

### god-file 观测点
- input.rs god-file 条目消亡确认；本 diff 未引入 >800 行新文件（key_popups.rs 428 < 800）

### 公共 API 面
- `chat::input::handle_key_event` / `apply_exit_reason` / `handle_non_key_event` 三个外部符号签名逐一保留（run.rs:454/479/497 调用点零改动，diff 不含 run.rs 修改）

## 逐臂位移核实（亲自抽查 5 处）

| 原位置 | 现位置 | 等价结论 |
|---|---|---|
| L121-127 PermissionAction::AllowOnce | key_popups.rs:1206-1217 | ✓ `bridge(rt_handle, async move {...})` 与原 `block_in_place(|| rt_handle.block_on(async move {...}))` 闭包体逐字等价（仅 `tool_id`/`agent` 捕获相同，`.await` 调用一致） |
| L156-159 PermissionAction::Reject | key_popups.rs:1236-1248 | ✓ `reason_clone` 捕获与 `.await reject_tool(&tool_id, reason_clone)` 等价 |
| L444-456 Ctrl+C cancel | key_actions.rs:890-905 | ✓ 状态分支（is_processing → cancel + set_status "Cancelling..." → return；其它 → tracing + return Quit）一致 |
| L504-545 Enter send_message | key_actions.rs:917-963 | ✓ 三段嵌套（apply_command_menu_selection → is_processing /slash 或非空 → send_input + slash → send_input + 非 slash send_message）完整保留；`super::agent_display_name` → `super::super::agent_display_name`（路径深一层，符合预期） |
| L606-620 Esc cancel | key_actions.rs:1049-1065 | ✓ is_processing → cancel + status；browse_mode → scroll_to_bottom + status，两分支完整 |
| L647-709 apply_exit_reason | non_key.rs:14-46 | ✓ 8 参数签名保留；match SwitchSession / NewSession / other 三分支逐字符等价 |
| L682-801 handle_non_key_event | non_key.rs:49-168 | ✓ Mouse/Paste/Resize 三 case 完整；`Self::apply_exit_reason(...)` 在 palette Execute 与 take_pending_command 两处的 8 参数调用逐一保留 |

## 桥接 helper 等价性（抽查 3 处）

| 调用点 | 原式 | 新式 | 等价性 |
|---|---|---|---|
| key_popups.rs:1211 (AllowOnce) | `block_in_place(|| rt_handle.block_on(async move { agent.confirm_tool(&tool_id, None).await }))` | `bridge(rt_handle, async move { agent.confirm_tool(&tool_id, None).await })` | ✓ 闭包体逐字等价；`bridge` 内部 `block_in_place(\|\| rt_handle.block_on(fut))` 重建原结构；future 在 call site 构造而非 block_in_place 内构造，对 Send+'static 边界无影响（cargo check 通过即证据） |
| key_actions.rs:953 (Enter send_message) | `block_in_place(\|\| rt_handle.block_on(agent.send_message(input_clone, &agent_type)))` | `bridge(rt_handle, agent.send_message(input_clone, &agent_type))` | ✓ future 仍在 call site 立即传入 block_on；`&agent_type` 借用生命周期在 match arm scope 内仍有效 |
| key_actions.rs:1053 (Esc cancel) | `block_in_place(\|\| rt_handle.block_on(async move { agent.cancel_current_turn().await }))` | `bridge(rt_handle, async move { agent.cancel_current_turn().await })` | ✓ 同 AllowOnce 模式 |

## Findings

### Critical
（无）

### Important
（无）

### Minor

**M-1**: `scripts/rot-budget.json` 中 `god_file:src/crates/assembly/core/src/service/lsp/manager.rs` 条目（与本任务无关）在 diff 中被重新缩进（key 字段由 1 空格前缀改为 2 空格前缀）。JSON 语义等价（key 名、ceiling 836、note 字符串均未变），仅空白调整。`cargo check`/测试均不受影响，rot-budget 闸门仍读得正确数值。建议后续轮次若要纯化 manifest 变更，单独提一个空格统一 commit 一次性吃掉本仓库所有不一致缩进，避免每个 R-14 清理都附带 cosmetic noise。

## Cannot verify from diff
（无）—— 所有报告声明均能在 diff 中找到对应代码。

## plan-mandated 冲突
（无）

## 结论
input.rs god-file 拆分行为零变化、签名零变化、调用点零变化、manifest 处置正确、桥接 helper 提取合理。唯一 Minor 是无关条目的空白 cosmetic。**放行**。