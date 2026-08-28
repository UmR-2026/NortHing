# W6-2 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w6-2-review-package.diff`（= `ebe57f2..7d53621`，3 文件 +85/-7，仅 scripts/）
- 需求：`.superpowers/sdd/w6-2-checker-semantics-brief.md`
- 授权：`.superpowers/sdd/w6-d1-checker-semantics-adjudication.md`（APPROVE-FIX + 3 附带条件）
- 实现者报告：`.superpowers/sdd/w6-2-checker-semantics-report.md`

## 编排者已磁盘复核（发现矛盾必须指出）

1. 复跑 `node scripts/verify-rot-budget.mjs` = **passed, exit 0**（469/937/388/69/106 全绿）。
2. json diff 中 5 处 `"ceiling"` 均为上下文行（数值零改动）。

## judge 重点

1. **语义正确性**：`collectRustFiles` 新排除逻辑——`tests.rs` 精确文件名匹配（不误伤 `contests.rs` 之类）、`*_tests` 目录段匹配（不误伤 `latest_results` 之类中间段）；既有排除（`tests` 段、`_tests.rs`、target、node_modules）未回归。逐行读 diff 里的实现。
2. **自测用例有效性**：新增 2 用例是否真的断言排除行为（不是恒真测试）；11/11 输出与用例数对得上。
3. **附带条件 3 条全落地**：note 追记（5 条 grep 规则）、自测用例、commit message 含 `checker-semantics-rebase`。
4. **反规避面**：新排除是否引入把生产代码藏进 `tests.rs`/`*_tests/` 就免计数的洞——该风险仲裁书已评估，你独立复核其结论是否成立。
5. **预期读数偏差**：unwrap 469 vs brief 预估 473，实现者解释为 W6-1 删测试连带 −4——核对 W6-1 diff（`11a4e5e`）中 keyring.rs 删除块是否含 4 处 `.unwrap()`。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过（SPEC 对照 brief §Spec 5 条逐条 file:line 证据；QUALITY 独立判断）。防腐必查：复用核查 / 无 owner 抽象 / 预算闸（本任务触闸已持 D1 授权，核查是否越授权范围——ceiling 改动或超出排除规则+note+自测的任何动作 = SPEC FAIL）。**Cannot verify from diff** 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w6-2-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
