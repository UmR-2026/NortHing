## 判决: REVISE

## Findings

- [Critical] **验证项 4 红态探针命令无法触发 A-4 fail-closed** — brief:43 写 `node scripts/verify-rot-budget.mjs` 必须 exit 1，但 `attestFixtureRegistry` 仅在 `runSelftest()` 路径被调用（`scripts/verify-rot-budget.mjs:939`），主路径 `verifyRotBudget()`（:547）根本不读 `registry.json`（全文件 grep `registry` 命中点仅 :384/:385/:388/:389/:391/:939）。移走 `registry.json` 后跑主命令 → main 路径无任何代码感知 registry 缺席 → exit 0，而非 brief 要求的 1。证据：本次实际跑 `node scripts/verify-rot-budget.mjs` 在 base（registry 在位）= exit 0；W19-1 报告（.superpowers/sdd/w19-1-report.md:6-8）已明确 attestFixtureRegistry 是「selftest 起步校验」而非 `verifyRotBudget` 内置项；W19-2 报告（:104-117）单独列出 `--selftest` 与无 flag 两条路径行为差异。
  - **修改处方**：brief:43 红态探针命令改为 `node scripts/verify-rot-budget.mjs --selftest`（与验证项 1 同源），违规清单期望 `registry missing` 文案对齐 implementer 拟定的 violation 字符串；并在 #4 末尾追加一句"探针后再跑 `node scripts/verify-rot-budget.mjs` 仍 exit 0 作为 main 路径与 registry 无关的旁证"。理由：meta-ratchet 车道红态证据只能从 selftest 路径获取；不改即 brief 不可派发（implementer 按字面跑会得到 exit 0，违反 brief；按 --selftest 跑会被视为偏离 brief）。

- [Minor] **F1.7 violation 文案未定** — brief:28 仅给语义（"registry missing = fixture 完整性无锁"）+ 风格参照（既有 `'registry fixtures missing or empty'`），但 implementer 须自定精确字符串并在 `verify-rot-budget.mjs:386` 修复行 + `verify-rot-budget.test.mjs:966` 测试断言之间逐字对齐，错位则 F1.7 fail。
  - **修改处方**：brief 功能要求 2 / 3 各加一行钉死 violation 字面量，例如 `fixture registry missing`（fix 与 test assertion 共用同一字符串）。一行的成本，消除 implementer 反复迭代。

## Cannot verify from diff
- 无（brief 改动尚未发生；本审查仅对照 BASE ff6e447 的当前文件 + brief 文本核验）

## 范围变动
- 无（brief 仍为文档/单元测试/metascript 三件未动到工作树）

## 增量复审（rev-1，13bdc94）

## 判决: PASS

- finding 1 (Critical): **已修复** — brief:43 红态探针命令改为 `node scripts/verify-rot-budget.mjs --selftest`，并显式注明 attestFixtureRegistry 仅在 :939 被调、主路径 verifyRotBudget 不读 registry（编排者已实测核对）；新增**旁证**条：registry 移走期间主路径 `node scripts/verify-rot-budget.mjs` 仍 exit 0（证明 main 路径行为不变）；恢复确认新增 **registry.json 内容 sha256 与探针前一致**（防恢复时文件被覆写/截断）。
- finding 2 (Minor): **已修复** — brief:28 功能要求 2 violation 文案钉死为 `fixture registry file missing (fail-closed): ${regPath}`，模板字面量用 `${regPath}` 引用既有变量；功能要求 3 加「断言必须逐字包含上面钉死的文案（fix 与 test assertion 逐字对齐）」，`fixture registry file missing` 子串足以覆盖 fix 输出的违规与测试断言的双向匹配（与 verify-rot-budget.test.mjs:955 既有 F1.6 `.includes('fixtures missing or empty')` 同款风格）。

## 新发现问题

- 无。
