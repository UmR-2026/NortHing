# W15-1h 审查报告 — MemoryDb 迁移竞态修复

- 审查者：minimax-m3（judge）
- 包：`df38057` → `7532b2d`（单 commit）
- 允许文件集：`memory_db.rs` + `memory_db_tests.rs`
- 实际 diff：`git diff --name-only` 仅上述两文件，符合允许集
- `git diff df38057..7532b2d -- '**/Cargo.toml'`：空（未触发依赖变更）
- `git diff df38057..7532b2d -- scripts/rot-budget.json`：空（防腐预算未触碰）
- 工作树状态：`git status` 无 WIP 残留，仅未追踪的 review 产物（预期）

## 1. SPEC 判决（双判决第 1 项）

| 条目 | 要求 | 判决 | 证据 |
|---|---|---|---|
| 1 | `MemoryDb::open` 设置 `busy_timeout` 5s | **PASS** | `memory_db.rs:40` `conn.busy_timeout(Duration::from_secs(5))`，错误带 `NortHingError::service` 上下文传播（`:41`） |
| 2 | `migrate_facts_columns` PRAGMA + 条件 ALTER 在单个 BEGIN IMMEDIATE 事务内 | **PASS** | `memory_db.rs:120-127` `transaction_with_behavior(Immediate)`；`:129-138` PRAGMA 读取在 tx 上；`:145-158` 三个 ALTER 全部走 `tx.execute` |
| 3 | text_fts 块（检查 + ALTER + 回填）同样在 BEGIN IMMEDIATE 事务内 | **PASS** | text_fts 检查合入 `:143`；ALTER 在 `:161-163`；回填 SELECT/UPDATE 在 `:164-181`，全部走 `tx`。报告对"合并单事务 vs 各自事务"的判断写明理由（spec 3 授权点） |
| 4 | 7 个 `.ok()` 吞错点全部改显式传播带上下文 | **PASS** | 旧 `:120/122/124/143/158/160/162` 对应位置全部改为 `?` + `NortHingError::service` 上下文文案。详见 §3 残留 `.ok()` 复查（迁移块内 0 处遗留） |
| 5 | 新增多线程并发 open 全新临时路径回归测试 | **PASS** | `memory_db_tests.rs:771-828` `concurrent_open_fresh_db_all_succeed`：8 线程 + `Barrier` 同步（`:789`）；路径 `std::env::temp_dir()` + uuid 唯一名（`:772`）；断言全部 `Ok`（`:804-808`）；最终 `PRAGMA table_info(facts)` 验证 status / superseded_by / fact_type / text_fts 四列（`:821-827`） |
| 6 | `cargo test ... memory_db` 与 `cargo check --workspace` 全绿 | **PASS** | report §5 命令 1 输出：24 tests passed（含 `concurrent_open_fresh_db_all_succeed ... ok`），命令 2 输出：`Finished 'dev' profile ... in 24.11s`，无编译错误 |
| 7 | diff 只触及允许文件集 | **PASS** | `git diff --name-only` 仅 `memory_db.rs` + `memory_db_tests.rs` |
| 8 | 产品行为不变（schema/SQL/单 opener 行为一致，幂等语义保留） | **PASS** | 所有 ALTER 语句默认值（`DEFAULT 'active'` / `DEFAULT 'feedback'` / `DEFAULT ''`）与原代码逐字一致；事务内 re-check 后列已存在则跳过对应 ALTER（`:145/:149/:153/:161` 的 `if !has_xxx` 分支）；`create_tables` 的 `CREATE TABLE IF NOT EXISTS` 未入事务，保持现状 |

## 2. QUALITY 判决（双判决第 2 项）

### 2.1 复用核查（必查项）
- 实现者报告 `复用了 `rusqlite::TransactionBehavior::Immediate` 与 `Connection::transaction_with_behavior`（rusqlite 内置事务能力，无需新依赖）`：独立验证 `rg "rusqlite" src/crates/` 仅在 `memory_db.rs` + `memory_db/dream.rs`（子模块）使用，仓库内无第二个 SQLite 迁移事务化 helper 可供复用。**PASS**
- 报告 `复用了 `Connection::busy_timeout`（rusqlite 内置能力）`：独立验证 Cargo.toml 无新增依赖，`memory_db.rs:2` 仅 import `rusqlite::{params, Connection, TransactionBehavior}` + `std::time::Duration`，无新 crate。**PASS**
- 报告 `复用了 `memory_db.rs:859` 的 `segment_for_fts` 中文双元分词函数`：实际位于 HEAD `memory_db.rs:843`（旧行号 stale，见 Minor §3）。函数本身确实在 `memory_db.rs:178` 被新迁移事务调用，未重复实现分词逻辑。**PASS（line ref stale）**

