# Handoff：前端重设计探讨收官 → Phase 0/1 待开工

> 2026-07-22。主题：desktop-tauri 前端重设计（咨询室设计哲学落地）的用户 × 编排者逐题探讨**已全部完结**，下一步是 Phase 0/1 派遣施工。
> **唯一事实源**：`docs/plans/2026-07-22-frontend-redesign-plan.md`（本 handoff 不重复其内容，只给状态与入口）。

---

## 1. 需求基线状态

- **前端实施以 `2026-07-22-frontend-redesign-plan.md` 为准**，它取代三轨计划 Track A（`2026-07-21-three-track-refinement-plan.md` v0.2.2 #5 的"A 线暂缓"解除，16 单对账见计划 §2）。
- 设计输入（QoderWork 产出，Phase 0 需复制入仓 `docs/design/2026-07-22-frontend-redesign/`）：
  - `C:\Users\UmR\.qoderworkcn\workspace\mrs1mi22y3u2cmta\outputs\` 下的 `northing-design-philosophy.md`、`northing-frontend-design-handoff.md`、`northing-home-v1-final.html`、`northing-ui-expanded.html`、`northing-self-cognition-chronicle.html`
  - 注意：`northing-ui-expanded.html` 已重解读为**暗色模式**展开态（其 mic/旧 deck/`--drive-*`/weight 300 仍属过时，以交付规范为准）。
- 三轨计划 B/C 线状态不受影响，仍以三轨计划 v0.2.4 状态表为准（B5 revert 转设计先行、C4 待 judge 门禁设计）。

## 2. 本轮探讨已完成（无代码改动、无 commit）

七组拍板全部写入计划文档对应章节：

| 决议 | 位置 |
|---|---|
| A 线 16 单对账（作废 2/并入 8/抢救 2/独立 2/转决策 1/backlog 1） | §2 |
| 会话管理=单活跃流+档案馆（只读 v1、手动+自动封存 7 天、跨工作区分组、先标题后全文） | §3 |
| 代表色 HSL 模型 + 五取向定稿（行动/好奇/秩序/深思/温和，首次结晶=行动珊瑚） | §4 |
| 拍板表：双色模式等亮度重映射、出生灰白、临时代号、空态极简、deck 控制行虚实 | §5 |
| 派生后端票据 B9-B14 | §6 |
| Phase 0-4 分期 + 风险表 | §7-§8 |
| 展开态补充（Phase 3 时机、模块虚实、右栏底色随模式） | §10 |
| 设置页结构（通用 P1/档案馆+模型服务 P2/技能 P3/不做 MCP） | §11 |
| 异常态（LLM 错误=输入框上方悬浮卡：状态码+简写+详情+分流操作；流内只留轻标记） | §12 |

**仓库未提交产物（2 个 untracked）**：
- `docs/plans/2026-07-22-frontend-redesign-plan.md`（本轮产出）
- `docs/superpowers/specs/2026-07-22-c4-phase0-judge-gate-design.md`（另一工作线 C4 设计稿，非本轮产出，勿误删）

HEAD = `77fb7a0`（2026-07-22 checkpoint，W3a+1 scheduler 测试已落）。

## 3. 进行中卡点

无。探讨收官，工作树干净（除上述两文档）。下次会话直接进派遣。

## 4. 队列（blocking 边 + 并行可行性）

**第一批（可立即派）**：
1. **Phase 0**（半天，纯搬运，编排者可直接做或派轻单）：
   - 设计四件套+expanded mockup 复制入仓 `docs/design/2026-07-22-frontend-redesign/`
   - 字体到位：Fraunces 可变轴（WONK/SOFT）+ Noto Sans SC CJK 子集（常用 3500 字 woff2，注意 installer 体积 R1）+ JetBrains Mono，自托管
2. **Phase 1**（纯前端，顺序施工，阻塞 Phase 2）：
   - P1.1 `tokens.css` 双色模式体系（OKLCH；基底=锚点亮度灰阶×2；rep/abyss 等亮度色阶×2；`[data-theme]` 切换），旧 `app.css` 退役
   - P1.2 组件重构：topbar（名片+sess-tag+品牌水印）/对话流（活跃轮竖线/思考块/工具 chip 暖→冷/turn-meta）/操控台（发送键变形+控制行：＋存根/目录只读/模型只读/自治只读/**无思考强度**/无 ctx 环→P2.2）
   - P1.3 空态出生态+懒建流转（A8）；P1.4 sess-tag 菜单（重命名/封存）；P1.5 设置·通用（临时代号+显示模式）
   - 验收：`pnpm --dir src/apps/desktop-tauri/ui run type-check` 0 error + 用户视觉走查对 `northing-home-v1-final.html`
   - 注意：Phase 1-2 **不做把手/展开态**（用户拍板，展开归 Phase 3）
3. **后端 B9/B10/B14**（与 Phase 1 并行可行，不同代码面）：
   - B9 facade `archive_session` + Standard 过滤 ← 阻塞 P2.3 档案馆
   - B10 facade 跨 workspace 会话枚举 ← 阻塞 P2.3
   - B14 desktop-tauri commands 透出 provider/模型 + 技能面（纯转发） ← 阻塞 P2.3b 模型服务页
   - B12 facade 透出 context window ← 阻塞 P2.2 ctx 圆环（不阻塞第一批）
   - B11 全文检索、B13 agent identity 存储 ← 后续

**依赖红线**：P2.3←B9/B10；P2.2←B12；P3.x←B13+C4；Phase 3 未到前界面保持出生灰白（回退预案：`--rep-*` 切珊瑚，单变量）。

## 5. subagent 运维变更

- 本轮无 coder/judge 派遣、无模型台账/skill/MCP 变更。
- 下次派遣纪律（三轨计划既定）：coder 默认 **m27hs** + 处方级任务书（明写"必须 commit、禁止 git restore 非你创建文件"）；judge 验收 **m3**；coder 任务期间编排者不留未提交改动（先把 §2 两份 untracked 文档处理掉）。
- 派遣前建议先把 `2026-07-22-frontend-redesign-plan.md` 提交入库，任务书按路径引用。

## 6. Suggested skills（下次会话）

- `subagent-driven-development` —— Phase 0/1 派遣执行
- `to-tickets` —— 如需把 Phase 1 拆成 tracer-bullet 票据队列
- `dispatching-parallel-agents` —— Phase 1 前端与 B9/B10/B14 后端并行时
- `verification-before-completion` / `requesting-code-review` —— 每单验收
- `handoff` —— 下个 checkpoint

## 7. 参考索引

- 计划（唯一事实源）：`docs/plans/2026-07-22-frontend-redesign-plan.md`
- 三轨计划（B/C 线）：`docs/plans/2026-07-21-three-track-refinement-plan.md`
- 产品 UX 探索报告（背景）：`product_ux_exploration_20260721.md`
- 现有前端：`src/apps/desktop-tauri/ui/src/`（`app.css` 待退役，`hooks/useChat.ts` 保留适配）
- facade 锚点：`src/crates/contracts/kernel-api/src/{session,memory,agents,settings,usage}.rs`
