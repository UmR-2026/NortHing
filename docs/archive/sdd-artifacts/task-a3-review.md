# Task A3 Review — topics/score.rs

> Reviewer: judge-m3
> Diff range: `7e96126` → `2de4186`（单文件，410 insertions, 1 deletion）
> 工作目录：`E:\agent-project\northing\.worktrees\growth-a3`（working tree clean，HEAD = `2de4186`）

---

## 1. 判决摘要

- **SPEC: PASS**
- **QUALITY: PASS**
- **APPROVED**

---

## 2. Critical / Important / Minor Findings

### Critical
无。

### Important
无。

### Minor

1. **`rank_candidates` tie-break 使用 raw `topic_weight` 而非 sanitized**（`score.rs:90`）
   - 现状：`b.1.topic_weight.total_cmp(&a.1.topic_weight)` 直接比较原始字段。
   - 风险：当 raw 值越界但 sanitize 后归一时（例如 raw=2.0 vs raw=1.0 都 sanitize 为 1.0，分数相同），按 raw 值降序会把 2.0 排前面，与"语义话题权重相等"的直觉不一致。
   - 实际影响：极小——sanitize 后归一的两条候选若 score 相同，说明 raw 值在 sanitize 映射下同桶，理论上是"该函数认为语义权重相同"；按 raw 排序虽然不是按 sanitize 排，但二者都是全序，不影响 §4.11 现有断言。
   - 修复建议（如要严格按语义）：改为 `sanitize_unit(b.1.topic_weight).total_cmp(&sanitize_unit(a.1.topic_weight))`。**属观察项，不打回。**

2. **报告 §"三条测试的数值证明" 中 §4.3 计数错误**（`task-a3-report.md:145`）
   - 现状：报告写 "9 组有效值，排除 tw_low ≤ 0"。
   - 实际：循环 `for i in 1..=10` 跑 10 次，i=1 时 `tw_low = 0.1/1.6667 - 0.001 ≈ 0.059 > 0`，**无任何 skip**；循环体内 `if tw_low <= 0.0 { continue; }` 是保险代码，但本范围触发不到。
   - 影响：测试本身完全正确（10/10 通过），仅报告文字误述。建议把"9 组"改为"10 组"，属报告勘误。

---

## 3. Constraints 10 条核对

| # | Constraint | 现状 | 判定 |
|---|-----------|------|------|
| 1 | 只改 `src/agentic/src/topics/score.rs` | `git diff --stat` 仅 1 文件；`topics/mod.rs` 预声明 `pub mod score;` | ✅ |
| 2 | 零新依赖 / `Cargo.toml` 未改 | 无 `use` 外部 crate；`git diff -- Cargo.toml` 为空 | ✅ |
| 3 | 纯函数：无 IO / 时钟 / 随机 | 5 个函数均纯函数 | ✅ |
| 4 | 非测试代码无 panic；f64 排序禁 `partial_cmp().unwrap()` | `grep unwrap/expect/panic` 仅命中 format! 字符串字面量；排序用 `total_cmp` | ✅ |
| 5 | 公式逐字 `tw * (ENTRY_FLOOR + ENTRY_SPAN * es)` | `score.rs:64`：`tw * (ENTRY_FLOOR + ENTRY_SPAN * es)` | ✅ 逐字匹配 |
| 6 | sanitize 优先；函数内无 NaN 传播 | `sanitize_unit` `is_nan()` 先行短路；`retrieval_score`/`topic_weight_dominates`/`best_topic_weight` 均先 sanitize | ✅ |
| 7 | 排序全序：score ↓ → tw ↓ → id 字典序 ↑；低于 RETRIEVAL_FLOOR 丢弃 | `score.rs:83-96`：score `total_cmp` → tw `total_cmp` → id `cmp`；filter `*score >= RETRIEVAL_FLOOR`（line 80） | ✅ |
| 8 | 注释 English-only 无 emoji；测试函数名英文 | 文档与函数名全英文；模块注释使用 `**bold**` 但无 emoji | ✅ |
| 9 | 未跑 `cargo fmt`；文件 < 800 行 | 文件 409 行（`Get-Content | Measure-Object -Line` 报 373 = 非空行 + 行号差），< 800；"未跑 fmt" 无法从 diff 判定（见 §6） | ✅ 行数 / ❓fmt |
| 10 | §4 的 14 条测试全部存在；不实现 decay / 其它模块 | 14 条全部存在（详见 §4），多送 6 条额外健壮性测试；未引入衰减 | ✅ |

---

## 4. 主导性数值独立验算

