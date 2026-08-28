# W6-D1 检查器语义修正 —— 独立仲裁判决书

**裁判**: 编排者（独立仲裁视角，只读审查，不改代码）
**日期**: 2026-08-28
**标的**: `scripts/verify-rot-budget.mjs` 的 `collectRustFiles` 是否应将测试文件从 `*_production` 指标中排除

---

## 裁决

> **APPROVE-FIX**

**理由一句话**: 指标名 `*_production` 即语义——只计生产代码；测试文件误入是测量口径缺陷，不是计数超标，属合法修正且已有既定先例。

---

## 问题逐条裁决

### Q1: 测量修正 vs. 规避闸门？

**结论: 测量修正。证据如下**

| 证据 | 来源 |
|------|------|
| 指标名 `_production` 已约定计量的对象是生产代码 | `rot-budget.json` 键名 |
| `unix_epoch_inline` 条目 note 明确记载 precedent | `rot-budget.json:24` — *"ceiling rebased 2026-08-21 to checker semantics (excludes tests dirs/_tests.rs)"* |
| 仓库自有 hygiene 脚本已将这些文件定义为测试代码 | `check-repo-hygiene.mjs:90` — `testFilePattern = /(_tests?\.rs$|\/tests\.rs$)/` |
| Rust 社区惯例：`tests.rs` = 模块级测试文件，`*_tests.rs` = 集成测试模块，`*_tests/` 目录 = 集成测试集合 | 磁盘实测 + 编码惯例 |

这不是"改让自己绿"——生产代码 rot 的真实值从未改变，改变的是测量程序是否对正确的对象做测量。

### Q2: 真实生产 rot 隐形增长风险？

**结论: 风险可控，已被既有机制覆盖。**

| 风险场景 | 覆盖机制 |
|---------|---------|
| 有人把生产代码放进 `tests.rs` 规避计数 | Code review：将 `.unwrap()` 等防御性代码移入测试文件会被审查标记；Cargo module 结构不认 `tests.rs` 为生产模块 |
| 有人新建 `foo_tests/` 目录藏生产 rot | Code review + IDE 项目树可见；目录命名本身就暴露意图 |
| 有人建 `test_foo.rs`（单数） | 此类文件不存在于磁盘；即使将来出现，修改 `collectRustFiles` 扩展排除是显式决策，不会偷渡 |

- 修正后计数最低的 `let_underscore` = 388，恰好等于 ceiling = 388，无冗余余量
- 修正后最低的 `unwrap_production` = 473，距 ceiling 502 有 29 点余量 —— rot 真增长仍会被捕捉
- 若有人持续往测试文件塞 `.unwrap()`，天花板效应是有限的（测试代码也有合理 unwrap 需求），但生产代码的 rot 增长仍由生产文件独立反映

### Q3: 是否违反家规 7 "ceiling 只降不升" 的精神？

**结论: 不违反。**

家规 7 原文：*"`rot-budget.json` ceilings may only go down in normal commits"*。:

- **Ceiling 数值未变**：502 / 1089 / 388 / 109 全部原地不动
- **计数因口径修正下降**：测量对象从"src/ 下所有 .rs"收窄为"src/ 下生产 .rs"
- 这与 W6-1 已完成的 `allow_dead_code 128→106` 同道——后者是生产代码真减才降，前者是口径修正才降，但 ceiling 本身都不动
- `unix_epoch_inline` 2026-08-21 的先例已经建立了同一模式：*"ceiling rebased to checker semantics"*，ceiling 值保留，排除规则更新

**判定**: 口诀正确解读是"ceiling 值不能上调"，不是"修正口径导致计数下降违规"。

### Q4: 最小修正范围是否充足？有无漏网？

**磁盘实测结果**:

| 模式 | 磁盘实际存在 | 当前排除 | 修正后排除 | 漏网？ |
|------|------------|---------|-----------|--------|
| 文件名为 `tests.rs` | **15 个** | ❌ 不排除 | ✅ 排除（规则①） | 是→修正覆盖 |
| 文件名为 `*_tests.rs` | **18 个** | ✅ 已有排除（L44 `endsWith('_tests.rs')`） | ✅ 仍排除 | 否 |
| 目录段以 `_tests` 结尾 | **1 目录**，内含 6 个 .rs | ❌ 不排除 | ✅ 排除（规则②） | 是→修正覆盖 |
| 文件名为 `test.rs` | **0 个** | — | — | 无此类文件 |
| 文件名为 `*.test.rs` | **0 个** | — | — | 无此类文件 |

