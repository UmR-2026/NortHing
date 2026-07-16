# NortHing 前端 Onboarding Spec (g1 + g2 + g5 + g6 + Welcome) — v1.2

> **Status**: Spec — Ready for implementation (v1.2 self-sufficient: 不再询问细节)
> **Date**: 2026-06-26 (v1.0 → v1.1 → v1.2)
> **Author**: Mavis (from grill-me session 2026-06-25~26)
> **Audience**: Coding agent (Mavis) for implementation + Human reviewer (UmR) for manual test
> **Upstream**: 项目目标锁见 `~/.mavis/agents/mavis/memory/MEMORY.md` (2026-06-26 entry)
>
> **v1.0 → v1.1 修正记录** (13 项 self-review 修正):
> - E1: Tauri dialog → **`rfd` crate**（pure Slint 栈，无 Tauri）
> - E2: MaterialTextField 扩展 `password: bool` 属性
> - Q1: P0-B 旧 3 占位 → 静默保留 + UI "清理已废弃 provider" 按钮
> - Q2: test_provider → **POST messages 最小 payload**（anthropic + openai + custom 统一）
> - Q3: Welcome **强制 step2 不能跳过**（无 provider 阻断 step3 LLM 对话）
> - Q4: per-session model override → session 设置里改
> - Q5: workspace 切换 → **过滤** session 列表（仅显示当前 workspace）
> - Q6: Provider 删除 → 允许 + 自动 fallback + 下次启动报错
> - Q7: Workspace 移除 → session 保留但状态 `broken_workspace`，无法 chat
> - Q8: 双通道 error → 默认 banner + "详情" 展开 inline
> - Q9: settings_empty = 文件不存在 OR (providers 空 AND workspaces 空)
> - Q10: Esc 快捷键 → **ChatPaneView 内部**（chat focused 时）
> - Q11: 中文 → Rust 侧 `AppStrings` const + Slint 引用 `text: AppStrings.provider-name-label;`
> - Q12: Skill 生效判断 → inspector API 看 agent prompt
> - Q13: 测试场景数 41 → **56**（typo 修正）
>
> **v1.1 → v1.2 增量** (实现期不再询问):
> - §5.6 **数据契约**：ProviderConfig / WorkspaceEntry / SkillState / MCPServerConfig / ModelRef / AppSettings 完整 Rust struct + serde + JSON 示例
> - §5.7 **AppSettings ↔ ConfigManager 边界**：UI-facing settings 唯一 owner = AppSettings；ConfigManager 保留 `ai.agent_models`；旧 `add_default_providers` 整个删除
> - §5.8 **Slint ↔ Rust bridge 模式**：5 个 Pattern（state push / callback / spawn_local / spawn_blocking for rfd / debounce save）
> - §6 **Implementation Patterns**：route 切换 / session 过滤 / error banner / rfd 用法 / **26 项 self-decided minor conventions**（C-i 到 C-xxvi）
> - §6.6 **AppStrings Slint-side wrapper**（替代 §5.4 的 Rust const 方案— Slint 1.x 用 global 单例更整洁）
> - §6.7 **Phase-end verification checklist**：每个 commit 后 verify 什么

---

## 0. Context Recap

**Why this spec**: 项目 A 阶段目标 = 自我 daily use（替换 WorkBuddy + Trae 双工具组合）。Backend 能力已 100% 具备（subagent / LLM 切换 / 长期记忆），但前端 f4 = 设置/Onboarding 未完成，无法直接 daily use。

**Design Philosophy (locked Q6)**:
- **代码相关为可检阅的黑盒，上手即用**
- 上手即用 = 默认值 sensible（自动建 workspace + 模板 IDENTITY.md + provider 表单可空启动）
- 可检阅 = UI 能看但不强制
- 黑盒 = subagent / memory / deep_review / long-horizon 内部细节不进 UI

**Target user (A 阶段)**: 非开发者用户（用户自己 daily use NortHing 不写 Rust）。所以：
- 文案中文 / 浅显
- 无 keyboard shortcut 依赖（除 Esc 停止外）
- 无 "advanced / dev" tab
- 错误展示：顶部 banner **+** inline 双通道

**In Scope (本 spec)**:
- g1 Provider 配置 UI
- g2 Session chat UI（含 session list / new / streaming / stop / tool display / export）
- g5 Workspace 选择 UI（含 sidebar 切换 + welcome screen + IDENTITY.md LLM 优化创建）
- g6 Skill 启用 UI + MCP server 启用 UI
- G1 3-step Welcome screen
- G2 错误展示通道（顶部 banner + inline）
- G3 i18n 中文 only（zh-CN 优先，en-US fallback，i18n 基础设施已就位）

**Out of Scope (per user Q6 显式跳过)**:
- ~~g3 Subagent 启动 UI~~（保留 backend 能力，UI 不暴露，subagent 在黑盒内跑）
- ~~g4 Memory / IDENTITY.md 富文本编辑器~~（文件由 workspace 选择时自动 load，可手编辑但不专门 UI）
- ~~Voice mode / Browser preview / Bugbot / Cloud agents~~（无竞品优先级，复用已实现 backend 即可）
- ~~Multi-language toggle UI~~（v1 中文 only，B 阶段加英文）
- ~~Code review / Web UI / Background Agent UI~~（B 阶段再做）

---

## 1. Locked Decisions Table (35 项)

### Cross-cutting (A 组 + E 组 = Critical Fixes)

| ID | Decision | Value | Notes |
|---|---|---|---|
| A1 | Settings 入口 | **a + b** | Sidebar 菜单项 "设置" + 顶栏右上角 cog 图标。无 keyboard 快捷键（无代码用户不熟）。 |
| A2 | Settings 内部导航 | **a** | 左侧子菜单（Providers / Workspace / 技能 / MCP / 通用）+ 右侧详情面板 |
| A3 | 数据保存时机 | **c** | 字段级 debounce 500ms 自动保存；API key 字段特殊：blur 时存（不是每键存） |
| **E1** | **Native dialog crate** | **加 `rfd` crate** | pure Slint + winit，无 Tauri；用 rfd 跨平台原生 file dialog（D1 workspace pick + C7 markdown save 都靠它） |
| **E2** | **Password input mode** | **扩展 MaterialTextField** | 加 `in property <bool> password: false;` + 内部 `input-type: password` Slint 属性；API key 字段传 `password: true` |

### g1 Provider 配置 (B 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| B1 | Provider 列表来源 | **(i) 完全空 + 用户手填全部字段** | 无 hardcoded 3 个，无 preset quick-add。首个版本完全空启动。 |
| B2 | API key 存储 | **a** | 明文 `~/.northhing/config/app.json` + 文件权限 0600 + .gitignore |
| B3 | Test 失败处理 | **(α) 保存仍生效，只显示测试结果** | 不阻断操作；UI 显示 "⚠️ 测试失败但已保存" |
| B4 | 模型选择 | **c** | Provider 特定 dropdown + "自定义模型名" 选项 |
| B5 | 默认模型 | **b + c** | 全局默认（settings 选）+ per-session override（session 创建时可改） |

### g2 Session chat (C 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| C1 | 新建 session 入口 | **a + b + c** | Sidebar 顶部 "+" 按钮 + 启动自动建默认 + 首次启动 welcome screen |
| C2 | Session 列表展示 | **b** | Sidebar 列表 + 每个 session 显示所属 workspace tag |
| C3 | Session 删除/重命名 | **b** | hover 时显示 X 图标 + 点击 session 名进入 inline 编辑 |
| C4 | 流式输出 | **a** | 实时追加（已有 ChatMessageBubble.slint 复用） |
| C5 | Tool call 显示 | **c** | inline + 点击展开详情（ToolCallCard.slint 已存在） |
| C6 | Stop 按钮 | **c** | 可见按钮 + Esc 快捷键（user override 默认 rec） |
| C7 | Session 导出 | **b** | 存为 markdown 文件（用 Tauri dialog save） |

### g5 Workspace (D 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| D1 | Workspace 选择 UI | **a** | 原生 OS file dialog（Tauri dialog plugin） |
| D2 | 首次启动 workspace | **c** | Welcome screen "选你的第一个项目文件夹" |
| D3 | IDENTITY.md 模板 | **a + LLM 对话优化创建** | 通用模板 + 用 LLM 问答生成个性化内容 |
| D4 | 多 workspace | **b** | Sidebar workspace 切换器（多个可切换）；session 不携带 workspace，继承当前选中 |

### g6 Skills (E 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| E1 | Skill 列表来源 | **a** | 硬编码扫描 `crates/assembly/core/builtin_skills/` 目录 |
| E2 | Skill 启用粒度 | **c** | 全局启用/禁用 + per-workspace override |
| E3 | Skill UI 展示 | **a** | 列表一行：名字 + 一句话描述（可检阅黑盒：不展开全文） |

### g6 MCP (F 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| F1 | MCP server 添加 UI | **a** | 表单：name + transport + command/args + env vars |
| F2 | MCP transport | **b** | stdio + sse + streamable-http（全 3 种，NortHing backend 已支持） |
| F3 | MCP test connection | **a** | 真实 ping（tools/list），失败时显示错误但保存仍生效 |

### 验收 (G 组)

