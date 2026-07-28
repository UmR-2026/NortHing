# FR-T4 前端布局迁移计划：三栏骨架 → v2 单栏空间

> **执行方式**：subagent-driven-development（每 Task 一派一验，judge-m3 门）。步骤用 `- [ ]` 追踪。
> **真值**：`docs/design/2026-07-22-frontend-redesign/prototypes/theme-system.html`（CSS 唯一视觉真值）；mockup `northing-home-v1-final.html`；本计划与真值冲突一律以真值为准。

**Goal**：把 Slint 桌面主界面从旧三栏骨架（Sessions 280px | Chat | Inspector 240px + StatusBar）迁移为 v2 设计的单栏"空间"（居中 presence + 居中 stream 660px + 居中 deck + 左右把手抽屉 + 水印 + 自绘 win-ctrl）。

**Architecture**：不改数据通路（Rust app_state 的 sessions/skills/messages 管线不动），只重构 main route 的 Slint 布局树与组件挂载点；旧组件文件（SidebarView/InspectorView/StatusBarView）保留不删，内容逐步搬入抽屉/档案册/设置页。一步拆（用户拍板），不搞并存过渡。

**Tech Stack**：Slint 1.16（`no-frame` frameless 已落地）、RedesignTheme token 体系（生成器产物勿手改）、既有 v2 组件 11 个。

## Global Constraints（每个 Task 隐含遵守）

- 视觉数值照抄 theme-system.html，禁自创浓度/圆角/位置/字号。
- 无硬编码 hex/px（生成器产物 redesign_palette.slint 除外）；palette 只经 `docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py` 生成器改。
- 红线：品牌不染 rep；思考块 abyss 冷底；沉积轮褪色灰不染当前 rep；用户气泡不染 rep；.msg 不染 rep；暗色禁纯黑/霓虹；暗色 ambient 必须亮暗对称（暗底浓度上调）。
- 暗色 = `RedesignTheme.dark` 三元既有机制，新组件两组值都要有。
- 验证（每个 Task 必跑）：`$env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'; rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` 零 error 零新 warning。
- 磁盘 diff 是唯一事实来源，假汇报=永久停用；汇报截断≠未产出，先盘后判。
- 同分支并行批次禁止 amend/rebase（修复一律新 commit）。
- 家规：顺手清配额；doc 同步硬规则；god-file 防线（.rs >800 警戒）。

## 已拍板的产品决策（2026-07-28 用户）

| 决策点 | 结论 |
|---|---|
| 左把手「内在」 | agent 自身：自我认知（identity.md 已有）、成长鉴定、skill 生成——**新功能面**，本计划只做抽屉壳 + 自我认知 MVP |
| 右把手「身外之物」 | subagent worktree、生成产物、未来集成（comfyui 等）——**新功能面**，本计划只做抽屉壳 + Skills/MCP 状态 MVP |
| 会话列表 | 主界面不再显示；会话集合归宿 = **档案册**（archive 原型已有 `prototypes/archive`）；当前会话名不显示 |
| StatusBar | 整条退场；MCP 状态 → 右抽屉底部 + 设置页可见 |
| 设置入口 | ⚙ 入 deck-bar 控制行 |
| 拆栏节奏 | 一步拆；旧组件文件保留（抽屉批次复用其内容） |

## 现状锚点（侦察已核，file:line）

- `src/apps/desktop/src/ui/main.slint`：路由 L33 `current-route`；main route 内容 L248-374（HorizontalLayout 三栏）；StatusBarView L321-330；MaterialBanner L337-352；WindowChrome L378-388（跨路由，frameless 三键已接 Rust）；`left-panel-open` L61 / `right-panel-open` L72。
- `src/apps/desktop/src/ui/views/ChatPaneView.slint`：会话头（current-session-name 等 props L289-297）、消息循环 L195、TurnContainer 包裹、ThinkBlock/ToolChip 已接、DeckBar L331（placeholder 用旧文案 `AppStrings.chat-input-placeholder`）、inline-error L310-327、model picker popup L348+。
- `src/apps/desktop/src/ui/views/SidebarView.slint`：PresenceBar L201（侧栏版名片）+ sessions 列表 + 设置入口。
- `src/apps/desktop/src/ui/views/InspectorView.slint`：头部 "Inspector" + 🌙 toggle-theme L37 + Model + Skills 列表。
- `src/apps/desktop/src/ui/views/StatusBarView.slint`：mcp-status / model-status / app-title。
- 既有 v2 组件：`components/` 下 AirTint、WindowChrome、TurnContainer、ThinkBlock、ToolChip、AvatarWrap、ChronicleBar、MoodText、PresenceBar（侧栏版）、DeckBar。
- Rust 侧接线：`app_state/create_ui.rs`（ui.on_* 含 window-min/max/close）、`sessions.rs`（message_to_item）、`event_bridge.rs`、`settings/sync.rs`（identity 通路 inline_think_in_text）。

