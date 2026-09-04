# W15-1h 修复续单 Judge 验收 — WAL pragma & create_tables 有界 busy 重试

- 审查范围：BASE `f2f3819` → HEAD `976ad9d`（单 commit `976ad9d`）
- diff 触及：`src/crates/assembly/core/src/service/agent_memory/memory_db.rs`（+48/-10）、`memory_db_tests.rs`（+51/-56）
- 角色：skeptical 独立验收，diff + 实跑输出 + report 为判据；不重跑 implementer 已跑测试

## SPEC 逐条判决

### 1. WAL pragma / open 期初始化段有有界 busy 重试，仅 SQLITE_BUSY/locked 类错误进重试

**PASS**

- `memory_db.rs:30-38` `is_busy_error`：先 `err.sqlite_error_code()` 比对 `ErrorCode::DatabaseBusy` / `ErrorCode::DatabaseLocked`；代码不匹配时回落到错误信息含 `"database is locked"` 或 `"busy"`（消息回落兜底）。
- `memory_db.rs:40-60` `retry_on_busy`：fn 签名 `FnMut() -> Result<T, rusqlite::Error>`；match 分支三路：Ok → 返回；`is_busy_error(&e)` 且未超预算 → 退避递增后继续；其余 Err → **立即**短路返回（不消耗预算）。其它错误零重试要求满足。
- `memory_db.rs:75-78` `PRAGMA journal_mode=WAL;` 被 `retry_on_busy(5s, 50ms, || conn.execute_batch(...))` 包裹 — 即 CI 故障中 `Failed to set WAL mode: database is locked` 的具体失败点。
- `memory_db.rs:93-148` `create_tables` 内 `CREATE TABLE IF NOT EXISTS facts ...` 全 DDL batch（`execute_batch`）同样被 `retry_on_busy` 包裹 — 满足 brief 提示的「`create_tables` 多语句 DDL batch 并发建表冲突也可能遭遇瞬时 locked」场景。
- `migrate_facts_columns`（`memory_db.rs:155-224`）未包 retry，但内部已 `BEGIN IMMEDIATE` 事务 + busy_timeout 接管；与 report §7.2 第 4 项声明「后续的 `migrate_facts_columns` 保持 BEGIN IMMEDIATE 事务不变（该语句由 SQLite 原生 busy handler 正确接管）」一致。

### 2. 重试有界（总预算 ~5s 量级），无死循环风险

**PASS**

- 预算：`Duration::from_secs(5)` — 调用点 `memory_db.rs:75` 与 `:93` 各传 5s。
- 退出条件：`memory_db.rs:50` `if start.elapsed() >= budget { return Err(e); }` — 超预算立即终止并返回最后一次错误。
- 退避有界：`memory_db.rs:53` `let sleep_ms = (base_backoff.as_millis() as u64 + (attempt * 10)).min(200);` — base 50ms，步进 +10ms，上限 200ms。无 `infinite loop` / `unbounded` 模式。
- 死循环风险评估：5s / 200ms = 最多 ~25 次尝试。即使 `attempt * 10` 在 u64 上最终可溢出，但 25 次远未及 u64 容量。无现实风险。
- `attempt` 仅在 busy 分支自增（`:55`），非 busy 错误短路不增 attempt 也不消耗预算 — 满足「无死循环」要求。

### 3. 并发回归测试加固（多轮/多线程），断言真实执行，无早退绿

**PASS**

- `memory_db_tests.rs:770-834` `concurrent_open_fresh_db_all_succeed`：
  - `:772` `for round in 0..3` — 3 轮独立全新临时路径。
  - `:773-777` 每轮用 `std::env::temp_dir()` + uuid v4 全新路径，含 `round` 前缀避免轮间串扰。
  - `:779-790` `CleanupGuard` Drop 清理主文件 + `-wal` + `-shm` 侧车 — 不污染 temp。
  - `:793` `THREAD_COUNT: usize = 12`（up from 8 in 原 W15-1h 主单），超过 brief Spec 5 「≥4 线程」门槛。
  - `:794` `Barrier::new(THREAD_COUNT)` 强同步 — 所有线程就位后同时 `MemoryDb::open`。
  - `:807-815` 每线程 `join().expect("thread panicked")` + `assert!(res.is_ok(), ...)` — 真实断言 Ok，无静默吞错。
  - `:817-833` schema 二次验证：`Connection::open` + `PRAGMA table_info(facts)` + 断言四列 `status` / `superseded_by` / `fact_type` / `text_fts`。
