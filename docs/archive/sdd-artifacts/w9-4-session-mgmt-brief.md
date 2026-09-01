# Task Brief — W9-4: 会话管理（搜索/重命名/导出/删除）+ subagent 低显著度可见

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop`（+ 必要时 desktop api.rs wrapper）。
来源：校准裁决 C2+C3（`docs/product/requirements-vs-current-2026-08-29.md` §三）。

## 裁决语义（钉死）

- C2：会话管理要做全——**搜索、重命名、导出、删除**（归档页为载体）。
- C3：subagent 会话**低显著度可见**——在会话列表里能看出来、能点进去看详情，但视觉上弱化（小标记/缩进/灰化均可，不抢主会话戏）。SessionSummaryDto 的 kind/父子关系字段以代码实际为准。

## 现状（编排者已核实）

- facade 全有：`delete_session` / `rename_session` / `archive_session` / `get_session` / `get_messages` / `list_sessions_all_workspaces`（kernel_facade/session.rs）。
- desktop api.rs：list/get wrapper 已有；delete/rename wrapper 需补。
- 归档页：`pages_archive.rs` 有会话列表。
- 导出固定路径先例：W9-2 memory 面板导出走 `<config_dir>/northhing/exports/`——沿用同一目录约定（`session-<id>-<ts>.md` 或 `.jsonl`）。
- 防线：app.rs 791/800 余量 9（本任务**零触碰 app.rs** 为底线）；css.rs 830/830 余量 0（复用既有 class）。

## Spec（验收标准）

1. **搜索**：归档页顶部搜索框，按标题/内容摘要过滤当前列表（纯前端过滤即可）。
2. **重命名**：行内或弹窗编辑标题 → `rename_session` → 列表刷新。空名/超长（>80 字符）前端拦截。
3. **删除**：两段确认 → `delete_session` → 列表刷新。删除当前活跃 room 会话的处理：禁止（按钮置灰+提示）或删除后自动重建——实现者读代码选定，report 说明。
4. **导出**：每条会话可导出 Markdown（含时间戳、角色、正文、工具调用摘要）到 exports 固定目录，完成显示路径。
5. **subagent 低显著度**：subagent 会话在列表中带"子任务"小标记（或等效弱化样式），可点开看消息详情（只读）。
6. 空态/错误态中文显式展示。

## 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error，warnings ≤47
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`：全绿（纯逻辑如 Markdown 导出格式化须抽函数并附测试）
3. `node scripts/verify-rot-budget.mjs`：绿
4. 截图：归档页（含搜索框+行操作+子任务标记）`.superpowers/sdd/w9-4-shot-1.png`（不 commit）

## Global Constraints

1. 分层边界：只动 `src/apps/desktop`。
2. 日志英文无 emoji。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止整树 git 操作**（restore/checkout/stash），只许点名文件 add/commit。**开工前先 `git status` 核查工作树（前任事故教训）**。
4. rot-budget：不上调任何 ceiling；新文件 <800；收口 rot 绿。
5. commit：恰好一个；不含 `.superpowers/`。
6. 不新建无 owner 抽象；复用既有（exports 目录约定、确认两段式模式参照 provider 删除）。
7. i18n frozen：硬编码中文 UI 文案。
8. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE（全做完）；受阻 = BLOCKED + 原因。禁止报 Done 留 next steps。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / 截图路径 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff + 读截图逐条核对。
