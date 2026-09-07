# W18-5a Brief — 扫描范围 attestation + 收集器扩展 + 登记/lease

## 任务标识

W18-5a（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2.1；原 W18-5 经 brief review 拆分为 5a/5b，本单 = attestation + 收集器扩展 + 登记 + lease；退役规则在 5b）。前置：W18-3 lease 机制已落地（D07 顺序满足）。

## BASE

`356b33c`（main HEAD；代码树无 WIP，SDD 文档另行提交）

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/rot-budget.json`（仅：新增 3 个 god_file 登记项，见功能要求 3；零触碰既有 entry）
- `scripts/workflow-policy.json`（仅：新增 `rotScanScope` 字段）
- `scripts/exception-leases.json`（仅：新增 2 条 lease）

report 写 `.superpowers/sdd/w18-5a-report.md`，不进本单验收 diff。fixture 全部 selftest 内联 tmpdir 合成，不新增文件。

## 编排者实测预检（2026-09-07，BASE 上已跑，reviewer-53 独立重算全部命中）

- installer `.rs` >800 行：**零**（无需登记）；installer .rs 总数 11。
- scripts 顶层 `.mjs/.js` 21 个，非测试 18 个；>800 行 4 个：`i18n-audit.mjs` 3833、`i18n-contract.test.mjs` 1042（测试命名豁免，用户拍板）、`verify-rot-budget.mjs` 1692、`verify-task-gate.mjs` 806。
- 扩扫描后预期 `checkedFilesCount` = 1368 + 11 + 18 = **1397**；测试命名排除集恰好 = `check-core-boundaries.test.mjs`、`i18n-contract.test.mjs`、`verify-rot-budget.test.mjs`。

## 用户拍板（2026-09-07 二次拍板，家规 7 登记授权）

- `god_file:scripts/verify-task-gate.mjs`：ceiling **847**（floor 公式精确值 806 + max(5, ceil(40.3))）。
- `god_file:scripts/verify-rot-budget.mjs`：ceiling **2300**（装下本波增长——前四单净增 115~537 行/单实证；lease next_action = 波后拆分 selftest 外置，届时经 only-down 下调）+ lease。
- `god_file:scripts/i18n-audit.mjs`：ceiling **4025** + lease（next_action = i18n 解冻时拆分）。
- 两条 lease：`owner: "orchestrator"`，`revisit_after: "2026-10-15"`（与 scripts 额度同期复审）。

## 功能要求

1. **rotScanScope 声明进 SSOT**：`scripts/workflow-policy.json` 新增 `"rotScanScope": {"grepRoots": ["src"], "fileLinesRoots": ["src", "northing-installer/src-tauri", "scripts"]}`。checker 启动时读取并断言实际扫描根 == 声明，根集合不一致 ⇒ violation（O-1 扫描范围变化自动报警）。**语义钉死**：policy 文件缺失 ⇒ attestation 跳过（合成根专用语义——既有 38 项 selftest 与 12 项 test.mjs 的 tmpdir 均不创建 policy 文件；真实仓库由 git 跟踪 + `validate-policy` 兜底，report 写明该残余面）；文件存在但缺 `rotScanScope` 字段 ⇒ violation；`rotScanScope` 存在但形状非法（非对象 / 缺 `grepRoots` 或 `fileLinesRoots` / 非字符串数组）⇒ violation。兜底说明：validate-policy 不校验 rotScanScope，真实兜底 = checker 自身缺字段/形状规则 + git 跟踪。成功输出（非 silent）增加逐根文件计数清单。
2. **收集器扩展**：file-lines 规则扫描面扩为三根——`src`（.rs，现有语义）、`northing-installer/src-tauri`（.rs，同现有排除规则）、`scripts`（仅顶层 `.mjs`/`.js`，排除 `/\.(test|spec)\.(mjs|js)$/`——较计划的 `*test*` glob 有意收窄，`test-acp.js` 计入扫描）。**grep-count 规则仍只扫 `src/`（读数零漂移硬约束）**。>1000 硬边界、allow-god-file 注释禁令、exempt-list 豁免、**>800 未登记检查**对新扫描面全部生效——>1000 等检查目前写死在 .rs 循环内，须先提取为对通用文件面执行的逻辑再扩面。**注释禁令判定锚定行首注释形态** `/^[ \t]*\/\/[ \t]*allow-god-file/`（已验证：不命中 checker 自身 8 处字面量，仍命中历史标记形态 `// allow-god-file: ...`）。
3. **manifest 登记**（拍板值逐字）：`god_file:scripts/verify-task-gate.mjs` ceiling 847、`god_file:scripts/verify-rot-budget.mjs` ceiling 2300、`god_file:scripts/i18n-audit.mjs` ceiling 4025；note 注明「W18-5a 扫描范围扩面登记，用户拍板 2026-09-07」。
4. **lease 登记**（拍板值逐字）：`exception-leases.json` 加两条——`scripts/verify-rot-budget.mjs`（reason：W18 波内 selftest 内联增长；next_action：波后拆分 selftest 外置）与 `scripts/i18n-audit.mjs`（reason：i18n 工程冻结期存量；next_action：解冻时拆分）。
5. **selftest 扩展**（内联沿用既有结构，tmpdir 合成）：
   - 负例：policy rotScanScope 与实际扫描根不一致（红）/ policy 存在但缺 rotScanScope 字段（红）/ rotScanScope 形状非法（红，如缺 fileLinesRoots）/ 合成 scripts 顶层 1001 行 .mjs 无登记无 lease（红，>1000 对新面生效）/ 合成 scripts 顶层 900 行 .mjs 未登记（红，>800 未登记对新面生效）/ 合成 scripts 文件含行首 `// allow-god-file` 注释（红，禁令对新面生效）；
   - 正例：policy 文件缺失 ⇒ attestation 跳过（绿）/ 测试命名 .mjs 不计入扫描（绿）/ installer 侧 801 行 .rs 登记后通过（绿）/ checker 自身文件含非行首形态的 `allow-god-file` 字面量不误报（绿，去触发机制钉死）；
   - 既有 38 项保持全绿。
