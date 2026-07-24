# 探索报告：northing 自我认知与记忆架构演进

> **日期**：2026-07-24 04:07 GMT+8
> **HEAD**：6ac68bd
> **范围**：自我认知（C1 正篇）、Memory 多 agent 架构（C4 正篇）、Memory P0 检索层、已有代码实现
> **探索者**：subagent reviewer

---

## 1. 自我认知设计

### 1.1 首次启动流程

**设计文档**：`docs/design/2026-07-23-self-cognition/first-entry-design.md`

流程为四字段配置 → LLM 生成身份提示词 → 持久化存储：

```
用户看到：
  用户是【UmR】
  你是【北】
  你是用户的【同事】
  你的性格更偏向大五人格中的【敏感、深刻、内敛】性格

→ "身份生成中..." (LLM 生成)
→ 存储到 config_dir()/northhing/identity.md
→ 通过 PLACEHOLDER_PERSONA 注入 system prompt Layer 2
```

**注入链路**（代码已实现）：
1. `identity.rs::identity_path()` → `config_dir()/northhing/identity.md`
2. `identity.rs::load_identity()` → 读取文件内容
3. `system_prompt.rs::build_workspace_persona_with_identity()` → 将 identity 内容追加到 persona 后，标记为 `# Self-cognition` 段
4. 通过 `PLACEHOLDER_PERSONA` 替换注入 agent prompt 模板

**评价**：链路设计干净。persona + identity 在同一层拼接，不侵入能力层（agentic_mode.md），也不走 judge 门禁。身份是 agent 自己的事，这个定位清晰。

### 1.2 五色板与大五人格的对应

| 色 | 大五特质 | 关键词 |
|---|---|---|
| 紫 | 开放性 | 好奇·想象·不拘一格 |
| 深蓝 | 尽责性 | 严谨·可靠·有条理 |
| 暖珊瑚 | 外向性 | 热情·主动·善于表达 |
| 柔绿 | 宜人性 | 温和·体贴·善解人意 |
| 冷青 | 神经质 | 敏感·深刻·内敛 |

**合理性分析**：

- **优点**：大五人格是心理学公认的人格模型，5 维恰好对应 5 色，选择成本低。用户不需要理解大五理论，hover 关键词即可决策。色板选择同时决定界面强调色（设计 spec v1 §3），形成视觉-性格统一。
- **问题**：
  - **"神经质"命名**：在心理学中 Neuroticism 是中性术语，但中文"神经质"有负面联想。设计稿用"敏感·深刻·内敛"作为 hover 关键词，实际上是在做语义美化——这很聪明，但如果用户看到对应的英文 "Neuroticism" 可能产生认知冲突。
  - **单选局限**：大五人格是维度模型（每个人在 5 个维度上都有位置），但色板强制单选。这意味着选了"柔绿（宜人性）"的用户，其 agent 的开放性和外向性就完全不被定义。LLM 生成的身份提示词会倾向于放大单一特质，缺乏人格的立体感。
  - **冷启动可信度**：5 个选项对于"你是什么性格"这个问题过于简化。但考虑到这是 agent 首次启动（用户还没有和 agent 建立关系），简化是合理的——"成长时刻"设计允许后续自主改色，弥补了初始简化的不足。

### 1.3 "改色完全自主，不提醒用户"

设计文档明确写道：
> 成长时刻的代表色不受初始五色板限制，agent 可自由生成任意颜色。
> identity.md 由 agent 自主管理，不走 judge 流程，不提醒用户。

**利**：
- 符合"agent 是成长主体"的哲学——身份是它自己的事
- 避免"请你确认你的新性格"这种尴尬交互
- 让成长时刻成为 agent 的自我决定，而非用户审批

