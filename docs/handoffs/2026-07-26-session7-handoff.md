# Session 7 Handoff — 2026-07-26 (K4a 线：m3 复活 + T1/T4p/T23q/T23 落地，T23 R2 挂起)

> HEAD（northing）：`95db64d`（绿）。本 session 起 6 commit 未推送 + 设计稿 3 commit。
> 触发：session6 队列 #1（K2 设计稿）→ 发现前提过期 → 重定向 K4a。
> 止损：03:00 宵禁，coder-lc R2 循环被用户中止，破碎 WIP 已 stash，工作区回到绿的 95db64d。

## 1. 本 session 做了什么

### judge-m3 翻案复活（重要）
- 空返回 ×8 根因查明：**key 挂错 provider**——用户重录落在 `minimax-cn-coding-plan`（auth.json），变体指向无 key 的自建 `minimax/*`（opencode.jsonc）。
- 修复：gen-agent-variants.py REGISTRY + judge.md 回指 `minimax-cn-coding-plan/*`，重跑生成，重启后探针通过。
- 复活后实绩：T1/T4p/T23q PASS + T23 判 BLOCKED（抓到 lc 的 event_bridge 订阅泄漏，真知）——judge 首选恢复 m3。1 次 grep 误报（`agentic::` 当 `agent::`）被编排者实测推翻，后续判单无误。
- 遗留：opencode.jsonc 里自建 `minimax` provider 已无人用，待确认后可删。

### K2 → K4a 重定向（用户拍板）
- 发现：`34a2397`（07-23）已删 desktop-tauri，session5/6 队列的"K2 desktop-tauri 设计稿"是过期前提。K2b 切换（ae15d22）曾落地但宿主已亡。
- 用户拍板改做 **K4a（Slint desktop 切 facade）**；northstar §5 K2 条目已标注关闭（同 commit）。

### K4a 设计稿定稿（`docs/design/2026-07-25-k4a-desktop-facade.md` v1.0）
- judge-lc 两轮审判：FAIL（T2 误含 log.rs / T0 缺 D2 深化 / grep 花括号盲区）→ 修 → APPROVED。
- 用户五项拍板：① debug_log → `contracts/debug-log` 新微 crate ② mcp_adapter 保留接口改 facade 纯映射层 ③ w4_repro 豁免 ④ 尽可能并行 ⑤ DTO 缺字段可补（不占 P2 额度）。
- 依赖口径修订（§6）：desktop 保留 `northhing-core` Cargo 依赖（K2b 先例，composition-root 手柄在 core 内）；豁免清单 = kernel_facade 手柄 / shutdown_mcp_servers / w4_repro / actor set_actor_runtime + state.coordinator()。
- T0 核对关闭：init_core 序列完整；发现 ProviderFormDto 缺 provider_type（立 T4p）；N+1 MCP 方案定（join_all 并发）。
- §12 缺口裁定（lc R0 上报 8 缺口）：5 个加 DTO 字段、分页改客户端、actor 注入豁免、rename 不改签名（rename+get_session_metadata 读回）。

### Tickets 落地（4 commit，全绿）
| commit | 单 | coder | judge |
|---|---|---|---|
| `85fdd35` | T1 bootstrap 收编（删 agent/，init_core 塌缩，state.rs readiness） | lc | m3 PASS |
| `a4ccc5a` | T4p ProviderFormDto.provider_type + form_to_model_config + 2 测试 | s37 | m3 PASS |
| `8a9b16e` | T23q 5 DTO 字段（timestamp/parent_session_id/state/outcome_kind/name）+ 映射 + 5 测试 | s37（误记纪律没 commit，编排者代提） | m3 PASS |
| `95db64d` | T23 turn+session 数据流迁移（61 处，7 文件减到 6） | lc（R0 23 错空汇报→R1 修绿） | m3 BLOCKED→R2 挂起 |

## 2. 当前状态

- **工作区 = `95db64d`（绿）**：cargo check 0 err，desktop lib 69 测试全绿。
- **T23 差 R2 收尾**（judge-m3 验收项）：
  1. **BLOCKER**：event_bridge 订阅泄漏——DesktopEventBridge 无 Drop、无 unsubscribe_events 调用；原 core subscribe_internal 的 key 覆盖保护丢失，重复 register 累积订阅。修法：Drop 里 take subscription_id → `Handle::try_current()` 则 `handle.spawn(unsub)`（禁 block_in_place）→ 无 runtime 新建 current_thread 兜底；+1 条回归测试。
  2. **IMPORTANT**：lib.rs:14 `pub use northhing_core::kernel_facade::kernel_facade;` 是死代码（4 个模块仍走全路径）。judge 建议方案 A（模块改 `use crate::kernel_facade;`）——**但注意 lc R2 正是这么做的且 E0432**：app_state 编在 bin target（crate root=main.rs），lib.rs re-export 够不着。正确做法 = 方案 B（删 lib.rs re-export，模块维持全路径）或把 re-export 挪进 bin 侧。编排者倾向方案 B（保守）。
