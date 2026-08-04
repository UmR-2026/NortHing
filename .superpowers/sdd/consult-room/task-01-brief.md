# Task 1 Brief — chrome 与系统层（consult-room Slint 建构期）

> 本 brief 是唯一需求来源。不要问问题；歧义按 §6 已解决项处理，其余按真值文件自行判断并在 report 记录。
> 总计划：`.superpowers/sdd/consult-room/plan.md`（可读作背景，需求以本文为准）。

## 1. 位置

- worktree：`E:\agent-project\northing\.worktrees\consult-room-build`（分支 `feat/consult-room-slint`）。
- 目标代码：`src/apps/desktop/src/ui/`。
- 你只在此 worktree 工作；不碰其他 worktree / 分支。

## 2. 必读（按序，全部读完再动手）

1. `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html` — **唯一视觉真值**。chrome/边界/呼吸/编年史/头像的形态以它为准。
2. `docs/design/2026-07-22-frontend-redesign/consult-room/PANELS-BRIEF.md` §1 — 系统继承总纲。
3. `docs/design/2026-07-22-frontend-redesign/slint-feasibility-consult-room.md` — Slint 翻译词汇（spike 实测）。
4. `docs/design/2026-07-22-frontend-redesign/prototypes/slint-safe-conventions.md` — Slint-safe CSS 规范。
5. `docs/design/2026-07-22-frontend-redesign/visual-iter-compass_20260802.md` §2/§3 — 戒律与 token（与真值冲突处以真值为准）。
6. 现有代码：`src/apps/desktop/src/ui/main.slint`、`components/WindowChrome.slint`、`components/AirTint.slint`、`components/ChronicleBar.slint`、`components/AvatarWrap.slint`、`redesign_palette.slint`（mind 25 token 已在）。

## 3. 范围（六项，全做）

1. **WindowChrome 重制**（改 `components/WindowChrome.slint` + `main.slint` 挂载）：
   - 废除标题栏形态：无独立 titlebar 条。状态行（brand-inline）= 真 logo + 页面身份文字，按真值形态。
   - 窗控四键（主题切换 ☀ / 最小化 ─ / 最大化 □ / 关闭 ✕）移入**主体区右上**（不锚侧栏、不进独立条），保留既有 Rust 回调接线（minimize/maximize/close/dark-mode toggle），只移形态与位置。
   - 印章（品牌水印）收进房间左下，opacity 按真值（~0.25），不与任何元素重叠。
   - 状态行/room-head 区域兼作窗口拖拽区（现状若已有拖拽接线则保持；没有则报告，不新造 Rust FFI）。
2. **containment + membrane-frame 双边界**（新组件或 WindowChrome 内实现）：外圈 border（暗=frame 发光色/亮=line，按真值取值）+ 内圈 1px 线（`visible: !dark` 语义按真值）；尺寸用 100%/parent 表达式贴窗口，**禁硬编码 px 尺寸**；resize 跟随。
3. **room-fog 沉积底雾**（演进 `components/AirTint.slint`）：底染 + 顶晕 + 底雾三层按真值强度；archive 路由冷雾（cold）语义保留；speaking 升档语义保留（本任务不新增升档触发）。
4. **呼吸 8s 单钟全局范式**：`animation-tick()` + `Math.sin`，周期 8000ms，绑 **opacity**（Slint 无 scale-x/scale-y，spike 已验）。振幅分级按真值（主体>边界>结构）。把周期/振幅做成可复用常量或组件属性，供后续页面任务复用；本任务在 AvatarWrap 与边界/顶晕上实际接线验证。
5. **头像方形化**：`components/AvatarWrap.slint` 及 chrome 所及范围内头像/条/pill 一律 radius 0（近尖角语言；极小圆点除外，按真值）。
6. **ChronicleBar 定稿**（重构 `components/ChronicleBar.slint`）：尖角（radius 0）、高 4px、opacity ~0.7；渐变=历史色按龄褪向出生灰 → 右端当前代表色；**右端色 ≡ 界面强调色，同一属性/变量驱动**（暴露属性供后续任务绑定，本任务内在 chrome 可见处自洽）。换色仪式动画与双击演示属 T2，不做。

## 4. Global Constraints（逐字生效）

