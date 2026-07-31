# Task 1 Brief: Relay 磁盘层路径防线

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`）
来源：`northing-full-bug-audit-2026-07-31.md` C-1（路径部分）、C-2（磁盘部分）、M-8；补充报告 §5（V-1 磁盘原语）

## 背景（已核实的现状）

- `src/apps/relay-server/src/lib.rs` `DiskAssetStore`：
  - `room_dir()` (L59-61) 直接 `base_dir.join(room_id)`，room_id 来自协议字符串，未校验。
  - `map_to_room()` (L78-86) 直接 `room_dir.join(rel_path)`，`create_dir_all(parent)` 后 `remove_file(dest)` 再建链接；`rel_path` 绝对路径会让 `PathBuf::join` 丢弃前缀。
  - `get_file()` (L88-101) 同样直接 join。
  - `cleanup_room()` (L107-116) 对 join 结果 `remove_dir_all`。
- `src/crates/services/relay-core/src/lib.rs`：`WebAssetStore` trait (L28-47) + `MemoryAssetStore` (L52-106)。
- `src/crates/services/relay-core/src/routes/api.rs`：
  - `upload_web` (L258-301) 唯一防护是 `rel_path.contains("..")` (L273)。
  - `check_web_files` (L327-) 先 `existing_count += 1` 后忽略 `map_to_room` 错误（M-8）。
- 现有 `generate_room_id()`（websocket.rs L212-219）产出 32 字符小写 hex；现有测试用 `"stale-room"`（含 `-`）作 room_id。

## 需求

### 1. 新增校验类型（放 relay-core，建议新模块 `src/crates/services/relay-core/src/validated.rs`，从 lib.rs `pub mod` 导出）

`ValidatedRoomId`：
- 规则：ASCII 字母数字 + `-` + `_`，长度 1..=64。
- 构造即校验：`TryFrom<&str>` / `TryFrom<String>`，失败给明确错误。
- 提供 `as_str()`。兼容 `generate_room_id()` 输出与 `"stale-room"` 这类测试 ID。

`ValidatedRelPath`：
- 逐 component 校验：规范化后只允许 `Component::Normal`；拒绝 `Prefix`（Windows 盘符/UNC）、`RootDir`、`ParentDir`、`CurDir`、空路径。
- 必须把 `\` 与 `/` 都当分隔符检查（Windows 语义下 `a\b` 会解析成两段）；实现上可先统一替换再走 `Path::components`。
- 拒绝含 NUL/控制字符。允许 `assets/index.html` 这类多级相对路径。

`ContentHash`（轻量）：64 字符小写 hex 校验，用于 `store_content`/`map_to_room`/`has_content` 的 hash 入参。

### 2. WebAssetStore trait 签名升级

`map_to_room`、`get_file`、`has_room_files`、`cleanup_room` 的 `room_id` 参数改 `&ValidatedRoomId`；`rel_path`/`path` 改 `&ValidatedRelPath`；hash 参数改 `&ContentHash`。`MemoryAssetStore` 同步更新。trait 保持 `Send + Sync + 'static`。

### 3. DiskAssetStore 磁盘层二次防线

即使类型已校验，磁盘层仍做 containment 复查（纵深防御，防未来调用点绕过类型）：
- `map_to_room`：join 后 `create_dir_all(parent)`，然后 canonicalize parent 并确认其在 canonical 后的 room_dir 内；**先完成全部验证再 `remove_file(dest)`**（现状是先删后建，验证失败时已删旧文件）。
- `cleanup_room`：canonical 后的目标必须严格位于 canonical base_dir 内且不等于 base_dir 本身，否则拒绝删除并 `tracing::warn!`。
- `get_file`：SPA fallback 的 `index.html` 拼接同样走校验后的路径。

### 4. 路由层入口转换（api.rs）

- `upload_web` / `check_web_files` / `upload_web_files` / `serve_room_web_catchall`：handler 入口把 `String` 转为 Validated 类型，失败返回 `StatusCode::BAD_REQUEST`（room_id 非法可返回 `NOT_FOUND`，与现状语义一致）。
- 删除 L273 的 `contains("..")` 补丁式检查（被类型取代）。
- **M-8 修复**：`check_web_files` 中 `map_to_room` 失败的条目不得计入 `existing_count`——失败条目应计入 `needed`（让客户端重传）或返回错误；响应计数必须与实际磁盘状态一致。

### 5. 测试（必须随实现同提交）

relay-core（validated 类型单测）：
- `ValidatedRoomId`：`"stale-room"`、32-hex 通过；`".."`、`"a/b"`、`"a\\b"`、`"/etc"`、`"C:\\x"`、`"\\\\unc\\x"`、空串、65 字符、非 ASCII（如 `"房间"`）拒绝。
- `ValidatedRelPath`：`"index.html"`、`"assets/app.js"` 通过；`"../x"`、`"..\\x"`、`"/abs"`、`"C:\\abs"`、`"\\\\unc"`、`"a/./b"`、`""`、含 NUL 拒绝。
- `ContentHash`：64-hex 通过；63/65 字符、非 hex 拒绝。

relay-server（DiskAssetStore 集成测试，用 `tempfile` 临时 base）：
- `map_to_room` 正常路径落盘且内容可读；验证失败时不产生任何文件/目录、且不删除已存在的 dest。
- `cleanup_room` 删除合法房间目录；对构造不出的非法 ID 场景（直接单测内部防线）不删除 base 外任何路径。
- `get_file` 正常读取 + SPA fallback 命中 `index.html`。
- `check_web_files` 映射失败条目计入 `needed`，`existing_count` 不虚增（路由层测试可用 tower `oneshot` + MemoryAssetStore 或直接测 handler 逻辑）。

## 明确不做（防范围蔓延）

- 不改 `RoomManager`/`create_room` 签名与房间生命周期（Task 2 范围）。
- 不加认证（Task 2 范围）。
- 不改 WS 消息尺寸/队列（Task 2 范围）。
- 不动 `generate_room_id`。
- GET catch-all 的动态可达性定性归 Task 3，本任务只封磁盘原语。

## 约束（逐字自仓库 AGENTS.md / 全局规则）

- Logs must be English-only, with no emojis.
- 生产 `.rs` 文件超 800 行触发 review pressure；超 1000 行必须拆分或顶部加 `// allow-god-file` 注释。新模块从低行数开始。
- 顺手清配额：附近小的同范围债务可一并修，但要在 commit message 里可追溯。
- 协议兼容：本任务不得改变合法请求（现有 32-hex room_id、正常相对路径上传）的线上行为。

## 验证命令（必须全部通过，report 附输出）

```
cargo check -p northhing-relay-core -p northhing-relay-server
cargo test -p northhing-relay-core -p northhing-relay-server
cargo fmt -- --check  （或 pnpm run fmt:rs 格式化改动文件）
```

注意：`cargo check --workspace` 当前被 webdriver→tauri→embed-resource 3.0.11 链阻断（已知上游问题），不要用 workspace 级命令判定成败。

## Report

写到 `.superpowers/sdd/task-01-report.md`：改动文件清单、每项需求的落点（file:line）、测试清单与命令输出、状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、遗留疑虑。
