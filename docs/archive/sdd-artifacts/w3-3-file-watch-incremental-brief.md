# Task 3 (W3-3): F6 — FileWatchService 增量 watch/unwatch

来源与验收标准（逐字，r3-services.md F6）：

> - **Where**: `src/crates/services/services-integrations/src/file_watch/service.rs:87-154`
> - **What**: Every `watch_path` (line 57-72) and `unwatch_path` (line 74-85) call calls `create_watcher()`, which builds a brand-new `notify::RecommendedWatcher`, re-subscribes ALL watched paths (line 100-110), AND spawns a new `tokio::task::spawn_blocking` (line 122) that holds the new `rx`.
> - **Fix direction**: Track the JoinHandle of the spawn_blocking task; on next `create_watcher`, `handle.abort()` the previous one before spawning the new. Or refactor so the watcher is built once and `watcher.watch(path, mode)` / `watcher.unwatch(path)` are called incrementally (notify's `Watcher` trait supports this — see `service.rs:107-109`).
> - **Effort**: S

编排者预检结论（直接采信，2026-08-27 codegraph 核过当前源码）：

- `FileWatchService` 字段：`watcher: Arc<Mutex<Option<RecommendedWatcher>>>`、`watched_paths: Arc<RwLock<HashMap<PathBuf, FileWatcherConfig>>>`（service.rs:23-29）。
- 现行空转行为：watched_paths 为空时 `create_watcher` 把 watcher 置 None（:90-93），后台任务随 rx Disconnected 自终（:141）。
- **方案裁定：增量重构**（fix direction 第二项）——watcher 已存在时直接 `Watcher::watch/unwatch`，不重建、不新起后台任务；消除"全量重注册 + 双任务窗口"两个痛点。不选 JoinHandle-abort 方案（留下全量重建浪费）。

Spec（全部满足）：

1. `watch_path`：插入 `watched_paths` 后，若 `self.watcher` 已存在，对现存 watcher 调 `watcher.watch(path, mode)`（mode 由本次插入的 config 的 `watch_recursively` 决定），失败时返回 Err 且行为与现状同级（错误消息含 path）；仅当 watcher 不存在时才走 `create_watcher()`。
2. `unwatch_path`：移除 `watched_paths` 项后，若 watcher 存在，对现存 watcher 调 `watcher.unwatch(path)`；当 watched_paths 因此变空时维持现行行为（watcher 置 None，后台任务自终）。
3. 任一 watch/unwatch 调用完成后，存活的后台 spawn_blocking 任务数 ≤1（双任务窗口消除）。
4. 新增/扩展聚焦测试：watch 路径 A → 增量 watch 路径 B → unwatch A 之后，B 的事件仍正常投递到缓冲/发射链。测试落点 `src/crates/services/services-integrations/tests/file_watch_contracts.rs` 或就近测试设施。
5. `should_ignore_event` / 事件缓冲 / debounce / `convert_event` 语义零改动；**不动** `assembly/core/src/service/workspace/identity_watch.rs`（同形态但非本审计项，留观察）。

验证：`cargo check --workspace`；`cargo test -p services-integrations` file_watch 相关测试（含新测试）。

## Global Constraints（逐字遵守）

1. 分层边界（根 AGENTS.md 六层）：改动只在指定 crate；不得引入向上的跨层依赖。
2. 日志纪律：新增日志一律英文、无 emoji；warn!/debug! 消息带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 `tokio::select!` / cancellation token / tokio 任务生命周期的改动，必须随附至少一个自动化测试。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`（不 add/commit/restore/checkout/clean）；禁止编辑 `progress.md`；自己的 report 文件用 write 工具写入 `.superpowers/sdd/`，由编排者统一入库。
5. rot-budget：不得上调任何 ceiling；不得新增 >800 行文件。
6. 验证最小集：`cargo check --workspace` + 本任务指定的就近聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息格式对齐近期 git log（`fix(...)` / `refactor(...)`）；commit 不含 `.superpowers/` 产物。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
