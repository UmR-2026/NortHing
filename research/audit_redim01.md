## 结构腐化重审报告

> **重审时间**：2026-06-27 15:15 CST  
> **重审员**：结构腐化重审员_01  
> **基线**：`coordinator.rs` 7,215 行 / `session_manager.rs` 6,506 行（上次审核）  
> **基线报告**：`research/audit_dim01.md`（未找到本地副本，使用用户提供的口头基线）

---

### 1. 修改状态确认

#### 文件级变化（对比基线）

| 文件 | 基线行数 | 当前行数 | 变化 | 状态 |
|------|----------|----------|------|------|
| `agentic/coordination/coordinator.rs` | 7,215 | 618 | -6,597 (-91.4%) | 已大幅拆分 |
| `agentic/session/session_manager.rs` | 6,506 | 6,532 | +26 (+0.4%) | 未拆分 |
| `agentic/coordination/dialog_turn.rs` | — | 3,656 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/subagent_orchestrator.rs` | — | 1,778 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/ports.rs` | — | 1,745 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/scheduler.rs` | — | 1,526 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/a1_path.rs` | — | 560 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/state_manager.rs` | — | 135 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/coordination/turn_outcome.rs` | — | 3 | 新增 | 从 coordinator.rs 迁出 |
| `agentic/session/session_persistence.rs` | — | 1,272 | 新增 | 新增/迁移 |
| `agentic/session/session_restore.rs` | — | 757 | 新增 | 新增 |
| `agentic/session/session_evidence.rs` | — | 749 | 新增 | 新增 |
| `agentic/session/evidence_ledger.rs` | — | 540 | 新增 | 新增 |
| `agentic/session/compression/` | — | ~1,400 | 新增 | 新增模块 |
| `agentic/session/context_store.rs` | — | 59 | 新增 | 新增 |
| `agentic/session/file_read_state.rs` | — | 122 | 新增 | 新增 |
| `agentic/session/session_store_port.rs` | — | 92 | 新增 | 新增 |
| `agentic/session/turn_skill_agent_snapshot_store.rs` | — | 120 | 新增 | 新增 |
| `agentic/persistence/manager.rs` | — | 3,640 | 已存在 | 未变化（非本次改动） |

#### 新增目录

- `agentic/session/compression/` — 包含 compressor.rs (739行)、fallback/ 子目录（builder, mod, payload, render, sanitize, tests, types）
- `agentic/coordination/` 新增了 a1_path.rs, dialog_turn.rs, ports.rs, scheduler.rs, state_manager.rs, subagent_orchestrator.rs, turn_outcome.rs

---

### 2. coordinator.rs 重审

#### 当前指标

| 指标 | 数值 |
|------|------|
| 行数 | 618 |
| 函数数 | ~12 个独立 / 辅助函数 + 若干 impl 内方法 |
| impl 块 | 6 个（SubagentResult, Drop×3, SubagentExecutionScope, SubagentConcurrencyPermitGuard, SubagentTimeoutHandle, ConversationCoordinator） |
| 结构体 | 14 个（SubagentResult, SubagentExecutionRequest, WrappedUserInputPayload, BackgroundSubagentStartResult, HiddenSubagentExecutionRequest, CancelTokenGuard, ActiveSubagentExecution, SubagentExecutionScope, SubagentConcurrencyLimiter, SubagentConcurrencyPermitGuard, SubagentTimeoutHandle, ConversationCoordinator, SubagentPhase1Output, SubagentPhase2Output） |
| `use` 语句 | 42 条 |
| `pub`/`pub(crate)` 接口 | 128 个 |
| 测试代码 | 无（`#[cfg(test)]` 未出现） |

#### 拆分验证

**已拆分**。

原 `coordinator.rs` 中承载的核心逻辑已迁移至以下模块：

| 目标文件 | 行数 | 职责 |
|----------|------|------|
| `dialog_turn.rs` | 3,656 | 对话轮次核心逻辑（原 coordinator.rs 最臃肿部分） |
| `subagent_orchestrator.rs` | 1,778 | 子 Agent 编排与调度执行 |
| `ports.rs` | 1,745 | 协调端口与接口契约定义 |
| `scheduler.rs` | 1,526 | 回合调度器 |
| `a1_path.rs` | 560 | 路径解析与上下文路由 |
| `state_manager.rs` | 135 | 有限状态机管理 |
| `turn_outcome.rs` | 3 | 回合结果枚举 |