**弊**：
- **用户失控感**：用户可能一觉醒来发现 agent 的性格变了，且没有任何通知。这在产品层面是激进的——大多数用户期望对自己使用的工具有一定掌控力
- **自我验证闭环风险**：如果 agent 通过 episode log 看到"自己上次做了 X"，然后据此修改身份，就形成了循环验证。设计文档提到"agent 不读 episodes 做决策（防自我验证闭环）"，但 identity.rs 的成长时刻路径尚未实现，这条红线能否守住还需观察
- **可调试性差**：没有审计日志记录"为什么改色"。如果 agent 改色后行为异常，用户无法回溯原因

**建议**：改色可以不提醒用户，但应在 audit log 中记录触发原因和旧→新对比，保留可追溯性。

---

## 2. Memory 多 Agent 架构

### 2.1 分权设计：主 agent → memory-judge → memory-writer

**架构**（来自 `memory-multi-agent-architecture.md`）：

```
主 agent（前台，用户交互）
  │ 实时轨：用户显式"记住X" → 立即异步发 judge
  │ 后台轨：每 N 轮 / session 结束 → 批量发 judge
  ▼
memory-judge（异步，后台）
  ├── 读 judge-mom（时机偏好 + 内容偏好）
  ├── 评估：该不该写 / 谁来写 / 写到哪
  ├── 路由：→ writer 执行
  ├── 自学习：更新 judge-mom
  └── 定期 dream：全量扫描，合并/降权/删除
  ▼
memory-writer（异步，judge 指派）
  └── 执行写入（facts/keyword_weights/prune）
```

**评价**：

分权设计的核心价值是**让主 agent 保持轻量**。主 agent 不需要理解"什么是值得记住的"、"什么时候整理记忆"——它只需要把候选丢给 judge。这和 OpenClaw 的"主 agent + 技能"模式一脉相承。

但分权也带来了**通信开销**：每次记忆操作需要一次 agent 间通信（主 agent → judge → writer）。在异步模式下这不成问题，但如果 judge 的 LLM 调用延迟较高，实时轨的"用户显式记住 X"可能让用户等待。

**关键设计决策**：judge 既是裁判也是编排者。这意味着 judge 的 prompt 需要同时处理"该不该记"（判断）和"怎么记"（路由策略）。如果判断逻辑复杂化（比如需要理解语义关联），judge 的 prompt 会变得臃肿。设计文档中 judge 路由决策表只有 5 行，目前足够，但随记忆类型增多（用户偏好、项目状态、技术决策、情感记忆...），这张表会膨胀。

### 2.2 存储隔离方案

**设计**：
```
config_dir()/northhing/
├── identity.md              ← core agent（agent 自主管理）
├── memory/
│   └── memory.db            ← SQLite（facts + facts_fts + keyword_weights）
├── judge/                   ← judge 隔离区（core agent 不可读）
│   ├── mom.db               ← judge 专用记忆
│   └── audit/               ← 审计日志
└── episodes/                ← 日记（UI 功能，agent 不读）
```

**代码层保证**：

设计文档提出"不同 Rust module 持有不同 DB handle"来保证隔离。但在当前的 `memory_db.rs` 实现中，**所有表（facts、facts_fts、keyword_weights、judge_mom）都建在同一个 SQLite 文件中**：

```rust
// memory_db.rs create_tables()
conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS facts (...)
     CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(...)
     CREATE TABLE IF NOT EXISTS keyword_weights (...)
     CREATE TABLE IF NOT EXISTS judge_mom (...);"
)
```

这与设计稿的"judge/mom.db 独立文件"不符。当前实现是 P0 阶段的简化（单 DB 文件），隔离靠 Rust 模块的 API 边界（core agent 的 Rust 模块只暴露 facts_* 和 keyword_weights 的查询接口，judge_mom 表由 judge 专用模块访问）。但 SQL 层面没有做 row-level 权限控制——如果 core agent 的代码 bug 导致 `SELECT * FROM judge_mom`，技术上是可以执行的。

