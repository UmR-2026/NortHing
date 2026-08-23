# Task EF-E3 Settings 全局设置 实现报告

## 1. 改动文件列表

- 新增文件：`src/apps/desktop/src/ui_dioxus/pages_settings.rs` (348 行，实现独立 OS 窗 `settings`：双列哲学——左列「它的自我」只读 + 右列「设施」可点 mock 交互 + W2.7 独立卡片折叠与顶部轻 chrome)
- 接线与联动文件：
  - `src/apps/desktop/src/ui_dioxus/mod.rs` (声明 `mod pages_settings;`)
  - `src/apps/desktop/src/ui_dioxus/registry.rs` (注册 `settings` 插件 760×580 `DockSide::Center`，追加生命周期单测)
  - `src/apps/desktop/src/ui_dioxus/windows.rs` (将 `self_app_root` 与 `facility_app_root` 两处 `≡ 全局设置` 死按钮接上 `spawn_module_window_with_theme_rx("settings", ...)`，带 `stop_propagation`)
  - `src/apps/desktop/src/ui_dioxus/css.rs` (`OVERLAY_CSS` 追加 `body[data-window="settings"]` 双列流体卡片覆盖样式，增量 20 行，TRUTH_CSS 字节零修改)
  - `src/apps/desktop/src/ui_dioxus/i18n.rs` (增加 25 个 `SETTINGS_*` 键)
  - `src/crates/assembly/core/locales/zh-CN.ftl` (新增 25 条词条)
  - `src/crates/assembly/core/locales/zh-TW.ftl` (新增 25 条词条)
  - `src/crates/assembly/core/locales/en-US.ftl` (新增 25 条词条)

## 2. 门禁验证证据

### 2.1 编译与单元测试
```bash
$ cargo check -p northhing
Finished `dev` profile [unoptimized + debuginfo] in 25.13s (exit 0)

$ cargo test -p northhing ui_dioxus
running 7 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s

$ cargo test -p northhing flags
running 3 tests
test flags::tests::default_mode_id_is_agentic ... ok
test flags::tests::dioxus_shell_default_false ... ok
test flags::tests::session_tree_view_default_phase_c2 ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s
```

### 2.2 i18n 契约审计
```bash
$ pnpm run i18n:audit
> northhing@0.2.10 i18n:audit
> node scripts/i18n-audit.mjs

[i18n:audit] WARN mobile-web-source has 2 grandfathered CJK source candidate line(s). First entries: src/mobile-web/src/theme/ThemeProvider.tsx:69, src/mobile-web/src/theme/ThemeProvider.tsx:123
[i18n:audit] Passed with 1 warning(s).
```

### 2.3 代码行数约束 (<800 行)
- `src/apps/desktop/src/ui_dioxus/pages_settings.rs`: 348 行
- `src/apps/desktop/src/ui_dioxus/pages_space.rs`: 640 行
- `src/apps/desktop/src/ui_dioxus/pages_archive.rs`: 459 行
- `src/apps/desktop/src/ui_dioxus/css.rs`: 778 行 (增量 20 行 < 40 行)
- `src/apps/desktop/src/ui_dioxus/windows.rs`: 786 行
- `src/apps/desktop/src/ui_dioxus/app.rs`: 630 行
- `src/apps/desktop/src/ui_dioxus/registry.rs`: 470 行
- `src/apps/desktop/src/ui_dioxus/i18n.rs`: 332 行

## 3. CDP 视觉取证与目验

### 3.1 截图路径
- Dark 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e3-settings-dark.png`
- Light 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e3-settings-light.png`
- 折叠态：`C:\WINDOWS\TEMP\opencode\t7-shots\e3-settings-folded-dark.png`

### 3.2 目验结论（Read 打开 3 张 PNG 逐项验证）
- **双列哲学与视觉差**：左列「它的自我」严格呈现只读排版（`cursor: default`，# 标签与 meta 数据中性对齐，无控件交互指示），右列「设施」具备丰富的可点 mock 控件（单选 dot-radio、多选 sq-toggle、当前/未授权等状态高亮与重定位按钮），视觉边界清晰，非管理后台表格。
- **轻 chrome 契约**：标题「全局设置」居左，右侧为「▴ 收纳」、主题切换钮与「✕」关窗钮；无系统边框，`skip-taskbar` 独立居中呈现。
- **可折卡语法**：两列内所有卡片（如 SEDIMENT、CHRONICLES、IDENTITY、AXIOMS、ENGINE 等）均支持点击标题独立折叠至标题栏（箭头切换为 `▸`），展开卡片自动以 `flex: 1 1 auto` 流体填充；标题水平 padding 18px 保证列表对齐与不贴边。
- **双光学响应**：Light 态与 Dark 态均正常响应主题切换，变量体系与卡片背景阴影一致流转。

## 4. 约束与状态

- **Flags 还原**：取证完成后 `src/apps/desktop/src/flags.rs` 已还原为 `DIOXUS_SHELL = false`，`flags` 单元测试通过。
- **无 Commit**：所有 E1、E2、E3 改动均保留在工作区未 commit。
- **无光标劫持**：CDP 端口 9333 + Hidden 模式完成全部 DOM 触发与取证。
