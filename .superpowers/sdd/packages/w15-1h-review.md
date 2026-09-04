# Review Package — W15-1h（MemoryDb 迁移竞态修复）

- 分支：`main`，BASE `df38057` → HEAD `7532b2d`（单 commit）
- diff：`git diff df38057..7532b2d`，补丁文件 = `.superpowers/sdd/packages/w15-1h-diff.patch`（27.9KB，仅 2 文件）
- brief：`.superpowers/sdd/w15-1h-brief.md`
- report：`.superpowers/sdd/reports/w15-1h-report.md`

## 任务一句话

CI windows parallel job 红于 `test_search_facts_returns_ok`：`MemoryDb open failed: duplicate column name: status`——`migrate_facts_columns` 的 PRAGMA 检查 + ALTER 是跨连接 check-then-act 竞态（全新 CI 机 + 并行 opener 必现）。修复 = `busy_timeout(5s)` + BEGIN IMMEDIATE 事务化（含 text_fts 回填块）+ 迁移块 `.ok()` 吞错链改显式传播 + 新增 8 线程并发 open 回归测试。

## 验收标准（逐条判 PASS/FAIL，对应 brief §1）

1. `MemoryDb::open` 对每个新连接设置 `busy_timeout`（量级 5 秒）。
2. `migrate_facts_columns` 的 PRAGMA 检查 + 条件 ALTER 在单个 BEGIN IMMEDIATE 事务内执行。
3. text_fts 块（检查 + ALTER + 回填）同样在 BEGIN IMMEDIATE 事务内执行。
4. 两个迁移块的 `.ok()` 吞错点（brief §2 列了 7 处：memory_db.rs:120/122/124/143/158/160/162）全部改为显式错误传播带上下文。
5. 新增并发回归测试：多线程并发 `MemoryDb::open` 同一全新临时路径，全部成功，并验证四列 schema。
6. `cargo test -p northhing-core --features product-full memory_db` 与 `cargo check --workspace` 输出原文在 report 中且与 diff 对得上。
7. diff 只触及允许文件集（`memory_db.rs` + `memory_db_tests.rs`）。
8. 产品行为不变：最终 schema 与 SQL 语句集不变；事务内 re-check 后列已存在则跳过 ALTER（幂等语义保留）。

## Global Constraints（逐字）

- 禁止新增依赖（rusqlite 内置能力足够）。
- 并发改动必带自动化测试（家规④）。
- 界外禁区：`kernel_facade/tests.rs`、`auto_memory.rs`、`.github/workflows/ci.yml` 及其它一切文件。
- 事务实现若用显式 `execute_batch("BEGIN IMMEDIATE")`，错误路径必须 ROLLBACK。
- 测试禁止指向 `<config_dir>/northhing/memory/memory.db`（真实用户配置）。

## 背景证据（审查时参考，非判据）

- CI 失败实证：run 33789958328。
- 根因分析（qwf P1/P3 处方，编排者已逐行核实与代码相符）：`C:\WINDOWS\TEMP\opencode\bench\result2-qwf.md`。
- 本地开发机不重现是结构性的（DB 早已迁移，ALTER 是死代码），serial CI 绿同理——不要以"本地/serial 绿"质疑修复必要性。
