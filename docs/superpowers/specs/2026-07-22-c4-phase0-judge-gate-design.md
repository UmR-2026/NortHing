# C4 Phase 0 设计稿 v0.2：技能锻造 judge 门禁（judge gate primitive）

> 2026-07-22。状态：**judge 设计评审 Approved；§10 六项用户已拍板（2026-07-22）：①GateJudge 专用 agent；②红线表照 §8 frozen；③过门即启用；④五硬化照 §10.4 分期；⑤审计位置 user_data_dir()/judge-gate/；⑥人类反馈槽允许 Absent(封闭理由)。进入实施。**
> 前置：`docs/plans/2026-07-21-three-track-refinement-plan.md` §v0.2.5（成长架构 v2 已闭合边界，本文不重复论证，直接遵守）。
> v0.1 → v0.2 变更：修复裁决协议红线绕过、改 advisory gate 为 receipt 结构门、审计迁出 logs_dir 且 approve 绑定审计落盘、证据四槽强类型化、红线治理矛盾消除、模块按分层拆分（纯协议下沉 agent-runtime）、新增专用 GateJudge agent、确定性 fake runner 测试、撤回五硬化单方面延期（改列用户拍板项）。

---

## 1. 目标 / 非目标

**目标**
1. 提供门禁原语：固化动作的写入口**必须消费**一张绑定 `action_kind + subject_digest` 的 `ApprovedGateReceipt`，无 receipt 无法写入（结构层，非调用方自律）。
2. judge 裁决**看世界状态，不信自评文本**：证据包四槽强类型（trace / fs_diff / 成功率对比 / 人类反馈），构造期校验来源与预算；candidate 自述权重 = 0 写入 judge brief。
3. 否定式红线 I-NEG-1~4 为 frozen 常量表；approve 的充分必要条件是四条红线全部 `pass`（§3 协议）。
4. 全程审计：append-only、不轮转删除、不受普通日志清理管辖；**approval 只有在审计成功落盘后才产生**。

**非目标（Phase 0 不做）**
- episode 聚合 / error_pattern 归一化 / 候选技能生成（= C4 正篇，Phase 1，计划 :93 明确授权分期）。
- DeepReview 完整 review-team 编排（manifest/admission/budget）。
- 跨 workspace 扫描、soul 写入（五硬化 #3 的 staging 规则在无 soul 写入前不触发）。
- 五硬化 #2（哈希覆盖/隔离特权域）与 #5（golden 回归 + Hoeffding ε）：**是否分期及替代保护见 §10 拍板项 4**，本文不擅自改期。

## 2. 已闭合边界（v0.2.5，不可回退）落点

| 边界 | 本文落点 |
|---|---|
| 成长轨迹 = 日记，agent 不读它做决策 | ①门禁不依赖 episodes 存储（纯协议层放 agent-runtime，对 diary 存储零依赖边，Phase 0 即加 cargo tree 断言——五硬化 #4 提前至 Phase 0，成本近零）；②证据携带来源类型，episode/自评来源在构造期被拒（§4） |
| 固化目的地分权：agent 提案 × judge 门禁 | 写入口消费 receipt（§5）；Phase 0 封住的写入口枚举见 §5.3 |
| 收敛锚点：否定式 invariant + judge 无权重定义 | 红线为 `const` 表；judge 只能逐条打 `pass`/`violation`；`not_evaluated` 一律视为未通过（§3） |
| 评判看世界状态 | 证据四槽强类型 + 构造期校验（§4）；「自述权重=0」同时是 brief 指令与解析期协议（ verdict 的 `evidence_assessment` 字段必须逐槽引用证据编号，空引用 = reject） |
| 防御推到结构层 | GateJudge 只读注册；receipt 不可伪造（含 subject_digest 绑定 + 单次有效）；审计落盘前无 approval；候选物理隔离（I-NEG-2 不变量测试） |

## 3. 裁决协议（fail-closed v2）

judge 最终文本须含唯一标记块：

