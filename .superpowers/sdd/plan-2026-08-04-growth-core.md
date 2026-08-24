# 成长核心（Growth Core）执行计划 v2（2026-08-04）

> v2 说明：v1（同日）基于设计稿原处方。本版并入用户 2026-08-04 的 7 项拍板，其中 3 项**推翻设计稿处方**（见 §0.2）。以本版为准。

设计输入：`docs/design/2026-07-23-self-cognition/memory-multi-agent-architecture.md`（C4 正篇）+ `docs/design/2026-07-25-judge-mom/memory-growth-design.md`（tracer 1/2/3）。

## 0. 用户拍板与设计稿冲突处置

### 0.1 拍板清单（2026-08-04）

| # | 决策 |
|---|---|
| D1 | 成长核心 = 顶层 `src/agentic` 独立 crate（确认修改 `AGENTS.md` 六层表） |
| D2 | judge-mom 混合形态：routine 评估进程内 LLM 单次调用；skill promote 走 subagent |
| D3 | 范围 G1 → G3 全做 |
| D4 | **记忆分库**：用户画像库 与 自我认知库 分离 |
| D5 | **双层权重（C 方案）**：话题权重（主）+ 条目分数（次），**话题权重 > 条目重要度** |
| D6 | **相对权重作废**：竞争组内占比挤压导致自然失效，不用硬作废标记；judge-mom 语义权重需加重设计 |
| D7 | **竞争认定 = 混合（C）**：管家 LLM 提议 → N 次一致证据门槛 → 生效后可回滚 + 全程审计 |
| D8 | **硬作废只留窄口子（B）**：仅当用户**显式否定**时硬作废；管家自身判断永远只能改权重 |
| D9 | 自我认知库：**agent 独占读写**，管家完全无权，不经评审 |
| D10 | 用户画像库：人类**只读** |
| D11 | 日志权限：人类只读 / agent 只写 / 管家读写（写=只能追加标注行） |

### 0.2 与设计稿冲突的处置（用户决策优先）

| 设计稿处方 | 本版处置 |
|---|---|
| `memory-growth-design.md` §3/§4：dream 判定过时 → 标 `status='superseded'`，检索层过滤 | **推翻**（D6/D8）。dream 不再判作废，改任"关系图园丁"；`status` 字段保留但只由 D8 的显式否定口子写 |
| §6 Tracer 2：judge-mom 只做"评估 + 路由 + 时机自学习"，FTS 近似去重 | **扩容**（D5/D6/D7）。judge-mom 主体是语义权重系统：抽话题 / 判竞争 / 定幅度 / 维护关系图；近似去重降为其中一个动作（合并加权，不新增条目） |
| C4 正篇：`identity.md` 由 agent 自主管理，facts 为单一记忆库 | **细化**（D4/D9）。自我认知升格为独立库（agent 独占），与外部记忆库物理隔离 |

**执行纪律**：合并 main 前必须回写这两份设计稿的对应节，标注被本轮决策取代（doc sync 硬规则）。

## 1. 现状核对（2026-08-04 实测锚点）

| 件 | 锚点 | 状态 |
|---|---|---|
| 记忆存储 | `assembly/core/src/service/agent_memory/memory_db.rs`（918 行） | ✅ FTS5 + facts + keyword_weights（含 `related_keywords` 字段，为话题关系图预留）+ fact_reviews + judge_mom KV |
| Tracer 1 蒸馏 | `agent_memory/distiller.rs`（408 行） | ✅ LLM 蒸馏 + 严格 JSON + 关键词回落 |
| Tracer 3 dream | `agent_memory/dream.rs`（340 行） | ✅ 但处方被 D6 推翻，需改造为园丁 |
| Tracer 2 judge-mom | `agent_memory/judge_memory.rs`（8 行 KV 壳） | ❌ 本轮主体工作量所在 |
| 调度状态 | `agentic/coordination/dialog_turn/turn_persist.rs:458-512` | ⚠️ 4 个裸字符串 KV，调度逻辑内联在持久化代码 |
| 成长 hook | `turn_persist.rs:310` / `:324` / `:590` / `:606` | ⚠️ 4 处散 hook 无统一入口 |
| 权重缺陷 | `memory_db.rs:584` `boost_keyword` **零生产调用方**；`turn_persist.rs:590` 每轮 `decay_all_weights(0.99,0.1)`；`memory_db.rs:432` `search_facts` 读权重排序 | 🔴 只衰减不提升 → 全量降到地板 0.1，排序信号失效 |
| 分库 | facts 单表，`fact_type ∈ user\|feedback\|project\|reference` 混存混检索混注入 | ❌ D4 要求物理分库 |
| 去重 | 仅精确文本去重 | ❌ 需改为"合并加权" |
| 日志 | `agentic/episodes/`：`append_episode` / `read_episodes` | ✅ 写读齐备，无消费方；权限矩阵未落地 |
| 技能门禁 | `agentic/judge_gate/mod.rs:72` `evaluate`、`:256` `promote_candidate_skill`、`runner.rs` `SubagentJudgeRunner`、`audit.rs` | ✅ 可直接复用，无候选送入 |
| 自我认知 | `agentic/identity.rs`（70 行）：`save_identity` **零调用方**；读取点 `agents/prompt_builder/system_prompt.rs:28-29` | 🔴 从未被写过；且当前是自由格式单文件，非"库" |
| 配置 | `service/config/memory.rs`：`MemoryConfig { distiller_enabled, distiller_model }` | ✅ 单一事实源，新字段加此处 |
| 边界脚本 | `rules/crate-layout.mjs:4/:38`、`rules/crate-rules.mjs:3`、`checker.mjs:325/:385/:398` | ⚠️ `:385` 只对 `src/crates/` 成员强制层路径 → 顶层 `src/agentic` 处弱约束区，须显式登记 + 补断言 |

## 2. 目标架构

### 2.1 两库分离（D4/D9/D10）

| | 外部记忆库（含用户画像） | 自我认知库 |
|---|---|---|
| 内容 | 用户偏好、用户对 agent 的反馈、项目动机、外部资源指针 | agent 对自己的认识：擅长/不擅长、在本项目的角色、反复犯的模式 |
| 写入方 | judge-mom（评审后写） | **仅 agent 自己** |
| 读取方 | agent（检索注入）+ judge-mom | 仅 agent |
| 人类 | 只读 | 无权 |
| judge-mom | 读写 | **完全无权（不读不写）** |
| 检索机制 | 双层权重 + 竞争组（条目可达千级，必须靠权重筛） | **不参与权重竞争**：小而稳定，全量注入 |

