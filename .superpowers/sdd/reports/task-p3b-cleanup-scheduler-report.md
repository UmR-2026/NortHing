# Task P3b — B3 Cleanup 调度接线报告

## 改动清单

1. `src/apps/desktop/src/main.rs:66-80`:
   - 在 `initialize_core_services()` 内部、`APP_STATE.set_core_ready()` 之后追加 `tokio::spawn` 异步任务。
   - 实例化 `CleanupService::new(PathManager::default(), CleanupPolicy::default())`，在启动时立即执行一次 `svc.cleanup_all().await`，随后通过 24 小时 `interval_at` 周期调度循环执行。

2. `docs/status/tech-debt-ledger.md:112-117`:
   - 更新 P2-4 技术债条目：
     - Symptom 追加部分修复说明：`Fixed partially by consult-room P3b (2026-08-26): CleanupService now spawned at desktop startup (once + daily 24h) in main.rs initialize_core_services.`
     - Evidence 补充 `main.rs:66-80` spawn 位置。
     - Proposed fix 收窄为剩余两项（session 删除触发与 orphan snapshot 清理），并注明 orphan snapshot 清理需 per-workspace 解析。
     - Status 保持 `active`（部分修复）。

## 架构与选择论证

1. **Spawn 位置与生命周期**：
   - `initialize_core_services()` 在 desktop worker 线程的 multi-thread Tokio runtime 上执行（`main.rs:151`）。
   - worker 线程会被 `shutdown_rx.recv()`（`main.rs:161`）保持存活，直到 UI 退出发送 shutdown 信号。因此在 `initialize_core_services()` 中 spawned 的 `tokio::spawn` 任务会随 worker runtime 长期保持运行，生命周期与应用一致。

2. **`PathManager::default()` vs `PathManager::new()`**：
   - 检查 `PathManager` 实现（`path_manager.rs:124-139`）：`Default` 内部已调用 `Self::new()`，并在 `Err` 时记录日志并安全降级至 `temp_dir().join("northhing")`，无 panic 风险。
   - 选用 `PathManager::default()` 语义明确且不引入额外的 error unwrapping 负担。

3. **`tokio::time::interval_at` 选型**：
   - 裸 `tokio::time::interval` 首个 tick 会立刻触发，导致启动时连续调用两次 `cleanup_all()`。
   - 采用 `interval_at(Instant::now() + Duration::from_secs(86400), Duration::from_secs(86400))` 可精确消除首跑重复，确保启动时显示运行 1 次清理，之后每隔 24 小时运行 1 次。

4. **测试判定**：
   - 本次改动仅为 bootstrap 接线，未新增 `tokio::select!`、cancellation token 或 timeout 竞争逻辑，家规 #4 不触发。
   - `CleanupService` 核心逻辑在 `cleanup.rs` 中已有单测覆盖。按 Task Brief 判定，无需编写新测试。

## 复用侦察结论

- 全仓 grep `CleanupService::new` 与 `cleanup_all(` 确认除 `cleanup.rs` 内部测试外，全仓此前无任何实例化与调度代码（符合 P2-4 预估）。

## 验证结果

执行验证命令：
1. `cargo check -p northhing` -> PASSED
2. `cargo check -p northhing --tests` -> PASSED

### 命令行输出尾部证据

```
warning: `northhing` (bin "northhing" test) generated 38 warnings (run `cargo fix --bin "northhing" -p northhing --tests` to apply 7 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.25s
```
