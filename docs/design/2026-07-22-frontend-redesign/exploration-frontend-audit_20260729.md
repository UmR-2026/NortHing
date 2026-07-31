# 探索式前端审计报告 — FR-T3/T4 落地质量

> **审计日期**：2026-07-29 03:00 CST  
> **审计范围**：38 commit（bcbdd7c..11337ac），FR-T3（RedesignTheme 换绑）+ FR-T4（三栏→v2 单栏迁移）  
> **审计方式**：探索式 grep + 组件抽样阅读，非逐行审查  
> **审计人**：subagent（explore-frontend）

---

## 1. RedesignTheme 覆盖率

| 指标 | 数值 | 判定 |
|---|---|---|
| `MaterialTheme` 残留引用 | **0** | ✅ 完全清除 |
| `RedesignTheme` 引用总数 | **973** | ✅ 全面覆盖 |
| `import { MaterialTheme }` 行 | **0** | ✅ |
| `import ... theme.slint`（仅 struct 类型） | 11 处 | ⚠️ 可接受（仅取 MessageItem 等数据类型，非主题 token） |

**结论**：528→0 的换绑目标**已完成**。`theme.slint` 的 11 处 import 是为了 `SessionItem`、`MessageItem` 等 struct 定义，不是 MaterialTheme token 引用，可接受。但 `theme.slint` 文件本身仍存在（含旧 MaterialTheme global），属于死代码。

---

## 2. v2 视觉标志性元素（9 项逐查）

### 2.1 暗色皮肤翻转 ✅ 已落地

- `RedesignTheme.dark`（in-out property，默认 true）驱动 `t: dark ? DARK : LIGHT` 三元切换
- `main.slint` 中 `changed dark-mode => { RedesignTheme.dark = dark-mode; }` 接通
- `GeneralSettingsPanel.slint` 有 toggle UI（dark-mode 透传 + toggle-theme 回调）
- Rust 侧 `register_toggle_theme_callback` 已注册
- 亮/暗两套完整 token（LIGHT/DARK 各 50+ token），均由生成器产出

**判定**：production 实现，非 POC。

### 2.2 整屋空气染色（AirTint） ✅ 已落地

- `AirTint.slint`（1881 bytes）完整实现三层：底色平铺 + 顶晕径向 + 底部冷雾
- `speaking` 升档：`air-rep-speaking` / `halo-rep-speaking` token 差异化
- `cold` 模式（档案册）：`fog-abyss` / `halo-abyss` 切冷雾
- `main.slint` 中 AirTint 层 z 序最底，跨路由覆盖
- `cold: root.current-route == "archive"` 路由联动
- `animate background { duration: 300ms }` 过渡动画

**判定**：production 实现，完整落地。

### 2.3 编年史条（ChronicleBar） ✅ 已落地

- `ChronicleBar.slint`（633 bytes）实现 `@linear-gradient(90deg, birth 0%, rep-300 50%, now 100%)`
- `PresenceZone.slint` 中 `ChronicleBar { width: 140px; height: 4px; }` 实例化
- `PresenceBar.slint` 中也有实例化（但 PresenceBar 未在 main.slint 中使用 → 死代码）

**判定**：production 实现。

### 2.4 活跃轮竖线 + 面 ⚠️ 部分落地

- **竖线**：`TurnContainer.slint` 实现 `2.5px rep-400` 左缘竖线，`active` 状态触发 ✅
- **面**（4% rep 背景）：`turn-active` token 存在（亮态 `#F6F5F3`，暗态 `#201E1B`），TurnContainer 用 `RedesignTheme.t.turn-active` ✅
- **但**：设计稿要求活跃轮 `.msg` 字重 450（v1 400 → v2 450），ChatMessageBubble 中**未设置 font-weight: 450** ⚠️
- **msg 宽度 450px**：设计稿要求活跃轮消息 450px 宽，ChatPaneView 中未体现此差异化 ⚠️

**判定**：结构落地，文案字重/宽度差异化缺失。用户目验可能注意到活跃轮与沉积轮视觉差异不够明显。

### 2.5 头像呼吸光环（AvatarWrap） ✅ 已落地

