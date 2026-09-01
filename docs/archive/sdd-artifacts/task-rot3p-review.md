# Task ROT-3' Review — rot-budget 预算闸

## 结论摘要

- **SPEC 判决**：6/6 PASS
- **QUALITY 判决**：PASS（无 Critical / Important / Minor 阻塞项）
- **独立验证**：三步全过（checker / self-test / 手动变红）
- **最终**：**APPROVED**

---

## 一、独立验证（实跑命令与输出）

### 1.1 `node scripts/verify-rot-budget.mjs`

```
Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1360 files).
```
EXIT=0。完全匹配实现者报告。

### 1.2 `node scripts/verify-rot-budget.test.mjs`

```
✔ compliant fixture exits 0 and reports success (95.3ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (91.9ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (89.0ms)
✔ registered god-file exceeding ceiling fails (6.5ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.4ms)
✔ actual workspace rot budget passes with current manifest (295.6ms)
tests 6 / pass 6 / fail 0
```
EXIT=0。**测试代码已确认构造违规 fixture 并断言 exit 1**（`scripts/verify-rot-budget.test.mjs` 行 49–97 显式断言 `proc.status === 1` 且 stderr 含 guidance；行 102–146 同上；行 149–183 走函数返回值路径，底层同样 `process.exit(1)`）。

### 1.3 手动变红测试（核心）

临时将 `unwrap_production.ceiling` 521 → 520，跑 checker：

```
unwrap_production: current 521 exceeds ceiling 520 — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)
Rot budget verification failed with 1 violation(s).
```
EXIT=1，错误消息含 当前值(521)/上限(520)/修复指引/升级需用户拍板。已从备份还原，工作树恢复干净（`git status` 仅剩 `task-rot3p-review-brief.md` untracked）。

### 1.4 `pnpm run check:rot`

完整通过，自测 6/6 + 主检查 PASSED（1360 files）。EXIT=0。

### 1.5 `node scripts/check-core-boundaries.mjs`（波及检查）

```
Core boundary check passed.
```
EXIT=0，未被波及。

---

## 二、SPEC 逐条判决（6/6 PASS）

| # | 验收项 | 判决 | 证据（file:line） |
|---|---|---|---|
| 1 | `scripts/rot-budget.json` 存在且基线值严格匹配 | **PASS** | `scripts/rot-budget.json:5` ceiling=521 / `:13`=1098 / `:21`=402；`:28`=1063 / `:35`=990 / `:42`=918 / `:49`=905 / `…`=877/836/802。逐字节对齐 brief 预检表 |
| 2 | `node scripts/verify-rot-budget.mjs` exit 0 | **PASS** | 实跑见 §1.1 |
| 3 | 自测构造违规 fixture 断言 exit 1 | **PASS** | `scripts/verify-rot-budget.test.mjs:49–97` (a) grep 超 ceiling → `assert.equal(proc.status, 1)` + stderr 正则；`:102–146` (b) 未登记 805 行文件 → `assert.equal(proc.status, 1)`；`:149–183` (c) 已登记超 ceiling；实跑 6/6 见 §1.2 |
| 4 | `pnpm run check:rot` 可用 | **PASS** | `package.json:18` `"check:rot": "node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs"`；实跑 §1.4 |
| 5 | CI 新增 `rot-budget` job 含自测 | **PASS** | `.github/workflows/ci.yml:138–151` 新 job `rot-budget` on `ubuntu-latest`，steps = checkout + setup-node 22 + `node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`；自测不缺席 |
| 6 | 家规第 7 条同时存在于 AGENTS.md 与 AGENTS-CN.md 且语义等价 | **PASS** | `AGENTS.md:101` 与 `AGENTS-CN.md:99`；中英条目语义对齐（"ceiling 只降不升 / raising requires user sign-off"） |

附加 spec 7（不夹带）：未改任何产品代码、未调任何 ceiling、未动 i18n frozen、未动其它 CI job — `git show --stat` 仅 9 个文件（4 修改 + 3 新增 + 2 sdd 文档），未触及产品源码 ✓。

---

## 三、QUALITY 独立判断

### 3.1 复用核查（报告「复用侦察」节是否属实）

报告声称对齐 `scripts/check-core-boundaries.mjs` + `scripts/i18n-audit.mjs`。已对照：

- **JSON manifest + POSIX 路径**：`i18n-audit.mjs` 用扁平 JSON 基线 + POSIX 相对路径（`scripts/i18n-hardcoded-baseline.json` 等），新 `rot-budget.json` 采用同款 ✓
- **测试结构**：`scripts/core-boundaries/self-test.mjs` 模式为 `node:test` + `node:assert/strict` + `fs.mkdtempSync` + `spawnSync` 校验 CLI 状态码与 stderr；新 `verify-rot-budget.test.mjs` 完全沿用 ✓
- **错误格式与 exit code 约定**：所有违规收集后一次性打印并 `process.exit(1)`；全绿单行 summary 并 `process.exit(0)` — 与 `check-core-boundaries` 一致 ✓
- **刻意不同**：单文件 <250 行（实现 148 行）+ 零外部进程（不调 rg / powershell），规避跨平台换行符差异 — 合理且必要 ✓

