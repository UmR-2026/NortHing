# W8-2 Judge Review

**Verdict**: ✅ **Approved**

**Counts**: SPEC C=0 / I=0 / M=3 · QUALITY C=0 / I=0 / M=3

---

## SPEC（5 条 + Global Constraints 9 条）

### §1 消三重复制 — PASS

逐块核对（最高优先）：

| 复制块 | 旧实现 | 新 helper | 等价性 |
|---|---|---|---|
| stmt 构造 Some/None | `get_facts` l:236-252 / `search_facts` l:404-430（params 不同） | 利用 `if workspace_key.is_some() { … } else { … }.map_err(?)?` 统一去重，构造与原版逐字相同：workspace 臂 SQL 与 `params![ws]`/`params![match_expr, ws, candidate_limit]` 保留；global 臂 SQL 与 `params![]`/`params![match_expr, candidate_limit]` 保留 | ✅ 字面一致，无空格/列序差异 |
| query_map 行映射闭包 | `get_facts` l:254-287 (34 行) / `search_facts` l:434-469 (36 行) | `map_fact_row` (8 元组) / `map_search_row` (10 元组)，get_facts 拿掉 last_mentioned_at 后列数 9→8 同步 | ✅ 元组顺序与字段类型与原闭包逐字段一致 |
| 字符串→枚举 match ×3 | `get_facts` l:294-328 / `search_facts` l:481-515 | `parse_scope` / `parse_confidence` / `parse_fact_type` + `parse_fact_fields` 聚合 | ✅ `_ => Err(NortHingError::service(format!("Unknown <field>: {}", …)))` 逐字保留，未引入新 warn 或静默默认 |

### §2 死变量处置 — PASS

- `bm25_pos = -rank` 行（l:542 旧）已删除，`let score = -rank * keyword_weight * recency_boost` 表达式等价。`ScoredFact.bm25: rank` 仍存 FTS5 native raw rank（负值 float），存储语义零变化。✅
- `last_mentioned_at` 从 `get_facts` 的 SELECT 列表与解构中同步删除（8 列）。**BLOCKED 检查**：`fact.rs` 已确认 `Fact` 结构体无 `last_mentioned_at` 字段（facts.rs l:11-22 只有 8 字段：schema_version/id/text/provenance/confidence/scope/fact_type/created_at），原 `get_facts` 旧 l:291 解构后该值进入 `last_mentioned_at = ...` 变量后从未被构造到 `Fact`（深审 §文件1 量规 1 也标为「观察项」非「真 bug」），实现者删除是正确的清理，非静默改 Fact 重建逻辑。✅

### §3 回退 hack 处置 — PASS

- **NaN 沉底**：降序语境下 `(true, false) => Ordering::Greater` 意为「a=NaN 应排在 b 之后」= NaN 沉底，方向正确；`(false, true) => Less` 与之配对；`(true, true) => Equal`；非 NaN 沿用 `b.score.partial_cmp(&a.score)` 保持原降序。✅ 非 Important 风险。
- **时钟异常跳 boost**：`match … { Ok(d) => Some(...), Err(e) => { tracing::warn!(...); None } }` → `compute_recency_boost(None, _) => 1.0`，路径只在异常臂生效，正常路径 `compute_recency_boost(Some(_), _)` 仍走 `(now - last).max(1.0)/86_400_000.0` 原公式。`tracing::warn!` 英文无 emoji 且带关键上下文 `"System clock before UNIX_EPOCH ({}); skipping recency boost"`。✅

### §4 防线 — PASS

- ceiling 918→894 同 commit 下调（仅此一处 manifest 变更）。`verify-rot-budget.mjs` 实测报 `7 god-file rules checked`，manifest 仅本条减项。✅
- `dream` 子模块与 `judge_mom` KV 区域未触及（grep 结果证实）。✅
- 既有测试语义零改动；测试 23/23 全绿（`cargo test -p northhing-core --features product-full memory_db`），含 2 个新例 `sort_scored_facts_nan_sinks_to_bottom` + `recency_boost_skips_on_clock_anomaly`。✅

### §5 验证集 — PASS

- `cargo check --workspace`：0 error（仅有与改动无关的既有 warnings）。✅
- `cargo test -p northhing-core memory_db` 实跑：23/23 含 2 新例绿（本次亲自重跑，输出末尾 `23 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out`）。✅
- `node scripts/verify-rot-budget.mjs` 实跑：`Rot budget verification passed`。`allow_dead_code` 从历史 109 降到 106（`bm25_pos` 移除带来的附带动）、`unwrap_production` 从 502 降到 474（helper 提取后 `?` 替代多份 `.map_err`）。✅

### Global Constraints 9 条 — PASS