**建议**：P0 的单文件方案可以接受（降低复杂性），但应在 `MemoryDb` 的公开 API 中完全不暴露 judge_mom 的查询方法，靠 API 边界而非 DB 文件隔离。到 P1 阶段再考虑物理分离。

### 2.3 Judge-mom（时机自学习）

设计文档定义 judge-mom 存储：
- 记忆维护时机偏好（"用户习惯晚上结束工作 → session 末尾整理"）
- 内容偏好（"用户重复提了 pnpm → 权重已提升"）
- 忽略列表
- 维护日志

进化循环：judge 按偏好执行 → 用户反馈 → 更新 judge-mom → 下次调整。

**复杂度评估**：

这个设计**确实过于复杂**，原因：

1. **信号稀疏**：用户不会频繁给出"这次整理时机不好"的显式反馈。隐式反馈（比如下次 session 是否更快进入工作状态）噪声太大，难以可靠归因到"上次整理时机"
2. **冷启动漫长**：设计文档说"前几轮走固定流程"，但"几轮"是 3 轮还是 30 轮？在数据稀疏的情况下，judge-mom 的"优化"可能不如固定参数可靠
3. **元认知负担**：judge 需要同时处理"记什么"和"什么时候记"两个正交问题。时机自学习引入了额外的 LLM 调用开销，但收益不确定
4. **过度工程**：对于一个 agent 记忆系统，"每 N 轮批量整理"的固定策略 + "用户显式记住"的实时策略，已经覆盖了 90% 的场景。judge-mom 优化的是剩余 10%，但增加了 50%+ 的系统复杂度

**建议**：P0 不实现 judge-mom。用固定参数（"每 5 轮或 session 结束时批量整理"）启动，积累足够数据后再评估是否需要自学习。设计文档的分期（P1 才做 judge-mom）是正确的。

---

## 3. Memory P0 检索层

### 3.1 JSONL → SQLite FTS5 迁移

**现状**（`facts.rs`）：
- `facts.jsonl` 文件，append-only
- `select_facts_for_prompt`：按 scope > confidence > recency 排序，1000 token 预算截断
- `distill_facts_from_user_message`：关键词触发（"以后"、"记住"、"prefer" 等），按句分割
- `append_facts_dedup`：exact-text 去重

**问题**（设计文档总结）：
- 不相关：全量 top-N 注入，不关心当前查询
- 不精准：exact-text dedup 无法识别语义重复
- 不进化：重复提及不增加权重

**迁移方案**（`memory_db.rs`）：
- SQLite + FTS5 虚拟表，WAL 模式
- `facts` 表 + `facts_fts` FTS5 索引 + 触发器自动同步
- `keyword_weights` 表存储语义权重
- `judge_mom` 表（judge 隔离）

**评价**：

迁移方向正确。FTS5 是 SQLite 内置的全文检索引擎，零外部依赖（除 rusqlite 的 bundled 编译），适合 P0 阶段。WAL 模式保证了读写并发的安全性。

**但有一个编译问题**：`memory_db.rs` 引用了 `Fact`、`FactScope`、`FactConfidence`、`FactProvenance` 类型，但没有 `use super::facts::{...}` 导入语句。这意味着 `memory_db.rs` **当前无法通过编译**。这与 `git diff --stat HEAD` 显示它是未提交的新文件一致——WIP 状态，尚未通过编译检查。

此外，rusqlite 的 `bundled` feature 需要 C 编译器（gcc），在当前 Windows 环境下 `gcc.exe` 不可用，构建失败。这是一个环境依赖问题，需要在 CI 环境或安装 MinGW/MSVC 后解决。

### 3.2 keyword_weights 表设计

```sql
CREATE TABLE keyword_weights (
    keyword TEXT PRIMARY KEY,
    weight REAL NOT NULL DEFAULT 1.0,
    mention_count INTEGER NOT NULL DEFAULT 1,
    last_boosted_at INTEGER NOT NULL,
    related_keywords TEXT NOT NULL DEFAULT '[]'  -- JSON array
);
```