1. **Slint 翻译红线**：禁 box-shadow（用 drop-shadow 或线+底色阶，spike 判 B 为默认）；禁运行时 color-mix（预计算 hex，调色走 `oklch-to-srgb.py` 生成器重跑，勿手改 palette 派生值）；禁 @keyframes infinite 新增；渐变仅线性/径向；border-radius 不吃 %（圆=定值 px）；Rectangle 无 scale-x/scale-y（呼吸一律绑 opacity）。
2. **哲学红线**：rep 只属 agent（用户/见证者侧不染边/底）；禁 dashboard 数字；禁 emoji；品牌水印化（印章 opacity ~0.25）；8s 单钟呼吸、振幅分级、不新增 infinite；近尖角语言；编年史右端 ≡ 界面强调色同源。
3. **i18n**：新增 UI 文案走 i18n 契约（`node scripts/generate-i18n-contract.mjs` 重跑并一起提交）；日志 English-only。
4. **纪律**：禁裸 `cargo fmt`；本任务**恰好一个 commit**，message 见 §7；不碰范围外文件。
5. **验证**（report 附命令+输出）：
   ```powershell
   # 本机 MSVC 链接器不可用（link.exe 解析到 Git 的 GNU link）；仓库 override = GNU/MinGW，唯一可行路径。
   # opencode shell 不加载 PowerShell profile，必须手动前置 msys64（否则 gcc/cc1 坏）。
   $env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;" + $env:PATH
   $env:CARGO_TARGET_DIR='E:\agent-project\northing\target'
   $env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'
   cargo check -p northhing
   cargo build -p northhing   # 运行/截图前
   ```
6. **探针纪律**：不得引入 spike 探针（poc_consult_probe.slint / build.rs 探针段）。

## 5. 验收（全要）

1. `cargo check -p northhing` 通过（上述环境变量）。
2. 运行应用并截图（暗 + 亮双光学，main 路由）：
   ```powershell
   # 运行（同一 shell 保留上述 CARGO 环境变量；必须走 rustup run MSVC，本目录有 GNU override 会导致 ring 链接失败）
   rustup run stable-x86_64-pc-windows-msvc cargo run -p northhing
   # 另开 shell 截图/点击（必须用独立 powershell 进程，避免 Add-Type 类型冲突）
   powershell -NoProfile -File 'C:\Users\UmR\.local\share\opencode\worktree\16ba4143154c219fe7f43650ae6f4d297aa32c23\visual-iter\.opencode\tools\shot-window.ps1' -OutFile '<abs path>.png'
   powershell -NoProfile -File '...\click-window.ps1' -X <int> -Y <int>   # 需要点击导航/切主题时
   # 若截图被全屏应用遮挡：先 ShowWindow 最小化遮挡窗口；应用改 build 后需 kill 重启 northhing.exe
   ```
   截图落 `docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/`（命名 `t1-<内容>-dark|light.png`），**不要 commit 截图**，report 给绝对路径。亮色切换用窗控 ☀ 键（click-window 点击其位置）。
3. 若应用启动被环境阻断（如需后端/凭证）：先报告启动需要什么，尝试最小可行路径；完全阻断则状态 BLOCKED 附细节，不用假数据伪造截图。

## 6. 已解决歧义

- **logo SVG**：真值里的 brand-inline 是内联 SVG；Slint 侧优先用仓库既有 logo 资源（查 `resources/` 与现 WindowChrome 水印用法）；无可用资源时退化为 Fraunces 文字 wordmark，report 记录所用方案。
- **窗控回调**：Rust 侧 minimize/maximize/close/dark-mode 接线已存在，只移 UI 位置，不改 Rust（除非编译必需的最小适配）。
- **拖拽区**：不新增 Rust FFI；现状有则保持，无则 report 记录。
- **呼吸常量放置**：自决（redesign_palette 全局或新 system 文件），report 给出最终 API。
- **旧 Material 组件并存**：本任务不清理 MaterialBanner 等旧件（FR-T3 换绑债务另行处理），除非与 chrome 新形态直接冲突。

## 7. 交付

1. 恰好一个 commit，message 恰为：
   `feat(desktop): T1 consult-room chrome 与系统层 — 窗控入流/双边界/room-fog/8s呼吸/编年史定稿/头像方形化`
   （含 i18n 生成文件，如有；不含截图。）
2. report 写入 `.superpowers/sdd/consult-room/task-01-report.md`（不 commit，留给编排者）：
   - 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）
   - 改动文件清单 + 每项范围对应关系
   - **定稿系统 API 清单**（组件名 / 属性 / 回调 / 呼吸常量位置）——后续任务将直接引用，务必准确
   - 验证输出（check 结果 + 截图绝对路径 + 运行中观察）
   - 设计决定与偏离真值处（如有）+ 理由
   - 遗留/风险
