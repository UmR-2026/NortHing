# E2 space 走廊窗 — implementer brief

先读：`task-ef-pages-master-brief.md`。E1 archive 已在工作树未 commit（`pages_archive.rs` + `DockSide::Center` + `#nav-archive`）。**在此基础上追加，不要 revert E1。** 不要 commit。不要问问题。

## 0. 真值

`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-space-v2.html`

抄结构不抄「同页左右抽屉变 OS 窗」。侧栏 = 同窗可折卡（W2.7）。

## 1. 产品

独立 OS 窗 `id="space"`，`DockSide::Center`，约 720×820。

**Chrome**：标题「走廊」+ ▴ 收纳 + 主题钮 + ✕。轻，frameless，skip-taskbar。

**中枢 hall-head**：无 agent 头像（真值：它只住在亮着的房里）。名「走廊」；状态「诊室之外 · 你尚未进入任何一间」；小字「它只在亮着的那间房里说话。」可折成一行。

**门厅 door-hall**（主体可滚）：
- 分组标「亮着」：1 扇 `.door.lit`（诊室 03 · 重新定义对齐 · 门灯「序」· 进入这间房按钮 mock）
- 「熄灯的门」：2–3 扇 `.door.dim`（02 昨夜 / 01 三天前 / 00 命名仪式，至少 2 扇）
- 「沉积层」：2 扇 `.door.sunk` 更淡 + 尾句「再往下的门已经看不清轮廓」
- 底链「档案馆 · 去看沉下去的门」→ `spawn_module_window("archive")`（可调已有 manager）

亮门独占 `--mind-base` 珊瑚光；暗门中性；沉门更低 opacity。不要每扇门都发光。

**底栏**：给新房起名的输入行（placeholder + ➤），mock，不必真开房。

**左列同窗卡（可折到标题）**：
1. 走廊排序 ORDER — 按最近亮起 / 按沉积深度
2. 工作文件夹 WORKSPACE — `~/northing/alignment` 已挂载
3. 走廊显示 DISPLAY — 显示沉积层 / 门后摘要

**右列同窗卡「门缝所见」（可折）**：
- 这扇门 DOOR — 跟当前选中门同步（默认亮门文案）
- 留在门内的沉积
- 门内产物 chips
- 可选 term-well 吃底（W2.7 终端语法）

点一扇门：该门 `.lit`/选中，右列 peek 文案更新。不必真切回 room。

## 2. 接线

- 新文件 `pages_space.rs`（<800）。`mod.rs` 声明。
- registry 注册 `space`。
- room 状态行在 `#nav-archive` 旁加 `#nav-space`「走廊」，onclick spawn space。
- i18n 新键 ×3 语。门上 mock 正文可硬编码中文标本。
- OVERLAY `body[data-window="space"]`。门样式写 overlay，禁改 TRUTH_CSS 文件。
- DropGuard + register_window_with_hwnd。无 geometry follow。
- 「进入这间房」可 close space 窗（mock）；不要崩进程。

## 3. 不要做

settings / onboarding；真多会话；commit；flags 持久 true。

## 4. 验证

MSVC `cargo check -p northhing` + `cargo build -p northhing`（临时 DIOXUS_SHELL=true）。  
CDP 点 `#nav-space`。截图：

- `C:\WINDOWS\TEMP\opencode\t7-shots\e2-space-dark.png`
- `e2-space-light.png`
- `e2-space-folded-dark.png`（折至少两张侧卡）

Read 打开目验：一扇门亮、其余熄、沉门更淡；轻 chrome；可折卡。  
restore flags；Stop-Process northhing。  
报告：`task-ef-e2-space-report.md`

## 5. 完成定义

room「走廊」能开 space 窗；亮/暗/沉三态门可见；侧卡可折；双光学；flags false；无 commit。
