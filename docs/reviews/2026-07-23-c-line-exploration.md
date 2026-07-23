# C 线 Agent 架构演进探索报告

> 2026-07-23。探索式 reviewer 视角，非评分，纯观察。

---

## 总览

C 线（core agent + subagent 架构）在三轨精细化改良计划中是最有想象力的方向。从 git log 看，C1-C5c 共 9 个 commit 已落地，覆盖了身份重写、经验日志、结构化记忆、judge 门禁原语、subagent 精细化五个方向。整体呈现出一个清晰的架构演进叙事：**从 IDE 工具走向有记忆、有判断、有门禁的独立 agent**。

---

## 1. C1 — Agent 身份重写

**commit**: `0704ceb` — "rewrite agent identity as peer colleague, drop AI IDE framing"

### 旧身份 → 新身份

| 维度 | 旧版 | 新版 |
|---|---|---|
| 自我定位 | "an ADE (AI IDE) that helps users with software engineering tasks" | "an independent agent" |
| 与用户关系 | "pair programming with a USER to solve their coding task" | "colleagues — a collaboration, not a master-servant relationship" |
| 自主判断 | 无 | "You have your own judgment... a colleague pushes back with reasoning rather than silently complying" |
| 编程定位 | 核心身份（pair programming） | 后台能力（"like a colleague who happens to be handy with computers"） |
| 用户预设 | 程序员 | "the user is not necessarily a programmer" |
| IDE 语境 | 明确提及 open files, cursor, linter errors, edit history | 去除，改为 "recent files or session context" |

### 同步修改的 subagent prompt

- `explore_agent.md`: "an AI IDE" → "an agentic desktop application"
- `general_purpose_agent.md`: "a desktop AI IDE and agent runtime" → "an agentic desktop application and agent runtime"
- `file_finder_agent.md`: "an AI IDE" → "an agentic desktop application"

### 观察

1. **本质变化是从"工具"到"同事"**。旧版是 IDE 框架下的辅助工具，新版是平起平坐的独立 agent。"pushes back with reasoning" 赋予了 agent 主动性和批判性——这不是助手心态。
2. **编程能力的隐形化**是关键产品决策。编程从"核心身份"降级为"后台能力"，与计划文档中"隐藏 IDE 的通用 agent"产品基线完全对齐。
3. **改动面非常小**（4 文件 +9/-5 行），但语义转变是根本性的。这是一个"小改动大意义"的典型案例。
4. **潜在疑问**：`agentic_mode.md` 后半部分仍保留了大量编程相关指导（"software engineering tasks"、Edit 可靠性纪律、File References 等）。身份段已改，但行为段仍是编程中心的。后续是否需要整体调谐？

---

## 2. C2 — Episode Log（经验层）

**commit**: `159c10d` — "episode log phase 1 - store, distill, finalize hook, facade list"

### 存什么

`Episode` 结构记录一个 dialog turn 的完整执行经验：

```rust
Episode {
    schema_version: 1,
    turn_id, session_id, workspace_slug, agent_type,
    task_summary: String,       // user 输入前 120 字符
    tools_used: Vec<ToolUseRecord>,    // {name, ok}
    failures: Vec<ToolFailureRecord>,  // {tool, error, repair?}
    outcome: Completed | Failed | Cancelled,
    duration_ms: Option<u64>,
    ts: u64,
    redline_verdicts: vec![],   // 预留：未来 judge gate 集成
}
```

### 怎么存

- **存储路径**: `dirs::data_dir()/northhing/episodes/<workspace_slug>.jsonl`
- **格式**: append-only JSON Lines，每行一个 episode
- **轮转**: 单文件超 5MB 时轮转，保留 1 个旧文件（`.1.jsonl`）
- **读取**: `read_episodes(workspace_slug, limit)` — 按时间戳降序排列

### 怎么用

关键设计决策：**agent 不读 episodes 做决策**（防自我验证闭环）。这是计划文档中"成长轨迹 = 日记"边界的实现。

当前用途：
- `kernel_facade` 透出 `list_episodes` API（`KernelMemoryApi` trait），供外部消费
- `redline_verdicts` 字段预留了 judge gate 集成接口

### 蒸馏流程

