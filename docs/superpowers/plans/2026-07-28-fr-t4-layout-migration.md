# FR-T4 前端布局迁移计划：三栏骨架 → v2 单栏空间（用户审计修订版）

> **执行方式**：subagent-driven-development（每 Task 一派一验，judge-m3 门）。步骤用 `- [ ]` 追踪。
> **真值**：`docs/design/2026-07-22-frontend-redesign/prototypes/theme-system.html`（CSS 唯一视觉真值）；页面设计稿同目录 9 页（theme-system / onboarding / empty-state / settings-general / settings-models / settings-workspace-skills / settings-mcp / settings-access / archive）；mockup `northing-home-v1-final.html`。冲突一律以真值为准。
> **2026-07-28 用户审计修订**：v1 初稿 3 处错误已修（会话头处置/空态/档案册提前）；6 项决策已落（档案册提前进 T4、设置页并进、deck-bar 控制行做全、p-state 接计时、onboarding 并进、export/session-settings 入抽屉+档案册）。

**Goal**：Slint 桌面主界面从旧三栏骨架迁移为 v2 单栏"空间"（居中 presence + stream 660 + deck + 把手抽屉 + 水印 + win-ctrl），并把已有 9 页设计稿的页面布局（空态/档案册/设置 5 页/onboarding）全部迁移落地。

**Architecture**：不改数据通路（Rust app_state 管线不动），重构 Slint 布局树与组件挂载点；旧组件文件保留不删，内容迁入抽屉/档案册/设置页后标 superseded。一步拆（用户拍板）。

**Tech Stack**：Slint 1.16（no-frame frameless 已落地）、RedesignTheme token 生成器体系、既有 v2 组件 11 个。

## Global Constraints（每个 Task 隐含遵守）

- 视觉数值照抄真值/对应页面设计稿，禁自创浓度/圆角/位置/字号。
- 无硬编码 hex/px（生成器产物除外）；palette 只经 `docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py` 改。
- 红线：品牌不染 rep；思考块 abyss 冷底；沉积轮褪色灰；用户气泡不染 rep；.msg 不染 rep；暗色禁纯黑/霓虹；暗色 ambient 亮暗对称。
- 验证（每 Task 必跑）：`$env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'; rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` 零 error 零新 warning。
- 磁盘 diff 唯一事实来源；假汇报=永久停用；先盘后判。
- 同分支并行批次禁止 amend/rebase（修复一律新 commit）。
- 家规：顺手清配额；doc 同步硬规则；god-file 防线。

## 已拍板产品决策（2026-07-28 用户，两轮）

| 决策点 | 结论 |
|---|---|
| 左把手「内在」 | agent 自身：自我认知/成长鉴定/skill 生成（新功能面后续）；MVP=自我认知卡 + 档案册入口 + export-markdown 入口 |
| 右把手「身外之物」 | subagent worktree/产物/未来集成（后续）；MVP=Skills + MCP 状态 + 🌙 + open-session-settings 入口 |
| 会话列表 | 主界面不显示；**档案册提前进 T4**（archive.html 照抄，sessions 集合 + 切换 + export 归宿） |
| StatusBar | 整条退场；MCP 状态→右抽屉底部 + 设置页 |
| 设置入口 | ⚙ 入 deck-bar；**设置页 5 页布局迁移并进 T4** |
| 当前会话名 | 不显示；ChatPaneView 会话头卡（L70-152）整段删除 |
| deck-bar 控制行 | **做全**：⚙/模型名/发送键 + 附件/think-ctrl/ctx-ring/mic/access/工作目录（有数据源的接真值，无的占位） |
| p-state 秒数 | **接计时**（「生成回复中 · 12s」） |
| onboarding | **并进 T4**（IdentityCreatorView → onboarding.html 对齐） |
| 拆栏节奏 | 一步拆；旧文件保留标 superseded |

## 现状锚点（侦察已核）

