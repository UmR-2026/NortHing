# W18-7 Brief — fixture registry sha256 锁定 + D-1 replay + E02 新旧对照

## 任务标识

W18-7（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2.1，相位 5.3 + D-1 + E02，波末单）。前置：W18-1（5 个 manifest 负向 fixture）、W18-5a（attestation 通道）、W18-6（rubric SSOT + 用例承载于 test.mjs 先例）已落地。

## BASE

`0b63224`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 0b63224 代码树等价，仅 docs 差异）。

## 允许文件集

- `scripts/fixtures/rot-budget/registry.json`（新增，fixtures 子目录不占 scripts 顶层额度）
- `scripts/workflow-policy.json`（修改，仅 metaRatchetPaths 追加一行）
- `scripts/verify-rot-budget.mjs`（修改，体量纪律见下）
- `scripts/verify-rot-budget.test.mjs`（修改，仅尾部追加本单新用例块）
- `scripts/fixtures/rot-budget/incident-w15-1g-symlink.json`（条件槽位：仅当 D-1 判定 W15-1g 可表达为 rot checker fixture 时新增；判定 not-applicable 则不用此槽位，gate 对未兑现条目只出 warning 不红）

fixture 除上述槽位外不新增文件；D-1/E02 的分析与对照运行全部在 tmpdir 进行，产物只进 report。report 写 `.superpowers/sdd/w18-7-report.md`，不进本单验收 diff。

**主文件体量纪律**：`verify-rot-budget.mjs` 现 2278 行 / ceiling 2300，净增 **≤ 20 行**。校验规则**降范围至 plan 最低语义**（plan line 153 只强制「哈希一致，漂移 ⇒ fail」）：malformed registry ⇒ fail + 逐条目「文件存在 + sha256 匹配」+ 无 registry ⇒ 跳过——估 15-17 行可容。条目缺字段不单列 schema 校验（缺 sha256 ⇒ 比对自然失败，fail-closed 兜底）。**降范围剔除项**（预算允许时亦不得做，留待波后 selftest 外置拆分单一并评估）：未登记 fixture 检测、path·sha256·purpose 三字段 schema——缓释依据 = registry.json 本单进 metaRatchetPaths（改动即升最高车道）+ fixture 新增必经评审任务。若连最低语义也无法在预算内交付 ⇒ BLOCKED 上交编排者转用户决策（ceiling 调整属家规 7 用户拍板，不在本单授权），不得压线硬塞。

## 功能要求

### 1. registry.json + 哈希锁定（相位 5.3）

- 新增 `scripts/fixtures/rot-budget/registry.json`，形态钉死：
  ```json
  {
    "fixtures": [
      { "path": "scripts/fixtures/rot-budget/bogus-kind.json", "sha256": "<hex>", "purpose": "<一句话用途>" }
    ],
    "notApplicable": [
      { "incident": "<事故编号>", "reason": "<不可表达理由>" }
    ]
  }
  ```
  `fixtures` 覆盖目录内既有 5 个 fixture（bogus-kind / empty-pattern / path-escape / string-ceiling / unknown-field），sha256 为文件实测哈希；`notApplicable` 承载 D-1 判定（见功能 3）。
- `verify-rot-budget.mjs` 新增 registry attestation（W18-5a 模式同构，目录参数化以便合成测试）：
  - `--selftest` 起步先对**真实仓库** fixtures 跑校验（scriptDir 锚定），漂移 ⇒ 非零退出；
  - 校验规则（fail-closed，plan 最低语义）：registry 存在但 JSON malformed ⇒ fail；登记条目指向缺失文件 ⇒ fail；sha256 不符 ⇒ fail（条目缺 sha256 字段 ⇒ 比对自然失败，同向兜底）；
  - registry 文件不存在（合成环境）⇒ 跳过（与既有 attestation 哲学一致）。
- 用例（承载于 `verify-rot-budget.test.mjs` 尾部新块，tmpdir 合成）：哈希匹配 ⇒ 绿 / 篡改字节 ⇒ 红 / 登记指向缺失文件 ⇒ 红 / malformed registry ⇒ 红 / 条目缺 sha256 ⇒ 红 / 无 registry ⇒ 跳过绿。
- report 贴**篡改演示**：对真实 fixture 改一字节 → `--selftest` 红（原文输出 + exit code）→ `git checkout --` 还原 → 复跑绿。

### 2. metaRatchetPaths 增补

- `scripts/workflow-policy.json` 的 `metaRatchetPaths` 追加 `"scripts/fixtures/rot-budget/registry.json"`（改 fixture 集 = 改判据，升最高车道；本单验收走双 judge 车道，代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

