# W16 计划：最小可信核（Phase -1）+ 存量债解封

- 日期：2026-09-05
- 来源：`E:\agent-project\.opencode\external-review\2026-09-05\D-synthesis-plan-2026-09-05.md` §3 Phase -1 + §9 用户拍板记录
- BASE：`19349cd`（main HEAD，工作树已验干净）
- 波次目标：建立最小可信核——此后所有改动（含 Phase 0 checker 加固）在核的约束下产出，解自举悖论（F-M-2）。

## 本波范围（只这些，不蔓延）

| 任务 | 内容 | 相位映射 | 可并行性 |
|---|---|---|---|
| W16-1 | 工作流闸：`scripts/workflow-policy.json`（SSOT 雏形）+ `scripts/verify-task-gate.mjs`（attempt 身份 + allowlist 机械比较 + brief/report schema 校验，内联 `--selftest`）+ `scripts/rot-budget.json` scripts 额度 42→48 | -1.1/-1.2/-1.3/-1.7 | 与 2/3/4 文件集不相交 |
| W16-2 | 审查包 manifest：改造 `assemble-review-pkgs.ps1`，输出 `package-manifest.json`（文件清单 + sha256 + 脚本版本 + 缺失项显式 `OMITTED(reason)`） | -1.6 | 与 1/3/4 不相交（不同仓库目录） |
| W16-3 | 流程文档：AGENTS.md/AGENTS-CN.md 增补家规 8（commit-bound 闸：attempt 身份、续单 = 新 attempt+新 brief、CANNOT_VERIFY 分级、meta-ratchet 路径清单） | -1.4 文档侧 + D-2 + D-8 | 与 1/2/4 不相交 |
| W16-4 | theme.rs 行数预算内修复（unsafe O_NONBLOCK + 死代码腾行，净增 ≤0） | 拍板项 1 执行 + F-B-7 存量补登 | 与 1/2/3 不相交 |

编排者 memory 侧更新（BOOTSTRAP/CORE 增补 brief review 环节与选派）由编排者本人执行，不派子代理。

## Global Constraints（逐字钉死，进每个 brief 的 constraints 块）

1. 纯 Node 标准库，零新依赖；PowerShell 脚本兼容 pwsh 7。
2. `scripts/rot-budget.json`：除 `dir_entries:scripts` 按拍板 42→48（note 记拍板日期 + 到期 2026-10-15 + commit message 引拍板原文）外，**任何 ceiling 不得上调**。
3. 日志与脚本输出 English-only（仓规）。
4. 所有验证命令必须在 report 贴原文输出（命令 + exit code）。
5. commit 规则：逐文件点名 `git add`，禁 `git add -A`；message 前缀 `feat(scripts):` / `docs:` / `fix(cli):` + `(W16-N)` 后缀。
6. W16-4 专项：净行数变动 ≤0（`rg -c "^"` 前后对比贴出）；unsafe 块必须带 `// SAFETY:` 注释；cfg(unix) 代码本地 MSVC 编译覆盖不到，须在 brief 指定命令下做跨 target check 或显式标注由 CI ubuntu 兜底。
7. 每单 report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 任务卡

### W16-1 工作流闸脚本

- **允许文件集**（越界 = Critical）：
  - `scripts/workflow-policy.json`（新增）
  - `scripts/verify-task-gate.mjs`（新增，自测内联 `--selftest`，不另建测试文件）
  - `scripts/rot-budget.json`（仅 `dir_entries:scripts` 42→48 + note）
- **功能要求**：
  1. policy.json 字段：`version`、`judgeChecklist`（canonical 9 项：spec 逐条 / quality 独立判断 / 复用核查 / 无 owner 抽象 / 预算闸 / 条件早退 / god-file 观测（非空转版）/ pure-move 等价 / evidence 抽查）、`statusWords`、`reviewVerdicts`（含 `APPROVE_WITH_CONCERNS`）、`cannotVerifyPolicy`（判定性证据阻塞 / 辅助证据 ≤2 项且不触 trust boundary ⇒ APPROVE_WITH_CONCERNS + owner + 截止）、`metaRatchetPaths`（改这些路径的 commit 升最高审查车道）、`briefRequiredSections`、`reportRequiredSections`。
  2. gate 脚本子命令：`verify-attempt --base <sha> --tip <sha> --allowlist <file>`（`git diff --name-only` 与 allowlist 精确比较，越界非零退出，输出列出越界文件）；`validate-brief <path>`（必含节 + 禁预设豁免短语无拍板标注 + 禁 "do not flag" 类预判审查措辞）；`validate-policy`（schema 自校验）。`--selftest` 跑负向 fixture（越界文件 / 错 TIP / 缺节 brief / 坏 policy）必须全红、正向必须全绿。
  3. 续单语义写进 validate-brief：brief 中含"续单"字样必须同时含独立 BASE 与 allowlist 节。