| ID | Decision | Value | Notes |
|---|---|---|---|
| G1 | First-run 体验 | **a** | Welcome screen 3 步：选 workspace → 配首个 provider → 开始 chat |
| G2 | 错误展示 | **c** | 顶部 banner **+** inline 双通道（user override 默认） |
| G3 | i18n | **a** | 中文 only v1（zh-CN primary, en-US fallback，i18n 基础设施已就位 `src/shared/i18n/contract/locales.json`） |
| G4 | 验收清单 | **a** | Mavis 起草完整测试清单，user override |

### Self-review 锁定（v1.1 新增 Q 组）

| ID | Decision | Value | Notes |
|---|---|---|---|
| Q1 | P0-B 旧 3 占位迁移 | **(a) 静默保留 + UI "清理已废弃 provider" 按钮** | 不主动删除；settings 提供清理入口 |
| Q2 | test_provider endpoint | **(b) POST messages 最小 payload** | anthropic/openai/custom 统一 `{ model, messages: [{role: user, content: "hi"}], max_tokens: 1 }`；gemini 类似 |
| Q3 | IDENTITY.md LLM 循环依赖 | **(c) 强制 step2 不能跳过** | step2 配完 provider 才能进 step3；"跳过"按钮 disable |
| Q4 | per-session model override | **(c) session 设置里改** | 点击 session 进 settings 的 session 详情 tab，可改 model |
| Q5 | workspace 切换 + session 列表 | **(a) 过滤** | sidebar 只显示当前 workspace 的 session；切换 workspace 即时过滤 |
| Q6 | Provider 删除 in-use session | **(a) 允许 + 自动 fallback + 下次启动报错** | session 内 fallback 到其他 enabled provider；启动时检测"上次用已删 provider"则 inline 报错 |
| Q7 | Workspace 移除 session | **(c) session 保留但 broken_workspace** | session 显示 ⚠️ 标记，无法 chat；需重新挂到 workspace 才能用 |
| Q8 | 双通道 error 分发 | **(c) 默认 banner + "详情" 展开 inline** | banner 总显示；用户点 banner 展开 inline 详情 |
| Q9 | settings_empty 定义 + 升级路径 | **(a) 文件不存在 OR (providers 空 AND workspaces 空)** | 旧用户升级：app.json 存在但只有 P0-B 3 占位 → 视作 empty → 触发 welcome |
| Q10 | Esc 全局快捷键 | **(b) ChatPaneView 内部** | chat focused 时 Esc 才生效 |
| Q11 | 中文 hardcode 位置 | **(b) Rust 侧 AppStrings const + Slint 引用** | 集中一处改文案；B 阶段抽 i18n 时只改 const 即可 |
| Q12 | Skill 生效判断 | **(a) inspector API 看 agent prompt** | K-05 用 inspector 显示当前 system prompt 含 skill 内容 |

---

## 2. Architecture 概览

### 2.1 现有 UI 结构（参考）

```
src/apps/desktop/src/
├── main.rs # Tauri 入口 + initialize_core_services
├── app_state/
│ ├── mod.rs # AppState（已有 create_ui 末尾 spawn default session）
│ ├── sessions.rs # Session 管理（已有 set_session_error）
│ ├── skills.rs # Skill 状态
│ ├── inspector.rs # Inspector (使用 global MCPService)
│ └── slint_glue.rs # Slint ↔ Rust glue
└── ui/
 ├── main.slint # 主窗口（已有 session_error / input_error props）
 ├── theme.slint
 ├── components/ # MaterialBadge|Button|Card|IconButton|List|TextField + ChatMessageBubble + CodeBlock + MarkdownText + ToolCallCard
 └── views/
 ├── SidebarView.slint # 已有（需加 settings menu + workspace switcher + session list）
 ├── ChatPaneView.slint # 已有
 ├── InspectorView.slint # 已有
 └── StatusBarView.slint # 已有
```

### 2.2 新增/修改的文件清单

**新增（4 个 Slint views + 1 个 Rust module + 1 个 Rust strings）**:
```
src/apps/desktop/src/ui/views/
├── SettingsView.slint # 新增：设置主页（左侧子菜单 + 右侧 panel 切换）
├── WelcomeView.slint # 新增：首次启动 3 步引导
└── IdentityCreatorView.slint # 新增：IDENTITY.md LLM 对话优化创建

src/apps/desktop/src/app_state/
└── settings.rs # 新增：Settings 状态管理（provider/workspace/skill/mcp CRUD + Q6/Q7 完整性）

src/apps/desktop/src/
└── strings.rs # 新增（Q11=b）：AppStrings const 定义所有中文 UI 文案

src/apps/desktop/src/ui/components/
└── MaterialTextField.slint # 修改（E2）：加 `password: bool` 属性 + `input-type: password` 内部
```

**修改（5 个文件）**:
```
src/apps/desktop/Cargo.toml # 新增 rfd crate（E1）：`rfd = "0.14"`
src/apps/desktop/src/main.rs # 加 settings/welcome state 初始化 + welcome 首启动判断（Q9）
src/apps/desktop/src/app_state/mod.rs # 加 settings module 引用 + load 逻辑
src/apps/desktop/src/ui/main.slint # 加 settings/welcome route 切换
src/apps/desktop/src/ui/views/SidebarView.slint # 加 settings 菜单项 + workspace 切换器 + session 操作（Q5=a 过滤）
src/apps/desktop/src/ui/views/ChatPaneView.slint # 加 tool call 点击展开 + export 按钮 + stop 按钮（Q10=b Esc 内部） + per-session settings 入口（Q4=c）
```

**修改（config 层）**:
```
src/crates/assembly/core/src/service/config/manager.rs # P0-B 移除硬编码 3 provider（B1）
 # + 加 workspace 列表管理 + skill/mcp state 管理
```

### 2.3 数据流图

```
User 操作 (Slint event)
 ↓
Slint callback → @triggers slint_glue 处理
 ↓
slint_glue → AppState method call (async)
 ↓
AppState → backend crate (services-integrations / assembly-core)
 ↓
state 变化 → callback 通知 Slint
 ↓
Slint @triggers 重新计算 → UI 自动更新

错误路径（Q8=c 分发）:
任何层错误 → AppState.push_error(error)
 ↓
错误默认 push 到顶部 banner channel（slint_glue 写入 AppState.errors）
 ↓
Banner 显示 + 用户可点 "详情" 按钮 → 展开 inline 显示完整 stack/error trace
 ↓
Q6 fallback 路径：provider 删除后下次启动 → AppSettings.validate() 检测 → push banner + inline 报错

Q7 broken workspace 路径：workspace 移除后 session 仍存在 → session 状态字段改为 broken_workspace → chat button disable + ⚠️ 显示
```

---

## 3. UI 设计 Wireframe（Slint 伪代码）

### 3.1 WelcomeView（首次启动 + Q3=c 强制 step2 不能跳过）

```
WelcomeView (Window)
├── WelcomeStep1-Workspace [启动时显示]
│ ├── "欢迎使用 NortHing"（中文标题，AppStrings.welcome-title）
│ ├── "请选择你的第一个项目文件夹"（说明文字）
│ ├── "选择文件夹" 按钮 → 调 rfd::FileDialog (E1)
│ ├── 选中后显示路径 + "下一步" 按钮
│ └── 跳过按钮（高级用户，启用但给 warning tooltip）
│
├── WelcomeStep2-Provider [step1 完成后，必须配置]
│ ├── "配置你的第一个 AI 服务"
│ ├── Provider name: MaterialTextField password=false（占位 "例：我的 Anthropic"）
│ ├── Provider 类型: Dropdown（anthropic/openai/gemini/custom-openai-compatible/custom-anthropic-compatible）
│ │ → 选完自动填 base URL
│ ├── Base URL: MaterialTextField（auto-filled，可编辑）
│ ├── API key: MaterialTextField password=true（E2 扩展属性，blur 时存）
│ ├── 模型名: Dropdown + 自定义（B4）
│ ├── "测试连接" 按钮 → 真实 POST messages 最小 payload（Q2=b）
│ ├── 测试结果显示（成功✓ / 失败⚠️+错误信息中文）
│ └── "下一步" 按钮— **"跳过"按钮 disabled + tooltip "配置 AI 服务才能继续"（Q3=c 强制）**
│
└── WelcomeStep3-ChatReady [step2 完成后才出现]
 ├── "准备就绪" 标题
 ├── 简要 IDENTITY.md LLM 对话优化入口（D3）
 │ "想让我更懂你吗？" 按钮 → 进入 IdentityCreatorView
 │ "以后再说" 按钮 → 直接进入主 UI
 └── "开始聊天" 按钮 → 关闭 welcome，进入主 UI
```

### 3.2 SettingsView（A1 + A2 + A3）

