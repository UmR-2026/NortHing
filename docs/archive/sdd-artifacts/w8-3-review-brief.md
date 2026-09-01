# W8-3 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w8-3-review-package.diff`（单 commit `53e70dc`，6 文件 +130/-64）
- 需求：`.superpowers/sdd/w8-3-selectors-dedup-brief.md`
- 病灶：`.superpowers/sdd/deep-rot-onboarding-selectors.md` §二（三处复制带 file:line）
- 实现者报告：`.superpowers/sdd/w8-3-selectors-dedup-report.md`

## judge 重点核查项

1. **三处复制的消除真实性**：①ModelItem 映射——`ui/model_selector.rs` 新增 `from_config`（+45 行）与旧两处逐字段等价（含 `.filter(enabled)` 语义）；②time-ago——`ui/session_selector.rs` 新共享函数（+56 行）与旧两处四档阈值/文案逐字符等价；③custom_headers——文件内 helper 化后 save/update 两臂等价。
2. **归属合理性**：新 helper 落在 ui/model_selector.rs / ui/session_selector.rs（而非 selectors.rs 或新 util 文件）——消费方是 selectors.rs + chat/model.rs、chat/session.rs，确认归属选择有真实理由（ModelItem/time-ago 的类型 owner 在哪）。
3. **行为零变化**：重点核对 time-ago 四档边界值（秒数阈值）与 ModelItem 字段顺序/过滤条件。
4. **测试有效性**：+3 新测（from_config / four_tiers / recent）非恒真，覆盖档位边界。
5. **ceiling 875→861** 与实测一致；json 仅此一处变更。
6. Spec 与 Global Constraints 逐条。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。防腐必查：复用核查（本任务就是消重复——确认没造出"第三个副本"）/ 无 owner 抽象 / 预算闸（仅下调）/ god-file 观测点（selectors.rs 861/861，附健康度观察一句）。**Cannot verify from diff** 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w8-3-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
