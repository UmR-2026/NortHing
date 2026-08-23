# E1 archive 窗 — implementer brief

先读总 brief：`.superpowers/sdd/consult-room/task-ef-pages-master-brief.md`。不要问问题。不要 commit。

## 0. 坐标

worktree：`E:\agent-project\northing\.worktrees\consult-room-build`  
真值：`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-archive-v2.html`（地层 12 层 + 节气 + 只读）  
W2.7 参照：`windows.rs` 的 `self_app_root` 卡折叠 + `css.rs` OVERLAY 的 `.is-folded` / 18px padding。

## 1. 要交付的产品

一扇独立 OS 窗 `id="archive"`：

**Chrome（轻，W2.7 同款）**  
标题「档案馆」+ ▴ 收纳（折/展全部地层卡或侧卡）+ ✕ 关窗（走现有 `hide_and_close_hwnd` + `window().close()` + DropGuard）。窗控不要 ─□（契约：浮窗只留主题可选 + ✕；主题钮可放 chrome 右，与 room 同步 GlobalTheme）。

**主体**  
- 中枢：印章改「渊」（Fraunces），名「深渊的领域」，状态 pill「沉积 · 只读 · 缓」。可骑缝折叠中枢（抄 room-head folded）。
- 地层流：至少 **8 层** mock（真值有 12，本刀 8 层即可，data-depth 1..8，opacity 按真值表 1.00→0.28）。每层：层号+节气时间 / 标题 / 摘录 / 它·见证者。点选一层 = `.active`（左缘 3px abyss 条）。
- **禁 dashboard 数字**；深度说明用叙事句（真值：「二十三段对话沉在这里」）。

**左列（同窗内，不要第二 OS 窗）**  
W2.7 卡语法，可折到标题：
1. 沉积深度 DEPTH — 叙事 + 可选 seg/depth-bar
2. 节气刻度 SOLAR — 最近·立春 / 大寒 / 冬至…（mock 单选）
3. 见证标记 WITNESS — 在·多数时段 / 独·沉默间隙

分组缝可选。卡标题 18px 内边距。

**不要**再做真值里的左右门铃宝石开独立 inner/outer（archive 侧栏是同窗卡，不是新 OS 窗）。

## 2. 接线

1. `DockSide` 增加 `Center { width, height }` 或 `Center` + plugin.initial_* 。`spawn_module_window` 对 Center：居中于当前 room 几何，skip-taskbar，decorations=false。
2. `WindowRegistry::default_registry` 注册 `archive` → `pages_archive::archive_app_root`。
3. **新文件** `src/apps/desktop/src/ui_dioxus/pages_archive.rs`（禁止继续堆 `windows.rs`）。`mod.rs` 加 `mod pages_archive;`。
4. room 状态行 `architect_sub 介入中` 右侧、`sp` 之前加两个文字链（10px mono，W2.7 节奏）：
   - `id="nav-archive"` 「档案」→ `spawn_module_window("archive", ...)`
   - `id="nav-space"` 「走廊」本刀可先做成按钮但 **onclick 空/disabled**，或只做档案一个链（推荐只做档案，走廊留给 E2，避免假入口）。**只做档案。**
5. i18n：新键 × zh-CN / zh-TW / en-US。地层 mock 正文可硬编码中文标本（与 room 对话 mock 同例），chrome/导航/卡标题必须走键。
6. OVERLAY：`body[data-window="archive"]` 前缀。archive 窗 `--mind-base` 覆盖为 `#3F837B`（仅此窗）。地层 `.stratum[data-depth=N]` opacity。不要改 TRUTH_CSS 文件。
7. 生命周期：抄 `self_app_root` 的 DropGuard + `register_window_with_hwnd`。Center 窗**不必**跟 room 移动（本刀不写 geometry follow 线程）。

## 3. 不要做

- 不实现 space/settings/onboarding
- 不接真会话存储
- 不改 flags 持久为 true
- 不 commit
- 不把 12 层全抄导致文件 >800 行——8 层 + 3 侧卡即可

## 4. 验证

1. rustup：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing` 再 `cargo build -p northhing`（临时 DIOXUS_SHELL=true）
2. CDP：Hidden + port 9333。点 `#nav-archive`。截图：
   - `C:\WINDOWS\TEMP\opencode\t7-shots\e1-archive-dark.png`
   - `e1-archive-light.png`（窗内主题钮或 room 主题同步后重截）
   - `e1-archive-folded-dark.png`（折起至少两张侧卡或中枢）
3. **Read 打开三张 PNG 目验**：冷青 abyss 不是珊瑚 rep；地层越深越淡；轻 chrome；标题不贴边。
4. `git checkout -- src/apps/desktop/src/flags.rs`；Stop-Process northhing。
5. 报告：`.superpowers/sdd/consult-room/task-ef-e1-archive-report.md`

## 5. 完成定义

- 从 room 点「档案」开出一扇 archive 窗，关得掉，再点不会叠尸（单例）
- 8 层地层 + 3 张可折侧卡 + 渊中枢
- 双光学；flags 还原；无 commit
