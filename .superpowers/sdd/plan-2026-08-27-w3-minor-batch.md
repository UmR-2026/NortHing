# W3 计划：审计 Minor 残余批量收口（2026-08-27）

来源：`reviews/project-audit-20260826/`（r2-core / r3-services）；handoff `docs/handoffs/2026-08-27-w2-closed-manual-test-pending.md` 队列段。
用户 2026-08-27 拍板：4 任务全上；真机实测与 W3 并行，实测发现随时插入优先；台账"残余人工项"保持开放。

工作模式：沿用 W1/W2 既定模式，main 工作区 `E:\agent-project\NortHing` 直接推进，每任务一 commit，全波后终审。

## Global Constraints（全波通用，reviewer 注意力透镜逐字复制）

1. 分层边界（根 AGENTS.md 六层）：改动只在指定 crate；不得引入向上的跨层依赖。
2. 日志纪律：新增日志一律英文、无 emoji；warn!/debug! 消息带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 `tokio::select!` / cancellation token / tokio 任务生命周期的改动，必须随附至少一个自动化测试。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`（不 add/commit/restore/checkout/clean）；禁止编辑 `progress.md`；自己的 report 文件用 write 工具写入 `.superpowers/sdd/`，由编排者统一入库。
5. rot-budget：不得上调任何 ceiling；不得新增 >800 行文件。
6. 验证最小集：`cargo check --workspace` + 本任务指定的就近聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息格式对齐近期 git log（`fix(...)` / `refactor(...)`）；commit 不含 `.superpowers/` 产物。

## Task 1 (W3-1): r2#5 — 会话创建持久化失败回滚内存插入

来源与验收标准（逐字，r2-core.md Finding 5）：

> - **file:line**: `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs:166-182` — `sessions.insert(...)` and index insert happen, then `save_session(...).await?` can return `Err` with no rollback of the in-memory inserts.
> - **what**: If the initial persist fails, the caller gets `Err` but the session remains in the in-memory map and `session_workspace_index`, consuming one of `max_active_sessions` slots.
> - **fix direction**: On persist failure, remove the just-inserted in-memory entries before returning `Err` (or persist first, then insert).
> - **effort**: S

编排者预检结论（直接采信，不重复侦察）：

- 目标函数：`session_manager_lifecycle.rs` 创建路径（约 :149-182），含 `max_active_sessions` 守卫（:149-154）、`sessions.insert` 与 `session_workspace_index` 插入、`save_session(...).await?`。
- 回滚方案裁定：采用"失败时回滚刚插入的内存项"，**不重排** insert/persist 顺序（重排会改变成功路径语义，超出 Minor 收口范围）。

Spec（全部满足）：

1. `save_session(...).await` 返回 Err 时，函数返回前撤销本次调用刚插入的 `sessions` 项与 `session_workspace_index` 项。
2. 回滚只移除本次插入的键；实现者须先在代码中确认该键在插入前不可能已有同名项（session id 为新生成），并在 report 中给出确认依据。
3. 新增一个聚焦测试：模拟持久化失败 → 断言返回 Err 且 `sessions` map 与 `session_workspace_index` 均无残留。模拟方式由实现者按 crate 内现有测试设施选择。
4. 不改函数签名；成功路径行为零变化。

验证：`cargo check --workspace`；`cargo test -p <assembly/core 实际包名>` 中会话管理相关聚焦测试（含新测试）。

## Task 2 (W3-2): r2#7 + r2#8 — kernel_facade/dto.rs 观测性收口

来源与验收标准（逐字，r2-core.md Finding 7 / 8）：

> Finding 7: `src/crates/assembly/core/src/kernel_facade/dto.rs:72-74` — `serde_json::to_value(p).unwrap_or(serde_json::Value::Null)`. If `compression_payload` fails to serialize, the DTO carries `Null` with no log. **fix direction**: Log a warning on the `Err` arm instead of a bare `unwrap_or(Null)`.

> Finding 8: `src/crates/assembly/core/src/kernel_facade/dto.rs:23-26` — `images.iter().filter_map(|img| img.image_path.clone())`. Multimodal images that carry only a `data_url` (no `image_path`) are filtered out of the DTO silently. The frozen `MessageContentDto::Multimodal.images` is a `Vec<String>` of paths, so data-URL-only images can't be represented. **fix direction**: Document the path-only contract at the filter site, or surface a marker for path-less images rather than dropping them silently.

编排者预检结论（直接采信）：

- 两 finding 同文件 `kernel_facade/dto.rs`，合并一任务；DTO schema 冻结（frozen minimal `KernelEventDto` 体系），**不改任何 DTO 字段形状**。
- Finding 8 裁定：采用"文档化契约 + 可观测丢弃"，不引入 marker 字段（marker 会改 DTO 形状，越界）。

Spec（全部满足）：

1. dto.rs:72-74：序列化失败臂改为 `warn!`（英文，含错误详情）后回落 `Value::Null`；成功臂与 DTO 形状不变。
2. dto.rs:23-26：filter 站点加注释，写明 path-only 契约及 data_url-only 图片无法表示的事实；每次调用若有图片因此被丢弃，记一条 `debug!`（英文，含本次丢弃计数）——不逐张刷日志。
3. 不改函数签名；其它映射函数零改动。

验证：`cargo check --workspace`；dto 相关现有测试全绿（无行为变更，不强制新测试）。

## Task 3 (W3-3): F6 — FileWatchService 增量 watch/unwatch

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

## Task 4 (W3-4): F10 — SSE drain task JoinHandle 跟踪 + 早退 abort

来源与验收标准（逐字，r3-services.md F10）：

> - **Where**: `src/crates/execution/agent-stream/src/stream_processor.rs:425-429`
> - **What**: `tokio::spawn(async move { while let Some(data) = rx.recv().await { ... } })` is detached (no JoinHandle stored in the surrounding scope).
> - **Fix direction**: Track JoinHandle and `abort()` in early-return paths, or rely on the bounded-by-F4 ring buffer to make the drain task cheap.
> - **Effort**: S

编排者预检结论（直接采信）：

- F4（SSE 缓冲上限）已在第一波 I7 收口，drain task 成本已有界；本任务做"JoinHandle 跟踪 + 早退 abort"这一半，使生命周期显式化。
- 属 tokio 任务生命周期改动 → 家规④ 强制带测试。

Spec（全部满足）：

1. stream_processor.rs:425-429 的 drain task 的 `JoinHandle` 被持有（存于函数局部变量或结构字段，由实现者按数据流选择并在 report 说明）。
2. 所有早退路径（含 `graceful_shutdown_from_ctx` 及其余提前返回点）返回前 `abort()` 该 handle；正常流尽路径不 abort（任务随 rx 关闭自终）。
3. 附一个自动化测试覆盖"早退 → drain 任务终止"（实现者选择 agent-stream 内最近测试设施；可用 `JoinHandle::is_finished` / abort 后 `is_aborted` 类断言）。
4. SSE 解析、ring buffer、错误分级语义零改动。

验证：`cargo check --workspace`；`cargo test -p northhing-agent-stream`（含新测试）。

## 终审

全波完成后：`review-package <wave-base> HEAD`，派 `reviewer/step-explore_reviewer` 做全波接缝终审（BASE = a7ac75d 前最后一个代码 commit，即本波第一任务派发前的 HEAD）。
