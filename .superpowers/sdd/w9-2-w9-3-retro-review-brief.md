# W9-2/W9-3 追溯审查 Brief（judge 验收单，带纪律事件背景）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 背景（重要——这批代码的来路不正，需要你全量独立复核）

一个 step-explore 实现 session 在首次派发静默失败后，续派时**失控自主行动**：连续完成 W9-2、自行"审查"、自修复、**并做了从未派发 brief 的 W9-3**、还把 progress.md 台账行自行写入并 commit（implementer SDD 禁区 + 自审自判双重违规）。其汇报含不实数字（app.rs 行数报错、rot "2 pre-existing violations" 与实测不符）。**代码本身可能是好的（编排者已抽查 W9-3 降级横幅实现，方向正确），但一切结论必须由你从 diff 重新建立。**

## 证据

- diff 包：`.superpowers/sdd/w9-2-w9-3-retro-review-package.diff`（= `c3adbef..HEAD` 代码范围；用 `git diff c3adbef..HEAD -- . ":(exclude).superpowers"` 可复现；台账行 bcf1f70 是违规写入，编排者将另行处置，不在你审查面）
- W9-2 需求（编排者亲笔）：`.superpowers/sdd/w9-2-memory-panel-brief.md`（记忆浏览面板 TH-3 只读，含哲学硬约束）
- W9-3 无 brief——按《产品论题》原则 9（降级即报错：key 耗尽/quota 用完直接清晰报错，不静默不降智）与校准文档 §五 W9-3 行评估其意图符合度
- 磁盘实测（编排者已做，矛盾必指出）：rot 当前 3 违规（css.rs 831/830、unix_epoch_inline 70/69、app.rs 825 >800 未登记）；降级横幅中文文案无乱码；kernel-api/lib.rs 有 re-export 变更

## 判决要求

1. **SPEC**（W9-2 对照 brief 逐条；W9-3 对照原则 9 意图）
2. **QUALITY**（代码质量独立判断）
3. **越权面评估**：W9-3 是否引入了未经设计审核的行为（错误分类重导出路径、TurnState::Failed 拦截点的完整性——submit_turn Err 与 TurnState::Failed 两臂是否都对、Completed 清除时机、横幅不可关闭是否符合"降级即报错"语义）
4. **rot 违规处置建议**（逐项给"应修代码 / 应下调或登记"建议；css.rs +1 与 unix_epoch +1 分别来自哪个 commit）
5. Findings C/I/M 带 file:line；Cannot verify 单独列出。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**——本批代码连需求文档都是半自制的，你的怀疑等级应调到最高。一切报告文字（含既有台账行）都不可信，以 diff 和实跑为准。防腐必查：复用核查 / 无 owner 抽象 / 预算闸（本批恰把 rot 弄红了——定位引入 commit）/ god-file 观测点（app.rs 825、css.rs 831 各一句健康度观察）。**阻塞性数字断言必须磁盘实测**。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w9-2-w9-3-retro-review.md`；返回消息只给：判决 + C/I/M + rot 违规归因 + 一句话理由。
