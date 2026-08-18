# Task T6a Review — 话题权重上升通道接线

> 双判决：spec 合规 + 代码质量。
> 范围：`985bbb9` → `27c9738`（worktree `feat/growth-core-0804`，HEAD `27c9738`）。
> 仅审查实现者已交付内容，不重跑 §5 命令（报告即证据）。

---

## 1. 判决摘要

- **SPEC: PASS**（按修正后的权威口径：首次提及 = 基线 1.0；第二次起严格 > 1.0；上限 5.0；底线 1.0；先 boost 后 decay）
- **QUALITY: PASS**（实现与文档注释一致；测试用 epsilon；warn-only；不引入双层打分；不动 search_facts / boost_keyword / decay_all_weights / get_keyword_weight）
- **APPROVED WITH NOTES**（一个 Important finding：含连接符的 topic 在检索侧命中条件比简单 ASCII 更严格，应在 triage 立项列入后续 `topics::score` 工作）

---

## 2. Findings

### Critical
无。

### Important

**I-1 — 含连接符的 topic（路径/带符号 token）在检索侧的命中面比简单 ASCII 更窄**
- 证据：`memory_db.rs:531-540` 的 fold 逻辑要求 `segment_for_fts(kw).split_whitespace()` 中**任一 token** 在 fact_tokens 集合里；`segment_for_fts`（`memory_db.rs:883-912`）只按 `is_whitespace()` 切，**不按 `/` `.` `_` `+` `-` 切**。
- 具体场景：
  - 用户输入 `"src/agentic"` → `extract_topics` 保留为 `"src/agentic"`（`topics/extract.rs:92-94, 111-131`：连接符允许出现在 token 内部、不做内部分割）。
  - 写入 `keyword_weights` 的 keyword 列 = `"src/agentic"`。
  - 若某条 fact 文本字面包含 `"src/agentic"`（连续无空白） → fact_tokens 包含 `"src/agentic"` → **命中**。
  - 若某条 fact 文本只含 `"src"` 与 `"agentic"`（被空白或标点隔开，如 `"edited the src and agentic files"`） → fact_tokens 不含 `"src/agentic"` → **不命中**。
- 结论：对 `pnpm` 这类简单 ASCII 词与 CJK 段（通过 bigram 重叠命中），信号**真的活了**；对路径型 topic 仅当 fact 文本字面保留连接符时才活。对排序的实际影响取决于 fact 文本的写法风格，路径风格通常仍命中。
- **本任务范围内不可修**（修法不在 brief §3 授权范围内——不动 `search_facts`、不动 `segment_for_fts`）。
- 建议：列入后续 `topics::score` 接线 / 检索侧 token 化对齐工作的 triage 清单，不在本任务阻塞。

### Minor

**M-1 — `growth_adapter.rs:38-49` 常量注释与 §5 测试数值证据**
- `TOPIC_DECAY_FLOOR = 1.0` 的文档注释清楚说明了"提过又冷却"与"从未提及"在 fold 视角下等价，与 `search_facts` 的 fold 初值 1.0 自洽。无误。
- 仅风格层面：`MAX_TOPIC_DECAY_FACTOR` 之类的对称命名没必要；当前命名已经反映用途。

**M-2 — 上限 5.0 × 衰减 0.99 的排序抖动**
- 在 cap 触发后，每个完成回合的同一个 topic 权重在 `[4.95, 5.0]` 之间循环（5.0 → boost+cap → 5.0 → decay → 4.95 → boost → 5.0 → ...）。
- 抖动幅度 ≈ 1%（相对 bm25 与 recency_boost 的整体分数量级），不影响实际排序稳定性。可接受，不要求修。

**M-3 — 测试 6 降级说明的细节**
- 报告 §8 已诚实说明降级（无 `related_keywords` 列的公开读取 API，`memory_db.rs:639-653` 的 `get_keyword_weight` 只读 `weight` 列）。
- 降级后断言"行数 == 话题数"是较弱的不变式：丢失了对**共现图内容正确性**的覆盖（`related_keywords` 真的写入了同回合其它话题吗？）。
- 缓解：共现图内容正确性由 `boost_keyword` 自身既有测试（`memory_db_tests.rs`）间接覆盖。本任务范围内已尽力。