- **验证最小集**：`node scripts/verify-task-gate.mjs --selftest` 全绿 + `node scripts/verify-rot-budget.mjs` 绿（额度消耗后 44/48）+ `pnpm run check:repo-hygiene`。
- **skill 前置**：`E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`（遵循与本任务相关约定，不扩展范围）。

### W16-2 审查包 manifest

- **允许文件集**：`E:\agent-project\.opencode\external-review\assemble-review-pkgs.ps1`（注意：实际路径以磁盘为准，当前在 `2026-09-05/` 子目录，改造后应说明归档位置或上移）。
- **功能要求**：组装时输出 `package-manifest.json`：每项材料的 path + sha256 + bytes + 组装脚本版本；应有而缺失的材料必须显式写 `{"status":"OMITTED","reason":"..."}`，禁止静默空节。
- **验证**：重跑组装现有 A/B 包目录，manifest 输出正确；故意移走一个材料文件，manifest 中该项为 OMITTED 而非静默。

### W16-3 家规 8（双语文档）

- **允许文件集**：`AGENTS.md`、`AGENTS-CN.md`。
- **内容**（家规 8，commit-bound workflow gate）：
  1. 任务验收以 BASE_SHA/TIP_SHA + allowlist 为界；`node scripts/verify-task-gate.mjs verify-attempt` 越界即失败。
  2. 续单 = 新 attempt：必须有独立 brief（含 BASE 与 allowlist），不接受事后叙述扩围。
  3. 审查结论状态机：PASS / FAIL / CANNOT_VERIFY / BLOCKED；CANNOT_VERIFY 按 `scripts/workflow-policy.json` 的 `cannotVerifyPolicy` 分级，禁止直接转 APPROVE。
  4. meta-ratchet：修改 `policy.json metaRatchetPaths` 所列文件的 commit 自动升最高审查车道（双 judge + 用户拍板）。
  5. `APPROVE_WITH_CONCERNS` 是一等状态，"无法确定"不被惩罚，但必须有 owner + 截止。
- **验证**：`pnpm run check:repo-hygiene` + 双语文档结构一致（逐条对照）。

### W16-4 theme.rs 行数预算内修复

- **允许文件集**：`src/apps/cli/src/ui/theme.rs`。
- **修复清单**（deep audit 2026-09-05 实据）：
  1. unsafe 块（L164-194）：加 `// SAFETY:` 注释；`fcntl(F_SETFL)` 恢复调用（L193）返回值检查 + 失败时 warn，消除 O_NONBLOCK 泄漏。
  2. 删死 API `load_opencode_theme_json`（L728 附近）腾行。
  3. 删两个误标 `#[allow(dead_code)]`（StyleKind L637、OpencodeThemeJson.defs L700）——deep audit 已坐实为活符号。
  4. 修正两条陈旧注释（L635 StyleKind 与现实矛盾、L215 parse_osc_color "not yet wired" 部分错误）。
  5. 两条 allow(dead_code) 删除后 `allow_dead_code` 计数应 −2（顺带只降不升）。
- **约束**：净行数 ≤0；不改任何运行时行为（除 fcntl 错误处理本身）；颜色数学零触碰。
- **验证最小集**：`cargo check -p <cli crate 名以实际为准>` + `cargo test -p <cli> theme` + `node scripts/verify-rot-budget.mjs` 绿 + 行数前后对比。cfg(unix) 覆盖问题在 brief 中钉死方案（见 Global Constraint 6）。
- **skill 前置**：`E:\agent-project\.opencode\skills\rust-skills\unsafe-checker\SKILL.md` + `m15-anti-pattern`（遵循相关约定，不扩展范围）。

## 波次收口

- 四单全过后：波级终审（review-package `19349cd..HEAD`，reviewer-53）→ handoff → 台账。
- 本波本身即 Phase -1 的试点：W16-1 起，judge 验收必须跑 `verify-attempt`（哪怕基线期先用人工对照）。
- W16-1 brief 是首个走 brief review 的单：编排者写 brief → reviewer-53 审 → 修正 → 派发。