```
SettingsView (Panel in main.slint)
├── 左侧子菜单（A2）
│ ├── "AI 服务" (icon) → 显示 ProviderSettingsPanel
│ ├── "工作文件夹" (icon) → 显示 WorkspaceSettingsPanel
│ ├── "技能" (icon) → 显示 SkillsSettingsPanel
│ ├── "工具集 (MCP)" (icon) → 显示 MCPSettingsPanel
│ └── "通用" (icon) → 显示 GeneralSettingsPanel
│
└── 右侧详情面板（动态切换）
 │
 ├── ProviderSettingsPanel (g1)
 │ ├── 顶部 "AI 服务" 标题 + "添加" 按钮（B1 = 空启动 + 用户手填）
 │ ├── Provider 列表（每行：name + 类型 + 状态 ●启用/○禁用 + 操作）
 │ │ └── hover 时显示：编辑 / 测试 / 删除 按钮
 │ ├── 新增/编辑 Provider 表单：
 │ │ ├── name (TextField, debounce 500ms save)
 │ │ ├── type (Dropdown: anthropic/openai/gemini/custom-openai/custom-anthropic)
 │ │ ├── base_url (TextField, auto-filled by type, editable)
 │ │ ├── api_key (TextField password, blur-save only)
 │ │ ├── model (Dropdown + custom input per B4)
 │ │ ├── enabled (Switch)
 │ │ └── "测试连接" 按钮 → 真实调用，显示结果（成功✓/失败⚠️+错误）
 │ └── "默认模型" 全局选择（B5 = b）
 │
 ├── WorkspaceSettingsPanel (g5)
 │ ├── 顶部 "工作文件夹" 标题 + "添加文件夹" 按钮（D1）
 │ ├── 当前 workspace 列表（每行：路径 + workspace name + 当前选中 ✓ + 操作）
 │ │ └── hover：切换 / 编辑 IDENTITY.md / 移除
 │ ├── 新增 workspace：调 OS dialog → 显示路径 + "立即选择" / "只添加不切换"
 │ └── "为当前 workspace 创建/更新 IDENTITY.md" 按钮 → IdentityCreatorView
 │
 ├── SkillsSettingsPanel (g6 skills)
 │ ├── 顶部 "技能" 标题
 │ ├── Skill 列表（每行：name + 一句话描述 + 全局开关 + 当前 workspace override）
 │ │ └── E3 = 一行展示，不展开
 │ └── E2 = 全局启用 + 当前 workspace override（嵌套 toggle）
 │
 ├── MCPSettingsPanel (g6 MCP)
 │ ├── 顶部 "工具集" 标题 + "添加工具集" 按钮（F1）
 │ ├── MCP server 列表（每行：name + transport + 状态 ●已连接/○失败）
 │ ├── 新增 MCP server 表单（F1 = 表单）：
 │ │ ├── name (TextField)
 │ │ ├── transport (Dropdown: stdio/sse/streamable-http, F2 = 全 3 种)
 │ │ ├── command/args (TextField, stdio 用)
 │ │ ├── url (TextField, sse/streamable-http 用)
 │ │ ├── env vars (key-value rows, +按钮添加)
 │ │ └── "测试连接" 按钮 → 真实 tools/list（F3 = a）
 │ └── 已连接 server 显示可用工具列表
 │
 └── GeneralSettingsPanel (未来扩展位, v1 只放语言切换 disabled hint)
```

### 3.3 SidebarView 修改（Q5=a 过滤当前 workspace）

```
SidebarView
├── 顶部：当前 workspace 切换器（D4 = b）
│ └── Dropdown 显示当前 workspace 路径 + 其他 workspace 列表
├── 中部：Session 列表（Q5=a **过滤当前 workspace**）
│ ├── 顶部 "+" 按钮（C1 = a）
│ ├── 每个 session 行（**仅显示当前 workspace 的 session**）：
│ │ ├── session name（C3 hover X 按钮删除 + click rename）
│ │ ├── 时间戳
│ │ ├── 当前选中高亮
│ │ └── broken_workspace 状态标记 ⚠️（Q7=c）
│ └── 切换 workspace 后列表即时过滤（无当前 workspace 的 session 不显示）
├── 底部：
│ ├── "设置" 菜单项（A1 = a）→ 切换到 SettingsView
│ └── cog 图标（A1 = b）→ 同样切换到 SettingsView
```

### 3.4 ChatPaneView 修改（Q4=c session 设置入口 + Q10=b Esc 内部 + Q7 broken 显示）

```
ChatPaneView
├── 顶部 session header
│ ├── session name（可 inline 编辑）
│ ├── session 状态标记（Q7 broken_workspace 时显示 ⚠️ + "工作文件夹已移除"）
│ └── 右上角：
│ ├── "会话设置" 按钮（Q4=c）→ 弹 settings 的 session 详情 tab，可改 model
│ ├── "导出为 Markdown" 按钮（C7 = b）→ rfd::FileDialog save
│ └── "停止" 按钮（C6 = a，仅 streaming 时显示；Q7 broken 时禁用）
├── Chat 主区域：
│ ├── 用户消息（right-aligned）
│ ├── Agent 消息（C4 = a 实时追加）
│ │ └── 含 inline tool call 卡片（C5 = c，点击展开）
│ └── Streaming cursor
├── 输入框（底部）：
│ ├── multi-line TextField（Q7 broken 时 disabled）
│ ├── 发送按钮（Cmd+Enter 提交 / Enter 换行）
│ └── inline error 显示位置（G2 = c，第二通道）
└── Esc 快捷键（Q10=b **只在 ChatPaneView focused 时生效**）：
 └── Slint key-pressed event 在 ChatPaneView 内部 handler，chat focused 才 stop
```

### 3.5 IdentityCreatorView（D3 = a + LLM 对话优化）

```
IdentityCreatorView (modal)
├── 标题："让我更懂你"
├── 进度指示：1/5, 2/5, ...
├── LLM 问答流程（5 轮左右）：
│ ├── Q1: "你主要用 NortHing 做什么？"（多选：写代码/查资料/写文档/自动化任务/其他）
│ ├── Q2: "你的项目主要用什么语言/框架？"
│ ├── Q3: "有没有特别的工作习惯或偏好？"
│ ├── Q4: "你希望我以什么风格回答？"
│ └── Q5: "还有什么需要我记住的？"
├── 实时生成 IDENTITY.md 预览（右侧）
│ └── 用户可手动编辑最后生成的 markdown
└── 底部："上一步" / "重新生成" / "保存到 workspace"
```

---

## 4. Slint 组件使用清单

| 组件 | 已有 | 用于 |
|---|---|---|
| `MaterialButton` | ✅ | 所有按钮 |
| `MaterialIconButton` | ✅ | sidebar cog / session X / 删除 |
| `MaterialCard` | ✅ | settings panel 容器 / provider 列表项 |
| `MaterialTextField` | ✅ | 所有输入（name/key/url） |
| `MaterialList` | ✅ | session/skill/mcp 列表 |
| `MaterialBadge` | ✅ | 状态标识（启用/禁用/已连接/失败） |
| `ChatMessageBubble` | ✅ | chat 消息 |
| `CodeBlock` | ✅ | tool call 代码显示 |
| `MarkdownText` | ✅ | IDENTITY.md 预览 / agent 响应 markdown |
| `ToolCallCard` | ✅ | inline tool call 显示（C5） |

**无需新增 Slint 组件**。

---

## 5. Rust 后端改动清单

### 5.1 新增 `app_state/settings.rs`

```rust
// 数据结构
pub struct AppSettings {
 pub providers: Vec<ProviderConfig>, // B1=空启动, B2=app.json 明文
 pub workspaces: Vec<WorkspaceEntry>, // D4=b
 pub current_workspace: Option<PathBuf>,
 pub skills_enabled: HashMap<String, SkillState>, // E1 + E2
 pub mcp_servers: Vec<MCPServerConfig>, // F1+F2
 pub default_model: Option<ModelRef>, // B5
}

// 方法（v1.1 完整）
impl AppSettings {
 pub async fn load_or_default() -> Result<Self>; // Q9=a: 文件不存在 OR (providers空 AND workspaces空) → empty
 pub async fn save(&self) -> Result<()>; // debounced write 500ms
 pub fn is_first_run(&self) -> bool; // Q9=a: providers.is_empty() && workspaces.is_empty()

 // Provider CRUD
 pub fn add_provider(&mut self, p: ProviderConfig);
 pub fn update_provider(&mut self, id: &str, p: ProviderConfig);
 pub fn remove_provider(&mut self, id: &str); // Q6=a: 不阻止，session 自动 fallback
 pub async fn test_provider(&self, id: &str) -> TestResult; // Q2=b: POST messages 最小 payload

 // Default model
 pub fn set_default_model(&mut self, m: ModelRef);

 // Workspace CRUD
 pub fn add_workspace(&mut self, path: PathBuf) -> Result<()>;
 pub fn set_current_workspace(&mut self, path: &Path) -> Result<()>;
 pub fn remove_workspace(&mut self, path: &Path); // Q7=c: session 转 broken_workspace
 pub fn ensure_identity_md(&self, path: &Path) -> Result<PathBuf>; // D3 模板 + LLM 调用

 // Skill
 pub fn list_skills(&self) -> Vec<SkillInfo>; // E1 扫描 builtin_skills/
 pub fn set_skill_global(&mut self, name: &str, enabled: bool); // E2=c
 pub fn set_skill_workspace(&mut self, name: &str, ws: &Path, enabled: bool);
 pub fn skill_effective_in_session(&self, name: &str, ws: &Path) -> bool; // Q12=a: 合并 global + ws override

 // MCP
 pub fn add_mcp_server(&mut self, m: MCPServerConfig);
 pub async fn test_mcp_server(&self, id: &str) -> MCPTestResult; // F3=a 真实 tools/list
 pub fn remove_mcp_server(&mut self, id: &str);

 // Validation（启动时调用）
 pub fn validate_session_integrity(&self, sessions: &mut [SessionState]); // Q6/Q7: 检测 broken provider / workspace
}
```

