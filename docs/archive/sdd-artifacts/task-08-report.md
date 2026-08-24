# Task 8 Report: M-9 LSP plugin ID 校验 + M-2 dropped Future

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 9be74ec）
未 git commit（遵循 brief 约束）。

## 改动文件

### M-9 — `src/crates/assembly/core/src/service/lsp/plugin_loader.rs`
- 新增 `ValidatedPluginId`（L14–104）：`pub struct ValidatedPluginId(String)`，不变量 ASCII 字母数字 + `-` + `_`、长度 1..=64；`TryFrom<&str>` / `TryFrom<String>` 构造即校验；`PluginIdError { Empty, TooLong, InvalidCharacter }`；`as_str()` / `Debug` / `Display` / `Clone` / `PartialEq` / `Eq` / `Hash` / `Ord`。规则对齐 Relay `ValidatedRoomId`，未跨 crate 依赖 relay-core。
  - 因为允许字符集仅含路径安全、无分隔符字符，合法 ID 接到目录上恒为单个 `Component::Normal`，天然拒绝 `/` `\` `..` `.` 盘符 UNC 绝对/父级逃逸与空串。
- `staging_nonce()`（L106–110）：进程内单调 `AtomicU64`，保证 staging 目录名唯一。
- `load_plugin`（L125）：签名改 `&ValidatedPluginId`，join 用 `as_str()`。
- `load_all_plugins`（L176–182）：磁盘目录名先 `ValidatedPluginId::try_from`，非法 `warn!` + `continue`，不再把脏名透传给 `load_plugin`。
- `install_plugin_package`（L201–268）：返回 `Result<ValidatedPluginId>`。流程改为：打开 zip -> 内存读 `manifest.json` -> 反序列化 -> **立即校验 ID**（非法 `Err`，此时零 fs 写入） -> 计算最终目录、若已存在则 `Err`（仍零副作用） -> 建 `.staging-{pid}-{nonce}` -> `archive.extract(staging)` -> `fs::rename(staging, plugin_dir)`（同文件系统，原子） -> 任何 extract/rename 失败仅清理 staging 并 `Err`。删除了原 `.temp-{pid}` 死逻辑。
- `uninstall_plugin`（L270–308）：签名改 `&ValidatedPluginId`。卸载前 containment 复查：`dunce::canonicalize` 双方，要求 `canonical_target != canonical_plugins && canonical_target.starts_with(canonical_plugins)`，否则 `warn!` + `Err`，绝不 `remove_dir_all`（纵深防御，参照 Relay Task 1）。
- `cleanup_temp_dirs`（L320）：同时清理 `.temp*` 与 `.staging*` 残留。
- `get_server_path`（L339–346）：保持 `&LspPlugin` 签名，但在 join 前 `ValidatedPluginId::try_from(plugin.id.as_str())?` 校验（非法 ID 在 fs 操作前 `Err`）。
- `get_plugin_dir`（L434）：签名改 `&ValidatedPluginId`。
- `#[cfg(test)] mod tests`（L451–740）：13 个测试（见下）。

### M-9 — `src/crates/assembly/core/src/service/lsp/manager.rs`
- L10：`use super::plugin_loader::{PluginLoader, ValidatedPluginId};`
- `install_plugin`（L88）：`Ok(plugin_id.as_str().to_string())`（返回类型保持 `String`）。
- `uninstall_plugin`（L93–110）：入口 `ValidatedPluginId::try_from(plugin_id)?`，`stop_server`/`unregister` 仍用原 `&str`，`plugin_loader.uninstall_plugin(&validated_id)`。

### M-9 — `src/crates/assembly/core/src/service/lsp/mod.rs`
- L32：`pub use plugin_loader::ValidatedPluginId;`（公开导出，供潜在外部使用）。

