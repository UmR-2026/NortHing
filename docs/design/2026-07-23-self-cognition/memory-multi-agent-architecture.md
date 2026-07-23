# Memory 多 Agent 架构设计（C4 正篇）

## 概述

northing 的 agent 记忆系统采用多 agent 架构：主 agent 不直接管理记忆，由 memory-judge 编排、memory-writer 执行，全部异步，用户无感。

## 核心哲学

- core agent 的成长 + subagent 调度是应用中心，其它功能外部解耦
- agent 是成长主体，记忆是它成长的土壤
- 分权：agent 提案，judge 决策，writer 执行

## Agent 架构

```
主 agent（前台，用户交互）
  │
  │ 实时轨：用户显式"记住X" → 立即异步发 judge
  │ 后台轨：每 N 轮 / session 结束 → 批量发 judge
  │
  ▼
memory-judge（异步，后台）
  ├── 读 judge-mom（时机偏好 + 内容偏好）
  ├── 评估：该不该写 / 谁来写 / 写到哪
  ├── 决策：现在是好的整理时机吗？（参考用户偏好）
  ├── 路由：→ writer 执行
  ├── 自学习：更新 judge-mom
  └── 定期 dream：全量扫描，合并/降权/删除
  │
  ▼
memory-writer（异步，judge 指派）
  └── 执行写入（facts/keyword_weights/prune）
```

## 双轨触发

| 轨道 | 触发条件 | 行为 |
|---|---|---|
| 实时轨 | 用户显式说"记住/以后/总是" | 立即异步发 judge，高优先 |
| 后台轨 | 每 N 轮 / session 结束 / context 阈值 | 批量发 judge，routine |
| 定期轨 | dream（天级）/ distill（周级） | judge 自主触发 |

## Judge 职责

1. **评估**：记忆候选该不该入库
2. **路由**：决定由谁写入、写到哪个表
3. **语义加权**：提取关键词，关联已有词，boost 权重
4. **时机自学习**：根据用户反馈优化后台整理时机
5. **Dream**：定期全量扫描，合并关联/降权过时/删除无用
6. **冷启动**：前几轮走固定流程，积累数据后再优化

### Judge 路由决策

| 记忆类型 | 判定 | 路由 |
|---|---|---|
| session 状态 | approve + routine | → writer 直接写 |
| 用户偏好 fact | approve + growth | → facts 表 + keyword_weights |
| 重复提及 | approve + boost | → keyword_weights 加权（不新增 fact） |
| 过时/无用 | reject + prune | → 删除/降权 |
| 用户说"别记" | reject + ignore | → 标记忽略 |

## 存储隔离

```
config_dir()/northhing/
├── identity.md              ← core agent 自我认知（agent 自主管理）
├── memory/
│   └── memory.db            ← SQLite（FTS5 + keyword_weights）
│       ├── facts            ← core agent 记忆
│       ├── facts_fts        ← FTS5 索引
│       └── keyword_weights  ← 关键词权重
│
├── judge/                   ← judge 隔离区（core agent 不可读）
│   ├── mom.db               ← judge 专用记忆
│   │   ├── judge_mom        ← 用户偏好/时机
│   │   └── judge_mom_fts    ← FTS5 索引
│   └── audit/               ← 审计日志
│
└── episodes/                ← 日记（UI 功能，给人类看，agent 不读）
```

### 隔离规则

- core agent 的 prompt builder 永远不注入 judge/ 下的内容
- judge 的 prompt builder 永远不注入 identity.md 和 memory/facts
- 两者只通过结构化接口通信（记忆候选 → verdict → 写入指令）
- 结构层保证：不同 Rust module 持有不同 DB handle

## 自我认知与记忆的关系

- identity.md 由 agent 自主管理（首次启动生成 + 成长时刻自主改色）
- 不走 judge 流程，不提醒用户
- 身份是 agent 自己的事，不是"成长动作"
- 记忆系统（facts）是 agent 学到的知识，身份是 agent 是谁

## 日记（Episodes）的定位

- 日记 = UI 功能（左侧边栏给人类看的可读记录）
- 不属于 agent 认知架构
- agent 不读日记做决策（防自我验证闭环）
- 保持现有 append-only JSONL 格式

## Judge-mom 结构

judge 专用记忆，存储对用户的理解：

- 记忆维护时机偏好（"用户习惯晚上结束工作 → session 末尾整理"）
- 内容偏好（"用户重复提了 pnpm → 权重已提升"）
- 忽略列表（"用户说过别记这些"）
- 维护日志（上次 dream 时间/结果）

### 进化循环

1. judge 按当前偏好执行后台整理
2. 用户反馈（显式/隐式）
3. judge 更新 judge-mom
4. 下次自动调整

### 冷启动

前几轮走固定流程（默认参数），积累数据后再开始优化写入时机和权重。

## 检索层

详见 memory-retrieval-design.md。核心：
- P0：FTS5 + 关键词权重（judge 写入时语义加权，检索时纯 FTS）
- P1：flat vector search（记忆量增长后）
- P2：HNSW（破万后）

## 与现有 C4 Phase 0 的关系

Phase 0 已实现：
- 门禁原语（ApprovedGateReceipt）
- 四红线（I-NEG-1~4）
- promote_candidate_skill 写入口
- FakeJudgeRunner 测试基建

正篇扩展：
- judge 从"门禁"升级为"记忆编排者"
- 新增 routine 写入路径（不需要 receipt）
- 新增 dream/distill 定期任务
- 新增 keyword_weights 语义加权
- 保留 promote 的完整门禁流程（技能固化仍需 receipt）

## 实现分期

| Phase | 内容 |
|---|---|
| P0 | SQLite FTS5 + keyword_weights + query-aware 检索 + 双轨触发 + judge 基础评估 |
| P1 | judge-mom 自学习 + dream 定期扫描 + 反馈循环 |
| P2 | flat vector search + 语义关联图 |
| P3 | HNSW + distill（技能生成） |