**归属推导（本计划裁定，供用户否决）**：`project` / `reference` 类记忆归**外部记忆库**。依据是权限边界按"谁能写"划分——这两类的写入方是 judge-mom，而 judge-mom 对自我认知库无权，故只能落外部库。
**`feedback` 的特殊路径**：用户对 agent 的反馈留在外部库（它是"用户如何评价"，属对外理解）；agent 读到后**自行决定**是否据此在自我认知库写一条——**这一步就是"成长"发生的时刻**，且天然不破坏权限矩阵。

### 2.2 双层权重（D5）

| 层 | 是什么 | 谁在动 | 动态范围 |
|---|---|---|---|
| 话题权重 | 话题（pnpm / 依赖安装 / 代码风格）的热度 | 提到即升、久不提衰减、竞争组内互挤 | 主导 |
| 条目分数 | 该条记忆本身的可信度/具体度 | 写入时定，之后基本不动 | 小幅微调 |

**初始参数（本计划设定值，需实测调整，落 crate AGENTS.md）**：
- 检索分 = `topic_weight_norm × (0.6 + 0.4 × entry_score)` —— 条目分数动态范围仅 0.4，确保话题权重主导（D5 的"话题权重 > 条目重要度"）
- 一条记忆挂 1..3 个话题，取其话题权重最大者（避免多标签刷分）
- 每轮衰减保持 `0.99`，地板 `0.1`（沿用现值）；单次 boost 上限 `+0.15`

### 2.3 竞争组与自然失效（D6/D7）

**技术前提（必须写进 brief，防实现者漏掉）**：光有权重不会产生作废——pnpm 涨不影响 npm，两者在系统中是独立话题。自然失效**必须**依赖竞争组：

- 竞争组 = 同一语义槽位的替代关系集合（如「包管理器」= {pnpm, npm, yarn}）
- **组内归一化**：组内权重和恒为 1，涨必有跌
- **压制门槛（初始值）**：组内占比 < `0.15` 且绝对权重 < `0.2` → 关联记忆检索分压至门槛下（想不起来）
- **可复活**：条目与关系全在，用户再提即回升 —— 这是相对权重方案对硬作废的核心优势，必须有测试证明"压制后再提及能恢复检索"
- **可解释**：压制原因是"组内占比 14%"这类可读数字，进审计

**竞争认定 = 混合（D7）**：
1. judge-mom LLM **提议**候选竞争关系（不直接生效）
2. 需 **N 次一致证据**（初始 `N=3`，同 workspace 内）才落库生效
3. 生效后仍**可回滚**：`fact_reviews` 记录 + 关系表带 evidence 计数与创建来源
4. 误判防线：竞争关系仅在同库、同 workspace 内生效；压制≠删除，最坏后果是"暂时想不起来"

**判权分离**：judge-mom 只能改权重与关系。**它无权作废任何记忆。**

### 2.4 硬作废窄口子（D8）

唯一硬作废通道 = **用户显式否定**（"别记这个"/"那条是错的"/明确推翻旧偏好）。
- 识别在 crate 内纯函数（关键词 + LLM 双确认，宁漏不误）
- 动作：`status='superseded'` + `superseded_by`（若有新事实指向）+ `fact_reviews` 记 `reviewer="user-negation"`
- 可回溯：条目不删，审计可查
- **管家/dream 走此通道 = 违规**，进边界规则拦截

### 2.5 dream 转型：关系图园丁（D6）

不再判定作废。新职责：
- 清理孤立话题（无关联记忆的僵尸话题）
- 合并同义话题（LLM 判定 + 证据门槛，同 D7 纪律）
- 长期沉底记忆移出热索引（冷存，**条目与状态不变**）
- 权重体检：检出跑飞（组内和 ≠ 1、越界值）并修正 + 报警

### 2.6 日志权限矩阵（D11）

| 角色 | 权限 | 结构保证 |
|---|---|---|
| 人类 | 只读 | 无写入 API 暴露到宿主层 |
| agent 主体 | **只写** | 写入 API 可见；读取 API 对主 agent prompt 路径不可见（边界规则拦） |
| judge-mom | 读 + **追加写** | 只能 append 标注行（如"已提炼技能 X"），不得改写既有行 |

### 2.7 crate 结构

```
src/agentic/                    ← northhing-agentic-growth（纯逻辑 + 端口）
  AGENTS.md                     层定位 / 权限矩阵 / 参数表 / 边界
  src/
    lib.rs        GrowthCore：on_turn_finalized / on_session_end（唯一入口，warn-only）
    ports.rs      ExternalMemoryStore · SelfCognitionStore · TopicStore · LlmPort
                  · JudgePort · EpisodeLog · Clock
    state.rs      GrowthState（schema_version=1 单 JSON blob）+ 旧 4 KV 迁移
    scheduler.rs  纯决策：decide(state, signals) -> Vec<GrowthAction>
    executor.rs   GrowthAction → 端口调用（唯一 IO 触点）
    distill/      蒸馏 prompt + 严格 JSON 解析 + 候选整形（自 core 迁入纯逻辑）
    topics/       话题抽取 · 权重升降 · 竞争组归一化 · 压制判定 · 关系图维护
    review/       judge-mom 评审：合并加权 · 路由 · 判决解析 · 竞争提议与证据累积
    negation.rs   用户显式否定识别（唯一硬作废入口）
    garden/       dream 园丁：孤立清理 · 同义合并 · 冷存 · 权重体检
    promote.rs    日志聚合 → SkillCandidate → JudgePort（subagent）
    selfcog.rs    自我认知库写入判定（agent 独占，管家无权）
```

core 侧 `assembly/core/src/agentic/growth_adapter/`（薄适配，唯一 IO 实现）：
`ExternalMemoryStore`/`TopicStore` → `MemoryDb`；`SelfCognitionStore` → 新库（见 T3）；`LlmPort` → `resolve_memory_llm_client` + timeout 15s；`JudgePort` → `SubagentJudgeRunner` + `promote_candidate_skill`；`EpisodeLog` → `episodes`；`Clock` → `SystemTime`。

**依赖方向**：growth crate 不依赖 `northhing-core`（进 `noCoreDependencyCrates`），只依赖 contracts + serde/serde_json/thiserror/async-trait/tracing。存储实现留 core（用户决策：先注入不迁移）；**crate 禁直接依赖 rusqlite**。
**决策/执行分离**：`scheduler::decide`、`topics::*`、`review::route`、`garden::*`、`negation::detect` 全为纯函数（无 IO/时钟/随机）；crate 自测用 fake ports，零磁盘零网络。

## 3. 分支与波次