**评价**：

- `related_keywords` 用 JSON 数组存储语义关联词组，简单但不支持 SQL 级联查询。如果需要"查 pnpm 的所有关联词"，需要先 `SELECT related_keywords FROM keyword_weights WHERE keyword = 'pnpm'`，再在应用层解析 JSON。P0 够用，但 P1 做语义关联图时需要拆成独立表。
- `last_boosted_at` 是 INTEGER（Unix 毫秒时间戳），用于时间衰减计算。但缺少 `created_at`——无法区分"新关键词首次出现"和"老关键词从未被 boost"。
- 没有 `scope` 字段——keyword_weights 是全局的，不区分 workspace。这意味着"pnpm"在项目 A 和项目 B 中共用权重。如果用户在项目 A 用 npm、项目 B 用 pnpm，权重会互相干扰。

**权重进化规则**（设计文档）：

| 事件 | 权重变化 |
|---|---|
| 用户再次提及同义内容 | weight += 0.5, mention_count++ |
| judge 识别语义关联 | related_keywords 互相关联，共享 boost |
| 注入后 agent 实际使用 | weight += 0.2（正反馈） |
| 注入后完全没被引用 | weight -= 0.1（衰减，不低于 0.1） |
| 时间衰减 | 每 30 天 weight *= 0.9 |
| 用户说"别记这个" | weight = 0（标记忽略） |

**问题**："注入后 agent 实际使用"如何检测？这需要分析 agent 的回复是否引用了注入的 fact 内容。实现上非常困难——自然语言生成不会精确引用原文。设计文档没有定义"实际使用"的检测机制，这条规则可能在 P0 被跳过。

### 3.3 排序公式

```
score = BM25(query, fact_text) × keyword_weight × recency_boost
```

其中：
- BM25：FTS5 内置关键词匹配度
- keyword_weight：judge 写入时更新的语义权重
- recency_boost：`1.0 + 0.1 × (1 / days_since_last_mention)`

**合理性分析**：

- **BM25**：标准文本检索排序函数，适合关键词匹配场景。但 FTS5 的 BM25 返回的是负数（越小越好），`memory_db.rs` 中 `ORDER BY rank` 是升序（最负 = 最相关），这是正确的 FTS5 用法。但 `ScoredFact` 结构体中 `bm25: f64` 存储的是原始 rank 值（负数），如果后续要乘以 keyword_weight，需要注意符号——负数 × 正数 = 更负 = 更相关，逻辑上正确但语义上反直觉。
- **keyword_weight**：乘法叠加简单有效。但 `memory_db.rs` 的实现中，`keyword_weight` 取的是 `max`（所有匹配关键词中权重最高的），而非累加或平均。这意味着一个 fact 命中 3 个高权重关键词，和命中 1 个，效果一样。设计文档没有明确说明应该是 max/sum/avg，这是一个需要决策的点。
- **recency_boost**：`1.0 + 0.1 × (1 / days_since_last_mention)` —— 这个公式有问题。当 `days_since_last_mention = 0`（今天提到的）时，`1/0` 会除零。需要加 1：`1.0 + 0.1 / (1 + days_since_last_mention)`。此外，1 天前提及的 boost 是 1.1，30 天前是 1.003——衰减曲线过于平缓，几乎不影响排序。相比之下 keyword_weight 从 1.0 到 3.0+ 的变化范围远大于 recency_boost，意味着 recency 在排序中几乎不起作用。

**建议**：
1. 修正 recency_boost 公式避免除零
2. 使用指数衰减：`1.0 + 0.5 × exp(-days / 7)`（一周内 boost 显著，之后快速衰减）
3. 明确 keyword_weight 的聚合方式（建议用 max，因为一个高权重关键词命中已经足够说明相关性）

---

## 4. 已有代码分析

### 4.1 facts.rs（已提交，commit 之前就有）

