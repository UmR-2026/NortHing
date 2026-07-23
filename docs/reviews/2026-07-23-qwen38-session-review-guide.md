# Review Guide — Qwen3.8 Pipeline Session (2026-07-23)

> 面向**独立外部审查者**。请只读审查，不要修改任何文件、不要运行会改变状态的命令。

## 你的角色

本 session 用 **qwen3.8**（`alibaba-token-plan-cn/qwen3.8-max-preview`）同时担任编排者和 subagent（coder-qw / judge-qw），完成了一批后端工作，并**自评** qwen3.8 能力"不亚于 kimi k3"。

你的任务**不是确认这些自评**，而是：独立探索、形成自己的判断、**尽力找出问题**。请保持怀疑——内部 coder 和 judge 是同一个模型，可能存在同源盲区。

## 最小背景

- 仓库：`E:\agent-project\northing`（Rust workspace + `scripts/core-boundaries/` 边界检查器）
- 本 session 7 个 commit：`git log` 从 HEAD `9d4516a` 往前到 `8ed897d`
- 自评记录：`E:\agent-project\.opencode\model-capability-notes.md`（2026-07-23 两段）
- 交接文档：`docs/handoffs/2026-07-23-qwen38-pipeline-handoff.md`
- 分诊报告：`docs/status/2026-07-23-p2-9-stage2-triage.md`
- 忽略：`docs/handoffs/2026-07-22-frontend-redesign-*.md`、`docs/plans/2026-07-22-frontend-redesign-plan.md`（用户侧未跟踪文件，与本工作无关）

## 本 session 做了什么（待你审视的 claims）

| Commit | 内容 | 自评 |
|---|---|---|
| `8ed897d` | C5c：新增 Test/Refactor 内置 subagent | judge PASS（抓 1 回归→返修） |
| `4a7c31b` | K4：dev profile `debug = "line-tables-only"` | judge PASS |
| `3a2b170` | P2-9 stage1：边界检查器 ENOENT 修复，~34 路径 remap | judge PASS |
| `abd5f0b` | P2-9 stage2：230 违规分诊 + 修 25 | judge PASS |
| `4a6a354` | P2-9 stage2b：self-test 锚点同步，解锁 112 | judge PASS |
| `d9fb971` | P2-9 stage2c：groups 4-16，再解锁 56 | judge PASS |
| `9d4516a` | handoff | — |

**核心声称**：边界违规 230 → 37，self-test 绿，无 ENOENT，所有 remap "regex 字节级守恒 / 无放宽 / 无丢弃 contract"。

## 开放探索方向（不必逐条，鼓励自己发现新问题）

### A. 边界检查器工作的真实性（最值得深挖）
- 自己跑 `node scripts/check-core-boundaries.mjs`，看**实际**违规数、有无 ENOENT。
- "无放宽/无丢弃"是核心声称。挑若干处 remap 自己核对：规则 regex 是否真原样搬迁？self-test 锚点是否真还钉住原 contract？有没有某条规则被悄悄改宽以消除违规？
- **关键陷阱**：本 session 的 judge 自己承认，self-test 只是"规则数据守恒 + 解析器正确性"测试，**不等于源码真满足合约**。那么：
  - 有没有被"修掉"的违规，其实源码并不满足（remap 指向"文件存在但符号不在其中"）？这是 m27hs 的前科模式。
  - `self-test.mjs` 锚点拆分后，保护强度是否真等价，还是覆盖悄悄变弱了？
- 剩余 37 条之外，自己抽查几条"已修复"的，grep 源码验证符号真在新落点。

### B. 分诊判断的正确性
- 读 `docs/status/2026-07-23-p2-9-stage2-triage.md`。挑几个 NEEDS-DECISION / REAL-VIOLATION，自己判断分类对不对。
- 有没有把"真实违规"误判成"陈旧规则"然后修掉、从而掩盖问题的？

### C. C5c / K4 工程质量
- C5c：Test/Refactor agent 真注册好了吗？跑相关 `cargo test`。prompt 模板（`src/crates/assembly/core/src/agentic/agents/prompts/{test,refactor}_agent.md`）内容质量如何？有无遗漏的注册点/测试？
- K4：`debug = "line-tables-only"` 合理吗？有无被忽略的副作用？

### D. qwen3.8 能力声称的可信度（元审查）
- 读 `model-capability-notes.md` 的 2026-07-23 段。那些"✅实证可靠"被本 session 产物支持吗，还是过誉？
- 本 session 任务**全是机械 + 强验证类型**（remap、分诊、精确编辑）。这个样本能支撑"不亚于 k3"吗？哪些能力维度（开放式设计、超大实现、架构判断、多轮调试）**根本没被测到**？
- coder-qw 和 judge-qw 同源（都是 qwen3.8）——有没有系统性互相漏掉同类错误的风险？你怎么验证这一点？

### E. 自由探索
- 任何可疑处：`git show <commit>` 看每个改动的实际内容；文档与代码一致性；有没有改动越界（声称只改 X 实际改了 Y）。

## 输出建议

1. **独立裁决**：这批工作整体质量如何？qwen3.8 的能力声称站得住吗？
2. **具体问题**：带 file:line / 命令输出证据，按严重度排序。
3. **过誉/遗漏**：本 session 自评里你认为过誉或刻意无意遗漏的地方。
4. **qwen3.8 vs k3**：基于你能看到的证据，你的独立看法（注意 k3 在本仓库从未当过 subagent，无直接对照数据——请说明这个限制如何影响你的判断）。

## 可用命令（只读）
- `git log / git show / git diff`（只读）
- `node scripts/check-core-boundaries.mjs`、`$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs`
- `cargo check / cargo test`（编译/测试，不改源码）
- `rg`（ripgrep）grep 源码取证
