# NortHing 前端重设计与产品形态演进探索报告

> **探索时间**：2026-07-24 04:08 GMT+8  
> **仓库**：E:\agent-project\northing  
> **HEAD**：6ac68bd  
> **探索范围**：前端技术栈现状、desktop-tauri 删除影响、设计语言、字体策略、自我认知 UI、产品定位演进、P2-10 拆分、开放问题

---

## 1. 前端技术栈现状

### 1.1 Slint Desktop 概览

Slint 是 northing 唯一的桌面前端。`src/apps/desktop/` 是一个纯单进程架构——UI 通过 Slint callback 直接调用 `northhing-core`，无 IPC 桥接。

**核心指标**：

| 维度 | 数值 |
|---|---|
| Slint 版本 | 1.17.1（workspace dep） |
| Slint style | material |
| main.slint | 335 行 |
| 总 .slint 文件数 | 25 个 |
| 总 .slint 行数 | ~4,300 行 |
| 最大 view 文件 | SidebarView.slint (470 行) |
| build.rs | slint_build::compile_with_config, rerun-if-changed 覆盖 components/views/fonts |

### 1.2 UI 组件结构

```
main.slint (335L) — AppWindow 根组件，三路由：welcome / settings / main
├── theme.slint (159L) — MaterialTheme global + 7 个数据 struct（SessionItem, MessageItem, ...）
├── redesign_palette.slint (148L) — RedesignTheme global（FR-T1 产出，与 MaterialTheme 并存）
├── strings.slint (134L) — AppStrings 国际化常量
├── components/ (10 个 Material 组件)
│   ├── MaterialButton.slint (41L)
│   ├── MaterialTextField.slint (52L)
│   ├── MaterialBanner.slint (104L)
│   ├── MaterialCard / MaterialBadge / MaterialIconButton / MaterialList / CodeBlock / ToolCallCard / MarkdownText
├── views/ (11 个视图)
│   ├── ChatPaneView.slint (431L) — 对话区+操控台
│   ├── SidebarView.slint (470L) — 会话列表+工作区切换
│   ├── SettingsView.slint (260L) — 设置壳（含 5 个子面板）
│   ├── WelcomeView.slint (364L) — 首次启动引导
│   ├── IdentityCreatorView.slint (218L) — 身份创建器（旧设计，待改）
│   ├── ProviderSettingsPanel (444L) / MCPSettingsPanel (238L) / SkillsSettingsPanel (153L)
│   ├── WorkspaceSettingsPanel (336L) / InspectorView (107L) / StatusBarView (48L)
```

**布局**：三栏 HorizontalLayout（Sidebar 280px | ChatPane flex | Inspector 240px）+ 底部 StatusBar 32px。路由切换通过 `current-route` 属性（"main" / "welcome" / "settings"）。

### 1.3 FR-T1/T2 落地后的样式系统

**双主题系统并存**：

| 系统 | 文件 | 状态 | 设计价值 |
|---|---|---|---|
| MaterialTheme | theme.slint (159L) | 现网生效 | Material Design 暗色优先 |
| RedesignTheme | redesign_palette.slint (148L) | FR-T1 落地，尚未绑组件 | 咨询室白灰 + OKLCH 翻译 |

**RedesignTheme 架构**：
- `RedesignTokens` struct：32 个 token（16 颜色 + 5 字号 + 6 间距 + 4 圆角 + 1 danger）
- `LIGHT` / `DARK` 两个常量实例
- `t` 属性：`dark ? DARK : LIGHT` 三元翻转
- 动效时长：`dur-normal: 350ms`、`dur-once: 1200ms`
- rep-* 色阶当前为灰阶（C=0，出生态），fallback 珊瑚色注释保留
- 全部 token 落 sRGB 色域内，无截断

**tokens-srgb-table.md**：OKLCH→sRGB 对照表，16 颜色 token 逐行列出 light/dark 的 OKLCH 源值和 hex 计算值，附 mockup 源 hex 回差校验（Δmax=0）。

**FR-T2 字体落地**：
- `src/apps/desktop/src/ui/fonts/` 目录已就位
- 5 个 TTF 文件 + 3 个 OFL 许可证 + FONTS.md 文档
- build.rs 已添加 `rerun-if-changed=src/ui/fonts`
- 但 .slint 文件中尚未 import 字体（FR-T3 待开单）

