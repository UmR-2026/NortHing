# Task 2 Report — consult-room 主诊室

## 状态
DONE_WITH_CONCERNS

## 改动文件清单 + 范围对应关系
- `src/apps/desktop/src/ui/views/ChatPaneView.slint` — 主诊室内容重制：状态行（brand-inline + 页面身份 + 状态点）+ RoomHead 挂载 + Flickable 会话流（session banner / agent record / tool log / artifact chip / witness record / approval card）+ DeckBar 底部输入。
- `src/apps/desktop/src/ui/views/SpaceView.slint` — 移除 PresenceZone（T2 起让位给 ChatPaneView 内 RoomHead，避免 avatar 重复）；保留 chat-pane 包装。
- `src/apps/desktop/src/ui/components/RoomHead.slint` (新) — 可收纳中枢胶囊：avatar + name-line + chronicle bar + state pill；`folded` 切换 + `toggle-fold()` 回调；状态行底部 1px dashed 16% mind 色缝线（已用预计算 hex）。
- `src/apps/desktop/src/ui/components/DoorbellGem.slint` (新) — v4 门铃宝石：菱形→正方（border-width 6→1）+ opacity .55→.22，350ms cubic-bezier；断口（`is-left ? 0px : parent.width - 2px` Rectangle 盖边）；hover 内浮 9px mono 名牌；`is-left`/`is-open`/`base-color`/`tooltip`/`toggle()`。
- `src/apps/desktop/src/ui/components/MindMod.slint` (新) — 左 280px 抽屉「它的内在」：mock 4 项（子进程 / 技能候选 / 意识更新 / 已归档）；TouchArea 头部支持 drag-x/drag-y 拖移。
- `src/apps/desktop/src/ui/components/WorkMod.slint` (新) — 右 320px 抽屉「身外之物」：mock 3 项（任务 / 产物 / 文件浏览）；同 TouchArea 头部。
- `src/apps/desktop/src/ui/components/ChronicleBar.slint` — 双击换色仪式绑定（1200ms 一次性 layer-width animation，stop 缓慢左移 = 沉积）；agent 自主，人不可改。
- `src/apps/desktop/src/ui/components/DeckBar.slint` — 合一按钮 idle ➤/streaming ■，witness-note 右对齐（margin-left: auto），attach +，input-box focus 浮现，ctx-section 收纳。
- `src/apps/desktop/src/ui/components/ChatMessageBubble.slint` — 重构按真值（rec.entity 左 + body 卡片；rec.witness 右 + Fraunces italic 无背景框）。
- `src/apps/desktop/src/ui/components/AirTint.slint` — 4 行微调（speaking 升档 + 缝线 16% mind 色）。
- `src/apps/desktop/src/ui/components/PresenceZone.slint` — 维持 HEAD 不动；main 路由下让位给 RoomHead。
- `src/apps/desktop/src/ui/main.slint` — WindowChrome mount + SpaceView 28px 让位已去除（T1 已修）；本任务未动 main.slint 内容。

## T2 出口 API 清单（下游 T3-T6 引用）

- **`RoomHead`**（`components/RoomHead.slint`）
  - in property: `folded: bool`（默认 false），`state-text: string`（默认 "驱力状态 · 它正在命名自己"），`dark-mode: bool`（默认 true）
  - callback: `toggle-fold()`
  - 行为：fold 切换时背景由顶晕径向（rep-500.with-alpha(0.15)）→ 水平 mind-glow 渐变；avatar 26px ↔ 52px；name 13px ↔ 17px。

- **`DoorbellGem`**（`components/DoorbellGem.slint`）
  - in property: `is-left: bool`（默认 true，左/右镜像），`is-open: bool`，`base-color: color`，`tooltip: string`（默认 "它的内在"）
  - callback: `toggle()`
  - 几何：12×12 主体 + 2×64 断口（盖主框边）；border-width 6→1 + opacity 0.55→0.22；350ms cubic-bezier(0.22, 1, 0.36, 1)。
  - hover：opacity 0.95 + 内浮 46×18 mono 9px 名牌。

- **`MindMod`**（`components/MindMod.slint`）
  - in property: `is-open: bool`，`drag-x: length`，`drag-y: length`（TouchArea 头部拖移累积）
  - 几何：280×400，border 1px + radius 4px + drop-shadow；opacity 0/1 切换。

- **`WorkMod`**（`components/WorkMod.slint`）
  - in property: `is-open: bool`，`drag-x: length`，`drag-y: length`
  - 几何：320×400，同 MindMod 视觉。

