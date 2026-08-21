# Review Brief — ROT-3' rot-budget 预算闸

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-rot-budget`（worktree，分支 feat/rot-budget-0821）
- 范围：`43c2c29..964afda`（单 commit）
- diff 包：`.superpowers/sdd/review-package-rot3p.diff`
- 实现 brief：`.superpowers/sdd/task-rot3p-brief.md`（spec 唯一来源）
- 实现者 report：`.superpowers/sdd/task-rot3p-report.md`

## 约束（本任务 spec 要求的精确值）

- `scripts/rot-budget.json` 的基线值必须是：unwrap_production=521 / expect_production=1098 / let_underscore=402；七个 god-file ceiling 依次为 1063/990/918/905/877/836/802（对应文件见 brief 表格）。逐字节核对。
- 计数口径：`src/**/*.rs` 排除 `**/tests/**` 与 `**/*_tests.rs`；生成物 `generated_locale_contract.rs` 豁免且以带注释常量数组形式存在。
- 附加规则：未登记生产 .rs >800 行必须报违规。
- 输出 English-only、无 emoji；报错须含 当前值/上限/修复指引。
- CI job 须含自测（不能只跑主检查）。
- 家规第 7 条须同时存在于 `AGENTS.md` 与 `AGENTS-CN.md` 且语义等价。

## 独立验证（你必须实跑）

1. `node scripts/verify-rot-budget.mjs`（exit 0？输出内容？）
2. `node scripts/verify-rot-budget.test.mjs`（自测是否真的构造了违规 fixture 并断言 exit 1——读测试代码确认，不只看 pass 数）
3. 手动验证一次"闸能变红"：临时把某 ceiling 调低 1（或临时把一个 >800 行文件条目删掉），跑 checker 必须 exit 1；**验证后还原**。也可以用 git stash 方式隔离，注意别污染工作树。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照实现 brief 的验收标准六条逐条判定 PASS/FAIL，给 file:line 证据。
2. **QUALITY**：代码质量独立判断。除常规项外，以下三条为必查项：
   - **复用核查**：实现者 report 的「复用侦察」一节是否存在且属实——抽查其声称参考/对齐 check-core-boundaries.mjs 与 i18n-audit.mjs 的说法；发现复制既有能力而不复用 = Important 起评。
   - **无 owner 抽象**：diff 中每个新增抽象必须绑定当前真实消费方；投机性抽象 = Important 起评。
   - **预算闸**：diff 若触碰 `scripts/rot-budget.json` 且是上调 ceiling/放松规则，除非 brief 附有用户拍板原文，一律 SPEC FAIL。（本任务是建闸本身，初始登记属正常；检查是否有夹带上调或放松。）

## Cannot verify from diff

无法从 diff 判定的项单独列出，禁止猜。

## 档位

Critical（正确性/安全/数据丢失）/ Important（必须修）/ Minor（记台账，不阻塞）。发现与 brief 原文冲突时（plan-mandated），不自行裁决，列出并交编排者。

## 报告

写到 `.superpowers/sdd/task-rot3p-review.md`：双判决结论、逐条验收证据、独立验证结果、findings 列表（带档位）。最终消息以 APPROVED / REJECTED 开头。