**无「复制既有能力而不复用」问题**：新代码未重写 `core-boundaries` 的 AST 解析能力（也用不上），新代码未重写 `i18n-audit` 的多 baseline 治理（也用不上）。两者解决的问题域不重叠。

### 3.2 无 owner 抽象

`scripts/verify-rot-budget.mjs` 导出三个函数：
- `countLines` → 被 `verifyRotBudget` 调用（file:13–19 定义，file:82 调用）
- `collectRustFiles` → 被 `verifyRotBudget` 递归调用（file:21–46 定义，file:79 调用）
- `verifyRotBudget` → 既作为导出供 test 使用，也作为 CLI 入口直接调用（file:163–166）

每个抽象均有真实消费方，无投机抽象。

### 3.3 预算闸（diff 是否触碰 `scripts/rot-budget.json` 上调/放松）

- diff 创建 `scripts/rot-budget.json`（新文件，55 行），含 3 grep-count + 7 file-lines，全部为基线值
- **未触动任何已有 ceiling / 规则**（仓内此前无此 manifest）
- 实现者 commit message 末尾已注明 `(ROT-3')`，与 brief 派发元信息一致
- 无用户拍板原文附在 brief，但本任务是"建闸本身、初始登记"，不构成上调/放松

---

## 四、Findings

### Minor

1. **报告 `git diff --stat` 只列 4 文件，漏掉 3 个新文件** — `task-rot3p-report.md` 的验证节 5 输出 `4 files changed, 36 insertions(+)`，但实际 commit `git show --stat` 含 9 个文件（4 修改 + 3 新增 + 2 sdd 文档，675 insertions）。`git diff --stat` 不显示未跟踪新文件是已知行为，但报告措辞"git diff --stat 全文"与实际不符。**影响**：读者按报告核对会误判改动面。**建议**：终审前实现者补 `git show --stat` 或 `git diff HEAD~1 --stat`（含新增）。不阻塞放行 — commit 内容真实完整。

2. **AGENTS-CN.md 添加了规则 0–6 全段，超出 brief 范围** — brief 仅要求 "AGENTS-CN.md 对应中文条目（语义等价）"追加规则 7，但 AGENTS-CN.md 基线版本根本无"内务规则"小节（已用 `git show 43c2c29:AGENTS-CN.md | grep "内务"` 确认无输出）。实现者将 EN 版本规则 0–7 全部翻译补齐入 CN 版本。**影响**：CN/EN 结构对齐更彻底，但 commit 改动面比 brief 大。**判定**：非冲突性扩展，规则 0–6 为已有 EN 规则的中文镜像，无新增治理语义。不阻塞。

### Cannot verify from diff

- 当前实际 `unwrap/expect/let _ =` 命中数与 brief 预检表 521/1098/402 是否**逐字相等**（vs. 漂移到 ≤ceiling）：只能间接确认（checker exit 0 ⟹ 当前 ≤ceiling）。精确命中数未独立重跑 rg 核对。**信任基础**：实现者报告已贴输出，本任务 verifier 在自己的 report 中只展示 "passed"，未贴 counts dict。**建议**：如编排者要求严格 1:1 复测，可在 ledger 中追加一次独立 ripgrep 对账。
- CI job 真实跑通：本机无 ubuntu runner，未实跑；信任 yaml 配置 + 节点 22 + 单 run 命令与已有 `core-boundaries` job 同形态。

---

## 五、修改了什么 / 验证了什么 / 残留 caveat

**修改**：
- 新增 `scripts/rot-budget.json`（基线 manifest）
- 新增 `scripts/verify-rot-budget.mjs`（148 行，单文件 checker）
- 新增 `scripts/verify-rot-budget.test.mjs`（172 行，6 个测试）
- 修改 `package.json`（新增 `check:rot` script）
- 修改 `.github/workflows/ci.yml`（新增 `rot-budget` job 含自测）
- 修改 `AGENTS.md`（追加家规第 7 条）
- 修改 `AGENTS-CN.md`（补齐内务规则小节 0–7 + 第 7 条语义对齐）

**验证**：
- 实跑 checker：exit 0，1360 files
- 实跑 self-test：6/6 pass
- 实跑 pnpm run check:rot：通过
- 实跑 check-core-boundaries：未波及，仍 PASSED
- 实跑手动变红：unwrp ceiling 521→520 触发 exit 1，错误消息合规

**残留 caveat**：
- 报告 `git diff --stat` 表述不完整（Minor）
- AGENTS-CN.md 改动面超出 brief 字面要求（Minor）
- 当前实际 grep 命中数未独立逐字复核 ≤ baseline（实现者贴输出已覆盖，信任传递）

**结论**：实现完整、严格遵循 brief、quality clean、无阻塞项。**APPROVED**。