- 分支 `feat/growth-core-0804`，worktree `E:\agent-project\.worktrees\growth-core-0804`。
- 基线 = 派发前实测 main HEAD（当前 `ae44334`，`fix/p1-security-0804` 可能先并 → **派发前 `git log` 复核，不信本行**）。
- G1 = T1..T7（地基：分库 + 骨架 + 收敛 + 话题层）；G2 = T8..T13（语义权重主体）；G3 = T14..T17（成长闭环）。
- G1 期末可选先并 main（行为等价 + 分库迁移，风险中低）。

## 4. G1 — 地基（T1..T7）

### T1 — crate 骨架 + 层表登记 [scaffolding]
新建 `src/agentic/{Cargo.toml, AGENTS.md, src/lib.rs}`（空壳 + 1 smoke 测试）。同 commit 登记 5 处：
1. 根 `Cargo.toml` members（`:2-33`）加 `"src/agentic"`
2. `crate-layout.mjs`：`crateLayoutLayerNames`（`:38`）加 `'growth'`；`crateLayoutRules`（`:4`）加 `{ crateName: 'agentic-growth', layer: 'growth', path: 'src/agentic' }`
3. `crate-rules.mjs`：`noCoreDependencyCrates`（`:3`）加 `'agentic-growth'`
4. `AGENTS.md` 层表（`:21-28`）新增成长核心层 + Boundary rules（`:30-37`）加"成长核心只依赖 contracts，宿主经端口注入"；同步 `AGENTS-CN.md`
5. `docs/status/surfaces.md` 加条目

补断言（因 `checker.mjs:385` 不覆盖非 `src/crates/` 成员）：所有非 `src/crates/` workspace 成员必须存在于 `crateLayoutRules`，否则 fail。
验证：`cargo test -p northhing-agentic-growth` + `node scripts/check-core-boundaries.mjs` + `cargo check -p northhing`。

### T2 — ports + 状态迁移 [contracts]
`ports.rs` 7 个 trait（§2.7）；`state.rs`：`GrowthState`（含 distill 统计 / dream 游标 / judge 统计 / 时机偏好 / 竞争证据游标）+ 旧 4 KV 迁移（首读无 blob → 读旧键填充 → 写 blob，旧键保留不删）+ 未知 schema_version 回落 + warn。core 侧适配 state 读写与 Clock，**不改 `turn_persist.rs`**。
测试：迁移幂等（跑两次同结果）、旧键保留、无旧键默认值、未知版本回落、fake store 往返。

### T3 — 记忆分库 [architecture，D4/D9/D10]

> **拆分（编排者 2026-08-05）**：T3 拆为 **T3a（存储 + 一次性迁移，零提示词改动，已完成 `258d2ea`+`39fadea`）** 与 **T3b（注入位置分离 + 权限门控，待做）**。理由：原范围同时含用户可见数据迁移与 agent 行为改动，捆一起回滚语义不明。
> T3a 已落地：`self_cognition` 表（全局、无 workspace 列）、独立访问模块 `self_cognition.rs`、`SelfCognitionDbStore` 端口实现、`identity.md` 一次性非破坏迁移（固定主键 `migration:identity-md` + `INSERT OR IGNORE`）。
> **T3b 已完成（`9f261cd`）**：注入优先级链（store → identity.md 回落 → persona）、2000 字符预算与溢出策略（保最旧 + 最新填充）、dream.rs 内的 D9 密集路径负测试。**T3 整体完成。**
> **T3b 原剩余范围（已闭环，勿重做）**：注入位置分离（自我认知 → 系统提示"关于我"块，`system_prompt.rs`；外部记忆 → 对话上下文"关于用户/项目"块，`auto_memory` 现有路径）、多笔记渲染策略与预算、密集路径不可见的门控与测试、`prompt_injection` 前后对照。

- 外部记忆库 = 现 facts 表（保留 `fact_type`），自我认知库 = **新独立存储**（同 DB 独立表 + 独立访问模块，或独立文件——由 implementer 就现有存储形态给方案并在 report 论证；硬要求是**物理隔离 + 管家侧不可见**）。
- 现有 `identity.md` 自由文本迁入自我认知库（保留原文件为兼容读，注入路径不变）。
- 注入位置分离：外部记忆 → 对话上下文"关于用户/项目"块（沿用 `auto_memory` 现路径）；自我认知 → 系统提示"关于我"块（沿用 `system_prompt.rs:28-29` 位置）。
- 权限落地：`SelfCognitionStore` 在 crate 内**不暴露给 review/topics/garden 模块**（管家路径），仅 `selfcog.rs` 可见；core 适配同构隔离。
- 测试：管家路径无法触达自我认知库（编译期不可见 + 一条负向测试）；分库后 prompt 两块内容各自正确；迁移幂等。
- 验证：+ `cargo test -p northhing-core --features product-full prompt_injection`

### T4 — 单点 hook 收敛（行为等价）[refactor]
把 `turn_persist.rs:458-512` 的暂停门/计数/自暂停阈值（20 轮 0 命中）与 `dream.rs:52-62` 的 24h 间隔判断逐字搬为纯函数 `decide`；`turn_persist` 4 处 hook（`:310`/`:324`/`:590`/`:606`）收敛为一个 `GrowthCore::on_turn_finalized`；episode 与 facts 先后顺序不变；`load_last_assistant_text`（`:612`）截 500 字符行为不变；warn-only 语义保持。
测试：调度决策表（paused / 未到 24h / 命中率归零触发暂停）+ core 集成测试断言一次 finalize 仍产出 facts + episode。

> **T4c 裁定（用户拍板，2026-08-07）**：不单独实施剩余的统一门面，**并入 T12**。只读差距报告确认：T4a/T4b/R-7/R-2/T6a 已拿到判定纯函数化、成长状态收敛、facts 门禁与话题升降等实质收益；当前只剩形式包装，而 T12 必须在同一活跃回合路径把 dream 拆到独立 `on_session_end`，此刻包装会造成二次改动。证据：`.superpowers/sdd/task-t4c-gap-report.md`。T12 的防遗忘验收见其任务节。

### T5 — distiller / dream 纯逻辑迁入 [refactor] — ✅ 已完成（拆为 T5a / T5b / T5c）
迁入 crate：蒸馏 prompt、`<user_message>` 防注入包裹、严格 JSON + 白名单 + text ≤300 截断、关键词回落；dream 的候选筛选与批量判决解析（`strip_json_fence`、越界/未知 action 跳过、reason ≤200）。core 侧收缩为薄适配（模型解析：`distiller_model` → `config.ai.models` 匹配 → `get_client_resolved`，失败回落 fast → primary）。`dream.rs:289-339` 6 个解析测试逐条平移。**不改判定语义**（园丁改造在 T12）。