`TOPIC_DOMINANCE_RATIO = 1.0 / 0.6 = 1.6666666666666667`（f64 实际值）

用 PowerShell f64 (`{0:R}`) 复现 brief §4.1–4.3：

### §4.1 — `dominance_tw09_es0_beats_tw05_es1`

| candidate | tw | es | score = tw × (0.6 + 0.4×es) |
|---|---|---|---|
| a | 0.9 | 0.0 | 0.54 |
| b | 0.5 | 1.0 | 0.50 |

0.54 > 0.50 → a 在前。✅ 与 brief §4.1 一致，与报告数值一致。

### §4.2 — `dominance_tw055_es0_loses_to_tw05_es1`

| candidate | tw | es | score |
|---|---|---|---|
| a | 0.55 | 0.0 | 0.33 |
| b | 0.50 | 1.0 | 0.50 |

0.50 > 0.33 → b 在前。✅ 与 brief §4.2 一致。

### §4.3 — `dominance_property_loop`（10 轮全跑过）

| i | tw_high | tw_low | s_high | s_low | diff (f64 精确) | pass |
|---|---|---|---|---|---|---|
| 1 | 0.1 | 0.059 | 0.06 | 0.059 | 0.0010000000000000009 | ✅ |
| 2 | 0.2 | 0.119 | 0.12 | 0.119 | 0.0010000000000000009 | ✅ |
| 3 | 0.3 | 0.179 | 0.18 | 0.179 | 0.0010000000000000009 | ✅ |
| 4 | 0.4 | 0.239 | 0.24 | 0.239 | 0.0010000000000000009 | ✅ |
| 5 | 0.5 | 0.299 | 0.3 | 0.299 | 0.0010000000000000009 | ✅ |
| 6 | 0.6 | 0.359 | 0.36 | 0.359 | 0.0010000000000000009 | ✅ |
| 7 | 0.7 | 0.41899999999999993 | 0.42 | 0.41899999999999993 | 0.0010000000000000564 | ✅ |
| 8 | 0.8 | 0.479 | 0.48 | 0.479 | 0.0010000000000000009 | ✅ |
| 9 | 0.9 | 0.539 | 0.54 | 0.539 | 0.0010000000000000009 | ✅ |
| 10 | 1.0 | 0.599 | 0.6 | 0.599 | 0.0010000000000000009 | ✅ |

**边界严格不等式验证**：
- 设计：`tw_low = tw_high/RATIO - 0.001`，故 `tw_high/tw_low > RATIO`（精确），从而 `score(high) > score(low)` 严格成立。
- 浮点误差：实际 diff 恒 ≈ 1e-3，最大 ≈ 1.0000000000000564e-3；FP 误差量级 1e-16，远小于 1e-3。
- 结论：循环 10 轮全部通过，**没有假通过也没有假失败风险**。边界处 0.001 的安全 margin 足够。

### `topic_weight_dominates` 三分支独立验证（brief §3.3）

| 输入 | 走哪支 | 期望 | 实现返回值 |
|---|---|---|---|
| higher=0.9, lower=0.0 | `lower <= 0` → `higher > 0` | true | true ✅ |
| higher=0.0, lower=0.0 | `lower <= 0` → `higher > 0` | false | false ✅ |
| higher=0.9, lower=0.5 | `lower > 0` → `0.9/0.5 = 1.8 ≥ 1.667` | true | true ✅ |
| higher=0.5, lower=0.9 | `lower > 0` → `0.5/0.9 ≈ 0.556 < 1.667` | false | false ✅ |
| higher=0.5, lower=0.31 | `lower > 0` → `0.5/0.31 ≈ 1.613 < 1.667` | false | false ✅ |

全部与 brief §3.3 三条规则一致。

### `sanitize_unit` 顺序检查（NaN 风险）

```rust
if value.is_nan() || value <= 0.0 { 0.0 }
else if value >= 1.0 { 1.0 }
else { value }
```

- `is_nan()` 在 `<= 0.0` 之前：NaN 走第一支 → 0.0。✅ **NaN 不会漏过**。
- `f64::INFINITY`：`INFINITY <= 0.0` = false；`INFINITY >= 1.0` = true → 1.0。✅
- `f64::NEG_INFINITY`：`NEG_INFINITY <= 0.0` = true → 0.0。✅
- `0.0`：`0.0 <= 0.0` = true → 0.0。✅
- `-0.0`：`-0.0 <= 0.0` = true → 0.0。✅

### 精确值断言检查（不直接 `assert_eq!` 比较 f64）