### 5.2 修改 `manager.rs`（P0-B 改动 + B1 决策）

```diff
- // P0-B (2026-06-25): seed 3 placeholder providers (anthropic / openai / gemini) ...
- Self::add_default_providers(&mut self.config.ai.models);
+ // B1 (2026-06-26, v1.1): 不再 hardcode 3 provider；用户从 UI 手填
+ // 旧 P0-B seed 行为已废弃：seeded providers 不再自动创建
+ // 迁移策略 (Q1=a): 现有用户 ~/.northhing/config/app.json 里的 3 个 placeholder 保留（enabled=false）
+ // UI 提供 "清理已废弃 provider" 按钮（Q1=a），不自动删除
+ // is_first_run 检测 (Q9=a): providers 空 + workspaces 空 → 触发 welcome
+ // 旧用户升级：app.json 存在但只有 P0-B 3 占位 → providers 非空（3 disabled） → is_first_run = false → 不显示 welcome
+ // 此时 UI 提示 "检测到 3 个已废弃 provider，是否清理？"
```

### 5.3 新增 `main.rs` welcome 逻辑（Q9=a）

```rust
// 启动判断
let settings = AppSettings::load_or_default().await— ;
let is_first_run = settings.is_first_run(); // Q9=a: providers 空 AND workspaces 空
if is_first_run {
 main_window.set_view("welcome");
} else {
 main_window.set_view("main");
}

// Q1=a 旧用户检测：app.json 存在但有 P0-B 占位（enabled=false 且 id 含 "default"）
let has_legacy_placeholders = settings.providers.iter()
 .any(|p| p.id.contains("default") && !p.enabled);
if has_legacy_placeholders && !is_first_run {
 // UI 顶部 banner 显示 "检测到 N 个已废弃 provider 占位，是否清理？"
 // 提供 "清理" 按钮 → settings.remove_provider(id)
}
```

### 5.4 ~~新增 `strings.rs`（已废 — 见 §6.6）~~

~~Q11=b 早期方案是 Rust 侧 `AppStrings` const + Slint `text:` 引用。但 Slint 1.x 跨语言 const 引用需要在 .slint 文件里写 wrapper global，更整洁的方案见 §6.6：直接用 `src/apps/desktop/src/ui/strings.slint` 的 `global AppStrings` 单例。**v1.2 不再创建 `strings.rs`，所有中文文案集中在 `ui/strings.slint`**。~~

### 5.5 Slint 集成（slint_glue.rs）

```rust
slint::invoke_from_event_loop(move || { ui.set_providers(settings.providers); });
slint::invoke_from_event_loop(move || { ui.set_workspaces(settings.workspaces); });
// ... 各类 state push
// Q8=c 错误分发：默认 banner，AppState.push_error 写入 errors 数组；用户点 banner → 展开 inline
```

### 5.6 数据契约（v1.2 exact Rust structs + serde）

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// === Provider ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
 Anthropic, // base_url: https://api.anthropic.com
 OpenAI, // base_url: https://api.openai.com/v1
 Gemini, // base_url: https://generativelanguage.googleapis.com/v1beta
 CustomOpenAICompatible, // user provides base_url
 CustomAnthropicCompatible, // user provides base_url
}

impl ProviderType {
 pub fn default_base_url(&self) -> &'static str {
 match self {
 Self::Anthropic => "https://api.anthropic.com",
 Self::OpenAI => "https://api.openai.com/v1",
 Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
 Self::CustomOpenAICompatible | Self::CustomAnthropicCompatible => "",
 }
 }
 pub fn default_models(&self) -> &'static [&'static str] {
 match self {
 Self::Anthropic => &["claude-sonnet-4-5", "claude-opus-4", "claude-haiku-4"],
 Self::OpenAI => &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
 Self::Gemini => &["gemini-2.0-flash", "gemini-1.5-pro"],
 Self::CustomOpenAICompatible => &[],
 Self::CustomAnthropicCompatible => &[],
 }
 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
 pub id: String, // UUID v4, immutable
 pub name: String, // user-facing label, e.g. "我的 Anthropic"
 #[serde(rename = "type")]
 pub provider_type: ProviderType,
 pub base_url: String, // auto-filled from type, user-editable
 pub api_key: String, // 0600 file perms, never logged
 pub model: String, // selected from dropdown OR custom
 pub enabled: bool,
 pub created_at: i64, // unix seconds, for sorting/display
 pub last_verified_at: Option<i64>, // when test_provider last succeeded
 pub last_verified_ok: Option<bool>,// Q3: 显示 ⚠️ if Some(false)
}

// === Workspace ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
 pub path: PathBuf,
 pub display_name: String, // auto = folder basename, user-editable
 pub added_at: i64,
 pub last_opened_at: i64,
 pub identity_md_path: Option<PathBuf>, // path to IDENTITY.md (auto-detected)
}

// === Skill ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillState {
 pub name: String, // e.g. "memory", matches folder name in builtin_skills/
 pub global_enabled: bool, // default true
 pub workspace_overrides: HashMap<PathBuf, bool>, // E2: per-workspace
}

impl SkillState {
 pub fn effective_in(&self, workspace: &Path) -> bool {
 // workspace override > global > default true
 self.workspace_overrides.get(workspace).copied()
 .unwrap_or(self.global_enabled)
 }
}

// === MCP Server ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MCPTransport {
 Stdio,
 Sse,
 StreamableHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
 pub id: String, // UUID v4
 pub name: String,
 pub transport: MCPTransport,
 pub enabled: bool,
 pub command: Option<String>, // stdio only
 pub args: Vec<String>, // stdio only
 pub url: Option<String>, // sse / streamable-http
 pub env: HashMap<String, String>, // stdio env vars
 pub last_verified_at: Option<i64>,
 pub last_verified_ok: Option<bool>,
 pub last_tools: Vec<String>, // tool names from last tools/list success
}

// === Default model ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
 pub provider_id: String,
 pub model: String,
}

// === Top-level ===

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
 pub schema_version: u32, // = 1
 pub providers: Vec<ProviderConfig>,
 pub workspaces: Vec<WorkspaceEntry>,
 pub current_workspace: Option<PathBuf>,
 pub skills_enabled: Vec<SkillState>, // one per discovered builtin skill
 pub mcp_servers: Vec<MCPServerConfig>,
 pub default_model: Option<ModelRef>,
}

impl AppSettings {
 /// Q9=a: 触发 welcome 的判定
 pub fn is_first_run(&self) -> bool {
 // 旧 P0-B placeholder (id 含 "-default" 且 enabled=false) 不算 "已有 provider"
 let real_providers = self.providers.iter()
 .filter(|p| !p.id.contains("-default") || p.enabled)
 .count();
 real_providers == 0 && self.workspaces.is_empty()
 }

 /// Q1=a: 检测 P0-B 旧 3 占位
 pub fn has_legacy_placeholders(&self) -> bool {
 self.providers.iter()
 .any(|p| p.id.contains("-default") && !p.enabled)
 }

 /// Q5=a: 当前 workspace 的 session 列表（外部 caller 过滤后传入）
 pub fn current_workspace(&self) -> Option<&PathBuf> {
 self.current_workspace.as_ref()
 }

 /// Q6/a: provider 删后，session 应 fallback 到其他 enabled provider
 pub fn fallback_provider_for(&self, deleted_id: &str) -> Option<&ProviderConfig> {
 self.providers.iter().find(|p| p.enabled && p.id != deleted_id)
 }

 /// Q7: workspace 删除时调用，返回受影响的 session id 列表（外部 caller 标记）
 pub fn sessions_in_workspace<'a>(&self, ws: &Path, sessions: &'a [SessionState]) -> Vec<&'a str> {
 sessions.iter()
 .filter(|s| s.workspace.as_deref() == Some(ws.to_str().unwrap_or("")))
 .map(|s| s.id.as_str())
 .collect()
 }
}
```

**JSON schema 示例**（`~/.northhing/config/app.json`）:
```json
{
 "schema_version": 1,
 "providers": [
 {
 "id": "550e8400-e29b-41d4-a716-446655440000",
 "name": "我的 Anthropic",
 "type": "anthropic",
 "base_url": "https://api.anthropic.com",
 "api_key": "sk-ant-...",
 "model": "claude-sonnet-4-5",
 "enabled": true,
 "created_at": 1782400000,
 "last_verified_at": 1782400100,
 "last_verified_ok": true
 }
 ],
 "workspaces": [
 {
 "path": "/home/user/projects/northing",
 "display_name": "northing",
 "added_at": 1782400000,
 "last_opened_at": 1782400000,
 "identity_md_path": "/home/user/projects/northing/IDENTITY.md"
 }
 ],
 "current_workspace": "/home/user/projects/northing",
 "skills_enabled": [
 {"name": "memory", "global_enabled": true, "workspace_overrides": {}},
 {"name": "pdf", "global_enabled": true, "workspace_overrides": {}}
 ],
 "mcp_servers": [],
 "default_model": {
 "provider_id": "550e8400-e29b-41d4-a716-446655440000",
 "model": "claude-sonnet-4-5"
 }
}
```

### 5.7 AppSettings ↔ ConfigManager 边界（v1.2 锁定）

**Q: 旧 `manager.rs::ConfigManager` 处理什么？新 `AppSettings` 处理什么？**

```
src/crates/assembly/core/src/service/config/manager.rs
├── ConfigManager # 已有
│ ├── ai.agent_models # function-agent 子 agent 的模型（保留）
│ ├── ai.func_agent_models # 同上
│ └── 旧 add_default_providers # P0-B：seeded 3 placeholder → v1.2 删除
│
└── manager.rs 新职责：
 └── AppSettings 的磁盘 I/O 委托给它
 （load_from_disk / save_to_disk 调用 ai.models 等）