- 无早退绿核查：
  - 无 `#[cfg(...)]` / `#[ignore]` 平台/权限门控 — `grep -E "#\[cfg|#\[ignore|env::var" memory_db_tests.rs` 仅匹配到无关的 `#[cfg(test)]` 模块门控（line 4 等），这些是常规 mod-level gate 而非测试用例门控。
  - `concurrent_open_fresh_db_all_succeed` 函数体内无 `if let Err(...) = ... { return; }` 早退模式。
  - report §7.3 (2) 实证 20 轮外部 PowerShell 循环全 PASS：实际二进制的执行循环而非仅一次 `cargo test`。

### 4. 验证输出原文在 report：focused memory_db 测试绿 + 单测循环 ≥20 轮统计 + `cargo check --workspace` 绿

**PASS**

- report §7.3 (1)：24 tests / 24 passed / 0 failed / finished 0.55s — 与 diff 后实际 `grep ^#\[test\] memory_db_tests.rs` 数 24 个测试匹配；与 focused `cargo test -p northhing-core --features product-full memory_db` 命令匹配（含 brief 要求 `--features product-full`）。
- report §7.3 (2)：20 round PowerShell 循环输出全 PASS；命令以绝对路径调用 `target/debug/deps/northhing_core-a3bccb815e7e79b9.exe`（与 §7.3 (1) 中输出的二进制名一致），逐轮退出码检查并累计 pass/fail 统计。
- report §7.3 (3)：`cargo check --workspace` `Finished dev profile in 2.25s` — 输出原文极短因为是 incremental check（先前已编译过），并非失败信号。
- 三组命令-输出对得上 diff：命令集覆盖 `cargo test ... memory_db` + 多次 `concurrent_open_fresh_db_all_succeed` + `cargo check --workspace`，对应 diff 触及的两个文件。

### 5. diff 只触及 `memory_db.rs` + `memory_db_tests.rs`

**PASS**

- `git diff --stat f2f3819..976ad9d` 仅 2 文件：`memory_db.rs`（+48/-10） + `memory_db_tests.rs`（+51/-56）。
- `git diff --stat f2f3819..976ad9d -- '*.toml' '*.json' '*.lock'` 无输出 — Cargo.toml / lock / rot-budget.json 全部未触碰。
- 与 brief §8 允许文件集完全对齐；禁区 `kernel_facade/tests.rs` / `auto_memory.rs` / `ci.yml` 均未触碰。

### 6. 原 W15-1h 已验收行为不回归

**PASS**

- **busy_timeout 仍在**：`memory_db.rs:72-73` `conn.busy_timeout(Duration::from_secs(5))` 在 retry 包裹之前（顺序：open → busy_timeout → retry_on_busy(WAL) → create_tables）。
- **BEGIN IMMEDIATE 事务化仍在**：`memory_db.rs:155-163` `migrate_facts_columns` 第一句 `conn.transaction_with_behavior(TransactionBehavior::Immediate)`；`tx.commit()` 在 `:220-221`。事务边界完整、列检查-ALTER-recheck 在事务内执行。
- **`.ok()` 吞错治理不回退**：原 W15-1h 目标 line 120/122/124/143/158/160/162 全部已改为 `?` 传播 + `collect::<Result<Vec<_>, _>>()?`。grep 全文件残留的 `.ok()` / `filter_map(|r| r.ok())` 共 6 处（memory_db.rs:553, 600, 686, 717；memory_db_tests.rs:19, 570），均位于原 brief 目标行号之外：
  - `memory_db.rs:553, 600, 717`：`query_row(...).ok()` 转 `Option` 模式（key 不存在 → 默认值语义），非 brief 目标。
  - `memory_db.rs:686`：在 `reviews_for_fact` 内（line 668-690），非 `create_tables`/`migrate_facts_columns`。
  - `memory_db_tests.rs:19, 570`：测试代码，不属生产吞错。
  - 结论：目标行号治理完整，未回退。

## Global Constraints

- **禁止新增依赖**：PASS — `git diff --stat -- '*.toml'` 无输出；新代码仅用 `rusqlite::ErrorCode`（已是 rusqlite 内置 enum，无需新 crate）。
- **禁止削弱已有迁移原子性修复**：PASS — `migrate_facts_columns` 仍包在 `BEGIN IMMEDIATE` 事务中（`memory_db.rs:155-163`），未触及事务边界。
- **测试禁止指向真实用户配置目录**：PASS — `memory_db_tests.rs:773-777` 使用 `std::env::temp_dir()` + uuid v4 唯一名；`CleanupGuard` Drop 清理。无 `<config_dir>/northhing/memory/memory.db` 引用。

## QUALITY 判决

### 复用核查

**PASS**