**Fact 结构**：
```rust
pub struct Fact {
    pub schema_version: u32,
    pub id: String,
    pub text: String,
    pub provenance: FactProvenance,  // session_id + turn_id
    pub confidence: FactConfidence,  // High/Med/Low
    pub scope: FactScope,            // Workspace/Global
    pub created_at: u64,
}
```

**功能**：
- `append_facts`：append-only JSONL 写入
- `append_facts_dedup`：exact-text 去重写入
- `read_facts`：读取全部 facts（跳过损坏行）
- `select_facts_for_prompt`：按 scope > confidence > recency 排序，token 预算截断
- `distill_facts_from_user_message`：双语关键词触发，句分割，300 字截断

**评价**：代码质量高，测试覆盖充分（20+ 测试覆盖 round-trip、去重、排序、边界条件）。`distill_facts_from_user_message` 的关键词列表偏窄（只有 14 个关键词），但对 P0 够用。

### 4.2 auto_memory.rs（已提交）

**功能**：
- `build_workspace_agent_memory_prompt`：构建完整的 memory prompt（base prompt + facts section）
- base prompt 是一个大型 Markdown 模板（~3000 字），定义了 4 种记忆类型（user/feedback/project/reference）、写入规范、访问时机
- facts section 从 `facts.jsonl` 读取，调用 `select_facts_for_prompt` 截断到 1000 token

**评价**：base prompt 非常详尽——可能是过于详尽了。3000+ 字的 memory 指令会持续占用 context window。对于"用户聊代码时"的场景，这些记忆管理指令大部分是无关的。考虑在 P0 后将 memory 指令也分层：只注入核心规则（200 字），详细规范放在 agent 可按需读取的文件中。

### 4.3 memory_db.rs（未提交，WIP）

**已实现**：
- `MemoryDb::open`：打开/创建 SQLite DB，WAL 模式，建表
- `create_tables`：facts + facts_fts（含触发器同步）+ keyword_weights + judge_mom
- `insert_fact`：INSERT OR IGNORE（幂等）
- `get_facts`：按 workspace_key 过滤，返回 global + workspace facts
- `touch_fact`：更新 last_mentioned_at
- `delete_fact`：删除 fact（触发器自动同步 FTS）
- `search_facts`：FTS5 MATCH + BM25 排序 + keyword_weight 加权
- `boost_keyword`：关键词权重提升（+0.5）+ related_keywords 合并
- `get_keyword_weight` / `decay_all_weights` / `set_keyword_ignored`
- `tokenize_query`：CJK bigram 分词 + 西文整词

**CJK 分词**：`tokenize_query` 实现了 CJK bigram 分词——对中文查询 "以后都用" 生成 ["以后", "后都", "都用"] 三个 bigram。这是 FTS5 中处理 CJK 的常见方案，比单字分词更精准。

**与设计稿的对应关系**：

| 设计稿 | memory_db.rs | 状态 |
|---|---|---|
| facts 表 | ✅ 完整实现 | 对齐 |
| facts_fts FTS5 | ✅ 含触发器 | 对齐 |
| keyword_weights 表 | ✅ 完整实现 | 对齐 |
| judge_mom 表 | ⚠️ 建在同一个 DB | 偏离（设计稿要求独立文件） |
| BM25 × keyword_weight | ✅ search_facts 实现 | 对齐（但 recency_boost 未实现） |
| recency_boost | ❌ 未实现 | 缺失 |
| workspace_key 隔离 | ✅ 查询和写入都支持 | 对齐 |

