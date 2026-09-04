# W15-1h Brief — MemoryDb 迁移竞态修复（busy_timeout + BEGIN IMMEDIATE 事务化 + .ok() 吞错链治理）

## 1. 来源与验收标准（逐字）

来源 = handoff `docs/handoffs/2026-09-04-agy-fixed-benchmark-night.md` §4 队列第 1 条：

> **W15-1h** MemoryDb 迁移竞态：busy_timeout + BEGIN IMMEDIATE 事务化 check-then-act（含 text_fts 回填块）；qwf 的 P3 答案（result2-qwf.md）是现成处方。注意 llm3 发现的次级缺陷：PRAGMA 读取的 `.ok()` 吞错链一并治。

验收标准（逐条可机械核对）：
1. `MemoryDb::open` 对每个新连接设置 `busy_timeout`（量级 5 秒）。
2. `migrate_facts_columns` 的 PRAGMA 检查 + 条件 ALTER 在单个 BEGIN IMMEDIATE 事务内执行。
3. `create_tables` 里 text_fts 块（检查 + ALTER + 回填）同样在 BEGIN IMMEDIATE 事务内执行。
4. 两个迁移块中所有 `.ok()` 吞错点改为显式错误传播（带上下文）。
5. 新增并发回归测试：多线程并发 `MemoryDb::open` 同一个全新临时路径，全部成功。
6. `cargo test -p northhing-core --features product-full memory_db` 与 `cargo check --workspace` 全绿（输出原文进 report）。

## 2. 编排者预检结论（直接采信，勿重复侦察）

故障链（CI run 33789958328 实证：`test_search_facts_returns_ok` 红，`duplicate column name: status`）：

| 事实 | 位置（已核实） |
|---|---|
| `MemoryDb` = `Mutex<Connection>`，per-instance，跨连接不互斥 | `src/crates/assembly/core/src/service/agent_memory/memory_db.rs:8-10` |
| `open` 每次新建 Connection，无 busy_timeout | `memory_db.rs:30-53`（WAL pragma 在 :41） |
| `create_tables` 建表后调 `migrate_facts_columns`，再做 text_fts 检查/ALTER/回填 | `memory_db.rs:55-152` |
| text_fts 块 `.ok()` 吞错点 | `memory_db.rs:120`（prepare）、`:122`（query_map）、`:124`（行迭代）、`:143`（backfill filter_map） |
| `migrate_facts_columns` check-then-act 竞态本体 | `memory_db.rs:154-184`；`.ok()` 吞错点 `:158`（prepare）、`:160`（query_map）、`:162`（行迭代） |
| 竞态触发条件：CI 全新机（DB 文件不存在，必走迁移）+ 并行测试线程 ≥2 个 opener | `kernel_facade/tests.rs:634-646` 两个测试未走隔离缝，直接开机器全局路径 |
| 既有隔离缝（thread-local，`#[tokio::test]` 同线程可解析） | `memory_db.rs:739-790`（`with_test_memory_db_path` / `unique_test_memory_db_path` / `MemoryDbPathGuard`） |
| 产品侧同型竞态被吞 | `auto_memory.rs:302-305`（`Err(_) => return Ok(None)`）——本任务**不动**它（见 §4 界外） |

codegraph blast radius（编排者代查）：`MemoryDb` 的 callers 在 `judge_memory.rs` / `dream.rs` / `mod.rs` / `facts.rs` / `kernel_facade/memory.rs`（每个 facade 调用现开现关）；`open` 签名不变、结构体字段不变 → 对外零影响。同 crate 内无第二个 SQLite 迁移实现可复用。

rusqlite 既有能力（无需新依赖）：`Connection::busy_timeout(Duration)`；立即事务可用 `conn.transaction_with_behavior(TransactionBehavior::Immediate)`（需 `&mut Connection`——`MutexGuard<Connection>` 可 DerefMut）或显式 `execute_batch("BEGIN IMMEDIATE")` … `COMMIT`（错误路径必须 ROLLBACK）。`Mutex` guard 持锁期间整个迁移串行，本就成立。

## 3. 复用侦察（强制）

