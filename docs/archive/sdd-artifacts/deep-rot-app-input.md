# Deep Rot Review — app.rs + input.rs (2026-08-29)

> 量规: `deep-rot-review-rubric.md` · 前情: `rot-probe-2026-08-28.md`  
> 口径: 只读审查，不改代码不 commit；一切发现带 file:line 证据。

---

## 1. `src/apps/desktop/src/ui_dioxus/app.rs` — 959 行

**总判定：腐化中** · 与结构层初判"更纠结"一致

### 1.1 死代码 — 0 腐化证据, 1 观察项

- **观察**：非 Windows 的 `win_ops::close_os_window` (L80: `pub fn close_os_window(_hwnd: usize) {}`) 是无意义 no-op，但 `close_module`(L86)、`close_all_modules`(L94)、`quit_shell`(L101) 无条件调用；非 Windows 构建中 win_ops 模块产生结构性死重（函数体空转 3 次）。代码graph 确认全仓仅此文件引用。

### 1.2 重复 — 1 腐化证据

- **腐化** L37-54 `close_all_popups` 与 L58-98 `navigate_back` hide 段 (L62-77) 含完全相同的 PopupType→hide_method 11 字段映射：每次新增 popup 必须修改两处完全相同的 match，是经典 DRY 违反。`close_all_popups` (L54) 还需同步维护 `popup_stack.clear()`，`navigate_back` 需同步 re-show 分支。  
  代表案例: `PopupType::McpSelector => chat_view.hide_mcp_selector()` 在两个 match 中一字不差。

### 1.3 模式不一致 — 1 腐化证据, 1 观察项

- **腐化** L74 `.ok()` 在 `std::thread::Builder::new().name(...).spawn(...)` 上静默吞掉线程创建失败，无注释；vs L765 `let _ = window().new_window(dom, cfg)` 有 T7 注释解释。同一文件两次丢弃值，一处有理由一处无——审查者无法判断 L74 是有意 best-effort 还是遗漏。
- **观察** `sid.read().as_ref().map(|s| s == &session_id).unwrap_or(true)` 三处相同 fallback (L176/L184/L209)：session 未绑定时匹配所有事件的意图一致，但未提取为函数。

### 1.4 注释腐化 — 0 腐化证据, 1 观察项

- **观察** L5 "Mirrors the truth HTML consult-room-main.html body (LL275..L459)": 行号引用已通过 rg 验证(`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html` 仍存在)，注释未过时。但如果 HTML 结构变更，注释不保证同步更新。列入观察。

### 1.5 hack/绕路 — 1 腐化证据, 1 观察项

- **腐化** L684 `let scale = if scale > 0.0 { scale } else { 1.0 };` — 魔数式防御性代码，无 ponytail 注释标注 ceiling（何种 scale 为负值、是否应 fallback-to-1 是有意设计还是健壮性残留，不可判定）。
- **观察** L92 ponytail 注释已标注双重关闭路径冗余；L760 T7 注释已解释 `new_window` 结果丢弃。

### 1.6 职责归属错误 — 2 腐化证据

- **腐化** L876-931: `parse_hex_rgb`/`mix_hex`/`chronicle_gradient` 为纯 CSS linear-gradient 计算工具，rg 全仓确认仅此文件使用。同目录有 `css.rs`，颜色工具应属该模块而不应混放于 RSX 组件文件。
- **腐化** L37-103: `win_ops` unsafe FFI 模块 (37-96) + `close_module`/`close_all_modules`/`quit_shell` (83-103) = 67 行 OS 平台层窗口管理逻辑，属于 adapter/platform 职责（参照 AGENTS.md Layer 1 boundary rules），不应与 RSX 组件混放。

### 1.7 复杂度热点 — 2 腐化证据

- **腐化** `room_app_root` L106-642 = 536 行单函数（量规 80 行阈值的 6.7 倍），内部含 3 个 `use_future` 闭包 + 265 行 RSX 内联模板 (L376-641)。单组件承载了 session 初始化、事件通道处理、窗口注册、RSX 渲染、消息发送/停止、窗口操作七个职责。
- **腐化** `navigate_back` L62-96 = 35 行含 2 个 11-arm PopupType match（hide 11 + re-show 10 + InfoPopup 空臂），22 个分支分布在 35 行中，密度 0.63 分支/行。

### 1.8 测试质量 — 0 腐化证据, 1 观察项

- **观察** 3 个测试 (L933-959) 仅用 substring 断言（`assert!(grad.contains("35.00%"))`），无边界 case：非法 hex 输入 (`"#GGGGGG"`)、空历史、纯黑/纯白渐变极值。`mix_hex` 的 tolerance 路径（clamp 范围）未覆盖。

---

## 2. `src/apps/cli/src/modes/chat/input.rs` — 802 行