## v2 目标结构（真值 theme-system.html L55-70/L77-89 + 用户决策）

```
AppWindow (frameless, AirTint 底层 + speaking 升档已有)
├─ SpaceView（新，main route 唯一主区，margin 0 34px）
│  ├─ PresenceZone（居中：avatar-wrap 64 + 光环 / p-name / p-state / p-chrono 140×4 / p-mood）
│  ├─ stream（flex:1 滚动，inner max-width 660 居中，gap 22）
│  │  └─ 轮（TurnContainer 活跃/沉积 + ThinkBlock + ToolChip + msg + user-bubble）
│  └─ deck-wrap（padding 6/2/20，DeckBar max-width 660 居中，控制行含 ⚙）
├─ 左把手（34px 全高，垂直居中 ‹ + 竖排「内在」+ signal dot + 暖渗光）
├─ 右把手（34px，› + 竖排「身外之物」+ 冷渗光）
├─ 左抽屉（overlay 滑入：自我认知 MVP + 档案册入口）
├─ 右抽屉（overlay 滑入：Skills + MCP 状态底部）
├─ watermark（左下 left:44 bottom:22 opacity .25，已有）
└─ win-ctrl（右上 top:16 right:44，已有，数值待对齐）
```

---

### Task T4-1：单栏骨架 SpaceView + 一步拆双栏

**Files:**
- Create: `src/apps/desktop/src/ui/views/SpaceView.slint`
- Modify: `src/apps/desktop/src/ui/main.slint`（main route L248-374 整段重写；left/right-panel-open 属性语义保留供抽屉用）
- Modify: `src/apps/desktop/src/ui/views/ChatPaneView.slint`（改为 SpaceView 内部使用或将其消息流/DeckBar 逻辑迁入 SpaceView——由 coder 二选一，倾向 SpaceView 包裹 ChatPaneView 的消息流部分，复用最大）

**Interfaces:**
- Consumes: ChatPaneView 现有 props（messages/input-text/is-streaming/current-session-*/providers/default-model-provider-id/inline-error + 回调 send-message/load-more-messages/stop-streaming/export-markdown/open-session-settings/set-default-model）；root.left-panel-open/right-panel-open。
- Produces: `SpaceView.slint` export component，props/回调与 ChatPaneView 现签名一一对应（main.slint 绑定表达式除宿主名外不变）；`toggle-left-drawer()/toggle-right-drawer()` 回调从 root 透传。

- [ ] Step 1：SpaceView 骨架——VerticalLayout：presence 占位 Rectangle（height 由内容定）、消息流区（复用 ChatPaneView 消息循环与轮组件，inner 包一层 `max-width` 容器：Slint 无 max-width，用 `width: min(parent.width - 2*s6, 660px)` 表达式 + `horizontal-alignment: center` 等效）、deck-wrap（DeckBar 同法 660 居中）。margin 0 34px（把手区）。
- [ ] Step 2：main.slint main route 整段替换：HorizontalLayout 三栏 → 单个 SpaceView；StatusBarView 实例删除（属性 mcp-status/model-status/app-title 保留在 root 供 T4-4 右抽屉用）；SidebarView/InspectorView 实例删除（文件保留）；sessions/skills 等 root 属性与回调**保留**（T4-3/T4-4 复用），暂时未绑定的在报告列明。
- [ ] Step 3：banner overlay、inline-error、model picker popup 接线迁移到 SpaceView 内或 root 保留，功能不丢（发送/停止/加载更多/模型选择/export/session-settings）。
- [ ] Step 4：⚙ 设置入口——DeckBar 控制行加 ⚙ 按钮（token 化，clicked → root.open-settings 等价回调），SidebarView 旧入口随实例删除。
- [ ] Step 5：验证 + 构建冒烟（cargo build 后 exe 起 10s 无崩，可交给编排者跑）。commit：`feat(desktop): FR-T4-1 单栏骨架 SpaceView + 一步拆双栏`。
- [ ] Step 6：报告写明：哪些旧回调/属性暂时悬空（sessions 列表、toggle-skill、toggle-theme、show-subagents 等），供 T4-3/T4-4 接续。

**验收**：cargo check 零 error；主界面只剩单栏 + 把手 + 水印 + win-ctrl；发送/停止/模型选择链路 diff 可追；sessions/skills 数据属性仍在 root（未删 Rust 侧任何代码）。

