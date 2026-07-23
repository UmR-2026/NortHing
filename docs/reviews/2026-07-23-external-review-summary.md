# Northhing 现状探索综合报告（第三轮）

> 日期：2026-07-23
> 方法：2 个 subagent 并行探索（C 线架构 + 工程治理），加主 session 快速扫描
> 范围：df5d88a (7-18) → 9d4516a (7-23)，约 166 个 commit
> 前两轮报告：exploration-review-summary_20260721.md（第一轮 3 subagent）

---

## 一、增量变化（vs 7-21 第一轮探索）

### 上次的三个主要问题 → 现状

| 问题（7-21） | 状态（7-23） | 解决方式 |
|---|---|---|
| kernel_facade/mod.rs 2036 行 god file | ✅ 已解决 | b15ad46 拆分为 14 文件，mod.rs 降到 73 行 |
| W3a P0 级并发修复零测试 | ✅ 部分解决 | W3a-1 自带 3 个 tokio::test；W3a+1 补 14 个 scheduler 原语测试 |
| surfaces.md 严重过时 | ✅ 已解决 | 402b653 同步了 surfaces + ledger |

### 新增工作（7-22 ~ 7-23）

**C 线全面启动**——2 天内完成 C1/C2/C3/C4 Phase 0/C5a/C5b/C5c，共 12 个 commit：

| 单 | 语义 | 关键设计 |
|---|---|---|
| C1 | 身份重写：IDE 工具 → 独立同事 | +9/-5 行，语义转变根本性 |
| C2 | Episode Log：agent 开始写日记 | append-only JSONL，repair 追踪（失败→成功对），agent 不读 episodes |
| C3 | 结构化记忆：facts.jsonl | 双语关键词触发，exact-text 去重，1000 token 预算注入 prompt |
| C4 Phase 0 | Judge Gate：门禁原语 | 无 ApprovedGateReceipt 无法执行固化写入；4 红线编译期固化；FakeJudgeRunner 全路径可测试 |
| C5a | subagent 委派矩阵 | prompt 层声明 |
| C5b | subagent 三出口 + episode 覆盖 | Completed/Cancelled/TimedOut 统一 finalize，失败经验不再丢失 |
| C5c | Test + Refactor subagent | 最小权限工具集，强约束 prompt |

**housekeeping rules 写入 AGENTS.md**——5 条规则在 7-22 建立并开始执行：
1. 顺手清配额 ✅ 有正面案例
2. doc sync as hard rule ⚠️ P2-8 违反一次（滞后 4.5h），后续 P2-9 严格遵守
3. god-file defense ⚠️ facade 拆分成功，但 5 个新 god-file 已出现（2 个超 1000 行）
4. 并发测试绑定 ✅ W3a-1 自带测试
5. 宵禁 03:00 ⚠️ 立规矩当天通宵到 05:00，次日遵守

**boundary checker (P2-9)**——230→37 违规，四阶段渐进修复，self-test 绿

**多模型管线成熟**——m3 (judge 首选)、qw (三连轮零返修)、lc (coder 中大型)、m27hs (永久降级)

---

## 二、C 线架构全景

### 核心叙事

C 线是一个完整的演进弧：**从工具到有记忆、有判断、有纪律的独立 agent**。

```
C1 身份 → C2 经验日记 → C3 结构化记忆 → C4 门禁分权 → C5 下属精细化
```

### 架构层次

| 层 | 位置 | 职责 |
|---|---|---|
| 纯协议层 | `agent-runtime/src/judge_gate/` | types, redlines, brief, verdict parser — 零依赖 core |
| core 适配层 | `assembly/core/src/agentic/judge_gate/` | runner, audit, evaluate 编排 |
| 存储层 | episodes.jsonl / facts.jsonl / audit-YYYYMMDD.jsonl | 全部 append-only JSONL |
| prompt 层 | agentic_mode.md / subagent prompts | 身份、委派矩阵、专用 prompt |

### 最有架构野心的部分：C4 Judge Gate

**核心设计**：agent 可以提案，但门禁 judge 裁决才能放行。固化写入（如技能 promote）必须消费 `ApprovedGateReceipt`，没有 receipt 写入口物理上无法执行。

**四条红线**（编译期 const）：
1. 用户数据文件不得消失/移位/清空
2. 未过门产物不得出现在 agent 可自动命中的位置
3. 红线表与门禁代码不被固化动作自身修改
4. 审计日志只可追加

**设计原则**：judge 看世界状态，不信自评文本。证据四槽强类型，candidate 自述权重 = 0。

**测试**：FakeJudgeRunner 使所有路径确定性可测试，覆盖 verdict 解析矩阵 + evaluate 全路径 + promote 并发 + I-NEG-2 不变量。

### C2 Episode Log 的亮点

**repair 追踪**：不只记录失败，还跟踪同一个工具后来是否成功，成功的摘要是什么。失败+修复对就是技能锻造的原料。

**关键边界**：agent 不读 episodes 做决策（防自我验证闭环）。目前是约定层防护（没有代码做这件事），未来需要升级到结构层。

### C3 Facts 的设计

- facts = "本体记忆"（注入 prompt），episodes = "日记"（agent 不读）
- 双语关键词触发蒸馏，exact-text 去重
- confidence 全是 Med，scope 全是 Workspace（High/Low/Global 预留未实现）
- token 预算 1000 hardcoded