**总判定：腐化中** · ⚠️ 推翻结构层初判"持平"

结构层初判依据为零提交休眠（2026-07-12 snapshot 后 47 天）判断"持平"。代码审查结论：休眠 ≠ 健康。休眠期间未积累 bug 修复或功能变更，但未暴露的架构腐化在增长——"没修不等于没腐"。

### 2.1 死代码 — 0

- 无死代码。rg 确认 `block_in_place`/`handle_key_event`/`handle_non_key_event` 等标识符在 `src/apps/cli/src/` 中被引用。非 Windows `close_os_window` no-op 属于 app.rs 范畴。

### 2.2 重复 — 1 腐化证据

- **腐化** `block_in_place(|| rt_handle.block_on(async move { ... }))` async 桥接模式重复 7 处 (L121/L135/L156/L181/L444/L504/L606)——每次 crossterm 的同步事件处理需要调用异步 agent 方法时，完全相同的 boilerplate 被复制粘贴。提取为单个 `fn bridge<T>(&self, rt: &Handle, fut: impl Future<Output = T>) -> T` 方法可消除 7 处重复。

### 2.3 模式不一致 — 1 观察项

- **观察** `handle_key_event`(L101) 返回 `Result<Option<ChatExitReason>>`（递归式 API，递归调用 `handle_command` propagate error）；`apply_exit_reason`(L647) 通过 8 个 `&mut` 参数传递副作用（`this`, `chat_view`, `chat_state`, `session_id`, `rt_handle`, `should_quit`, `exit_reason`）；`handle_non_key_event`(L682) 返回 `Result<NonKeyEventOutcome>`。三种错误/副作用传递风格并存于 802 行文件。

### 2.4 注释腐化 — 0

- 无过期 TODO/FIXME/HACK。L428 crossterm bracketed paste workaround 有 inline 注释引用 issue #962。`reshow_info_popup`(popups.rs L19) 的 "kept for upcoming re-show UX" 注释说明未来功能意图，非墓碑。

### 2.5 hack/绕路 — 0 腐化证据, 1 观察项

- **观察** L428-436: Ctrl+V 直接调 `Clipboard::new().and_then(|mut cb| cb.get_text())` 绕过 crossterm bracketed paste (issue #962)，有 inline 注释，属有文档的 workaround。

### 2.6 职责归属错误 — 1 腐化证据

- **腐化** 802 行集中处理 11 个 popup（command_palette/model/agent/session/skill/subagent/MCP/MCP-add/provider/theme_config/info）的键盘事件分发逻辑；rg 确认 `any_popup_visible`/`close_all_popups`/`navigate_back` 仅在此文件定义，未下沉到 UI 模块。此文件是事件分发的"kitchen sink"，违反了分层边界——每个 popup 的 key handling 应归属各自 view 模块提供 `handle_key` trait method，而非在事件枢纽中 switch。

### 2.7 复杂度热点 — 2 腐化证据

- **腐化** `handle_key_event` L101-644 = 543 行（占文件 68%），单 match + 5 层 popup intercept 嵌套（permission → question → global popup → info → command palette → specific popup → catch-all）+ 30+ match 臂。全文件最大复杂度热点，也是修改风险最高的函数。
- **腐化** `handle_non_key_event` L682-801 = 119 行含 3 层 nested match（Event::Mouse → MouseEventKind → 具体动作），每个分支可能需要调用 `apply_exit_reason`（8 参数函数）。

### 2.8 测试质量 — 1 腐化证据

- **腐化** 0 测试、0 内联测试模块。802 行核心事件处理逻辑（30+ key bindings、7 种 popup 分发、鼠标手势、paste/resize）完全无覆盖。27 个 `KeyCode` 分支中每个绑定都可能是潜在的回归点，但没有任何自动化防线。

---

## 统计汇总

| 文件 | 腐化证据 | 观察项 | 判定 | 初判一致性 |
|---|---|---|---|---|
| `app.rs` (959L) | 7 | 4 | 腐化中 | ✅ 一致（"更纠结"确认） |
| `input.rs` (802L) | 5 | 3 | 腐化中 | ⚠️ 推翻（初判"持平"→实际"休眠中的健康隐患"） |
| **合计** | **12** | **7** | — | — |

### 推翻初判的证据

结构层初判 `input.rs` "持平"的依据是"单事件处理职责"且零提交。代码审查发现：零提交 = 休眠而非稳定。`handle_key_event` 543 行巨函数、7 处 `block_in_place` 复制、11-popup 事件分发未下沉、802 行零测试——这些是结构层行数/职责分析看不清的代码体内腐化。休眠 47 天意味着所有人都在回避这个文件（无人敢改），恰恰说明它已变成"超线但不可碰"的 god-file。