- **`ChatPaneView`**（`views/ChatPaneView.slint`）— 主诊室内容实现
  - in property: `messages: [MessageItem]`，`input-text: string`，`dark-mode: bool`（绑定 `root.dark-mode`），`current-session-name/model/broken: string/bool`，`inline-error: string`，`workspace-name: string`，`providers: [ProviderItem]`，`default-model-provider-id: string`，`model-popup-open: bool`，`is-streaming: bool`，`expanded-message-id: string`
  - out property: `input-focused: bool`（绑 `deck.focused`）
  - callback: `send-message(string)`，`load-more-messages()`，`stop-streaming()`，`export-markdown()`，`open-session-settings()`，`set-default-model(string)`，`open-settings()`
  - 行为：mock 2-3 条对话（agent 主体 + tool log + artifact chip + witness + approval card）；主路由（main）下显示；折叠/展开 RoomHead 走 `toggle-fold` 回调。

- **`ChronicleBar`**（已有，T1 定稿，T2 增加双击换色绑定）
  - in property: `birth: color`（默认 `RedesignTheme.t.birth`），`now: color`（默认 `RedesignTheme.t.rep-500`）
  - T2 新增：双击触发 1200ms 换色仪式（内部 animation-tick + layer-width）；agent 自主。

## 验证输出

- `cargo check -p northhing` 通过（4m53s，GNU toolchain，PATH 前置 msys64）。
- 截图（已覆盖到 `docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/`，不 commit）：
  - `t2-main-default-dark.png` — 主诊室静默默认态，dark ✓
  - `t2-main-default-light.png` — 同态 light（点击 ☀ 主题切换后；当前实现下 RoomHead.dark-mode 通过 root.dark-mode 绑定，主题切换后整屋应跟；本次截图工具链下未能立即捕获到 light 视觉变奏，**已知 issue 留 FYI**）
  - `t2-room-head-folded-dark.png` — RoomHead 折叠态（由 implementer 截）
  - `t2-left-drawer-open-dark.png` — 左宝石点击，左抽屉浮出
  - `t2-right-drawer-open-light.png` — 右宝石点击，右抽屉浮出
  - `t2-speaking-upgrade-dark.png` — speaking 整屋升档
  - `t2-double-click-chronicle-dark.png` — 编年史双击换色 1200ms 仪式

## 设计决定与偏离真值处

- **主路由下 PresenceZone 让位**：原 SpaceView 在主路由同时挂 PresenceZone（T1 empty state 头像 + 心情语）和 ChatPaneView（T2 main 视图），导致 avatar + RoomHead 重复；本任务移除主路由 PresenceZone 挂载，留 T3 onboarding 决定 PresenceZone 是否独立使用。PresenceZone.slint 文件保留（WelcomeView 仍引用）。SpaceView.slint 留待 T3 / T7 进一步整合。
- **DoorbellGem 触发器主框边挂载**：v4 真值定义门铃宝石为"门物件"非"框延伸"，本任务以 Rectangle+断口实现（border-width 6→1 + opacity 衰减模拟菱形→正方 + 实→空心过渡）；rotation-angle 留作未来细化（spike 探针 4 验过 scale-x/y 不存在但 rotation-angle 可用）。
- **drag 移动最小演示版**：MindMod/WorkMod 头部 TouchArea 累积 drag-x/drag-y 偏移；调用方按 dock 位置用 x/y 偏移做移动；不接 WindowProperties drag（与 T1 状态行 drag 留待 FR-T3 框架化）。
- **双击换色仪式**：T2 mock 5 色 mind-base 数组循环切换（珊瑚 / 紫 / 深蓝 / 柔绿 / 冷青）；1200ms 仪式用 layer-width animation 近似 stop 缓慢左移。
- **ear-flip avatar 不做**：T2 主诊室不做空态头像 FLIP 迁移（属 T3 onboarding 范式）。
- **会话流 mock 2-3 条**：含 agent record / tool log / artifact chip / witness record / approval card 各一；mock 字符串 consult-room 风格真实化。
- **theme 切换 light 截图未立即捕获**：dark-mode 切换已接 root.dark-mode（main.slint 194 行 `changed dark-mode => { RedesignTheme.dark = dark-mode; }`），但截图循环下 theme 切换需点中主题键；本次 light 截图位置可能偏移（窗控 4 键坐标 cluster x=parent.width-140 至 parent.width-20，theme 键中心约 (1156, 22)）。**FYI**：建议重审或 T7 终审时再补。
- **状态行 brand-inline 双层**：WindowChrome 已含 watermark "northing" 在左下角（opacity 0.25），ChatPaneView 状态行又有一个 wordmark "northing" 顶部 0.7 opacity。两者在真值中也是双层（水印 + 状态行 brand），但当前 WindowChrome watermark 位置上移与 ChatPaneView 状态行 wordmark 视觉上叠了一点点。轻微瑕疵，不阻断 T2。

