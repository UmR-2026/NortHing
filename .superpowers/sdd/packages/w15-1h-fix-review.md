# Review Package — W15-1h 修复续单（WAL pragma busy 重试）

- 分支：`main`，BASE `f2f3819` → HEAD `976ad9d`（单 commit；这是对已通过审查的 W15-1h 主单 `df38057..7532b2d` 的 CI 回归修复）
- diff：`git diff f2f3819..976ad9d`，补丁 = `.superpowers/sdd/packages/w15-1h-fix-diff.patch`（8.2KB，2 文件）
- 原 brief：`.superpowers/sdd/w15-1h-brief.md`；report（含追加的续单节）：`.superpowers/sdd/reports/w15-1h-report.md`

## 修复对象（CI 实证）

CI run 33859531617 serial job 红：新并发测试 `concurrent_open_fresh_db_all_succeed` panic——`Failed to set WAL mode: database is locked`。机制：journal_mode 转换需排他锁，其 SQLITE_BUSY 不经 busy handler，busy_timeout 覆盖不到。

## 验收标准（逐条判 PASS/FAIL）

1. WAL pragma（及必要的 open 期初始化段）有有界 busy 重试；只有 SQLITE_BUSY / "database is locked" 类错误进重试，其它错误立即短路传播。
2. 重试有界（总预算 ~5s 量级），无死循环风险。
3. 并发回归测试加固（多轮/多线程），断言真实执行，无早退绿。
4. 验证输出原文在 report：focused memory_db 测试绿 + 单测循环 ≥20 轮统计 + `cargo check --workspace` 绿。
5. diff 只触及 `memory_db.rs` + `memory_db_tests.rs`。
6. 原 W15-1h 已验收行为不回归：busy_timeout 仍在、BEGIN IMMEDIATE 事务化仍在、`.ok()` 吞错治理不回退。

## Global Constraints（逐字）

- 禁止新增依赖；禁止削弱已有迁移原子性修复。
- 测试禁止指向真实用户配置目录。

## 背景（非判据）

- 失败日志：run 33859531617 / job 100980541695。
- 最终 CI 终判 = 推送后新 run（本审查不含 CI 观测）。
