# W19-2 Brief — checker 清账单（波终审 triage「记下一波」收口）

## 任务标识

W19-2（W18 波级终审 triage 处置 + W19-1 53 Minor 修复，一单清账）。前置：W19-1 拆分已落地（主文件 983/1050，余量 67 行；task-gate 807/847，余量 40 行）。无独立波次计划文档，本 brief 自带全部 constraints。**verdict 口径（findings 是否只计 violations）不在本单**——用户拍板待定，本单维持现状。

## BASE

`547c228`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 547c228 代码树等价，仅 docs 差异）。

## 允许文件集

- `scripts/verify-rot-budget.mjs`（修改，净增 ≤30 行，超 ⇒ BLOCKED）
- `scripts/verify-task-gate.mjs`（修改，净增 ≤40 行，超 ⇒ BLOCKED；ceiling 847 硬约束，上调须用户拍板）
- `scripts/verify-rot-budget.test.mjs`（修改：F2.5 断言收紧 + 尾部追加本单新用例块）
- `scripts/rot-budget.json`（修改，**仅** `god_file:scripts/verify-rot-budget.mjs` 的 note 字段，见功能 7）

report 写 `.superpowers/sdd/w19-2-report.md`，不进本单验收 diff。

## 功能要求

1. **`--flag=value` 形态 + 未知 flag fail-closed（两脚本统一）**：`verify-rot-budget.mjs` parseArgs（line ~940）与 `verify-task-gate.mjs` parseArgs（line ~695）支持 `--flag=value` 拆分（`--base=sha` 等价 `--base sha`）；未知 flag ⇒ 非零退出 + 英文错误（现状：`--base=sha` 静默降级为无基线模式，fail-open）。task-gate 的 parseArgs 可导出以便直接调用测试。
2. **注释禁令正则扩面**：`BANNED_COMMENT_REGEX`（verify-rot-budget.mjs:25）从仅 `//` 行注释扩展到 `///`（doc comment）与 `/* */` 块注释形态；**既有正例不破**（「非锚定 allow-god-file 字面量不触发」用例须保持绿——只拦行首/行内注释形态，不拦字符串字面量）。
3. **悬空 lease warning**：lease 登记的文件已不存在 ⇒ warning（不红），复用死登记预扫位置；live lease（文件存在）⇒ 无 warning。
4. **deadSince 未来日期守卫**：manifest `deadSince` > todayUtc ⇒ validateManifest 拒绝（fail-closed；终审授权的行为口径收紧）。
5. **F2.5 断言钉机制**：`verify-rot-budget.test.mjs` 中 deadSince 用例的断言从 `includes('deadSince')` 收紧为匹配新校验机制文案（如 `must match YYYY-MM-DD`），钉住机制而非巧合拒绝。
6. **task-gate「续单」误报**：`verify-task-gate.mjs:410` 的 `includes('续单')` 误伤「后续单」；修为不匹配「后续单」的形态（如负向后行断言），并补双向用例（真续单缺 BASE ⇒ 红；含「后续单」合规 brief ⇒ 绿）。
7. **ceiling note 追记修复**（W19-1 53 Minor）：`rot-budget.json` 该条目 note 恢复累积式：`"W18-5a 扫描范围扩面登记，用户拍板 2026-09-07；W19-1 selftest 外置拆分重组 2300→1050"`（仅 note，ceiling 不动）。
8. **registry 错误消息 polish**：`attestFixtureRegistry` 对 null/缺字段条目的 `fixture not found: ${...}` 消息改打印 `<invalid entry>` 兜底。
9. **E02 映射表**：report 附 selftest/test.mjs 用例 id ↔ E02 场景映射表（纯 report 内容，不进代码）。

## Constraints（自 W18 波 Global Constraints 摘录适配）

1. 纯 Node 标准库，零新依赖；脚本输出与日志 English-only。
2. **任何 ceiling 不得上调**；manifest 删指标禁止；本单 rot-budget.json 仅 note 字段。
4. checker 改动**先负向后实现**：report「旧代码行为确认」节贴「本单新用例对旧代码跑 ⇒ 红/误绿演示」原文输出 + exit code（如 `--base=1ac7438` 在旧代码静默降级 fail-open 的实测对照）。
6. 不得用 Markdown 说明替代机器校验；report 中验证命令贴原文输出 + exit code。
7. commit 逐文件点名 `git add`（禁 `-A`）；message 前缀 `fix(scripts):` + `(W19-2)` 后缀。
8. 产物内本地绝对路径用 `<REPO_ROOT>` 等占位；收口前必跑 `node scripts/check-repo-hygiene.mjs`。
10. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

另：本单触碰 `metaRatchetPaths`（两脚本 + rot-budget.json），验收走双 judge 车道（代码级批准已委托 judge 流程，用户拍板 2026-09-07）。

## 禁区

- 禁改收集器/扫描面/其他校验规则；禁改 registry.json、exception-leases.json、5 个 fixture、selftest-cases.test.mjs、workflow-policy.json。
- 禁为过关调整 fixture 判定标准；禁上调任何 ceiling。
- 禁动 verdict 口径（findings 计算维持现状）。

## 验证

1. `node scripts/verify-rot-budget.mjs` —— 绿；读数零漂移（5 grep、45/48、9 god-file、1397、verdict: rotting）。
2. `node scripts/verify-rot-budget.mjs --selftest` —— 53/53 绿（含既有「非锚定字面量不触发」用例）。
3. `node scripts/verify-rot-budget.mjs --base 547c228` —— 绿；`--base=547c228` 等价形态 —— 绿（新解析）。
4. `node scripts/verify-rot-budget.test.mjs` —— 33 + 本单新增全绿。
5. `node scripts/verify-task-gate.mjs --selftest` —— 13 + 本单新增全绿。
6. `node scripts/verify-task-gate.mjs validate-policy` —— 绿。
7. `pnpm run check:repo-hygiene` —— 绿。

## skill 前置

阅读 `E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`（含 2026-09-08 NortHing 部署注记），遵循其中与本任务相关的约定，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w19-2-report.md`，必含三节：**改动摘要**（含 E02 映射表）/ **验证**（7 条命令原文输出 + exit code + 旧代码行为确认 + GC4 负向先行证据）/ **状态**（状态词结尾）。
