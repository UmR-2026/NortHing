# Task 8 Brief: M-9 LSP plugin ID 校验 + M-2 dropped Future

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 9be74ec）
来源：补充报告 M-9（LSP 插件 ID 根目录逃逸）、主报告 M-2（workspace search idle release Future 被丢弃）

## 已核实现状

### M-9 `src/crates/assembly/core/src/service/lsp/plugin_loader.rs`
- `install_plugin_package` L93-144：从 ZIP `manifest.json` 取 `plugin.id`（L125-126）未校验；L128 `plugins_dir.join(&plugin_id)` 绝对路径/ParentDir 可逃逸；L133 `archive.extract(&plugin_dir)`。
- `uninstall_plugin` L147-162：L151 同样 join；L158 `fs::remove_dir_all(&plugin_dir)` 可递归删除根外目录。
- `load_plugin`/`get_server_path`/`get_plugin_dir`（L193-249, 282-289 区间）同款 join 调用点。
- temp staging 目录已存在（L107-114 `.temp-{pid}`），但 extract 直接进最终目录，非 staging+rename。
- zip 4.6.1 的条目级 Zip Slip 有上游防护（enclosed_name），问题只在 extraction root 本身。

### M-2 `src/crates/assembly/core/src/service/search/service.rs`
- L67-69：`pub fn schedule_repo_release(self: &Arc<Self>, ...)` 同步 wrapper 调用 inner async fn 不 await——Future 惰性导致内部 timer/spawn 从不运行，idle 资源不释放（编译器有 unused_must_use warning）。
- inner 实现在 `services-integrations/src/workspace_search/service_session.rs`。

## 需求

### 1. M-9：ValidatedPluginId 贯穿

- 新增 `ValidatedPluginId`（plugin_loader 内或 lsp 模块）：ASCII 字母数字 + `-` + `_`，长度 1..=64；`TryFrom<&str>` 构造即校验（单 `Component::Normal`，拒绝分隔符/盘符/绝对/ParentDir/空）。
- `install_plugin_package`：manifest 反序列化后立即校验 ID，非法 → Err 且**文件系统零副作用**（temp 目录清理）。
- 安装改 staging 原子化：extract 到 `plugins_dir/.staging-{pid}-{nonce}` → 全部校验通过 → `fs::rename` 到最终目录；任何失败只清理 staging。
- `uninstall_plugin`/`load_plugin`/`get_server_path`/`get_plugin_dir` 全部改收 `&ValidatedPluginId`（公开 API 签名变更——grep 调用方适配；非法 ID 在文件系统操作前 Err）。
- 卸载前 containment 复查：canonical 目标必须严格位于 canonical plugins_dir 内，否则 warn + 拒绝 remove_dir_all（纵深防御，参照 Relay Task 1 模式）。

### 2. M-2：schedule_repo_release 真正执行

- 先 grep 全部调用方数量与上下文（同步/异步）。
- 方案 A（优先）：wrapper 改 `pub async fn` + `.await`，调用点适配 `.await`。
- 方案 B（调用点在同步上下文无法改 async 时）：保持签名，`tokio::spawn` 显式执行 + 注释说明所有权转移。
- 修复后该处的 `unused_must_use` warning 必须消失。
- 测试：fake clock/可控 timer 验证 idle release 实际发生（若 inner 测试设施不支持，至少加一个证明 future 被驱动的集成测试：调用后断言 daemon 释放计数变化；做不到就在 report 说明测试缺口与理由）。

### 3. 测试（M-9 必须）

- 非法 ID 用例（参照 Relay validated 测试集）：`../outside`、`a/../../outside`、`/abs`、`C:\x`、`\\unc\x`、混合分隔符、空、`.`、`..`、超长、非 ASCII → install/uninstall 全部在 fs 操作前 Err，**plugins_dir 内外零文件变化**。
- 合法 install→uninstall 往返：staging 无残留。
- staging 中途失败（如损坏 zip）→ 无半安装目录。

## 明确不做

- 不改 LspPlugin manifest schema、不改插件加载执行逻辑。
- 不动 workspace search 的搜索功能本身。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 严禁裸 `cargo fmt` 与 `cargo fmt -p northhing-core`；本任务可以不格式化。
- 公开 API 签名变更（ValidatedPluginId）需 grep 全部调用方逐一适配并在 report 列出。

## 验证命令

```
cargo check -p northhing-core
cargo test -p northhing-core lsp
cargo test -p northhing-core plugin
cargo test -p northhing-core schedule_repo_release
```
（过滤器按实际调整；warning 检查：`schedule_repo_release` 的 unused_must_use 必须消失）

## Report

写 `.superpowers/sdd/task-08-report.md`：改动 file:line、API 调用方适配清单、M-2 方案选择及理由、测试与输出、状态。