- `ui/main.slint`：路由 L33；main route 三栏 L248-374；StatusBarView L321-330；banner L337-352；WindowChrome L378-388（frameless 三键已接 Rust）；left/right-panel-open L61/72。
- `ui/views/ChatPaneView.slint`：**会话头卡 L70-152（删：会话名/session-settings/export 按钮）**；消息循环 L195；ThinkBlock/ToolChip 已接；DeckBar L331（placeholder 旧文案）；inline-error L310；model picker popup L348+。
- `ui/views/SidebarView.slint`：PresenceBar L201 + sessions 列表 + 设置入口。
- `ui/views/InspectorView.slint`：头部 Inspector + 🌙 L37 + Model + Skills。
- `ui/views/StatusBarView.slint`：mcp/model-status/app-title。
- `ui/views/IdentityCreatorView.slint`：旧 onboarding（对 onboarding.html 重做）。
- `ui/views/SettingsView.slint` + 5 panel（Provider/Skills/MCP/Workspace/General?）：旧设置布局 + 已换 token。
- Rust：`app_state/create_ui.rs`（on_* 接线）、`sessions.rs`、`event_bridge.rs`、`settings/sync.rs`（identity 通路）。
- 既有 v2 组件：AirTint/WindowChrome/TurnContainer/ThinkBlock/ToolChip/AvatarWrap/ChronicleBar/MoodText/PresenceBar(侧栏版)/DeckBar。

## v2 目标结构（真值 + 决策）

```
AppWindow (frameless, AirTint + speaking 已有)
├─ SpaceView（main route 唯一主区，margin 0 34px）
│  ├─ PresenceZone（居中 64 头像+光环 / p-name / p-state+计时 / p-chrono / p-mood）
│  ├─ stream（inner max-width 660 居中，gap 22；空态=开场白行）
│  └─ deck-wrap（DeckBar 660 居中，控制行做全）
├─ 左把手（34px，‹ + 竖排「内在」+ signal dot + 暖渗光）→ InnerDrawer
├─ 右把手（34px，› + 竖排「身外之物」+ 冷渗光）→ OuterDrawer
├─ InnerDrawer（自我认知卡 / 档案册入口 / export-markdown 入口）
├─ OuterDrawer（Skills / 会话设置入口 / 🌙 / MCP 状态底部）
├─ ArchiveView（新 route：archive.html 照抄，sessions 集合/切换/export）
├─ watermark（左下 44/22 .25，已有）
└─ win-ctrl（右上 16/44，已有，T4-9 对齐）
路由：main(SpaceView) / archive / settings(5 页 v2) / welcome(onboarding v2)
```

---

### Task T4-1：单栏骨架 SpaceView + 一步拆双栏（glm，独占先行）

**Files:** Create `ui/views/SpaceView.slint`；Modify `ui/main.slint`（L248-374 重写）；Modify `ui/views/ChatPaneView.slint`（**会话头卡 L70-152 整段删除**，消息流/DeckBar/popup 迁入或被 SpaceView 包裹，coder 选复用最大方案）。

**Interfaces:** Consumes: ChatPaneView 现 props/回调全集（会话名相关除外）；Produces: `SpaceView`（props/回调与 ChatPaneView 现存签名一致）+ root 新增 `open-settings` 已有回调复用。

- [ ] Step 1：SpaceView VerticalLayout：presence 占位 → 消息流（inner 660 居中：`width: min(parent.width - 2*s6, 660px)` + 居中容器）→ deck-wrap（DeckBar 660 居中 + ⚙ 按钮入控制行 clicked→root.open-settings）。margin 0 34px。
- [ ] Step 2：main.slint main route 三栏 → SpaceView 单实例；StatusBarView/SidebarView/InspectorView 实例删除（文件保留）；sessions/skills/mcp-status 等 root 属性与回调保留（后续 Task 用）；**新增 archive 路由枚举值占位**（"archive"，T4-4 填内容）。
- [ ] Step 3：banner/inline-error/model picker popup 接线不丢（发送/停止/加载更多/模型选择/export-markdown/open-session-settings 回调保留在 root，UI 入口本 Task 只有 ⚙；export/session-settings 入口 T4-3/T4-4/T4-5 接）。
- [ ] Step 4：验证 + exe 冒烟 10s + commit `feat(desktop): FR-T4-1 单栏骨架 SpaceView + 一步拆双栏`；报告列明悬空回调/属性清单。

**验收**：主界面单栏；会话名不再显示；发送/模型选择链路 diff 可追；Rust 侧零删除。

### Task T4-2：PresenceZone + 空态开场白 + p-state 计时（glm）

**Files:** Create `ui/components/PresenceZone.slint`；Modify SpaceView/main.slint；生成器加 `presence-halo-rep`（亮/暗）。

