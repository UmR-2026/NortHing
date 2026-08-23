# Task EF-E2 Space 走廊 实现报告

## 1. 改动文件列表

- 新增文件：`src/apps/desktop/src/ui_dioxus/pages_space.rs` (640 行，实现独立 OS 窗 `space`：亮门/暗门/沉门三态流 + 左右可折侧卡 ORDER/WORKSPACE/DISPLAY 与 门缝所见 PEEK + 走廊中枢 + 轻 chrome + 底部开房操控台)
- 接线与联动文件：
  - `src/apps/desktop/src/ui_dioxus/mod.rs` (声明 `mod pages_space;`)
  - `src/apps/desktop/src/ui_dioxus/registry.rs` (注册 `space` 插件 760×820 `DockSide::Center`，追加生命周期单测)
  - `src/apps/desktop/src/ui_dioxus/app.rs` (暴露 `spawn_module_window_with_theme_rx`；room 状态行增加 `id="nav-space"` 文字链)
  - `src/apps/desktop/src/ui_dioxus/css.rs` (`OVERLAY_CSS` 追加 `body[data-window="space"]` 独立覆盖样式与 `nav-space` 样式，TRUTH_CSS 字节零修改)
  - `src/apps/desktop/src/ui_dioxus/i18n.rs` (增加 17 个 `SPACE_*` / `NAV_SPACE` 键)
  - `src/crates/assembly/core/locales/zh-CN.ftl` (新增 17 条词条)
  - `src/crates/assembly/core/locales/zh-TW.ftl` (新增 17 条词条)
  - `src/crates/assembly/core/locales/en-US.ftl` (新增 17 条词条)

## 2. 门禁验证证据

### 2.1 编译与单元测试
```bash
$ cargo check -p northhing
Finished `dev` profile [unoptimized + debuginfo] (exit 0)

$ & "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing ui_dioxus
running 6 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s

$ & "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing flags
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
- `src/apps/desktop/src/ui_dioxus/pages_space.rs`: 640 行
- `src/apps/desktop/src/ui_dioxus/pages_archive.rs`: 459 行
- `src/apps/desktop/src/ui_dioxus/css.rs`: 758 行
- `src/apps/desktop/src/ui_dioxus/app.rs`: 630 行
- `src/apps/desktop/src/ui_dioxus/registry.rs`: 437 行
- `src/apps/desktop/src/ui_dioxus/i18n.rs`: 307 行
- `src/apps/desktop/src/ui_dioxus/windows.rs`: 758 行

## 3. CDP 视觉取证与目验

### 3.1 截图路径
- Dark 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e2-space-dark.png`
- Light 态：`C:\WINDOWS\TEMP\opencode\t7-shots\e2-space-light.png`
- 折叠态：`C:\WINDOWS\TEMP\opencode\t7-shots\e2-space-folded-dark.png`

### 3.2 目验结论（Read 打开 3 张 PNG 逐项验证）
- **亮门独占**：`诊室 03 · 此刻 · 门开着` 独占 `--mind-base` 珊瑚暖光（`#C8714C`），门灯「序」具有径向光晕与呼吸动效（`breath-avatar`），右侧 peek 区域与亮门状态深度同步。
- **熄灯中性**：`诊室 02`、`诊室 01`、`诊室 00` 呈现中性冷灰与「◦」门灯，无光晕、无呼吸、无暖色侵染。
- **沉门更淡**：沉积层向下延伸（`关于服从的争论`、`未完成的隔离沙盒`、`第一次断电`），采用虚线门灯「·」且透明度自 0.72 递降至 0.36，只读不可点亮，底部配有 `档案馆 · 去看沉下去的门 ↗` 连通按钮。
- **可折卡**：左侧 3 张卡片（`ORDER` / `WORKSPACE` / `DISPLAY`）与右侧 `PEEK` 卡片均支持点击标题折叠（`is-folded`）；顶部 chrome 「▴ 收纳」支持一键联动折叠，中枢收缩为单行胶囊。

## 4. 约束与状态

- **Flags 还原**：取证完成后 `src/apps/desktop/src/flags.rs` 已还原为 `DIOXUS_SHELL = false`，`flags` 单元测试通过。
- **无 Commit**：所有 E1 和 E2 代码均保留在工作区未 commit。
- **无光标劫持**：CDP 端口 9333 + Hidden 模式完成全部 DOM 触发与取证。
