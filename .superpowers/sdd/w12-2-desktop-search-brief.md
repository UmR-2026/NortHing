# Task Brief — W12-2: 归档页接入会话全文搜索（desktop UI）

仓库：`E:\agent-project\NortHing`（分支 main）。**BASE commit = `ca38f88`**（W12-1 后端已合入）。
本单只碰 `src/apps/desktop/` 与 3 个 FTL 语言文件。

## 1. 来源与验收标准

计划：`.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md`（W12 三拆，本单 = W12-2 + W12-3 合并）。
需求：PRD SE-02「会话搜索」+ 用户 2026-08-29 裁决 C2 + 用户 2026-08-31 拍板默认值。

验收（逐条可机械核对）：
1. `src/apps/desktop/src/ui_dioxus/api.rs` 有 `search_sessions` wrapper，转调 facade。
2. 归档页搜索框走**服务端全文搜索**（不再是纯客户端标题过滤），空串回退完整列表。
3. 结果行展示：会话名 + snippet + 时间；点击可打开该会话详情（复用既有详情展开路径）。
4. 标题命中仍排在前（同一输入框，标题匹配优先于正文匹配）。
5. 空态 / 错误态 / 加载态中文文案，走既有 `locale.t()` + FTL 三语。
6. `cargo check -p northhing` 0 error；`cargo test -p northhing --lib` 全绿；rot 绿。
7. 恰好一个 commit，不含 `.superpowers/`。

## 2. 编排者预检结论（直接采信，不重复侦察）

| 事实 | 位置 |
|---|---|
| 契约（W12-1 已实现，逐字） | `async fn search_sessions(&self, query: &str, workspace: Option<&str>, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError>` |
| DTO 字段（逐字） | `SessionSearchHitDto { session_id: String, session_name: String, message_id: String, role: String, snippet: String, timestamp_ms: i64 }` |
| facade 实现位置 | `src/crates/assembly/core/src/kernel_facade/session.rs:181` |
| desktop wrapper 既有模板（照抄风格） | `src/apps/desktop/src/ui_dioxus/api.rs:70-82`（`get_messages` / `delete_session` / `rename_session`） |
| 归档页现有搜索实现（标题过滤，要替换） | `src/apps/desktop/src/ui_dioxus/pages_archive.rs:245`（signal）、`:312-321`（filtered 计算）、`:449-458`（搜索框 RSX） |
| 归档页当前行数 / ceiling 余量 | 686 行 / 上限 800（**余量 114**） |
| i18n 既有模式 | `locale.t(keys::ARCHIVE_*)` + `src/crates/assembly/core/locales/{en-US,zh-CN,zh-TW}.ftl` 三语同步 |
| **css.rs 零余量** | 790/790 —— **禁止触碰 css.rs**，新样式用既有 class 或 inline style |

## 3. 复用侦察（强制，report 必须有此节）

动手前用 codegraph_explore / rg 确认：归档页既有的列表渲染、详情展开、删除/重命名/导出的 op_error 展示、`locale.t` key 定义位置（i18n.rs / keys）。
**禁止**：新写第二套会话列表渲染、新写一套 toast/错误展示、把 `format_session_export` 之类既有纯函数复制改改。
report 写明查了哪些符号、复用了什么、若新写了等价物给理由。

## 4. Spec

1. **api wrapper**（`api.rs`，≤15 行，风格对齐 `:70-82`）：
   `pub async fn search_sessions(query: &str, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError>`
   —— 内部：`kernel_facade().search_sessions(query, None, limit).await`（workspace 传 None = default workspace，与既有 `list_sessions` 语义一致）。