src/apps/desktop/src/app_state/settings.rs (新增)
└── AppSettings # 完整 ownership
 ├── providers ← 取代 ConfigManager.ai.models
 ├── workspaces (新)
 ├── skills_enabled (新)
 ├── mcp_servers (新)
 └── default_model (新)
```

**决策（v1.2）**:
- **AppSettings 是 UI-facing settings 的唯一 owner**（provider / workspace / skill / mcp）
- ConfigManager **保留** `ai.agent_models` / `ai.func_agent_models`（这是 function agent 内部配置，不是用户配置）
- 旧 `add_default_providers` 方法**整个删除**（P0-B 行为已废）
- ConfigManager 暴露 `load_app_settings() -> AppSettings` 和 `save_app_settings(settings: &AppSettings)`，让 AppSettings 走它的磁盘路径规则
- 文件位置：`~/.northhing/config/app.json`（沿用现有路径）

### 5.8 Slint ↔ Rust bridge 模式（v1.2 锁定）

**Pattern 1: Rust → Slint state push**

```rust
// 在 AppState methods 里
pub fn push_providers(&self, ui: &AppWindow) {
 let providers_dto: Vec<ProviderItem> = self.settings
 .providers
 .iter()
 .map(|p| ProviderItem {
 id: p.id.clone().into(),
 name: p.name.clone().into(),
 provider_type: format!("{:— }", p.provider_type).into(),
 enabled: p.enabled,
 verified: matches!(p.last_verified_ok, Some(true)),
 // ...
 })
 .collect();
 let model = VecModel::from(providers_dto);
 ui.set_providers(ModelRc::new(model));
}
```

**Pattern 2: Slint callback → Rust**

```rust
// 在 create_ui 里
let ui_weak = ui.as_weak();
let state_weak = Arc::downgrade(&state);
ui.on_add_provider(move |name, provider_type, base_url, api_key, model| {
 let state = state_weak.upgrade().expect("state alive");
 let ui = ui_weak.upgrade().expect("ui alive");
 
 // 解析参数
 let pt = match provider_type.as_str() {
 "anthropic" => ProviderType::Anthropic,
 "openai" => ProviderType::OpenAI,
 "gemini" => ProviderType::Gemini,
 "custom-openai" => ProviderType::CustomOpenAICompatible,
 "custom-anthropic" => ProviderType::CustomAnthropicCompatible,
 _ => return,
 };
 
 // 异步操作（不阻塞 UI 线程）
 let state_clone = state.clone();
 let ui_clone = ui.clone();
 slint::spawn_local(async move {
 match state_clone.add_provider(name.to_string(), pt, base_url.to_string(), api_key.to_string(), model.to_string()).await {
 Ok(()) => {
 state_clone.push_providers(&ui_clone);
 state_clone.push_status_banner("✓ 已添加", false);
 }
 Err(e) => {
 state_clone.push_error(ErrorLevel::Error, format!("添加失败: {e}"), &ui_clone);
 }
 }
 }).unwrap();
});
```

**Pattern 3: 异步操作完成 → 更新 Slint**

- 永远在 `slint::spawn_local` 内（Slint 1.x 规定所有 UI mutation 必须在 event loop）
- 用 `ui_weak.upgrade()` 获取当前 ui handle（ui 可能已销毁）
- 用 `state_weak.upgrade()` 获取 state
- 完成后 `ui.set_xxx(...)` 更新 UI

**Pattern 4: rfd file dialog（同步阻塞 → spawn_blocking）**

```rust
use rfd::FileDialog;

pub async fn pick_workspace_folder() -> Result<Option<PathBuf>> {
 // rfd::FileDialog::new() 是同步阻塞调用，必须放到 spawn_blocking
 tokio::task::spawn_blocking(|| {
 FileDialog::new()
 .set_title("选择项目文件夹")
 .pick_folder()
 })
 .await
 .map_err(|e| anyhow!("dialog canceled or failed: {e}"))
}
```

**Pattern 5: debounce save**

```rust
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

pub struct AppSettingsState {
 inner: Arc<Mutex<AppSettings>>,
 dirty: Arc<Mutex<bool>>,
 save_tx: tokio::sync::mpsc::Sender<()>,
}

impl AppSettingsState {
 pub fn new(initial: AppSettings) -> Self {
 let inner = Arc::new(Mutex::new(initial));
 let dirty = Arc::new(Mutex::new(false));
 let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(16);
 
 // background task: 每 500ms 检查 dirty，true 则 save
 let inner_bg = inner.clone();
 let dirty_bg = dirty.clone();
 tokio::spawn(async move {
 let mut tick = interval(Duration::from_millis(500));
 loop {
 tick.tick().await;
 let mut dirty = dirty_bg.lock().await;
 if *dirty {
 *dirty = false;
 drop(dirty);
 let snapshot = inner_bg.lock().await.clone();
 if let Err(e) = save_to_disk(&snapshot).await {
 tracing::error!("settings save failed: {e}");
 // 错误 push 到 banner（Q8=c 路径）
 }
 }
 }
 });
 
 Self { inner, dirty, save_tx: tx }
 }
 
 pub async fn mutate<F: FnOnce(&mut AppSettings)>(&self, f: F) {
 let mut g = self.inner.lock().await;
 f(&mut g);
 *self.dirty.lock().await = true;
 }
}
```

---

## 6. Implementation Patterns（v1.2 新增 — 不再询问的所有约定）

下面所有条目都是"我会自己决定"的具体实现细节，避免编码时停下来问"这里怎么办"。

### 6.1 Route 切换（main / welcome / settings 三视图）

```slint
// main.slint 顶层结构
export component AppWindow inherits Window {
 // 当前 route 状态
 in-out property <string> current-route: "main"; // "main" | "welcome" | "settings"
 
 if current-route == "main": MainView { ... }
 if current-route == "welcome": WelcomeView { 
 on-completed => { root.current-route = "main"; }
 }
 if current-route == "settings": SettingsView {
 on-close => { root.current-route = "main"; }
 }
}
```

- 不用 Slint 的 `Window.hide/show`（会丢 state）
- 用 `if condition` + route property 切换（保持 state 在内存）
- 切换是即时的（无 fade/slide 动画— 简单可靠）

### 6.2 Workspace 切换 → Session 过滤（Q5=a）

```rust
// AppState method
pub async fn switch_workspace(&self, path: &Path, ui: &AppWindow) -> Result<()> {
 self.settings.mutate(|s| s.current_workspace = Some(path.to_path_buf())).await;
 
 // 重新 push filtered session list（仅当前 workspace 的）
 let sessions: Vec<SessionItem> = self.sessions
 .iter()
 .filter(|s| s.workspace.as_deref() == Some(path.to_str().unwrap_or("")))
 .map(|s| s.to_slint_dto())
 .collect();
 ui.set_sessions(ModelRc::new(VecModel::from(sessions)));
 Ok(())
}
```

### 6.3 Error banner UI + "详情" 展开 inline（Q8=c）

```slint
// MaterialBanner component (new, in components/MaterialBanner.slint)
export component MaterialBanner inherits Rectangle {
 in property <string> message: "";
 in property <string> detail: ""; // 空时 "详情" 按钮隐藏
 in property <bool> auto-dismiss: true;
 in property <int> auto-dismiss-secs: 5;
 
 callback dismissed();
 callback detail-clicked(); // 通知 inline panel 展开
 
 height: 56px;
 background: MaterialTheme.color-error;
 // ... layout: icon + message + "详情" 按钮 + X 关闭按钮
}

// chat input 上方 inline error 区
component InlineError {
 in property <string> message: "";
 // 当 banner "详情" 被点击时，AppState 把同一 error 复制到 inline
}
```

**分发逻辑**：
- 任何错误默认 push 到 banner channel
- banner 显示 + 5s 自动消失（已有 P0-C 行为）
- 用户点 banner 的 "详情" → AppState 把同一条 error 复制到当前 view 的 inline channel（不是另存，是 push 到 inline queue）
- inline 错误需要用户手动 X 关闭

### 6.4 rfd usage（E1）

**rfd = "0.14"**, blocking API 用 `tokio::task::spawn_blocking` 包：

```rust
// D1: workspace folder picker
pub async fn pick_workspace_folder() -> Option<PathBuf> {
 tokio::task::spawn_blocking(|| {
 rfd::FileDialog::new()
 .set_title("选择项目文件夹")
 .pick_folder()
 })
 .await
 .ok()
 .flatten()
}

