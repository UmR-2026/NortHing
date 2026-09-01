# Task ROT-3' Report — rot-budget 预算闸

## 一、六条验收标准逐条证据

1. **`scripts/rot-budget.json` 存在且基线值严格匹配**：
   - 文件已创建，包含 3 条 `grep-count` 规则与 7 条 `file-lines` 规则。
   - `unwrap_production`: ceiling 521, pattern `\\.unwrap\\(\\)`
   - `expect_production`: ceiling 1098, pattern `\\.expect\\(`
   - `let_underscore`: ceiling 402, pattern `let _ =`
   - 7 个 god-file 路径与 ceiling 行数完全匹配：
     - `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`: 1063
     - `src/apps/cli/src/ui/theme.rs`: 990
     - `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`: 918
     - `src/crates/assembly/core/src/service/agent_memory/facts.rs`: 905
     - `src/apps/cli/src/ui/startup/selectors.rs`: 877
     - `src/crates/assembly/core/src/service/lsp/manager.rs`: 836
     - `src/apps/cli/src/modes/chat/input.rs`: 802

2. **`node scripts/verify-rot-budget.mjs` exit 0（全绿）**：
   - 运行输出：`Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1360 files).`
   - Exit code: 0

3. **自测证明闸能变红（`scripts/verify-rot-budget.test.mjs`）**：
   - 覆盖 6 个独立测试用例：
     1. 合规 fixture exit 0
     2. grep 计数超 ceiling fixture exit 1，包含当前值/上限/修复指引
     3. 未登记 >800 行文件 fixture exit 1，包含当前值/上限/修复指引
     4. 已登记 god-file 超 ceiling fixture exit 1
     5. 豁免文件 `src/shared/i18n/generated_locale_contract.rs` >800 行正常放行
     6. 当前真实工作区 rot-budget 校验全绿
   - 运行输出 6/6 全部 pass。

4. **`pnpm run check:rot` 可用**：
   - `package.json` 添加 `"check:rot": "node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs"`
   - 运行 `pnpm run check:rot` 成功执行并通过。

5. **CI `rot-budget` job**：
   - `.github/workflows/ci.yml` 在 `core-boundaries` 之后追加 `rot-budget` job，配置 `ubuntu-latest` + `actions/setup-node@v4 (node-version: 22)` + 运行 `node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`。

6. **家规第 7 条同步**：
   - `AGENTS.md` Housekeeping rules 追加第 7 条（英文）。
   - `AGENTS-CN.md` 补齐内务规则小节并包含等价中文第 7 条。

---

## 二、复用侦察

- **参考先例**：
  - `scripts/check-core-boundaries.mjs` & `scripts/check-core-boundaries.test.mjs`
  - `scripts/i18n-audit.mjs`
- **对齐点**：
  - **Manifest 与路径**：对齐 `i18n-audit.mjs` 模式，采用 JSON flat manifest，路径一律使用以工作区根为相对基准的 POSIX `/` 路径。
  - **测试结构**：对齐 `check-core-boundaries.test.mjs`，采用 Node.js 18+ 原生 `node:test` + `node:assert/strict`，配合 `fs.mkdtempSync` 构造独立隔离 fixture，并通过 `spawnSync` 校验 CLI 进程状态码及 stderr/stdout 文本。
  - **错误提示与退出约定**：收集全部违规后一次性打印并 `process.exit(1)`；全绿输出单行 summary 并 `process.exit(0)`；纯英文、无 emoji。
- **刻意不同与理由**：
  - **单文件极简设计（140 行 < 250 行限制）**：`check-core-boundaries` 因涉及复杂 AST facade 与 Cargo 特性组合被拆解为子目录结构；而 rot budget 规则高度聚焦（计数、行数、800行警戒），单文件实现更紧凑、内聚，无需跨模块引用。
  - **纯 Node 原生实现（零外部进程依赖）**：不通过 shell 调 `rg` 或 `powershell`，杜绝跨操作系统平台行为差异（换行符 `\r\n` vs `\n` 处理对齐 `(Get-Content).Count`）。

---

## 三、Spec 3 与 Spec 5 选择与理由

- **Spec 3 自测集成**：
  - 编写 `scripts/verify-rot-budget.test.mjs`，并通过 `pnpm run check:rot` 串联执行 `node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`。
- **Spec 5 CI 配置**：
  - 在 `.github/workflows/ci.yml` 的 `rot-budget` job 中直接执行 `node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`，避免引入不必要的 pnpm/corepack 安装步骤，同时保证自测与主检查均在 CI 中执行。

---

## 四、验证命令与完整输出

### 1. `node scripts/verify-rot-budget.mjs`
```
Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1360 files).
```
Exit code: 0

### 2. `node scripts/verify-rot-budget.test.mjs`
```
✔ compliant fixture exits 0 and reports success (104.3543ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (106.1703ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (100.4301ms)
✔ registered god-file exceeding ceiling fails (7.4386ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.6118ms)
✔ actual workspace rot budget passes with current manifest (390.9462ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 723.5862
```
Exit code: 0

### 3. `pnpm run check:rot`
```
> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-rot-budget
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (112.8029ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (101.3926ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (106.0516ms)
✔ registered god-file exceeding ceiling fails (6.4901ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.9218ms)
✔ actual workspace rot budget passes with current manifest (363.485ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 704.1842
Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1360 files).
```
Exit code: 0

### 4. `node scripts/check-core-boundaries.mjs`
```
Core boundary check passed.
```
Exit code: 0

### 5. `git diff --stat`
```
 .github/workflows/ci.yml | 15 +++++++++++++++
 AGENTS-CN.md             | 19 +++++++++++++++++++
 AGENTS.md                |  1 +
 package.json             |  1 +
 4 files changed, 36 insertions(+)
```

---

## 五、偏离声明

无任何偏离。所有 baseline 数值逐字对齐 brief 预检表，未修改任何产品代码，各检查全部通过。
