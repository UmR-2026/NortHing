# Qwen3.8 Pipeline Handoff — 2026-07-23

> Session ends at HEAD `d9fb971`. 本 session 用 qwen3.8（与编排者同模型）subagent（coder-qw/judge-qw）跑后端 A/B 管线并做能力实证。6 commits，三连轮零返修。

## 1. 需求基线（未变）

- Product: hidden-IDE 通用 agent（AGENTS.md v0.1.0 骨干不变量）；面: Slint desktop + installer，余冻结-experimental；人类全免确认；完成判据 e2e chat 绿 + computer-use 五步冒烟。
- 工作模式: 后端全 subagent 管线。`docs/handoffs/2026-07-22-frontend-redesign-discussion.md` + `docs/plans/2026-07-22-frontend-redesign-plan.md` 是用户侧前端工作，**未跟踪，不要碰**。

## 2. 今日已完成（6 commits，全 judge-qw PASS）

| Commit | 内容 | 状态 |
|---|---|---|
| `8ed897d` | C5c: Test/Refactor builtin subagent（三处注册 + 契约测试索引位移 + prompt 模板） | judge PASS（抓 1 关联回归→返修→绿） |
| `3a2b170` | P2-9 stage1: checker ENOENT 修复，~34 路径 remap | judge PASS（25+ remap 符号取证；self-test 同步返修） |
| `4a7c31b` | K4: dev profile `debug = "line-tables-only"` | judge PASS（一次过） |
| `abd5f0b` | P2-9 stage2: 分诊 230 条 + 修 25 陈旧规则 | judge PASS（一次过） |
| `4a6a354` | P2-9 stage2b: self-test 锚点同步，解锁 112（runtime-ports 79 + task_execution 20 + runtime 13） | judge PASS（一次过，multiset 字节级守恒证明） |
| `d9fb971` | P2-9 stage2c: groups 4-16 再解锁 56 | judge PASS（一次过） |

基线: `northhing-core` lib 1057 passed / 3 failed（subagent_ports 环境敏感家族 P2-7，数量随运行波动）。boundary checker: **230 → 37 违规**（无 ENOENT），self-test 绿。

## 3. qwen3.8 能力实证结论（本 session 关键产出）

- **coder-qw ✅ 中大型可靠**：C5c 一次成型（主动补任务书遗漏的 agents/mod.rs 再导出）；P2-9 1362 插大 remap 无 ENOENT 无造假；stage2b/2c 编辑 self-test 护栏文件 + multiset 字节级守恒证明；红线纪律稳（绿边界主动停 PARTIAL、不 game）。
- **judge-qw ✅ 深度验收可靠**：抓到 coder 漏的 prompt_stability 关联回归；remap 逐条符号取证（25+/28/35 抽样）；独立 multiset 复核 regex/contract 守恒；定位 self-test 根因给修复方案。
- **选派策略**：qw 升为 coder 中大型 / judge 深度验收首选之一（与 lc/m3 同级）；额度独立于 kimi，可作 k3 额度紧张时主力派发池常设。详见 `E:\agent-project\.opencode\model-capability-notes.md`。
- **机制**：coder-qw/judge-qw 变体由 `.opencode/gen-agent-variants.py`（REGISTRY 已加 qw）生成，**需重启 opencode 才注册**；重启前内置 general/explore 继承会话模型亦为 qwen3.8。

## 4. 进行中 / 卡点

| 项 | 状态 | 接续点 |
|---|---|---|
| P2-9 剩余 37 | 已登记 | 10 陈旧 regex 修正（full-path impl→short-name、pub→pub(crate) 等，regex 修正非路径 repoint）+ 7 需源码核实（turn_submit remote queue policy 测试×5 + catalog `get_global_tool_registry`×2，判迁移 vs 回归）+ 13 需架构决定（crate 布局 relay-core/agent-dispatch/test-support/cli-internal、desktop-tauri product-full、optional deps、northhing-core default feature）+ stage3 接 CI |
| C4 Phase 0 手工实证 | 缺（需用户在场） | 设计稿 `docs/superpowers/specs/2026-07-22-c4-phase0-judge-gate-design.md` §10 |
| git 未推送 | 本地 main 领先 origin ~156 commits | 等用户决定 |

## 5. 队列

| 序 | 单 | 优先级 | 备注 |
|---|---|---|---|
| 1 | P2-9 残余：10 regex 修正 + 7 源码核实（先核实再改规则） | 中 | 机械 + 核实，可直接派 coder-qw |
| 2 | C4 正篇（episode 聚合 + 候选生成，设计稿先行） | 高 | 需用户实证或先起设计稿 |
| 3 | C6 并行白名单 / C7 上下文懒加载（设计稿先行 + judge 设计评审） | 中 | |
| 4 | P2-9 stage3 接 CI（backlog 清完后） | 中 | 依赖残余清理 |
| 5 | 13 架构决定 | — | 需人/架构层决定 |

设计稿起手惯例：`docs/superpowers/specs/2026-07-XX-<topic>-design.md` → judge 设计评审 → 用户拍板 → 拆实施 → judge 代码评审 → commit。

## 6. 已知雷区（不要重蹈）

1. PowerShell `Set-Content` 等写非 ASCII 会 GBK 双重编码——新 session 写非 ASCII 一律 `edit` 工具。
2. **boundary-checker self-test 是"规则数据守恒 + 解析器正确性"测试，非逐源码核验**；源码满足度由主 checker 违规数追踪。勿把 self-test 绿等同于"所有 contract 在源码满足"（本 session judge-qw 新洞察）。
3. m27hs 永久降级 ≤2 文件机械单，禁大尺度 remap（造假前科）；qw 无此问题。
4. coder-qw/judge-qw 需重启注册；重启前用内置 general/explore（继承 qwen3.8）。
5. frontend-redesign-* 两文件：用户侧，不碰。
6. 宵禁 03:00（家规#5）。本 session 02:47 收满。

## 7. 一句话状态

今日 6 commits（C5c + K4 + P2-9 stage1/2/2b/2c），boundary checker 230→37 且 self-test 绿；qwen3.8 coder/judge 双角色三连轮可靠、可常设主力派发池；P2-9 余 37（10 regex + 7 核实 + 13 决定）+ stage3，C4 正篇 / C6 / C7 待起。