### 2.2 无 owner 抽象（必查项）
- 新增结构体仅 1 个：`CleanupGuard`（`memory_db_tests.rs:774-785`），定义在测试函数内部，**作用域仅为该测试**，无 trait/interface/公共封装层。
- 无新增配置项、无新建常量化。
- 直接以 rusqlite 的 `Transaction` RAII 事务保护迁移过程，失败自动 rollback（`tx` 变量 drop 即 rollback，`tx.commit()` 显式提交）。
- **PASS**

### 2.3 预算闸（必查项）
- `scripts/rot-budget.json` 未触碰（`git diff` 空）。**PASS**
- 无 ceiling 上调/规则放松。**PASS**

### 2.4 条件早退测试（必查项，2026-09-04 立）
- 复查 `concurrent_open_fresh_db_all_succeed`：
  - 无 `if let Ok(...) = ...` skip-on-failure 模式
  - 无权限/平台条件 guard
  - 8 线程全部走 `Barrier::wait()` 同步、真实 `MemoryDb::open`、真实 `Connection::open`、真实 `PRAGMA table_info` 查询 + 列名断言
  - 测试输出 `... ok`（非 `ignored`），24 passed; 0 failed; 0 ignored
- **PASS（测试真跑、无早退绿）**

### 2.5 god-file 观测点（必查项）
- `memory_db.rs` 行数变化：df38057 = 785 → 7532b2d = 772，**净减 13 行**（迁移块合并去重 + format 折行）。原已 < 800（785），本轮更清。
- `memory_db_tests.rs`：646 → 697（+51，新测试 + CleanupGuard + 几行 format）。仍 < 800。
- 健康度观察：**更清晰**——迁移逻辑单事务化、文本长度缩短、行内折行符合仓库 fmt 风格；新测试与既有 23 测试一致风格（temp_dir + uuid + 清理）。**PASS**

### 2.6 错误回滚（Global Constraints）
- `transaction_with_behavior(Immediate)` 创建的 `tx` 在 drop 时自动 rollback（rusqlite 文档保证）；`tx.commit()` 显式成功才提交。
- 迁移体内任一 `?` 早退均导致 `tx` drop → rollback → DDL 全部回滚，无半迁移窗口。
- 未使用显式 `execute_batch("BEGIN IMMEDIATE")` 路径，故不需要手工 ROLLBACK 分支。
- **PASS**

### 2.7 条件兼容性
- 测试用 `std::env::temp_dir()` + uuid，未触生产路径 `<config_dir>/northhing/memory/memory.db`。**PASS**（Global Constraints §5）
- 测试并发起点为全新路径（无既有 DB 文件），复现 CI"全新机 + 并行 opener"场景。
- **PASS**

### 2.8 日志/错误文案
- 所有 `.ok()` 替换处均带 `NortHingError::service("...具体上下文...")` 文案（如 `:122-126` "Failed to begin immediate transaction for facts migration"、`:131` "Failed to prepare table_info for facts" 等），日志可定位到具体失败点。
- **PASS**

## 3. Cannot verify from diff

| 项 | 说明 |
|---|---|
| busy_timeout 5s 是否在 CI 全新机 + 并行 job 下确实避免 SQLITE_BUSY | 本地开发机 + serial cargo test 无法复现 CI 全新机 + 并行 job 的同型条件；只能依赖 SQLite 文档保证（busy_timeout 对 BEGIN IMMEDIATE 的 lock-acquire 等待生效）。CI 实跑为唯一权威证据（编排者 follow-up） |
| `transaction_with_behavior(Immediate)` 在 rusqlite 当前版本下确实等价于 `BEGIN IMMEDIATE` | 文档保证（rusqlite 0.x）；无源码核对 |

## 4. 总判

### APPROVE

**理由**：8 条 SPEC 全部 PASS；6 条 QUALITY 必查项全部 PASS（含复用真实、无 owner 抽象、未碰 rot-budget、测试无早退绿、god-file 更清晰、错误回滚语义正确）；唯一瑕疵是报告里的 `memory_db.rs:859` 行号 stale（实为 843），不影响代码正确性，记 Minor。

### Findings

**Critical**: 无

**Important**: 无

**Minor**:
- `w15-1h-report.md:34` 引用 `memory_db.rs:859` 的 `segment_for_fts`，HEAD 实际位于 `memory_db.rs:843`。行号 stale，函数本身与复用事实无误。终审 triage 顺手刷一下。

### 留待编排者/CI 终判
- busy_timeout 5s 在 CI windows parallel job 下是否真的避免 `duplicate column name: status` 失败，**唯一权威证据 = CI 实跑**（run 33789958328 是失败样本）。本任务本地 / parallel cargo 全部 PASS；建议在 merge 前先观察一次 CI windows parallel job 全绿再打 APPROVE-PR（如已有现成 CI 触发可走一波观察流，无需本任务追跑）。