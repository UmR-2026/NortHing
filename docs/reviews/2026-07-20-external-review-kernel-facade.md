# External Review Brief — kernel facade 冻结面评审导引（2026-07-20）

> 给外部评审 agent 的入场包。评审对象是两份**设计文档**（不是代码）：agent-kernel 北极星架构 + K1 facade 面定稿。请先读完本文件再开始；它定义了背景、三个评审靶点、已通过 4 轮内部评审的结论（**不要重复报告**），以及输出格式。

## 1. 背景（一段话）

northing 是 Rust workspace 的桌面 agent 应用（分层：contracts ← execution ← services ← adapters ← assembly ← apps）。当前 god crate `northhing-core`（assembly/core）被 4 个宿主 crate（desktop、cli、acp、cli-internal）+ 独立 workspace 的 desktop-tauri 直接依赖，core 改一行 → 全宿主重编（实测：touch 一个 leaf crate 后增量 `cargo check --workspace` = 29.86s，cold = 316.8s，core 单 crate = 116.8s）。解耦方案：**kernel 轮毂-辐条**——新建极薄 facade crate `contracts/kernel-api`（命令面+事件面），宿主只依赖它；后续再把 kernel 实现从 core 下沉。两份文档就是这套方案的设计稿，即将冻结并进入实施（K1 实施 → K2 desktop-tauri 切换）。

## 2. 评审对象

1. `docs/architecture/agent-kernel-northstar.md`（v0.3.1，北极星：目标/原则 P1-P5、现状诊断、编译收益模型、K0-K5 迁移路线、风险）
2. `E:\agent-project\.opencode\sdd\kernel\k1-facade-surface.md`（v2，facade 面定稿：9 组 trait 设计稿签名、N=44 上限 53、FROZEN 事件 schema、13 条 out-of-facade 处置、10 异常处置、§8 验收口径）

辅助输入（需要核对事实时读）：`E:\agent-project\.opencode\sdd\kernel\k1-inventory.md`（91 条宿主调用面清单）；根 `AGENTS.md`（分层与骨架不变量）；`docs/superpowers/plans/2026-07-19-frontend-rebuild-tauri.md`（F1.5/F2/F3——facade 未来面来源）。

## 3. 三个评审靶点（只评这些，不评风格）

**靶点 1 — F2/F3 未来面还有漏的吗？**
facade 的面 = 现有 91 条调用清单 ∪ 前端计划 F1.5（turn 轨迹：chat-tool 事件 + duration_ms）/ F2（settings CRUD、test_provider、onboarding、Inspector、core_health、产物面板）/ F3（panels 配置）。漏一个未来需求 = 实施后每加功能都要改 facade → 全宿主重编的新扇出（这是本方案最怕的事）。请对照 F 线计划逐条核对，找出**任何 facade 没有覆盖、但 F2/F3 明确需要的命令或事件**。

**靶点 2 — cargo 机制论证有没有反例？**
文档声称的编译收益依赖三条机制：① facade 不声明 `northhing-core` 的 `product-full` feature（防 feature unification 把 rmcp/git2/reqwest 传染回宿主）；② facade 不 re-export 泛型/derive 宏类型（防 kernel 内部类型进宿主 rlib metadata）；③ facade 薄且稳定（方法 ≤⌈N×1.2⌉、≤1500 行）。请从 cargo/rustc 实际行为角度攻击这三条：有没有场景让宿主仍然会随 kernel 内部改动重编？（泛型单态化、macro_rules 导出、inline 函数、feature 链、workspace 统一 feature 解析等）文档的验收口径（cargo tree 零命中、AST 方法计数）够不够检出违规？

**靶点 3 — 9 组 trait 粒度合理吗？**
Bootstrap=2 / Session=8 / Turn=2 / Events=3 / Settings=10 / Agents=9 / Tools=3 / Usage=3 / Platform=2 + 自由函数 1，N=44。太碎（宿主要组装 9 个 trait object）还是太粗（单组内聚不够）？分组边界有没有把改动频率差很远的 API 捆在同一组（会导致单组改动扇出不必要的重编）？有没有明显分错的（某方法放错组）？

## 4. 不要报告（内部 4 轮评审已覆盖/已裁决）

- judge-m3 两轮 + judge-lc 一轮 + 用户五条质询的全部已修复项（见文档头部版本记录）：facade 量化约束、K3 探针前置、K2 回退路径、K3 ROI 闸门、TurnStateKind 冻结、13 条 out-of-facade 处置、ToolPort 用 #[async_trait]、w4_repro 留 K4a、cli-internal 死依赖删除、workspace=true 依赖写法、MCPServerDto.location、ProviderPort 暂不经 facade（K3 闸门再议）。
- F2-conditional 的 2 个占位方法（start/stop_mcp_server）以注释形态豁免 + 三闸门的裁决（已录入 P2）。
- 任何风格/命名/文档措辞类发现。
- 代码实现问题（尚无实现，纯设计稿评审）。

## 5. 输出格式

按严重度分级（Critical / Important / Minor），每条：所属靶点（1/2/3）、文档 `file:line`、问题描述、建议修复方向。最后给一段"总体评价 + 冻结前必须修的 3 件事"（如果没有必须修的，明说"可冻结"）。
