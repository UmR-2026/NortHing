# Frontend-Redesign Orchestrator Handoff — 2026-07-23

> 本 session（编排者，qwen3.8）开线"前端重设计 Slint 翻译线"并推进到 FR-T3 门口收尾。与 `2026-07-23-session2-handoff.md`（边界/债务线）、session3（P2-13/记忆检索线，活跃中）平行。

## 1. 需求基线

- **用户拍板（2026-07-23）**：Slint 是唯一桌面壳（desktop-tauri 已删，session2 执行）；前端重设计**做翻译不放弃**。
- 基线文档：原计划 `docs/plans/2026-07-22-frontend-redesign-plan.md`（用户侧，未跟踪，不碰）+ 重定向映射 `docs/design/2026-07-22-frontend-redesign/slint-retarget-notes.md`（已入库，翻译票据 FR-T1..T5 + 资产生死表，**接续者先读**）。

## 2. 今日完成（commit 表，northing main）

| commit | 内容 |
|---|---|
| `34a2397`（session2 扫场卷入） | **B9/B10 已落地**：facade `archive_session` + `SessionSummaryDto.status` + `list_sessions_all_workspaces`（judge-qw PASS；完整性+HEAD 复测已核验） |
| `2b484a7` | fix(desktop)：sample_summary 夹具补 `status` 行（B9 扫尾，HEAD 曾因此必炸） |
| `4afa7b0` | Phase 0 资产：设计五件套 + tokens-draft.css + fonts 暂存（woff2 1.47MB，R1 实测不降级） |
| `9946da9` | **FR-T1**：oklch-to-srgb.py（幂等零依赖生成器）+ tokens-srgb-table.md + `src/ui/redesign_palette.slint`（32 token 双模式 struct 三元翻转 + dur 350/1200ms）+ main.slint 一行 import（judge-qw PASS，独立换算 Δmax=0） |
| `7d1d07f` | **FR-T2**：`src/ui/fonts/`（Fraunces 静态三实例 WONK/SOFT 烘焙 + Noto SC 3655 子集 + JetBrains Mono + OFL×3 + FONTS.md）+ build.rs 一行（judge-qw PASS） |

编排者记忆仓库（独立 git）：`d713f22`（minimax 前缀修复）/ `9ab2217`（实证+episode）/ `64370ee`（qwen 七连轮 + m3 降级）。

## 3. FR-T3 状态（下次 session 接续点）

**状态：未开单，任务书未写；依赖全部就位。**

- 范围（照 retarget-notes）：topbar 名片 / 对话流（活跃轮竖线、思考块深渊青左缘、工具 chip 暖→冷 350ms、turn-meta mono）/ 操控台（发送键变形 ↑/■/禁用 + 控制行拍板存根）。
- 可直接消费的基建：
  - 调色板：`src/ui/redesign_palette.slint` → `RedesignTheme.t.*`（`dark` 默认 true 取齐现状，FR-T5 接线显示模式）；
  - 字体：`src/ui/fonts/`，引用名与 import 写法见 `src/ui/fonts/FONTS.md`（**使用文件内需 `import "./fonts/Xxx.ttf"`**；Slint 1.17 仅 ttf/ttc/otf、无可变轴设置）。
- 视觉基准：`docs/design/2026-07-22-frontend-redesign/northing-home-v1-final.html`（收起态）+ 同目录 handoff 规范。
- 换绑锚点：`src/ui/views/ChatPaneView.slint`(431) / `main.slint`(334) / `components/ChatMessageBubble.slint` / `ToolCallCard.slint` / `MaterialTextField.slint`；旧 `theme.slint`（Material 暗色体系）**不删不改**，组件逐步迁移到 RedesignTheme。
- 验收：`cargo check -p northhing` 绿 + `pnpm run desktop:dev` 实跑 + **用户视觉走查对 mockup（首个需走查的单）**。
- 选派：coder-qw + judge-qw（七连零返修）；m3 停用观察。

## 4. 队列（blocking 边 + 并行可行性）

| 序 | 单 | 依赖 | 并行 |
|---|---|---|---|
| 1 | **FR-T3 组件骨架换绑** | T1✅+T2✅ | — |
| 2 | FR-T4 空态出生态+懒建+sess-tag 菜单 | T3 | — |
| 3 | FR-T5 设置·通用页（临时代号、显示模式接 RedesignTheme.dark） | T1✅ | 可与 T3/T4 并行（不同文件集，注意同 crate） |
| 4 | 档案馆 v1（Slint 设置页 nav + 时间轴/分组/沉积淡化/只读翻阅） | T5；B9/B10✅ | — |
| 5 | B12 ctx 圆环透出 / B11 全文检索 / B13 agent identity | 壳无关 | 可与任何前端单并行（crates vs ui 不相交） |

B14（desktop-tauri commands）已作废（tauri 删除）。

## 5. subagent 运维变更

- **MiniMax 前缀修复**：plan 渠道 `minimax-cn-coding-plan` 从平台目录消失，gen 脚本 + 4 变体 + judge.md 改自建 `minimax/*`（已重启生效，派发解析恢复）。
- **m3 停用观察**：修复后探针 + 首单（judge FR-T2）空返回 ×2，疑似 think 模式与派发通道不兼容；judge 默认 judge-qw 单点。
- **qwen 七连轮零返修**（coder 达 lc 级、judge 达 m3 级再确认，含独立重算/webfetch 核文档类深度取证）。
- session2 记录的用户决策"晚 10 点前不用 qwen 做 coder"——长期性待用户确认。

## 6. 已知雷区（新增）

- **另一 session 活跃时其 `git add -A` 会污染 index**：本 session 发生 `5f2771a` 事故（席卷 session3 staged 工作 + 用户侧文件），已 `reset --mixed` 修复并分两笔干净 commit（9946da9/7d1d07f）。**铁律：commit 前必须 `git diff --cached --name-only` 复核 index 全量**（已入 ERRORS.md）。
- Slint 1.17 字体只认 ttf/ttc/otf，无可变轴设置；woff2 编译报错。
- session3（identity/记忆检索）仍在活跃，工作树有其未提交文件（identity.rs 等）——勿碰、勿 stage。

## 7. Suggested skills

`preflight-skill-check`（每回合）→ `dispatching-parallel-agents` / `subagent-driven-development`（派单）→ `verification-before-completion`（宣称完成前）→ `handoff`（收尾）。

## 8. 一句话状态

重设计 Slint 翻译线 Phase 0 + FR-T1/T2 全绿入库，B9/B10 后端已落地；**FR-T3（组件骨架换绑）万事俱备待开单**，首个需要用户视觉走查的单；m3 停用、judge-qw 单点；commit 前必查 index（他 session 会污染）。