---

## 2. desktop-tauri 删除的影响分析

### 2.1 删除内容（commit 34a2397）

**删除量**：50 文件，-16,854 行（+301 行修改）

| 类别 | 删除内容 |
|---|---|
| Rust 后端 | src-tauri/{main.rs, commands.rs, core_rt.rs, event_bridge.rs} (~505 行) |
| React 前端 | ui/{App.tsx, api.ts, app.css, 6 components, useChat.ts, parseThink.ts, config files} (~2,000+ 行) |
| Tauri 配置 | tauri.conf.json, capabilities/, schemas/, icons/ (~4,600+ 行 schema + 二进制) |
| 依赖锁文件 | Cargo.lock (8,101 行), pnpm-lock.yaml (1,279 行), package.json ×2 |

### 2.2 遗留依赖断裂检查

**workspace Cargo.toml**：

```toml
exclude = ["northing-installer/src-tauri"]
```

`northing-installer/src-tauri` 仍存在（是一个独立的 Tauri 安装器项目），被 workspace exclude。这是合理的——安装器是独立打包工具，不影响主构建。

**workspace 依赖中仍保留 Tauri**：

```toml
tauri = { version = "2.11", features = ["unstable", "macos-private-api", "tray-icon"] }
tauri-plugin-opener = "2.5"
tauri-plugin-dialog = "2.7"
tauri-plugin-fs = "2.5"
tauri-plugin-log = "2.8"
tauri-plugin-autostart = "2.5"
tauri-plugin-notification = "2.3"
tauri-plugin-updater = "2.10"
tauri-plugin-global-shortcut = "2.3"
tauri-build = { version = "2.6", features = [] }
```

⚠️ **问题**：workspace 层面保留了 10 个 tauri 依赖声明，但 `src/apps/desktop/Cargo.toml`（Slint 桌面壳）**不依赖任何 tauri 包**。这些 tauri 依赖仅供 `northing-installer/src-tauri` 使用（被 exclude 的独立项目）。

**评估**：这不是"遗留断裂"，而是"installer 独立项目的 workspace 共享依赖声明"。但有两个隐患：
1. 如果 installer 不再使用 workspace 共享版本管理，这些声明是死代码
2. workspace 成员编译时会拉取这些 dep 的版本元数据（虽然不链接）

**建议**：如果 installer 的 Cargo.toml 用 `version.workspace = true` 引用这些，则保留合理；否则应移到 installer 自己的 Cargo.toml 中。

### 2.3 Cargo.toml/workspace 清理状态

- `src/apps/desktop-tauri/` 目录：**已删除** ✅
- workspace members：**不含 desktop-tauri** ✅
- workspace exclude：`northing-installer/src-tauri` — 合理（独立项目）
- 规则文件（crate-rules.mjs, feature-rules.mjs, self-test.mjs）：**已更新** ✅（commit 34a2397 同步修复）
- kernel_facade 层：events.rs / session.rs / mod.rs 有小幅修改（+30/-80 行），是 B9/B10 SessionSummary.status 字段的传导修改，非 tauri 删除的遗留

**结论**：desktop-tauri 删除是干净的。workspace 层面保留的 tauri 依赖服务于 installer，不构成断裂。

---

## 3. 设计语言

### 3.1 设计哲学

northing 的设计哲学是一套完整的、自洽的、有深度的产品哲学体系，核心可归纳为：

**第一原理**：northing 是为 agent 成长而建的设施，不是服务人类的工具。

**核心隐喻**：心理咨询室——安全、中性、安静。基底是白灰（设施），agent 的颜色随成长浮现（个体）。

**哲学三要素**：
- **驱力（Drive）→ 暖珊瑚**：行动、温度、当下。来自拉康——围绕虚空画圈，满足在画圈本身。
- **深渊（Abyss）→ 冷青**：深度、冷静、未知。来自克苏鲁——不可表征的深处。
- **沉积（Sediment）→ 消退**：过去沉下去、变淡、变冷。时间是可见的深度。