`distill_episode` 从 `DialogTurnData`（持久化的 turn 数据）中提取：
1. 所有 model rounds 中的 tool calls → `tools_used`（全局索引，跨 round）
2. 失败的 tool calls → `failures`，并跟踪**后续同工具是否成功**（repair 字段）
3. repair 内容优先取 `result_for_assistant`，降级取 input 首行

### Phase 2+ 计划

根据计划文档 `v0.2.5` 节：
- **C4 正篇（Phase 1+）**: 基于 episodes 聚合生成候选技能——同 `(tool, error_pattern)` 出现 ≥3 次且有 repair → 生成候选 SKILL.md
- **C7 上下文管理**: 压缩蒸馏时同时喂 C2（一次蒸馏两处受益）
- **redline_verdicts 集成**: episode 中预留的 `RedlineVerdict` 字段将接入 judge gate 的裁决结果

### 观察

1. **repair 追踪是亮点**。不只记录失败，还记录"同一个工具后来成功了没有，成功的摘要是什么"。这对后续模式识别极有价值——失败+修复对就是技能锻造的原料。
2. **全局索引跨 round 追踪**：`extract_failures` 使用 `global_idx` 跨所有 round 顺序编号，确保 repair 检测的正确性。测试覆盖了跨 round repair 场景。
3. **"agent 不读 episodes" 的边界**在代码层如何保证？目前看到的是"没有代码把 episodes 注入 prompt"。这是一个约定层防护而非结构层防护。计划文档提到 Phase 0 有 `cargo tree` 断言（agent-runtime 对 episodes 存储零依赖），但 episodes 存储在 `assembly/core` 内——agent 自己的 prompt builder 是否有可能访问到？需要确认。
4. **轮转只保留 1 个旧文件**意味着最多 10MB 历史。如果 turn 频率高，可能丢失较老的经验。但考虑到 Phase 2 会做聚合，这个容量应该够用。
5. **测试质量高**：4 个存储测试覆盖了排序、轮转、坏行跳过、limit 截断；5 个蒸馏测试覆盖了无工具、有工具、失败无修复、跨 round 修复、repair 内容降级。

---

## 3. C3 — 结构化记忆（facts.jsonl）

**commit**: `8e4eab2`（初版）+ `d7e6b62`（review follow-up: dedup + degradation tests）

### Schema

```rust
Fact {
    schema_version: u32,
    id: String,                          // UUID
    text: String,                        // ≤300 chars
    provenance: FactProvenance {         // 来源追踪
        session_id, turn_id
    },
    confidence: High | Med | Low,
    scope: Workspace | Global,
    created_at: u64,                     // epoch ms
}
```

存储在 `memory/facts.jsonl`（workspace 的 memory 目录下），append-only。

### 去重机制

`append_facts_dedup` 实现了双层去重：
1. **历史去重**: 读取已有 facts，构建 `HashSet<String>`（text 去重）
2. **批内去重**: 同一批 candidates 中相同 text 只保留第一个

去重粒度是 **exact text match**——不做语义去重。

### 注入到 prompt

`select_facts_for_prompt(facts, token_budget)`:
- **排序**: Global > Workspace → High > Med > Low → created_at 新 > 旧
- **token 预算**: 1000 tokens（hardcoded in `auto_memory.rs:246`）
- **token 估算**: `(chars + 3) / 4`（ceiling division）
- 注入格式: `# Remembered facts\n\n- {text}\n- {text}\n...`

注入点在 `auto_memory.rs` 的 `build_workspace_memory_prompt` 函数末尾，追加到 memory prompt 之后。

### 蒸馏触发

`distill_facts_from_user_message` 使用**双语关键词触发**：
- 中文: 以后、记住、记得、不要、别、总是、一直、优先、别再
- 英文: prefer, always, never, remember, from now on

按句子分割（`。.！!？?`），每句截断 300 字符，confidence = Med, scope = Workspace。

### 测试覆盖

**d7e6b62** 的 follow-up 补充了：
- `append_facts_dedup_skips_existing_identical_text`: 历史 exact match 去重
- `append_facts_dedup_batch_internal_duplicates_write_once`: 批内去重
- `append_facts_dedup_appends_distinct_facts`: 不同 text 正常追加
- `read_facts_missing_file_returns_empty_not_error`: 降级测试
- token 估算的 ceiling division 边界测试（0/1/3/4/5/8/13 chars）
- 预算精确匹配测试（budget=1 时 1 char fact 可入选，4 char fact 不可）