- `AvatarWrap.slint`（2393 bytes）实现 `breath` 属性：`1 + 0.015 * Math.sin(animation-tick() / 6000ms * 360deg)`
- 光环 halo Rectangle 用 width/height 等比缩放（因 Slint 1.16 无 scale-x/y）
- `breathing: streaming` 绑定流式状态
- `PresenceZone` 中 `AvatarWrap { size: 64px; initial: "序"; breathing: streaming; }`

**判定**：production 实现，有 POC 验证背书（slint-feasibility-poc.md）。

### 2.6 心境语双动画（MoodText） ⚠️ 部分落地

- `MoodText.slint`（624 bytes）实现淡入动画：`opacity: 0 → init => { root.opacity = 0.9; }`，`animate opacity { duration: dur-once(1200ms) }`
- **但**：设计稿要求「延迟淡入 + 呼吸」，MoodText 只有淡入，**无呼吸动画** ⚠️
- 设计稿 CSS：`.p-mood{ animation: moodFade 1.2s ease-out forwards, moodBreathe 6s ease-in-out 1.2s infinite alternate }` — 第二段 `moodBreathe` 未实现

**判定**：淡入已落地，呼吸缺失。用户不太可能注意到（心境语本身是次级元素），但设计完整性有缺口。

### 2.7 Speaking 升档 ✅ 已落地

- 链路完整：`DeckBar.focused`（`ti.has-focus`）→ `ChatPaneView.input-focused` → `SpaceView.input-focused` → `main.slint: changed input-focused => { root.speaking = space.input-focused; }` → `AirTint.speaking`
- `AirTint` 中 speaking=true 时底色从 `air-rep`(3.5%) 升到 `air-rep-speaking`(4.5%)，顶晕从 `halo-rep`(7%) 升到 `halo-rep-speaking`(10%)
- 离开 main 路由时复位 `speaking = false`

**判定**：production 实现，链路完整。

### 2.8 自定义滚动条 ❌ 未落地

- grep `scrollbar`、`ScrollBar`、`scroll-bar`、`ScrollbarArea` → **0 结果**
- 设计稿 CSS 明确定义 `::-webkit-scrollbar` + `scrollbar-color` + `scrollbar-width: thin`
- Slint 1.16 的 Flickable **不原生支持自定义滚动条样式**（无 ::scrollbar 伪元素）
- 14 处 Flickable 使用，全部无滚动条视觉

**判定**：未落地。Slint 平台限制，需自定义滚动条组件或接受默认/无滚动条。用户目验时会注意到滚动时无视觉指示器（或默认丑陋滚动条）。

### 2.9 ::selection 染色 ❌ 未落地

- grep `selection-color`、`text-selection`、`::selection` → **0 结果**（仅 1 处注释提到 "single selection pattern"）
- 设计稿 CSS：`::selection{background:color-mix(in srgb,var(--rep-300) 40%,transparent);color:var(--fg)}`
- Slint 1.16 的 TextInput **不支持 selection-color 属性**

**判定**：未落地。Slint 平台限制。用户选中文字时将看到系统默认选中色（蓝色），与 v2 色系不符。

---

## 3. 组件完整性

### 3.1 FR-T4 新建组件清单

| 组件 | 文件 | 大小 | 状态 | 说明 |
|---|---|---|---|---|
| SpaceView | `views/SpaceView.slint` | 3565 B | ✅ 有内容 | 单栏骨架，PresenceZone + ChatPaneView 组装 |
| PresenceZone | `components/PresenceZone.slint` | 5749 B | ✅ 有内容 | 体温光晕 + 头像/名字/状态/编年史/心境语，含 p-state 计时 |
| DeckBar | `components/DeckBar.slint` | 12963 B | ✅ 有内容 | 输入区 + 控制行（⚙/模型/思考/工作目录/access/发送），send/stop 两态 |
| ArchiveView | `views/ArchiveView.slint` | 13696 B | ✅ 有内容 | 沉积剖面 + 时间线 + 会话卡片，abyss 冷系 |
| AirTint | `components/AirTint.slint` | 1881 B | ✅ 有内容 | 底色 + 顶晕 + 底雾三层 |
| WindowChrome | `components/WindowChrome.slint` | 6883 B | ✅ 有内容 | 水印 + 左右把手 + 窗口控制三键 |
| MoodText | `components/MoodText.slint` | 624 B | ✅ 有内容 | 淡入动画（缺呼吸） |
| TurnContainer | `components/TurnContainer.slint` | 1391 B | ✅ 有内容 | active/sedimented 状态 + 左缘竖线 |
| ThinkBlock | `components/ThinkBlock.slint` | 772 B | ✅ 有内容 | abyss-400 左缘 + 半透底 |
| ToolChip | `components/ToolChip.slint` | 1342 B | ✅ 有内容 | pill 形 running/done 两态 |