> **执行裁定（编排者，2026-08-06）**——本节按实际拆成三个提交，均双 PASS：
> - **T5a**（`71df0dd`）distiller 主体：`distill/prompt.rs`(128) + `distill/parse.rs`(389)，`distiller.rs` 656→416。crate 侧产出**中性** `DistilledFact` 与 `DistillParseOutcome{facts,keywords,was_empty_array,parse_error}`；uuid / `SystemTime::now()` / provenance / `schema_version` 全留 host，`warn!` 由 `parse_error` 驱动在 host 侧发。prompt 的 system 段经编排者独立复算为 **2886 字节逐字一致**。
> - **T5b**（`8b64aa8`）dream 批量判决解析：`review/verdict.rs`(161) 的 `parse_verdicts(json, item_count, allowed_actions)` + `llm_output.rs`(48) 唯一 `strip_json_fence`（原有**两份**副本合一）。**动作白名单参数化**，crate 内零 dream 词汇 —— 这是 crate `AGENTS.md` §3 的硬要求，非风格选择。6 个解析测试平移（fixture 动作名改中性，真实 `keep`/`supersede` 词汇由新增的 core 端到端测试覆盖）。
> - **T5c**（`2e986ce`）§5.4-C 的 `auto_memory` 指引四条，见 §12 尾部。
>
> **⚠️ 本节的"dream 候选筛选"未做，已并入 T12。** 理由：① 候选筛选依赖 core 的 `Fact` 类型，需与 T5a 同等的中性化改造；② T12 要整体重写筛选语义（dream 变园丁、四动作、去 supersede），此刻迁移等于搬两次。T12 验收须覆盖它。
>
> **给 T12 的现成资产**：`parse_verdicts` 的白名单是入参，园丁改造时只换实参即可，不必重写解析器；`review/verdict.rs` 与 `llm_output.rs` 同样是 T9（judge-mom 判决解析）的复用点。

### T6 — 话题层 + 双层打分 [feature + 修缺陷，D5]
- 话题抽取（crate 内纯函数，不引分词依赖：空白/标点切 + 停用词 + 长度过滤 + 上限 3 个，**必须含 CJK 测试固定行为**）
- 话题权重升降接线：修 🔴 缺陷——`boost_keyword`（现零调用）接入写入/命中路径；`decay_all_weights` 保留 per-turn
- 双层打分：`topic_weight_norm × (0.6 + 0.4 × entry_score)`，参数进 crate AGENTS.md
- 风险：改变 prompt 注入排序 → report 必须贴 `prompt_injection` 前后对照
测试：boost/decay 轨迹表、多话题取最大值、参数边界、CJK 抽取。

### T7 — 边界规则 + memory_db 拆分 [hygiene]

> **T3a 转入的硬要求（Important，勿漏）**：`memory_db.rs:47-68` 的 `pub(crate) fn conn_locked()` 暴露原始 `MutexGuard<Connection>`，实质扩大 D9 权限面——改动前 `dream.rs`/`judge_memory.rs` 读不到 `self_cognition` 表，改动后同 crate 内拿裸连接即可读，而 D9 要求 judge-mom 连读都不行。同一 crate 内可见性无法隔离（两者是同模块树兄弟），**必须在 `forbidden-rules.mjs` 加规则：dream / judge_memory / review 路径禁止出现 `\bconn_locked\b`**。
> **用户裁定（2026-08-05）**：800 行上限的目的是防代码腐化；memory 这块用户几乎每天自用，腐化风险低 → **`memory_db.rs` 的拆分不作为阻塞项**，优先级低于边界规则与功能推进。

- 权限矩阵结构化进 `forbidden-rules.mjs`：`prompt_builder/**` 禁出现成长状态符号；`growth_adapter/**` 禁 prompt 构造符号；**管家路径禁出现自我认知库符号**；~~**dream/园丁与 review 路径禁出现 `supersede` 符号**~~（见下方 T7a 拆分裁定，此条转 T12）
- `memory_db.rs` 918 行按域拆分（facts / topics+keyword_weights / judge_mom / reviews / migration），入口 <250 行，测试同步分组

> **编排者拆分裁定（2026-08-06）——T7 拆为 T7a / T7b**
> - **T7a（边界规则）**：三组规则 = ① 管家路径（`dream.rs`/`judge_memory.rs`）禁自我认知符号；② 同路径禁 `\bconn_locked\b`（上方 T3a 转入的硬要求）；③ `prompt_builder/**` 禁成长状态写符号。附带把 `dream.rs` 的 D9 负测试移入独立文件（`#[path]` 子模块，保持对私有 `build_dream_messages` 的访问），以免规则被整文件 `allowPaths` 掏空。
> - **T7b（`memory_db.rs` 拆分）**：按上方用户裁定，不阻塞，最后做。
> - **`supersede` 规则延后到 T12（已核实的阻塞事实）**：`dream.rs:156` 至今在生产调 `db.supersede_fact(...)`（`:155` 的 `"supersede"` 判决分支），`:165` 写 `action: "supersede"` 审计行，`:211`/`:214`/`:218` 的 dream 提示词里也写着这个动词。T7a 若加此规则会**立刻变红**，或逼实现者越界去改 T12 的行为。故 T12 的验收追加一条：**移除 dream 硬作废之后，同一提交内补上"dream/园丁与 review 路径禁 `supersede`"的边界规则**，并证明该规则会触发。
> - **`prompt_builder` 只读的既有例外（如实记录，不掩盖）**：T3b 的 `system_prompt.rs:317` 以 `let _store = init_self_cognition_store(&db);` 触发一次幂等的 identity 迁移——返回值被丢弃，唯一效果就是**写**。裁定：保留惰性迁移（幂等、自愈），改为把规则写精确 + 显式 `allowPaths` 记档；迁移触发点是否上移到启动引导，另案再议，不在 T7a 范围。

## 5. G2 — 语义权重主体（T8..T13）

### T8 — 竞争组与自然失效 [feature，D6]
竞争组表（组 id / 成员话题 / 归一化权重 / 证据计数 / 来源 / 创建与更新时间）；组内归一化（和恒为 1）；压制判定（占比 < 0.15 且绝对值 < 0.2 → 检索分压至门槛下）；**可复活**路径。
测试（硬要求）：涨必有跌、和恒为 1、压制后再提及恢复检索、单次 boost 上限、越界钳制、0 除。

