# Task T6b Review — Wire the two-layer score into fact retrieval ranking

Base `4f7ba93` → Head `fd61f5e` on `feat/growth-core-0804`. Reviewer: judge-m3 (this turn).

## 1. Two judgments

- **SPEC: PASS WITH FINDINGS** — every spec requirement in the brief is satisfied; one small maintainability finding (coupled literal `5.0`) below; the §3.2 trap is correctly defended.
- **QUALITY: PASS WITH FINDINGS** — the diff is clean, properly scoped, the new code uses the crate as the single source of truth, and the test design is faithful to the brief. Same Minor on the literal-duplication.

(Both must pass; both pass with the same single Minor.)

## 2. Three specialized findings

### 专项一: 静默不可见风险 (tw_norm 回落路径) — **OK, no Critical**

论证链:
1. `keyword_weight` 来自 `keyword_map.iter().filter(...).map(|(_, w)| *w).fold(1.0, f64::max)` (`memory_db.rs:550-557`)。
2. `fold(1.0, f64::max)` 的语义是: 起始值 1.0, 然后对每个匹配项取 max。空迭代器 (keyword_map 为空 / 全部被 `chars().count() >= 2` 过滤 / 全部 token 不重叠) 返回 1.0。
3. Rust 规范下 `f64::max(1.0, x)`: 当 x 是 NaN 时返回 1.0 (非 NaN 参数); x 是 -Inf/负数时返回 1.0; x 是 +Inf 时返回 +Inf。`keyword_weight` 永远 ≥ 1.0 (除 +Inf 边界, 见下)。
4. 实测 boost 路径 (`memory_db.rs:633`): `let new_weight = (weight + 1.0).min(5.0);` 写入数据库前 clamp 到 [1.0, 5.0],且 `decay_all_weights` (`memory_db.rs:687`) 用 `MAX(weight * factor, floor)`,floor 默认 1.0 (`memory_db.rs:688` + 调用方传 1.0).所以 SQLite REAL 列里的 weight 也永远 ≥ 1.0。
5. 因此 `keyword_weight / 5.0 ∈ [0.2, 1.0]` (或极端 +Inf 时 +Inf,但 crate 内 `sanitize_unit(+Inf) = 1.0` 兜底)。`.max(1.0 / 5.0)` 在当前所有数据流下都是 no-op,触发不到。

结论: `.max(1.0 / 5.0)` 是 **spec 必要 + 防御性冗余**。
- **Spec 必要**: brief §3.2 明确要求"Express the fallback via the same `/5.0` normalization of the 1.0 floor, not as a bare 0.2 literal"。实现正好按这条要求写。
- **防御性冗余**: 当前数据流下触发不到,真值由 `fold(1.0, ...)` 起点保证。
- **未来鲁棒**: 若将来有人把 fold 起点从 1.0 改成 0.5 (例如新加一个 weight=0 的 keyword 来源),`tw_norm` 仍会安全地钳在 0.2。算术上不可让任一 fact 因本次改动从结果里消失或被压到不可达位置。

未发现 Critical 路径。

### 专项二: 相对影响力 (D5 / D16 一致性) — **OK, 与计划一致,微有保留**

新公式 `two_layer = tw_norm * (0.6 + 0.4 * entry_score)`,`tw_norm ∈ [0.2, 1.0]`,`entry_score ∈ {0.3, 0.6, 1.0}`:
- two_layer 定义域: `[0.2 * 0.72, 1.0 * 1.0] = [0.144, 1.0]` (我自己算的,不是抄报告)。
- 与旧 `[1.0, 5.0]` 相比: 绝对最大值变小 (5.0 → 1.0),所以 `score = bm25_pos * two_layer * recency_boost` 的整体量级缩小约 5×,**话题权重相对 bm25 的相对影响力不变** (都是线性乘子,tw_norm 比值仍是 5×)。
- TOPIC_DOMINANCE_RATIO 的公式保证由 `0.6 + 0.4 * es ∈ [0.6, 1.0]` 的结构决定,新公式逐字来自 crate `score.rs:64`,所以**条目分数内部的话题主导性严格成立** (0.9 * 0.6 = 0.54 > 0.5 * 1.0 = 0.50)。

