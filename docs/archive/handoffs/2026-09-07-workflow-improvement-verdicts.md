# Handoff — 2026-09-07 外部审查改善方案裁决 + 效率六条并入现行工作流

> freshest session state。旧篇：`2026-09-06-w16-trusted-core-w17-windows-ci.md`（同目录）；再早见 `docs/archive/handoffs/`。
> **注意：W18 正由并行 session 执行中。本 session 只写本文档 + archive 移动，未碰 scripts/、计划文件、ledger。**

## 0. 一句话状态

2026-09-07 外部独立审查（工作流＋防腐机制）的改善汇总已经编排者**证据级复核**：抽查的 6 项关键事实全部成立；P01–P10 按三档落地；探索项只采纳 E02＋E05 半张决策卡；效率改善六条全部判定可用，即日起作为编排者工作规则生效（§3）。summary 的 D01–D08 决策表仍待用户填写。

## 1. 需求基线

- 审查材料：`E:\agent-project\.opencode\external-review\2026-09-07\`——主文档 `workflow-improvement-summary-for-review-2026-09-07.md`（REVIEW_DRAFT），原报告 `independent-review-2026-09-07.md`，可复现 probe `independent-review-probes.mjs` 及 evidence JSON 若干。
- 源码基线：`df5c1ce`（复核时与 origin/main 一致）。W18 计划 `.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md`（审查时为未跟踪文件）。
- **生效约束（summary 原文，继续有效）**：批准某条原则 ≠ 批准某次涨额、某份 lease 或未列出的代码改动；theme.rs ceiling 不升、禁复活 allow-god-file 头注释、meta 双 judge＋用户拍板全部保留。

## 2. 编排者复核结论（本 session 新增，均已亲自抽查验证）

事实核验（非转述）：rename 漏检成立（`verify-task-gate.mjs:232` 用 `git diff --name-only` 无 `--no-renames`）；豁免路径笔误成立（`northhing-installer` 双 h 不存在，`northing-installer` 存在）；i18n-audit.mjs CRCRLF 成立（checker `countLines` 实测 3834，文档写 3833，±1 尾行差异不改结论）；W18 计划引文（css=790 / selectors=827 / epoch=69 / lease schema / deadSince）逐条对得上。

分档裁决：

- **档 A（廉价，应立即并入 W18 各单合同）**：rename 修复（`--name-status -z` 两端检查）；豁免路径修正（**须显式记录口径修正，不得称"原样搬运"**）；入口文档同步（P06：AGENTS 与 anti-rot skill 头注释矛盾对齐）；阈值边界测试 800/801、1000/1001＋日期边界（mutation 证据：现有 12 测试钉不住边界）；退役数量合同消歧（44−1+5=48 vs 一增一退，**二选一须用户拍板**）。
- **档 B（只取最小版，完整版留下波）**：P02 只接"CI 用 merge-base 计算 BASE＋审查绑冻结 SHA"，brief_hash 全绑定是对抗场景镀金，暂不做；P03 最小版＝按任务类型在 policy 文件列 required 材料清单；P04/P05 先定义"实质放宽"分级标准，再动 schema，否则 W18-1 反复返工。
- **档 C（不属本波）**：P07 只守"同工作树单写者＋未知归属残留不删不还原"，不建并发隔离平台。
- **E 项**：E02 并入 W18-7 fixture 工作（旧 vs 新规则跑同批历史 fixture，列新增拒绝/解除拒绝，不单独立项建模拟器）；E05 只立决策卡模板；E01 暂缓观察一波（防变成第二套台账）；E03/E04 不进队列。

## 3. 效率六条 → 现行工作流（2026-09-07 起生效）

1. **审查包机器生成**：修现有 `assemble-review-pkgs.ps1`（顺手并掉 P03 完整性校验＋首页导航），不新建第二套打包工具。摘要不替代原始材料，judge 保留读完整证据权。
2. **修复轮增量审**：reviewer 输入＝上轮未关闭问题清单＋修复 diff，不整单重读；需求/范围/公共接口/判据变更 → 回全量审；最终批准绑新 TIP。落点＝review-package 组装逻辑＋reviewer 派发正文。
3. **证据复用**：同快照/同命令/同工具链/同相关环境的有效结果可复用，输入一变即失效；不只凭"实现者说跑过"，独立抽查保留（校准铁律不可让渡）。
4. **流水线准备**：当前单执行时可预起草下一单 brief、整理 fixture。**钉死：预起草的验证命令是在旧 BASE 验的，冻结派发前必须在新 BASE 上重跑**（brief 预检铁律不变，防 W15-1g 复发）。
5. **重试分类**：网络故障（有限重试）／缺上下文（补料重派）／推理失败（升档或缩任务）三分——BOOTSTRAP 静默失败 SOP 的细化，同因反复失败不得无限等同。
6. **用户决策卡模板**：改什么／为什么／上限与期限／拒绝后果／judge 分歧，证据放链接；可集中展示，但集中展示 ≠ 合并授权，逐项批准与双 judge 保留。
- **耗时账（最轻版）**：ledger 行加两个时间戳（派发时刻／审查通过时刻）＋返工轮数；不建面板不建库；需要人工维护第二套账本即砍。
- **"暂不建议"四条全部背书**：W18 七单禁并行（同 checker 文件集必相交＋P07 单写者，非"暂缓"是现行规则禁止）；不按单价换便宜模型（回合数实证反例）；抽查不可删；不优化秒级 Node 测试。

## 4. 待用户拍板（D 表推荐填法）

D01–D04、D06：采纳（执行范围按档 A 收窄）。D05：采纳原则、不建平台。D07：**采纳，W18 开工硬阻塞**（schema 分阶段衔接＋超千行文件准入顺序）。D08：修改后采纳（仅 E02 并入 W18-7＋E05 决策卡）。
另需单独一拍：W18 退役数量合同二选一（§2 档 A 末条）。

## 5. 与并行 W18 session 的交接点

- 档 A 五条须注入 W18 对应单的 brief（rename→gate 相关单；豁免路径→W18-4；边界 fixture→W18-7；文档同步→W18-6 或独立小单）。
- **W18-5 扩围前置**：`i18n-audit.mjs`（checker 口径 3834 行）与 `i18n-contract.test.mjs`（1042 行）的超千行准入须先定 lease/拆分/类型豁免——W18-3 先于 W18-5 的顺序不能反；不能临时缩范围制造绿色。
- 档 A 全并入会使 W18 实际膨胀至约十单合同量；替代方案＝作为各单 brief 补充条款逐单带入，不单独立项。
- CRCRLF 口径：以 checker `countLines` 为现行闸口径；只修换行产生的下降不得报告为结构改善（P09）。

## 6. 子代理运维

本会话无变更。沿用：coder＝`gemini-38-flash-agy`；judge＝`minimax-m3`；brief review＋波级终审＝`reviewer-53`；meta-ratchet 车道＝双 judge＋用户拍板。

## 7. Suggested skills

- W18 各单派发：`subagent-driven-development`；brief 前置阅读写 `anti-rot-system`（钉死语：不因此扩展任务范围）
- 修现有组包器/review-package：`verification-before-completion`
- 本会话同类收口：`handoff` skill