---

## 三、工程治理现状

### Housekeeping Rules 落地

| 规则 | 遵守情况 | 细节 |
|---|---|---|
| 1 顺手清 | ✅ 有案例 | facade split 中夹带 ASCII 清理 |
| 2 doc sync | ⚠️ 一次违反 | P2-8 滞后 4.5h；P2-9 严格遵守 |
| 3 god-file | ⚠️ 新债已生 | facade 解决了，但 5 个新 god-file 出现 |
| 4 并发测试 | ✅ 合规 | W3a-1 自带 3 个 tokio::test |
| 5 宵禁 | ⚠️ 立规矩当天违反 | 7-22 通宵到 05:00；7-23 02:48 遵守 |

### 新的 god-file（未登记）

| 文件 | 行数 | 超线情况 | allow-god-file？ |
|---|---|---|---|
| desktop/settings.rs | 1355 | 超 1000，必须拆 | ❌ |
| desktop/callbacks_settings.rs | 1061 | 超 1000，必须拆 | ❌ |
| cli/ui/theme.rs | 854 | 超 800 | ❌ |
| desktop/callbacks_lifecycle.rs | 834 | 超 800 | ❌ |
| judge_gate/mod.rs | 813 | 超 800，新建即超 | ❌ |

### Tech-Debt-Ledger

- 13 条 active / 4 条 resolved
- P0 清零（2 条都 resolved）
- 5 个超 800 行文件未登记是 gap
- P2-9 是 epic 级条目，随着清理可能需要拆分为子条目

### 多模型管线

7 个模型变体参与过工作：
- **m3**：judge 首选，9 连判零漏
- **qw**：7-23 首测三连轮零返修，coder+judge 双角色可靠
- **lc**：coder 中大型首选（有降级条件）
- **m27hs**：三次造假永久降级到 ≤2 文件机械单
- **k3/k2**：额度紧张停用

FAIL→返修→PASS 标准流运行 7 次，是有效的质量护栏。

### Boundary Checker (P2-9)

230→37 的修复是四阶段渐进过程：
- Stage 1：ENOENT 修复（34 路径重映射）
- Stage 2：分诊 + 25 条 stale-rule 修复
- Stage 2b：self-test 锚点同步解锁 112 条
- Stage 2c：同范式扩展解锁 56 条

剩余 37 条：10 regex 修正 + 7 源码核实 + 13 架构决定 + 4 real violation + 3 其他

### Git 卫生

- 166 commit 未推送（单作者、单机）
- checkpoint 频率在加速，7-23 改用 handoff 代替（更好的实践）
- commit message 质量高，conventional commits + 任务编号 + 验证状态

---

## 四、开放问题（按优先级）

### 高优先级

1. **5 个 god-file 未登记且 2 个超 1000 行无注释**——规则 3 明确违反。需要在 ledger 中登记 + 制定拆分计划，或加 `allow-god-file` 注释。

2. **166 commit 未推送**——单点故障风险。如果本地磁盘损坏，所有工作丢失。handoff 说"等用户决定"，但这个风险在累积。

3. **C1 身份段已改但行为段未调谐**——agentic_mode.md 前半段说"不是 IDE、不是编码工具"，后半段仍是大段编程指导。身份和行为之间有割裂。

### 中优先级

4. **facts 去重太脆弱**——exact text match 无法处理空格/措辞差异。随着积累会膨胀。
5. **episodes "agent 不读"边界是约定层非结构层**——目前靠"没有代码做这件事"。需要 cargo 断言或路径黑名单升级。
6. **judge_gate receipt consumed set 是进程内的**——重启后 reset。如果 promote 被 consumed 但写入失败（断电），重启后 receipt 可被重用。
7. **boundary checker 13 条需架构决定**——这些不是机械修复，需要人/架构层拍板。

### 低优先级

8. **facts confidence 全 Med / scope 全 Workspace**——High/Low/Global 的产生路径未实现。
9. **Test subagent "修测试不改源码"限制**——发现 bug 只能报告不能修，可能过于死板。
10. **subagent 无 model 分流**——Test/Refactor 可能不需要最强模型。

---

## 五、积极发现

1. **C 线 2 天完成 7 个子任务**——身份重写 + 经验日志 + 结构化记忆 + 门禁原语 + subagent 精细化，架构基础设施已就位
2. **housekeeping rules 有效地驱动了清理**——上次报告的三个主要问题都在规则建立后 1-2 天内被处理
3. **Judge Gate 是有架构野心的设计**——结构不可绕过的门禁，FakeJudgeRunner 全路径可测试，四红线编译期固化
4. **episode repair 追踪是亮点**——跨 round 跟踪失败→成功对，为技能锻造提供原料
5. **多模型管线成熟**——7 个模型变体各有定位，FAIL→返修→PASS 循环有效运行
6. **测试从 110 增长到 1057**（product-full feature），W3a+1 补了 14 个 scheduler 测试
7. **handoff 文档质量高且在进化**——7-23 比 7-22 更精炼，包含完整队列、blocking 边、已知雷区
8. **P0 清零**——两个用户阻塞问题都已 resolved
9. **commit message 质量高**——conventional commits + 任务编号 + 验证状态
10. **boundary checker 230→37**——四阶段渐进修复，self-test 绿