### M-2 — `src/crates/assembly/core/src/service/search/service.rs`
- L67–68：`pub fn` -> `pub async fn schedule_repo_release`，`self.inner.schedule_repo_release(repo_root)` -> `.await`。inner（`services-integrations/.../service_session.rs:19`）本就是正确的 `async fn` + `tokio::spawn`，bug 仅在 core 同步 wrapper 丢弃 Future；await 后 Future 被驱动，spawn 真正发生。
- L180–196：新增 `#[cfg(test)] mod tests`（见下）。

## API 调用方适配清单（grep 全仓 `.rs`）

| 变更 API | 旧签名 | 新签名 | 调用方 | 适配 |
|---|---|---|---|---|
| `PluginLoader::install_plugin_package` | `-> Result<String>` | `-> Result<ValidatedPluginId>` | `manager.rs:78` | 改为持有 `ValidatedPluginId`，返回 `as_str().to_string()` |
| `PluginLoader::load_plugin` | `(&str)` | `(&ValidatedPluginId)` | `manager.rs:80`、`plugin_loader.rs` `load_all_plugins` 内部 | manager 直接传 `&plugin_id`；`load_all_plugins` 先 `try_from` 目录名 |
| `PluginLoader::uninstall_plugin` | `(&str)` | `(&ValidatedPluginId)` | `manager.rs:105` | manager 入口 `try_from` 后传 `&validated_id` |
| `PluginLoader::get_server_path` | `(&LspPlugin)` | 不变（内部校验 `plugin.id`） | `manager.rs:148` | 无需改动 |
| `PluginLoader::get_plugin_dir` | `(&str)` | `(&ValidatedPluginId)` | 无外部调用方 | 仅签名变更 |
| core `WorkspaceSearchService::schedule_repo_release` | `pub fn`（同步） | `pub async fn` | **0 个外部调用方**（仅 wrapper->inner 自调用） | 无需适配 |

`LspManager::install_plugin` / `uninstall_plugin` 对外签名不变（`String` / `&str`），内部消化。`cargo check -p northhing`（桌面发布面）通过，确认无跨 crate 调用方被破坏。

### 设计决定：`get_server_path` 未改收 `&ValidatedPluginId`
该函数需要 `LspPlugin.server.command` 等，天然接收整个 manifest；若再加一个 `&ValidatedPluginId` 参数则与 `plugin.id` 冗余。改为在 fs 边界内部 `try_from(plugin.id)` 校验，满足 brief 的硬性要求“非法 ID 在文件系统操作前 `Err`”，且无冗余参数。其余三个收裸 id 的函数（load/uninstall/get_plugin_dir）严格改为 `&ValidatedPluginId`。

## M-2 方案选择及理由

- grep `schedule_repo_release` 全仓：inner 定义 1 处、core wrapper 定义 1 处、wrapper->inner 调用 1 处，**core wrapper 外部调用方 = 0**。
- 采用**方案 A**（`pub async fn` + `.await`）：无调用点需适配（0 个），无所有权转移/`spawn` 注释负担，语义最干净。方案 B（`tokio::spawn`）仅在调用点卡死同步上下文时才需要，此处不适用。
- 修复后该处 Future 被驱动，inner 的 `normalize_repo_root` + `tokio::spawn(sleep(45s) -> release_repo_if_idle)` 真正执行。

## 测试与输出

### M-9（`cargo test -p northhing-core --lib lsp::plugin_loader`，default features）
```
running 12 tests
... all ok ...
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out
```
注：12 个测试函数（含 `validated_plugin_id_*` ×3、`install_rejects_invalid_id_with_zero_fs_effect`、`install_rejects_missing_manifest...`、`install_rejects_corrupt_archive...`、`install_extract_failure_in_staging_leaves_no_half_install`、`install_then_uninstall_roundtrip_no_residue`、`install_already_installed_fails_no_residue`、`uninstall_missing_plugin_errors`、`load_plugin_rejects_mismatched_manifest_id`、`uninstall_refuses_target_outside_plugins_dir_via_symlink`）。

