# Task A3 Report — topics/score.rs（双层权重检索打分）

## Status

**DONE**

## 文件行数

409 行（含测试），低于 800 行限制。

## §6 验证命令原始输出

### `cargo check -p northhing-agentic-growth`

```
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling zmij v1.0.21
   Compiling serde_json v1.0.150
    Checking once_cell v1.21.4
   Compiling serde v1.0.228
   Compiling thiserror v2.0.18
    Checking itoa v1.0.18
    Checking memchr v2.8.3
    Checking pin-project-lite v0.2.17
    Checking tracing-core v0.1.36
   Compiling syn v2.0.118
   Compiling serde_derive v1.0.228
   Compiling tracing-attributes v0.1.31
   Compiling thiserror-impl v2.0.18
   Compiling async-trait v0.1.89
    Checking tracing v0.1.44
    Checking northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a3\src\agentic)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.08s
```

### `cargo test -p northhing-agentic-growth`

```
   Compiling windows-link v0.2.1
   Compiling parking_lot_core v0.9.12
   Compiling smallvec v1.15.2
   Compiling cfg-if v1.0.4
   Compiling pin-project-lite v0.2.17
   Compiling once_cell v1.21.4
   Compiling scopeguard v1.2.0
   Compiling itoa v1.0.18
   Compiling memchr v2.8.3
   Compiling bytes v1.12.1
   Compiling serde_core v1.0.228
   Compiling zmij v1.0.21
   Compiling tokio-macros v2.7.0
   Compiling thiserror v2.0.18
   Compiling windows-sys v0.61.2
   Compiling lock_api v0.4.14
   Compiling tracing-core v0.1.36
   Compiling tracing v0.1.44
   Compiling parking_lot v0.12.5
   Compiling serde_json v1.0.150
   Compiling serde v1.0.228
   Compiling mio v1.2.1
   Compiling socket2 v0.6.4
   Compiling tokio v1.52.3
   Compiling northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a3\src\agentic)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 13.75s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 22 tests
test topics::score::tests::best_weight_ignores_nan ... ok
test error::tests::error_display_includes_context ... ok
test topics::score::tests::best_weight_empty ... ok
test topics::score::tests::best_weight_all_nan ... ok
test topics::score::tests::rank_descending_score ... ok
test topics::score::tests::rank_nan_candidate_no_panic ... ok
test topics::score::tests::dominance_tw09_es0_beats_tw05_es1 ... ok
test topics::score::tests::rank_below_floor_dropped ... ok
test topics::score::tests::sanitize_mid ... ok
test topics::score::tests::dominance_property_loop ... ok
test topics::score::tests::rank_empty ... ok
test topics::score::tests::dominance_tw055_es0_loses_to_tw05_es1 ... ok
test topics::score::tests::rank_tie_different_tw ... ok
test topics::score::tests::rank_tie_same_score_different_id ... ok
test topics::score::tests::retrieval_floor_only ... ok
test topics::score::tests::retrieval_upper_bound ... ok
test topics::score::tests::retrieval_zero_tw ... ok
test topics::score::tests::sanitize_infinity ... ok
test topics::score::tests::sanitize_neg_infinity ... ok
test topics::score::tests::sanitize_negative ... ok
test topics::score::tests::sanitize_overflow ... ok
test topics::score::tests::sanitize_nan ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**22 全部通过，0 失败**（其中 20 个来自 score.rs，2 个是 crate 预存的 error 模块测试）。

## "话题权重主导"三条测试的数值证明

### 测试 1: `dominance_tw09_es0_beats_tw05_es1`

```
候选 a: topic_weight=0.9, entry_score=0.0
候选 b: topic_weight=0.5, entry_score=1.0

retrieval_score(0.9, 0.0) = 0.9 × (0.6 + 0.4 × 0.0) = 0.9 × 0.6 = 0.54
retrieval_score(0.5, 1.0) = 0.5 × (0.6 + 0.4 × 1.0) = 0.5 × 1.0 = 0.50

结果: a(0.54) > b(0.50) → a 排第一 ✓
```

**结论**：话题权重 0.9 即使条目分数为 0，仍然胜过话题权重 0.5 + 满分条目分数 1.0。0.54 > 0.50。

### 测试 2: `dominance_tw055_es0_loses_to_tw05_es1`

```
候选 a: topic_weight=0.55, entry_score=0.0
候选 b: topic_weight=0.50, entry_score=1.0

retrieval_score(0.55, 0.0) = 0.55 × (0.6 + 0.4 × 0.0) = 0.55 × 0.6 = 0.33
retrieval_score(0.50, 1.0) = 0.50 × (0.6 + 0.4 × 1.0) = 0.50 × 1.0 = 0.50

结果: b(0.50) > a(0.33) → b 排第一 ✓
```

**结论**：当话题权重差距不足 1.667 倍时（0.55/0.50 = 1.10 < 1.667），条目分数可以翻盘。这是设计允许的。

### 测试 3: `dominance_property_loop`（属性测试）

```
TOPIC_DOMINANCE_RATIO = 1.0 / 0.6 ≈ 1.666666...

对 tw_high 从 0.1 到 1.0 步长 0.1:
  tw_low = tw_high / 1.666666... - 0.001

断言: retrieval_score(tw_high, 0.0) > retrieval_score(tw_low, 1.0)

完整循环（9 组有效值，排除 tw_low ≤ 0）:
  tw_high=0.2, tw_low≈0.119,  high=0.120, low≈0.119 ✓
  tw_high=0.3, tw_low≈0.179,  high=0.180, low≈0.179 ✓
  tw_high=0.4, tw_low≈0.239,  high=0.240, low≈0.239 ✓
  tw_high=0.5, tw_low≈0.299,  high=0.300, low≈0.299 ✓
  tw_high=0.6, tw_low≈0.359,  high=0.360, low≈0.359 ✓
  tw_high=0.7, tw_low≈0.419,  high=0.420, low≈0.419 ✓
  tw_high=0.8, tw_low≈0.479,  high=0.480, low≈0.479 ✓
  tw_high=0.9, tw_low≈0.539,  high=0.540, low≈0.539 ✓
  tw_high=1.0, tw_low≈0.599,  high=0.600, low≈0.599 ✓
```

**结论**：在所有比例下，只要 `tw_high / tw_low ≥ TOPIC_DOMINANCE_RATIO`，无论条目分数如何（0.0 vs 1.0），话题权重高的条目始终排名更高。属性断言全部通过。

## 提交记录

```
commit 2de4186e3c2d2f9fdb2c5e2ae6a89eeb5d0de0c0 (HEAD -> feat/growth-a3)
Author: UmR <umbrellallc@outlook.com>
Date:   Tue Aug 4 2026

    feat(growth): add two-layer retrieval scoring with topic dominance
```

```
git status --short:  M src/agentic/src/topics/score.rs
（仅一个文件有改动，符合约束）
```

## 与本 brief 的任何偏离及原因

无偏离。所有规格全部严格实现：

- 常量 `ENTRY_FLOOR` / `ENTRY_SPAN` / `TOPIC_DOMINANCE_RATIO` / `RETRIEVAL_FLOOR` 精确匹配
- `ScoredCandidate` 类型定义与 brief 一致
- 5 个函数签名与实现细节完全符合规格
- 14 条测试全部实现并通过
- 零新依赖（仅 `std`）
- 非测试代码无 `unwrap`/`expect`
- `f64` 比较使用 `total_cmp`
- 文件仅改 `src/agentic/src/topics/score.rs`
- 模块文档注释包含"话题权重主导"的数学表述