**空壳检查**：无空壳组件。最小的是 ThinkBlock（772 B）和 MoodText（624 B），但都有实际实现逻辑。

### 3.2 额外发现的新组件

| 组件 | 文件 | 大小 | 说明 |
|---|---|---|---|
| AvatarWrap | `components/AvatarWrap.slint` | 2393 B | 头像 + 呼吸光环 |
| ChronicleBar | `components/ChronicleBar.slint` | 633 B | 编年史渐变条 |
| InnerDrawer | `components/InnerDrawer.slint` | 5145 B | 左抽屉「内在」 |
| OuterDrawer | `components/OuterDrawer.slint` | 6785 B | 右抽屉「外物」 |
| PresenceBar | `components/PresenceBar.slint` | 2753 B | ⚠️ 死代码（未被 main.slint import） |

---

## 4. Slint 语法合规

### 4.1 非法属性

| 检查项 | 结果 | 判定 |
|---|---|---|
| `font-style`（Slint 不支持） | 0 处 | ✅ |
| `loop` 关键字（仅注释中出现 6 次） | 0 处实际使用 | ✅ |
| `font-weight` | 33 处 | ✅ 合法 Slint 属性 |

**commit 87b4217 已清除 Slint 非法属性**（`loop`/`font-style`），确认落地。

### 4.2 硬编码 hex 色值

| 文件 | 行数 | 色值 | 应替换为 |
|---|---|---|---|
| `CodeBlock.slint:9` | 1 | `#1E1E1E` | 应新增 token 或用 `abyss-500` |
| `CodeBlock.slint:24` | 1 | `#D4D4D4` | 应用 `muted` 或 `faint` |
| `ToolCallCard.slint:19` | 1 | `#2D2D2D` | 应用 `surface` 或 `elevated` |
| `ToolCallCard.slint:37` | 1 | `#4CAF50` | 应用 `abyss-400`（完成态绿）或新增 token |
| `ToolCallCard.slint:67` | 1 | `#1E1E1E` | 同 CodeBlock |
| `ToolCallCard.slint:74` | 1 | `#D4D4D4` | 同 CodeBlock |
| `SidebarView.slint:76` | 1 | `#00000060` | 应用 `scrim.with-alpha(0.5)` |
| `SidebarView.slint:150` | 1 | `#00000060` | 同上 |

**总计 8 处 hex 残留**，集中在 CodeBlock（2）、ToolCallCard（4）、SidebarView（2）。

**注意**：commit 9ad23e7 标题声称「hex/padding 清零」，但上述 8 处仍残留。其中 SidebarView 是死代码（未在 main.slint 中使用），CodeBlock 和 ToolCallCard 是活跃组件。

**判定**：⚠️ 活跃组件中 6 处 hex 残留。用户目验时不太可能直接注意到（都是代码块/工具卡片内部色），但违反「hex banned」纪律。

### 4.3 旧 Material 组件残留

| 旧组件 | 引用次数 | 说明 |
|---|---|---|
| MaterialButton | ~30 处 | 广泛用于设置页、Welcome、IdentityCreator |
| MaterialCard | ~15 处 | 设置页卡片 |
| MaterialTextField | ~6 处 | 设置页输入框 |
| MaterialIconButton | ~4 处 | ChatPaneView、InspectorView |
| MaterialList/MaterialBadge/MaterialBanner | ~3 处 | SidebarView（死代码）、MaterialBanner（活跃） |

**总计 ~144 处 Material* 组件引用**。这些旧组件已换绑到 RedesignTheme token（不再读 MaterialTheme），但组件名和结构仍是 Material Design 风格。

**判定**：⚠️ token 层面已换绑，但组件壳仍是 Material。FR-T5 计划中的 W1（设置壳重做）正是要解决此问题。

---

## 5. Rust-Slint 接线

### 5.1 窗口控制按钮（−□×） ✅ 已接线