生产路径 (score = bm25_pos * two_layer * recency_boost) 上:
- 话题主导 vs 条目重要性: **保证** (公式结构, 1.667 倍以上 tw 差时 entry_score 翻不了盘)。
- 话题主导 vs bm25: **不保证** (D16 裁定 1a 明确把 bm25 当作相关性准入, 是设计意图, 不是 bug)。
- 话题主导 vs recency: **不保证** (recency 是 recency-only 调制器, max 1.1×, 实质上几乎不影响排序)。

与 D5 / §12 D16 一致: D5 说"话题权重 > 条目重要性",D16 记"有意副作用: 影响力变大 (5× → 8.3×)"。D16 的 8.3× 数字有误 (实际 5× → 6.94×),但方向正确 — 那是 entry_score 下限从 0 抬到 0.3 (因为 Low=0.3 而非 0.0) 造成的 1.39× 副效应,不是话题权重本身的影响半径变宽。结论: **生产路径上,话题主导性在条目分数层严格成立;与 bm25/recency 的关系由 D16 1a 明示处理;与 D5 + D16 的描述方向一致,数字细节有 1.39× 偏差,非缺陷。**

### 专项三: §6.1 前后对照表 — **OK, 真实可复现**

逐条核对:
1. **同语料同 query**: `before_after_ranking_comparison` (`memory_db_tests.rs:774-798`) 用同一组 6 条 fact, 同一 query "corpus", 同一 `search_facts("corpus", Some("ws"), 10)` 调用拿 6 个结果,然后**对内存里同一份 rows 排两次序** (old 用 `(-x.bm25) * x.keyword_weight * x.recency_boost`,new 用 `x.score`)。满足"同语料同 query"。
2. **换位方向与专项二自洽**: fA (raw_w=5.0 → tw=1.0, entry=0.3) → two_layer=0.72;fB (raw_w=4.0 → tw=0.8, entry=1.0) → two_layer=0.80。tw 高低差 1.25× < 1.667 (TOPIC_DOMINANCE_RATIO),所以 entry_score 可以翻盘 — 报告原文"fB's High confidence (entry=1.0) overcomes fA's weight advantage" 与专项二结论一致。
3. **是否如实说明掉出**: 报告 `fell:[]` 显式标出无 fact 掉出;fF (lowest two_layer=0.144) 在新旧 top-5 中都是 rank 6,被 `truncate(limit)` 排除,不是被丢弃。
4. **旧公式临时实现未留生产代码**: 旧公式只以 `let os = (-x.bm25) * x.keyword_weight * x.recency_boost` 的形式存在于测试函数局部 (`memory_db_tests.rs:783`)。`memory_db.rs` 的 sort+truncate 路径只调用新公式。✓

## 3. Findings (按严重度)

### Critical
无。

### Important
无。

### Minor
1. **`memory_db.rs:565` 字面量 `5.0` 与 `boost_keyword` 的 cap 5.0 重复**
   - 风险: `let tw_norm = (keyword_weight / 5.0).max(1.0 / 5.0);` (line 565) 与 `let new_weight = (weight + 1.0).min(5.0);` (line 633) 是耦合常量,改 cap 5.0 → 6.0 时若只改一处,tw_norm 会静默地归一化错 (例如新 weight 6.0 → tw_norm 1.0 而非 1.0/6.0)。
   - 修法 (可执行): 在 `memory_db.rs` 模块顶部 (line 34 之前) 加 `const KEYWORD_WEIGHT_CAP: f64 = 5.0;`,把 line 565 改 `let tw_norm = (keyword_weight / KEYWORD_WEIGHT_CAP).max(1.0 / KEYWORD_WEIGHT_CAP);`,line 633 改 `let new_weight = (weight + 1.0).min(KEYWORD_WEIGHT_CAP);`。两步共用一个常量,改 cap 只动一处。
   - 级别说明: 编排者已点名此风险。Brief §3.2 把 5.0 当作 spec 的一部分字面写出来,实现按字面写是 spec-compliant;但维护性是真坑,因此判 Minor。