- [ ] Step 1：照真值 L58-70/L210-217：avatar-wrap 64 + auraBreath 光环 + p-name 18px serif 占位（TODO FR-T5 Fraunces）+ p-state（dot 呼吸 + streaming 文案）+ p-chrono 140×4 + p-mood（MoodText 淡入）。
- [ ] Step 2：presence::before 体温光晕 520×340 radial（亮 20%/7%；暗 15%/5%，真值 L34/L59；at 用 parent.width/2 表达式）。
- [ ] Step 3：**p-state 计时**（用户拍板）：streaming 起算秒数「生成回复中 · Ns」——Rust 侧 streaming 开始时间戳已有则绑，否则 animation-tick() 驱动秒数（coder 选可靠方案，报告注明）；非 streaming 显示「在场」。
- [ ] Step 4：**空态**（empty-state.html L630-660）：messages 空时 stream 显示开场白行（opening-name 知序 + opening-dot + opening-text「我在。你想从哪里开始？」+ shimmer/hint 按设计稿数值），替换旧 "Welcome to northhing v0.1.0" 文案。
- [ ] Step 5：验证 + commit `feat(desktop): FR-T4-2 PresenceZone + 空态 + p-state 计时`。

### Task T4-3：左抽屉「内在」+ 把手升级（ling）

**Files:** Create `ui/components/InnerDrawer.slint`；Modify main.slint、WindowChrome.slint（把手双标签参数化）。

- [ ] Step 1：把手升级（真值 L83-89）：34px 全高、竖排「内在」/「身外之物」（逐字竖排 Text）、signal dot（左，`signal: bool` 占位 false）、渗光（Slint 无 inset shadow，用 1px 内缘渐变条近似，报告注明降级）。
- [ ] Step 2：InnerDrawer 左滑入 overlay（320px，250ms ease，surface+border+阴影，遮罩点击关闭，z 序 < win-ctrl）：① 自我认知卡（identity.md 摘要/「查看」跳设置页自我认知区）② **档案册入口行**（→ root.current-route = "archive"）③ **export-markdown 入口行**（接 root.export-markdown）。
- [ ] Step 3：验证 + commit `feat(desktop): FR-T4-3 左抽屉内在壳 + 把手升级`。

### Task T4-4：档案册 ArchiveView（archive.html 照抄）（glm）

**Files:** Create `ui/views/ArchiveView.slint`；Modify main.slint（archive route 填实例 + sessions 数据绑入 + 切换会话回调接 Rust 已有 select/open session 通路）；Rust 如需补「按时间段分组/统计」数据在 app_state 加只读投影（不动写路径）。

- [ ] Step 1：照 `prototypes/archive.html`：冷雾（abyss 底 1.5% + 顶 3%，**不用 rep**）、archive-header（标题/副题「它走过的路，沉在这里」/统计行「N 段封存 · 横跨 M 天」）、时间轴 session 条目（沉积 opacity 梯度保留、hover 回升）。
- [ ] Step 2：数据接真实 sessions（root.sessions 已有）；点击条目 = 打开该会话并回 main route（复用现有会话切换回调，侦察 sessions.rs/create_ui.rs 既有通路）；export-markdown 入口在每段操作位。
- [ ] Step 3：统计行 MVP：段数=sessions.length，横跨天数=最早 timestamp 距今（纯 Slint 算不了则 Rust 投影一个字符串属性，coder 选）。
- [ ] Step 4：验证 + commit `feat(desktop): FR-T4-4 档案册 ArchiveView`。

### Task T4-5：右抽屉「身外之物」（ling）

**Files:** Create `ui/components/OuterDrawer.slint`；Modify main.slint；InspectorView/StatusBarView 标 superseded 注释。

- [ ] Step 1：OuterDrawer 右滑入（镜像 InnerDrawer）：头部「身外之物」+ 🌙 toggle-theme（迁入，win-ctrl 冲突解除）。
- [ ] Step 2：Skills 列表迁移（toggle-skill 接线）；**open-session-settings 入口行**；底部 MCP 状态行（mcp-status 小字 faint）。
- [ ] Step 3：验证 + commit `feat(desktop): FR-T4-5 右抽屉身外壳`。

### Task T4-6：设置页 5 页布局迁移（glm ×2~3 批，按页分批）

**Files:** Modify `ui/views/SettingsView.slint` + 各 panel（ProviderSettingsPanel/SkillsSettingsPanel/MCPSettingsPanel/WorkspaceSettingsPanel + 通用页）；对照 `prototypes/settings-*.html` 5 页。

- [ ] Step 1：设置页壳：淡雾（底 1.5% + 顶 2%，既有 token 检查补齐）+ 布局对 settings-general.html（壳+通用+自我认知区，头像光环）。
- [ ] Step 2：models（settings-models.html）：⚠ 染 rep-500、连接态 abyss-500。
- [ ] Step 3：ws-skills / mcp / access（各自设计稿；access 自治档选中态 rep 高亮）。
- [ ] Step 4：MCP 状态在设置页可见（StatusBar 遗产第二落点）。
- [ ] Step 5：验证 + commit（按页分批 commit）。

