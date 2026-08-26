# Task P3b — B3 Cleanup 调度接线（范围收窄版）

来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §B3。独立任务。

## 现状（已核实）

`CleanupService` 完整实现却零调用方：`northhing_core::infrastructure::storage::CleanupService`（cleanup.rs:49-88，`cleanup_all` = temp/logs/cache 三类清理，内部已有 info/warn 日志）。无任何代码实例化它。这就是台账 **P2-4**。

处方原文写落点「apps/desktop/src/lib.rs bootstrap」——**锚点漂移**：真实 bootstrap 在 `src/apps/desktop/src/main.rs`，kernel_facade 初始化 = worker 线程上的 `initialize_core_services()`（main.rs:57-64，经 L151 的 runtime.block_on 执行），Slint/Dioxus 分支在 L182-189。按处方的语义意图落位。

## 改动

### ① main.rs 调度 spawn

在 `initialize_core_services()` 内、`APP_STATE.set_core_ready()` 之后追加：

```rust
// B3 (prescription v3): daily file cleanup scheduler. Runs once at startup,
// then every 24h on the long-lived worker runtime.
tokio::spawn(async move {
    let svc = northhing_core::infrastructure::CleanupService::new(
        northhing_core::infrastructure::PathManager::default(),
        northhing_core::infrastructure::CleanupPolicy::default(),
    );
    let _ = svc.cleanup_all().await;
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(86400));
    loop {
        tick.tick().await;
        let _ = svc.cleanup_all().await;
    }
});
```

要点：
- 导出路径已核实：`infrastructure/mod.rs:19` re-export `PathManager`；`storage/mod.rs:7` re-export `CleanupPolicy/CleanupService`。use 写法按 main.rs 现状整理（可全限定或加 use）。
- `PathManager::default()` 与 `new() -> NortHingResult<Self>` 并存——先读 `app_paths/path_manager.rs` 的 Default 体确认不 panic；若 Default 是 new().expect() 且你判断不妥，改用 `new()?` 上抛（initialize_core_services 已是 Result 路径），report 里记一句选择理由。
- interval 首 tick 即触发 → 启动会连跑两次 cleanup_all。**偏好**：`tokio::time::interval_at(tokio::time::Instant::now() + Duration::from_secs(86400), Duration::from_secs(86400))` 替代裸 interval（一行，消除无害的重复首跑）；用不用都行，report 记录选择。
- `let _ =` 保持处方原样：cleanup_all 内部已有完整 info/warn 观测（"Starting cleanup process" / "Cleanup completed"），外层不再加日志。
- 该 spawn 在 worker 线程的长命 runtime 上，随 L161 recv 阻塞保活到 UI 退出——生命周期正确，无需额外 handle。

### ② 台账 P2-4 改写（同 commit，家规 2）

`docs/status/tech-debt-ledger.md:112-117` 改为：

- Symptom 段保留前半句历史描述，追加一句：`Fixed partially by consult-room P3b (2026-08-26): CleanupService now spawned at desktop startup (once + daily 24h) in main.rs initialize_core_services.`
- Evidence 追加：`main.rs` spawn 点行号（实现后填实际行号）。
- Proposed fix 收窄为剩余两项：(2) Trigger cleanup on session deletion；(3) Include orphaned snapshots in CleanupService —— 并注明：**orphan snapshot 清理需 per-workspace 服务解析**（`FileSnapshotSystem` 挂在每个 workspace 的 `SnapshotService` 内，`service/snapshot/service.rs:36`，无全局实例），属独立立项。
- Status 保持 `active`（部分修复，未全清）。

## 禁区

- 不动 cleanup.rs 本体（既有实现与测试零改动）。
- 不动 Slint/Dioxus 分支逻辑、shutdown_mcp_servers、worker/main 双 runtime 结构。
- 不做 session 删除触发清理（P2-4 剩余项，范围外）。
- 不碰 snapshot 系统。
- 无新依赖。

## 复用侦察（必填进 report）

- 全仓 grep `CleanupService::new` / `cleanup_all(` 确认除 core 自身测试外确无既有调用点/调度器（应无）。
- main.rs 是否已有其它 tokio::spawn 先例可对齐风格（turn_runtime handle 暴露是 set 不是 spawn）。

## 测试判定

家规 4 不触发（无 select!/取消 token/超时竞争新增）。bootstrap spawn 不另测（处方明文）；CleanupService 既有单测覆盖 cleanup_all 本体。你只需保证编译绿 + report 说明此判定。

## 验证（report 必贴命令+尾部输出）

```
cargo check -p northhing
cargo check -p northhing --tests
```

## Report

写 `.superpowers/sdd/reports/task-p3b-cleanup-scheduler-report.md`：改动清单（file:line）、spawn 位置与生命周期论证、Default vs new 选择理由、interval 选型、验证输出尾部。