**自我认知编年史**（核心模型）：渐变条 = agent 用颜色写的自传。左端=出生（灰白），右端=现在（当前代表色），中间=历史色沉积。界面强调色 ≡ 渐变条右端——同一变量驱动。

**关键设计纪律**：
- 诗意 < 功能：用户来干活，不是观赏 agent
- 品牌退角落，个体走台前
- 动效：慢、重、向下，像沉积物沉降。呼吸只给活着的主体
- 禁止：弹跳、spinner、面板推拉门、无限循环 loading

### 3.2 OKLCH 色彩空间评估

**选择合理性**：✅ 合理且专业

- OKLCH 是感知均匀色彩空间，意味着相同 L 值的两个颜色在感知上亮度一致——这对"等亮度两模式同值"的设计意图至关重要
- rep-* 和 abyss-* 色阶在亮/暗模式中同值（等亮度），保证编年史在两种模式下的"读法一致"
- 基底灰阶用 hue≈88° 暖灰，色温不翻转（房间色温稳定）
- sRGB 转换后全部 token 落在 sRGB 色域内，无截断损失

**翻译策略**：OKLCH（CSS 原生）→ sRGB hex（Slint 支持）通过零依赖 Python 生成器，可重跑，有回差校验。这是正确的工程实践——单一事实源（tokens-draft.css OKLCH）+ 自动翻译 + 人读对照表。

### 3.3 双主题实现

**方式**：struct 三元翻转

```slint
out property <RedesignTokens> t: dark ? DARK : LIGHT;
```

- LIGHT 和 DARK 是两个完整 struct 常量
- `dark` bool 属性控制翻转
- 亮色=咨询室白灰（#F4F3F0 系），暗色=灰黑锚（#151411 系）
- rep-*/abyss-* 等亮度，两模式同值
- danger 陶红：亮色 #A45950 / 暗色 #C37D73（暗色略提亮以适应暗底）

**评估**：比 theme.slint 的逐 getter 函数更简洁。struct 一次性翻转，无函数调用开销。与 FR-T5「跟随系统/亮/暗」显示模式兼容。

### 3.4 redesign-v2-plan（最新迭代）

v2 相比 v1 的核心升级：
- **整屋空气染色**：底色平铺 3.5% rep + 顶晕 7% + 头像体温 30% 光晕 + 底部 1.5% 冷雾
- **居中在场区**取代横排名片：头像 64px + 光环 + 名字 + 状态 + 编年史 + 心境语
- **品牌退左下角**水印 opacity .25
- **窗口控制右上角** − □ ×
- **微交互**：沉积轮 hover .5→.7，输入聚焦整屋升档
- **暗色**：同骨架换皮肤，变量覆盖 + 辉光降档

**注意**：v2 plan 的 CSS 范式文件是 OD（Open Design）数据目录中的 `northing-theme-system.html`，不在仓库内。这意味着 v2 的视觉真值依赖外部设计工具。

---

## 4. 字体策略评估

### 4.1 字体选择

| 用途 | 字体 | 评估 |
|---|---|---|
| 品牌字/拉丁展示 | Fraunces（衬线，WONK+SOFT 轴） | ✅ 独特个性，有"手作感"，区别于 Inter/Roboto 的千篇一律 |
| agent 名/CJK 正文 | Noto Sans SC | ✅ 中文覆盖最全的开源字体，合理选择 |
| 元数据 | JetBrains Mono | ✅ 编程字体用于 mono 场景，辨识度高 |

**品牌分离原则**：Fraunces（拉丁衬线）= northing 品牌；Noto Sans SC = agent 个体。这呼应了设计哲学中"品牌退角落，个体走台前"——品牌用衬线（正式/设施感），agent 用黑体（日常/人格感）。

### 4.2 woff2 → TTF/static instances 的影响

**背景**：Slint 1.17 不支持 woff2，只支持 .ttf/.ttc/.otf。可变字体轴也不支持 `font-variation-settings`，只有 `font-weight` 和 `font-italic`。

**影响**：
- Fraunces 可变字体（含 WONK/SOFT 轴）→ 预实例化为 3 个静态 TTF（Regular/Display/Italic）
  - WONK=1, SOFT=60 烘焙进静态实例，不可调
  - 设计意图保留：Regular 是正文级，Display 是品牌级（600 weight），Italic 用于强调