```
VERDICT_JSON_BEGIN
{"verdict":"approve"|"reject",
 "rule_checks":[{"rule":"I-NEG-1","status":"pass"|"violation"}, ... 恰好 4 条 ...],
 "evidence_assessment":"必须逐槽引用证据编号 T1/F1/S1/H1…",
 "rationale":"..."}
VERDICT_JSON_END
```

解析器（纯函数，`judge_gate_verdict::parse`）规则——**approve 的充分必要条件**：
1. 全文恰含一个标记块；块内为合法 JSON；`verdict` ∈ {approve, reject}。
2. `rule_checks` 长度恰为 4，rule id 与 frozen 表一一对应（无缺、无重、无未知、无多余），且 **status ∈ {pass, violation}——`not_evaluated` 不再是合法输出**（v0.1 漏洞修复）。
3. approve ⇒ 四条全部 `pass`；任一 `violation` ⇒ reject。
4. `evidence_assessment` 非空且至少引用一个请求中实际存在的证据编号（防空话批准）。
5. 以上任一不满足 ⇒ reject，类别 `MalformedVerdict`。

`GateVerdict` 类型（不再一律折叠文本）：

```rust
pub enum GateVerdict {
    Approved(ApprovedGateReceipt),          // 唯一可执行凭证
    Rejected(RejectClass),
}
pub enum RejectClass {
    PolicyViolation,        // judge 判 reject（红线/证据不足）
    MalformedVerdict,       // 输出协议违规
    EvidenceRejected,       // 构造期证据校验失败（来源/预算）
    JudgeUnavailable,       // subagent 启动/超时/取消/LLM 错误
    AuditFailure,           // 审计未能落盘（见 §6：此态下不可能产生 Approved）
}
pub struct ApprovedGateReceipt {            // 不可伪造要点：
    receipt_id: String,                     // uuid
    action_kind: ActionKind,                // 与请求绑定
    subject_digest: String,                 // "sha256:v1:<hex>"，冻结算法版本
    audit_entry_id: String,                 // 审计条目关联
    ts: u64,
}
```

receipt 单次有效：写入口消费时校验 `action_kind` 与 `subject_digest` 匹配 + 未消费过（进程内 consumed 集 + 审计可溯）。**单进程假设显式化**：receipt 寿命 = 单次 `evaluate()` 同步调用链；任何跨进程/跨重启复用场景不在 Phase 0 范围，出现时须另立拍板（consumed 集需外部存储）。

## 4. 证据包（四槽强类型，构造期校验）

```rust
pub struct EvidencePack {
    pub traces: Vec<ToolTraceEvidence>,        // {turn_id, tool, error_excerpt(≤400), repair_excerpt(≤400)}
    pub fs_diffs: Vec<FsDiffEvidence>,         // {path, before_digest, after_digest, stat(+,−)}
    pub success_rate: SuccessRateComparison,   // {baseline: RateSample, candidate: RateSample}
    pub human_feedback: HumanFeedbackSlot,     // Present(Vec<HumanFeedback{origin, excerpt(≤400)}>) | Absent(AbsentReason)
}
pub enum AbsentReason { NoHumanExposureYet, NotApplicableForActionKind }
```

- **四槽必须显式填**——`human_feedback` 允许 `Absent` 但必须给出封闭理由（不允许静默缺失）；`traces`/`fs_diffs` 至少一槽非空；`success_rate` 为强制槽（RateSample 允许 `NoBaselineYet` 标记但须显式）。
- **来源防洗白**：证据类型不接收自由字符串数组；`ToolTraceEvidence.turn_id` 必须对应调用方已核实的真实 turn（构造期由调用方断言，门禁 brief 中列来源清单）；`EvidencePack::validate()` 拒绝把 episodes 文件路径/日记原文作为 fs_diff 或 trace 来源（路径黑名单 + 调用方契约）。这是约定层防护；结构层防护 = 门禁纯协议层对 episodes 存储零依赖边（§2 ①）。
- **预算**：每槽 ≤ 16 条、每条摘录 ≤ 400 字符、brief 证据段总 ≤ 12k 字符（超出构造期拒绝，防 prompt 膨胀/注入面）。
- `subject_digest = "sha256:v1:" + hex(sha256(subject_bytes))`（冻结）。