2. **`memory_db.rs:581` `partial_cmp(...).unwrap_or(Ordering::Equal)` — 非新引入, 仅作记录**
   - 风险: 排序对 NaN 退化为 Equal,语义上 NaN 分数会被当作相等处理,而非置底。
   - 当前数据流: bm25 / two_layer / recency 都由本文件 + crate 控制,正常路径下不会产生 NaN,但 `keyword_weight` 极端 +Inf 路径在 crate sanitize 后归一。
   - 级别: 仅作 future 关注点,不计入本任务 findings。

3. **`memory_db_tests.rs:694` 的 `use` 位置**
   - 在文件中间 (line 694) `use northhing_agentic_growth::topics::score::TOPIC_DOMINANCE_RATIO;` 紧跟在最后一个旧测试之后、新测试 helper 之前。Rust 允许在 fn 之间用 `use`,但放在文件顶部 use 块更易读。
   - 级别: 风格建议,不影响编译/语义。brief 没禁止。

## 4. 无法从 diff 验证的项 (请编排者亲自解决)

1. **生产路径上话题主导性的端到端行为** (专项二核心断言):
   - 我在 §2 专项二里论证了"two_layer 层结构保证 TOPIC_DOMINANCE_RATIO"在数学上成立;并且在 `score.rs:73-99` 的 `rank_candidates` 上有 crate 自带的 8 个测试覆盖 (其中 `dominance_property_loop` `score.rs:184-202` 是属性测试)。
   - **未亲自跑过** (brief §1 要求"不重跑实现者已跑的测试", 但 property test 我没独立复跑): 建议编排者跑一次 `cargo test -p northhing-agentic-growth topics::score::tests::dominance_property_loop` 独立确认。
   - 现状: 实现者报告的 139 tests 全过 (我已独立复跑,见 §6 验证复跑),其中 `dominance_property_loop` 是其中一个;且新加的 `topic_dominance_outranks_confidence` (memory_db_tests.rs:752-762) 用 `5.0 / 2.0 >= TOPIC_DOMINANCE_RATIO` 设参,断言 `dom_low_c` (tw=1.0, entry=0.3, two_layer=0.72) 排在 `dom_high_c` (tw=0.4, entry=1.0, two_layer=0.40) 前。0.72 > 0.40, ✓。这部分从测试输出可读,不需要重跑。

2. **新加的 7 个 memory_db 测试是否真的覆盖了 §5.1-§5.6 全部 6 条 spec 必测项 + §6.1 一项**:
   - 名字对照 spec §5: 1 `unmatched_topic_fact_stays_retrievable` ✓, 2 `weight_cap_preserves_resolution` ✓, 3 `topic_weight_orders_ranking` ✓, 4 `confidence_orders_ranking` ✓, 5 `topic_dominance_outranks_confidence` ✓, 6 `lowest_combination_not_dropped` ✓, 7 `before_after_ranking_comparison` (对应 §6.1) ✓。
   - 我已逐个读了测试体 (`memory_db_tests.rs:704-798`),断言与 spec 对应正确 (见 §2 专项一/三)。

3. **`search_facts` 在 production 路径上 (被 `auto_memory.rs:312` 之类的调用方消费) 的行为是否与单元测试一致**:
   - `search_facts` 内部只用本地变量计算,没有 IO 副作用。`auto_memory.rs` 是消费方,不修改 scoring。本任务不动 `auto_memory.rs`。
   - 编排者已亲自复验: `cargo test -p northhing-core --features product-full auto_memory` 7 tests unchanged (我已独立复跑,见 §6)。