当前 `coordinator.rs` 仅保留：
- `ConversationCoordinator` 结构体定义（薄壳）
- 子 Agent 执行相关的轻量结构体（`SubagentResult`、`SubagentExecutionRequest`、`CancelTokenGuard` 等）
- 并发限制器与超时管理器
- 少量辅助工具函数（`format_background_subagent_delivery_text`、`build_subagent_session_relationship` 等）
- 显式注释：`// No impl ConversationCoordinator here; that is split across dialog_turn.rs + subagent_orchestrator.rs.`

#### God Object 评分更新

| 维度 | 上次 | 本次 | 变化 |
|------|------|------|------|
| 行数负担 | 1/10 | 7/10 | +6 |
| 职责集中度 | 1/10 | 8/10 | +7 |
| 拆分清晰度 | 1/10 | 8/10 | +7 |
| 依赖复杂度 | 3/10 | 6/10 | +3 |
| **综合评分** | **1.5/10** | **7.25/10** | **+5.75** |

> **结论**：`coordinator.rs` 从严重的 God Object 蜕变为薄层协调壳，拆分方向正确，质量良好。但注意 `dialog_turn.rs`（3,656行）本身已成为新的大型文件，需纳入下一轮重审。

---

### 3. session_manager.rs 重审

#### 当前指标

| 指标 | 数值 |
|------|------|
| 行数 | 6,532 |
| 生产代码行数 | ~4,363（排除第4363行起的测试模块） |
| 测试代码行数 | 2,170（占33%） |
| 函数数 | ~45 个（有效函数） |
| impl 块 | 3 个（`Default for SessionManagerConfig`, `SessionTitleMethod`, `SessionManager`） |
| 结构体 | 6 个（`SessionManagerConfig`, `SessionTitleMethod`, `ResolvedSessionTitle`, `SessionManager`, `SessionAutoSaveSnapshot`, `SessionCleanupCandidate`） |
| `use` 语句 | 64 条 |
| `pub`/`pub(crate)` 接口 | 96 个 |

#### 拆分验证

**未拆分**。

与基线（6,506行）相比，`session_manager.rs` 仅增加了 26 行，几乎零变化。虽然 session 目录新增了多个文件：

| 文件 | 行数 | 说明 |
|------|------|------|
| `session_persistence.rs` | 1,272 | 持久化逻辑 |
| `session_restore.rs` | 757 | 会话恢复 |
| `session_evidence.rs` | 749 | 证据收集 |
| `evidence_ledger.rs` | 540 | 证据账本 |
| `compression/compressor.rs` | 739 | 压缩器 |
| `compression/fallback/` | ~1,000 | 压缩回退机制 |

但这些新增文件**并未减少 `session_manager.rs` 的体积**，而是：
- 新增功能独立扩展；或
- 从 `session_manager.rs` 中抽出的代码量约等于新增功能代码量，导致总体积不变。

`session_manager.rs` 仍包含以下庞杂职责（根据函数名推断）：
- 会话生命周期管理（创建、恢复、清理）
- 模型选择与配置同步
- 消息分页与上下文窗口管理
- 自动保存与快照管理
- 工作区路径解析
- 过期会话清理
- 模型协调监听器
- 内部提醒剥离与元数据重建
- 对话 Agent 类型推导
- 大量集成测试（2,170行）

#### God Object 评分更新

| 维度 | 上次 | 本次 | 变化 |
|------|------|------|------|
| 行数负担 | 1/10 | 1/10 | 0 |
| 职责集中度 | 1/10 | 1/10 | 0 |
| 拆分清晰度 | 1/10 | 1/10 | 0 |
| 测试内聚度 | 2/10 | 2/10 | 0 |
| **综合评分** | **1.25/10** | **1.25/10** | **0** |

> **结论**：`session_manager.rs` 是本轮重审中**唯一未做任何拆分动作**的核心文件。6,532 行、45 个函数、单 `impl SessionManager` 块承载全部逻辑，仍是项目中最严重的 God Object，优先级最高。

---