### Task T4-2：居中 PresenceZone（真值 .presence）

**Files:**
- Create: `src/apps/desktop/src/ui/components/PresenceZone.slint`（新居中版；侧栏版 PresenceBar 保留到 T4-3 决定去向）
- Modify: SpaceView.slint（Step 1 占位替换为 PresenceZone）
- Modify: main.slint（绑 is-streaming 等真实状态）

**Interfaces:**
- Consumes: AvatarWrap（breathing 属性）、ChronicleBar、MoodText（均已有）；root.is-streaming、root.current-session-model。
- Produces: `PresenceZone { in property<bool> streaming; in property<string> agent-name; in property<string> mood-text; }`。

- [ ] Step 1：照真值 L58-70/L210-217 结构：avatar-wrap 64×64 + ::after 光环（auraBreath 6s，已有机制）+ p-name 18px serif 占位（Fraunces 未自托管，注释 TODO FR-T5）+ p-state（dot 呼吸 + 文案：streaming 时「生成回复中」否则「在场」）+ p-chrono 140×4 渐变（ChronicleBar 调宽度）+ p-mood（MoodText，淡入已有）。
- [ ] Step 2：presence::before 体温光晕（520×340 radial，rep 20%→7%→透明 64%；暗色 15%/5%，真值 L34）——新增 token 走生成器（命名 `presence-halo-rep` 亮/暗），AirTint 同款 radial 写法（at 用 parent.width/2 表达式禁 %）。
- [ ] Step 3：padding 36px 0 20px；gap 8px；居中。
- [ ] Step 4：验证 + commit：`feat(desktop): FR-T4-2 居中 PresenceZone`。

**验收**：数值与真值逐条对（judge 读 HTML 核）；streaming 切态文案变；暗色光晕降档。

### Task T4-3：左抽屉「内在」（壳 + 自我认知 MVP + 档案册入口）

**Files:**
- Create: `src/apps/desktop/src/ui/components/InnerDrawer.slint`
- Modify: main.slint（toggle-left → 抽屉开合，取代 left-panel-open 旧语义；WindowChrome toggle-left 回调改绑抽屉）
- Modify: `src/apps/desktop/src/ui/components/WindowChrome.slint`（把手样式升级：34px 宽 + 竖排「内在」/「身外之物」vlabel + signal dot + 渗光——两个把手文案不同，拆参数）

**Interfaces:**
- Consumes: root.left-panel-open（复名 drawer-left-open 亦可，coder 选定一并改）、identity 数据（设置同步链路 settings/sync.rs 已有 identity.md 读取——MVP 可只读展示或入口跳设置页自我认知区）。
- Produces: `InnerDrawer { in property<bool> open; callback close(); }`；WindowChrome 把手 props `left-label/right-label: string`。

- [ ] Step 1：WindowChrome 把手按真值 L83-89 升级：width 12→34px 全高、垂直居中已有、加竖排标签（Slint 无 writing-mode，用逐字竖排 Text 或旋转，coder 选最简可靠）、signal dot（左把手，占位属性 `signal: bool` 默认 false）、把手渗光 inset 阴影（左暖 rep .3 / 右冷 abyss .3——Slint box-shadow inset 不支持，用 1px 渐变条近似，报告注明降级）。
- [ ] Step 2：InnerDrawer overlay：从左滑入（x 动画 250ms ease），宽 320px，全高，surface 底 + border + 阴影；内容 MVP：① 自我认知卡（名字/色块/identity.md 摘要或「查看」跳设置页）② 档案册入口行（点击 → 报告记录占位，档案册页面是 FR-T6 单独立项）③ 关闭手势（点击遮罩/×）。
- [ ] Step 3：z 序：抽屉 > SpaceView，< win-ctrl；抽屉打开时 SpaceView 不遮挡点击（遮罩）。
- [ ] Step 4：验证 + commit：`feat(desktop): FR-T4-3 左抽屉内在壳 + 把手升级`。

**验收**：把手/抽屉数值对真值；左把手开合抽屉；旧 SidebarView 文件未删；sessions 属性悬空状态在报告列明（档案册 FR-T6 接）。

### Task T4-4：右抽屉「身外之物」（壳 + Skills + MCP 状态）

**Files:**
- Create: `src/apps/desktop/src/ui/components/OuterDrawer.slint`
- Modify: main.slint（toggle-right → 抽屉；mcp-status/model-status 绑入抽屉底部；toggle-theme 🌙 迁入抽屉头部）
- Modify: InspectorView.slint 或抽其 Skills 段落复用（coder 选：抽 `SkillsList` 子组件或直接内嵌，优先抽子组件复用）