动手前用 rg/codegraph 确认仓库内无现成的 SQLite 迁移事务化 helper。report 必须有「复用侦察」一节：查了哪些符号、复用了什么、新写了什么已有能力的等价物（逐条给理由）。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. `open` 在 `Connection::open` 成功后设置 `conn.busy_timeout(Duration::from_secs(5))`（5s 是授权值，3–10s 内可自裁，report 说明理由），错误传播带上下文。
2. `migrate_facts_columns`：PRAGMA table_info 读取 + 三个条件 ALTER 全部包进一个 BEGIN IMMEDIATE 事务。事务内 re-check 后列已存在则跳过对应 ALTER（保持现有幂等语义）。
3. `create_tables` 的 text_fts 块（`memory_db.rs:119-149`）：检查 + ALTER + 回填包进 BEGIN IMMEDIATE 事务。**判断点（已授权）**：与 Spec 2 合并为一个事务或各自一个事务均可，report 写明选择与理由。建表 DDL（`CREATE TABLE IF NOT EXISTS`，`memory_db.rs:60-115`）保持现状，不入事务亦可（IF NOT EXISTS 幂等）。
4. 消除以下 `.ok()` 吞错（改为 `?` 传播，`NortHingError::service` 带上下文文案）：`memory_db.rs:120, 122, 158, 160`；行迭代 `:124, :162` 的 `filter_map(|c| c.ok())` 与 `:143` 的 `filter_map(|r| r.ok())` 改为显式收集并传播错误（如 `collect::<Result<Vec<_>, _>>()?`）。
5. 新增回归测试（落在 `memory_db_tests.rs`——`memory_db.rs:892-894` 以 `#[path]` 挂的测试模块）：≥4 个 OS 线程（`std::thread::spawn`）并发 `MemoryDb::open` 同一个**全新**临时路径（用 `std::env::temp_dir()` + uuid 唯一名，参考 `unique_test_memory_db_path` 的造法），所有 open 必须 Ok；随后验证最终 schema 含 status / superseded_by / fact_type / text_fts 四列。测试结束清理临时文件（含 -wal/-shm 侧车）。
6. 产品行为不变：最终 schema、SQL 语句集、单 opener 行为与现状一致；只改变 check-then-act 的原子性与错误传播。

**明确界外（不要碰，judge 见到越界即 Critical）**：
- `kernel_facade/tests.rs` 两个未走缝的测试（隔离化是独立 follow-up）。
- `auto_memory.rs:302-305` 的吞错（产品行为变更，需单独评估）。
- `.github/workflows/ci.yml` 的 serial 重复 job（去除需用户拍板）。

## 5. Global Constraints（逐字遵守）

- 禁止新增依赖（rusqlite 内置能力足够）。
- 并发改动必带自动化测试（家规④，已含于 Spec 5，judge 审查不替代）。
- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit（W7-2 台账被回滚、`5f2771a` 席卷事故）。
- 测试必须真实执行：`cargo check` 绿 ≠ 测试跑过；report 贴测试二进制真实输出原文；环境阻断须明示并交编排者补跑，不得自报 DONE（2026-08-23 m3 交付未运行测试）。
- 涉 keyring / 真实 OS 资源 / 用户真实配置：测试不得触生产存储；本任务测试一律用临时目录新路径，禁止指向 `<config_dir>/northhing/memory/memory.db`。

## 6. 验证（命令 + 输出原文都要进 report）

在仓库根 `E:\agent-project\NortHing` 执行（Windows 下 cargo 走 rustup 前缀）：

```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full memory_db
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace
```

（与 CI 有效 feature 集一致：`cargo test --locked --workspace` 靠 workspace feature 统一从 desktop consumer 带上 `product-full`；单测 `-p northhing-core` 必须显式 `--features product-full`，否则裸 default 编译不过（已知缺口，编排者已在 BASE 实证）。编排者已在 BASE 上预跑第一条，基线绿。）

## 7. 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w15-1h-report.md`。含：改动摘要、Spec 逐条自核、复用侦察节、每个编译错误修在哪一层（机制层/设计层，一行一个）、验证命令 + 输出原文、遗留问题。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`df38057`（main 当前 HEAD）。
- **允许文件集**（diff 越出 = judge Critical）：
  - `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`
  - `src/crates/assembly/core/src/service/agent_memory/memory_db_tests.rs`
- 禁区：其它一切文件（含 ci.yml、kernel_facade/tests.rs、auto_memory.rs）。
- commit 规则：点名 `git add` 上述文件；commit message 走仓库惯例（`fix(core): ... (W15-1h)`）。
- 参考实现处方（直接采信的设计来源）：`C:\WINDOWS\TEMP\opencode\bench\result2-qwf.md` §P3（已核实与现状代码逐行相符）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

## Skill 前置阅读（约束输入，不是需求输入）

- `E:\agent-project\.opencode\skills\rust-skills\m07-concurrency\SKILL.md`（本任务是并发竞态修复）
- `E:\agent-project\.opencode\skills\long-running-shell\SKILL.md`（Windows 下 cargo 长命令的执行纪律）

遵循其中与本任务相关的约定，不因此扩展任务范围。