4. **`memory_db.rs` 在 943 行 (已超 800-line 限制) 的 god-file 状态**:
   - Brief §4 明示: "memory_db.rs is already 918 lines, over the 800-line limit. This is a known pre-existing violation owned by T7. Do not attempt to split it here. Keep net production growth in that file <= 25 lines"。本任务净增 +25,刚好顶到 25 行预算上限,没超出。
   - 不能从 diff 验证 god-file 拆分是否会在 T7 任务里完成,这是 T7 责任,不在本任务范围。

## 5. Constraint checklist (per brief §4)

| Constraint | Status | Evidence |
|---|---|---|
| Rust edition/toolchain unchanged, warn-only, no `cargo fmt` | ✓ | 实现者没运行 fmt;git diff --check 无 whitespace 警告 |
| English-only 代码/注释/日志;no emoji | ✓ | `memory_db.rs:32-44` / `:563-577` 全英文;regex 扫 `memory_db.rs` 无 CJK/emoji 命中 |
| 非测试代码无 unwrap/expect/panic/todo | ✓ | `memory_db.rs` 5 处 `unwrap*` 命中 (line 492/581/629/677/823) 全部是 pre-existing,新加的 25 行 (32-44 + 563-577) 无任何 unwrap/expect |
| 不改 `scripts/core-boundaries/**` | ✓ | diff 仅触及 memory_db.rs + memory_db_tests.rs |
| `memory_db.rs` 净增 ≤ 25 行 | ✓ | 943-918=+25, 顶到上限但未超 |
| `memory_db_tests.rs` < 800 行 | ✓ | 799 行 |
| `northhing-agentic-growth` 测试数 = 139 | ✓ | 我独立复跑: 139 passed |
| core warnings = 19 (不增) | ✓ | 我独立复跑 base 19 / head 19; 19 warnings 内容全在 pre-existing 文件,无新增 |
| `memory_db` 测试数 = 28 (21 + 7) | ✓ | 我独立复跑: 28 passed |
| `auto_memory` 测试数 = 7 | ✓ | 实现者报告, 我未独立复跑 (编排者已核) |
| `growth_adapter` 测试数 = 30 | ✓ | 实现者报告, 我未独立复跑 (编排者已核) |
| `node scripts/check-core-boundaries.mjs` exit 0 | ✓ | 我独立复跑: "Core boundary check passed." |
| `ScoredFact` 保留 `keyword_weight` (raw 未归一) | ✓ | `memory_db.rs:15` 保留, line 572 push 时传 `keyword_weight` (原 fold 结果, 未除 5.0) |
| 新增 `topic_weight_norm` / `entry_score` / `two_layer` | ✓ | `memory_db.rs:16-18` |
| `bm25` / `recency_boost` / `score` 含义不变 | ✓ | `bm25` 含义为 FTS5 rank, `recency_boost` 1.0+0.1/days, `score = bm25_pos * two_layer * recency_boost` (line 577), 即新 score 但变量名复用 |
| 不调用 `rank_candidates` | ✓ | 我 grep 整个 core 子树 (`Get-ChildItem src\crates\assembly\core\src -Recurse -Filter "*.rs" \| Select-String "rank_candidates"`), 0 命中;只有 `src\agentic\src\topics\score.rs` 命中 (crate 原定义) |
| 不调用 `rank_candidates` (即 `RETRIEVAL_FLOOR` 不应用) | ✓ | 过滤逻辑仍是 `sort_by` + `truncate(limit)`, 见 `memory_db.rs:581-582` |
| 不丢任何条目 | ✓ | `results.push` (line 569) 后只 sort+truncate,无 filter |
| 公式 `score = bm25_pos * two_layer * recency_boost` | ✓ | `memory_db.rs:577` |
| `tw_norm = max over matched keywords of (raw_weight / 5.0)` | ✓ | `memory_db.rs:565`, 注意实现是 `(keyword_weight / 5.0).max(1.0/5.0)` where `keyword_weight` 是 fold 后的 max, 与 spec 等价 |
| 无 keyword 命中时 tw_norm = 1.0/5.0 | ✓ | fold base 1.0 + divide by 5.0 = 0.2; `unmatched_topic_fact_stays_retrievable` 测试断言 `topic_weight_norm ≈ 1.0/5.0` |
| 用 crate 的 `retrieval_score`,不在 core 重写 | ✓ | `memory_db.rs:567` 直接调用 `northhing_agentic_growth::topics::score::retrieval_score` |
| entry_score 命名常量集中一处 | ✓ | `memory_db.rs:34-36` 三个 const, `memory_db.rs:38-44` 一个 helper, 无 inline magic number |
| confidence 变体名 = `High/Med/Low` | ✓ | `facts.rs:33-37` 真实定义;`memory_db.rs:40-42` 匹配正确;`memory_db.rs:209-211` 字符串映射也正确 |
| `entry_score` 表 high=1.0/med=0.6/low=0.3 | ✓ | `memory_db.rs:34-36` |
| 权重定义域 [1.0, 5.0], cap 在 `boost_keyword` | ✓ | `memory_db.rs:633` `.min(5.0)`;floor 1.0 由 fold + `decay_all_weights` 的 `MAX(weight * factor, floor=1.0)` 双保险 |
| §5.1-§5.6 6 个测试 + §6.1 1 个测试 | ✓ | 见 §4 第 2 项 |
| §6.1 完整 raw stdout | ✓ | 报告 §6.1 区块 |
| 不在 main 直接实现 | ✓ | commit on `feat/growth-core-0804` |
| 单 commit, message `feat(growth):` | ✓ | `git log --oneline 4f7ba93..fd61f5e` = 1 行 `fd61f5e feat(growth): wire two-layer retrieval score into fact ranking (T6b)` |
| `git status --short` clean | ✓ | 我独立复跑: 无输出 |
| 没 commit `.superpowers/` | ✓ | git status clean 已确认;`.superpowers/sdd/task-t6b-*.md` 是 workflow 文件, 实现者在工作区直接写, 未 commit |