2. **搜索行为**（判断点已授权）：输入框 `oninput` 触发，**debounce 300ms** 后发请求；空串不发请求、直接回退完整列表（沿用现有 `all_sessions` 列表）。
3. **结果排序**：标题命中排前，其次按 `timestamp_ms` 倒序。
4. **结果行**：会话名（主行）+ snippet（次行，截断到 UI 宽度内，用既有 class）+ 时间（复用归档页现有时间格式化写法，不要新写格式化函数）。
5. **点击行为**：点击结果行 → 打开该会话详情，复用归档页既有的详情展开路径（不要新写详情面板）。
6. **状态文案**：新增 i18n key（如 `ARCHIVE_SEARCH_EMPTY` / `ARCHIVE_SEARCH_FAIL` / `ARCHIVE_SEARCHING`），**三语 FTL 同步**；沿用 `locale.t()` 模式（W9-4 的 M-1 教训：本仓 Dioxus UI 自始至终走 FTL→LocalePack，不许硬编码中文）。
7. **纯函数测试**（≥2）：若有可提纯的排序/截断逻辑，写成纯函数并加测试，放归档页既有 `#[cfg(test)]` 模块；无则跳过并在 report 说明。
8. **行数纪律**：`pages_archive.rs` 不得超过 800 行。若增量导致越线 → 抽新文件（如 `pages_archive_search.rs`），新文件 <800 行。

## 5. Global Constraints（逐字遵守）

1. 分层：UI 逻辑只在 desktop；不改 contracts / assembly（W12-1 已定稿，发现契约问题 → BLOCKED 上报，不许自己改后端）。
2. 日志英文、无 emoji。
3. rot-budget：不上调任何 ceiling；`pages_archive.rs` ≤800；**css.rs 零余量，禁止触碰**；收口 rot 绿。
4. god-file 观测点：`pages_archive.rs` 是观测文件，report 必须附一句健康度观察（更纠结 / 持平 / 更清晰 + 依据）。
5. i18n frozen 的真实含义：不引入新 locale 模块，沿用既有 `locale.t()` + FTL 三语；零新增 i18n:audit 错误。
6. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git restore .` / `git checkout .` / `git clean` / `git add -A`；只许点名文件 add/commit。
7. 恰好一个 commit，不含 `.superpowers/`。
8. 遇编译错误先加载对应 rust skill（见文末），禁止无脑 clone/unwrap。

## 6. 验证（命令 + 输出原文进 report）

```powershell
cd E:\agent-project\NortHing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib
node scripts/verify-rot-budget.mjs
git diff --stat
```
**环境硬事实**：PATH 上的 GNU cargo 会遮住 rustup shim 并导致链接失败，必须用上面 `rustup.exe run stable-x86_64-pc-windows-msvc` 完整前缀，直接敲 `cargo` 会失败。

**截图**：归档页真实运行截图存 `.superpowers/sdd/w12-2-shot-1.png`；若本机起不了 Dioxus 壳（无 GUI 运行时/无 Windows GUI 子系统），改用 SVG mockup 并在同名 `.NOTE.md` 里写明"mockup 非真机"+重拍步骤（W9-6 先例）。

## 7. 报告

路径：`.superpowers/sdd/w12-2-report.md`
必含：状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit SHA、`git show --stat`、四条验证命令输出尾部、「复用侦察」节、god-file 健康度观察、偏离清单、截图路径（或 mockup 说明）、每个编译错误修在哪一层。
假汇报 = 停用：编排者会用磁盘 diff + git log 逐条核对。

## 8. 派发元信息

- BASE commit：`ca38f88`
- 禁区：`.superpowers/`（除报告与截图）、`progress.md`、`src/crates/`、`src/apps/cli/`、`scripts/`、`css.rs`
- 点名可改：`src/apps/desktop/src/ui_dioxus/api.rs`、`src/apps/desktop/src/ui_dioxus/pages_archive.rs`（或抽出的新文件）、`src/apps/desktop/src/ui_dioxus/i18n.rs`、`src/crates/assembly/core/locales/{en-US,zh-CN,zh-TW}.ftl`
- commit 规则：恰好一个 commit

---

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
