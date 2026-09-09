# W21-3 Brief — P03 最小版：workflow-policy.json 审查材料清单字段

## 任务标识

W21-3（清账波次单；P03 最小版，2026-09-07 用户拍板方案 B：「按任务类型从 policy 文件取 required 材料清单」；2026-09-09 用户拍板按编排者推荐方案执行，meta-ratchet 签字已含）。

## BASE

`4923af4`（main HEAD，工作树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 4923af4 代码树等价，仅 docs 差异）。

## 背景与 BASE 证据（编排者已实测）

- `scripts/verify-task-gate.mjs` `validatePolicy`（line ~68 起）逐字段校验已知字段，**无未知字段拒绝**——新增字段不触发 validator 改动（task-gate 余量 13 行不受影响）。
- `scripts/verify-rot-budget.mjs` 的 policy attestation 只查 `rotScanScope` / `rotVerdictRubric` 两个具名字段，同样不拒未知字段。
- `scripts/workflow-policy.json` ∈ `metaRatchetPaths` → 本单双 judge 车道 + 用户签字（已获）。
- 执行点（编排者侧审查包组装工具）在仓外 `.opencode/tools/`，本单只落数据字段，仓内零代码改动。

## 允许文件集

- `scripts/workflow-policy.json`（修改，仅加一个字段，见功能 1）

report 写 `.superpowers/sdd/w21-3-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **新增字段**：`workflow-policy.json` 顶层加：
   ```json
   "reviewPackageRequired": { "default": ["brief", "diff", "report"] }
   ```
   位置：紧随 `reportRequiredSections` 之后（保持字段分组语义）。JSON 合法、缩进与文件既有风格一致（2 空格）。不多加任何其他键——任务类型分化的 shape 已预留（对象 keyed by 类型），只有 `default` 有实例（YAGNI：第二个类型出现时再加）。

## Constraints

- commit 逐文件点名 `git add`（禁 -A，1 文件）；message 前缀 `ci(workflow-policy):` 或 `chore(workflow-policy):` + `(W21-3)` 后缀，body 注明「P03 最小版，用户拍板 2026-09-09（编排者推荐方案）；meta-ratchet 签字含于拍板」。
- report 贴原文输出 + exit code；结尾状态词。
- 本单触碰 `metaRatchetPaths`，验收走双 judge 车道。

## 禁区

- 禁动 policy.json 其他任何字段；禁动 verify-task-gate.mjs / verify-rot-budget.mjs 及任何清单外文件。
- 禁在仓内写消费代码（执行点在仓外，不在本单范围）。

## 验证（编排者已在 BASE 预跑：全部绿）

1. `node scripts/verify-task-gate.mjs validate-policy` → 修复后仍 `Policy validation passed`，exit 0（证明无未知字段拒绝；BASE 已实测现状绿）。
2. `node scripts/verify-rot-budget.mjs` → exit 0，verdict 输出与 BASE 等价（attestation 不受新字段影响；warnings 属设计内）。
3. `node scripts/verify-task-gate.mjs --selftest` → 全 pass（policy 相关 fixture 不回归）。
4. report 附：改动后 policy.json 的 `node -e "JSON.parse(...)"` 合法性验证输出 + 新字段内容原文。

## skill 前置

无强相关；不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w21-3-report.md`，必含三节：**改动摘要** / **验证**（4 项原文输出 + exit code）/ **状态**（状态词结尾）。
