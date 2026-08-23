# Task EF-E1 Archive 档案馆 实现报告

## 1. 改动文件列表

- 新增文件：`src/apps/desktop/src/ui_dioxus/pages_archive.rs` (456 行，实现独立 OS 窗 `archive`：8 层地层沉积流 + 3 张 W2.7 流体侧卡 + 渊中枢 + 轻 chrome)
- 接线文件：
  - `src/apps/desktop/src/ui_dioxus/mod.rs` (声明 `mod pages_archive;`)
  - `src/apps/desktop/src/ui_dioxus/registry.rs` (`DockSide` 增加 `Center`，注册 `archive` 插件 720×820 `DockSide::Center`)
  - `src/apps/desktop/src/ui_dioxus/windows.rs` (`mod win` 改为 `pub(crate) mod win` 暴露 `hide_and_close_hwnd`)
  - `src/apps/desktop/src/ui_dioxus/app.rs` (`spawn_module_window` 支持 `DockSide::Center` 居中计算；room 状态行增加 `id="nav-archive"` 文字链)
  - `src/apps/desktop/src/ui_dioxus/css.rs` (`OVERLAY_CSS` 追加 `body[data-window="archive"]` 独立覆盖样式与 `nav-archive` 样式，TRUTH_CSS 字节零修改)
  - `src/apps/desktop/src/ui_dioxus/i18n.rs` (增加 14 个 `ARCHIVE_*` / `NAV_ARCHIVE` 键)
  - `src/crates/assembly/core/locales/zh-CN.ftl` (新增 14 条词条)
  - `src/crates/assembly/core/locales/zh-TW.ftl` (新增 14 条词条)
  - `src/crates/assembly/core/locales/en-US.ftl` (新增 14 条词条)

## 2. 门禁验证证据

### 2.1 编译与单元测试
```bash
$ cargo check -p northhing
Finished `dev` profile [unoptimized + debuginfo] in 20.71s (exit 0)

$ cargo test -p northhing ui_dioxus
running 5 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s

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
- `src/apps/desktop/src/ui_dioxus/pages_archive.rs`: 456 行
- `src/apps/desktop/src/ui_dioxus/css.rs`: 591 行
- `src/apps/desktop/src/ui_dioxus/app.rs`: 603 行
- `src/apps/desktop/src/ui_dioxus/registry.rs`: 382 行
- `src/apps/desktop/src/ui_dioxus/i18n.rs`: 290 行

## 3. CDP 视觉取证与目验

### 3.1 截图路径
- Dark 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e1-archive-dark.png`
- Light 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e1-archive-light.png`
- 折叠态：`C:\WINDOWS\TEMP\opencode\t7-shots\e1-archive-folded-dark.png`

### 3.2 目验结论（Read 打开 3 张 PNG 逐项验证）
- **冷色基调**：`--mind-base` 严格采用深渊青 `#3F837B`（冷 abyss 色），无珊瑚 rep 暖色介入。
- **地层递降**：8 层 mock 沉积透明度自顶到底由 1.00 递降至 0.28；选中第一层时带有 3px abyss 边条高亮（`box-shadow: inset 3px 0 0 var(--mind-base)`）。
- **轻 chrome**：标题「档案馆」居左、▴ 收纳置中/右、主题切换钮与 ✕ 关窗钮成组，无冗余系统边框。
- **W2.7 折叠语法**：中枢折叠收缩为水平胶囊（印章「渊」26px + 名称行），左列卡片「档案状态 STRATA」「节气刻度 SOLAR」折起至标题栏且箭头变为 `▸`，未折叠卡片（「见证标记 WITNESS」）流体吃满剩余高度。标题 padding 18px 不贴边。

## 4. 约束与状态

- **Flags 还原**：取证完成后 `src/apps/desktop/src/flags.rs` 已还原为 `DIOXUS_SHELL = false`，`cargo test -p northhing flags` 全通。
- **无 Commit**：改动保留在工作区未 commit。
- **无走廊假按钮**：状态行仅实现 `nav-archive` 档案入口，走廊留待 E2 实现。