> **T8 参数裁定（用户拍板，2026-08-07）**：T6a 已将 `keyword_weights.weight` 的有效域与衰减地板固定为 `[1.0, 5.0]`，因此原文的绝对阈值 `<0.2` 在当前系统中永远不可达。压制语义改为：**组内占比严格 `<0.15` 且现有话题权重处于冷基线 `<=1.0`**。不新增第二套活跃度，不恢复 `0.1` 地板；`1.0` 必须集中为命名常量并登记，恰等基线是冷态、严格高于 `1.0` 的重复提及立即具备复活信号。原文 `<0.2` 仅作为历史设计冲突留痕，不再是 T8 验收值。

### T9 — 竞争认定（混合）[feature，D7]
judge-mom LLM 提议 → 证据累积（初始 N=3，同库同 workspace）→ 生效 → 可回滚；全程写 `fact_reviews`（`reviewer="judge-mom"`，action 含 `propose_competition`/`confirm_competition`/`rollback_competition`）。坏 JSON → 零动作。
测试：证据不足不生效、达阈值生效、回滚恢复、跨 workspace 不串。

> **T9 裁定（用户拍板，2026-08-07，预检矛盾两条）**：
> 1. **跨组收敛 = confirm 时强制单归属**：确认新组时，其成员从其它既有组中摘除（被摘组重归一化 + 各自写审计行；摘除后不足 2 成员的组解散），「一个 topic 至多一个组」从此成为系统不变量。T8 的写侧 first-group / 读侧 max-share 退化为不可达防御，保留不删。不采用"写侧更新所有组"（热路径写放大、竞争语义稀释）。
> 2. **证据按 workspace 隔离计数，确认后组全局生效**：pending 证据按 workspace 分键（judge_mom KV），不同 workspace 的提议互不累计（「跨 workspace 不串」落在证据阶段）；确认后的组不写 workspace_key，全局生效，与 T8 已交付的全局 `competition_groups` 表和全局 `keyword_weights` 一致。此条是对 D7「同 workspace 内生效」字面的追认式偏离——全局生效实质已在 T8 交付并经两轮审查。证据计数不解析 fact_reviews（reason 自由文本脆弱），fact_reviews 仅作审计痕。

### T10 — 合并加权取代新增条目 [feature，D5/D6]
近似检索命中（FTS）→ 合并：`touch_fact` + 话题 boost + 条目分数取高者，**不新增条目**；阈值与 top-k 为纯函数参数并记档。
测试：近义输入不增条目但权重上升；差异足够大的输入正常新增。

### T11 — 用户显式否定（唯一硬作废）[feature，D8]
`negation.rs`：关键词 + LLM 双确认（宁漏不误）→ `status='superseded'` + `superseded_by` + `fact_reviews(reviewer="user-negation")`；条目不删。
测试：显式否定生效、模糊表述不触发、审计行完整、管家路径调用不到此入口（负向）。

### T12 — dream 转园丁 [refactor + feature，D6]
移除作废判定；改为孤立话题清理、同义话题合并（LLM + 证据门槛，同 T9 纪律）、长期沉底移出热索引（条目/状态不变）、权重体检（组内和 ≠ 1 / 越界 → 修正 + warn）。
测试：园丁四动作各一条 + "园丁绝不产生 supersede"负向测试。

**T4c 转入（用户拍板，2026-08-07，勿漏）**：
1. 本提交必须落地单一回合收口门面（`on_turn_finalized` 或等价命名），统一编排 episode → facts；保持 episode 先于 facts、assistant 文本 500 字符截断、warn-only、`SessionKind` 主会话门禁，并用集成测试证明一次 finalize 不丢失也不重复写 episode/facts。
2. 园丁必须由有生产调用方的 `on_session_end`（或等价独立入口）触发，与本回合蒸馏是否产出 facts 无关；不得同时保留回合内 dream/garden 触发造成双跑。
3. `dream_last_sweep_at` 旧键迁入 `GrowthState.garden`，生产只保留一个真相来源；crate 的 `should_run_garden_sweep` / `record_garden_sweep` 不再是仅测试调用的死接线。
4. 原 T5b 转入的候选筛选三参数以中性输入类型迁入 crate 并登记参数表；本节后续条款继续适用。
**T7a 转入（勿漏）**：本任务移除 dream 硬作废后，**同一提交内**把"dream/园丁与 review 路径禁出现 `supersede` 符号"规则加进 `forbidden-rules.mjs`（D8 唯一入口 `negation.rs`），并按 T7a 的标准逐条证明规则会触发（临时植入违规 → checker 报错 → 还原）。规则在 T7a 时期无法落地的原因见 §4 T7 拆分裁定。

**T5b 转入（勿漏）**：**dream 的候选筛选迁入 crate 属本任务**（T5b 故意未做，理由见 §4 T5 节裁定）。当前筛选逻辑在 `dream.rs` 的 `run_dream_sweep` 内联，依赖 core 的 `Fact` 类型与 `STALE_THRESHOLD_MS`(30d) / `MAX_STALE_FACTS`(20) / `DREAM_KEEP_EXEMPTION_DAYS`(7) 三个参数。迁移时须按 T5a 的做法给 crate 定**中性输入类型**（不得让 crate 看见 `Fact`），三个参数登记进 crate `AGENTS.md` §4，并把筛选写成接收"当前时刻"入参的纯函数（不得在 crate 内调 `SystemTime::now()`）。

**T5b 留下的现成资产（不要重写）**：判决解析已是通用件 `review::verdict::parse_verdicts(json, item_count, allowed_actions)`，园丁四动作只需传新的白名单实参；`llm_output::strip_json_fence` 是全 crate 唯一副本，勿再造第三份。

### T13 — 时机自学习 [feature]
纯函数 `tune(prefs, stats) -> TimingPrefs`：命中率高→提频，低→降频并逼近自暂停阈值；冷启动前 N 轮固定参数；`background_every_n_turns ∈ [1,50]` 硬钳制。
测试：命中率序列 → 参数轨迹表、上下界、冷启动不调参。

## 6. G3 — 成长闭环（T14..T17）

### T14 — 日志权限矩阵落地 [architecture，D11]
agent 主体只写（读取 API 对主 agent prompt 路径不可见，边界规则拦）；judge-mom 读 + 仅追加标注行（不得改写既有行）；人类只读（无写入 API 暴露到宿主）。
测试：追加标注不改旧行、主 agent 路径无读取通道（负向 + 边界脚本）。

### T15 — 日志 → 技能候选 [feature]
纯函数识别"重复失败 + 稳定修复"（同工具/同错误签名 ≥K 次且末次成功）→ `SkillCandidate { title, trigger, steps, evidence: Vec<turn_id> }`；生成后回写日志标注（T14 通道）。
crate AGENTS.md 写明授权边界：管家读日志是 D11 授权；**禁止把日志内容回灌进主 agent prompt**。