6. 纯 Node 标准库，零新依赖；输出 English-only。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 除功能要求 3 的三个登记项（用户拍板值）外，任何 ceiling 不得上调；manifest 删指标视同上调（禁止）。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-5a)` 后缀 + commit message 引用户拍板原文（家规 7 登记授权留痕）。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位；pnpm banner 路径用 `<REPO_ROOT>` 占位；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（checker + policy），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。
- checker 改动**先负向 fixture 后实现**（GC4）：report「旧代码行为确认」节如实记录旧代码的扫描盲区（installer/scripts 不在闸内——fail-open by absence）。

## 禁区

- 禁改 grep-count 规则的扫描根（`src` only 不变）；禁改既有 manifest entry；禁改 `scripts/` 下其他文件、`scripts/fixtures/`、`docs/`。
- 禁缩范围制造绿色：测试排除集就是上述 3 个现存测试文件，不得扩大；禁以「豁免 checker 自身文件」方式解决注释禁令自触发（只能用功能要求 2 的行首锚定机制）。
- 禁为过关调整 fixture 判定标准。

## 验证

编排者已在 BASE `356b33c` 预跑基线（已实证绿）。交付验收：

1. `node scripts/verify-rot-budget.mjs` —— 绿；**预期读数变化（属扩面预期，非漂移）**：god-file rules 6→9、checkedFilesCount 1368→1397；grep 读数逐项不变（483/940/370/69/104）；dir 读数不变（45/48、1/1、sdd 以实跑为准）；零余量 warning 仍为 5 项（新登记项均有 headroom）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（38 + 本单新增）。
3. `node scripts/verify-rot-budget.mjs --base 356b33c` —— 绿（新 key 不触发 only-down/删除面。选点说明：用更早 BASE 会追溯触发本波后续任务的新规则，规则生效起点 = 落地 commit，已核实等价性）。
4. `node scripts/verify-rot-budget.test.mjs` —— 绿（12 回归）。
5. `node scripts/verify-task-gate.mjs validate-policy` —— 绿（本单改 policy.json，结构校验回归）。
6. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-5a-report.md`，必含三节：**改动摘要**（attestation 机制 / 收集器扩展面 / 登记与 lease 清单 / 检查提取方式）/ **验证**（6 条命令原文输出 + exit code + 旧代码行为确认 + 扩面前后读数对照表 + policy 缺失跳过的残余面说明）/ **状态**（状态词结尾）。