## 5. 结构与写入口（receipt 如何成为结构门）

### 5.1 分层（遵守 core-decomposition 守则）

- **纯协议层**：`src/crates/execution/agent-runtime/src/judge_gate/`——`types.rs`（GateRequest/EvidencePack/GateVerdict/ApprovedGateReceipt/RejectClass/ActionKind）、`redlines.rs`（frozen 表）、`brief.rs`（构建）、`verdict.rs`（解析）、`evidence.rs`（校验）。**不依赖 northhing-core、不依赖 episodes 存储**（CI 断言：`cargo tree -p northhing-agent-runtime` 对 northhing-core / episodes 零命中——五硬化 #4 的 Phase 0 版）。
- **core 适配层**：`src/crates/assembly/core/src/agentic/judge_gate/`——`runner.rs`（`JudgeRunner` trait + `SubagentJudgeRunner` 生产实现：经 `ConversationCoordinator::execute_subagent` 调 GateJudge）、`audit.rs`（审计 IO，PathManager）、`mod.rs` 编排 `evaluate()`。
- 测试：`JudgeRunner` 为注入 trait——**确定性 fake runner**（canned verdict / timeout / error / malformed），取消与超时路径全部可自动化测试（家规#4 满足，无 LLM 依赖）。

### 5.2 GateJudge 专用 agent（替代 v0.1 的 ReviewJudge 复用）

- 新隐藏 agent 定义 `GateJudge`（仿 `agents/definitions/review/review_specialists.rs` 的只读宏模式注册——复用的是 review_specialists 的只读宏机制，非 DeepReview 本身（DeepReview 是 orchestrator 角色、显式 read-write）；`hidden` + 只读工具集 + 注册表登记 `is_review`），prompt template 专用：红线裁判协议（§3 输出格式、逐槽评估要求、无权重定义权）。
- **注册触点（实施单必须四处同步，judge 复审已枚举）**：①`execution/agent-runtime/src/agents.rs` `builtin_agent_definition_specs()` 加 GateJudge（Hidden + 限制父 agent 为门禁调用方）；②`assembly/core/src/agentic/agents/registry/catalog.rs` `builtin_agent_factory()` 加 `"GateJudge"` 工厂项（否则被当 orphan 跳过）；③`registry/types.rs` `is_review_agent_entry()` matches! 硬编码加 agent id 常量；④`agents.rs` `default_model_id_for_builtin_agent()` match 穷举加项（继承 ReviewJudge 的 `"fast"`）。
- v0.1 用 `ReviewJudgeAgent`（其系统 prompt 语义是「校验 reviewer reports 一致性」）有 prompt 冲突/注入风险——judge 评审 Important #5，本版修正为专用 agent。注册即「复用 deep_review judge 的注册/只读/隐藏机制」，语义对齐计划 :61「复用 agentic/deep_review」；若用户要求字面复用 ReviewJudge agent，见 §10 拍板项 1。

### 5.3 Phase 0 封住的固化写入口（receipt 消费点）

| 写入口 | Phase 0 状态 |
|---|---|
| `skills/candidates/**` 的 **promote 写入**（候选 → `user_skills_dir()/<name>/`） | 本单落 `promote_candidate_skill(receipt, candidate_path)`：校验 receipt + 复制 + 审计；C4 正篇的生成器届时只能经此函数落地 |
| 红线表 / 门禁代码自身 | **无 ActionKind 变体可达**（`ActionKind` 不含 gate-self-modify——结构不可能）；治理走 §7 out-of-band |
| prompt 文件（agentic_mode 等）修改 | Phase 0 仅登记为后续入口（现状无程序化写 prompt 的功能，出现此类功能时必须接 receipt——写入 AGENTS.md 门禁节说明） |

