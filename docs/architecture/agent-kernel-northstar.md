# Agent Kernel 北极星架构（主 agent 为心，万物为模块）

> 状态：草案 v0.3.1（2026-07-20；v0.2 过 judge-m3 评审，v0.3 吸收用户五条架构质询，v0.3.1 过 judge-lc 复审吸收 7 项修正；评审全文 `.opencode/sdd/kernel/northstar-review.md` / `northstar-review-v3.md`）。立项形态：北极星文档 + 渐进迁移（用户拍板）。
> 第一驱动痛点：**编译/构建慢**——迁移顺序以削减重编扇出为第一优先级，其次才是认知解耦。

## 1. 目标与原则

把"主 agent 运行内核"收敛为一个**小而稳定**的 crate 集合（kernel），其它一切——UI 宿主、provider 适配、工具、OS 服务、持久化、subagent 运行时——全部作为**可插拔模块**挂在 kernel 的端口上。

- **P1 端口隔离**：kernel 只面向 `contracts`/`runtime-ports` 编程；模块只依赖端口 crate，互不依赖。
- **P2 薄 facade（量化硬指标）**：宿主只见一个极薄 facade（命令面 + 事件订阅面），永不见 kernel 内部类型。**"薄"的数字约束**：K1 冻结时统计宿主实际调用面方法数 N（机械清单得出，写入本节），此后 facade 公开方法数 **≤ ⌈N × 1.2⌉**；任何超出 20% 余量的新增必须先在本文档记录评审结论（谁提出、为什么现有方法覆盖不了、能否合并进现有面）才允许加。facade 代码量同步设上限：**≤ 1500 行**（DTO + trait + 错误类型；测量：`tokei -t=Rust contracts/kernel-api/src`，排除注释与 `#[cfg(test)]` 模块），超线即停步审视是不是把业务逻辑偷渡进了 facade。

  **P2 评审记录（2026-07-20，K1 冻结）**：N = 44 → 上限 ⌈44×1.2⌉ = 53，K1 实发 53（合规）。F2-conditional 的 2 个占位方法（`start_mcp_server(id)`/`stop_mcp_server(id)`）以**注释形态**留在 trait 块——裁决（judge-m3）：P2 触发条件是"新增"不是"注释占位"，K1 零超额；占位不进公开面、不计数。三条闸门：① K1 验收的方法数按 AST 统计且排除注释行，并加 grep 守卫拒绝非注释的 start/stop_mcp_server 出现；② F2 实施 ticket 提交这两个方法前必须重跑本 P2 评审（带三要素：提出人/覆盖缺口分析/合并可行性——裁定：启停是运行时生命周期动作，无法合并进 get_mcp_status/delete_mcp_server）；③ 未经复审不得解注释提交。
- **P3 assembly 只做接线**：`assembly/core` 从"什么都装的 god crate"退化为 composition root——只负责 new 出各模块并注册进 kernel，不含业务逻辑。
- **P4 编译扇出优先**：任何迁移步骤的验收都带编译指标——目标：改一个 leaf 模块（工具/provider/UI）时，重编范围不超出该模块及其直接反向依赖，**不触发 kernel 与全部宿主重编**。
- **P5 行为不变**：这是结构迁移不是功能重写；每一步 `cargo check --workspace` + 相关测试全绿，e2e chat 不回归。

## 2. 现状诊断（2026-07-20 实测）

**扇出问题**：
- god crate `northhing-core`（assembly/core）被 **5 个宿主/接口 crate 直接依赖**：`apps/desktop`、`apps/desktop-tauri`、`apps/cli`、`cli-internal`（实现层为 stub，扇出贡献可忽略）、`interfaces/acp`。core 任何一行改动 → 宿主全量重编 → 宿主体量巨大（desktop 含 Slint 生成码），日常迭代被这个扇出拖死。
- 宿主深入 core 内部模块：`desktop` 单文件最高 27 处 `northhing_core::` 引用（`callbacks_lifecycle.rs`）；`desktop-tauri` 直接 import `agentic::` / `service::` / `infrastructure::` / `util::` 四大内部域。没有稳定面，任何 core 内部重构都外溢到宿主。
- 好的一面：leaf 层已经干净——`ai-adapters`/`tool-contracts`/`services-core` 等只依赖 `core-types`（contracts），不依赖 god core。端口 crate（`runtime-ports`、`events`、`core-types`）已存在。**问题集中在"宿主↔core"这一条边上**。

**结论**：编译痛点的 80% 在"5 宿主直连 god core"。第一刀不需要拆 core 内部，只需要在宿主和 core 之间插一个薄 facade——core 内部重构立即停止外溢，宿主重编触发面大幅收窄。

## 3. 目标架构