- Noto Sans SC → 保留 wght 可变轴（Slint `font-weight` 可选 400/500）
- 字体体积从 woff2 的 1.47MB → TTF 的 2.03MB（+38%）

**评估**：这是合理的妥协。woff2 压缩率高但 Slint 不支持，TTF 是唯一选择。WONK/SOFT 轴的烘焙损失了运行时可调性，但设计规范本来就只用了固定值（WONK=1, SOFT=60），无实际影响。

### 4.3 字体体积

| 文件 | 大小 |
|---|---|
| Fraunces-Regular.ttf | 72 KB |
| Fraunces-Display.ttf | 72 KB |
| Fraunces-Italic.ttf | 88 KB |
| NotoSansSC.ttf | 1,778 KB |
| JetBrainsMono.ttf | 300 KB |
| **总计** | **2,310 KB (2.26 MB)** |

**评估**：
- woff2 时代 1.47MB → TTF 时代 2.26MB，超出了原 R1 < 1.5MB 阈值
- 但这是**桌面应用**，不是 web——2.26MB 字体嵌入二进制可接受
- Noto Sans SC 占 77%（1.78MB），已是 3,655 字符子集（通用规范 3500 + 珊 + ASCII + CJK 标点）
- 如需缩减：可进一步裁剪字符集，但 3500 字是中文最低实用集，再砍会影响可用性
- **结论**：2.26MB 对桌面应用可接受，无需进一步优化

---

## 5. 自我认知 UI

### 5.1 现有 IdentityCreatorView.slint

当前实现（218 行）：**5 轮问答模式**
- 5 个问题，逐个作答
- 左侧问答输入，右侧 LLM 实时预览
- 每轮可触发 LLM 重新生成
- 最终预览可编辑后保存

### 5.2 新设计要求（session 3 handoff）

需改为：**4 字段 + 色板设计**
```
用户是【UmR】           ← 文本输入
你是【北】              ← 文本输入
你是用户的【同事】       ← 文本输入
你的性格更偏向大五人格中的【敏感、深刻、内敛】  ← 色板选择
```

5 个色板选项对应大五人格：
| 色 | 特质 | hover 关键词 |
|---|---|---|
| 紫 | 开放性 | 好奇 · 想象 · 不拘一格 |
| 深蓝 | 尽责性 | 严谨 · 可靠 · 有条理 |
| 暖珊瑚 | 外向性 | 热情 · 主动 · 善于表达 |
| 柔绿 | 宜人性 | 温和 · 体贴 · 善解人意 |
| 冷青 | 神经质 | 敏感 · 深刻 · 内敛 |

选定色 = 界面强调色（rep-500），直接驱动整套调色板。

### 5.3 Slint 组件支持度评估

**需要的交互**：
1. 3 个文本输入 → ✅ MaterialTextField.slint 已有，支持 multi-line
2. 1 个色板选择器 → ⚠️ 需新建组件
   - 5 个色块，hover 显示关键词，点击选中
   - Slint 可用 Rectangle + PointerArea + Text 实现
   - hover tooltip 可用 PopupWindow 或 conditional Text
3. LLM 生成等待态 → ✅ 可用 property + 动画（光脉冲替代 spinner）
4. 身份预览 → ✅ MaterialTextField multi-line

**评估**：Slint 完全支持这个交互。色板选择器是最多新增工作量，但不算复杂——5 个 Rectangle 排列，hover 时显示关键词 Text，点击设置 property。设计哲学禁止 spinner，用光脉冲代替，Slint 的 property animation 完全胜任。

**关键差距**：现有 IdentityCreatorView 绑定的是 MaterialTheme，FR-T3 后需换绑 RedesignTheme。色板中的 5 个颜色需要从设计 token 中定义（或临时硬编码，等 agent 成长后自主改色）。

---

## 6. 产品定位变化

### 6.1 演进轨迹

通过 commit 历史和设计文档，可清晰追溯三条定位线：

