# W9-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w9-1-review-package.diff`（= `151f77c..d742e75`，两 commit：`921c09d` 抽离 + `d742e75` 第三档，4 文件 +269/-90）
- 需求：`.superpowers/sdd/w9-1-approval-third-tier-brief.md` + 编排者钉死裁定（桌面侧会话级允许集，见下）
- 实现者报告：`.superpowers/sdd/w9-1-approval-third-tier-report.md`（两任实现者：step-explore 做抽离+半成第三档，minimax-m3 接续完成）

## 编排者钉死裁定（SPEC 的一部分）

facade 第三参实测为 reason 文本、无 remember 语义（step-explore 发现并正确 NEEDS_CONTEXT）。裁定：桌面侧内存态 `HashSet<工具名>` 允许集——「本会话内允许 <工具名>」= 入集 + 批准当前卡；后续 AwaitingConfirmation 命中 → 自动批准 + 落"已自动允许（本会话）"resolved 可见记录；重启清空。不动 core。

## 编排者已核实（矛盾必指出）

1. 截图（HTML mockup）已视觉验收：三按钮横排、文案含工具名、风险行在。**mockup ≠ 运行时**——真机行为列入实测清单第 10 项。
2. app.rs 760→792（+32），<800 无 manifest 条目属正常；rot 实测绿；115/115 测试（+2 允许集纯逻辑）。
3. 前任 step-explore 的半成品由 minimax-m3 接续完成，最终 commit 单一（d742e75）覆盖第三档全部。

## judge 重点核查项

1. **自动放行路径（最高优先）**：AwaitingConfirmation 事件 → 允许集命中 → 自动 respond 的链路逐行走查——事件工具名字段取值是否正确（ToolCall 事件的 name 字段，以代码为准）；自动批准失败时是否有兜底（不能静默吞）；自动放行记录是否用户可见。
2. **允许集生命周期**：重启清空（内存态）核实；会话切换时是否清空或隔离（裁定说"重启即失效"——会话切换语义由实现者选，核实其选择与注释一致）。
3. **三按钮语义**：批准=只批当前；本会话允许=入集+批当前；拒绝=reason 传 None 还是空串（facade 第三参 reason——拒绝时是否该传理由文本，评估当前选择合理性）。
4. **抽离纯位移核对**（921c09d）：approval_card.rs 原 79 行 vs 原 app.rs 分支——纯位移。
5. **测试非恒真**：2 个新测试读断言逻辑。
6. **中途断线残留排查**：d742e75 diff 中是否有无法用需求解释的改动（前任断线残留风险）。
7. Spec + Constraints 逐条；偏离 3 条（后端旁路裁定/mockup 截图/缩进残留）复核其处置合理性。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸（manifest 仅删条目无上调）/ god-file 观测点（app.rs 792 <800 脱离登记，附一句观察）。**阻塞性数字断言必须磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w9-1-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