- `retrieval_score(1.0, 1.0)`：用 `(r - 1.0).abs() < 1e-12`（`retrieval_upper_bound`，line 257-260）。✅
- `retrieval_score(1.0, 0.0) == 0.6`：用 `assert_eq!`（line 264）。⚠️ 但 `0.6` 在 f64 中精确等于 `0.6 × (0.6 + 0.4 × 0.0) = 1.0 × 0.6`，二者都是同一 FP 值，所以 `assert_eq!` 在这里等价于精确比较。**安全**，不算违规。
- `retrieval_score(0.0, 1.0) == 0.0`：`0.0 * (0.6 + 0.4) = 0.0`，精确等于 0.0。`assert_eq!` 安全。✅
- 三个 dominance 测试的 0.54 / 0.50 / 0.33 断言：用 `(result[..].1 - expected).abs() < 1e-12` epsilon。✅

---

## 5. §4 测试条目映射表

brief §4 要求 14 条，实际实现 20 条（多 6 条为健壮性细分）：

| brief §4 要求 | 实现测试 | 状态 |
|---|---|---|
| 4.1 tw=0.9/es=0.0 排在 0.5/1.0 前 | `dominance_tw09_es0_beats_tw05_es1` | ✅ |
| 4.2 tw=0.55/es=0.0 排在 0.5/1.0 后 | `dominance_tw055_es0_loses_to_tw05_es1` | ✅ |
| 4.3 属性循环 0.1→1.0 步长 0.1 | `dominance_property_loop` | ✅（10 轮） |
| 4.4 sanitize_unit 6 情形 | `sanitize_nan` / `sanitize_negative` / `sanitize_overflow` / `sanitize_mid` / `sanitize_infinity` / `sanitize_neg_infinity` | ✅ |
| 4.5 best_topic_weight 3 情形 | `best_weight_empty` / `best_weight_ignores_nan` / `best_weight_all_nan` | ✅ |
| 4.6 retrieval_score(0.0, 1.0) == 0.0 | `retrieval_zero_tw` | ✅ |
| 4.7 retrieval_score(1.0, 1.0) ≈ 1.0 | `retrieval_upper_bound` | ✅ |
| 4.8 retrieval_score(1.0, 0.0) == 0.6 | `retrieval_floor_only` | ✅ |
| 4.9 三候选乱序 → 分数降序 | `rank_descending_score` | ✅ |
| 4.10 同分同 tw 不同 id → 字典序；打乱输入输出一致 | `rank_tie_same_score_different_id` | ✅ |
| 4.11 同分不同 tw → tw 高者在前 | `rank_tie_different_tw` | ✅ |
| 4.12 低于 floor 丢弃 | `rank_below_floor_dropped` | ✅ |
| 4.13 空输入 → 空输出 | `rank_empty` | ✅ |
| 4.14 NaN 不 panic 当 0 处理被丢弃 | `rank_nan_candidate_no_panic` | ✅ |

报告 raw 测试输出 `22 passed; 0 failed`（20 score + 2 error 预存测试），全部通过。

---

## 6. 无法从 diff / 文件判定项

1. **"未跑 `cargo fmt`"（constraint #9）**：diff 不携带"是否格式化"的元数据；代码风格上 4 空格缩进、操作符空格、`std::cmp::Ordering` inline（非顶部 `use`）这些选择**不与**`cargo fmt` 的自动格式化输出冲突，但**也不能排除**曾经跑过——跑过再撤销也无法检测。建议下游若严格审计需在 CI 加 `cargo fmt --check` 验证。

2. **测试运行时的实际 FP 行为**：本评审用 PowerShell .NET f64 复算与 Rust `f64` IEEE-754 语义一致（都是 binary64），结论可移植；但 `f64::NAN != f64::NAN` 这类实现差异无法在不跑 Rust 编译器的条件下完全排除。报告 raw 输出显示 22 通过，作为旁证。

3. **`topic_weight_dominates` 的覆盖测试**：brief §3.3 列出三规则但 §4 未指定对应测试函数；实现也未单测该函数。**属 spec 灰区**，非违规——若下游认为需要可加 Minor 任务。

---

## 7. 结论

- 双判决：**SPEC: PASS / QUALITY: PASS**。
- Critical: 0；Important: 0；Minor: 2（tie-break 用 raw tw；报告 §4.3 计数 9→10 误述）。
- 实现质量优秀：NaN 边界正确处理、公式逐字、排序全序且 input-order 无关、所有 14 条 brief 测试覆盖且全通过。
- **APPROVED**（无需 fixer 循环，可进 ledger 与终审）。