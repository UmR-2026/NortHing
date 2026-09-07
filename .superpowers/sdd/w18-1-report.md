# W18-1 Report — validateManifest() fail-closed + D-3 红队 probe 首批

## 改动摘要

1. **`validateManifest(manifest, projectRoot)`**：
   - 在 `scripts/verify-rot-budget.mjs` 中新增导出函数，返回 `{ success, errors }`，逐 entry 收集全部错误（不遇错即停）。
   - 规则清单：
     - `kind` 必填，必须属于白名单集合 `{ 'grep-count', 'file-lines', 'dir-entry-count' }`（`entry.kind === undefined || entry.kind === null` 报 missing，其余非白名单报 invalid）；
     - `ceiling` 必须为有限非负整数（`Number.isInteger(ceiling) && ceiling >= 0`）；
     - `note` 若存在必须为字符串；
     - `grep-count` 的 `pattern` 必须为非空字符串且 `new RegExp(pattern)` 可编译；
     - `dir-entry-count` 解析目录（`entry.dir` 或 key 去除 `dir_entries:` 前缀）经 `\`→`/` 归一化与 `path.resolve(projectRoot, dir)` 后必须在 `projectRoot` 内（禁 `..` 越界、禁 projectRoot 外绝对路径），违规消息插值 `resolvedRoot`；
     - 未知字段拒绝：采用表驱动白名单 `FIELD_WHITELIST`，以 `Object.hasOwn(FIELD_WHITELIST, entry.kind)` 进行自有属性守卫（免疫原型链污染属性如 `__proto__`/`toString`）。当前启用公共 `kind`/`ceiling`/`note`；`grep-count` 增加 `pattern`；`dir-entry-count` 增加 `dir`。表结构预留终态扩展能力；
   - `verifyRotBudget()` 解析 manifest 后最先调用 `validateManifest()`；若校验失败，错误全部记入 `violations` 并立即 fail-closed 退出（不执行后续文件扫描）。

2. **5 个负向 manifest fixture**（新建于 `scripts/fixtures/rot-budget/`）：
   - `bogus-kind.json`：`kind: "bogus-kind"`
   - `string-ceiling.json`：`ceiling: "10"`
   - `path-escape.json`：`dir_entries:../escaped`
   - `empty-pattern.json`：`pattern: ""`
   - `unknown-field.json`：含未定义字段 `unknown_extra: true`

3. **`--selftest`（`runSelftest()`）**：
   - 风格对齐 `verify-task-gate.mjs`（`[PASS]` / `[FAIL]`、11 个 checks 全量记录、任一失败整体非零）；
   - 5 个负向 fixture JSON 经 `validateManifest` 均 `success: false` 且 error 明确指向对应原因；
   - 3 个内联负例：
     - `pattern: "["` 正则不可编译；
     - `note: 123` 非字符串类型；
     - `kind: "__proto__"` 原型链继承属性名，验证不抛 TypeError，平稳返回 `success: false`；
   - 正向：当前仓库 `scripts/rot-budget.json` 原样通过；
   - 阈值边界（档 A）：在临时目录中生成恰好 800 行与 801 行的两个 `.rs` 文件，验证 800 行不触发违规、801 行触发 god-file 违规；
   - 集成接线：以 `bogus-kind.json` 调 `verifyRotBudget`，验证入口真正 fail-closed 且 `violations` 含 schema 错误。

---

## 验证

### 1. 旧代码行为确认记录（GC4，BASE `eaa592f`）

在 clean 临时环境运行 BASE 版 `verifyRotBudget`，确认 5 个负例的原始行为：

| Fixture | BASE 输出摘要 | BASE 判决 | 旧行为性质判定 |
|---|---|---|---|
| `bogus-kind.json` | `Rot budget verification passed (0 god-file rules checked across 1 files).` | `success: true, violations: []` | **真 fail-open 绿**（未知 kind 被跳过，未建任何规则） |
| `string-ceiling.json` | `Rot budget verification passed (1 grep rules [test_entry=1/10], 0 god-file rules checked across 1 files).` | `success: true, violations: []` | **真 fail-open 绿**（字符串 `"10"` 进数字比较被静默放行） |
| `path-escape.json` | `dir_entries:../escaped: directory does not exist at ../escaped — non-existent directory violates dir-entry-count guard` | `success: false, violations: 1` | **错误原因的红**（因磁盘目录不存在报错，未做 manifest 路径约束校验） |
| `empty-pattern.json` | `test_entry: current 18 exceeds ceiling 10 — split, reduce, or register a justified manifest entry` | `success: false, violations: 1` | **错误原因的红**（因空正则匹配每一个字符导致 count=18 超 ceiling，未拦截空 pattern） |
| `unknown-field.json` | `Rot budget verification passed (1 grep rules [test_entry=1/10], 0 god-file rules checked across 1 files).` | `success: true, violations: []` | **真 fail-open 绿**（未知字段 `unknown_extra` 被完全忽略） |

### 2. 验证命令 1：`node scripts/verify-rot-budget.mjs`

```
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=65/400], 6 god-file rules checked across 1368 files).
EXIT_CODE: 0
```

#### 基线读数对照表

| 指标项 | BASE 基线 | 当前读数 | 差异说明 |
|---|---|---|---|
| `unwrap_production` | 483/502 | 483/502 | 0 漂移 |
| `expect_production` | 940/1089 | 940/1089 | 0 漂移 |
| `let_underscore` | 370/388 | 370/388 | 0 漂移 |
| `unix_epoch_inline` | 69/69 | 69/69 | 0 漂移 |
| `allow_dead_code` | 104/109 | 104/109 | 0 漂移 |
| `dir_entries:scripts` | 44/48 | 44/48 | 0 漂移（`fixtures/` 为子目录，不计入顶层文件） |
| `dir_entries:docs/design` | 1/1 | 1/1 | 0 漂移 |
| `dir_entries:.superpowers/sdd` | 63/400 | 65/400 | +2 属工作树未跟踪 `w18-1-brief.md` 与 `w18-1-report.md` 环境漂移（committed 状态为 63） |
| `god-file rules` | 6 | 6 | 0 漂移 |
| `checked files` | 1368 | 1368 | 0 漂移 |

### 3. 验证命令 2：`node scripts/verify-rot-budget.mjs --selftest`

```
[PASS] negative fixture: bogus-kind: rejects invalid kind in manifest
[PASS] negative fixture: string-ceiling: rejects string ceiling in manifest
[PASS] negative fixture: path-escape: rejects directory path escaping project root
[PASS] negative fixture: empty-pattern: rejects empty pattern for grep-count
[PASS] negative fixture: unknown-field: rejects unknown field in manifest
[PASS] negative inline: invalid-regex: rejects uncompilable regex pattern
[PASS] negative inline: non-string-note: rejects non-string note
[PASS] negative inline: proto-kind: rejects prototype-inherited kind without throwing
[PASS] positive: workspace manifest: current scripts/rot-budget.json passes validation
[PASS] threshold boundary: 800 vs 801 lines: 800 lines passes, 801 lines triggers god-file violation
[PASS] integration: verifyRotBudget wires validateManifest: verifyRotBudget fails-closed on invalid manifest
Selftest passed: 11 checks passed (8 negative, 3 positive).
EXIT_CODE: 0
```

### 4. 验证命令 3：`node scripts/verify-task-gate.mjs validate-policy`

```
Policy validation passed: <PROJECT_ROOT>\scripts\workflow-policy.json
EXIT_CODE: 0
```

### 5. 验证命令 4：`pnpm run check:repo-hygiene`

```
> northhing@0.2.10 check:repo-hygiene <PROJECT_ROOT>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (8 content files scanned, 3850 filenames checked).
EXIT_CODE: 0
```

### 6. 验证命令 5：`node scripts/verify-rot-budget.test.mjs`

```
✔ compliant fixture exits 0 and reports success (111.5948ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (105.5005ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (111.1735ms)
✔ registered god-file exceeding ceiling fails (7.4006ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.9801ms)
✔ dir-entry-count compliant fixture passes (107.886ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (108.8396ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (102.8787ms)
✔ tests.rs file is excluded from rot budget measurement (7.088ms)
✔ *_tests directory files are excluded from rot budget measurement (7.749ms)
✔ actual workspace rot budget passes with current manifest (345.4274ms)
✔ dead god-file registration warns but does not fail verification (110.25ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1142.7353
EXIT_CODE: 0
```

### 7. 修复轮 1（Judge 审查意见处置）

- **Important**（`scripts/verify-rot-budget.mjs:93-94`：`kind` 为原型链继承属性名引发未捕获 TypeError）：
  - 根因：`FIELD_WHITELIST[entry.kind]` 在 `entry.kind` 为 `__proto__` / `toString` / `constructor` 时会解析出原型链上的非数组对象/函数，导致 `new Set(...)` 抛出不可迭代 TypeError。
  - 处置：将规则 5 守卫改为自有属性判定 `Object.hasOwn(FIELD_WHITELIST, entry.kind)`。并在 `--selftest` 补充内联负例 `negative inline: proto-kind`（`kind: "__proto__"`），断言其平稳返回 `success: false` 且错误信息包含 invalid kind 语义，钉住「不抛异常、平稳拒收」。
- **Minor 1**（L46 附近 falsy 非 undefined 的 kind 文案误判为 missing）：
  - 处置：将判空条件精确化为 `entry.kind === undefined || entry.kind === null` 判 missing，其余非白名单值（如 `""`、`0`、`false`）一律判定为 invalid kind。
- **Minor 2**（L86 附近 escape 错误消息插值未 resolve 的 `projectRoot` 实参）：
  - 处置：错误消息插值改为归一化解析后的 `resolvedRoot`。

---

## 状态

DONE