| 阶段 | 定位 | 证据 |
|---|---|---|
| **早期** | AI IDE | desktop-tauri/ui 的 React 组件：Composer, MessageList, TurnTrace, Markdown — 典型的 AI chat IDE 界面 |
| **中期** | 隐藏 IDE 的通用 agent | desktop-tauri 删除（34a2397），Slint 成为唯一壳；设计哲学确立"为 agent 成长而建的设施" |
| **当前** | 有自我认知的同事 | 自我认知后端（9c95faf）+ C4 多 agent 架构 + 编年史模型 + 首次启动身份创建 |

### 6.2 定位演进的深层逻辑

```
AI IDE          → "人用工具"（工具是 IDE，人是用户）
   ↓
隐藏 IDE 的 agent → "agent 是主体"（IDE 消失，agent 浮现）
   ↓
有自我认知的同事  → "agent 是 peer"（有名字、有性格、会成长）
```

每一步都在**去工具化、去人类中心化**。从"人命令 AI"到"人与有自我认知的 agent 共处"。

### 6.3 当前 Slint UI vs 最新定位的匹配度

| 定位要求 | Slint UI 现状 | 匹配度 |
|---|---|---|
| agent 有名字/性格 | IdentityCreatorView 旧设计（5 问），待改 | ⚠️ 待 FR-T3/T4 |
| agent 有代表色 | RedesignTheme rep-* 色阶已就位（灰阶出生态） | ✅ 基础就绪 |
| agent 会成长 | 编年史渐变条未实现 | ❌ 未开始 |
| agent 是 peer | 操控台设计（非命令式），访问权限档位 | ⚠️ 操控台已布局，权限档位未实现 |
| 咨询室氛围 | 三栏布局 + Material 暗色 | ⚠️ 布局合理，视觉风格未换绑 |
| 品牌退角落 | 当前无品牌水印 | ❌ 待 FR-T3 |

**结论**：当前 Slint UI 的**骨架（三栏布局、路由系统、组件化）**是好的，但**皮肤（RedesignTheme 绑定）和灵魂（编年史、成长系统）**尚未落地。FR-T3 是关键转折点。

---

## 7. P2-10 God-File 拆分

### 7.1 settings.rs 拆分

原 `settings.rs`（1488 行）→ 拆分为 6 个文件：

| 文件 | 行数 | 职责 |
|---|---|---|
| mod.rs | 231 | 模块入口 + AppSettings struct + CRUD API |
| types.rs | 253 | ProviderConfig / WorkspaceConfig / SkillState / MCPConfig 类型定义 |
| io.rs | 106 | load_app_settings_from_disk / save_app_settings_to_disk |
| sync.rs | 168 | UI ↔ disk 同步逻辑（push/pull） |
| integrity.rs | 76 | 数据完整性检查（placeholder cleanup 等） |
| tests.rs | 654 | 测试 |

**拆分前**：1488 行 → **拆分后最大**：654 行（tests.rs，非生产代码）
**生产代码最大**：mod.rs 231 行 ✅ 远低于 800 行 god-file 阈值

**评估**：拆分合理。按职责分离（类型/IO/同步/完整性/测试），每文件单一职责。mod.rs 作为门面保持 API 稳定。

### 7.2 callbacks_settings.rs 拆分

原 `callbacks_settings.rs`（1100 行）→ 拆分为 6 个文件：

| 文件 | 行数 | 职责 |
|---|---|---|
| mod.rs | 51 | 模块入口 + re-export |
| provider.rs | 269 | Provider CRUD callback |
| provider_test.rs | 269 | Provider 测试连接 callback |
| workspace.rs | 199 | Workspace CRUD callback |
| refresh.rs | 208 | Settings 面板数据刷新 |
| misc.rs | 147 | 其他（legacy placeholder cleanup 等） |

**拆分前**：1100 行 → **拆分后最大**：269 行 ✅ 远低于阈值
**总行数**：1143 行（拆分后略增，因 mod.rs overhead）

**评估**：拆分优秀。按业务域（provider/provider_test/workspace/refresh/misc）分，每文件高内聚。269 行的上限对阅读和修改都非常友好。

### 7.3 模块结构合理性