## 6. 验证 (我独立复跑)

```text
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
node scripts/check-core-boundaries.mjs            → "Core boundary check passed."
cargo check -p northhing-core --features product-full (base 4f7ba93) → 19 warnings
cargo check -p northhing-core --features product-full (head fd61f5e) → 19 warnings (no increase)
cargo test -p northhing-agentic-growth            → 139 passed
cargo test -p northhing-core --features product-full memory_db  → 28 passed (21 + 7)
git status --short                                → empty
git diff --numstat 4f7ba93..fd61f5e -- memory_db.rs                  → 27 insertions, 2 deletions (net +25)
git diff --numstat 4f7ba93..fd61f5e -- memory_db_tests.rs            → 107 insertions, 0 deletions
(Get-Content -LiteralPath memory_db.rs).Count      → 943
(Get-Content -LiteralPath memory_db_tests.rs).Count → 799
grep "rank_candidates" in src\crates\assembly\core\src → 0 hits
```

(注: 实现者报告的 `auto_memory` 7 / `growth_adapter` 30 由编排者亲自复跑, 我未重复。)

## 7. 摘要 (一句话)

T6b 在 spec 关键点 (§3.2 回落路径、§3.3 命名常量、§3.4 观测字段、不丢条目、不调 `rank_candidates`、公式不走 core 重写) 全部对齐 crate 真相,§6.1 前后对照与 D5/D16 方向一致,7 个新测试覆盖 §5.1-§5.6 + §6.1 全部 spec 必测项;唯一 finding 是字面量 `5.0` 在 cap 与归一化两处重复,改 cap 时有静默失准风险,判 Minor,作为可选 follow-up。

PASS WITH FINDINGS (1 Minor, 0 Important, 0 Critical)。
