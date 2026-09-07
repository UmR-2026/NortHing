# W18-4 Brief — config 字段兑现 + EXEMPT_FILE_PATHS 移入 manifest

## 任务标识

W18-4（波次计划：`.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md` v2；相位映射 0.4 + F-B-16）。前置：W18-1（FIELD_WHITELIST 表驱动）已落地。

## BASE

`eecb611`（main HEAD，工作树已验干净）

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改）
- `scripts/rot-budget.json`（仅：新增 `exempt_generated_files` entry；零触碰任何既有 ceiling/key/pattern/note）
- `scripts/verify-rot-budget.test.mjs`（**仅限用例 5 单点改写**：原用例依赖脚本常量里的死路径豁免（合成 1200 行 `src/shared/i18n/generated_locale_contract.rs` + 空 manifest 断言绿），常量删除后须改写为 manifest 豁免语义——合成 manifest 携带 `exempt_generated_files` entry 覆盖该合成路径，保留「豁免文件 >800 行放行」的断言目标；其余 11 个用例零触碰）

report 写 `.superpowers/sdd/w18-4-report.md`，不进本单验收 diff。fixture 全部 selftest 内联 tmpdir 合成，不新增文件。

## 背景

`EXEMPT_FILE_PATHS` 硬编码在脚本里（L8-12），manifest 读者不可见、不受 validateManifest 约束（F-B-16 静默无效配置）。且外部审查实证第 3 条 `northhing-installer/...`（双 h）是笔误死豁免（真实目录 `northing-installer`）。编排者实测三条豁免现状：`src/shared/i18n/generated_locale_contract.rs` **不存在**（死路径）、`src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 存在、`northing-installer/src-tauri/src/installer/generated_locale_contract.rs` 存在（修正后路径）。**注：计划任务卡写 paths 3 条，本 brief 收窄为 2 条（死路径不搬运），属相对计划卡的已记录偏差，理由如上。**

## 功能要求

1. **EXEMPT 移入 manifest**：`scripts/rot-budget.json` 新增 entry：
   `"exempt_generated_files": {"kind": "exempt-list", "paths": ["src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs", "northing-installer/src-tauri/src/installer/generated_locale_contract.rs"]}`
   —— 共 2 条：笔误修正为 `northing-installer`（单 h）；死路径 `src/shared/i18n/generated_locale_contract.rs` **删除**（文件不存在，F-B-16 静默无效配置归零；若将来重新生成届时再加）。脚本内 `EXEMPT_FILE_PATHS` 常量删除，改从 manifest 读；豁免语义（跳过扫描，不进 god-file/grep 计数）不变。
2. **口径修正显式记录**（外部审查档 A 硬要求）：commit message 与 report 必须写明「原豁免项第 3 条因笔误（northhing 双 h）从未生效，本次为修复而非原样搬运；第 1 条 src/shared 为死路径，删除」。
3. **validator 扩展**（加表项 + 该 kind 的校验分支 + ceiling 校验按 kind 分发）：`FIELD_WHITELIST` 加 `exempt-list` kind（字段白名单：`kind`/`paths`/`note`）；`paths` 必须为非空字符串数组、每条归一化后 confinement 在 projectRoot 内（复用/对齐 dir 的 confinement 检查）。**显式授权**：ceiling 必填校验改为按 kind 分发——`FIELD_WHITELIST[kind]` 含 `ceiling` 的 kind 才必填 ceiling，`exempt-list` 豁免该检查。`exempt-list` 不进 grep/god-file/dir 任何规则桶。
4. **config 承诺审计**（0.4 后半）：grep `scripts/rot-budget.json` 的 note 字段与 `docs/`（重点 `docs/status/`、`docs/architecture/`）中对 checker config 的引用，凡文档承诺而脚本未实现的 config 字段，处置 ∈ {本单实现（仅脚本侧）| report 列出、移交编排者另开文档单}——逐条列进 report（每条：出处 / 承诺内容 / 处置）。预期命中面不大；若发现需要大改的条目 ⇒ DONE_WITH_CONCERNS 上报，不硬做。
5. **selftest 扩展**（内联沿用既有结构）：
   - 负例：exempt-list 的 paths 含 `../` 越界（红）/ paths 为空数组（红）/ paths 含非字符串（红）/ exempt-list 带 ceiling 字段（红，未知字段）；
   - 正例：合成 projectRoot 中含 801 行豁免路径文件 ⇒ 不触发 god-file violation（豁免生效语义钉死；与 W18-3 的 >1000/注释禁令的交互按「豁免语义不变」——豁免文件跳过全部检查）；
   - 既有 33 项保持全绿。
6. 纯 Node 标准库，零新依赖；输出 English-only。

## Constraints（逐字自波次计划 Global Constraints，与本单相关项）

- 任何 ceiling 不得上调；manifest 删指标视同上调（禁止）。本单对 `rot-budget.json` 的改动 = 仅新增 `exempt_generated_files` entry。
- commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `feat(scripts):` + `(W18-4)` 后缀 + 口径修正记录。
- 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
- 产物内本地绝对路径用 `<USERPROFILE>` 等占位（hygiene 闸；report 贴 pnpm 输出时 banner 路径用 `<REPO_ROOT>` 占位，W18-3 教训）；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 本单触碰 `metaRatchetPaths`（`scripts/verify-rot-budget.mjs`），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。
- checker 改动**先负向 fixture 后实现**（GC4）：report「旧代码行为确认」节如实记录旧代码对 exempt-list kind 的行为（W18-1 起为 unknown-kind 红）与死豁免笔误的实证。

## 禁区

- 禁改 `scripts/rot-budget.json` 任何既有 entry；禁改 `scripts/` 下其他文件（`verify-rot-budget.test.mjs` 的用例 5 单点改写除外，见允许文件集）、`scripts/fixtures/`、`docs/`（文档承诺的删除/修改一律移交，不进本单）。
- 禁改 grep 扫描范围/收集器语义（读数零漂移）。
- 禁为过关调整 fixture 判定标准。

## 验证

编排者已在 BASE `eecb611` 预跑基线（已实证绿）：主运行（读数 483/940/370/69/104；45/48、1/1；6 god-file；1368 files；零余量 warning 5 项）、`--selftest` 33 绿、`verify-rot-budget.test.mjs` 12 绿。交付验收：

1. `node scripts/verify-rot-budget.mjs` —— 绿，读数与基线零漂移（483/940/370/69/104；45/48、1/1；`.superpowers/sdd` 以基线实跑输出为准；6 god-file；1368 files；零余量 warning 5 项）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 全绿（33 + 本单新增）。
3. `node scripts/verify-rot-budget.mjs --base f2c55b8` —— 绿（exempt-list 新增不进 only-down 对比面；若触发意外 violation ⇒ 如实上报不硬修）。
4. `node scripts/verify-rot-budget.test.mjs` —— 绿（12 回归；用例 5 改写为 manifest 豁免语义后全绿，其余 11 用例语义不变）。
5. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`，遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w18-4-report.md`，必含三节：**改动摘要**（exempt-list 机制 + 口径修正记录 + config 审计逐条处置表）/ **验证**（5 条命令原文输出 + exit code + 旧代码行为确认）/ **状态**（状态词结尾）。