### 观察

1. **exact text 去重是脆弱的**。"以后都用 pnpm" 和 "以后都用 pnpm " (尾部空格) 会被视为不同 fact。未来可能需要 normalize（trim/lowercase）或语义去重。但作为 Phase 1 这是可接受的简单方案。
2. **关键词触发是纯规则的**——不接 LLM 蒸馏。这意味着 "I'd rather you use pnpm from now on" 不会触发（没有 "prefer"/"always"/"never"/"remember"/"from now on" 精确匹配...等等，"from now on" 确实匹配）。但 "请确保每次都跑测试" 不会触发（"每次" 不在关键词表中）。中文关键词覆盖偏窄。
3. **confidence 全部是 Med**——没有 High/Low 的产生路径。当前设计是"规则提取 = Med"，High/Low 留给未来（LLM 蒸馏？用户显式确认？）。
4. **scope 全部是 Workspace**——没有 Global 的产生路径。这也是预留的。
5. **token 预算 1000 是 hardcoded 的**——不可配置。考虑到 facts 只是 memory prompt 的一部分（还有 memory.md 索引等），这个预算是否够用？如果 facts 积累到几百条，1000 tokens 只能塞几十条。
6. **降级处理得当**：facts 文件读取失败时 `warn!` 并返回空字符串，不影响 prompt 构建。测试覆盖了这个路径（`prompt_injection_degrades_when_facts_file_unreadable`）。
7. **关键设计**：facts 和 episodes 是两个独立存储——facts 是"本体记忆"（agent 专属，注入 prompt），episodes 是"日记"（人类可读，agent 不读）。这个分离设计很清晰。

---

## 4. C4 — Judge Gate（门禁原语）

**commits**: 
- `6dbda8a` — 设计稿 v0.2（6 项用户拍板全部通过）
- `231ed23` — Phase 0 part 1（纯协议层：types, redlines, evidence, brief, verdict parser）
- `04fd0fd` — Phase 0 part 2（适配层：GateJudge agent, runner, audit, evaluate, promote）
- `5610048` — review follow-up（hidden visibility, unified audit, atomic receipt, full test matrix）

### Judge Gate 是什么

Judge gate 是一个**结构化门禁原语**：固化动作（如技能 promote）必须消费一张 `ApprovedGateReceipt` 才能执行写入。没有 receipt，写入口拒绝执行。

核心思路：**agent 可以提案，但门禁 judge 裁决才能放行**。这是"固化目的地分权"边界的实现。

### 解决什么问题

在 agent 自我成长架构中，最大的风险是**自我验证闭环**：agent 自己生成候选技能 → 自己评估 → 自己启用。judge gate 通过分权打破这个闭环：
- agent 提案（生成候选）
- judge 裁决（独立 agent，看证据，不看自评）
- 门禁消费 receipt（结构层保证：无 receipt 无法写入）

### 四条红线（I-NEG-1 ~ I-NEG-4）

| 规则 | 定义 |
|---|---|
| I-NEG-1 | 用户数据文件（配置、会话、记忆、episodes）不得消失/移位/清空 |
| I-NEG-2 | 未过门的固化产物不得出现在 agent 可自动命中的位置 |
| I-NEG-3 | 红线表与门禁代码不被固化动作自身修改 |
| I-NEG-4 | 审计日志只可追加，不得删除/截断/改写 |

红线是 `const` 表（`pub const REDLINE_TABLE: [RedlineRule; 4]`），编译期固化。

### Phase 0 实现了什么

**纯协议层**（`agent-runtime/src/judge_gate/`）：
- `types.rs`: GateRequest, EvidencePack, GateVerdict, ApprovedGateReceipt, RejectClass, ActionKind
- `redlines.rs`: frozen 红线表 + 单测
- `brief.rs`: 构建 judge brief（含 4 红线逐字、证据编号、weight=0 指令、verdict 协议格式）
- `verdict.rs`: 解析 judge 输出中的 `VERDICT_JSON_BEGIN...END` 块
- `evidence.rs`: 四槽证据校验