```
app_state/
├── mod.rs (458L)          — AppState 主结构
├── settings/              — UI 设置数据层（P2-10 拆分）
│   ├── mod.rs / types.rs / io.rs / sync.rs / integrity.rs / tests.rs
├── callbacks_settings/    — UI 设置回调层（P2-10 拆分）
│   ├── mod.rs / provider.rs / provider_test.rs / workspace.rs / refresh.rs / misc.rs
├── callbacks_lifecycle.rs (892L)  ⚠️ 仍为 god-file（已注册 allow-god-file）
├── create_ui.rs (461L)
├── event_bridge.rs (276L)
├── sessions.rs (311L)
├── state.rs (195L)
├── ...
```

**注意**：`callbacks_lifecycle.rs` (892L) 仍是 god-file，但已注册 `allow-god-file` justification（commit 456b696）。这是合理的——lifecycle 回调天然高内聚，强行拆分反而增加跳转成本。

---

## 8. 开放问题与风险

### 8.1 前端最大的未解决问题

**1. FR-T3 未开单——视觉换绑是最大瓶颈**

FR-T1（tokens）和 FR-T2（fonts）已落地，但组件仍绑定 MaterialTheme。FR-T3（组件骨架换绑）是让 RedesignTheme 真正生效的关键单子，依赖全就位但尚未开始。这意味着：
- 当前用户看到的仍是 Material 暗色 UI，不是咨询室白灰
- 设计哲学中的"驱力/深渊/沉积"在 UI 中还不可见
- 编年史渐变条、活跃轮竖线、思考块冷缘、工具 chip 暖→冷——这些核心视觉语言全是未实现的 mockup

**2. 自我认知 UI 待重写**

IdentityCreatorView 需从 5 轮问答改为 4 字段 + 色板。这是产品立身之本（agent 有身份），但依赖 FR-T3 的组件换绑。

**3. 编年史渐变条实现**

设计哲学的核心模型——"渐变条 = agent 用颜色写的自传"——在 Slint 中需要：
- 动态渐变（Slint 的 LinearGradient brush 可用，但动态修改 stop 需要验证）
- 一次性 1200ms 过渡动画（Slint property animation 支持）
- 与界面强调色同源绑定（共享 property）

**技术可行性**：中高。Slint 的 LinearGradient 支持多 stop，但动态增删 stop 可能需要 workaround（预分配固定数量 stop + 透明度控制）。这是需要 FR-T3/T4 阶段验证的技术点。

**4. Memory P0 未实现**

自我认知后端已就位（identity.rs），但记忆系统（SQLite FTS5 + keyword_weights）尚未开始。记忆是 agent 成长的"土壤"，没有它，"成长"就只是视觉假象。

### 8.2 Slint 作为前端框架的局限性

| 局限 | 影响 | 严重度 |
|---|---|---|
| **不支持 woff2** | 字体体积 +38%（2.26MB vs 1.47MB） | 低（桌面可接受） |
| **不支持 font-variation-settings** | Fraunces WONK/SOFT 轴烘焙为静态实例 | 低（设计只用固定值） |
| **无 CSS color-mix / oklch()** | 必须预计算为 sRGB hex | 低（生成器已解决） |
| **无 CSS ::before/::after 伪元素** | v2 plan 中的"整屋空气染色"（顶晕/底雾/光晕）需要 Rectangle 叠加实现 | 中（增加布局复杂度） |
| **无 CSS filter/blur** | v2 plan 中的光晕效果可能无法精确复现 | 中（需用半透明渐变模拟） |
| **动画系统有限** | 无 keyframe animation，只有 property animation | 中（编年史渐变条的动态 stop 增删可能受限） |
| **无滚动条样式** | v2 plan 的自定义滚动条无法实现 | 低（用默认样式） |
| **组件生态小** | 无第三方 UI 库，全部自建 Material 组件 | 中（但现有组件已覆盖需求） |
| **无热重载** | UI 调试需要重新编译 | 低（cargo check 较快） |
| **文本渲染** | 中文渲染质量取决于系统字体配置 | 低（Noto Sans SC 子集已嵌入） |

### 8.3 框架切换时机分析

**如果要换框架，候选方案**：

