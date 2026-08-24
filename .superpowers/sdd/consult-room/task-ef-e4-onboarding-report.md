# Task EF-E4 Onboarding 房间诞生仪式 实现报告

## 1. 改动文件列表

- 新增文件：
  - `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` (700 行，实现独立 OS 窗 `onboarding`：全屏覆盖 Dock 模式 + 灰寂/着色双态 + 三章仪式内容流 + 大五 MIND PALETTE 色板选择器 + 左右抽屉可折叠/唤起)
  - `src/apps/desktop/src/ui_dioxus/pages_onboarding_css.rs` (209 行，将自包含真值样式抽离，忠实转写 color-mix 表达式、生物态 8s 单钟呼吸 keyframes、双光学变量体系及媒体查询)
- 接线与联动文件：
  - `src/apps/desktop/src/ui_dioxus/mod.rs` (声明 `mod pages_onboarding;` 及 `mod pages_onboarding_css;`)
  - `src/apps/desktop/src/ui_dioxus/registry.rs` (`DockSide` 扩展 `Fullscreen` 变体；注册 `onboarding` 插件 1280×860 `DockSide::Fullscreen`；追加生命周期单测)
  - `src/apps/desktop/src/ui_dioxus/app.rs` (`spawn_module_window_with_theme_rx` 增加 `DockSide::Fullscreen` 几何分支，覆盖 room 当前几何；room 状态行 `#nav-space` 之后增加 `#nav-onboarding` 导航入口)
  - `src/apps/desktop/src/ui_dioxus/i18n.rs` (增加 33 个 `ONBOARDING_*` 键)
  - `src/crates/assembly/core/locales/zh-CN.ftl` (新增 33 条词条)
  - `src/crates/assembly/core/locales/zh-TW.ftl` (新增 33 条词条)
  - `src/crates/assembly/core/locales/en-US.ftl` (新增 33 条词条)
- 零触碰文件：
  - `src/apps/desktop/src/ui_dioxus/css.rs` (保持 778 行零增量，复用既有 `.status-nav-link` 选择器)
  - `src/apps/desktop/src/ui_dioxus/windows.rs` (保持 786 行零触碰)
  - `consult-room-main.css` / `TRUTH_CSS` (零触碰)

## 2. 门禁验证证据

### 2.1 编译与单元测试
```bash
$ cargo check -p northhing
Finished `dev` profile [unoptimized + debuginfo] in 19.20s (exit 0)

$ cargo test -p northhing ui_dioxus
running 8 tests
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok
test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s

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
- `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs`: 700 行 (<800)
- `src/apps/desktop/src/ui_dioxus/pages_onboarding_css.rs`: 209 行 (<800)
- `src/apps/desktop/src/ui_dioxus/pages_settings.rs`: 459 行 (<800)
- `src/apps/desktop/src/ui_dioxus/pages_space.rs`: 640 行 (<800)
- `src/apps/desktop/src/ui_dioxus/pages_archive.rs`: 459 行 (<800)
- `src/apps/desktop/src/ui_dioxus/css.rs`: 778 行 (<800，零改动)
- `src/apps/desktop/src/ui_dioxus/windows.rs`: 786 行 (<800，零改动)
- `src/apps/desktop/src/ui_dioxus/app.rs`: 652 行 (<800)
- `src/apps/desktop/src/ui_dioxus/registry.rs`: 502 行 (<800)
- `src/apps/desktop/src/ui_dioxus/i18n.rs`: 368 行 (<800)

## 3. CDP 视觉取证与目验

### 3.1 截图路径
- Dark 灰寂初始态：`C:\WINDOWS\TEMP\opencode\t7-shots\e4-onboarding-dark.png`
- Light 亮色初始态：`C:\WINDOWS\TEMP\opencode\t7-shots\e4-onboarding-light.png`
- Selected 着色态（右抽屉展开）：`C:\WINDOWS\TEMP\opencode\t7-shots\e4-onboarding-selected.png`

### 3.2 目验结论（Read 打开 3 张 PNG 逐项验证）
- **灰寂初始态 (Dark/Light)**：未选色时 `--mind-base` 为初始灰 `#7e8896`，中枢头像呈现虚线边框 (`[data-inhabited="false"]`)，字符为初始「?」，中枢名称显示「未命名诊室」，状态 pill 显示「沉寂态 · 等待人类决定第一个色彩」，状态行显示「沉寂中 · 待注入印记」；
- **三章仪式流与五色板**：chat-flow 内「第一章：身份凝结」、「第二章：管道贯通」、「第三章：物理锚定」三卡齐全；五色板「驱力 #C8714C / 深渊 #3F837B / 跃迁 #8B5FBF / 凝视 #D99B48 / 镇静 #4B8F6B」完整排列；
- **着色态整屋流动**：点击「驱力」色板后即时切入 `--mind-base: #C8714C` 与 `data-inhabited="true"`，头像边框由虚转实且带光晕，中枢 pill 更新为「驱力色板 (探索 / 开拓) · 印记已注入」，状态行更新为「驱力状态 · 房间印记已铸造」，右抽屉「诞生存根」展开且 Step 1 显示已凝结（绿色 ok 标）；
- **双光学真实差异**：Light 与 Dark 态下背景、文本、边框与高斯晕影均准确根据 `data-theme` 切换，无样式破裂。

## 4. 偏差及理由说明

- **测定心跳脉冲与完成仪式**：按 brief §2/§3 要求，心跳测试 mock 点击即时返回 `✓ 心跳贯通 · 延迟 12ms · 神经元就绪`（ok 态）；完成仪式在未选色时进行状态行内联提示，选色后切换为完成态 `✓ 诊室已诞生 · 空间运行中`，窗口保持不关。
- **静态 Aura 与抽屉折叠**：Aura 光晕定位采用静态默认（`--aura-x: 50%; --aura-y: 200px`），抽屉折叠统一沿用 W2.7 语法。

## 5. 约束与状态

- **Flags 还原**：取证完成后 `src/apps/desktop/src/flags.rs` 已还原为 `DIOXUS_SHELL = false`，`cargo test -p northhing flags` (3 passed) 验证通过。
- **无 Commit**：所有改动保留在当前工作树未 commit。
- **无光标劫持**：CDP 端口 9333 + Hidden 模式完成全流程 DOM 交互与取证。