```
                 ┌────────────────────────────────────────┐
   UI hosts ────►│            kernel facade               │  (新 crate: contracts/kernel-api)
  (desktop/tauri │  命令面: send/stop/sessions/settings…   │   极薄, 只含 DTO + trait + 错误类型
   /cli/acp) ◄───│  事件面: subscribe(EventStream)        │
                 └──────────────────┬─────────────────────┘
                                    │ 实现
                 ┌──────────────────┴─────────────────────┐
                 │    AGENT KERNEL = execution/agent-     │  (扩展现有 agent-runtime 为 kernel,
                 │    runtime（扩展，不新建 crate）         │   其 scheduler/runtime/session_control/
                 │  session/turn 执行 · DialogScheduler   │   events 等模块已是目标位置;
                 │  事件总线 · 能力注册表                  │   core 侧对应实现下沉并入)
                 └──┬───────┬───────┬───────┬────────────┘
              端口▼       端口▼     端口▼   端口▼
          ┌────────┐ ┌─────────┐ ┌───────┐ ┌──────────┐
          │providers│ │ tools   │ │services│ │persistence│
          │ai-adapt.│ │contracts│ │mcp/git/│ │sessions   │
          │         │ │→execution│ │ssh/fs │ │transcript │
          └────────┘ └─────────┘ └───────┘ └──────────┘
                 ▲
          assembly/core = 纯 composition root（new + 注册，无业务）
```

**端口清单（初始面，K1 定稿时逐条评审）**：
- `KernelCommands`（宿主→kernel）：session CRUD、`send_message`、`stop_turn`、settings 读写代理（providers/workspaces/skills/mcp）、`test_provider*`。
- `KernelEvents`（kernel→宿主）：`TextChunk`、`TurnState(started/completed/failed/cancelled, duration_ms)`、`ToolCall(started/completed, name, summary)`（F1.5 需要的那条）、`Banner/Error`。
- `ProviderPort`：chat/stream 请求（现有 ai-adapters 已近似满足，收编为端口注册）。
- `ToolPort`：现有 tool-contracts/tool-execution，经 tool-provider-groups 注册（已接近目标，不动）。边界补充：`ToolRuntimePort` / `ToolProviderGroupPort` 的归属在 K1 逐条评审；`ToolUseContext`（工具执行上下文）必须抽成端口类型，不许让宿主/工具反向引用 kernel 具体类型。
- 调度端口（已在 `assembly/core/src/agentic/coordination/scheduler/scheduler_turn/` 成形，K1 收编）：`AgentDialogTurnPort`、`AgentTurnCancellationPort`、`AgentLifecycleDeliveryPort`。
- `ServicePort`：mcp/git/ssh/fs/terminal（services 层保持，kernel 经 runtime-ports 引用）。
- `PersistencePort`：session/transcript 读写（从 services-core 会话持久化抽出窄接口）。
- `Capabilities`/`CapabilitySet`：能力集是端口还是接线细节，K1 评审时显式决策（默认：接线细节，留 assembly，不进 facade）。

## 4. 编译收益模型

| 改动点 | 现状重编范围 | 目标重编范围 |
|---|---|---|
| core 内部（turn/scheduler 细节） | 5 宿主 + 全部下游 | 仅 kernel + assembly（宿主只链 facade 的 rlib） |
| 某工具/provider | 基本已局部 | 不变（保持 leaf 干净） |
| facade DTO | —（不存在） | 全宿主（刻意：契约改动本就应全量） |
| UI 前端 | desktop/desktop-tauri 自身 | 不变 |

关键机制：facade crate **薄且稳定**（只随契约变），宿主依赖它而非 kernel；kernel 内部成为高频改动区但不再外溢。**cargo 机制约束（评审实证，违反则收益归零）**：① `kernel-api` 不得声明 `northhing-core` 的 `product-full` feature，只取 DTO/error 子集——否则 feature unification 把 rmcp/git2/reqwest 等重依赖重新传染给所有宿主；K1 验收含 `cargo tree -p kernel-api` 对 `(rmcp|git2|axum|tower-http|reqwest)` 零命中。② facade 不得 re-export kernel 内部泛型/derive 宏类型（泛型单态化与 derive 代码在宿主侧生成，会把 kernel 内部类型拉进宿主 rlib metadata）。③ desktop-tauri 在根 workspace `exclude` 里，所有涉及它的验收命令必须用 `--manifest-path` 单独跑（含 K2 验收补一条 `cargo tree --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml -p northhing-kernel-api` 零命中——独立 Cargo.lock 解析可能不同于根 workspace）。④ **持续守卫（外部评审 2026-07-20）**：①② 的点态验收不够——feature unification 是全局 workspace 属性，后续任何 PR 给 contracts crate 加 optional feature 都会静默传染。CI 必须加 per-PR 守卫 job：`cargo tree -p northhing-kernel-api` 对 `(rmcp|git2|axum|tower-http|reqwest)` + `northhing-core` 零命中（成本 <5s），把机制 ① 从点态验收升级为持续不变量。