1. 分层边界（仅 assembly/core + manifest 下调）✅
2. 日志纪律（英文无 emoji，仅 §3 新增一条 warn）✅
3. SDD 禁区（diff 与 commit 均不含 `.superpowers/`）✅
4. rot-budget（ceiling 仅下调 918→894）✅
5. 验证最小集（3 条全跑过）✅
6. commit 规则（单 commit `5d4d98a`，`git show --stat` 与汇报 242/-188 一致）✅
7. 无 owner 抽象（`map_fact_row`/`map_search_row`/`compute_recency_boost` 各 2 个真实消费方 = get_facts+search_facts；`parse_scope/confidence/fact_type` 经 `parse_fact_fields` 被同两处消费；`sort_scored_facts` 1 业务+1 测试消费方）✅
8. 行为零变化（除 §3 点名微调）✅
9. 编译错误处理（无编译错误，未触发 skill 路径）✅

---

## QUALITY

### 死代码/死变量 — 已处置

`bm25_pos` 删除并已替代。`last_mentioned_at` 仅从 `get_facts` 的 SELECT/解构中删除（`search_facts` 仍保留，因 `compute_recency_boost` 需要）。两次决策都附理由（impl report §2 标「观察项」语义不变）。

### 去重后等价性 — 逐字核对通过

- 两个 stmt 的 SQL 文本逐字保留（含 `WHERE`/`ORDER BY`/`LIMIT` 子句）。
- 列顺序逐字段一致（get_facts 从 9 列变 8 列 = 仅移除 `last_mentioned_at`，其余 8 列顺序未变；search_facts 10 列未变）。
- `query_map` 的元组类型与字段顺序逐字段一致。
- 三个 parse helper 的错误文案（"Unknown <field>: {}", value）与原 match 逐字一致（不是 warn 不是 silent default，是 Err service propagate）。

### 新代码风格

- `#[allow(clippy::too_many_arguments)]` 用于 `parse_fact_fields`（8 参数）合理。
- 函数指针 `Self::map_fact_row` / `Self::map_search_row` 满足 rusqlite `Fn` 闭包约束。
- `compute_recency_boost(now_ms: Option<u64>, last_mentioned_at: i64)` 返回 `f64` 用 `Option` 信号异常，签名清晰。
- `sort_scored_facts(&mut [ScoredFact])` 命名一致（被动语态），单测构造调用方便。

### 测试有效性

- `sort_scored_facts_nan_sinks_to_bottom` 非恒真：构造 `[NaN, 2.0, 1.0]`，断言 `[high, low, nan1]`，撤掉 NaN 处理后测试会失败（`(false, false)` 臂会走 `partial_cmp` 对 NaN 返回 None → Equal → 顺序未定义）。✅
- `recency_boost_skips_on_clock_anomaly` 非恒真：正常路径断言 `> 1.0`（实际 1.1），异常路径断言 `== 1.0`。两条断言把两条臂都打到。✅
- 注入方式：`compute_recency_boost(None, _)` 直接调用 helper 注入时钟异常路径是 brief §3 的允许方式（"注入异常时钟路径若可测，不可测则 report 说明"），比 mock `SystemTime::now()` 简洁且等价。✅

### god-file 健康度观察

`memory_db.rs` 现 894/894（实测 `Select-String ^` 行数 = 894，与 verify-rot-budget.mjs 同口径）= 恰好到 ceiling，零 buffer。按 AGENTS.md 量规 3（>800 提级审查，>1000 必须拆分），目前仍在「提级审查」档，但 buffer 收紧。单步小增即破线 — 下一轮若再加行应同步挪向下调，或直接拆分。**minor 健康观察，不是本次 finding**。

---

## Findings 汇总

**Critical**: 0

**Important**: 0

**Minor**:

1. **god-file buffer 零**（QUALITY）— `memory_db.rs` 894/894，下一次在本文件加行即破 rot-budget ceiling。下波建议同步下调或拆分。
2. **`text.clone()` 移除是顺带优化**（QUALITY）— 旧版 `text: text.clone()` 因后续还要 `segment_for_fts(&text)`，新版 `text` 移动入 `Fact` 后改用 `&fact.text`，零行为差异 + 少一次 String 克隆。Non-blocking observation，归本波「质量改善」。
3. **三个 parse helper 各只有一个直接消费方**（QUALITY）— `parse_scope/confidence/fact_type` 都只被 `parse_fact_fields` 调用，理论可内联；但 brief §1 第 3 条明示要求三个独立 helper，故保留。非违反。

---

## Cannot verify from diff

无。

## Plan-mandated finding

无。

## 一句话理由

三块复制逐字等价（SQL 文本/列序/枚举 match fallback 全保），§3 两处微调方向正确（NaN Greater 沉底于降序；时钟异常仅在 Err 臂跳 boost 并 warn），死变量处置已查证非 BLOCKED 真 bug，ceiling 单向下调 918→894 与实测一致，三验证全绿且新测非恒真。

---

## 旁注

- `compute_recency_boost` 命名风格与文件其他方法（`map_fact_row`/`sort_scored_facts`）都用 snake_case + 动词/动作名词 ✅
- `#[allow(clippy::too_many_arguments)]` 是 brief 隐含允许的（事实：当结构体重构会引入辅助 enum，此时走 allow 是 lazy 版本正解）✅
- 未触及 `dream` 子模块（深审观察项）与 `judge_mom` KV 区域 — 与 brief §4 一致 ✅