- **stash@{0}** = lc R2 破碎 WIP（可参考其 event_bridge Drop 草稿，或直接丢弃重做）。stash@{1} 是更早的 pre-existing "desktop onboarding WIP"，勿动。
- desktop 剩余 `northhing_core::`：T4 范围（settings/provider/skills/inspector/mcp_adapter）+ T5 范围（debug_log 8 文件）+ 豁免行。

## 3. 队列（下一 session）

| 序 | 单 | 复杂度 | 备注 |
|---|---|---|---|
| 1 | **T23 R2 收尾** | 小 | 订阅 Drop/unsubscribe + 回归测试 + lib.rs re-export 方案 B。**换新 coder session**（lc 原 session 上下文已失控），可派 s37（边界明确）或 lc 新 session；judge-m3 复验 |
| 2 | **T4** settings/skills/mcp/inspector | 中-大 | 前置 T4p 已落地。mcp_adapter 改纯映射（D2-A'）；provider_test 走 test_provider/test_provider_config（provider_type 字段已备）；inspector N+1 join_all（§11-⑤ 方案） |
| 3 | **T5** 清扫验收 | 中 | `contracts/debug-log` 新微 crate（8 文件 log_event/COMP_* 迁移）+ grep 守卫（按 §6 豁免清单）+ cargo tree 零命中 + K0 编译对比 + surfaces.md/AGENTS.md 同步 |
| 4 | K4a 收尾 → K3 ROI 闸门评估 | 决策 | T5 后对照 K0 基线 |
| 5 | memory_db.rs 拆分（841 行） | 小 | 次级队列 |
| 6 | GUI 冒烟（用户） | — | T23 改了发消息主链路，建议 T5 前找空跑一次 desktop 冒烟（发消息→流式→tool call→完成/取消 + 会话 CRUD + load-more） |

## 4. 雷区补充（本 session 新增）

- **judge/评审 grep 断言必须实跑精确模式**：m3 把 `agentic::` 误报为 `agent::` 命中（1 次，已纠）；任务书写 grep 守卫时模式要精确到不受子串干扰。
- **lc 大单失控三连**：R0 空汇报+23 错 → R1 修绿漏生命周期 → R2 循环输出。T23 级（60+ 引用簇）已是 lc 边界；续派同 session 会累积失控，返修必要时要新 session。
- **bin+lib 双 target**：desktop app_state 编在 bin target（crate root=main.rs），`crate::` re-export 在 lib.rs 里够不着 bin——re-export 方案必须先确认 target 归属。
- **s37 会误记纪律**（该 commit 时说"不提交"）→ 任务书纪律条款后加"违记=失败"。
- **coder 跑 cargo 的瞬时互踩**：本 session T23 与 T23q 并行无冲突（文件集不相交验证了有效性）。

## 5. 选派台账更新

| 模型 | 本 session 实绩 | 当前定位 |
|---|---|---|
| judge-m3 | 复活后 3 PASS + 1 BLOCKED（真知）+ 1 误报（已纠） | **judge 首选恢复** |
| judge-lc | 设计稿审判 FAIL→APPROVED 一轮半（1 次空汇报重试） | judge 备选（设计评审仍强） |
| coder-lc | T1 一次成；T23 R0 23 错空汇报/R1 修绿/R2 循环被中止 | 中大型首选但大单需切小 + 空汇报补验流程照旧 |
| coder-s37 | T4p/T23q 两连交付（1 次纪律误记） | 机械~中型首选不变 |

## 6. 记忆更新（本 session 已写）

- CORE.md：m3 恢复首选；待办 = K4a 实施（T23 R2→T4→T5）
- facts/models.md：m3 复活记录
- .learnings/ERRORS.md：lc T23 三连失误 + s37 纪律误记
- episodes/2026-07-25.md：session 7 正片追加

## 7. 一句话状态

K4a 六单落地四（T1/T4p/T23q/T23 主体），m3 复活回 judge 首选；T23 差 R2 订阅生命周期收尾（stash 有破碎草稿，建议新 session 重做），其后 T4→T5→K3 闸门。