### T16 — promote 接线（subagent 门禁）[feature]
`JudgePort` → `SubagentJudgeRunner`；通过后 `promote_candidate_skill`（`judge_gate/mod.rs:256`），receipt + audit + 四红线全程复用现有门禁。配置门 `skill_promote_enabled` **默认 false**（subagent 成本）。
测试：FakeJudgeRunner 批准/驳回两路径；默认关闭时零 subagent 调用。

### T17 — 自我认知库写入（agent 独占）[feature，需 flag]
成长时刻判定（如"用户反馈累计确认某模式"、"技能首次固化"）→ agent 自己在自我认知库写一条 → 影响系统提示"关于我"块。
- **backbone invariant 合规**：内容进所有 system prompt = 行为边界改动 → flag flip + 集成测试。配置门 `identity_self_edit_enabled` **默认 false**
- 权限：写入路径只在 `selfcog.rs`；无用户侧编辑入口；管家无权（编译期不可见）；每次改写 append-only 审计（复用 `judge_gate/audit.rs` 模式）
- 派发前先 grep 复核 desktop 侧无写入口（现状 `save_identity` 零调用方，符合）

## 7. 全局约束（逐字进每个 brief）

- 成长路径**永远 warn-only**：失败只 `tracing::warn!`，绝不向 `turn_persist` 传播、绝不阻塞主流程。
- **judge-mom 无作废权**：唯一硬作废入口是 `negation.rs`（D8）；园丁/评审路径出现 `supersede` = 违规（边界脚本拦）。
- **管家对自我认知库无权**（D9）：编译期不可见 + 负向测试 + 边界规则三重保证。
- 权重系统三道闸：组内归一化、单次 boost 上限、越界钳制；所有参数集中在 crate 常量并记入 crate AGENTS.md（禁散落魔法数）。
- LLM 输出不可信：严格 JSON + 字段白名单 + 长度截断（text ≤300 / reason ≤200）+ 用户内容包 `<user_message>`、指令只认 system。
- 配置单一事实源 = core `GlobalConfig`（`service/config/memory.rs`）；禁第二份运行时可读配置。
- 决策纯函数、IO 只在 executor/adapter；crate 自测零磁盘零网络。
- 生产 `.rs` < 800 行；>1000 必须拆或带 `// allow-god-file` 理由。
- 日志 English-only、无 emoji（gemini-36-flash 有 emoji 惯性前科 → 交付后机械扫描）。
- **不裸 `cargo fmt`**（两次污染前科）；用 `pnpm run fmt:rs` 或手工对齐。
- cargo 命令带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`；core 测试必带 `--features product-full`。
- 远程 workspace：读侧 query-aware 注入已跳过远程，写侧沿用现状（设计稿 M4 已知限制），本轮不扩大。
- implementer 只 commit 范围内文件；crate 结构变动同 commit 更新 `docs/status/surfaces.md`。
- Coding curfew：03:00 后不派实现单。

## 8. 验证矩阵

| 面 | 命令 |
|---|---|
| 新 crate | `cargo test -p northhing-agentic-growth` |
| core 接线 | `cargo test -p northhing-core --features product-full agent_memory` |
| prompt 注入回归 | `cargo test -p northhing-core --features product-full prompt_injection` |
| 边界 | `node scripts/check-core-boundaries.mjs` |
| 桌面编译面 | `cargo check -p northhing` |

`cargo check --workspace` 被上游 embed-resource 3.0.11（webdriver→tauri 链）阻断（见 `plan-2026-08-04-backend-followups.md` §5），按 crate 验 + 交 CI。基线：core 1134/1134（Task 9 后）。

## 9. 模型分配（用户长期指令：k3 系不做 implementer）

| 任务 | implementer |
|---|---|
| T1, T2, T7 | `gemini-36-flash`（机械/登记/转录；交付后扫 emoji） |
| T4, T5, T12 | `glm-5.2`（等价重构 + 迁移） |
| T3, T6, T8, T9, T10, T11 | `gemini-31-pro`（架构面 + 判定密集 + 影响 prompt 排序） |
| T13, T14, T15, T16, T17 | `gemini-31-pro` / `step-explore`（step-explore 须写"文件一次写完"并收工验文件，截断前科） |
| 每单 reviewer | `minimax-cn-coding-plan/MiniMax-M3` |
| 期终审 / 分支终审 | `glm-5.2`（未参与单任务审查） |

ark provider 本环境不可解析（`ark/kimi-k3` 派发失败前科），一律走 volcengine 线。

## 10. 风险与开放项

| # | 风险 | 处置 |
|---|---|---|
| R1 | `src/agentic` 在层表外，`checker.mjs:385` 不覆盖 → 弱约束 | T1 补断言 |
| R2 | T6/T8/T10 改变检索排序 → 影响所有 prompt 注入内容 | 每单必跑 `prompt_injection`，report 贴前后对照 |
| R3 | **权重系统跑飞**（竞争组误判 / 权重震荡 / 全部沉底） | 三道闸 + 园丁权重体检 + 压制可复活 + 全程审计可回滚；T8/T9 测试为硬门槛 |
| R4 | 竞争关系 LLM 误判导致"想不起来" | D7 混合认定（N=3 证据 + 可回滚）；压制≠删除，最坏后果可恢复 |
| R5 | T17 自我认知进 system prompt = 行为边界改动 | 默认 flag off + 集成测试（backbone invariant） |
| R6 | T16 subagent 成本 | 默认 flag off |
| R7 | 存储仍在 core（先注入不迁移） | 记账：G3 收口后重评；期间 crate 禁依赖 rusqlite |
| R8 | 基线漂移（`fix/p1-security-0804` 未并） | 派发前 `git log` 实测 |
| R9 | 设计稿两节被本轮推翻 | 并 main 前回写设计稿（doc sync 硬规则） |
| R10 | `project`/`reference` 归属为本计划裁定（§2.1） | 已在 §2.1 标明推导依据，用户可否决；否决则 T3 需调整分库维度 |

## 11. 完成定义

- 每任务：双判决（spec + quality）通过 → ledger 追加 `Task N: complete (commits <base7>..<head7>, review clean)`。
- G1 期末：T1-T7 全绿 + 行为等价证据（facts/episode/dream 顺序不变）+ 分库后两块 prompt 正确 + 话题权重不再单调衰减的测试证据 → 期终审 → 可选并 main。
- G2 期末：权重系统三道闸测试全绿 + "园丁无作废权""管家无自我认知权限"两条负向测试通过 + 压制可复活证据。
- G3 期末：分支终审双 PASS → `--no-ff` 并 main → 回归扫（core ≥1134/1134）→ 回写 `AGENTS.md` 层表 / `surfaces.md` / 两份设计稿（标注被本轮决策取代）→ handoff。

---

## 12. Codex 对照修订（2026-08-05，用户提供的架构评审）

来源：用户拿 Codex 记忆架构做对照评审。**不改任何已合入代码**，全部是未启动任务的 brief 修订。已验证为"设计成立、不必抄"的部分：D5 提及驱动（northing 记忆的目的是"理解用户"，用户亲口再提即 ground truth；Codex 的 citation 反馈服务于"agent 的程序性知识"，目的不同机制不必同）、D6/D8 supersede 不删可复活、D9/D10/D11 权限矩阵与 `<user_message>` 注入防御（Codex 无对应物，我们更严）、workspace `memory/` 的渐进披露。

### R-1 → T12：园丁必须**产出**整合物，不只维护（最高优先）

缺口：目标架构三层（原子 facts → 话题权重 → 注入 top-5）里**没有任何组件产出蒸馏过的常驻摘要**。§A 的"1000 token 原始 facts 排序注入"在条目上百后必然退化——排序无法解决"稳定背景与当前 query 无关但必须始终生效"这一类需求。D4 的"用户画像库"目前只是 facts 仓库，缺画像本身。

修订：T12 园丁加**第五个动作**——从外部记忆库蒸馏出「用户画像摘要」，版本化（首行版本标记）、常驻注入、长度设上限，原始 facts 降为按需检索层。权限继承 D10：**人类只读**。

### R-2 → T4 / T13：维护生命周期与写入解耦；暂停不得是锁存终态

缺口：① 园丁挂在蒸馏成功的 early-return 之后，蒸馏没产出就永远不维护；② `distiller_paused` 全仓**无置回路径**（已实测），自暂停是无恢复的锁存；T13 的 tune 只"逼近阈值"，救不回来。

修订：① T4 brief 明确——园丁动作由 `on_session_end` **独立触发**，与蒸馏成败无关（该入口在本计划 §130 行 `GrowthCore` 签名里已声明，至今零调用方）；② T13 把 `paused` 建模为**频率 = 0 的可调参数**而非布尔锁存，命中率回升即自动恢复；失败走 retry backoff，永不永久停。

### R-3 → T15：技能候选签名太窄，漏掉最高频的一类

缺口：现签名只认"重复失败 + 稳定修复"（同工具 / 同错误签名 ≥K 次且末次成功）。**成功流程的重复也是技能**，且更高频。

修订：签名扩为「重复失败+稳定修复 ∪ **重复成功流程**（同工具序列 ≥K 次且全成功）」。`SkillCandidate` 结构借 SKILL.md 骨架：触发条件 / 输入 / 步骤 / 验证清单 / 陷阱与修复——同时给 T16 的 subagent judge 一个完整的评审对象。

### R-4 → 蒸馏器 + T6b：蒸馏顺手产出 keywords 并保全原话（零新增 LLM 成本，**建议提前**）

缺口：T6 的话题抽取是纯函数（空白切分 + 停用词），CJK 质量存疑；而 distiller 已经在调 LLM，白拿的结构化信号没拿。且转述会摧毁 FTS 召回。

修订：① distiller 的 JSON schema 加 `keywords: [..]`（同一次调用，白名单校验）；② T6 话题抽取**优先消费 LLM keywords**，纯函数切分降为回落；③ 蒸馏 prompt 加硬规则"保留用户原话关键短语 / 报错串 / 命令原文，可 grep"。

价值：这一条直接救活读侧 `search_facts`（`memory_db.rs:531-545` 的 keyword fold）里的 keyword 因子，**比等 judge-mom 主体早两个 wave**；也正好覆盖 T6a 遗留的 I-1（含连接符话题只是条件性命中）。

### R-5 → T10：去重门必须前置于 DB insert

缺口：现状是 DB 先写（按 id `INSERT OR IGNORE`，每次蒸馏都是新 UUID，等于永不去重）→ JSONL 后 dedup（只守护 JSONL append）→ **DB 积累文本重复且 `get_facts` 全量注入**。

修订：T10 brief 显式写明 dedup gate **前置于 DB 写入**，并带测试「同文本二次蒸馏不产生新行」。

### R-6 → T13：配额余量进 decide 信号集

缺口：judge-mom routine 评估（D2 进程内 LLM）+ 园丁 + T9 竞争提议会让记忆侧 LLM 调用量倍增，而 tune 的输入只有命中率。

修订：T13 的 `decide` 信号集加**配额余量**门控（Codex 用 `min_rate_limit_remaining_percent ≥ 25` 门控记忆任务）。

### R-7 → T4：蒸馏输入收敛为主对话用户轮次（**安全**，一行门禁）

缺口：subagent turn 的 `user_input`（子任务 brief）现在**也会被蒸馏**。brief 不是用户的话，且携带外部内容进未来 prompt，是自伤式注入向量。已实测：`turn_persist.rs:432` 的 `append_facts_entry(_agent_type: &str)` 参数带下划线前缀，**当前被完全忽略**。

修订：蒸馏入口加门禁，只接受主对话用户轮次。⚠️ **实现要点（已实测，勿按 `agent_type` 判定）**：`agent_type` 是人格名（`agentic` / `coder-lc` 等），不是主/子标记。可靠信号是 `SessionMetadata.parent_session_id`（`agentic/core/session.rs:103`）或 `created_by == Some("session-<parent>")`（`so_dispatch.rs:45`、`subagent_ports.rs:24`）。门禁须带负向测试。

### D12（新裁定）：重复证据晋升条目的**证据强度**，不动话题权重

问题（Codex 对照出）：§2.2 定"条目分数写入时定、之后基本不动"，重复提及只动话题权重 → 单次推断的 fact 与多次证实的 fact 在同热度话题下**不可区分**。

裁定：**采纳选项 b**，且实现上不引入新项——A3 已落地的公式 `score = tw * (0.6 + 0.4 * es)` 本身就预留了刚好 0.4 的动态范围。做法：T10 合并加权时，用 touch 次数小幅提升该条目的 `evidence_strength`（上限封顶），**永不触碰 `tw`**。由此：
- D5 话题主导天然不破（`TOPIC_DOMINANCE_RATIO = 1/0.6` 由公式结构保证，无需新增约束）；
- 动态范围严格压在 0.4 内，符合用户提出的边界；
- 认识论语义正确：被反复证实的记忆比单次推断的更可信，但**再可信也压不过话题热度**。

T10 brief 须带测试：同话题下多次证实条目排在单次推断条目之前；且高热话题的单次推断条目仍排在低热话题的多次证实条目之前（话题主导不被 es 反超）。

### D13（新裁定，2026-08-05）：不引入写时隐私分类

针对调研报告 Q9 / §5.4-D。用户拍板 **选项 a：不做，保持现状**。

依据：northing 本地优先、数据用户自有，Anthropic 那套的法律与多租户风险面不适用。故 §5.4-A 的 distiller prompt 补丁**下发时不得加入敏感类目负面清单**，也不引入任何分类器代码。

### D14（新裁定，2026-08-05）：§5.4-C3 双写分工话术须改写为"如实版"

调研报告 §5.4-C3 原文含一条 **agent 无法执行**的指令：「内容重叠时以文件层为准，不要为已存在的 fact 建文件」。因为注入进 prompt 的 `# Remembered facts` 只是**按当前 query 检索出的 top-5 子集**，不是全库，agent 无从判断某条 fact 是否已存在。写不可执行的指令比不写更糟——模型会**假装遵守**（幻觉式合规）。