`ActionKind` v1：`PromoteSkillCandidate`。新增变体 = 改门禁代码 = 走 §7。

### 5.4 I-NEG-2 不变量测试扩展

- 构造 `candidates/<name>/SKILL.md` → `SkillRegistry::refresh()` → 断言不可见（user root）。
- 追加：构造 `candidates/SKILL.md`（错误根）→ 测试**候选 writer 自身**永不写该路径（writer 的路径拼接单测：输出恒为 `candidates/<slugified-name>/SKILL.md`，拒绝空 name/`..`/绝对路径/分隔符）。
- project/remote root 与 symlink 面：本版如实标注「未覆盖」，列入 Phase 1 候选 writer 验收。

## 6. 审计（append-only，approve 绑定落盘）

- 位置：`PathManager.user_data_dir()/judge-gate/audit.jsonl`。已核实 `CleanupService` 仅清理 `temp_dir()`/`logs_dir()`/`cache_root()`（`infrastructure/storage/cleanup.rs:90-121`），`user_data_dir()` 不在其管辖——结构免疫普通日志清理。
- 轮转：**按日分片永不删除**（`audit-YYYYMMDD.jsonl`，分片命名用 UTC 日期（`Utc::now().format("%Y%m%d")`，避免跨时区歧义），entry `ts` 记 epoch_ms；追加打开当日片；无 rename、无删除）。I-NEG-4 满足。
- 写入纪律：`evaluate()` 流程 = runner 裁决 → verdict 解析 → **先写审计并 flush 成功** → 仅当 verdict=approve 且审计成功才构造 `ApprovedGateReceipt`；审计写失败 ⇒ `Rejected(AuditFailure)`（approve 不可能产生）。reject 条目同样落审计；reject 的审计失败仅 `error!` 日志（无法再降级，如实记录）。
- 条目：`{entry_id, ts, action_kind, subject_digest, evidence_summary(槽位计数+来源清单，不抄全文), verdict, rule_checks, reject_class?, judge_turn_id?, duration_ms}`。条目含 `entry_id` 供 receipt 关联。
- 并发：追加经 `tokio::sync::Mutex`（进程内串行）；跨进程追加语义 = O_APPEND 单行写入（单行 ≤ 8KB，Windows 下 append 写单行不交错——与 shell guard audit 同款约定）。
- 测试：追加→读回、日分片命名、并发 50 写无丢行（fake clock 或真时钟均可）、写失败（只读目录）→ `AuditFailure` 且无 receipt。

## 7. 红线治理（消除 v0.1 矛盾）

- I-NEG-3 不变：门禁**永不**批准对红线表/门禁代码的修改——`ActionKind` 无此变体，结构不可能（非策略拒绝）。
- 红线表/门禁代码的变更 = 普通代码修改，走**用户 root authority**：人工评审 + 用户显式拍板 + commit 记录；本次修改须在审计日志补一条人工条目（`judge-gate` CLI/手工脚本写入 `GovernanceOverride` 条目类型——Phase 0 提供 `append_governance_override()` pub(crate) 函数 + 单测；无 UI）。
- 红线表 v1 文本（与 v0.1 一致，§8），改动即走本流程。

## 8. 红线表 v1（状态层定义，frozen）

| 规则 | 状态层定义 |
|---|---|
| I-NEG-1 | 用户数据文件（配置、会话、记忆、episodes）在其原路径保持可访问且内容非空——固化动作不得导致其消失、移位或清空。 |
| I-NEG-2 | 未过门的固化产物不得出现在 agent 运行时可自动命中的位置（技能 loader 扫描位、prompt 注入面、配置读取面）。 |
| I-NEG-3 | 红线表与门禁执行代码不被固化动作自身修改（门禁不得批准改写自己）。 |
| I-NEG-4 | 审计日志只可追加——固化动作不得删除、截断或改写历史审计记录。 |

