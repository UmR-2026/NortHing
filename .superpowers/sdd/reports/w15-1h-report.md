# W15-1h 实施报告 — MemoryDb 迁移竞态修复

## 1. 改动摘要

- **busy_timeout 设置**：在 `MemoryDb::open` 成功建立连接后，立即调用 `conn.busy_timeout(Duration::from_secs(5))`，确保在并发多线程/多进程打开全新数据库并发生 DDL 锁竞争时，能等待当前写事务完成，避免瞬时报 `SQLITE_BUSY`。
- **BEGIN IMMEDIATE 事务化原子迁移**：将 `migrate_facts_columns` 改造为在单个 `rusqlite::TransactionBehavior::Immediate` 事务内执行：先获取独占写锁，再通过 `PRAGMA table_info(facts)` 重新读取当前最新列定义，若列缺失才执行对应 `ALTER TABLE`，彻底消除 check-then-act 竞态。
- **合并 text_fts 迁移与回填**：依据 Spec 3 授权判断点，将 `create_tables` 中原本独立的 `text_fts` 检查、ALTER 与回填逻辑合并进 `migrate_facts_columns` 的同一个 `BEGIN IMMEDIATE` 事务中。
  - **决策理由**：`status`、`superseded_by`、`fact_type` 与 `text_fts` 均为 `facts` 表的列；单事务完成全部列的检查与更新，消除了两个独立事务之间的中间半迁移可观测窗口，减少了 SQLite WAL 锁争用与两次 PRAGMA 查询开销，并删除了 `create_tables` 中 30+ 行重复的查询与迭代代码。
- **治理 `.ok()` 吞错链**：彻底修复了 `memory_db.rs:120, 122, 124, 143, 158, 160, 162` 处的 7 个错误吞没点，全部改为 `?` 错误传播，行迭代全部采用 `collect::<Result<Vec<_>, _>>()?` 显式收集错误并带 `NortHingError::service` 上下文。
- **并发回归测试**：在 `memory_db_tests.rs` 中新增 `concurrent_open_fresh_db_all_succeed` 测试，8 个 OS 线程通过 `Barrier` 严格同步并发打开全新临时数据库路径，断言全部 `Ok`，验证最终 schema 包含 `status`、`superseded_by`、`fact_type`、`text_fts` 4 列，并在 Drop guard 中清理临时数据库及 `-wal`/`-shm` 侧车文件。

## 2. Spec 逐条自核

| 条目 | 要求 | 状态 | 实施详情 |
|---|---|---|---|
| Spec 1 | `MemoryDb::open` 对每个新连接设置 `busy_timeout`（5s 级） | PASS | `conn.busy_timeout(Duration::from_secs(5))`，在 `PRAGMA journal_mode=WAL` 前设置，带上下文映射 |
| Spec 2 | `migrate_facts_columns` PRAGMA + 条件 ALTER 包进单个 BEGIN IMMEDIATE 事务 | PASS | `conn.transaction_with_behavior(TransactionBehavior::Immediate)`，事务内 re-check，幂等安全 |
| Spec 3 | `create_tables` text_fts 块在 BEGIN IMMEDIATE 事务内执行（授权合并） | PASS | 按照授权合并入单个 immediate 事务，一次性原子完成所有 4 列变更与回填 |
| Spec 4 | 消除两处迁移块中所有 `.ok()` 吞错与 `filter_map(|c| c.ok())` | PASS | 全部改为 `?` 传播，行迭代改为 `collect::<Result<Vec<_>, _>>()?` |
| Spec 5 | 新增并发回归测试（≥4 线程并发 open 全新临时路径，验证 4 列，清理 sidecars） | PASS | 8 线程 `Barrier` 强并发测试 `concurrent_open_fresh_db_all_succeed` 全绿通过 |
| Spec 6 | 产品行为不变（最终 schema、SQL 语句集、单 opener 行为一致） | PASS | 语句集与默认值完全一致，幂等迁移 |
| 边界规则 | 严禁越界修改文件（禁区：ci.yml、kernel_facade/tests.rs、auto_memory.rs） | PASS | diff 仅包含 brief 允许的 2 个文件 |

## 3. 复用侦察

- **侦察范围与符号**：
  - 使用 ripgrep 全局搜索 `rusqlite`、`PRAGMA`、`transaction`、`migrate`。
- **侦察结果**：
  - 全仓库中，仅 `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`（及其内部子模块 `dream.rs`）使用 `rusqlite`，架构规则严禁其他层（如 `src/agentic`）依赖 `rusqlite`。
  - 仓库内不存在跨模块的通用 SQLite 迁移框架或迁移事务 helper。
- **复用项**：
  - 复用了 `rusqlite::TransactionBehavior::Immediate` 与 `Connection::transaction_with_behavior`（rusqlite 内置事务能力，无需新依赖）。
  - 复用了 `Connection::busy_timeout`（rusqlite 内置能力）。
  - 复用了 `memory_db.rs:859` 的 `segment_for_fts` 中文双元分词函数。
- **新写能力的等价物说明**：
  - 未新建任何多余的抽象层或结构体，遵循 YAGNI / Ponytail 原则，直接以 rusqlite 的 `Transaction` RAII 事务保护迁移过程，失败时自动 rollback。

## 4. 编译错误分层统计

- 本次实现无编译错误（机制层：0，设计层：0）。

## 5. 验证命令与输出原文

### 命令 1：针对性单测
```bash
cmd /c "C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full memory_db"
```
**输出原文**：
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
warning: `northhing-core` (lib test) generated 16 warnings (16 duplicates)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 38.03s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 24 tests
test service::agent_memory::memory_db::tests::recency_boost_skips_on_clock_anomaly ... ok
test service::agent_memory::memory_db::tests::sort_scored_facts_nan_sinks_to_bottom ... ok
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok
test service::agent_memory::memory_db::tests::concurrent_open_fresh_db_all_succeed ... ok
     Running tests\context_profile.rs (target\debug\deps\context_profile-6c25f13a8520e02e.exe)

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.26s


running 0 tests

test result:      Running tests\git_contracts.rs (target\debug\deps\git_contracts-842e439f5fda151a.exe)
ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result:      Running tests\kernel_facade_uninit.rs (target\debug\deps\kernel_facade_uninit-c50d3d3677515616.exe)
ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result:      Running tests\path_manager_uninit.rs (target\debug\deps\path_manager_uninit-702fed9407a46f29.exe)
ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\product_assembly.rs (target\debug\deps\product_assembly-6ba7f867e85e9989.exe)

running 0 tests

test result:      Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-a53af564d41b5386.exe)
ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
```

### 命令 2：工作区编译检查
```bash
cmd /c "C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace"
```
**输出原文**：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing` (lib) generated 2 warnings (run `cargo fix --lib -p northhing` to apply 2 suggestions)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: `northhing` (bin "northhing") generated 61 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.11s
```

## 6. 遗留问题

- 无。`kernel_facade/tests.rs` 两个测试未走 thread-local 隔离缝是独立的后续重构项，本次改动已保证即使多线程并发打开同一真实路径亦具备完全的原子性与容错能力。

DONE