## 5. 迁移路线（tracer bullets，每步独立可验收）

- **K0 度量基线 + K3 可行性探针**：
  - 度量：`cargo build --timings` + `cargo check` 冷/热快照存档（`.opencode/sdd/kernel/`）；单次冷编译 wall-time（`Measure-Command`）、`cargo tree` 反向依赖基线（`cargo tree -i northhing-core` 等）、desktop-tauri 单独 timing（独立 workspace 必须单独测）。后续每步对比。**K2 验收目标值**：改一个 leaf tool crate 后 `cargo check --workspace` 增量 < 30s（校准方法：目标值 = min(30s, K0 基线 × 0.5)；若 K0 基线已 < 20s 则维持现状，目标改为"不劣化"）。
  - **K3 探针（不等 K2 做完才验证 K3 可行性）**：选 core 内一个最小、依赖最少的候选模块（首选候选：turn transcript 或 session 持久化的某个自包含片段；选定标准 = `codegraph impact` 显示扇出 ≤ 5 个符号）试搬到 `execution/agent-runtime`，记录：真实循环依赖数量、被迫带走的符号数、behavior 等价测试成本。产出 go/no-go 证据表。**若探针显示内部缠绕到搬迁成本 > 10 人天，K3 直接降级（见 K3 ROI 闸门），后续步骤不再假设 kernel 会下沉。**
- **K1 facade 定义（不改行为）**：新建 `contracts/kernel-api` crate。facade 面的输入 = **两份清单的并集**：
  1. 宿主现有调用机械清单（desktop-tauri commands.rs + desktop callbacks_* + cli + acp 的全部 `northhing_core::` 引用，grep 无遗漏）；
  2. **F 线已立项的未来面**（防止"F 线每加一个功能就改 facade → 全宿主重编"的新扇出）：F1.5 `chat-tool` 事件 + `TurnState.duration_ms`、F2 settings CRUD（providers/workspaces/skills/mcp + `test_provider(id)` / `test_provider_config(form)`）、onboarding 状态查询、Inspector 数据面（model 显示名、MCP/skills 状态）、`core_health` 健康感知、F3 panels 配置读取、「产物」面板的数据面（若 F2 评审保留）。
  assembly/core 实现该 facade（纯转发）。**Schema 冻结顺序**：先冻结 `KernelEvents::ToolCall` 与 F1.5 `chat-tool` payload 的 TypeScript 类型，字段逐一对齐（`session_id/turn_id/call_id/phase/name/summary/detail?`），冻结后 K2/F1.5 才动手；冻结同时统计 N 并写入 P2。验证：workspace check 绿 + 宿主未动 + §4 的 cargo tree 零命中。
- **K2 desktop-tauri 切 facade**：新宿主先行（体量小、在建中、F1.5/F2 正要加事件面）。src-tauri 只 import `kernel-api`。验证：GUI 冒烟（发消息→流式→完成）+ `cargo check --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml` 0 err + `cargo check -p northhing` 绿 + K0 目标值实测对比。**顺带把 F1.5 的 `chat-tool` 事件面定义进 facade**，前端计划与 kernel 迁移在此合流。
  **回退路径（K2 验收失败时）**：desktop-tauri 恢复直连 `northhing-core` 旧路径（git revert 即可，旧路径在整个 K 线期间不删除），`kernel-api` crate 保留但标记 dormant；K 线暂停并复盘 facade 设计，**F 线不被阻塞**（继续在旧路径上迭代）。**旧路径维护责任**：F 线在旧路径上的每次变更，K2 owner 负责同步进 kernel-api 分支，保证 revert 始终可行。facade 作为可选层并行存在的过渡期上限 = 到 K4 启动前；过渡期结束要么 K2 重做过验收，要么 K 线整体关闭并写 postmortem。
