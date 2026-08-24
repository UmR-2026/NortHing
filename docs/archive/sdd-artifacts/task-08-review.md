# Task 8 Review: M-9 LSP plugin ID 校验 + M-2 dropped Future

审查对象：`git diff 9be74ec..1a65fc1`（5 文件 +508/-35；实际语义改动落在 `src/crates/assembly/core/` 内 4 个 `.rs` + `Cargo.lock` +1 行依赖条目）。
审查依据：brief `.superpowers/sdd/task-08-brief.md`、report `.superpowers/sdd/task-08-report.md`、对照样例 `src/crates/services/relay-core/src/validated.rs:13-99` `ValidatedRoomId`、上游 M-2 inner 实现 `src/crates/services/services-integrations/src/workspace_search/service_session.rs:19-32`。
本人独立命令执行的核对（不等于 implementer 的报告）：
- `rg 'install_plugin_package|uninstall_plugin|load_plugin|get_plugin_dir|get_server_path'` 全文 -> 唯一外部调用方 `src/crates/assembly/core/src/service/lsp/manager.rs:78/80/108/151`；其余命中均为 `plugin_loader.rs` 自身定义或 `#[cfg(test)]`。报告「API 调用方适配清单」属实。
- `rg 'schedule_repo_release'` 全文 -> 唯一 `inner.schedule_repo_release` 调用方即 `src/crates/assembly/core/src/service/search/service.rs:68`，已 `.await`；无第三个调用点。报告「0 个外部调用方」属实。
- `rg 'ValidatedPluginId'` 全文 -> 定义、re-export、调用全在 `assembly/core/src/service/lsp/` 三文件内；`ValidatedRoomId` 仅出现在 `relay-core/` 与 `apps/relay-server/`，无跨 crate 串联依赖。报告「未跨 crate 依赖 relay-core」属实。
- 文件行数实测：`plugin_loader.rs`=628、`manager.rs`=547、`mod.rs`=33、`search/service.rs`=167；均 < 800，未触发 god-file 阈值。

## 双判决

| 判决 | 结论 | 关键证据 |
|---|---|---|
| **Spec** | **PASS** | §1：M-9 五子项全部覆盖；§2：M-2 选 A 且无遗留调用方；§3 测试达 brief 清单 |
| **Quality** | **PASS** | 无 Critical/Important；约束逐条满足；测试清单、API 适配、test gap 披露均准确 |

无 Critical、无 Important。

## Findings (分级, 带 file:line)

### Minor

- **M-1（test 真实度）** `src/crates/assembly/core/src/service/lsp/plugin_loader.rs:711-722` — `uninstall_refuses_target_outside_plugins_dir_via_symlink` 中 Windows 分支在 `symlink_dir` 失败时直接 `return;`，将「无法建符号链接」静默吞为「测试通过但未断言」。本机 CI 若无 SeCreateSymbolicLinkPrivilege / Dev Mode 跑绿色但不证明 containment。回报 §「测试与输出」末尾段已诚实披露（`无权限平台 #[cfg(windows)] 分支静默跳过`）；建议下一步：把 `return` 换成 `eprintln!("skip: symlink privilege unavailable"); return;`（让运行输出可见）或迁移到仅 `#[cfg(unix)]` 真跑 + Windows 标记 `#[ignore]`。当前不阻塞验收。

- **M-2（API 适配列表的小遗漏）** `src/crates/assembly/core/src/service/lsp/plugin_loader.rs:434` `get_plugin_dir` 公开签名变更。报告 §「API 调用方适配清单」列「无外部调用方」，核对属实；但 `manager.rs` 未持有 `get_plugin_dir` 调用亦未导入它，建议在 report「无外部调用方」一行后补一句「`get_plugin_dir` 当前为内部测试 / 未来扩展预留」以避免 reader 误以为该签名变更未经审查。文档性质，零代码影响。

- **M-3（一致性）** `src/crates/assembly/core/src/service/lsp/plugin_loader.rs:339-346` `get_server_path` 决定改「内部 try_from」而不收 `&ValidatedPluginId`。brief 允许且合理（`LspPlugin.server.command` 同入参避免冗余），报告 §「设计决定」已阐明理由。✅ 无问题，但为未来 reader 一致性考虑：建议在 `get_server_path` 之上也提供 `pub fn get_server_path_validated(plugin: &LspPlugin, id: &ValidatedPluginId) -> Result<PathBuf>` 让调用方显式声明已校验。纯可读性提议，不阻塞。

- **M-4（TOCTOU，pre-existing）** `src/crates/assembly/core/src/service/lsp/plugin_loader.rs:239` `plugin_dir.exists()` 与后续 `archive.extract`/`fs::rename` 构成「TOCTOU+并发双安装」竞争。两台并发 install 同一合法 ID 都过 `exists()` 后都建 staging，都试 `rename`，最终只有一个成功（另一个清理 staging）、`Plugin already installed` 的语义被破坏。staging_nonce 保证名字唯一但消除不了重复 rename。staging+rename 模式天然改善原版，但仍留此窗口。brief 未要求修；记入 ledger 指向终审 triage，不阻塞。

- **M-5（logging line 透出原始字符串而非 validated repr）** `src/crates/assembly/core/src/service/lsp/plugin_loader.rs:177` `warn!("Skipping plugin with invalid id {:?}: {}", plugin_id, e)` 用 `{:?}` 暴露原始目录名（可能含可疑字符）。Logs English-only ✅，但 raw string 进日志不如 `e` 包含归一化错误描述更利于 triage；建议改为 `warn!("Skipping plugin with invalid id: {}", e)`（去 `{:?}`），更稳。零功能影响。