### 3. D-1 replay（4 起历史事故逐条判定）

对每起事故判定「rot checker 可表达 ⇒ 转 fixture」或「不可表达 ⇒ registry.json 标注 not-applicable + 理由」，判定表进 report：

| 事故 | 层位预分析（编排者供参考，实现者复核裁决） |
|---|---|
| W15-1g 符号链接假绿 | 可能可表达（文件计数/行数口径对 symlink 的处理）；若表达，用条件槽位 `incident-w15-1g-symlink.json` 或 tmpdir 合成用例。**Windows 现实约束**：tmpdir symlink 用例依赖 `fs.symlinkSync`，无 Developer Mode/权限即抛错——用例须在此情形下跳过；此限制亦可作为 not-applicable 的正当理由 |
| W7-2 台账回滚 | 流程/git 卫生层，rot checker 不可表达 ⇒ not-applicable |
| P1-C3 编译红未察觉 | CI/桌面编译闸层（家规 6 已覆盖）⇒ not-applicable |
| W15-1l 续单扩围 | verify-task-gate 层（其 selftest 已有 W15 用例）⇒ not-applicable |

裁决标准：只有能写成「checker 对某输入必须红/绿」的机械判据才转 fixture；否则 not-applicable，不强行实现。

### 4. E02 新旧规则对照（用户拍板 D08）

- tmpdir 中 `git show df5c1ce:scripts/verify-rot-budget.mjs` 取旧版 checker，与本波全部 fixture 及合成场景逐一对照新旧 checker 判定。**对照场景底线清单**：5 个 fixture 文件 + 1001 行无 lease / allow-god-file 注释 / exempt-list / rotScanScope attestation / 退役净增 + **`--base` 家族三例**（未授权上调 / 过期授权 / 删指标）+ **扫描面两例**（scripts/installer 纳入 file-lines——本波最大覆盖变化）+ rubric mismatch + deadSince 31 天升级；**另须以 selftest 53 项负例清单为底册查漏补缺**；条件槽位第 6 个 fixture 若落成也进对照集。
- 输出**新增拒绝清单**与**解除拒绝清单**进 report（每条：场景 + 旧判定 + 新判定）；不建模拟器、不单独立项、不产生仓库文件。

## Constraints（自波次计划 Global Constraints 摘录，按本单语境适配占位符口径）

1. 纯 Node 标准库，零新依赖；脚本输出与日志 English-only。
2. **任何 ceiling 不得上调**；manifest 删指标视同上调（禁止）。本单不改 `scripts/rot-budget.json`、`scripts/exception-leases.json`。
4. checker 改动**先负向 fixture 后实现**（GC4）：report「旧代码行为确认」节如实记录旧代码无 registry 校验（fail-open by absence），**并贴「本单新用例对旧代码跑 `node scripts/verify-rot-budget.test.mjs`」的命令原文输出 + exit code——预期形态钉死：旧代码无新具名导出 ⇒ 模块加载失败（no export named …）整文件红，如实记录即 fail-open-by-absence 证据**。
6. 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
7. commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-7)` 后缀。
8. 产物内本地绝对路径用 `<REPO_ROOT>` 等占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
10. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 禁区

- 禁改收集器/扫描面/其他校验规则的任何部分；禁改 `scripts/` 下其他文件；禁改 `verify-task-gate.mjs`；禁改 5 个既有 fixture 文件内容（只允许登记其哈希）。
- 禁为过关调整 fixture 判定标准。
- 禁上调任何 ceiling、禁删 manifest 指标。
- E02/D-1 不在仓库内留任何运行残留。

## 验证

1. `node scripts/verify-rot-budget.mjs` —— 绿；读数零漂移（5 grep 同数、dir_entries:scripts=45/48、9 god-file rules、checkedFiles 1397、verdict: rotting）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（53 项 + 起步 registry 校验绿）。
3. `node scripts/verify-rot-budget.mjs --base 6dc9b13` —— 绿（顶层文件数不变）。
4. `node scripts/verify-rot-budget.test.mjs` —— 全绿（既有 26 + 本单新增）。
5. `node scripts/verify-task-gate.mjs validate-policy` —— 绿。
6. 篡改演示（见功能 1 末条）—— 红→还原→绿，原文输出进 report。
7. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-7-report.md`，必含三节：**改动摘要**（含 D-1 判定表 + E02 新增/解除拒绝清单）/ **验证**（7 条命令原文输出 + exit code + 旧代码行为确认 + GC4 负向先行证据）/ **状态**（状态词结尾）。