---

## 3. Constraints 核对表

| # | 条目 | 结果 | 证据 |
| --- | --- | --- | --- |
| 1 | 只改 2 个文件 | PASS | `git diff --name-only 985bbb9..27c9738` = `turn_persist.rs` + `growth_adapter.rs`；`memory_db.rs` / `facts.rs` / `dream.rs` / `distiller.rs` / `judge_memory.rs` / `src/agentic/**` / `Cargo.toml` 一行未动 |
| 2 | 两条已授权行为变更，无第三条可观察变化 | PASS | (a) 底线 0.1 → 1.0（`growth_adapter.rs:49` 常量 `TOPIC_DECAY_FLOOR = 1.0`，`turn_persist.rs:573` 删旧行 + 注释指明迁移去向）；(b) boost+decay 成对搬到 `candidates.is_empty()` 早退之前（`turn_persist.rs:489-497`）；其它可见行为未变 |
| 3 | 先 boost 后 decay；topics 空时仍执行 decay | PASS | `growth_adapter.rs:206` `extract_topics` → `:210-220` boost 循环 → `:224` decay；无早退路径 |
| 4 | 话题来源只用 crate 的 `extract_topics`；`related` = 同回合其它话题 | PASS | `growth_adapter.rs:206, 211-216`；无 LLM 调用、无自造抽词；crate 侧（`src/agentic/src/topics/extract.rs`）一行未改（`git diff` 确认） |
| 5 | warn-only；非测试代码无 `unwrap`/`expect`/`panic!` | PASS | `boost_turn_topics` 内全部 `if let Err(...) { warn! }` 形态（`:217-219, 224-226`）；测试代码 `unwrap` 是 brief 允许的 |
| 6 | 未改 `boost_keyword` / `decay_all_weights` / `get_keyword_weight` / `search_facts` | PASS | `memory_db.rs` 未在改动文件列表里（约束 1 已核实） |
| 7 | 未跑 `cargo fmt`；English-only 无 emoji | PASS | diff 无 `cargo fmt`-style 的格式改动（纯新增函数 + 局部注释）；注释与日志字符串均为英文；测试里中文是**被测数据**（按用户裁定允许） |
| 8 | `growth_adapter.rs` < 800 行 | PASS | 实际 **638 行**（`Measure-Object` 测得），余量 162 行；报告 §12 数字与实测一致 |
| 9 | 9 条测试按修正后口径全部存在 | PASS | 见下方测试清单 |
| 10 | §5 六条命令原始输出齐全；warning = 19；memory_db 21 tests 无回归 | PASS | 报告 §6 命令 1 末尾 `"generated 19 warnings"`；命令 3 `"21 passed; 0 failed"`；其它命令输出与状态见报告 §6 |

### 测试 9 条（按修正口径）

| brief §4 | 测试函数 | 修正口径对应 | 状态 |
| --- | --- | --- | --- |
| 测试 1（原 `> 1.0`） | `first_mention_equals_baseline_by_design`（1a） | 首次 == 1.0（epsilon） | ✓ |
| 测试 1b（新增） | `second_mention_raises_above_baseline` | 第二次 > 1.0，落 `[1.95, 2.0]` | ✓ |
| 测试 2（修正比较对象） | `repeated_mentions_increase_monotonically` | 第 2 vs 第 3 | ✓ |
| 测试 3 | `respects_five_cap` | 10 次 ≤ 5.0 | ✓ |
| 测试 4 | `floor_never_broken_by_long_cooling` | 500 次空输入后 ≥ 1.0 | ✓（走的是空输入路径：`growth_adapter.rs:604` `boost_turn_topics(&db, "", now + i)`，`extract_topics("")` 返回空 vec → 无 boost，仅 decay） |
| 测试 5 | `never_mentioned_returns_baseline` | `get_keyword_weight("never-mentioned")` == 1.0 | ✓ |
| 测试 6（降级） | `co_occurrence_records_related_row_count` | 降级为"行数 == 话题数 + 无杂散行" | ✓（诚实降级；见 M-3） |
| 测试 7 | `empty_and_stopword_input_still_decays` | 空/停用词不增行但 decay 仍跑、不 panic | ✓ |
| 测试 8 | `cjk_input_produces_a_row` | CJK 输入产生至少一行 | ✓ |
| 测试 9 | `warn_only_no_panic_on_healthy_db` | DB 正常时不 panic | ✓ |