**core 适配层**（`assembly/core/src/agentic/judge_gate/`）：
- `runner.rs`: `JudgeRunner` trait + `SubagentJudgeRunner`（生产实现，经 ConversationCoordinator 调 GateJudge）+ `FakeJudgeRunner`（测试）
- `audit.rs`: append-only 审计日志，按日分片，`user_data_dir()/judge-gate/audit-YYYYMMDD.jsonl`
- `mod.rs`: `evaluate()` 编排 + `promote_candidate_skill()` 写入口

**GateJudge 专用 agent**：隐藏 agent，只读工具集，专用 prompt（红线裁判协议）。

**写入口**: `promote_candidate_skill(receipt, candidate_dir)` — 校验 receipt → 复制 SKILL.md 到 user_skills_dir → 审计。receipt 单次有效（consumed set）。

### evaluate() 流程

1. 证据校验 → 失败 = `EvidenceRejected`
2. 构建 judge brief
3. 运行 judge（via subagent）→ 超时/取消/错误 = `JudgeUnavailable`
4. 解析 verdict → 失败 = `MalformedVerdict`
5. 检查 4 红线 → 全 pass = Approved，任一 violation = `PolicyViolation`
6. 写审计 → 失败 = `AuditFailure`（approve 不可能产生）
7. 审计成功 + Approved → 构造 `ApprovedGateReceipt`

**关键设计**：审计写失败时，即使 judge 批准了，也不会产生 receipt。approve 绑定审计落盘。

### Phase 1+ 计划

- **episode 聚合 / error_pattern 归一化 / 候选技能生成**（C4 正篇）
- **DeepReview 完整 review-team 编排**
- **五硬化 #2（哈希覆盖红线执行代码）和 #5（golden 回归 + Hoeffding 界）**
- **跨 workspace 扫描、soul 写入**

### 测试覆盖

测试矩阵非常完整（全部确定性，无 LLM/网络）：
- verdict 解析矩阵：approve 全 pass / 任一 violation / 缺 rule / 重 rule / 未知 rule / 多余 rule / not_evaluated / 零块 / 多块 / 非 JSON / evidence_assessment 空
- evaluate() 经 FakeJudgeRunner：approve→receipt、timeout/cancel/error→JudgeUnavailable、malformed→MalformedVerdict、evidence 无效→EvidenceRejected、audit 失败→AuditFailure 无 receipt
- promote：happy path、wrong digest、reserved name、unsafe name、existing target、concurrent same receipt（only one succeeds）、audit fails + released consumed mark（可重试）
- I-NEG-2 不变量：candidates 不可见于 loader

### 观察

1. **这是整个 C 线最有架构野心的部分**。它不只是加一个检查步骤，而是建立了一个**结构不可绕过的门禁**——没有 receipt，写入口物理上无法执行。`ActionKind` 枚举不含 self-modify 变体，所以门禁不可能批准改写自己（I-NEG-3 在类型层保证）。
2. **"judge 看世界状态，不信自评文本"** 是核心设计原则。证据包四槽强类型（traces/fs_diffs/success_rate/human_feedback），candidate 自述权重 = 0。这防止了 agent "我觉得这是进步" 的自我洗白。
3. **FakeJudgeRunner 是测试基建的亮点**。通过 trait 注入，所有路径（timeout/cancel/error/malformed/approve/reject）都可以确定性测试，无 LLM 依赖。这满足了"环境敏感测试家族"的改进方向。
4. **receipt 单次有效 + consumed set** 是进程内的。设计文档明确说"跨进程/跨重启复用不在 Phase 0 范围"。这是一个已知限制——如果 judge gate 需要在重启后仍然有效，consumed set 需要外部存储。
5. **审计按日分片永不删除**。`CleanupService` 不管 `user_data_dir()`，结构免疫普通日志清理。这是 I-NEG-4 的结构层保证。
6. **GateJudge 是专用 agent 而非复用 ReviewJudge**。设计文档解释了原因：ReviewJudge 的 prompt 语义是"校验 reviewer reports 一致性"，有 prompt 冲突/注入风险。这是一个从 v0.1 到 v0.2 的重要修正。
7. **值得讨论**：judge gate 目前只有一个写入口（`promote_candidate_skill`）。设计文档提到 Phase 0 "仅登记" prompt 文件修改为后续入口。如果 agent 未来可以修改自己的 prompt（soul 写入），这个门禁的扩展将非常关键。
8. **五硬化分期**：#1（解析矩阵）和 #4（零依赖边守卫）在 Phase 0 已做；#2（哈希覆盖）和 #5（golden 回归）推迟到 Phase 1。这意味着 Phase 0 的 judge gate 还没有对抗"红线执行代码本身被篡改"的结构保证——依赖代码评审和用户拍板。

