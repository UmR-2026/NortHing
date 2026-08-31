# 计划 — 会话 CRUD 真缺口（2026-08-31）

## 前提修正（必读）

用户指令"补齐删除/重命名/导出/搜索"——**四项已于 W9-4 交付**（`4aba165` 实现 + `9603a65` 修复，judge PASS 0C/0I/2M，122/122）。
需求表 `docs/product/requirements-vs-current-2026-08-29.md` 的会话行写"四项全无"，其底层盘点 `current-state-inventory-2026-08-29.md` 成文于 08-29 02:47，**早于 W9-4 落地（03:05）**，属过期结论。

本计划只做**真缺口**，不重复造。

| 项 | 真缺口 | 规模 |
|---|---|---|
| 搜索 | 现为标题过滤（`pages_archive.rs:319` `name.to_lowercase().contains`），非全文 | M（跨三层） |
| 导出 | 仅 Markdown、固定 `<config>/northhing/exports/`，无格式选择、无另存为（rfd 已删） | S |
| 删除 | 活跃 room 禁用 → `ROOM_SESSION_CACHE` 永不失效（`api.rs:115` ponytail 注释），潜伏 | S-M |
| 重命名 | 无缺口（M-2 CJK 已修） | — |

## 目标（推荐项，待用户确认）

**W12：会话全文搜索** —— 归档页搜索从"标题过滤"升级为"消息正文全文搜索"。

### 设计（默认决策，标注待确认）

1. **后端**（`assembly/core`，`PersistenceManager`/coordinator 层）
   - 新增 `search_session_messages(workspace_path, query, limit)`：遍历该 workspace 下 session 目录 → 复用既有 `rebuild_messages_from_turns` → 消息正文大小写不敏感包含匹配。
   - `ponytail: 全量扫描（会话数 × 消息数），无索引；会话量到百级/消息到万级需升级为 SQLite FTS 或 transcript index 复用。`
   - **不新增全局句柄**：沿用 `session_manager().persistence_manager` 既有访问路径（同 `list_sessions_all_workspaces`）。

2. **契约层**（`contracts/kernel-api/src/session.rs`）
   - `KernelSessionApi` 新增 `search_sessions(query: &str, workspace: Option<&str>, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError>`
   - `SessionSearchHitDto { session_id, session_name, message_id, role, snippet, timestamp_ms }`（snippet = 命中位置前后各 40 字符）

3. **facade**（`kernel_facade/session.rs`）纯 passthrough，错误映射对齐既有 `KernelError::Runtime`。

4. **desktop**（`ui_dioxus`）
   - `api.rs` 加 `search_sessions` wrapper（≤15 行）。
   - `pages_archive.rs` 搜索框行为：**输入即走全文搜索**（服务端），空串回退列表；标题匹配仍在客户端前置（同一输入框，命中标题的会话排前）。
   - 结果行展示：会话名 + 命中 snippet + 时间，点击跳该会话详情。

### 任务拆分

| # | 任务 | 层 | 依赖 |
|---|---|---|---|
| W12-1 | 后端 `search_session_messages` + 契约 DTO/trait 方法 | contracts + assembly | 无 |
| W12-2 | facade passthrough + desktop `api.rs` wrapper | assembly + desktop | W12-1 |
| W12-3 | 归档页 UI 接入（结果行 + snippet + 空/错误态 i18n） | desktop | W12-2 |

### 验证集（每任务必跑，输出原文进 report）

- `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check --workspace`
- `cargo test -p northhing-core --features product-full session` + `cargo test -p northhing --lib`（W12-1/2 需新增 ≥2 测试：命中、无命中、空 query）
- `node scripts/verify-rot-budget.mjs`
- W12-3 附截图（跑不了真壳用 mockup 并标注，同 W9-6 先例）

### 约束（逐字来自仓库 Global Rules）

- 分层：contracts 只加 DTO + trait 方法（behavior-light）；facade 纯 passthrough；UI 只在 desktop。
- 日志英文无 emoji。
- rot-budget：不上调任何 ceiling；新文件 <800；`pages_archive.rs` 现 686 行，余量 114。
- i18n：沿用既有 `locale.t()` + FTL（en-US/zh-CN/zh-TW 三语同步），零新增 i18n:audit 错误。
- 恰好一个 commit，不含 `.superpowers/`；禁编辑 `progress.md`。
- god-file 观测点：触碰 `pages_archive.rs` 需附健康度注记。

## 需用户拍板（3 条）

1. **搜索范围**：当前 workspace（默认）还是全部 workspace？
2. **是否含工具调用/思考块**：默认只搜 user+assistant 正文（工具调用噪声大）。
3. **是否顺带做导出增强**（JSON/纯文本 + 路径复制）合并进本波？—— 默认否，留下一波。

## 备选（若用户改选）

- **导出增强**：纯 desktop 层小单，无后端改动。
- **放开删除活跃 room + 缓存失效**：需先定"删了当前 room 后接谁"。
- **先真机验证现有四项**：不写新代码，补 W9-4 的 CV-2 截图缺口。