**结论**: 两条排除规则已覆盖"磁盘实际存在的全部测试惯例文件"，无需第三条。专有名词：已知不存在（`test.rs` / `.test.rs`）不是"需要处理但漏了"，是"仓库不采用此惯例"。

### Q5: 若拒绝的条件与替代方案

**不适用**——裁决为 APPROVE，不进入此分支。

但附带记录：若将来有人提出"不修 checker，改生产代码"的主张，可行性评估如下：

- 当前 `unwrap` 超 ceiling 45 处、`expect` 超 169 处
- 其中部分是防御性 unwrap（FFI 转换、配置加载等合法场景），不可简单删
- 真减需要逐处 review 合法性 + 写 `expect`/`Result` 替代路径 → 工程量大且含风险
- vs. 修正 checker：零改动生产代码，两行条件，precedent 已立
- **成本差距：数个量级**

---

## 精确语义定义（映射到代码）

修正后 `collectRustFiles` 的排除语义：

```
一个 .rs 文件被排除当且仅当以下任一条件成立：
  A. 它是目录 entry，且目录名 === 'tests'
  B. 它是目录 entry，且目录名.startsWith('target')
  C. 它是目录 entry，且目录名 === '.git' || === 'node_modules'
  D. 它是文件，且文件名 === 'tests.rs'              ← 新增 #1
  E. 它是文件，且文件名.endsWith('_tests.rs')
  F. 它是文件，且其路径段中存在段 === 'tests'        ← 已有 L47
  G. 它是文件，且其路径段中存在段.startsWith('target')← 已有 L47
  H. 文件在 EXEMPT_FILE_PATHS 中                     ← 已有
  I. 任意路径段以 '_tests' 结尾 → 目录递归跳过       ← 新增 #2
```

新增的两条在代码层面的精确实现：

```javascript
// #1 文件名恰好为 tests.rs（文件级排除）
!entry.name === 'tests.rs'   // L44 条件从 仅排除 _tests.rs 扩展为 排除 tests.rs

// #2 目录段以 _tests 结尾（目录递归级排除）
entry.name.endsWith('_tests')  // L34-41 的目录跳过程序中增加此项
```

> 注：规则 #2 已在目录递归跳过程序处理，因此 `session_manager_lifecycle_tests/` 下全部 6 个 .rs 文件自动被排除，无需在文件级再查路径段。

---

## risk list

| 风险 | 发生概率 | 影响 | 缓释措施 |
|------|---------|------|---------|
| 生产代码故意藏入 test 文件规避计数 | 极低 | 中等 | Code review + Cargo module 体系；修正后最低指标距 ceiling 仍有 29 余量 |
| 将来出现 `test.rs` / `.test.rs` 命名惯例 | 中 | 低 | 不属本修正范围；若出现可开独立任务追加排除 |
| 误排加法向测试代码时混入 `tests/` 目录内的合法集成测试 | 不适用 | — | `tests/` 目录一直是最严格的排除，仓储无生产代码混入此目录 |
| ceiling 未来因误排增长而失效 | 中 | 中 | rot-budget.json note 字段记录修正日期和先例；三个指标均有明裕余量 |

---

## 附带条件

1. `rot-budget.json` 四条 grep-count 指标须追加 note，格式参照 `unix_epoch_inline` 的先例（日期 + 语义重定说明）：
   ```
   "note": "R-13, ratchet: only down; baseline rebased 2026-08-28 to exclude tests.rs and *_tests/ dirs"
   ```
2. `verify-rot-budget.test.mjs` 须追加两条自测：① `tests.rs` 文件不计入；② `*_tests/` 目录内文件不计入。
3. 修正 commit message 须含 `checker-semantics-rebase` 标记，与 `unix_epoch_inline` 先例一致。

---

## 先例引用

- `unix_epoch_inline` (2026-08-21): *"ceiling rebased 2026-08-21 to checker semantics (excludes tests dirs/_tests.rs)"* — 同一模式，已接受
- W6-1 (2026-08-28): `allow_dead_code` 从 128 降至 106 — 同波次已完成类似口径对齐
- `check-repo-hygiene.mjs:90`: 仓储自有测试文件识别惯例已覆盖这些文件类型