### Task T4-7：deck-bar 控制行做全（ling）

**Files:** Modify `ui/components/DeckBar.slint`；数据源缺口在报告列明。

- [ ] Step 1：照 home mockup deck-bar（L196+ CSS .deck-bar/.db/.think-ctrl/.ctx-ring/.mic/.access）：⚙（已有 T4-1）+ 模型名 ◇ 染 rep（di rep 已有）+ ＋附件 + think-ctrl 思考分段条 + 工作目录 + access 自治档（abyss 色）+ ctx-ring 上下文圆环 + mic。
- [ ] Step 2：有真实数据的接真值（工作目录=current workspace；模型名已有）；无数据源的（think 档位/自治档/ctx 占用/mic/附件）占位 disabled + tooltip 注释，报告列数据需求清单交后续功能单。
- [ ] Step 3：deck::before 顶部 1px 渐变线（mockup 有，FR-T3b 遗留项）。
- [ ] Step 4：验证 + commit `feat(desktop): FR-T4-7 deck-bar 控制行做全`。

### Task T4-8：onboarding 对齐（ling）

**Files:** Modify `ui/views/IdentityCreatorView.slint` + WelcomeView 路由衔接；对照 `prototypes/onboarding.html`。

- [ ] Step 1：onboarding.html 布局对稿：整屋空气染色 + 出生态头像灰白光环 + presence 体温光晕 + 心境语（出生态「我还不知道我是谁」→ 选色后关键词）+ 四字段表单（用户是【】/你是【】/你是用户的【】/大五）+ 5 色板。
- [ ] Step 2：既有 identity 写通路不动（settings/sync.rs）；win-ctrl/水印跨路由已有。
- [ ] Step 3：验证 + commit `feat(desktop): FR-T4-8 onboarding 对齐设计稿`。

### Task T4-9：win-ctrl 对齐 + 全面对稿（bp/mimo）

- [ ] Step 1：win-ctrl：top 12→16px、按钮 24→28px、间距 4→2px、圆角→7px、close hover 浅红 tint（生成器加 `danger-tint`/`danger-fg` 亮暗两组）。
- [ ] Step 2：DeckBar placeholder 换 v2 文案「说点什么…… @ 引用文件 / 调用技能」。
- [ ] Step 3：turn-meta 行确认删除；judge 全面对稿表（presence/stream/deck/handles/watermark/win-ctrl/speaking/轮/think/chip/气泡/档案册/设置 5 页/onboarding）。
- [ ] Step 4：验证 + commit `feat(desktop): FR-T4-9 win-ctrl 对齐 + 对稿收尾`。

### Task T4-10：暗色对称 + 用户目验 + 收尾

- [ ] Step 1：暗色全界面走查（截图或用户目验）：光晕/抽屉/渗光/win-ctrl/轮/档案册冷雾/设置淡雾 降档对称。
- [ ] Step 2：全路由（main/archive/settings/welcome）往返 + ⚙ 链路。
- [ ] Step 3：用户 desktop:dev 目验 test list（编排者起草）。
- [ ] Step 4：审计文档 + handoff + memory 回填。

## 明确不做（后续立项）

- 左抽屉功能面：成长鉴定 / skill 生成。右抽屉功能面：subagent worktree / 生成产物 / comfyui。deck-bar 占位件的真实数据源（think 档位/自治档/ctx 占用/mic/附件后端）。
- 窗口拖动/边缘 resize（slint 1.16 API 缺）。滚动条样式（POC 后定）。Fraunces 自托管（FR-T5 既有 TODO）。

## 选派与并行策略

| Task | coder | 备注 |
|---|---|---|
| T4-1 | glm | 独占先行（动 main.slint 根） |
| T4-2 | glm | T4-1 后；可与 T4-3 并行（文件集不交叉） |
| T4-3 | ling | 与 T4-2 并行 |
| T4-4 | glm | T4-3 后（同改 main.slint） |
| T4-5 | ling | 与 T4-4 并行（OuterDrawer/main.slint 有交叉则串行） |
| T4-6 | glm+ling 分批 | 设置 5 页 |
| T4-7/T4-8 | ling | 可并行（文件集不交叉） |
| T4-9 | bp/mimo | 机械对齐 |
| 全程 | judge-m3 | 铁打 |