```
Slint: WindowChrome.slint → callback window-minimize/maximize/close
  ↓ main.slint 转发
Rust: create_ui.rs:324-343 → ui.on_window_minimize/maximize/close
  ↓ slint::Window API
  set_minimized(true) / toggle is_maximized / hide()
```

**判定**：完整接线，无缺失。

### 5.2 主题切换 ✅ 已接线

```
Slint: GeneralSettingsPanel → callback toggle-theme
  ↓ SettingsView → main.slint
Rust: callbacks_lifecycle.rs:535 → ui.on_toggle_theme
  ↓ RedesignTheme.dark = !dark-mode
```

**判定**：完整接线。

### 5.3 Stop/Cancel 按钮 ✅ 已接线

```
Slint: DeckBar.slint → callback stop()
  ↓ ChatPaneView → SpaceView → main.slint
Rust: callbacks_lifecycle.rs:862 → register_stop_streaming_callback
  ↓ AppState streaming_lifecycle
```

**判定**：完整接线。

### 5.4 设置页面数据 ✅ 已接线

```
Slint: SettingsView → 5 子面板（General/Access/Provider/Skills/MCP/Workspace）
  ↓ main.slint 属性绑定
Rust: callbacks_settings/*.rs → register_*_callback 系列
  ↓ AppState settings 模块
```

**判定**：完整接线，数据双向流通。

### 5.5 未接线的回调 ⚠️

| 回调 | Slint 声明 | Rust 处理 | 说明 |
|---|---|---|---|
| `open-session-settings` | ✅ 5 处 | ❌ 无 `on_open_session_settings` | 点击后无反应（OuterDrawer 中有调用） |
| `export-markdown` | ✅ 4 处 | ❌ 无 `on_export_markdown` | InnerDrawer + ArchiveView 中有调用 |
| `open-archive` | ✅ main.slint 路由切换 | ✅ 纯 Slint | 不需要 Rust |

**判定**：`open-session-settings` 和 `export-markdown` 两个回调在 Slint 侧声明了但 Rust 侧未注册 handler。用户点击「会话设置→」或「导出 Markdown」时将无反应。

---

## 6. FR-T5 计划合理性评估

**计划文件**：`docs/superpowers/plans/2026-07-29-fr-t5-settings-drawers.md`（3974 bytes）

### 6.1 计划结构

| 工作包 | 任务数 | 核心内容 |
|---|---|---|
| W1 设置统一 | T5-1~T5-4 | 设置壳重做 + 工作文件夹页迁移 + 五页校订 + 收纳确认 |
| W2 抽屉外扩 | T5-5~T5-6 | 窗口真实变宽 POC + 全面铺开 |
| W3 右抽屉重做 | T5-7~T5-9 | 收摊 Skills/MCP + 外物空态 + deck `/` 调 skill |
| W4 杂项 | T5-10~T5-12 | 设置页 glyph 排查 + 降级项收尾 + onboarding 拍板 |

### 6.2 合理性评估

**✅ 合理的部分**：
1. **W1 优先级正确**：设置页仍是旧 Material 壳（144 处 Material* 引用），用户目验首当其冲
2. **W2 标注 POC 先行**：Slint 1.16 window API 能力边界确实未知（`set_inner_size` 动画），先 POC 验证再铺开是正确的
3. **W3 定位准确**：右抽屉当前装的是 Skills/MCP（应属设置），「外物」=生成物/浏览器/subagent worktree 的定位与用户拍板一致
4. **纪律沿用 FR-T4**：无 hex、padding 只挂 layout、Flickable 用 preferred-height
5. **选派合理**：W1 大页给 glm，W2 POC 给编排者，W4 机械单给 bp/mimo

**⚠️ 遗漏/风险**：
1. **未提及滚动条缺失**：本报告发现 Slint 不支持自定义滚动条样式，FR-T5 计划中无应对方案。建议：要么接受默认滚动条，要么自建 Scrollbar 组件叠在 Flickable 上
2. **未提及 ::selection 染色缺失**：同理 Slint 平台限制，计划中无应对
3. **未提及 MoodText 呼吸动画缺失**：第二段 `moodBreathe` 动画未实现，计划中无补单
4. **未提及活跃轮 msg 字重/宽度差异化**：设计稿要求 450 字重 + 450px 宽，当前缺失
5. **未提及 `open-session-settings` / `export-markdown` 回调未接线**：用户点击会无反应
6. **未提及 CodeBlock/ToolCallCard 中 6 处 hex 残留**：虽然 commit 声称清零
7. **未提及 PresenceBar / SidebarView / InspectorView 死代码清理**：三个组件未被 main.slint import，占 ~27KB