浮点断言全部用 `WEIGHT_EPS = 1e-9`（`growth_adapter.rs:508`）。

---

## 4. "信号是否真的活了"专项分析

### 写入路径
`boost_turn_topics` (`growth_adapter.rs:205-227`)：
1. `extract_topics(user_input)` 产出 ≤ 3 个 topic 字符串
2. 每个 topic 调 `db.boost_keyword(topic, &related, now_ms)`
3. `boost_keyword` (`memory_db.rs:584-637`)：
   - **INSERT 分支**：`weight = 1.0, mention_count = 1`（首次提及——`memory_db.rs:627-629`）
   - **UPDATE 分支**：`weight = (weight + 1.0).min(5.0), mention_count += 1`（后续提及——`memory_db.rs:608-617`）
4. 写入 `keyword_weights.keyword` 列的就是 topic 字符串本身——不做内部切词

### 检索路径（`search_facts` 内的 keyword fold, `memory_db.rs:531-540`）

```rust
let fact_tokens: HashSet<String> = segment_for_fts(&text).split_whitespace().collect();
let keyword_weight = keyword_map.iter()
    .filter(|(kw, _)| kw.chars().count() >= 2
                     && segment_for_fts(kw).split_whitespace().any(|t| fact_tokens.contains(t)))
    .map(|(_, w)| *w).fold(1.0, f64::max);
```

关键匹配条件：
- `kw.chars().count() >= 2`：keyword 字符数 ≥ 2（首次提及写入的 1 字符 topic 永远命中不了，但 `extract_topics` 强制 CJK ≥ 2、ASCII ≥ 3，所以这里只是兜底）
- `segment_for_fts(kw).split_whitespace().any(|t| fact_tokens.contains(t))`：把 keyword 也用同样的 `segment_for_fts` 切，**任一切片**出现在 fact_tokens 集合里就算命中

`segment_for_fts` (`memory_db.rs:883-912`) 的切词策略：
- 仅按 `is_whitespace()` 切
- ASCII 段（连续非空白、非 CJK 字符，包括 `-_.+/`）整体作为 1 个 token
- CJK 段：1 个孤立 CJK 字符 → 输出该字符；≥ 2 个连续 CJK → 输出所有相邻二元对（bigrams），不输出单字符

### 三个具体例子

| 输入 topic | `extract_topics` 产出 | keyword_weights 行 | 检索侧能否命中 | 备注 |
| --- | --- | --- | --- | --- |
| `pnpm` | `["pnpm"]` | `{ keyword: "pnpm", weight: ≥ 1.0 }` | **能** — fact 文本含 `"pnpm"` 时，fact_tokens 必有 `"pnpm"`，与 keyword 切出的 `["pnpm"]` 重叠 | 信号完全活 |
| `src/agentic` | `["src/agentic"]`（连接符保留为 token 内部） | `{ keyword: "src/agentic", weight: ≥ 1.0 }` | **条件性** — 仅当 fact 文本字面保留连续 `"src/agentic"`（无空白打断）才命中；若 fact 是 `"edited the src and agentic"`（被隔开）则 fact_tokens = `{"src", "agentic", ...}`，与 keyword 切出的 `["src/agentic"]` 无交集 | 见 I-1；本任务范围内不可修 |
| CJK 段 `"用户偏好使用中文回复"` | `["用户偏好使用中文回复"]`（整段视为一个 topic，不切词——`topics/extract.rs:192-198`） | `{ keyword: "用户偏好使用中文回复", weight: ≥ 1.0 }` | **能** — `segment_for_fts` 把 keyword 与 fact 都切成相同的 bigrams（`用户 用偏 偏好 好使 使用 用中 中文 文回 回复`），任何一对共用 bigram 即命中 | CJK 段是信号最自然的形态 |