### 4. 文件膨胀清单更新

#### 全项目 ≥ 500 行文件统计

| 阈值 | 文件数 |
|------|--------|
| ≥ 500 行 | 94 个 |
| ≥ 1,000 行 | 44 个 |
| ≥ 2,000 行 | 13 个 |
| ≥ 3,000 行 | 6 个 |
| ≥ 4,000 行 | 2 个 |
| ≥ 6,000 行 | 1 个 |

#### Top 20 文件膨胀清单（按行数降序）

| 排名 | 文件 | 行数 | 状态 |
|------|------|------|------|
| 1 | `agentic/session/session_manager.rs` | 6,532 | 未拆分 |
| 2 | `service/review_platform/mod.rs` | 4,866 | 新发现 |
| 3 | `agentic/coordination/dialog_turn.rs` | 3,656 | 拆分产物但偏大 |
| 4 | `agentic/persistence/manager.rs` | 3,640 | 持续膨胀 |
| 5 | `agentic/execution/execution_engine.rs` | 3,494 | 持续 |
| 6 | `agentic/tools/implementations/task_tool.rs` | 3,085 | 持续 |
| 7 | `service/remote_connect/bot/command_router.rs` | 2,614 | 新发现 |
| 8 | `agentic/tools/implementations/control_hub_tool.rs` | 2,557 | 持续 |
| 9 | `service/session_usage/service.rs` | 2,458 | 新发现 |
| 10 | `service/config/types.rs` | 2,403 | 新发现 |
| 11 | `agentic/tools/implementations/computer_use_actions.rs` | 2,363 | 持续 |
| 12 | `service/workspace/service.rs` | 2,339 | 新发现 |
| 13 | `agentic/tools/implementations/computer_use_tool.rs` | 2,299 | 持续 |
| 14 | `service/remote_connect/bot/weixin.rs` | 2,157 | 新发现 |
| 15 | `agentic/tools/computer_use_host.rs` | 1,811 | 持续 |
| 16 | `agentic/coordination/subagent_orchestrator.rs` | 1,778 | 拆分产物 |
| 17 | `agentic/coordination/ports.rs` | 1,745 | 拆分产物 |
| 18 | `agentic/insights/service.rs` | 1,653 | 持续 |
| 19 | `service_agent_runtime.rs` | 1,643 | 新发现 |
| 20 | `service/remote_connect/bot/feishu.rs` | 1,638 | 新发现 |

#### 重点观察

- **拆分副作用**：`coordinator.rs` 拆分后，原逻辑散落在 `dialog_turn.rs`（3,656行）、`subagent_orchestrator.rs`（1,778行）、`ports.rs`（1,745行）、`scheduler.rs`（1,526行）中。这些文件虽不再是 God Object，但各自规模仍然较大，存在二次拆分潜力。
- **service 层膨胀**：`service/` 目录下出现多个新的大文件（review_platform/mod.rs 4,866行、command_router.rs 2,614行、config/types.rs 2,403行等），说明业务逻辑正在向 service 层迁移，但缺乏拆分。

---

### 5. 新出现的结构问题

#### 5.1 新 God Object 候选

| 文件 | 行数 | 问题描述 | 风险等级 |
|------|------|----------|----------|
| `service/review_platform/mod.rs` | 4,866 | 单个 mod.rs 承载整个 review 平台逻辑，未拆分子模块 | 高 |
| `service/config/types.rs` | 2,403 | 类型定义文件超过 2,400 行，可能存在类型堆积 | 中 |
| `service/session_usage/service.rs` | 2,458 | 单 service 文件承载所有使用统计逻辑 | 中 |
| `service/workspace/service.rs` | 2,339 | workspace 服务未拆分 | 中 |
| `service/remote_connect/bot/command_router.rs` | 2,614 | 路由分发器 2,600+ 行，可能包含过多平台适配逻辑 | 中 |
| `service_agent_runtime.rs` | 1,643 | 全局运行时壳文件，可能耦合过多 | 中 |

#### 5.2 拆分产物需进一步拆分

