# Handoff — 2026-09-21 深夜②：W24/W25 已推送全绿；W26 五单 brief 就绪待派发（token_rhythm 额度中断）

> freshest session state。旧篇：`2026-09-21-w25-complete-w26-next.md`（同目录）；再早见 `docs/archive/handoffs/`。

## 0. 一句话状态

W24+W25 两波**已推送且 CI 全绿**（W25 推送 run `35622027140` success @ 9222bbc；终审 concern 全关闭）。**W26 产品波规划完成**：5 份 brief 已落盘并 commit（BASE=`9222bbc42c2b0a13961dd8e63686c3c804abe698`），其中 4 份经 reviewer-53 复审且修复已按指引落实（复审结论均为「修完即 APPROVE」）；**W26-5 brief 未复审**——reviewer-53 派发时 token_rhythm 402 Payment Required（用户确认 5.3 额度用完）。**未派发任何 implementer，工作树无 WIP**。

## 1. 下次 session 第一事（按序）

1. 派 W26-5 brief 复审：token_rhythm 恢复用 reviewer-53；**未恢复则按 BOOTSTRAP 预案回落 `gemini-38-flash`**（brief 复审位）。复审要点已备：设计风险（哨兵方案/fail-closed/分层合规/runtime-ports 新模块过闸）、预检事实抽查、范围泄漏。
2. W26-5 brief 复审过后 → **五路全并行派发** implementer（主力 `gemini-38-flash-agy`；同模型五开，文件集不相交矩阵已在各 brief §8 钉死；同工作树 cargo 锁互踩可容忍）。全部单 judge 车道（非 meta-ratchet）。
3. 台账翻转（编排者收口，家规 2）：P2-5 / P2-18 / P2-2 三条在同一波收口 commit 翻转（W26-1 无台账项；P1-8 只加注记不翻转——部分修复）。三份 brief 的 report 会给建议翻转文案。
4. 波级终审（reviewer-53 主位，53 不在则 gemini-38-flash 补位）→ 推送（需用户授权）→ 盯首个 CI run。

## 2. W26 五单概览（brief 路径 + 复审状态 + 关键设计钉）

| 单 | brief | 复审 | 核心 |
|---|---|---|---|
| W26-1 | `.superpowers/sdd/w26-1-brief.md` | ✅ REVISE→已修（CLI 文案改英文——CLI 界面全英文实证；agentic.rs:296 过时注释纳入允许集单行更新；turn_id 门控；行号订正） | UserSteeringInjected → Banner(Info, display_content)；desktop 零改动复用 app.rs:216-220；CLI chat/exec 两臂；其余 8 事件维持静默 |
| W26-2 | `w26-2-brief.md` | ✅ REVISE→已修（**Critical：允许集补 distill.rs + session_usage/service.rs 两处全字段 literal 机械波及**；alias="error_detail"；增 services-core 测试命令；测试落点钉死 save.rs 内联） | DialogTurnData + `error_detail: Option<AiErrorDetail>`（复用既有类型）；fail_dialog_turn 落盘；save.rs 重建合成错误消息；双 surface 零改动（共享 core 重建路径） |
| W26-3 | `w26-3-brief.md` | ✅ REVISE→已修（台账翻转归属补上；user32/kernel32 归属订正；测试 mutex 名各自独立） | single_instance.rs 新文件；手写 extern CreateMutexW（零新 crate，W24 删依赖教训）；`Local\NorthHingDesktopSingleton`；main.rs :63/:67 间检查点；非 Windows stub |
| W26-4 | `w26-4-brief.md` | ✅ REVISE→已修（净减区间订正为 -3~-5，达标线=AC3 <836；rollback_registration 有 :139 活调用方不删） | uninstall_plugin 预留标注（reason: 先例形态）；stop_server 两处死分支清理；**836/836 顶格必须净减** |
| W26-5 | `w26-5-brief.md` | ⏸ **未复审（402）** | CredentialStore port（runtime-ports/credentials.rs + NullCredentialStore + `__kr_mcp_auth__` 哨兵 + 账号键函数）；services-integrations set/clear 写哨兵；core runtime_server_config 挂点 A 解析；desktop sync.rs / cli main.rs 注册（**desktop main.rs 让给 W26-3 避冲突**）；存量迁移不做；UI 入口不接（现零生产调用方） |

## 3. 盘面

- origin/main = 本地 = `9222bbc`，工作树干净（W26 briefs 已 commit，无 WIP、无 stash、无在跑子代理）。
- CI 基线 = run `35622027140`（9222bbc 全绿，含 W25 的三档 verdict/配额授权路径首跑实证）。
- 余量：task-gate 13 行（834/847）/ checker 993/1050 / scripts 45/48 / sdd ~140/400。
- defer 批（原样）：checker 加严批已闭环（W25-3）；剩 (a) ceiling-raise 通道形状收紧 + (c) 多 warning 边界用例（deadline 2026-10-15，下波触 checker 时收口）；(e) ci:file:job 投机分支删除（下次触 check-github-config 时）。

## 4. 子代理运维（本段新增）

- **token_rhythm（glm-5.3）额度 2026-09-21 深夜耗尽**（W26-5 brief 复审派发 402）。judge-53/reviewer-53 停派；brief 复审/波级终审按 BOOTSTRAP 预案回落 `gemini-38-flash`；任务审查位 m3 仍在（minimax 额度正常）。
- **kimi 禁派子代理**（用户 2026-09-21 重申：额度不够）：内置 `explore`/`general` 会静默继承编排者本体模型——本体若是 kimi，内置 agent 即 kimi。侦察任务派 `gemini-38-flash-agy`（本波三段侦察实证好用）。
- 派发正文一律绝对路径（W25-4 judge 相对路径踩父仓事故）。

## 5. 待决与卡点

- 无推送遗留（W24/W25 已推）。
- 缓办在案（不变）：D2 汇率 / D4 速率档 / E01 / P2-17 合并（frozen）/ K4b / K3 慢线。
- W26 之后：W27 未规划（plan 只到 W26 + K3 慢线）。

## 6. Suggested skills

- 续 W26：`subagent-driven-development`（五路全并行）；brief 已齐无需再写。
- 收口：`handoff`；CI 盯梢 `ci_status`。
