# Task B1 Fix 1 — 补并发保护 + 并发写测试（审查 Important-1，用户已拍板）

审查判决：SPEC FAIL / QUALITY PASS（`.superpowers/sdd/task-b1-review.md`）。唯一阻塞项 Important-1。用户 2026-08-05 拍板选 **(a) 加锁 + 并发测试**。本单只修该项，不重做已通过部分。

## 1. 修复要求

在 `services-integrations` `MCPConfigService`（`src/crates/services/services-integrations/src/mcp/config/service.rs`）为**读-改-写窗口**加互斥保护：

- 覆盖 `save_user_config`、`delete_server_config`、`save_project_config` 三条读-改-写路径（user + project 级；project 级虽非 FU-1 范围，但同一 service 实例内共享 `mcp_servers`/`project.mcp_servers` key，锁应统一，避免半保护）。
- 手段：service 实例内 `tokio::sync::Mutex`（或等价异步互斥），锁住 get→改→set 全程。锁粒度 = 单个 `MCPConfigService` 实例；不要求跨实例/跨进程全局锁（超出本债范围，若你认为有必要，记观察项不动手）。
- 读路径（`load_*`）不入锁。
- 参照模式：仓库已有并发不丢更新测试 `concurrent_updates_do_not_lose_entries`（remote_connect 域，grep 定位），照其测试形态写。

## 2. 测试要求

新增并发写测试（`tests/config_and_server_lifecycle.rs`）：
1. 多个 tokio 任务并发 `save_server_config`（User 级、不同 server id）→ 全部完成后条目一个不丢。
2. 并发 save + delete 混合不丢不错（最终态与串行执行某合法顺序一致，或至少断言目标条目存在性符合预期）。
3. project 级同款并发 save 不丢条目（若锁统一覆盖，顺手一个用例即可）。

## 3. 验证命令（贴原文输出进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-services-integrations --features product-full mcp
cargo check -p northhing-core --features product-full
```

并发测试需稳定：连跑 3 次该测试二进制（或 `--count` 等价手段）确认不 flaky。

## 4. 纪律

- 新 commit（勿 amend `d4b11b5`）；只 commit 范围内文件。
- 不裸 `cargo fmt`；日志 English-only；格式手工对齐。
- 家规 4：本次触及并发，测试为硬性交付物。
- commit message 建议：`fix(security): serialize MCP config read-modify-write windows (FU-1 follow-up)`。

## 5. 交付

报告写入 `.superpowers/sdd/task-b1-fix1-report.md`：首行 STATUS；改动清单；锁设计说明（锁什么、不锁什么、为什么）；§3 原文输出（含 3 次稳定性）；观察项。