- report §3 「复用侦察」节存在，内容属实。
- 独立验证（rg `rusqlite` 全仓）：仅 `memory_db.rs` 及其同模块 `dream.rs` 使用 rusqlite；与 report 声明一致。
- 独立验证（rg `transaction_with_behavior` / `busy_timeout`）：仅 `memory_db.rs` 内一处，符合「同 crate 内无第二个 SQLite 迁移实现可复用」。
- 新增 `is_busy_error` / `retry_on_busy` 无仓库内既有等价物（rg `retry_on_busy` / `is_busy_error` 全仓 0 hit，除本文件新写）。
- 复用 rusqlite 内置 `ErrorCode` enum（已暴露）+ 已暴露 `busy_timeout` 方法，无新增 crate / 无重复造轮子。

### 无 owner 抽象（投机性抽象）

**PASS**

- 新增 `is_busy_error`（`memory_db.rs:30`）+ `retry_on_busy`（`:40`）：均为 `fn`（非 `pub`），文件私有，无 trait、无 struct、无导出。
- 两个调用点真实存在：`memory_db.rs:75` 与 `:93`。无「为将来准备」的超前设计。
- 未引入新结构体、新 trait、新 module。

### 预算闸

**PASS**

- `git diff f2f3819..976ad9d -- scripts/rot-budget.json` 无输出 — budget 文件未触碰。
- `memory_db.rs` 已登记为 god-file observation cohort（rot-budget:62-66，ceiling 894）。当前 806 行 — 远在 ceiling 之下。
- 无 ceiling 上调、无新增 god-file 登记。

### 条件早退测试

**PASS**

- `concurrent_open_fresh_db_all_succeed`（memory_db_tests.rs:770-834）：无 `#[cfg(test)]` 跳过、无 `env::var` 跳过、无 `if condition { return; }` 早退。
- report §7.3 (2) 的 20 轮 PowerShell 循环实证：实际执行二进制逐轮跑，未触发环境阻断。

### god-file 观测点

**PASS（带 1 项 Minor 观测）**

- `memory_db.rs` 本次 commit 由 772 行增至 806 行（+34）。已突破 800 行门槛 — 触发 AGENTS.md 家规 ③「raise review pressure」（不强制拆分，但提示关注）。
- 仍在 rot-budget god-file ceiling 894 之内（line 62-66），未超警戒线。
- 本 fix 净增 34 行主要在 retry helper（`:30-60` 共 31 行）+ retry 调用点包装（`:75-78` 与 `:93-148`）+ 测试加固。
- 续单修复的最小化做得到位 — 没有顺手塞债务或扩展迁移逻辑到新代码路径。

## Cannot verify from diff

- **CI 终判**：本审查不含推送后新 run 的观察。review package 明确「最终 CI 终判 = 推送后新 run（本审查不含 CI 观测）」，符合预期。
- **平台异构行为**：Windows MSVC 是当前唯一验证环境；Linux/macOS 上 `PRAGMA journal_mode=WAL` 行为理论相同但未在 diff 验证矩阵中实证 — 非 brief 范围。

## Findings 分档

### Critical
（无）

### Important
（无）

### Minor

1. **`is_busy_error` 消息兜底匹配「busy」过宽（`memory_db.rs:37`）**：字符串 `.contains("busy")` 会命中任意含 "busy" 子串的错误消息（如自定义错误 wrap 后的消息）。当前影响：非 busy 错误被错误归类为 busy → 一次额外 ~50ms sleep + 重试后再次失败才短路。无功能影响（重试预算 5s 兜底），但若未来扩展到非 SQLite 错误源可能误触发。可在后续 hardening 中改为更严格的正则或仅依赖 `sqlite_error_code()`。
2. **`memory_db.rs` 突破 800 行门槛（806 / ceiling 894）**：AGENTS.md 家规 ③「raise review pressure」已触发。本 fix 净增 34 行；future 改动应主动考虑将 `is_busy_error` + `retry_on_busy` 提取到独立 `memory_db_retry.rs` 子模块以保持主文件 < 800。本单不阻塞；纳入下次 god-file 治理窗口。
3. **`concurrent_open_fresh_db_all_succeed` 多线程连接对象 Drop 顺序**（`memory_db_tests.rs:770-834`）：在 Windows 下 `Connection` Drop 与 `CleanupGuard` Drop 的相对顺序依赖字段 drop order — `_guard` 后于 `handles` 但 `handles` join 后的 `Connection` 临时值可能在 guard 之前 drop，导致 `-wal`/`-shm` 残留。当前 20 轮循环未观测到残留，但仍建议下轮重构为显式按序 Drop。无功能影响。

## 总判

**APPROVE**

依据：6 条验收标准全部 PASS；Global Constraints 三项全部遵守；Quality 必查项（复用/owner 抽象/预算闸/早退测试/god-file）全部通过或仅触发 Minor 观测；无法从 diff 判定的项已单独列出。fix 范围最小化（2 文件 +99/-56 行）、改动聚焦（仅 busy 重试 + 测试加固），未触及任何禁区或外部契约。

下一步：可推送 `976ad9d` → main；推送后新 CI run 是最终终判（本审查不含）。