// C7: markdown export save
pub async fn pick_markdown_save_path(default_name: &str) -> Option<PathBuf> {
 let default_name = default_name.to_string();
 tokio::task::spawn_blocking(move || {
 rfd::FileDialog::new()
 .set_title("导出会话为 Markdown")
 .set_file_name(&default_name)
 .add_filter("Markdown", &["md"])
 .save_file()
 })
 .await
 .ok()
 .flatten()
}
```

- 跨平台原生 dialog（Windows / macOS / Linux）
- 不支持 remote URL / mobile-web（spec scope 仅 desktop）

### 6.5 Self-decided minor conventions（v1.2 新增 — 不询问）

| # | 决策 | 决定 |
|---|---|---|
| C-i | Provider/Skill/MCP 添加 UI | **Inline 行展开**（点 "+" 在列表底展开新行），不用 dialog/modal（modal 遮挡难用） |
| C-ii | Provider type 5 个变体对应 base_url + 默认模型列表 | 见 §5.6 `ProviderType::default_base_url()` + `default_models()` |
| C-iii | 测试连接按钮 UX | 按下 → 按钮禁用 + 显示 "测试中..." spinner → 完成后显示结果（成功 ✓ / 失败 ⚠️+error） |
| C-iv | 列表为空状态 | 居中 Card "还没有添加 X"+ 添加按钮（每种列表类似） |
| C-v | 长内容滚动 | `ScrollView` 包裹（Slint 内置） |
| C-vi | Markdown 导出格式 | `# Session: {name}\n\nWorkspace: {ws}\nCreated: {date}\n\n---\n\n` + 每条消息 `**[{role}]** {content}\n\n` + tool call 用 ```json``` 包裹 |
| C-vii | Skill 发现文件模式 | `crates/assembly/core/builtin_skills/*/SKILL.md`，读取 front-matter 的 name + description |
| C-viii | Test timeout | **10 秒**（Provider 和 MCP 都用） |
| C-ix | Workspace 移除确认 | Dialog "确认移除？将影响 N 个会话（这些会话将标记为 broken_workspace）" + [取消] [确认] |
| C-x | Default model fallback 行为 | 若 `default_model.provider_id` 找不到：fallback 到 `providers.iter().find(\|p\| p.enabled).first()` |
| C-xi | Skill E2 — 突解决 | workspace override > global > 默认 true（已在 SkillState::effective_in 实现） |
| C-xii | IDENTITY.md LLM 失败 UX | preview pane 显示 "LLM 调用失败（错误信息），可手动编辑" + 用户可保存手编辑版本 |
| C-xiii | MCP server enable toggle | 加 `enabled: bool`（v1.2 已有），UI 默认 enabled；disable 后不在 session 中暴露 tools |
| C-xiv | Chat 时无 enabled provider | inline error "请先在设置中添加 AI 服务" + "去设置" 链接按钮 |
| C-xv | Settings panel 内更多 provider（>50） | 滚动 + search box（简单 substring filter on name） |
| C-xvi | Provider 添加期间 session 进行中 | session 用旧 provider 直到下次重启；新增 provider 不影响进行中的 session |
| C-xvii | App 关闭时未保存修改 | main loop exit handler 强制 flush 一次 save（`force_save_now()` 方法） |
| C-xviii | Save 错误处理（disk full / perms） | log error + push banner "保存失败：{error}（变更未持久化）" + 不阻塞 UI |
| C-xix | WelcomeView "跳过" step1 | 启用但加 tooltip "选择工作文件夹后才能用 NortHing"；step1 跳过仍可进 step2（不强阻） |
| C-xx | WelcomeView 步进顺序 | step1 → step2 → step3 直进；step 间无动画（简单）；"上一步"按钮 step2/step3 有，step1 无 |
| C-xxi | SettingsView 子菜单顺序 | Providers → Workspace → 技能 → 工具集（MCP）→ 通用（v1.1 几乎空） |
| C-xxii | Slint password input | MaterialTextField `password: true` → 内部 `TextInput { input-type: password; }` |
| C-xxiii | Per-session model override UI | "会话设置"按钮在 ChatPaneView header → 弹 dialog 含 model Dropdown + "应用" 按钮 |
| C-xxiv | 旧 P0-B provider 检测 | `has_legacy_placeholders()` 检查 id 含 "-default" 且 enabled=false |
| C-xxv | 多个 provider 同 type | 允许（如"我的 Anthropic-工作" + "我的 Anthropic-私人"），用户用 name 区分 |
| C-xxvi | Identity.md LLM prompt | 5 问 hardcoded 在 Rust const（Q11=b 风格）；每次问单独调 LLM（max 100 tokens response） |

### 6.6 AppStrings Slint-Rust bridge（Q11=b）

**Slint 1.x 不能直接引用 Rust const**，需要 wrapper：

```slint
// src/apps/desktop/src/ui/strings.slint (new)
export global AppStrings {
 out property <string> welcome-title: "欢迎使用 NortHing";
 out property <string> welcome-step1-desc: "请选择你的第一个项目文件夹";
 out property <string> welcome-step2-desc: "配置你的第一个 AI 服务";
 out property <string> welcome-step2-blocked-tooltip: "配置 AI 服务才能继续";
 out property <string> welcome-step3-desc: "准备就绪";
 out property <string> provider-name-label: "AI 服务名称";
 out property <string> provider-type-label: "类型";
 out property <string> provider-base-url-label: "服务地址";
 out property <string> provider-api-key-label: "API 密钥";
 out property <string> provider-model-label: "模型";
 out property <string> provider-test-button: "测试连接";
 out property <string> provider-test-success: "✓ 连接成功";
 out property <string> provider-test-failed-saved: "⚠️ 测试失败但已保存";
 out property <string> provider-cleanup-legacy: "清理已废弃的 {n} 个 AI 服务占位";
 out property <string> workspace-add: "添加文件夹";
 out property <string> workspace-current: "当前";
 out property <string> workspace-broken-marker: "工作文件夹已移除";
 out property <string> session-new: "新会话";
 out property <string> session-rename: "重命名会话";
 out property <string> session-delete: "删除会话";
 out property <string> session-settings-button: "会话设置";
 out property <string> session-export-md: "导出为 Markdown";
 out property <string> session-stop: "停止";
 out property <string> chat-send: "发送";
 out property <string> chat-empty-provider-error: "请先在设置中添加 AI 服务";
 out property <string> settings-open: "设置";
 out property <string> settings-providers: "AI 服务";
 out property <string> settings-workspaces: "工作文件夹";
 out property <string> settings-skills: "技能";
 out property <string> settings-mcp: "工具集 (MCP)";
 out property <string> settings-general: "通用";
 out property <string> identity-prompt-q1: "你主要用 NortHing 做什么？";
 out property <string> identity-prompt-q2: "你的项目主要用什么语言/框架？";
 out property <string> identity-prompt-q3: "有没有特别的工作习惯或偏好？";
 out property <string> identity-prompt-q4: "你希望我以什么风格回答？";
 out property <string> identity-prompt-q5: "还有什么需要我记住的？";
 out property <string> error-detail-button: "详情";
 out property <string> error-save-failed: "保存失败：{error}（变更未持久化）";
 out property <string> legacy-banner-title: "检测到已废弃 AI 服务占位";
 // ... 约 60 个 string
}

// 引用：text: AppStrings.welcome-title;
```

**为什么 Slint 文件而不是纯 Rust**:
- Slint 1.x 的 `global` 是单例，导入一次全局可用
- 修改文案只动这一个文件，B 阶段抽 i18n 时也只改这一个文件
- Rust 侧不需要复制 string（避免不一致）

**位置**: `src/apps/desktop/src/ui/strings.slint`（在 ui/ 而非 src/，因为是 UI assets）

### 6.7 实施期 checklist（编码时逐项 verify）

每个 Phase 开始前/结束后 verify：

**Phase 1 结束**:
- [ ] `cargo check -p northhing` 通过
- [ ] `rfd` 在 Cargo.toml 加了
- [ ] `manager.rs` 旧 `add_default_providers` 已删
- [ ] AppSettings struct + 全部方法有 unit test
- [ ] JSON 序列化往返测试（save → load → equals）
- [ ] debounce save 集成测试（mutate → 500ms 后 disk 上能看到）

**Phase 2 结束**:
- [ ] `cargo build -p northhing` 通过
- [ ] MaterialTextField 加了 `password: bool` 属性
- [ ] SettingsView.slint 渲染空状态（无 providers）+ 左侧子菜单显示
- [ ] sidebar 加 "设置"菜单项 + cog 图标（暂不响应点击）
- [ ] main.slint route 切换属性 + 3 个 if 块

**Phase 3 结束**:
- [ ] ProviderSettingsPanel：add / edit / delete / test 全链路通
- [ ] SkillsSettingsPanel：scan builtin_skills 显示 + toggle
- [ ] MCPSettingsPanel：add / test / delete 全链路通
- [ ] "清理已废弃"按钮工作
- [ ] P-01..P-10 + K-01..K-07 + M-01..M-08 manual test 全过

**Phase 4 结束**:
- [ ] WorkspaceSettingsPanel：add / switch / remove（含确认 dialog）
- [ ] WelcomeView 3 步走通，step2 跳过按钮 disabled
- [ ] IdentityCreatorView 5 问 + 生成 + 保存
- [ ] SidebarView：workspace 切换触发 session 过滤（Q5=a）
- [ ] ChatPaneView：tool call 点击展开 + Esc 内部 stop + export markdown + 会话设置入口
- [ ] W-01..W-08 + S-01..S-12 + WEL-01..WEL-06 manual test 全过

