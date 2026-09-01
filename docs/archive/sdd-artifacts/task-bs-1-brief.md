# Task BS-1 Brief — boundary self-test 锚漂移清理（T2-2a M5 挂账收口）

> 编排者本体直做（根因分析已完成的 2 文件脚本修复），本文件为事后补写的需求/约束记录，供 judge 对照。

## 背景

`node scripts/check-core-boundaries.test.mjs` 自 T2-2a 起有 1 个 pre-existing 失败
（self-test 模式：owner content anchor rule 断言）。默认模式 `check-core-boundaries.mjs` 一直绿。
挂账编号：T2-2a M5。

## 根因（编排者实测）

历史重构（tool-contracts framework.rs 拆目录、core scheduler.rs 拆目录、T2-2a' 全局注册表内联、
dialog 契约下沉 runtime-ports/agent-runtime）使 self-test.mjs 的 `requiredContentContracts`
期望清单与 required-rules.mjs 锚规则发生漂移：符号改名 / 搬走 / 删除后，期望与规则未同步。

## 需求

修复后 `node scripts/check-core-boundaries.test.mjs` 全绿且 `node scripts/check-core-boundaries.mjs` 保持绿。

## 逐条处置（每条须能在源码/rules 中取证）

| 期望条目 | 处置 | 依据 |
|---|---|---|
| `get_tool_spec_input_schema` → `tool_spec_input_schema` | 改名 + manifest.rs 规则补锚 | 现符号在 framework/manifest.rs:180 |
| `get_tool_spec_short_description` → `tool_spec_short_description` | 改名 + 补锚 | manifest.rs:194 |
| `get_tool_spec_is_readonly` → `tool_spec_is_readonly` | 改名 + 补锚 | manifest.rs:222 |
| `get_collapsed_tool_names` → `collapsed_tool_names`（framework.rs 块 :1459 处） | 改名 + registry.rs 规则补锚 | 现符号为 registry.rs:297 访问器 |
| `get_collapsed_tool_names`（core registry.rs 块 :1678 处） | **不动** | 该路径规则仍含此字面量，未失败 |
| `DialogSessionStateFact` 从 core scheduler 期望移除 | 移除 | 定义在 runtime-ports/agent_dialog.rs，rule @2920 覆盖；core 侧仅使用 |
| `BackgroundInjectionKind` 移除 | 移除 | 定义在 agent-runtime/sched_types.rs，rule @935 覆盖 |
| `DialogSteeringAction` 移除（仅 core scheduler 块 :1555；agent-runtime 块 :1074 保留） | 移除 | 同上 @935 |
| `resolve_dialog_steering_action` 移除 | 移除 | agent-runtime/sched_filter.rs，rule @1005 覆盖 |
| `resolve_background_delivery_injection` | **保留** | core scheduler rule @4967 仍覆盖 |
| `get_global_tool_registry` / `get_agent_registry` 移除 | 移除 | 符号已在 T2-2a' 删除，全仓零命中 |
| `product_assembly_plan_for_profile` 从 core product_runtime.rs 块移除 | 移除 + 迁锚 | 现属主 product-capabilities/src/lib.rs:388 |
| required-rules 新增锚 | 5 条 | manifest.rs 三条 + registry.rs 一条 + product-capabilities lib.rs 一条 |
| self-test 新增期望块 | product-capabilities/src/lib.rs（ProductCapabilityAssembly + product_assembly_plan_for_profile） | 闭环：规则被删时 self-test 会报警 |

## 约束

1. 只动 `scripts/core-boundaries/self-test.mjs` 与 `scripts/core-boundaries/rules/source/required-rules.mjs` 两个文件。
2. 任何"移除期望"必须有依据：符号已搬走且在属主处有规则锚覆盖，或符号已删除（全仓零命中）。
3. 新增锚规则的 regex 风格与相邻 pattern 一致（`\bpub fn X\b`）。
4. 验证：`node scripts/check-core-boundaries.test.mjs` pass 2/fail 0；`node scripts/check-core-boundaries.mjs` passed；`git diff --check` clean。

## 验证结果（编排者亲跑）

全部通过（输出见 commit 4b26692 message 与本任务 review package 对应的会话记录）。
另用一次性复刻扫描器（精确复刻 checker.mjs:80-90 的 regexSourceContainsContract 四分支子串语义）
全量扫 172 个期望条目：0 failures。扫描器在 C:\WINDOWS\TEMP\opencode\scan-anchor-drift.mjs（一次性，不入库）。