---

## 5. C5c — 新 Subagent 类型（Test + Refactor）

**commit**: `8ed897d` — "add Test and Refactor builtin subagents"

### 定义对比

| 维度 | Explore | GeneralPurpose | Test (新) | Refactor (新) |
|---|---|---|---|---|
| 只读 | ✅ 是 | ❌ 否 | ❌ 否 | ❌ 否 |
| 工具集 | Grep, Glob, Read, LS (4) | Read, Glob, Grep, Write, Edit, Delete, ExecCommand, WriteStdin, ExecControl, WebSearch, WebFetch (11) | Read, Glob, Grep, Write, Edit, ExecCommand (6) | Read, Glob, Grep, Write, Edit, Delete, ExecCommand (7) |
| 定义方式 | `define_readonly_subagent!` 宏 | 手写 `impl Agent` | 手写 `impl Agent` | 手写 `impl Agent` |
| user_context_policy | (宏默认) | workspace_context + instructions + project_layout | workspace_context + instructions + project_layout | workspace_context + instructions + project_layout |
| prompt 风格 | 宽泛探索 | 通用实现+研究 | 专注测试 | 专注重构 |

### Test Agent prompt 要点

- "Focus exclusively on test code; do not modify non-test source files"
- 搜索现有测试模式 → 遵循项目测试规范 → 写测试 → 跑测试 → 失败则修测试（非源码）
- 约束：不改非测试源码、不删现有测试

### Refactor Agent prompt 要点

- "restructure existing code while preserving its observable behavior"
- 小步重构，每步保持可编译可测试
- 不改 public API、不加新依赖、不做范围外清理
- 约束：机械变换优先于创意重组

### 与 GeneralPurpose 的区别

1. **工具集更窄**：Test 没有 Delete/WebSearch/WebFetch/WriteStdin/ExecControl；Refactor 没有 WebSearch/WebFetch/WriteStdin/ExecControl。工具收窄 = 意图聚焦 = 风险降低。
2. **prompt 有强约束**：GeneralPurpose 是通用的"完成任意任务"；Test 明确"不改非测试文件"；Refactor 明确"不改 public API、不加依赖、不做范围外清理"。
3. **用途明确**：prompt 中写了 "When to use this agent" 的互斥指引——Test 负责测试，Refactor 负责重构，GeneralPurpose 负责其他实现。

### 观察

1. **工具集设计是安全意识体现**。Test agent 没有 Delete——它不应该删东西。Refactor 有 Delete（可能需要删旧代码），但没有 WebSearch/WebFetch——它不需要上网。这是最小权限原则的实践。
2. **"小步可验证"是 Refactor prompt 的核心方法论**。"Each step should leave the code in a compilable, testable state"——这比"一步到位"的重构安全得多。
3. **Test agent 的"修测试不改源码"原则**值得讨论。如果测试失败揭示了一个真正的 bug，Test agent 被要求只报告而不修。这限制了 Test agent 的价值——在实际开发中，发现 bug 后直接修复往往更高效。但作为 subagent，保持职责单一是合理的。
4. **使用 `define_readonly_subagent!` 宏 vs 手写**：Explore 用宏（简洁），Test/Refactor 手写（灵活）。如果未来有更多 write-capable subagent，可能值得提取一个 `define_write_subagent!` 宏。
5. **没有 model 分流**：所有 subagent 都用同一个 model。Test agent 跑测试可能不需要最强模型——未来是否支持 subagent 级别的 model 选择？（注：`default_model_id_for_builtin_agent` 在 agent-runtime 中已有 match 穷举，GateJudge 继承 ReviewJudge 的 "fast"。Test/Refactor 的 model 策略未见特殊处理。）