覆盖 brief 用例：
- 非法 ID（`..`、`../outside`、`a/../../outside`、`/abs`、`C:\x`、`\\unc\x`、`a/b`、`a\b`、空、`.`、`a b`、`插件`、65 字符超长）-> install `Err` 且 `plugins_dir` 条目数 = 0（零文件变化）。
- 合法 install->load->uninstall 往返：无 `.staging*`/`.temp*` 残留，目录归零。
- staging 中途失败：构造 manifest 合法 + 绝对路径条目 `/escape.txt` 的 zip；zip 4.6.1 在 extract 阶段对绝对路径（`RootDir` 组件）经 `safe_prepare_path` 报错，**在 staging 已创建并部分填充后失败** -> 清理 staging，无半安装目录（`install_extract_failure_in_staging_leaves_no_half_install`）。
- 损坏 zip / 缺 manifest：均在 staging 创建前 `Err`，零副作用。
- containment：在 `plugins_dir` 内植指向外部的符号链接 `escaped` -> `uninstall` `Err` 且外部目标 `secret.txt` 仍在（本 Windows 环境允许建符号链接，该用例实际通过；无权限平台 `#[cfg(windows)]` 分支静默跳过）。

### M-2（`cargo test -p northhing-core --features product-full --lib search::`）
```
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1132 filtered out
```
- `schedule_repo_release_is_async_and_drives_inner_future`：`.await` wrapper，证明 Future 被驱动；同时锁死 async 签名（回退 `pub fn` 将无法编译，因 `()` 不可 await）。

### M-2 测试缺口（按 brief 允许，说明理由）
无法断言“idle release 实际发生（释放计数变化）”：
- `session_idle_grace` 为固定 `Duration::from_secs(45)`，`schedule_repo_release` 内 `tokio::time::sleep` 无 fake-clock 注入缝（services-integrations 侧未暴露可控时钟）。
- `release_repo_if_idle` 依赖已打开的 flashgrep repo session（需 flashgrep 守护进程二进制 + `get_or_open_session`），测试环境无该二进制。
- 改 inner 加时钟/会话注入缝超出 brief “不动 workspace search 的搜索功能本身” 的范围。
故仅加“Future 被驱动”的回归测试 + 编译期 async 守卫，release 计数验证留待具备 daemon 与时钟缝的环境。

### 编译/告警
- `cargo check -p northhing-core --tests`（default）：Finished，无 error。
- `cargo check -p northhing-core --features product-full --tests`：Finished（2m05s），19 条 warning 全为既有（unused vars 等，均不在本次改动文件）；`must_use`/`unused_must_use` 计数 = 0（`--message-format=short | Select-String 'must_use'` -> 0），M-2 丢弃 Future 处告警消失。
- `cargo check -p northhing`（桌面发布面）：Finished（4m06s），无 error，无跨 crate 破坏。

## Worktree 既有改动说明
- `core.autocrlf=true`，工作区大量文件显示 `M`（CRLF 噪声），`git diff` 内容为空，仅本次 4 文件有真实内容变更。
- `Cargo.lock` 有既存 `+1`（brief 已提及），非本任务产生，未触碰未还原。
- 未追踪文件 `tmp_a.txt` / `tmp_b.txt` / `.superpowers/` 为既有，未触碰。
- 未运行 `cargo fmt`（遵循 brief）。
- 未 git commit。

## 约束逐字遵守
- Logs English-only, no emojis：新增日志（“Skipping plugin with invalid id”、“Refusing to uninstall plugin …”、“Failed to remove staging/temp directory”）均英文无 emoji。
- 严禁裸 `cargo fmt` / `cargo fmt -p northhing-core`：未执行。
- 公开 API 签名变更已 grep 全部调用方并逐一适配（见上清单）。
- 不 git commit。

## 状态
DONE。M-9（ValidatedPluginId 贯穿 + staging 原子化 + containment 复查）与 M-2（async wrapper 驱动 Future）均实现并通过验证；M-2 release 计数测试缺口已说明理由。待 reviewer 审查。
