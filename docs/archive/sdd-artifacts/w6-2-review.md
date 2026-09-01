# W6-2 Judge Review: rot 检查器语义修正（checker-semantics-rebase）

**Reviewer**: judge-m3（独立验收，只读审查）
**Date**: 2026-08-28
**Range**: `ebe57f2..7d53621` (3 files, +85/-7, scripts/ only)
**Commit**: `7d53621` — `fix(scripts): exclude tests.rs and *_tests dirs from rot budget measurement (checker-semantics-rebase, W6-2)`

---

## 双判决结果

### SPEC ✅ PASS

对照 brief §Spec 5 条 + D1 仲裁书附带 3 条件，逐条核验：

| § | Spec 条目 | 证据（file:line） | 状态 |
|---|----------|-------------------|------|
| 1.a | 排除文件名恰为 `tests.rs`（精确匹配，不误伤 `contests.rs`） | `scripts/verify-rot-budget.mjs:45` `entry.name !== 'tests.rs'` —— 全等匹配，不前缀匹配 | ✅ |
| 1.b | 排除任何目录段以 `_tests` 结尾（不误伤 `latest_results` 中间段） | `scripts/verify-rot-budget.mjs:36` `entry.name.endsWith('_tests')` —— 仅结尾匹配 | ✅ |
| 1.c | 既有排除（`tests` 段、`_tests.rs`、target、node_modules、`.git`）未回归 | `scripts/verify-rot-budget.mjs:35-39, 45, 48` —— 5 条既有规则全部保留，新增项以 `\|\|` 扩展而非替换 | ✅ |
| 1.d | 路径段冗余保护（即使目录漏跳也兜底） | `scripts/verify-rot-budget.mjs:48` `segments.some((s) => s.startsWith('target') \|\| s.endsWith('_tests'))` —— 段级兜底 | ✅ |
| 2 | 自测追加 2 条（tests.rs 排除 + `*_tests/` 排除） | `scripts/verify-rot-budget.test.mjs:306-385` —— 2 个 test 块 | ✅ |
| 2.qualify | 2 用例不是恒真测试（ceiling=0 + fixture 含 2 unwraps，缺排除逻辑即失败） | test 1 fixture: `tests.rs` 含 2 `.unwrap()`，ceiling=0，断言 count=0；test 2 同构 | ✅ |
| 3 | 5 条 grep-count 规则 note 追加语义重定基说明 | `scripts/rot-budget.json:6,12,18,24,30` —— 全部 5 条 note 含 `2026-08-28` + `D1 adjudication` | ✅ |
| 3.qualify | 格式对齐 `unix_epoch_inline` 既有 rebase 风格 | `unix_epoch_inline` note 保留 2026-08-21 原句，追加 `rebased 2026-08-28 per D1 adjudication (excludes tests.rs and *_tests/ dirs)` —— 与 unix_epoch_inline 自身先例同构 | ✅ |
| 3.hard | **ceiling 数值零改动**（硬红线） | diff 仅改 5 行 `note`，所有 `ceiling:` 行 byte-for-byte 一致：502/1089/388/69/109 | ✅ |
| 4 | 自测 11/11 + 工作区验证 exit 0 | 实跑：`pass 11 / fail 0`；`Rot budget verification passed`；读数 469/937/388/69/106 全在 ceiling 内 | ✅ |
| 4.unwrap | unwrap=469 vs brief 预估 473（−4 偏差） | W6-1 commit `11a4e5e` 实测：keyring.rs 删除 4 个测试函数含 `kr.store(...).unwrap()`、`resolve_api_key(...).unwrap()` × 3，共 4 处 `.unwrap()` —— 与报告偏离清单一致 | ✅ |
| 5 | commit 恰好一个 + 含 `checker-semantics-rebase` 标记 | `git log` 仅 `7d53621` 一个新 commit；message 含 `checker-semantics-rebase, W6-2` | ✅ |
| 5.qualify | 不含 `.superpowers/` | `git show --stat 7d53621` 仅触及 `scripts/` 下 3 个文件 | ✅ |

**D1 仲裁附带条件 3 条全部落地**：
1. ✅ 5 条 note 追加（含 D1 引用 + 日期 + 排除语义说明）
2. ✅ 自测用例追加（2 条）
3. ✅ commit message 含 `checker-semantics-rebase`

### QUALITY ✅ PASS