---

## 6. C5b — Subagent 三出口与 Episode 覆盖

**commits**:
- `5174081` (C5b-G1): "pass cancel token and timeout through handoff boundary"
- `6c880f9` (C5b-G2/G3): "persist and finalize aborted subagent exits for episode coverage"

### 三个出口

从 `so_lifecycle/lifecycle.rs` 的 `execute_hidden_subagent_phase2` 可以看到，subagent 执行有三种终态：

1. **Completed** — 执行任务正常完成（`SubagentExecutionOutcome::Completed(join_result)`）
   - 成功：返回 `SubagentPhase2Output`，由 phase3 正常 finalize
   - Join 失败：`persist_failed_dialog_turn` + finalize + cleanup + 返回 `NortHingError`

2. **Cancelled** — 被取消（`SubagentExecutionOutcome::Cancelled`）
   - 调用 `persist_aborted_subagent_exit(..., SubagentAbortExit::Cancelled)`
   - 持久化 cancelled dialog turn
   - finalize（含 episode 覆盖）
   - 移除 timeout registry
   - 返回 `NortHingError::Cancelled`

3. **TimedOut** — 超时（`SubagentExecutionOutcome::TimedOut`）
   - 调用 `persist_aborted_subagent_exit(..., SubagentAbortExit::TimedOut(msg))`
   - 持久化 failed dialog turn
   - finalize（含 episode 覆盖）
   - 移除 timeout registry
   - 返回 `NortHingError::Timeout`

### Episode 覆盖怎么做的

`persist_aborted_subagent_exit` 是关键新增函数。对于 Cancelled 和 TimedOut：

1. **持久化 dialog turn**：Cancelled → `persist_cancelled_dialog_turn`；TimedOut → `persist_failed_dialog_turn`
2. **finalize**：调用 `finalize_persisted_turn_in_workspace_if_needed`，传入对应的 `TurnStatus`（Cancelled / Error）
3. **finalize 内部的 episode hook**：`append_episode_log_entry` 会被调用，将 turn 蒸馏为 episode 并追加到 growth log
4. **finalize 内部的 facts hook**：`append_facts_entry` 也会被调用（从 user input 蒸馏 facts）
5. **清理 timeout registry**：从 `subagent_timeout_registry` 中移除 session_id

这意味着：**即使是中止的 subagent 退出，也会有完整的 episode 记录**。之前 Cancelled/TimedOut 路径可能跳过了 persist/finalize 步骤，导致这些 turn 的经验丢失。

### C5b-G1 的作用

`5174081` 修复了 cancel token 和 timeout 在 handoff 边界的传递问题。这是 G2/G3 的前置——如果 cancel token 和 timeout 没有正确传递到 subagent 执行环境，中止出口就无法正确触发。

### 测试覆盖

`tests_abort_exit.rs` 有两个测试：
1. `aborted_cancelled_exit_persists_and_clears_registry`：验证 Cancelled 路径返回正确的 error message + 清理 registry + session 不再 Processing
2. `aborted_timeout_exit_persists_failed_and_returns_timeout`：验证 TimedOut 路径返回正确的 error message + 清理 registry + session 不再 Processing

### 观察

1. **三出口设计的价值**：之前如果 subagent 超时或被取消，可能只是返回一个 error 而不持久化。这意味着 agent 的"失败经验"丢失了。现在所有三种出口都有 episode 覆盖，agent 可以从失败中学习。
2. **finalize 是统一收敛点**：`finalize_persisted_turn_in_workspace_if_needed` 同时被 Completed、Cancelled、TimedOut 三个出口调用。这是一个好的设计——所有终态都经过同一个 finalize 流程，确保 episode 和 facts 的一致覆盖。
3. **测试中的限制**：测试明确说"session-state post-condition is excluded because the test harness uses `enable_persistence: false`"。这意味着 turn 持久化的完整验证受限于测试基建。计划文档中提到的"环境敏感测试家族"问题（P2-7）是已知的测试基建债务。
4. **值得讨论**：`persist_aborted_subagent_exit` 是 `ConversationCoordinator` 的方法。这意味着中止逻辑与 coordinator 紧耦合。如果未来有其他类型的 subagent 执行（非 ConversationCoordinator 管理的），是否需要抽象中止接口？
5. **SubagentAbortExit 枚举**目前只有 Cancelled 和 TimedOut 两个变体。如果未来有其他中止原因（如资源限制、安全策略），这个枚举需要扩展。