**判定**：计划大方向合理，但遗漏了若干本审计发现的问题。建议在 W4 杂项中追加：
- T5-13: 补全 MoodText 呼吸动画
- T5-14: 接线 `open-session-settings` + `export-markdown` 回调
- T5-15: 活跃轮 msg 字重/宽度差异化
- T5-16: 清理死代码（PresenceBar / SidebarView / InspectorView / theme.slint 旧 Material 组件）
- T5-17: 滚动条方案决策（接受默认 or 自建组件）

---

## 7. 用户目验时会注意到的视觉问题

按影响程度排序：

### 7.1 🔴 高影响（用户一眼会看到）

1. **设置页仍是旧 Material 壳**：nav 样式、卡片层次、底部「关闭」大按钮——与 v2 设计差距大。FR-T5 W1 已计划修复。
2. **滚动无视觉指示器**：消息流、设置页、档案册—all Flickable 都没有滚动条。长列表时用户不知道还能往下滚。
3. **右抽屉内容错位**：「外物」抽屉里装的是 Skills 列表 + MCP 状态 + 主题切换——用户期望的是生成物/浏览器。FR-T5 W3 已计划修复。

### 7.2 🟡 中影响（细看会注意到）

4. **文字选中色不对**：选中文字时是系统蓝色，不是 v2 代表色系。
5. **活跃轮与沉积轮差异不明显**：缺少 msg 字重 450 和宽度差异化，active 状态只有背景色和竖线。
6. **心境语不会呼吸**：只有淡入，没有持续的呼吸动画。
7. **设置页右上角有怪 glyph**（FR-T5 T5-10 已知）：可能是 win-ctrl 旁的 tofu 框。

### 7.3 🟢 低影响（很难注意到）

8. **CodeBlock/ToolCallCard 内部硬编码色**：`#1E1E1E` 等与 v2 token 体系脱节，但视觉上接近暗色态。
9. **InnerDrawer/OuterDrawer 用遮罩而非外扩**：FR-T5 W2 已计划改为窗口真实变宽。
10. **头像呼吸光环用尺寸缩放代替 scale**：视觉等效但性能略差（每帧重算 width/height）。

---

## 8. 总结

### 真正落地了（production）

- RedesignTheme 全面换绑（MaterialTheme 0 残留）
- AirTint 整屋空气染色 + speaking 升档（链路完整）
- 暗色皮肤翻转（token 三元 + Rust 接线）
- 头像呼吸光环（animation-tick 驱动）
- 编年史条（渐变 + token）
- PresenceZone 在场区（五元素垂直居中 + p-state 计时）
- TurnContainer 活跃轮竖线 + 沉积轮 hover
- ThinkBlock / ToolChip / DeckBar（v2 操控台）
- ArchiveView 档案册（abyss 冷系）
- WindowChrome 窗口控制（−□× 接 Rust）
- frameless 窗口（no-frame: true）
- 心境语淡入动画

### 只是声明落地（未真正完成）

- **自定义滚动条**：设计稿有 CSS，Slint 不支持，未实现也未替代
- **::selection 染色**：设计稿有 CSS，Slint 不支持，未实现
- **心境语呼吸动画**：只有淡入（第一段），无呼吸（第二段）
- **活跃轮 msg 字重 450 + 宽度 450px**：TurnContainer 有背景/竖线，但 msg 本身无差异化
- **hex 清零**：commit 声称清零，实际 CodeBlock/ToolCallCard 中 6 处残留
- **`open-session-settings` / `export-markdown` 回调**：Slint 侧声明了，Rust 侧无 handler

### 死代码（可清理）

- `PresenceBar.slint`（2753 B）— 被 PresenceZone 替代
- `SidebarView.slint`（20535 B）— 被 SpaceView + InnerDrawer 替代
- `InspectorView.slint`（3875 B）— 被 OuterDrawer 替代
- `theme.slint` 中 MaterialTheme global — 已无引用
- 旧 Material* 组件（MaterialButton/Card/TextField 等）— 仍在使用但应逐步被 v2 组件替代（FR-T5 W1）

---

*报告结束*