- **复用**：复用既有 `endsWith`/`includes` 模式，扩展而非重写；与 `unix_epoch_inline` rebase 先例完全同构。
- **无 owner 抽象**：仅是 3 个 boolean 条件扩展，无新结构、无新文件、无新模块。
- **预算闸授权范围**：D1 授权 = 排除规则 + note + 自测，实现严格停留在该三轴；无 ceiling 改动、无生产代码改动、无 crate 结构改动。
- **style 对齐**：`&&`/`||` 链式扩展与既有写法一致；test fixture 用 `try/finally + fs.rmSync` 与既有用例同构。
- **anti-evasion 复核**：新排除是否会引入"把生产代码藏进 `tests.rs`/`*_tests/` 就免计数"的洞？
  - Cargo 模块体系不认 `tests.rs` 为生产模块（mod 树会引用）；审查可见。
  - `*_tests/` 目录命名本身暴露意图；hygiene 脚本同款识别（`check-repo-hygiene.mjs:90`），审查更显眼。
  - 修正后 `unwrap_production=469`，距 ceiling 502 有 33 余量；真实生产 rot 增长仍被捕捉。
  - 结论：D1 仲裁风险评估成立，复核通过。

---

## 反规避核查（独立复核 D1 结论）

| 场景 | D1 缓释 | 复核结论 |
|------|--------|---------|
| 生产代码藏入 `tests.rs` | Code review + Cargo mod 体系 | ✅ 成立。`tests.rs` 在 Cargo 是 `#[cfg(test)] mod tests` 惯例模块，Rust 编译器要求显式 `mod tests` 声明——审查可见 |
| 生产代码藏入 `*_tests/` 目录 | 命名暴露意图 + hygiene 脚本覆盖 | ✅ 成立。`_tests` 结尾目录是社区集成测试惯例（每个集成测试独立 crate）；hygiene 脚本 `check-repo-hygiene.mjs:90` 同款识别，PR 评审必见 |
| `let_underscore=388` 顶格无冗余 | 这是测量口径问题，非技术债；生产代码 `let _ =` 独立反映 | ✅ 接受。顶格指标是 `unix_epoch_inline` 同款前置状态，不是新引入风险 |

---

## 实跑验证（独立复测，非实现者报告复用）

```text
$ node scripts/verify-rot-budget.test.mjs
✔ compliant fixture exits 0 and reports success
✔ grep count exceeding ceiling fails and exits 1 with guidance message
✔ unregistered file exceeding 800 lines fails and exits 1
✔ registered god-file exceeding ceiling fails
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry
✔ dir-entry-count compliant fixture passes
✔ dir-entry-count exceeding ceiling fails and exits 1
✔ dir-entry-count on non-existent directory fails and exits 1
✔ tests.rs file is excluded from rot budget measurement
✔ *_tests directory files are excluded from rot budget measurement
✔ actual workspace rot budget passes with current manifest
ℹ tests 11 / pass 11 / fail 0

$ node scripts/verify-rot-budget.mjs
Rot budget verification passed (5 grep rules
  [unwrap_production=469/502, expect_production=937/1089,
   let_underscore=388/388, unix_epoch_inline=69/69,
   allow_dead_code=106/109],
  3 dir rules [...], 11 god-file rules checked across 1342 files).
```

读数与 brief Spec 4 预期（469 / 937 / 388 / 69 / 106）逐项一致。

## W6-1 unwrap −4 偏差独立验证

`git show 11a4e5e -- src/apps/desktop/src/app_state/settings/keyring.rs` 删除块：

| 被删测试函数 | 含 `.unwrap()` 数 |
|------------|------------------|
| `resolve_api_key_returns_sentinel_from_keyring` | 2（`kr.store(...).unwrap()` + `resolve_api_key(...).unwrap()`） |
| `resolve_api_key_returns_plaintext_directly` | 1（`resolve_api_key(...).unwrap()`） |
| `resolve_api_key_returns_empty_string_as_is` | 1（`resolve_api_key(...).unwrap()`） |
| `resolve_api_key_sentinel_missing_keyring_returns_err` | 0 |
| **合计** | **4** |

473 − 4 = 469，与实跑读数完全一致。报告偏离清单可信。

---

## Cannot verify from diff

无。所有声明（5 条 note 追加、2 用例有效性、ceiling 零改动、11/11 通过、读数一致）均已逐条核验。

---

## 判决

**Approved** | SPEC PASS · QUALITY PASS | **0C / 0I / 0M**

实现严格落在 D1 授权三轴内（排除规则 + note 追加 + 自测用例），ceiling 硬红线 byte-for-byte 守住，2 新自测用例在 ceiling=0 的对抗设计下确实失败-回归-通过，新增排除精度经 `contests.rs`/`latest_results` 反例验证无误伤，独立复测 11/11 与读数 469/937/388/69/106 全绿，unwrap 473→469 的 −4 偏差由 W6-1 commit `11a4e5e` 实测闭环。