- **K3 ROI 闸门（K2 验收后、K3 启动前的强制决策点）**：对照 K0 目标值实测——**若"改 leaf tool 后增量 check < 30s"在 K2 已达成，K3 降级为"有空再做"**（其编译收益大头已在 K1/K2 拿到，剩下的是认知解耦收益，不值得冒最高风险强推）；若未达成，用 K0 探针的 go/no-go 证据决定 K3 是否启动。**探针证据时效**：闸门决策前重跑一次 K0 探针（同一模块），结果与 K0 证据偏差 > 50% 则探针证据失效，需重新评估。**高扇出探针前置（双 judge 建议，2026-07-20 采纳）**：K1/K2 期间补做一次代表性 HIGH-fan-out 探针（DialogScheduler 相邻模块），在 facade 尚可回退时提前暴露真实 cycles/拖拽/测试成本——K0 的 fan-out=0 探针只是成本下限（floor），不构成 K3 主体证明。闸门结论写入本文档。
- **K3 kernel 下沉（先书面设计后动手；仅闸门放行才启动）**：目标 = **扩展现有 `execution/agent-runtime` 为 kernel**（其 scheduler/runtime/session_control/events 模块已是目标位置），core 侧 turn 执行/DialogScheduler/事件总线下沉并入。**启动前置条件**：先产出 K3 owner design 文档（port 接口清单 + 与 `assembly/core/AGENTS.md`、`execution/agent-runtime/AGENTS.md` 边界冲突的书面 reconciliation + behavior equivalence tests 清单），judge 评审通过后才动代码；**不接受"先留 re-export 桥"的捷径**（违反 assembly/core 分解纪律）。验证：core lib 测试全绿 + `w4_repro --mode=dual` 完整 turn（W4 运行时纪律）+ `cargo check -p northhing-server -p northhing-relay-core` 回归（间接宿主）。
- **K4 desktop + cli + acp 切 facade（不依赖 K3，K2 验收后即可启动）**：**启动论证**——K2 证明的是 facade 面完整性（因 K1 输入已含 F2/F3 未来面），不是"所有宿主耦合模式相同"；desktop（27 处/文件）、cli（100+ 处）、acp（61 处）耦合模式差异大，若 K2 验收中发现 facade 面缺失需补充，K4 启动前必须重新评审 facade 面完整性。拆两个子步分别验收——**K4a desktop**（Slint callbacks_* 逐文件迁移，27 处/文件重灾区）；**K4b cli + acp**（cli 100+ 处 deep coupling、acp 61 处，分别验收）。验证：desktop check + CLI exec e2e + acp 测试。
- **K5 收尾验收（范围随 K3 闸门缩放）**：若 K3 完成 = assembly 瘦身成纯 composition root；若 K3 降级 = 收尾仅含"facade 不变量入 AGENTS.md + 编译对比报告"。更新根 AGENTS.md 骨架不变量（加"宿主只经 kernel-api facade"一条——涉及 GlobalConfig 等既有不变量表述变化，走 flag flip + integration test 流程，**AGENTS.md 更新与该 flag flip 必须在同一 commit**）。产出 K0 vs 终态编译对比报告。

依赖关系：K0→K1→K2→**K3 ROI 闸门**；K4 不依赖 K3，K2 验收后即可与闸门并行启动；K3 仅闸门放行才启动；K5 范围随闸门缩放。与前端计划关系：**K1/K2 插队在 F1.5 之前**——F1.5 的工具事件面直接长在 facade 上，避免在旧面上盖新楼；K2 回退路径保证 K 线翻车不阻塞 F 线。

## 6. 非目标

- 不重写 turn 执行逻辑、不动 agent loop 行为（P5）。
- 不动 Slint desktop 的 UI 代码本身（K4 只改 import/调用面）。
- 不做 plugin 动态加载（.so/wasm）——"模块化"指编译期 crate 解耦，不是运行期插件系统。
- 不动 frozen 面（server/relay/MiniApp/SDLC）。

## 7. 风险

- **facade 面设计反复**（最大风险）：K1 的面若漏了宿主真实需求，K2/K4 会反复返工；且 facade 每漏一个 F 线未来需求 = 一次全宿主重编扇出。缓解：K1 输入 = 现有调用机械清单 ∪ F2/F3 已立项未来面（§5 K1 两份清单）；P2 量化约束（N+20%、1500 行）挡膨胀；面定稿先过 judge 评审再实施。K2 验收失败有明确回退路径（§5 K2），回退成本 = 一次 git revert。
- **K3 高投入走不通**：core 内部可能缠到搬不动，若到 K2 才发现则计划收益大打折扣。缓解：K0 阶段即做 K3 探针（试搬最小模块，go/no-go 证据表）+ K3 ROI 闸门（编译目标已在 K2 达成就降级 K3，不硬推）。
- **kernel 下沉牵出隐藏依赖**：core 内部模块互缠（god object 战役的老问题）。缓解：K3 的 owner design 前置评审 + codegraph impact 逐符号确认扇出 + behavior equivalence tests（见 §5 K3 前置条件，re-export 桥捷径已封）。
- **与 F 线并行冲突**：desktop-tauri 同时被 F1.5/F2 和 K2 改；若 facade 没覆盖 F2/F3 需求，会出现"F 线每加功能先改 facade → 全宿主重编"的新扇出。缓解：K1 输入强制含 F2/F3 未来面（§5 K1 清单 2）；K2 与 F1.5 合并为同一批 ticket、一套验收；K2 回退路径保证 K 线出问题时 F 线照走。