**关键问题**：
1. **缺少 `use super::facts::{Fact, FactScope, ...}` 导入**——当前无法编译
2. **recency_boost 未在 search_facts 中应用**——设计稿的三因子排序只实现了两因子
3. **judge_mom 与 facts 在同一个 DB 文件**——与设计稿的物理隔离方案不符
4. **search_facts 的 SQL 参数绑定有 bug**：workspace_key.is_some() 分支用 `?2` 绑定 ws，但 `?1` 是 match_expr——这在有 workspace_key 时参数顺序是 `[match_expr, ws, limit]`，正确；但无 workspace_key 时参数是 `[match_expr, limit]`，SQL 中 `?2` 绑定的是 limit——但 SQL 写的是 `LIMIT ?2`，这是正确的。等等，仔细看：无 workspace_key 分支的 SQL 中 `LIMIT ?2`，而参数是 `params![match_expr, limit as i64]`——`?1`=match_expr, `?2`=limit。正确。

### 4.4 identity.rs（已提交，commit 9c95faf）

**已实现**：
- `IdentityConfig`：4 字段结构体（user_name, agent_name, relationship, personality_keywords）
- `identity_path()`：`config_dir()/northhing/identity.md`
- `identity_exists()` / `load_identity()` / `save_identity()` / `clear_identity()`：文件 IO
- `build_identity_prompt()`：构建 LLM 生成 prompt（50-80 字中文，第一人称，不含元信息）

**未实现**：
- LLM 调用（`build_identity_prompt` 只生成 prompt 文本，实际调用 LLM 的代码不在本文件中——可能在前端 UI 层或 coordinator 层）
- 成长时刻的自主改色逻辑
- 颜色到关键词的映射（色板 UI 是前端工作）

**评价**：identity.rs 是一个纯粹的"配置存储 + prompt 构建"模块，职责单一。70 行代码，没有多余抽象。`build_identity_prompt` 的模板质量高——"用名字代替所有代词"是一个好的约束，避免了"你/我/他"的指代歧义。

---

## 5. 身份生成 → 存储 → 注入的完整链路

```
[首次启动]
  用户填写 4 字段 → IdentityConfig
  → build_identity_prompt(&config) 生成 LLM prompt
  → [LLM 调用]（不在 identity.rs 中，需找到调用方）
  → save_identity(generated_text) → config_dir()/northhing/identity.md

[后续每轮对话]
  PromptBuilder::build_workspace_persona_with_identity(workspace)
  → build_workspace_persona_prompt(workspace)  // 读取 workspace 的 BOOTSTRAP.md / SOUL.md
  → identity_exists() → true
  → load_identity() → "我是北，UmR的同事..."
  → 拼接: persona + "\n\n# Self-cognition\n\n" + identity_content
  → 替换 PLACEHOLDER_PERSONA

[清空身份]
  clear_identity() → 删除 identity.md
  → 下次启动重新走首次配置流程

[成长时刻（远期）]
  agent 自主生成新身份 → save_identity(new_content)
  → 旧版沉入历史（渐变条）
  → 不提醒用户
```

**注入位置**：system_prompt.rs 中的 `build_workspace_persona_with_identity` 方法被两个路径调用：
1. `build_prompt_from_template`（完整 prompt 构建）
2. `build_agent_prompt_layer`（Layer 2 分层构建）

两条路径都正确地处理了 identity 的注入。

---

## 6. 与 C2/C3/C4 Phase 0 的关系

### C2（Episode Log）

**已有**：`src/crates/assembly/core/src/agentic/episodes/` 目录，commit `159c10d` 实现了 episode log phase 1（store, distill, finalize hook, facade list）。

**关系**：**独立共存**。设计文档明确：
> episodes（日记）= UI 功能（左侧边栏给人类看的可读记录），不属于 agent 认知架构，agent 不读日记做决策（防自我验证闭环），保持现有 append-only JSONL 格式。

Episode log 是给**人类**看的记录；memory（facts + keyword_weights）是给**agent** 用的检索库。两者完全独立，数据不互通。这是正确的边界——防止 agent 通过阅读自己的历史来"自我验证"。

### C3（Facts）

**已有**：`facts.rs` 的 JSONL 实现。

**关系**：**被替代**。P0 的 SQLite FTS5 方案是对 C3 的全面升级：
- JSONL → SQLite（存储层）
- 全量 top-N → query-aware 检索（检索层）
- exact-text dedup → keyword_weights 语义加权（去重/进化层）

