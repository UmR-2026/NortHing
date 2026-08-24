# Review Brief — P3a 死代码删除：ensure_assistant_bootstrap 死簇（−245/+17）

## 审查对象

- Diff: `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-p3a-deadbootstrap\diff.patch`（13 文件，+17/−245）
- BASE = `ff55a9b`；范围 = 全部 staged 改动。
- 仓库：`E:\agent-project\NortHing`。工作区有大量行尾幻影（`git diff` 为空）与 5 个已知在途文件（progress.md / memory / model-capability-notes / kernel-api memory.rs/turn.rs），**均不在审查范围**。

## 裁决背景（已确认，不必重审）

`ensure_assistant_bootstrap` 是 snapshot 先天孤儿（全历史唯一触碰为 2026-07-12 快照导入，从未有调用方），裁决"删"而非"接线"——接线等于凭空激活带 `skip_tool_confirmation(true)` 的自动系统 turn，属产品决策。附带收益：终审记录的"第四处未注解 skip 豁免"随文件消失。

## 被审删除清单（交付方自述，逐项核零调用方）

1. `dialog_turn/coordinator_bootstrap.rs` 整文件删除（137 行）
2. `dialog_turn/turn.rs`：删 `kickoff_query`、`system_reminder`、`is_chinese_locale` 三孤儿助手（声称唯一用户是被删文件）
3. `coordinator.rs`：删三个 `AssistantBootstrap*` 枚举 + `ASSISTANT_BOOTSTRAP_AGENT_TYPE` 常量 + 死 import（−32 行）
4. 6 文件空挂 import 清理：compaction / session / thread_goal / workspace / so_handlers / coordinator（声称只 use 不调的预存债）
5. `service/bootstrap/bootstrap_impl.rs`：删 `is_workspace_persona_pending`（`is_workspace_bootstrap_pending`）与 `reset_workspace_persona_files_to_default`（声称后者零调用方死 pub API）+ `bootstrap/mod.rs` 两处 re-export
6. `service/mod.rs`：−1 行（疑似 re-export 级联）
7. handoff P3a 记录 + 遗留段翻牌

## 执行中修正事件（重点复核区）

交付方自述：初判 `ensure_workspace_persona_files_for_prompt` 为死代码删除后编译报错——活函数 `build_workspace_persona_prompt`（prompt_builder ×3 消费）内部调用它做 persona 桩回填，已恢复函数及其测试。**盲区根因：消费方扫描把 bootstrap 模块自身排除在 grep 外**。

复核要求：
- 确认恢复后的 `ensure_workspace_persona_files_for_prompt` 与删除前逐字节等价（`git show ff55a9b:src/crates/assembly/core/src/service/bootstrap/bootstrap_impl.rs` 对比工作区版本）。
- 确认其测试也已恢复且等价。
- 举一反三：对删除清单 1–6 的**每个符号**，亲自全仓 grep（含同模块内部、测试、re-export 链、字符串引用如 `match` 分支名/序列化），确认零残留零调用方。不接受交付方 grep 结论转述。

## Constraints

1. **纯删除语义**：本批不得有任何行为变更（除删除死代码本身）；活着的代码路径一行不许动。
2. **skip_tool_confirmation 豁免面**：删除后全仓应只剩三处已注解豁免（probe-1 注解过），grep `skip_tool_confirmation` 核实第四处确实随文件消失、且无新增。
3. **rot-budget 只降不升**：handoff 声称文件数 1365→1364，如 rot-budget.json 有改动须为下调（本批 diff 未见该文件，确认即可）。
4. **日志英文-only**；家规分层边界（纯删除通常不触碰，确认无连带）。
5. **远程兼容/骨干不变量**：coordinator 属 agentic 协调面，确认删除不涉及骨干不变量清单（desktop 包名/配置 SSOT/UI 线程纪律/shell guard/slug/installer/v0.1.0 面）。

## 已声称验证（report 即证据，不重跑）

core 编译 0 error；`service::bootstrap` 6/6；`coordination` 52/52；cli + desktop 编译门过；fmt + rot-budget 绿；已删符号全仓 grep 零残留。复算命令已入 handoff。

## 输出要求

写到 `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-p3a-deadbootstrap\report.md`：
- 双判决：spec 合规（删除清单 1–7 + 修正事件，逐条 PASS/FAIL 带 grep 命令与命中证据）+ 代码质量。
- findings 分级 Critical/Important/Minor，文件:行号 + 证据。
- Cannot-verify-from-diff 单独列清单。
- 最后一行 `APPROVE` / `REQUEST_CHANGES`。
