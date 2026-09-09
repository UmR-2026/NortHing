# Plan — W23-W26 闭环与解耦（2026-09-10，决策全案已拍板）

> 唯一事实源：本文件 + `progress.md` 台账行。输入文档（全部已存档）：
> - `external-review/2026-09-07/`（F01-F10 + E01-E05，workflow-improvement verdicts）
> - `external-review/2026-09-09/independent-review-2026-09-09.md`（coder-qwf，1C/6I/5M，元工作流）
> - `external-review/2026-09-09/external-arch-impl-review-2026-09-09.md`（ZCode，0C/3I/4M，架构+实现）
> - `external-review/2026-09-09/anti-rot-recommendations-2026-09-09.md`（防腐六建议+四反建议）
> - `docs/architecture/agent-kernel-northstar.md`（K 线，K3 闸门与流程）

## 0. 用户拍板汇总（全部 2026-09-09 生效）

- **B 组六件**：A-2 组装器收口 / A-1 CI 接线（meta-ratchet 签字含）/ A-3 授权通道 / C1 仅修 steering UX / P2-5 字段级 error_detail / P2-2 拒启动 / P2-18 预留标注 / P2-17 标记暂缓 / P2-14 关单 / P1-8 修自己那处（CredentialStore port）。
- **防腐包 D1-D7**：D1 闸户口册 / D2 特批机器可读（汇率缓办）/ D3 防漂移三小件 / D4 红灯分级（一期现在，速率档缓办）/ D5 只记不罚 / D6 验证者先入库 / D7 注释诚实化 + 跨平台编译哨兵。
- **K3 逆转 D-8**：重启 core-agent 进程内解耦（先 owner design + judge 评审，不动代码）。
- **并行优先方针**：plan 默认多路并发；coder 5-6 路同发；串行仅限真实依赖边；文件集不相交为铁律；meta-ratchet 车道不因并行豁免。
- **反建议四条全部接受**：不建面板 / 不上签名 / 不上 AST / 不加 judge 车道与 verdict 词汇。

## 1. 波次拓扑（依赖边标注；无边 = 可并行）

```
W23 诚实化波（brief 已提交 53eeae7，待 brief 复审）
 ├─ W23-1 文档批（AGENTS.md/CN 分层表+验证表 / ledger P2-25 / cli-internal 注释）   [单 judge]
 ├─ W23-2 事件诚实化（agentic.rs 注释 + events.rs catch-all 显式化 + 测试）         [单 judge]
 ├─ W23-3 meta 小修（ci.yml 注释 + A-4 registry fail-closed + F1.7 翻转）           [双 judge]
 └─ W23-4 台账处置（P2-14 关单 + P2-17 标记暂缓，纯 ledger）                        [单 judge]
      四路文件集互不相交 → 全并行。依赖：无。

W24 闭环波（依赖 W23 完成，因同触闸文件）
 ├─ W24-1 gate-registry.json + 三方对差 checker + CI 接线（A-1 并入）+ D7 跨平台哨兵  [双 judge]
 ├─ W24-2 组装器收口（入 agent-project git + manifest 钉 allowlist/工具哈希 + maxBuffer + 流程改口） [编排者侧，无仓内闸]
 └─ W24-3 分层表单一源生成（crate-layout.mjs → AGENTS.md 表校验/生成）              [双 judge；依赖 W23-1 表已修正]
      W24-2 与其余两路仓内文件集不相交 → 三路并行。

W25 棘轮修复波（依赖 W24-1 的 registry 机制在位）
 ├─ W25-1 授权记录机器可读（rot-budget.json schema + checker 复用 isAuthorizationLive；A-3）
 └─ W25-2 verdict 三档一期（violations 计数口径，零余量警告转 advisory；workflow-policy.json rubric SSOT + selftest 同步）
      两路同触 verify-rot-budget.mjs → 串行或合单（派发前定）。

W26 产品功能波（依赖 W23/W24，与机制波文件集全不相交 → 可与 W25 并行）
 ├─ W26-1 steering UX（UserSteeringInjected → Banner，W22-1 模式复用）              [单 judge]
 ├─ W26-2 P2-5 失败留痕（DialogTurnData + error_detail 字段级方案 + 双 surface 渲染）  [单 judge]
 ├─ W26-3 P2-2 单实例拒启动（desktop main.rs，win32 mutex；v1 不做参数转发）          [单 judge]
 ├─ W26-4 P2-18 预留标注 + stop_server 死分支（lsp manager.rs，小）                  [单 judge]
 └─ W26-5 P1-8 CredentialStore port + update_remote_authorization 接线               [单 judge]
      五路文件集互不相交（events.rs/app.rs / dialog_turn.rs+渲染 / main.rs / lsp / mcp+runtime-ports）→ 全并行。

K3 慢线（独立轨道，不阻塞任何波）
 └─ K3-D owner design 文档（port 清单 + AGENTS 边界 reconciliation + 行为等价测试清单）
    依北极星 §5 前置：judge 评审通过前不动代码。架构子代理起草 → reviewer-53 评审 → 用户裁定。
```

## 2. 并行派发纪律（本规划期生效）

- 每波派发前输出「文件集不相交矩阵」一段进 handoff/台账；相交的单在同波内排序串行。
- brief 复审多路同发（53 主位；**53 余额不足/故障时回落 `gemini-38-flash`**——2026-09-09 已实证一次 Payment Required）。
- meta-ratchet 单（W23-3、W24-1、W24-3、W25 全部）双 judge 车道不变；并行不豁免。
- 同工作树 cargo 锁互踩为已知可容忍等待；implementer brief 里继续钉死验证命令带 `--features product-full`。

## 3. 每波验收口径（不变）

每单：53 brief 复审 → implementer → judge（按车道）→ 台账行立即 commit。每波末：53 波级终审（范围 = 波前 HEAD..波后 HEAD）→ 推送（需用户授权）。handoff 推送态与 `git ls-remote` 对账。

## 4. 风险与缓办清单

- 缓办：D2 偿债汇率 / D4 速率档 / E01 证据面板（反建议已确认）/ P2-17 合并（等第三调用方）/ K4b（cli+acp facade，未启动需重新评审）。
- 额度注意：task-gate 13 行（834/847）——W24-1 改 task-gate 时需拆分或用户拍板调 ceiling（brief 预留决策位）；checker 50 行（999/1050）。
- scripts 顶层配额 45/48，2026-10-15 回落 42/48 届时确认；W24-1 新增 gate-registry.json +1 文件 → 46/48。
- 已知接受副作用：W22-2 后 delete_session 触发真实 retention 清理（已声明）。