但 C3 的 `Fact` 结构体被保留——`memory_db.rs` 直接复用了 `facts.rs` 中的 `Fact`、`FactScope`、`FactConfidence` 类型。存储介质变了，数据模型不变。

### C4 Phase 0（Judge Gate）

**已有**：
- `judge_gate/` 目录：`mod.rs`（922 行）、`runner.rs`、`audit.rs`、`receipt_store.rs`
- `GateRequest` / `GateVerdict` / `ApprovedGateReceipt` 协议层
- 4 条红线（I-NEG-1~4）规则检查
- `SubagentJudgeRunner`（生产）+ `FakeJudgeRunner`（测试）
- `evaluate()` 函数：证据验证 → 构建 brief → 运行 judge → 解析 → 规则检查 → 审计 → 收据

**关系**：**扩展而非替代**。设计文档明确：
> Phase 0 已实现门禁原语（ApprovedGateReceipt）、四红线、promote_candidate_skill 写入口、FakeJudgeRunner 测试基建。
> 正篇扩展：judge 从"门禁"升级为"记忆编排者"，新增 routine 写入路径（不需要 receipt）、dream/distill 定期任务、keyword_weights 语义加权。
> **保留 promote 的完整门禁流程（技能固化仍需 receipt）。**

也就是说：
- **技能固化（promote）**：仍走 C4 Phase 0 的完整门禁（judge 评估 → 4 红线 → ApprovedGateReceipt → 写入）
- **记忆写入（routine）**：走新增的简化路径（judge 评估 → 直接写入，不需要 receipt）

这是合理的——技能固化是高风险操作（永久改变 agent 行为），需要严格门禁；记忆写入是低风险操作（可删除、可降权），不需要 receipt 级别的审计。

### 衔接总结

| 层 | C 编号 | 状态 | 与自我认知/记忆架构的关系 |
|---|---|---|---|
| Episode Log | C2 | 已实现 Phase 1 | 独立共存，互不干涉 |
| Facts | C3 | 已实现 JSONL | 被 Memory P0 SQLite 替代，数据模型保留 |
| Judge Gate | C4 Phase 0 | 已实现 | 扩展：judge 升级为记忆编排者，门禁流程保留 |
| 自我认知 | C1 正篇 | 后端已实现，前端待做 | 独立于 C2/C3/C4，通过 persona 通道注入 |
| Memory P0 | C4 正篇 P0 | 设计定稿，代码 WIP | 叠加在 C3 之上，复用 C4 Phase 0 的 judge 基建 |

---

## 7. 开放问题

### 7.1 最有想象力的部分

**自我认知的成长时刻**是整个设计中最有想象力的部分。一个 agent 在使用过程中，通过自我反思 / dream / skill 学习，自主决定改变自己的性格色彩和身份描述——这在目前的 AI 产品中几乎没有人做。

大多数 AI 助手是"出厂即定型"的（Siri 永远是 Siri，Claude 永远是 Claude）。northing 的设计让 agent 拥有**身份的演化权**，而且这种演化不需要用户审批。如果实现得当，每个用户的 agent 都会演化出不同的性格——这创造了真正的"个体性"。

与记忆系统的配合：agent 的成长由记忆驱动（它学到了什么 → 影响它成为什么），而身份又反过来影响记忆的偏好（什么性格的 agent 更关注什么类型的记忆）。这形成了一个**正反馈的成长循环**，在理论上可以让 agent 越用越"懂你"。

**Memory 多 agent 架构**的想象力在于：它把"记忆管理"从主 agent 的职责中完全剥离。这意味着主 agent 可以把全部 context window 用于"当前任务"，而记忆的筛选、整理、遗忘都在后台异步完成。如果 judge 的质量足够高，用户会感受到"agent 总是记得该记的、忘掉该忘的"，而无需理解背后的机制。

