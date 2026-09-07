# W18 计划：Phase 0 — checker 加固（fail-closed 化）

- 日期：2026-09-07
- 来源：`E:\agent-project\.opencode\external-review\2026-09-05\D-synthesis-plan-2026-09-05.md` §3 Phase 0（0.1~0.9 全项）+ §5 D-1/D-3 + Phase -1 顺延项（-1.5 headroom floor / -1.7 退役机械化）+ handoff `docs/handoffs/2026-09-06-w16-trusted-core-w17-windows-ci.md` §4 队列
- BASE：`df5c1ce`（main HEAD，工作树已验干净，2026-09-07）
- 波次目标：把 `verify-rot-budget.mjs` 从 fail-open 改为 fail-closed，并机械化 only-down / headroom floor / exception lease / 扫描范围 attestation / 退役规则 / mutation 预注册——堵 D 报告病理模型第 2 层「规则强度 > 执行机制强度」。

## 2026-09-07 外部审查并入（用户三项拍板：D 表全批 / 严格一增一退 / 混合准入）

- 新增 **W18-0**（rename 检测修复）；档 A 五条以各单 brief 补充条款带入，不单独立项。
- **目标 schema 前置**（D07 硬阻塞）：W18-1 的字段白名单直接按终态写，避免逐单返工。终态 entry 字段白名单 = `kind` / `pattern` / `ceiling` / `note` / `dir` / `paths`（exempt-list 专用）/ `action` / `authorization` / `deadSince`；终态 kind 枚举 = `grep-count` / `file-lines` / `dir-entry-count` / `exempt-list`。各单只实现自己需要的字段，但 W18-1 的 validator 结构须可容纳终态（未知字段拒绝逻辑按白名单表驱动，后续单只加表项不改结构）。
- 顺延到下一波：P02 最小版（CI merge-base BASE + 审查绑冻结 SHA）、P04/P05（先定「实质放宽」分级标准再动 schema 语义）、E01（观察一波）。
- **实测勘误**：`i18n-audit.mjs` checker `countLines` 口径 = 3833（handoff 写 3834，差 1 不改结论）。

## 实测基线（checker 口径，2026-09-07）

| 指标 | 读数 | 余量 |
|---|---|---|
| god_file:theme.rs | 979/989 | 10 |
| god_file:css.rs | 790/790 | **0** |
| god_file:pages_onboarding.rs | 859/866 | 7 |
| god_file:memory_db.rs | 849/859 | 10 |
| god_file:selectors.rs | 827/827 | **0** |
| god_file:manager.rs | 836/836 | **0** |
| unix_epoch_inline | 69/69 | **0** |
| unwrap / expect / let_ / dead_code | 483/502、940/1089、370/388、104/109 | 有余量 |
| dir_entries:scripts | 44/48 | 4（到期 2026-10-15） |
| checkedFilesCount | 1368（仅 `src/`） | — |

零余量自锁（F-B-6）仍在 4 处。本波的 headroom floor + exception lease 即解封通道。

## 本波范围（8 单，严格串行——全部触碰 `verify-rot-budget.mjs`/manifest 或 `verify-task-gate.mjs`，文件集互斥不可并行）

| 任务 | 内容 | 相位映射 |
|---|---|---|
| W18-0 | task-gate rename 漏检修复（`--name-only` → `--name-status -z`，rename 两端路径都过 allowlist） | 外部审查档 A |
| W18-1 | `validateManifest()` fail-closed + D-3 红队 probe 首批（schema 级反例 + 阈值边界 fixture） | 0.1 + D-3 |
| W18-2 | only-down 机械化（`--base` 模式）+ headroom floor（-1.5）+ cap-and-archive 结构化 action | 0.2 + 0.5 + -1.5 |
| W18-3 | >1000 硬边界走 exception lease（禁复活 allow-god-file 注释，O-2 errata） | 0.3 |
| W18-4 | config 字段兑现或删承诺 + EXEMPT_FILE_PATHS 移入 manifest | 0.4 |
| W18-5 | 扫描范围 attestation（覆盖 installer + scripts 自身）+ -1.7 退役机械化 | 0.6 + O-1 + -1.7 |
| W18-6 | verdict rubric 互斥化进 SSOT + dead registration 限期升级 violation | 0.7 + 0.8 |
| W18-7 | mutation/fixture 集外部预注册锁定 + D-1 历史事故 replay 并入 | 0.9 + D-1 |