### 结论
- **信号对 `pnpm` 这类简单 ASCII 与 CJK 段完全活了**——这是本任务预期覆盖的主要场景。
- **对路径型 / 含连接符的 topic 仅条件性活着**——需要 fact 文本保留连接符的连续性。这是 `extract_topics` 保留连接符与 `segment_for_fts` 不按连接符切词的策略错位导致的，**不是本任务引入的回归**（`search_facts` 这条路径在本次 diff 中未改），但它是本任务最有价值的发现：
  - 即便 boost 通道接上，对路径型 topic 的检索信号仍是稀疏的
  - 这条结论应该列入后续 `topics::score` 接线（或对 `segment_for_fts` 做连接符切分对齐）的 triage 清单
- **首次提及 = 基线**的设计本身不会留下"依赖提及过就 > 1.0"的隐患——`search_facts` 的 fold 初值是 1.0、boost 后首次仍 = 1.0，等价于未提及；排序单调性不破。

---

## 5. 额外重点结论

| 重点 | 结论 | 证据 |
| --- | --- | --- |
| 首次提及 = 基线是否留下隐患 | 否 | fold 初值 = 1.0；boost 后首次仍 = 1.0；等价于"未提及"；不破坏排序单调性 |
| decay 频次（双重 decay 风险） | 全仓生产路径下 decay 只调一次 | `rg decay_all_weights`：生产调用点 1 处（`growth_adapter.rs:224`）；测试调用点 1 处（`memory_db_tests.rs:388` 直接测试 floor 行为） |
| 上限 + 底线相互作用（抖动） | 可接受，幅度 ≈ 1% | 5.0 → boost+cap → 5.0 → decay → 4.95 → boost → 5.0 → ...；相对 bm25 与 recency_boost 量级排序稳定 |
| 测试 6 降级是否诚实 | 是 | `memory_db.rs:639-653` 的 `get_keyword_weight` 只读 `weight` 列；`memory_db.rs` 中没有读 `related_keywords` 列的 `pub(crate)` 方法——降级合理；代价是失去共现图内容正确性的覆盖（见 M-3） |
| 测试 4（500 次冷却）是否走空输入路径 | 是 | `growth_adapter.rs:602-604`：第一次 boost `"以后依赖安装都用 pnpm"` → 接下来 500 次 `boost_turn_topics(&db, "", now + i)`；`extract_topics("")` 返回空 vec → 没有 boost，仅 decay；不是偷偷跳过 |

---

## 6. 无法判定项

| 项 | 说明 |
| --- | --- |
| 测试 1b 第 2 次提及实测值 1.98 | 报告 §7 注明通过 `println!` 临时调试得到（运行后已删除）；diff 中只能看到断言 `w >= 1.95 && w <= 2.0`，数学 `(1.0 + 1.0) * 0.99 = 1.98` 与断言区间一致，但具体浮点结果未作为长期证据留下 |
| 测试 4 的 500 次后实测值 1.0 | 同上；断言 `w >= 1.0 - WEIGHT_EPS` 与数学推导一致，但具体浮点结果未留下 |
| 测试 6 降级对共现图内容正确性的间接覆盖强度 | `boost_keyword` 自身的 `memory_db_tests.rs` 测试覆盖了 `related_keywords` 字段的内容写入逻辑，但 `growth_adapter.rs::boost_turn_topics` 喂入的 `related` 切片是否与 `boost_keyword` 既有测试的形态对齐，无法从 diff 完全判定（需要静态推理 `for ... filter ... map ... collect` 切片与既有测试的等价性） |

---

## 7. 终判

**APPROVED WITH NOTES**：
- 任务范围内所有约束 1-10 通过。
- 双判决（spec 合规 + 代码质量）均 PASS。
- 一条 Important finding（I-1）属"信号激活度"的边界条件，属 brief 之外、不可在本任务范围修的内容，列入 triage。
- 无 Critical finding；无需要 fixer 的回归。

下一步建议：
1. 通过审查，追加 ledger 行（按编排者约定格式）。
2. I-1 转入后续 `topics::score` 接线任务的前置 triage（不阻塞本任务）。
3. 终审阶段一并复盘 I-1 是否需要新任务授权。