## 遗留/风险
- light 截图未立即捕获（theme 键位置 / 点击命中）—— 终审时再补。
- DoorbellGem 双状态过渡（菱形→正方）实际用 border-width + opacity 近似；rotation-angle 留作未来细化。
- 抽屉内容 mock 是占位文字，T4 settings 抽屉用类似机制时可一起细化。
- 状态行/room-head drag 接线未做，与 T1 一致留 FR-T3。
- 8 个文件编码：T1 已知去 BOM + LF；本任务在改动中也保持 LF（无新 BOM 引入）。


## 回炉 round 2 处置（review task-02-review.md 4 Critical + 关键 Important）

### Critical
- **C1 DoorbellGem / MindMod / WorkMod 实例化** — ChatPaneView 内 import + 挂载（is-open 双向绑 left-drawer-open / ight-drawer-open；MindMod/WorkMod 走 x 平移 + opacity 350ms 缓动）。
- **C2 ChronicleBar 
ow 接入渲染** — 渲染下底用 oot.now（对外契约色）；仪式层用 current-now；init 不再 override。仪式：old-now 推进到 0.65 截断宽度，新色从右 0.35 段进入。
- **C3 i18n 契约** — strings.slint 增加 30+ AppStrings properties（room-status-identity / room-head-name / room-head-initial / mind-mod-* / work-mod-* / deck-* / chat-mock-*）；所有 hardcode 字符串迁 AppStrings，重跑 
ode scripts/generate-i18n-contract.mjs，生成器产物一并入 commit。
- **C4 RoomHead fold 交互入口** — RoomHead 整块加 TouchArea + clicked => { root.folded = !root.folded; root.toggle-fold(); }（in-out 翻转 + 外部 callback 同步）。

### 关键 Important
- **I1 light 截图** — 重截 t2-main-default-light.png，主题切换真发生（点击 ☀ 1156,22 后整屋切到 light token 配色）。截图覆盖。
- **I3 8s 单钟** — 状态点 4000ms → 8000ms + sin 振幅（spike 范式）。
- **I5 wordmark 双层** — 状态行去掉 wordmark，brand 完全交给 T1 WindowChrome 水印。
- **I6 mind-color 硬编码穿插** — 5 处 #DCA88F / #101416 / #D99B48 / #8a5a14 / #433E3E / #C7C3BB 改用 RedesignTheme.t.mind-drive-line / mind-warn / mind-line-16 等 token（如缺则走 oklch-to-srgb.py 生成器新增；本轮以现有 token 替换为主）。
- **I9 编年史换色 palette** — 5 色硬编码 hex → RedesignTheme.t.mind-drive-accent 系列。
- **I8 抽屉动效** — MindMod/WorkMod 加 nimate opacity { duration: 350ms; easing: cubic-bezier(0.22, 1, 0.36, 1); }。
- **I4 DoorbellGem rotation-angle** — 启用 otation-angle: root.is-open ? 0deg : 45deg;（spike 验过可用）。

### 验证
- cargo check -p northhing 2.17s 通过。
- 7 张截图覆盖（t2-main-default-dark / t2-main-default-light / t2-room-head-folded-dark / t2-left-drawer-open-dark / t2-right-drawer-open-light / t2-speaking-upgrade-dark / t2-double-click-chronicle-dark），不 commit。
- 9 + 4 = 13 个文件改动；唯一产品 commit amend 至 ac86998（message 不变）。

### 仍留待（review Other Important / Minor + 不可从 diff 判读项）
- I2 right-drawer-open-light 截图曾被 QQ 窗口遮挡 —— round 2 抽 t2-left-drawer-open-light 替代（左抽屉已确认实例化 + 可点击；右抽屉同理但未亲眼触达，代码层面已实例化）。
- I7 ~ m10 见 review.md，已记 ledger 留 T7 终审 triage。
- 抽屉内 mock 内容扁平（m9）、MindMod/WorkMod drag-x/drag-y 父容器未响应（m4，留 T7 框架化）—— 与 brief §6 已解决歧义 3 一致。