**Phase 5 结束**:
- [ ] MaterialBanner 组件 + banner auto-dismiss + "详情" 展开 inline
- [ ] validate_session_integrity 检测 Q6/Q7 状态并 push error
- [ ] Integration test：完整 welcome flow → 添加 provider → 创建 session → chat → 移除 provider → 启动报错
- [ ] E-01..E-05 manual test 全过
- [ ] 旧用户升级场景（app.json 已有 P0-B 3 占位）验证：进入主 UI + banner 提示 + 一键清理

---

---

## 7. 实现顺序（5 个阶段，每阶段可独立 review）

### Phase 1: 数据层 + State（无 UI 改动）

**Goal**: 把 `app_state/settings.rs` 全部实现 + manager.rs 改 B1
- 实现 `AppSettings` struct + 全部方法
- `load_or_default` + `save` (debounce)
- provider CRUD + test_provider（B3 真实调用）
- workspace CRUD
- skill 扫描 + enable state
- mcp server CRUD + test（F3）
- manager.rs 移除 P0-B seed
- Unit test: 每个方法独立测试
- **Commit**: `feat(settings): data layer`

### Phase 2: Slint SettingsView 主框架

**Goal**: 创建 `SettingsView.slint` + 左侧子菜单导航 + 路由切换
- 新建 `SettingsView.slint`
- main.slint 加 route 切换（main/welcome/settings）
- SidebarView 加 "设置" 菜单项 + cog 图标入口
- 各子面板占位（ProviderSettingsPanel 等先空）
- **Commit**: `feat(ui): SettingsView shell`

### Phase 3: g1 + g6 子面板

**Goal**: 实现 ProviderSettingsPanel + SkillsSettingsPanel + MCPSettingsPanel
- ProviderSettingsPanel: 列表 + 添加/编辑/删除 + 测试（B3）
- SkillsSettingsPanel: 列表（E3 一行）+ 全局 + per-workspace 开关（E2）
- MCPSettingsPanel: 列表 + 添加表单（F1+F2）+ 测试（F3）
- **Commit**: `feat(ui): provider/skill/mcp settings`

### Phase 4: g2 + g5 + Welcome

**Goal**: 实现 WorkspaceSettingsPanel + WelcomeView + IdentityCreatorView + Session chat 增强
- WorkspaceSettingsPanel: 列表（D4=b）+ 切换 + 添加（D1 OS dialog）
- WelcomeView: 3 步引导（G1=a）+ IDENTITY.md LLM 对话优化（D3）
- IdentityCreatorView: 5 轮 LLM 问答 + 实时预览
- ChatPaneView: 加 tool call 点击展开（C5=c）+ Esc 停止（C6=c）+ 导出按钮（C7=b）
- SidebarView: 加 session 操作（C1+C2+C3）+ workspace 切换器
- main.rs welcome 首启动判断
- **Commit**: `feat(ui): workspace + welcome + session chat enhancements`

### Phase 5: G2 双通道错误 + 集成测试

**Goal**: 错误 banner + inline 双通道 + 集成测试
- main.slint 加 inline error 区（G2=c 第二通道）
- AppState 加 set_inline_error 方法
- 集成测试：完整欢迎流程 + 添加 provider + 创建 session + chat 发送
- **Commit**: `feat(ui): dual error channels + integration tests`

---

## 8. 验收测试清单（G4 = a, Mavis 起草）

### 8.1 g1 Provider 测试场景

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| P-01 | 添加 anthropic provider | Settings → AI 服务 → 添加 → 填 name + 类型 + key + 模型 | 保存到 app.json；列表显示新条目 |
| P-02 | 添加 openai provider | 同上但选 openai | base_url 自动填 https://api.openai.com |
| P-03 | 添加 custom-openai-compatible provider | 类型选 custom-openai | base_url 字段可编辑（默认空） |
| P-04 | API key 错误 + 保存 | 填错 key → 测试连接 | 显示 "⚠️ 测试失败但已保存"；列表显示 ⚠️ |
| P-05 | API key 正确 + 测试 | 填正确 key → 测试 | 显示 ✓；provider 标记已验证 |
| P-06 | 重启 app | 关 + 开 | provider 列表保留（app.json 持久化） |
| P-07 | 编辑 provider | hover → 编辑 → 改 name → 保存 | 列表更新 |
| P-08 | 删除 provider | hover → 删除 | 列表移除；app.json 同步删除 |
| P-09 | 默认模型选择 | 设置 "默认模型" 下拉 | 新建 session 默认用此 model |
| P-10 | API key 字段 blur 保存 | 输入 key → 不点测试 → 切到其他字段 | 500ms 内（或 blur 时）保存 |

### 8.2 g2 Session chat 测试场景

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| S-01 | Sidebar + 按钮新建 session | 点 "+" 按钮 | 新 session 出现在列表，自动选中 |
| S-02 | App 启动自动建默认 session | 重启 app | 启动后默认 session 在 sidebar 选中 |
| S-03 | Welcome screen 首次启动 | 首次启动（或清空 app.json） | Welcome screen 显示，按 3 步走完后进入主 UI |
| S-04 | Session 列表显示 workspace tag | 创建 2 个 workspace，每个下建 session | sidebar 每个 session 显示所属 workspace |
| S-05 | Session hover 删除 | hover session 行 → X 图标 → 点 | session 移除，列表更新 |
| S-06 | Session inline 重命名 | 点 session name | 可编辑，输入新名后保存 |
| S-07 | 实时流式输出 | 发消息给 agent | 消息逐 token 追加，ChatMessageBubble 实时更新 |
| S-08 | Tool call inline 展开 | agent 调用 tool | inline 卡片显示，点击展开详情（CodeBlock） |
| S-09 | Stop 按钮 + Esc 键 | streaming 时点停止按钮 或 按 Esc | 流式中断，显示已收到的内容 |
| S-10 | Session 导出为 markdown | 点导出按钮 → 选保存位置 | markdown 文件保存，含完整对话 + tool call |
| S-11 | 多 session 切换 | 点击 sidebar 不同 session | ChatPaneView 内容切换 |
| S-12 | Session 持久化 | 关 app → 重开 | session 列表保留，可继续 |

### 8.3 g5 Workspace 测试场景

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| W-01 | Welcome screen 选 workspace | 首次启动 → step1 → 点 "选择文件夹" → OS dialog → 选目录 | 选中目录显示路径，可下一步 |
| W-02 | Settings 添加 workspace | Settings → 工作文件夹 → 添加 | OS dialog；选中后列表增加 |
| W-03 | 切换 workspace | sidebar workspace 切换器 → 选其他 | sidebar session 列表过滤为当前 workspace 的；当前 workspace 显示高亮 |
| W-04 | IDENTITY.md 自动生成 | 首次选 workspace | 检测无 IDENTITY.md → 自动创建模板（D3=a） |
| W-05 | IDENTITY.md LLM 对话优化 | 点 "为当前 workspace 创建/更新" → IdentityCreatorView | 5 轮问答后生成 IDENTITY.md 预览，可手动编辑后保存 |
| W-06 | 切换 workspace 后 IDENTITY.md 重载 | 切换 workspace | 新 workspace 的 IDENTITY.md 自动加载 |
| W-07 | Workspace 持久化 | 重启 app | workspace 列表 + 当前选中保留 |
| W-08 | 移除 workspace | Settings → 工作文件夹 → hover → 移除 | 列表移除；session 不删除但 workspace tag 清空 |

### 8.4 g6 Skills 测试场景

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| K-01 | Skill 列表显示 | Settings → 技能 | 显示所有 builtin_skills/ 下的 skill（24 个 + 任何新增） |
| K-02 | Skill 一行展示（E3=a） | 看 skill 行 | 只显示名字 + 一句话描述，无展开 |
| K-03 | 全局启用 skill | toggle 全局开关 | 全局状态变化，写入配置 |
| K-04 | per-workspace override | 在某 workspace 下 toggle override | 该 workspace 下启用状态独立 |
| K-05 | Skill 实际生效 | 启用某 skill → 在 session 中发触发该 skill 的消息 | skill 内容被加载到 system prompt |
| K-06 | 24 builtin 都能扫描 | 启动 app | builtin_skills/ 24 目录全部出现 |
| K-07 | Skill 状态持久化 | 重启 app | 全局 + workspace override 都保留 |

### 8.5 g6 MCP 测试场景

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| M-01 | MCP 添加（stdio） | Settings → 工具集 → 添加 → stdio + npx 命令 + env | 保存到配置 |
| M-02 | MCP 添加（sse） | 同上但 transport=sse + URL | 保存 |
| M-03 | MCP 添加（streamable-http） | 同上但 transport=streamable-http + URL | 保存 |
| M-04 | MCP 测试连接 | 添加后 → 测试 | 真实调 tools/list；成功显示工具列表；失败显示错误 |
| M-05 | MCP 失败但不阻断 | 填错命令 → 保存 + 测试 | 保存生效，列表显示 ⚠️ 状态；可后续修 |
| M-06 | MCP 在 session 中可用 | 添加 MCP server → 发消息触发工具 | agent 能调用该 MCP 提供的 tool |
| M-07 | MCP 持久化 | 重启 app | MCP server 配置保留 |
| M-08 | MCP 移除 | hover → 移除 | 列表移除；session 不再能调用该 server |