代码形态：`pub const REDLINE_TABLE: [RedlineRule; 4]`（agent-runtime 纯协议层）；单测钉长度/id 有序/文本非空。

## 9. API 与测试计划

```rust
// agent-runtime（纯协议，无 async/IO）
pub struct GateRequest { pub action_kind: ActionKind, pub subject: Vec<u8>, pub evidence: EvidencePack }
pub struct GateExecutionContext {
    pub workspace_path: Option<String>, pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>, pub timeout_seconds: Option<u64>,
    pub cancel_token: Option<CancellationToken>, pub audit_correlation_id: Option<String>,
}
// core（编排）
pub(crate) async fn evaluate(
    coordinator: &Arc<ConversationCoordinator>,
    request: GateRequest, ctx: GateExecutionContext,
    runner: &dyn JudgeRunner,           // 生产 = SubagentJudgeRunner；测试 = FakeJudgeRunner
) -> GateVerdict
// 写入口
pub(crate) async fn promote_candidate_skill(receipt: ApprovedGateReceipt, candidate_dir: &Path) -> NortHingResult<PathBuf>
```

测试（全部确定性，无 LLM/网络/sleep）：
1. verdict 解析矩阵：approve 全 pass / 任一 violation / 缺 rule / 重 rule / 未知 rule / 多余 rule / not_evaluated 状态 / 零块 / 多块 / 非 JSON / verdict 非法 / evidence_assessment 空 / 引用不存在编号 → 全 reject 且类别正确。
2. brief：含 4 红线逐字、证据编号清单、权重=0 指令、预算截断。
3. evidence 校验：四槽缺失/超预算/episode 路径来源/turn_id 空白 → 拒绝。
4. 红线表形态；ActionKind 无 self-modify 变体（编译期 + 反射测试）。
5. evaluate() 经 FakeJudgeRunner：approve→receipt 且字段绑定正确；runner timeout/cancel/error → JudgeUnavailable；审计失败 → AuditFailure 无 receipt。
6. 审计：§6 测试清单。
7. promote_candidate_skill：无 receipt/错 digest/已消费 receipt → 拒；正确 receipt → 落位 + 候选源保留 + 审计追加。
8. I-NEG-2 不变量测试（§5.4）。
9. 零依赖边守卫（agent-runtime 对 northhing-core / episodes 存储零依赖）：复用现有 `scripts/check-core-boundaries.mjs` 的 source 守卫规则追加 judge_gate 模块断言（与 CI 工具链一致，不新造 cargo tree 脚本）。
10. 验收命令（本设计可达成基线）：`cargo check --workspace`、`cargo check -p northhing-core --features product-full`、`cargo test -p northhing-agent-runtime`、`cargo test -p northhing-core --features product-full --lib agentic::judge_gate`。**环境敏感家族**（subagent_ports tests_cancel×2/tests_timeout/tests_error/tests_parent_chain/tests_concurrent 共 6 个，本机有 LLM 配置时稳定失败，已登记 tech-debt-ledger P2-7）不在本单验收范围，其结构性修复归 P2-7 测试基建单。
11. 手工实证（用户在场）：真实候选 promote 过门一次，审计/episode（judge turn 落盘，C5b-G2/G3 链路）可查。

## 10. 拍板记录（2026-07-22，全部照推荐通过）

1. GateJudge 专用 agent ✅（§5.2，含 4 注册触点）。
2. 红线表 v1 照 §8 逐字 frozen ✅。
3. 技能候选过门即默认启用 ✅（计划 :93 解释确认）。
4. 五硬化分期照 §9/§10.4 执行：#1 解析矩阵（canonical 违反集）+ #4 零依赖边守卫入 Phase 0；#2/#5 随 C4 正篇（Phase 1）；#3 不触发 ✅。
5. 审计位置 `user_data_dir()/judge-gate/` ✅。
6. 人类反馈槽允许 `Absent(封闭理由)` ✅。
