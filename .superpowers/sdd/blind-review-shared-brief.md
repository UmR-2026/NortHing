# 盲审任务（共享）：god-file 对照组代码层深审 — 5 项清单验证轮

仓库：E:\agent-project\NortHing（main）。只读审查，不改代码不 commit。

## 任务

对你被指定的**一个** god-file 做代码层深审（目标文件见派发消息）。

## 方法（严格遵循）

1. 量规 = `E:\agent-project\NortHing\.superpowers\sdd\deep-rot-review-rubric.md` 的 8 项清单。
2. judge 纪律 = 5 项必查（复用核查 / 无 owner 抽象 / 预算闸 / 纯位移等价[无 diff 记 N/A] / 证据抽查）。
3. **证据抽查为硬格式要求**：报告必须含「证据抽查」一节，逐条列出你的每个发现断言 + 你怎么验证的（回读源码行号/rg 命令+命中数/codegraph）。数字类断言（行数/ceiling/计数）必须当次实测，禁止凭记忆。
4. **盲态纪律**：禁止搜索/引用任何既有审查报告（`.superpowers/sdd/deep-rot-*`、`blind-review-*`、`*-review.md` 等）。发现即你所见。
5. 一切发现带 file:line；不确定进"无法判定"；禁止猜。

## 输出

报告写入 `E:\agent-project\NortHing\.superpowers\sdd\blind-review-<NAME>-2026-08-29.md`（NAME = 目标文件名去掉 .rs）：8 项逐项 + 总判定（健康/稳定/腐化中）+ 证据抽查节。
返回消息只给：总判定 + 腐化证据/观察项计数 + 每条发现一行（含 file:line）。