**Interfaces:**
- Consumes: root.skills、root.mcp-status、root.model-status、root.dark-mode、toggle-skill/toggle-theme 回调（现接 InspectorView，改接抽屉）。
- Produces: `OuterDrawer { in property<bool> open; in property<...> skills/mcp-status/...; callback close/toggle-skill/toggle-theme; }`。

- [ ] Step 1：OuterDrawer 镜像 InnerDrawer（右侧滑入，320px）。
- [ ] Step 2：内容：头部「身外之物」+ 🌙 toggle-theme（从 InspectorView 迁入，win-ctrl 冲突解除——Inspector 实例已删）；Skills 列表（现有内容迁移，toggle-skill 接线）；底部 MCP 状态行（mcp-status 小字 faint）。
- [ ] Step 3：InspectorView.slint / StatusBarView.slint 文件保留，头部注释标「superseded by OuterDrawer，内容已迁移，待 FR-T6 清理」。
- [ ] Step 4：验证 + commit：`feat(desktop): FR-T4-4 右抽屉身外壳 + Skills/MCP 迁移`。

**验收**：Skills 开关链路 diff 可追；🌙 切换暗亮正常；win-ctrl 区域干净无冲突；mcp-status 显示。

### Task T4-5：win-ctrl 数值对齐 + 收尾对稿

**Files:**
- Modify: WindowChrome.slint（win-ctrl 区）

- [ ] Step 1：win-ctrl 对齐真值 L77-80：top 12→16px；按钮 24→28px；间距 4→2px；圆角 r-sm(9)→7px（新 token 或表达式，报告注明）；close hover 实心 danger → 浅红 tint（rgba(220,53,69,.1) 底 + #dc3545 字——token 化走生成器 `danger-tint`/`danger-fg`）。
- [ ] Step 2：对稿走查表逐项核（judge 执行）：presence/stream/deck/handles/watermark/win-ctrl/speaking/活跃轮/沉积轮/think/chip/气泡。
- [ ] Step 3：DeckBar placeholder 文案换 v2（`说点什么…… @ 引用文件 / 调用技能`——AppStrings 或直传，i18n frozen 硬编码中文可接受）。
- [ ] Step 4：turn-meta 行确认删除（真值 v2 删除整行）；deck-bar ⚙ 与发送键间距对稿。
- [ ] Step 5：验证 + commit：`feat(desktop): FR-T4-5 win-ctrl 对齐 + 对稿收尾`。

### Task T4-6：暗色对称 + 全路由 + 用户目验走查

- [ ] Step 1：暗色全界面截图走查（编排者用 Edge headless 模板或用户目验）：presence 光晕/抽屉/把手渗光/win-ctrl/轮/think/chip 暗色降档对称。
- [ ] Step 2：welcome/settings 路由在单栏世界的一致性（设置页淡雾 1.5%/2% 已有 token；⚙ 往返路由正常）。
- [ ] Step 3：用户 desktop:dev 目验 test list（编排者起草，参照 FR-T3b 走查单格式）。
- [ ] Step 4：审计文档 `audit-fr-t3-blockers_20260727.md` 重命名/续写为 FR-T4 收尾审计 + handoff。

## 明确不做（本计划外，后续立项）

- 档案册页面（FR-T6：archive 原型 → Slint，sessions 集合归宿 + 会话切换入口）。
- 左抽屉功能面：成长鉴定 / skill 生成（新功能，需产品单）。
- 右抽屉功能面：subagent worktree / 生成产物 / comfyui 集成（新功能）。
- 窗口拖动/边缘 resize（slint 1.16 API 缺，待框架或受控 unsafe 决策）。
- 滚动条样式（高风险，真值有 CSS 但 Slint 滚动条样式能力存疑，POC 后再定）。
- Fraunces 字体自托管（FR-T5 既有 TODO）。

## 选派建议

| Task | coder | 理由 |
|---|---|---|
| T4-1 | glm | 布局根大改 + 回调迁移判断量最大 |
| T4-2 | glm 或 ling | 组件新造，真值照抄 + token 生成器 |
| T4-3/T4-4 | ling ×2 并行（文件集：InnerDrawer/WindowChrome vs OuterDrawer/main.slint——main.slint 有交叉则串行） | 镜像结构 |
| T4-5 | bp/mimo | 数值对齐机械活 |
| 全程 | judge-m3 | 铁打 |

并行策略：T4-1 独占先行（动 main.slint 根）；T4-2 可与 T4-3 并行（文件集不交叉）；T4-4 在 T4-3 后（同改 WindowChrome/main.slint）；T4-5/T4-6 收尾。