挂账不在本波：P2-23（terminal-core 非 Windows 编译）、P2-24（~170 历史文件 hygiene）、Phase 1 全部。

## Global Constraints（逐字钉死，进每个 brief 的 constraints 块）

1. 纯 Node 标准库，零新依赖；脚本输出与日志 English-only。
2. **任何 ceiling 不得上调**；manifest 删指标视同上调（禁止）。新增 `scripts/` 顶层文件记账（现 44/48，到期 2026-10-15，commit message 引拍板原文）；fixture 一律放 `scripts/fixtures/` 子目录（dir-entry-count 只数顶层文件，不占额度）。
3. **meta-ratchet 车道**：全部 7 单触碰 `metaRatchetPaths`（`scripts/verify-rot-budget.mjs` / `scripts/workflow-policy.json`），每单 = 双 judge + 用户拍板（家规 8.4），不豁免、不合并。
4. checker 改动**先负向 fixture 后实现**（D 纪律 10）：每单 `--selftest` 负例必须先在旧代码上确认行为、在新代码上全红，正向全绿。
5. **禁止复活 `allow-god-file` 头注释**（O-2 errata，PHASE-0 历史裁定）；>1000 只能走 exception lease。
6. 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
7. commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` / `fix(scripts):` / `docs:` + `(W18-N)` 后缀。
8. 产物内本地绝对路径用 `<USERPROFILE>` 占位（hygiene 闸）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
9. 严格串行：后单 BASE = 前单 TIP；每单 brief 先过 reviewer-53 brief review 再派发。
10. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED（policy `statusWords`）。

## 任务卡

### W18-0 task-gate rename 漏检修复（外部审查档 A）

- **允许文件集**：`scripts/verify-task-gate.mjs`。
- **功能要求**：`verify-attempt` 的 diff 枚举从 `git diff --name-only`（L232，rename 时只报新路径，旧路径逃逸 allowlist）改为 `--name-status -z` 解析；rename/copy 的**旧路径与新路径都必须命中 allowlist**，任一越界非零退出并列出。meta-ratchet 车道（该文件在 `metaRatchetPaths`）。
- **验证最小集**：`node scripts/verify-task-gate.mjs --selftest` 全绿 + 构造含 rename 的 BASE..TIP 演示越界拦截（贴原文输出）+ `pnpm run check:repo-hygiene`。
- **skill 前置**：`E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`。

### W18-1 validateManifest() fail-closed + D-3 红队 probe 首批

- **允许文件集**：
  - `scripts/verify-rot-budget.mjs`（修改）
  - `scripts/fixtures/rot-budget/`（新增目录，fixture JSON，不占 scripts 额度）
- **功能要求**：
  1. 新增 `validateManifest(manifest, projectRoot)`，在 `verifyRotBudget()` 入口最先调用；任何一条不通过 → 记入 violations 并非零退出（不再静默跳过坏 entry）。
  2. 校验规则：entry 必须有 `kind` ∈ {`grep-count`, `file-lines`, `dir-entry-count`}；`ceiling` 必须为有限非负整数（`Number.isInteger` && `>= 0`）；`grep-count` 的 `pattern` 必须非空字符串且 `new RegExp()` 可编译；`dir-entry-count` 的 `dir`（或 key 推导路径）经 `path.normalize` + 分隔符归一后必须 confinement 在 projectRoot 内（禁 `..` 越界、禁指外绝对路径）；entry 级未知字段拒绝（按 kind 白名单：`kind`/`pattern`/`ceiling`/`note`/`dir`）；`note` 若存在必须为字符串。
  3. `--selftest`（内联，不另建脚本文件）跑负向 fixture：bogus kind / 字符串 ceiling / `../` 路径越界 / 空 pattern / 未知字段——必须各自产生 violation；正向 fixture（当前 manifest 副本）必须全绿。**阈值边界 fixture（档 A）**：file-lines 恰好 800 / 801 行各一（800 绿、801 未登记红），证明阈值判定钉在边界上（mutation 证据：现有测试钉不住边界）。
  4. validator 字段白名单按「目标 schema 前置」的终态表驱动（结构可容纳 `paths`/`action`/`authorization`/`deadSince`，本单只启用现有字段）。
  4. 现有仓库 manifest 必须原样通过校验（若有现存 entry 不合规，先修 manifest 再交付，diff 里说明）。
- **验证最小集**：`node scripts/verify-rot-budget.mjs --selftest`（负例全红、正例全绿）+ `node scripts/verify-rot-budget.mjs` 绿 + `pnpm run check:repo-hygiene`。
- **skill 前置**：`E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`（遵循与本任务相关约定，不扩展范围）。

### W18-2 only-down 机械化 + headroom floor + cap-and-archive action

- **允许文件集**：`scripts/verify-rot-budget.mjs`、`scripts/rot-budget.json`、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. `--base <sha>` 模式：`git show <sha>:scripts/rot-budget.json` 取 BASE manifest 逐 key 比较——TIP ceiling > BASE ceiling ⇒ violation，除非该 entry 携带未到期结构化授权 `authorization: {reason, commit, expires}`（ISO 日期，过期无效）；BASE 有而 TIP 无的 key ⇒ violation（禁删指标）。无 `--base` 时跳过（本地快速检查不强制 git）。
  2. headroom floor（-1.5）：`--base` 模式下，file-lines 类 ceiling 下调必须满足 `newCeiling ≥ current(TIP) + max(5, ceil(current × 0.05))`；违反 ⇒ violation 并指引走 W18-3 exception lease。另：对现存零余量登记项（当前 css/selectors/manager/epoch 四处）输出 warning 清单（非 violation），指引 lease 通道。
  3. cap-and-archive 结构化（0.5）：`dir_entries:.superpowers/sdd` entry 增 `"action": {"type": "cap-and-archive", "archiveTo": "docs/archive/sdd-artifacts/"}` 字段；触发时 checker 输出专用归档指引（非通用 violation 文案）。validator 白名单相应扩 `action`。
  4. 新增负向 fixture：未授权上调（红）/ 授权未到期上调（绿）/ 违反 floor 的下调（红）/ 删指标（红）。
- **验证最小集**：`--selftest` 全绿 + `node scripts/verify-rot-budget.mjs --base df5c1ce` 绿 + `node scripts/verify-rot-budget.mjs` 绿 + hygiene。
- **skill 前置**：`anti-rot-system`。

### W18-3 >1000 硬边界 exception lease

- **允许文件集**：`scripts/verify-rot-budget.mjs`、`scripts/exception-leases.json`（新增，scripts 44→45/48，commit message 记账）、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. lease schema：`{file, owner, reason, revisit_after (ISO date), next_action}`；live = `revisit_after >= 今日`。
  2. 规则：file-lines 读数 > 1000 ⇒ violation，除非存在该文件的 live lease；lease 过期 ⇒ violation（阻塞，不自动续）。零余量登记项（800~1000 区间）可登记 lease 以消除 W18-2 的 warning。
  3. O-2 机械化：扫描文件内容中出现 `allow-god-file` 注释 ⇒ violation（注释方案已历史裁定删除，防复活）。
  4. fixture：无 lease 的假想 >1000 文件（红）/ 过期 lease（红）/ live lease（绿）/ 含 `allow-god-file` 注释文件（红）。**边界 fixture（档 A）**：恰好 1000 行（绿，不触发硬边界）/ 1001 行无 lease（红）；lease `revisit_after` = 今天（live，绿）/ 昨天（expired，红）——日期边界钉死。
  5. **本单是 W18-5 的硬前置**（D07）：lease 机制未落地前，W18-5 不得扩扫描范围。
- **验证最小集**：`--selftest` 全绿 + `node scripts/verify-rot-budget.mjs` 绿（现有 6 登记项均 ≤989 不触发）+ hygiene。
- **skill 前置**：`anti-rot-system`。

### W18-4 config 字段兑现 + EXEMPT_FILE_PATHS 移入 manifest

- **允许文件集**：`scripts/verify-rot-budget.mjs`、`scripts/rot-budget.json`、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. manifest 增保留 entry：`"exempt_generated_files": {"kind": "exempt-list", "paths": [3 条 generated_locale_contract.rs 路径]}`；W18-1 的 kind 白名单扩 `exempt-list`（paths 必须非空字符串数组、每条 confinement 在 projectRoot 内）。脚本内 `EXEMPT_FILE_PATHS` 常量删除，改从 manifest 读（F-B-16 静默无效配置归零）。
  2. **豁免路径笔误修正（档 A，外部审查实证）**：现行 `EXEMPT_FILE_PATHS` 第 3 条 `northhing-installer/...`（双 h）目录不存在、是死豁免；移入 manifest 时改为 `northing-installer/src-tauri/src/installer/generated_locale_contract.rs`，commit message 与 report 必须显式记录「口径修正：原豁免项因笔误从未生效，本次为修复而非原样搬运」。
  3. config 承诺审计：grep manifest note 字段与 `docs/` 中对 checker config 的引用，凡文档承诺而脚本未实现的字段，本单内实现或删文档承诺——逐条列进 commit message。
  4. fixture：exempt-list 生效（被豁免文件不计数）/ exempt-list 内含越界路径（红）。
- **验证最小集**：`--selftest` 全绿 + `node scripts/verify-rot-budget.mjs` 绿且读数与 BASE 完全一致（豁免语义不变）+ hygiene。
- **skill 前置**：`anti-rot-system`。

### W18-5 扫描范围 attestation + scripts 退役机械化

- **允许文件集**：`scripts/verify-rot-budget.mjs`、`scripts/rot-budget.json`、`scripts/workflow-policy.json`、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. policy.json 增 `rotScanScope`：`{"grepRoots": ["src"], "fileLinesRoots": ["src", "northing-installer/src-tauri", "scripts"]}` 为声明单源。checker 启动时断言实际扫描根 == 声明；不一致 ⇒ violation（扫描范围变化自动报警，O-1）。
  2. file-lines 收集器扩展：`northing-installer/src-tauri`（.rs）与 `scripts`（顶层 .mjs/.js）纳入 god-file 登记扫描；**grep-count 规则仍只扫 `src/`（语义不变，读数不得漂移）**。scripts 收集沿用测试排除口径：**`*test*.mjs` / `*test*.js` 不入扫描**（与 .rs 的 tests 排除同口径——用户 2026-09-07 拍板，`i18n-contract.test.mjs` 1042 行据此豁免）。
  3. **超千行准入（用户 2026-09-07 拍板，混合）**：`scripts/i18n-audit.mjs`（3833 行，checker 口径）登记 manifest + 持 exception lease（W18-3 机制；lease 内容 = 拆分计划或类型豁免理由，revisit_after 由 lease 审批定）；`northing-installer/src-tauri` 下若有 >800 行文件需同单登记 + commit message 引用户拍板。派发前由编排者实测确认清单写进 brief。
  4. -1.7 退役机械化（口径 = **严格一增一退**，用户 2026-09-07 拍板）：`--base` 模式下 `scripts/` 顶层文件数较 BASE 上升 ⇒ 要求 BASE..TIP diff 中 ≥1 个 `scripts/` 顶层文件被删除（先退后增），否则 violation——一次性额度只兜历史存量，不兜新增。
  5. fixture：扫描根声明被篡改（红）/ scripts 净增无退役（红）/ 净增有退役（绿）/ 测试命名文件不计入（绿）。
- **验证最小集**：`--selftest` 全绿 + `node scripts/verify-rot-budget.mjs --base df5c1ce` 绿 + grep 读数与实测基线逐一对照贴出（证明无漂移）+ hygiene。
- **skill 前置**：`anti-rot-system`。
- **规模预警**：本单是波内最大单（两个机制 + 收集器扩展）；brief 阶段若 reviewer-53 判过大，拆 5a（attestation）/ 5b（退役机械化）。

### W18-6 verdict rubric 互斥化 + dead registration 限期

- **允许文件集**：`scripts/workflow-policy.json`、`scripts/verify-rot-budget.mjs`、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. policy.json 增 `rotVerdictRubric`：`{"healthy": "0 findings", "stable": "1-2 bounded findings", "rotting": ">=3 findings OR any unbounded"}` 为三处尺度（checker 输出 / anti-rot skill / audit 报告模板）的唯一来源；checker 摘要输出引用该 rubric 口径。
  2. dead registration 限期（0.8）：manifest god-file entry 增可选 `deadSince`（ISO 日期）；文件缺失时 checker warning 指引补登 `deadSince`；`deadSince` 已登且距今 > 30 天 ⇒ 升级 violation。
  3. fixture：dead entry 无日期（warning 不红）/ dead 31 天（红）/ 文件存活（无 warning）。
  4. **P06 入口文档同步（档 A）**：对齐 `AGENTS.md` 与 anti-rot skill 头注释中关于 god-file 口径的矛盾表述（以 checker `countLines` 为现行闸口径；只修换行产生的下降不得报告为结构改善，P09）。
  5. **P03 最小版（可选并入，brief review 判是否超范围）**：policy.json 按任务类型列 required 审查材料清单字段；若判超范围则顺延下波。
- **验证最小集**：`--selftest` 全绿 + `node scripts/verify-rot-budget.mjs` 绿 + hygiene。
- **skill 前置**：`anti-rot-system`。

### W18-7 mutation/fixture 集外部预注册锁定 + D-1 replay

- **允许文件集**：`scripts/fixtures/rot-budget/registry.json`（新增，子目录不占额度）、`scripts/workflow-policy.json`、`scripts/verify-rot-budget.mjs`、`scripts/fixtures/rot-budget/`。
- **功能要求**：
  1. `registry.json`：每个 fixture 文件 path + sha256 + 用途标注；`--selftest` 先校验 fixture 哈希与 registry 一致，漂移 ⇒ fail（执行者只能实现不能定义，handoff 5.3）。
  2. policy.json `metaRatchetPaths` 增补 `scripts/fixtures/rot-budget/registry.json`（改 fixture 集 = 改判据，升最高车道，需用户拍板）。
  3. D-1 replay 并入：4 起历史事故（W15-1g 符号链接假绿 / W7-2 台账回滚 / P1-C3 编译红未察觉 / W15-1l 续单扩围）逐条判定——rot checker 可表达的转为 fixture；不可表达的（属 verify-task-gate 或 CI 层）在 registry.json 标注 `not-applicable` + 理由，不强行实现。
  4. **E02 并入（用户拍板 D08）**：用本波全部 fixture 跑「旧规则（`df5c1ce` 版 checker）vs 新规则」对照，输出新增拒绝 / 解除拒绝清单进 report——不单独立项建模拟器。
- **验证最小集**：`--selftest`（含哈希校验）全绿 + 篡改任一 fixture 后 `--selftest` 必红（演示贴出）+ `node scripts/verify-task-gate.mjs validate-policy` 绿 + hygiene。
- **skill 前置**：`anti-rot-system`。

## 波次收口

- 8 单全过后：波级终审（review-package `df5c1ce..HEAD`，reviewer-53）→ handoff skill 收口 → 台账 ledger_append 并即时 commit（ledger 行带派发/通过时间戳 + 返工轮数，效率规则「耗时账」）。
- 每单流程钉死：编排者写 brief → **reviewer-53 brief review**（五判据）→ 修正 → 派 `gemini-38-flash-agy` → judge `minimax-m3`（meta-ratchet 双 judge 位：+ reviewer-53）→ 用户拍板 → ledger。
- 波完成后 Phase 0（0.1~0.9）全项闭环，下一波 = Phase 1 提交绑定闸（D 报告 §3）。

## Self-review 追溯

0.1→W18-1 / 0.2→W18-2 / 0.3→W18-3 / 0.4→W18-4 / 0.5→W18-2 / 0.6→W18-5 / 0.7→W18-6 / 0.8→W18-6 / 0.9→W18-7 / D-3→W18-1（schema 级 5 例；上调/lease 反例随依赖单落地）/ D-1→W18-7 / -1.5→W18-2 / -1.7→W18-5 / O-1→W18-5 / O-2→W18-3。外部审查档 A：rename→W18-0 / 豁免笔误→W18-4 / 边界 fixture→W18-1+W18-3 / P06→W18-6 / 退役口径→W18-5；E02→W18-7；D07→W18-3 先于 W18-5 + 目标 schema 前置。Phase 0 九项 + 顺延两项 + 档 A 五条全覆盖，无孤儿。