### 7.2 最大的风险

**风险一：judge 的 LLM 成本和质量**。

整个记忆架构的核心依赖是 memory-judge 的判断质量。judge 需要理解语义（"用户说'别用 npm'和'以后都用 pnpm'是同一个偏好的两面"）、做出路由决策（写入 facts 还是 boost keyword_weights）、还要做时机判断（"现在是整理的好时机吗"）。这些判断每次都需要一次 LLM 调用。

如果 judge 用强模型，成本会很高（每次 session 可能触发 3-5 次 judge 调用）。如果用弱模型，判断质量不可靠——错误的 judge 决策会导致重要记忆被遗忘或无关记忆被保留，这在长期使用中会累积成"记忆污染"。

**风险二：自我验证闭环**。

设计文档说"agent 不读 episodes 做决策"，但成长时刻的自我认知更新需要某种"自我反思"的输入。如果反思的输入是 agent 自己的对话历史（而非 episodes），那本质上还是自我验证。设计需要明确：**成长时刻的触发输入是什么？** 如果不能回答这个问题，"防自我验证闭环"就是一句空话。

**风险三：memory_db.rs 的编译问题**。

当前 `memory_db.rs` 缺少类型导入，无法编译。此外 rusqlite bundled feature 在 Windows 上需要 gcc（MinGW），当前环境不满足。这意味着 Memory P0 的实际开发还没有真正启动——`memory_db.rs` 是一个结构完整的草稿，但距离可编译、可测试还有一步之遥。

**风险四：系统复杂度的非线性增长**。

当前设计的分期看起来合理（P0 → P1 → P2 → P3），但每一期都引入了新的移动部件：
- P0：SQLite + FTS5 + keyword_weights
- P1：judge-mom 自学习 + dream + 反馈循环
- P2：flat vector search
- P3：HNSW + distill（技能生成）

到 P3 时，系统有 4+ 个 agent（主 agent、judge、writer、dream-runner），3 种存储（SQLite、向量索引、HNSW），2 套记忆体系（facts + judge-mom）。这种复杂度对于一个个人项目来说是激进的。每增加一个移动部件，bug 的可能性都指数级增长。

**建议**：在 P0 验证完整（facts 检索质量确实优于 JSONL top-N）之前，不要启动 P1 的设计。用真实数据跑 2-3 周，看看 keyword_weights 的权重分布是否符合预期，再决定是否需要 judge-mom 和 dream。

---

## 附录：文件索引

| 文件 | 状态 | 说明 |
|---|---|---|
| `docs/design/2026-07-23-self-cognition/first-entry-design.md` | 已提交 | 自我认知首次启动设计 |
| `docs/design/2026-07-23-self-cognition/memory-multi-agent-architecture.md` | 已提交 | C4 多 agent 架构 spec |
| `docs/design/2026-07-23-self-cognition/memory-retrieval-design.md` | 已提交 | 检索层设计 |
| `src/crates/assembly/core/src/agentic/identity.rs` | 已提交 (9c95faf) | 身份存储 + prompt 构建 |
| `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs` | 已提交 (9c95faf) | identity 注入 system prompt |
| `src/crates/assembly/core/src/service/agent_memory/facts.rs` | 已提交 | Fact 结构 + JSONL 读写 |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs` | 已提交 | memory prompt 构建 + facts 注入 |
| `src/crates/assembly/core/src/service/agent_memory/mod.rs` | 已提交 | 模块导出 |
| `src/crates/assembly/core/src/service/agent_memory/memory_db.rs` | **未提交 (WIP)** | SQLite FTS5 实现，缺类型导入 |
| `src/crates/assembly/core/src/agentic/judge_gate/` | 已提交 | C4 Phase 0 judge gate |
| `Cargo.toml` | 未提交变更 | 新增 rusqlite workspace 依赖 |
| `src/crates/assembly/core/Cargo.toml` | 未提交变更 | 新增 rusqlite 依赖 |