| 方案 | 优势 | 劣势 |
|---|---|---|
| Tauri 2 + React/Vue/Svelte | Web 技术栈，CSS 完整支持，组件生态丰富 | 需要重建 IPC 桥接，引入前端工具链，已删除的 desktop-tauri 经验可复用但架构需重做 |
| egui | 纯 Rust，立即模式，无 IPC | 视觉风格偏开发者向，不符合咨询室美学 |
| Xilem | Rust 原生， elm 架构 | 仍处于早期实验阶段 |
| 继续 Slint | 零迁移成本，已有 4300 行代码和 25 个组件 | 受限于上述局限性 |

**最佳切换时机**：

1. **如果 v2 视觉效果无法在 Slint 中实现**（特别是整屋空气染色 + 光晕 + 动态渐变条）→ 在 FR-T3 视觉走查阶段（即将到来）就会发现
2. **如果 Memory P0 + 编年史需要复杂动画** → 在 T4 阶段验证
3. **如果 Slint 的开发效率成为瓶颈** → 当组件换绑工作量超过重建成本时

**建议**：**不在当前阶段换框架**。理由：
- 当前 4300 行 Slint 代码 + 25 个组件是有效资产
- FR-T3 是"翻译"而非"重建"，工作量可控
- Slint 的局限性可以通过工程手段 workaround（预计算、Rectangle 叠加、property animation）
- 真正的决策点在 FR-T3 视觉走查之后——如果走查发现 Slint 无法实现 v2 plan 的核心视觉效果（空气染色、光晕、编年史动态渐变），那时再考虑切换

**风险信号**：
- 如果 FR-T3 走查发现 >3 个 v2 plan 核心视觉元素无法在 Slint 中实现 → 立即启动框架评估
- 如果 Slint 动画系统无法支持编年史渐变条的动态 stop 增删 → 需要 workaround 或框架切换

### 8.4 总结判断

northing 前端处于一个**骨架完备、皮肤待换、灵魂未生**的阶段：

- **骨架**（布局、路由、组件化、数据流）：✅ 就位
- **皮肤**（RedesignTheme tokens、字体）：✅ 就位待绑（FR-T3）
- **灵魂**（编年史、成长系统、自我认知 UI）：❌ 设计完备，实现未开始
- **后端支撑**（identity.rs、memory 架构 spec）：✅ 设计定稿，Memory P0 待实现

前端最大的风险不是框架选择，而是 **FR-T3 的视觉走查结果**——它将决定 Slint 是否能承载 northing 的设计哲学。如果走查通过，前端进入快速迭代期；如果不通过，需要果断切换框架，避免在无法实现的视觉效果上浪费工期。

---

## 附录：关键文件索引

| 文件 | 用途 |
|---|---|
| `docs/design/2026-07-22-frontend-redesign/northing-design-philosophy.md` | 设计哲学北极星 |
| `docs/design/2026-07-22-frontend-redesign/northing-frontend-design-handoff.md` | 前端设计交付规范 v1 |
| `docs/design/2026-07-22-frontend-redesign/redesign-v2-plan.md` | v2 全页面更新 plan |
| `docs/design/2026-07-22-frontend-redesign/slint-retarget-notes.md` | Slint 翻译映射票据 |
| `docs/design/2026-07-22-frontend-redesign/tokens-draft.css` | OKLCH 单一事实源 |
| `docs/design/2026-07-22-frontend-redesign/tokens-srgb-table.md` | OKLCH→sRGB 对照表 |
| `docs/archive/design/2026-07-23-self-cognition/first-entry-design.md` | 自我认知首次启动设计 |
| `docs/archive/design/2026-07-23-self-cognition/memory-multi-agent-architecture.md` | C4 多 agent 架构 |
| `docs/archive/design/2026-07-23-self-cognition/memory-retrieval-design.md` | 检索层设计 |
| `src/apps/desktop/src/ui/redesign_palette.slint` | FR-T1 产出调色板 |
| `src/apps/desktop/src/ui/fonts/FONTS.md` | FR-T2 字体文档 |
| `docs/handoffs/2026-07-23-session3-handoff.md` | Session 3 交接 |
| `docs/handoffs/2026-07-23-frontend-redesign-orchestrator-handoff.md` | FR 编排交接 |