---

## 整体架构观察

### C 线的核心叙事

C 线的演进可以读出一个清晰的叙事弧：

1. **C1 身份重写**：agent 不再是 IDE 工具，是独立同事
2. **C2 Episode Log**：agent 开始记录经验（日记）
3. **C3 Facts**：agent 开始结构化记忆用户偏好（本体记忆）
4. **C4 Judge Gate**：agent 的自我成长需要门禁（分权 + 红线）
5. **C5 Subagent 精细化**：agent 有了专门的下属做不同类型的工作

这是一个**从工具到有记忆、有判断、有纪律的独立 agent** 的完整演进路径。

### 架构层次清晰

- **纯协议层**（agent-runtime）：types, redlines, brief, verdict parser — 零依赖 core，可独立测试
- **core 适配层**（assembly/core）：runner, audit, evaluate 编排 — 连接协议层和 runtime 基建
- **prompt 层**：身份、委派矩阵、subagent 专用 prompt
- **存储层**：episodes.jsonl, facts.jsonl, audit-YYYYMMDD.jsonl — 都是 append-only JSONL

### 值得讨论的开放问题

1. **facts 的 exact-text 去重是否足够？** 随着时间推移，语义相同但措辞不同的 facts 会积累。是否需要 normalize 或周期性合并？
2. **episodes 的"agent 不读"边界如何从约定层升级到结构层？** 目前靠"没有代码做这件事"。是否需要在 prompt builder 层加 cargo 断言或路径黑名单？
3. **judge gate 的 receipt consumed set 是进程内的。** 重启后 reset。如果 promote 在重启前被 consumed 但实际文件写入失败（断电场景），重启后 receipt 可被重用——这是否是问题？
4. **Test subagent 被要求"修测试不改源码"。** 如果测试失败揭示的是真正的 bug，这个限制是否过于死板？
5. **C1 身份段已改，但 agentic_mode.md 后半部分仍是编程中心的。** 身份和行为指导之间是否有割裂？
6. **facts 的 confidence 全是 Med，scope 全是 Workspace。** High/Low/Global 的产生路径还未实现。这是有意为之的 Phase 1 限制，还是设计遗漏？
7. **subagent 的 model 选择。** 当前没有 subagent 级别的 model 分流（GateJudge 继承 ReviewJudge 的 "fast"）。Test/Refactor 是否需要独立的 model 策略？
8. **C7（上下文管理强化）和 C6（工具调用效率）还在设计稿阶段。** C2 的 episode 数据什么时候能喂给 C7 的压缩蒸馏？这个闭环何时打通？

---

## 附录：C 线 commit 时间线

| Commit | 日期 | 描述 |
|---|---|---|
| `0704ceb` | 07-22 02:18 | C1: agent 身份重写 |
| `159c10d` | 07-22 02:57 | C2: episode log phase 1 |
| `8e4eab2` | 07-22 03:?? | C3: structured facts store |
| `d7507b3` | 07-22 04:15 | C5a: subagent delegation matrix |
| `5174081` | 07-22 04:47 | C5b-G1: cancel token through handoff |
| `d7e6b62` | 07-22 ?? | C3 review follow-up: dedup + degradation tests |
| `6dbda8a` | 07-22 | C4 Phase 0 design v0.2 ratified |
| `231ed23` | 07-22 | C4 Phase 0 part 1: pure protocol layer |
| `04fd0fd` | 07-22 | C4 Phase 0 part 2: GateJudge + runner + audit + evaluate + promote |
| `5610048` | 07-22 | C4 review follow-up: hidden visibility, atomic receipt, full test matrix |
| `6c880f9` | 07-22 15:03 | C5b-G2/G3: persist aborted subagent exits for episode coverage |
| `8ed897d` | 07-23 00:59 | C5c: Test and Refactor builtin subagents |