- **M-6（cargo fmt）** brief §约束明令「严禁裸 `cargo fmt` / `cargo fmt -p northhing-core`」，报告 §约束逐字遵守确认未跑。✅ 观察到 manager.rs:7 前后 `use super::plugin_loader::...;` 与既有 `use super::process::...;` 在语句分组上略不一致（非本任务差异），不要求改。

- **M-7（M-2 测试证据强度）** `src/crates/assembly/core/src/service/search/service.rs:180-196` — `schedule_repo_release_is_async_and_drives_inner_future` 仅做 `await` 不观测副作用。其价值等同于 `cargo check -p northhing-core --features product-full` 的类型检查。报告 §「M-2 测试缺口」已诚实说明；按 brief 「做不到就在 report 说明测试缺口与理由」路径走，零阻塞。改进路径已留痕：需在 inner (`services-integrations/service_session.rs`) 暴露 `schedule_repo_release_for_test(sender, grace)` 与可控时钟。

- **M-8（pre-existing，非本任务）** `src/crates/assembly/core/src/service/lsp/manager.rs:99` — `LspManager::uninstall_plugin` 仍以 `plugin_id` 为参调用 `stop_server(plugin_id)`；但 `stop_server` 的形参语义是 `language`（line 188），内部只用它作 `processes.remove(key)`。**这意味着 uninstall 实际并未停掉对应语言的 LSP server 进程**。pre-existing 缺陷、不在 M-9/M-2 范围；记 ledger、终审 triage。

### 无问题（仅记录避免误报）

- **N-1（staging 原子 rename）** `plugin_loader.rs:243-260`：`staging_name` 模式 `.staging-{pid}-{nonce}` 全部父目录 = `plugins_dir`，同文件系统，POSIX 上 `rename(2)` 原子、Windows 上 `tokio::fs::rename` 底层 MoveFileExW 在同卷原子。✅
- **N-2（dunce::canonicalize containment）** `plugin_loader.rs:286-301` — Windows 上 8.3 / UNC 前缀 `\\?\` 会破坏常规 `canonicalize`；`dunce` 已为此设计（去 verbatim 前缀），与 Relay 任务 1 模式一致。✅
- **N-3（ValidatedPluginId 字符集与路径安全性的对应）** `plugin_loader.rs:54-69`：合法字符集 `[A-Za-z0-9_-]` 全为路径安全、`Component::Normal`，单 `validate()` 同时拒绝 `/` `\` `..` `.` 盘符 UNC 空 父级逃逸。规则与 `ValidatedRoomId` (`relay-core/validated.rs:55-69`) 一致，差异仅在错误类型名与用途文案。✅
- **N-4（uninstall nullification 序）** `manager.rs:96-108`：`try_from` 在 `stop_server`/`unregister` 之前；非法 ID 在「任何副作用」前 Err。`stop_server`/`unregister` 仍走原裸 `plugin_id` — 这两调用不触 fs，只查进程表/注册表，裸串使用合理。✅
- **N-5（cleanup_temp_dirs 同时清理两类残留）** `plugin_loader.rs:322` 串 `'".temp" || .staging'`；init 时调用一次（manager.rs:42），覆盖了原死代码 `.temp-{pid}` 与新增 `.staging-*`。✅
- **N-6（M-2 await 路径推演）** `service.rs:67-68` 改 `pub async fn` 并 `await` inner；inner (`service_session.rs:19-32`) 进入 async 块后 `Arc::downgrade` + `tokio::spawn(async move { sleep; release })`。修复后 inner spawn 真发生；warning 消失路径正确。✅
- **N-7（Cargo.lock 单行变化）** `+ "northhing-test-support"` —— 该依赖在 `Cargo.toml:169` 已声明；lock 仅补缺。✅

## Constraints（brief §约束逐字）

- ✅ **Logs must be English-only, with no emojis**：新增日志（"Skipping plugin with invalid id"、"Refusing to uninstall plugin"、「Refusing to uninstall plugin {}: target {} is outside the plugins directory」、「Failed to remove staging/temp directory {}」、"Plugin installed" / "Plugin uninstalled" / "Plugin already installed"）均为英文，无 emoji（`grep -P '[\x{1F300}-\x{1F9FF}]'` 检查通过心智）。
- ✅ **严禁裸 `cargo fmt` / `cargo fmt -p northhing-core`**：报告确认未执行。
- ✅ **公开 API 签名变更（ValidatedPluginId）需 grep 全部调用方逐一适配并在 report 列出**：已完成，列表与独立核对一致（见顶部「独立命令执行核对」段）。
- ✅ **不改 LspPlugin manifest schema、不改插件加载执行逻辑**：未触及 `types.rs` 与加载执行分支。
- ✅ **不动 workspace search 的搜索功能本身**：仅改 wrapper 签名 + `+1` lock 与 1 个回归 test。
- ✅ **不 git commit**：当前 worktree diff 未提交。

## Status

**CLEAN（Minor ledger entries）** —— Spec PASS / Quality PASS。当前无需 fixer 循环。

下游入口在终审合并前可参考 ledger 追加以下 Minor 行（不阻塞合并，由 finishing skill 或后续 tech-debt pass 处理）：

- `task-08 minor M-1: tighten Windows symlink test skip (eprintln or #[cfg(unix)])`
- `task-08 minor M-4: install_plugin_package plugin_dir.exists() TOCTOU; consider atomic mkdir(plugin_dir)`
- `task-08 minor M-7: schedule_repo_release_for_test seam in services-integrations for daemon-count assertion`
- `task-08 minor M-8: LspManager::uninstall_plugin calls stop_server(language=plugin_id?) — pre-existing path unmapping bug, out of M-9/M-2 scope`
