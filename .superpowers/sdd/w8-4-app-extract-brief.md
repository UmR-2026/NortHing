# Task Brief — W8-4: app.rs 抽离 + onboarding 硬编码路径修复

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop` 仅桌面 crate。
深审报告（先读，§1 全部病灶带 file:line）：`.superpowers/sdd/deep-rot-app-input.md`。

## Spec（验收标准）

### 1. 抽离颜色工具（行为零变化）

- `parse_hex_rgb` / `mix_hex` / `chronicle_gradient`（app.rs:876-931）+ 其测试（app.rs:933-959）→ 新文件 `src/apps/desktop/src/ui_dioxus/color.rs`；app.rs 只留 `mod color;` + `use` 适配。
- 深审观察项顺手收（家规 1 顺手清配额）：给三个函数补 2-3 个边界测试（非法 hex、空历史、纯黑/白极值），放 color.rs 内联测试。

### 2. 抽离窗口操作（行为零变化）

- `win_ops` 模块（app.rs:37-96，unsafe FFI）+ `close_module`/`close_all_modules`/`quit_shell`（83-103）→ 新文件 `src/apps/desktop/src/ui_dioxus/window_ops.rs`。
- 非 Windows 的 `close_os_window` 空 no-op 保留原样（深审观察项，不动）。

### 3. PopupType→hide 映射去重

- `close_all_popups`（37-54）与 `navigate_back`（62-77 hide 段）共享的 PopupType→hide 方法映射 → 抽单一定义（如 `fn hide_popup(chat_view, PopupType)`），两处消费。新增 popup 只改一处。

### 4. L74 线程 spawn 静默吞错（深审腐化项）

- `std::thread::Builder::new().name(...).spawn(...)` 上的 `.ok()` → 改为 Err 臂 `tracing::warn!`（英文，带上下文）后忽略；一行注释说明 best-effort 意图。

### 5. onboarding 硬编码路径修复

- `pages_onboarding.rs:133`：`workspace_dir_input` 默认值 `"E:\\agent-project\\northing\\workspace"` → 空串 + 输入框 placeholder 给示意（如 `例如 D:\projects\my-workspace`）。
- 先读 Step3 校验逻辑（step_gate 已有路径存在性校验），确认空默认值不破坏 onboarding 流程（用户必填才推进）。若现有流程对空串有意外行为，STOP BLOCKED。

### 6. manifest 处置

- 收口实测 app.rs 行数：<800 → 删除 `god_file:...app.rs` 条目（回归通用 800 线保护）；≥800 → ceiling 下调到实测值。同 commit。
- `pages_onboarding.rs` 行数变化若为负，ceiling 866 不动（只降不升指 ceiling 数值，本任务不主动降它——留给下次）。

### 7. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error，warnings ≤44 基线
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`：全绿（既有 + 新边界测试）
3. `node scripts/verify-rot-budget.mjs`：绿

## Global Constraints（逐字，源自 plan-2026-08-28-w8-godfile-rotfix.md）

1. 分层边界：改动只在 `src/apps/desktop`（+ manifest 处置）。
2. 日志纪律：英文无 emoji（§4 新增一条 warn）。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止 `git restore .`/`git checkout .`/`git stash` 等整树操作**，只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升/清死条目，同 commit 说明。
5. 验证最小集：上述 3 条；report 写入 `.superpowers/sdd/w8-4-app-extract-report.md`（write 工具）。
6. commit 规则：恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象；新模块消费方 = app.rs 既有调用点。
8. 行为零变化铁律：抽离纯位移；唯一行为变化 = §4 warn 日志 + §5 默认值。
9. 遇编译错误先加载对应 rust skill；unsafe FFI 移动不改任何 unsafe 块内容（unsafe-checker 纪律）。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / app.rs 新行数 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。