下发时改为如实版：

```text
The `# Remembered facts` block is a query-relevant subset maintained by the
system. Treat it as already known and do not copy its content into your own
memory files. You are not expected to deduplicate against facts you cannot
see - the system owns that.
```

### D15（裁定，2026-08-05）：§5.4-A 落地时须让"记忆捕获量下降"可观测

§5.4-A 引入最小信号门与 "no-op is allowed and preferred"，**记忆捕获量会下降**（这是设计意图：宁缺毋滥）。但用户会主观感到"它记得少了"。故 T5 下发该 prompt 补丁时，必须同时要求记一条统计日志（本轮判定无可记内容），使下降**可观测、可回调**，而不是闷着变。

### 已提前实施的修订（不必再走原任务）

| 修订 | 原落点 | 实际实施 | commit |
|---|---|---|---|
| R-7 蒸馏排除 subagent turn（§4.7 / Q7） | T4 | 已单独实施并通过双判决 | `27c9738..6365cf5` |
| R-2 自暂停恢复（§4.2 后半 / Q2 的 paused 部分） | T13 | 已单独实施并通过双判决；**园丁触发解耦仍留 T12** | `d1d6d92` |
| S-1 拆分两个贴顶文件（非调研项，容量所迫） | — | 纯搬移行为等价，judge 逐符号规范化对比 | `38d1e8d..c3d2b31` |
| R-4 蒸馏产 keywords + 措辞保全（§4.4 / Q4 / §5.4-A） | T5+T6 | 已单独实施并通过双判决；**§5.4-A 已按 D13 去隐私清单、按 D15 加 no-op 可观测日志** | `e8bb6a2..4f7ba93` |

### D16（用户拍板 2026-08-05）：双层打分接入检索的三项裁定（T6b）

计划 §2.2 只给了 `topic_weight_norm × (0.6 + 0.4 × entry_score)`，未裁定它与生产 `search_facts` 既有因子的关系。用户三选：

1. **保留 bm25 做乘数（1a）** → 最终 `score = bm25_pos × two_layer × recency_boost`。双层分是**优先级调节器**，bm25 仍是相关性准入。理由：计划公式无文本相关性因子，纯按它排序会把"热话题但无关"的旧 fact 顶到最前，即"记忆答非所问"。**代价：偏离 §2.2 字面的两因子形态，须在 crate AGENTS.md 与本节同时记录**。
2. **confidence → entry_score 取 high=1.0 / medium=0.6 / low=0.3（2a）**。T10 的重复证据晋升在此基础上动 entry_score（D12：永不触碰话题权重）。
3. **归一化用固定上界 `weight / 5.0`（3a）**，不用 `(weight-1)/4`。
   ⚠️ **被否决的 3b 是陷阱**：`(w-1)/4` 会让从未升温话题（w=1.0）得 `tw_norm=0` → `two_layer=0` → 总分 0 → 被 `RETRIEVAL_FLOOR` 丢弃，等于**用户没提过第二次的话题、相关记忆全查不出来**。
   由此附带两条硬约束：无关键词命中时 `tw_norm` 必须回落到 `1.0/5.0`（对齐现状 `fold(1.0, f64::max)` 的起始值），且 **T6b 不得调用 `rank_candidates`**（它按 floor 丢条目并用自己的 tie-break 重排，会吃掉 bm25 并改变删除语义）。

**副作用（经 T6b 审查者独立计算更正，原记载有误）**：
- `two_layer` 实际定义域是 **`[0.144, 1.0]`**（最小值 `0.2 × (0.6 + 0.4 × 0.3) = 0.2 × 0.72 = 0.144`），不是本节初稿写的 `0.12`——初稿误按 `entry_score` 下限为 0 计算，而 D16-2a 已把下限定为 `0.3`。
- **话题权重相对 bm25 的影响力并未变大**：话题权重贡献的动态范围仍是 `1.0/0.2 = 5×`，与改动前 `keyword_weight ∈ [1,5]` 完全相同（两者都是线性乘子）。真正新增的只有 `entry_score` 贡献的 `0.72→1.0`（约 `1.39×`）这一层。初稿"5× → 8.3×、话题权重影响力变大"的说法**作废**。
- 话题主导性（`TOPIC_DOMINANCE_RATIO`）在 `two_layer` 层由公式结构严格保证；**与 bm25 之间的主导关系按 D16-1a 有意不作保证**（bm25 是相关性准入，不参与话题主导性论证）。


### R-4 落地后仍未闭环的部分（勿重复实施）

- §5.4-B（园丁 prompt 借 INIT/INCREMENTAL 双模式等三件）→ 仍属 T12。
- §5.4-C（`auto_memory` 指引追加读取决策边界/预算/双写分工/应用纪律四条）→ **已完成（T5c，commit `2e986ce`，双 PASS）**；C3 已按 D14 如实版逐字落地。落地时发现并解决了一处冲突：D14 文本含 `` `# Remembered facts` `` 字样，与既有两条 `!prompt.contains("# Remembered facts")` 断言互斥 → 裁定把断言收紧为生产注入的精确形态 `"\n\n# Remembered facts\n\n"`（对应 `auto_memory.rs:304`）。**后续凡给提示词追加文本，先 grep 现有否定断言。**
- §4.4 中"T6 话题抽取优先消费 keywords"已完成，**双层打分接入检索排序（T6b）已完成**（commit `fd61f5e`）。
