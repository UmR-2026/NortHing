# W8-4 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w8-4-review-package.diff`（单 commit `7e42a65`，7 文件 +237/-164）
- 需求：`.superpowers/sdd/w8-4-app-extract-brief.md`
- 病灶：`.superpowers/sdd/deep-rot-app-input.md` §1
- 实现者报告：`.superpowers/sdd/w8-4-app-extract-report.md`

## 编排者已磁盘核实（矛盾必指出）

1. app.rs 实测 805 行；ceiling 962→805（下调，合规）。
2. **深审报告 §1.2 是幻觉**：`close_all_popups`/`navigate_back`/`PopupType` 在 desktop 全仓零命中；真身在 CLI `input/key_popups.rs`（W8-1 拆分后）。brief §3 popup 去重任务建立在错误证据上，实现者未执行 = **正确处置**（且未擅自扩 scope 到 CLI，守住了分层约束）。judge 无需审 §3，但请核实"desktop 确实无 popup 映射重复"。
3. 实现者报告会话开始时 app.rs 工作树破损（前任实现残留 stray `}`），点名 `git restore` 恢复后重做——**重点核对最终 diff 的连贯性**：无重复函数、无残留碎片、无半截位移。
4. 前任 = 两次 Gemini 渠道证书错误派发（疑第一单做了半截编辑后断线）——若 diff 中有无法用 brief 解释的改动，可能就是残留，标记出来。

## judge 重点核查项

1. **抽离纯位移**：color.rs（134 行，含测试）与 window_ops.rs（91 行，unsafe FFI）内容与原 app.rs 对应段逐字符等价（允许 mod/use 适配）；unsafe 块零改动。
2. **entry.rs 2 行改动**：理由（应是 window_ops 路径适配），无越界逻辑。
3. **onboarding 路径修复**：默认值空串 + placeholder；step_gate 对空串的行为（应阻止推进而非通过）；无引入新配置项。
4. **§4 warn 日志**：英文、带上下文、语义为 best-effort 标注。
5. **边界测试**：color.rs 新增 4 边界测试非恒真。
6. **warnings 44 = 基线**；3 个 unused import 清理无副作用。
7. Spec 7 条 + Global Constraints 逐条。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸（仅下调）/ god-file 观测点（app.rs 959→805，附健康度观察一句——这是观测实验的重要数据点）。**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w8-4-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