| 文件 | 行数 | 来源 | 建议 |
|------|------|------|------|
| `dialog_turn.rs` | 3,656 | 从 coordinator.rs 迁出 | 可按对话阶段（启动、执行、收尾、错误处理）进一步拆分 |
| `subagent_orchestrator.rs` | 1,778 | 从 coordinator.rs 迁出 | 可按子 Agent 类型（后台、前台、隐藏）拆分 |
| `ports.rs` | 1,745 | 从 coordinator.rs 迁出 | 可按端口类型（输入/输出/控制）拆分 |
| `persistence/manager.rs` | 3,640 | 原有 | 可按存储后端（文件/数据库/缓存）拆分 |
| `execution/execution_engine.rs` | 3,494 | 原有 | 可按执行阶段（计划、执行、回滚）拆分 |
| `task_tool.rs` | 3,085 | 原有 | 可按任务类型拆分 |

#### 5.3 目录结构变化

新增目录（coordination 拆分产物 + session 扩展）：

```
agentic/coordination/           (原有，但新增 6 个文件)
agentic/session/compression/    (新增)
agentic/session/compression/fallback/  (新增)
```

无新增顶层 crate 或模块边界。

---

### 6. 综合评分（更新）

#### 6.1 文件级评分

| 文件 | 上次评分 | 本次评分 | 趋势 | 说明 |
|------|----------|----------|------|------|
| `coordinator.rs` | 1.5/10 | 7.25/10 | +5.75 | 成功拆分 |
| `session_manager.rs` | 1.25/10 | 1.25/10 | 0 | 零改善 |
| `dialog_turn.rs` | — | 4.5/10 | 新增 | 拆分产物，仍偏大 |
| `subagent_orchestrator.rs` | — | 5.5/10 | 新增 | 拆分产物，尚可 |
| `ports.rs` | — | 5.0/10 | 新增 | 接口定义过多，待拆分 |
| `persistence/manager.rs` | — | 3.0/10 | 持续 | 持续膨胀，未拆分 |
| `review_platform/mod.rs` | — | 2.0/10 | 新增 | 新 God Object 候选 |

#### 6.2 项目级结构健康度

| 维度 | 上次评分 | 本次评分 | 变化 |
|------|----------|----------|------|
| God Object 密度 | 2/10 | 3.5/10 | +1.5 |
| 模块拆分进度 | 2/10 | 4/10 | +2 |
| 新增膨胀控制 | 3/10 | 2.5/10 | -0.5（service 层出现新大文件） |
| 测试与生产分离 | 2/10 | 2.5/10 | +0.5（session_manager 测试已隔离） |
| 目录边界清晰度 | 3/10 | 4/10 | +1 |
| **综合健康度** | **2.4/10** | **3.3/10** | **+0.9** |

#### 6.3 关键结论

1. **单点突破成功**：`coordinator.rs` 的拆分是本轮最大成果，从 7,215 行降至 618 行，拆分策略清晰、职责边界合理。
2. **核心瓶颈未解**：`session_manager.rs` 完全未动，仍是项目最大单一文件（6,532 行），优先级为 **P0**。
3. **拆分产物偏大**：`dialog_turn.rs`（3,656行）、`ports.rs`（1,745行）、`subagent_orchestrator.rs`（1,778行）虽然不再是 God Object，但仍有二次拆分空间。
4. **service 层膨胀**：`service/` 目录下出现多个新的大型文件（review_platform/mod.rs 4,866行、config/types.rs 2,403行等），需要关注业务层是否正在积累新的结构债务。
5. **整体改善有限**：仅一个核心文件被拆分，项目级健康度从 2.4/10 小幅提升到 3.3/10，结构腐化仍属严重。

#### 6.4 建议优先级

| 优先级 | 行动项 | 目标文件 |
|--------|--------|----------|
| P0 | 拆分 `session_manager.rs` | `session_manager.rs` -> 生命周期/恢复/持久化/清理 4+ 模块 |
| P1 | 拆分 `review_platform/mod.rs` | 拆分为 service/review_platform/ 子目录 |
| P1 | 拆分 `dialog_turn.rs` | 按对话阶段拆分 |
| P2 | 拆分 `persistence/manager.rs` | 按存储后端拆分 |
| P2 | 拆分 `service/config/types.rs` | 按配置域拆分 |
| P3 | 持续监控 `service/` 层膨胀 | 建立 1,000 行告警阈值 |