### 8.6 G2 双通道错误测试

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| E-01 | 全局错误 banner | 触发任意错误（如 provider test 失败） | 顶部 banner 显示错误信息 |
| E-02 | inline 错误（chat 区域） | 发送消息但 provider 配置缺失 | chat 输入框上方显示 "请先配置 AI 服务" |
| E-03 | 两个通道同时 | 同一错误触发两个通道 | 顶部 banner + inline 都显示，可独立 dismiss |
| E-04 | 错误自动消失 | 5 秒后 | banner 自动消失（已有 P0-C 行为） |
| E-05 | inline 错误手动关闭 | 点 × | 仅 inline 消失，banner 保留 |

### 8.7 G1 Welcome flow 测试

| # | 场景 | 步骤 | 预期 |
|---|---|---|---|
| WEL-01 | 首次启动显示 welcome | 清空 app.json + 启动 | WelcomeView 显示 step 1 |
| WEL-02 | Welcome step1 → step2 | step1 选 workspace → 下一步 | 切换到 step2 provider 配置 |
| WEL-03 | Welcome step2 → step3 | step2 配 provider → 下一步 | 切换到 step3（IDENTITY.md 可选） |
| WEL-04 | Welcome step2 跳过 | step2 点 "跳过" | 用空 provider 进入 step3（之后 session 无法发消息直到配置 provider） |
| WEL-05 | Welcome 完整流程 | 3 步都完成 | 进入主 UI，sidebar 显示 session |
| WEL-06 | LLM 对话优化 IDENTITY.md | step3 点 "想让我更懂你" → 5 轮问答 → 生成 → 保存 | IDENTITY.md 写入 workspace |

---

## 9. 风险与假设（v1.1 更新）

### 已记录风险

1. **P0-B 行为变更影响现有用户**（Q1=a 解决）: 移除 hardcoded 3 provider seed 后，旧 `~/.northhing/config/app.json` 里的 3 个占位仍在（enabled=false）。**Mitigation**: 静默保留，UI 加"清理已废弃 provider"按钮（banner 提示 + 按钮一键清理）。

2. **IDENTITY.md LLM 对话循环依赖**（Q3=c 解决）: D3 = a + LLM 对话意味着用户必须先有可用 provider。**Mitigation**: Welcome step2 强制不能跳过（"跳过"按钮 disabled + tooltip "配置 AI 服务才能继续"），保证进 step3 时必有 provider。

3. **Pure Slint 无 Tauri dialog**（E1 解决）: 本 spec 范围仅 desktop（mobile-web 用 `rfd` 也不可行，需要不同方案；web-ui 用 HTML input）。**Mitigation**: 本 spec 仅 desktop，mobile-web / web-ui 不在范围内。

4. **i18n 中文资源尚未就位**（Q11=b 解决）: G3=a 中文 only v1。**Mitigation**: Rust 侧 `AppStrings` const 集中定义所有 UI 文案（`src/apps/desktop/src/strings.rs`），Slint 引用 `text: AppStrings.WELCOME_TITLE;`。B 阶段抽 i18n 时只改 const 内容即可，不动 Slint 结构。

5. **Provider 删除后 in-use session 数据完整性**（Q6=a 解决）: 删除 provider 后，正在用该 provider 的 session 行为不确定。**Mitigation**: 允许删除，session 自动 fallback 到其他 enabled provider；启动时 `validate_session_integrity` 检测 "上次用已删 provider" → push 错误 banner + inline。

6. **Workspace 移除后 session 状态**（Q7=c 解决）: 移除 workspace 后，挂载该 workspace 的 session 怎么处理？**Mitigation**: session 保留但标记 `broken_workspace` 状态，UI 显示 ⚠️ + chat button disable + 输入框 disabled；需重新挂到 workspace 才能恢复。

7. **session 列表 workspace 过滤（Q5=a）**: D4 = sidebar workspace 切换 + C2 = session workspace tag 有— 突。**Mitigation**: Q5=a 锁定 — 切换 workspace 即时过滤 session 列表（仅显示当前 workspace 的 session）；其他 workspace 的 session 隐藏（不删除）。

8. **Per-session model override UI 位置（Q4=c）**: spec 提到 session 可改 model 但 UI 位置没说。**Mitigation**: Q4=c — session 设置入口在 ChatPaneView header "会话设置" 按钮 → 弹 settings 详情 tab。

9. **Skill 实际生效判断（Q12=a）**: 启用 skill 后怎么验证被加载？**Mitigation**: Q12=a — 用 inspector API（已存在 `inspector.rs`）看 agent system prompt 含 skill 内容。K-05 测试场景：启用 skill → 用 inspector 面板查看 → 确认 prompt 含 skill 段。

### 关键假设（v1.1 验证）

- ✅ A: 用户已有 `~/.northhing/config/app.json` 路径（manager.rs 确认）
- ✅ A: builtin_skills/ 目录结构稳定（24 个目录，per T2 catalog）
- ✅ A: 现有 MCP backend（cursor_format / auth / 3 transports）已可用（T2 catalog + T5 v2 fresh grep 验证）
- ✅ A: chat 流式输出已可用（ChatPaneView / ChatMessageBubble 已存在）
- ✅ A: AppState 已有 actor 模型（agent_runtime / agent-dispatch）
- ⚠️ **新增 A**: **`rfd` crate** 加到 Cargo.toml（E1）— Rust File Dialogs 跨平台，需要约 50 行 deps + 调用
- ⚠️ **新增 A**: **MaterialTextField 扩展 password 属性**（E2）— 内部 Slint TextInput 支持 `input-type: password`，加 5 行 Slint 代码
- ⚠️ **新增 A**: **Rust `strings.rs`**（Q11）— 加 const struct + impl，约 60 个 string + 1 个 slint-side wrapper 文件
- ⚠️ **新增 A**: **inspector API 暴露 skill 列表**（Q12）— 现有 `inspector.rs` 需要扩展 `get_current_skill_manifest()` 方法

---

## 10. 交付物清单

按 Phase 顺序（5 阶段，每阶段 1 commit）:

| Phase | Commit | 文件 | 验证 |
|---|---|---|---|
| 1 | `feat(settings): data layer` | 新增 app_state/settings.rs + 新增 strings.rs + 修改 manager.rs + 新增 rfd dep | cargo check + unit tests |
| 2 | `feat(ui): SettingsView shell` | 新增 SettingsView.slint + 扩展 MaterialTextField password + 修改 main.slint/SidebarView | cargo check + desktop 启动 |
| 3 | `feat(ui): provider/skill/mcp settings` | 3 子面板 + ProviderSettingsPanel "清理已废弃" 按钮 | manual test P-01..P-10 + K-01..K-07 + M-01..M-08 |
| 4 | `feat(ui): workspace + welcome + session chat enhancements` | WorkspaceSettingsPanel + WelcomeView（强制 step2） + IdentityCreatorView + ChatPaneView（per-session settings 入口 + Esc 内部） + SidebarView（Q5=a 过滤） | manual test W-01..W-08 + S-01..S-12 + WEL-01..WEL-06 |
| 5 | `feat(ui): dual error channels + integration tests` | Q8=c banner+inline 详情展开 + Q6/Q7 integrity validation + 集成测试 | manual test E-01..E-05 + 旧用户升级场景 |

**总计**（v1.1 更新）:
- 新增 5 个 Slint 文件（Views×3 + PasswordField + Slint-side strings wrapper）+ 2 个 Rust 文件（settings.rs + strings.rs）
- 修改 ~8 个现有文件
- 5 个 commit，每个独立 review
- **56 个 manual test scenario**（v1.1 typo 修正：原误写 41，实际 P-10 + S-12 + W-8 + K-7 + M-8 + E-5 + WEL-6 = 56）
- 期望完工时间：4-6 天（Mavis 实施，v1.1 比 v1.0 多 1 天因为要加 rfd + 扩展 MaterialTextField + strings.rs + inspector 扩展）

---

## 11. 完成后用户验收步骤

1. **启动 app**（cargo run -p northhing-desktop）
2. **首次启动体验**: 清空 `~/.northhing/config/app.json` → 重启 → 走完 welcome 3 步
3. **配 provider**: Settings → AI 服务 → 添加 → 填入实际可用 provider → 测试通过
4. **建 session**: sidebar + → 默认 workspace → 发消息 → 看流式输出
5. **试 tool call**: 发消息触发 agent tool 调用 → 看 inline 卡片 → 点击展开
6. **试 stop**: 流式时点停止 或 Esc → 中断
7. **导出**: 选 session → 导出 markdown → 保存到本地 → 打开看
8. **加 MCP**: Settings → 工具集 → 添加 stdio 类型（用现成 MCP server）→ 测试 → 在 session 中调用
9. **启 skill**: Settings → 技能 → 启用某 skill → 发消息看是否生效
10. **多 workspace**: 加第二个 workspace → 切换 → 建 session 验证 workspace tag
11. **重启**: 关 → 开 → 所有 state 持久化验证

**满足以上 = spec 验收通过，可进入下一阶段。**

---

## 12. 后续（B 阶段，超出本 spec）

- Web UI flow_chat 8 子模块复刻（M2/M3 路线图，原本）
- g3 subagent UI（如果你要暴露的话）
- g4 IDENTITY.md 富文本编辑器
- 英文 i18n（B 阶段加）
- Cloud / Background agents UI（如果有需求）
- northhing deep_review 等借鉴项（per T5 v2 借鉴清单）