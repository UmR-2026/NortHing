# Task T3a Review — Self-cognition store (storage + one-time migration)

- **Base**: `fd61f5e` (branch `feat/growth-core-0804`)
- **HEAD**: `258d2ea`
- **Material reread this turn**: brief, report, full 856-line diff, all five source files, baseline file (via `git show fd61f5e`), `growth_adapter/tests.rs`, `scripts/core-boundaries/rules/source/forbidden-rules.mjs`, identity.rs, `memory_db/dream.rs`

## Verdicts

- **SPEC**: PASS WITH FINDINGS
- **QUALITY**: PASS WITH FINDINGS

Both required, both pass, but each carries findings the next task must consider.

---

## 专项一 — 幂等性 (Task's stated highest risk)

**Guard source of truth**: `count_self_cognition(db) == 0` ⇒ proceed, else skip. The only mutation is a single `INSERT INTO self_cognition`. No "done" marker is written on the failure path (`self_cognition.rs:174,209`). Guard is observable, persists in SQLite, so survives restarts. Verified by tests `migration_runs_at_most_once_across_multiple_inits` and `migration_idempotent_across_db_reopen`.

Four-case verdict:

| Case | Outcome | Severity | Note |
| --- | --- | --- | --- |
| ① 连续 2/3 次 init | 表 rows 从 1 起不再增 | OK | 测试覆盖 |
| ② INSERT 成功但同流程后续步骤失败 | 不适用 — 迁移函数最后一步就是 INSERT，前面只是只读计数/读文件/计算 mtime | OK | 由代码结构保证 |
| ③ 表已有 T17 写入的**非迁移**笔记，而 `identity.md` 仍在 → 永久跳过迁移 | onboarding 文本永久不进 DB | Important | 下面详述 |
| ④ 两线程/两进程同时初始化同一 DB | TOCTOU race，可产生两条 | Important | 下面详述 |

### (iii) “丢 onboarding 文本”还是“正确”

**我的判断：这是一个由 brief 显式背书的设计取舍，但在工程语义上是“丢用户 onboarding 文本”，应在 T7/后续任务中以更严的迁移标记替代。**

- Brief §3.4 写明：“Repeated initialization must not append a second copy. Derive idempotence from observable state (e.g. table non-empty, or a **recorded migration marker**).” —— 即 brief 同时认可两种 guard，让实施者选一种。
- 实施者选了**较轻**的 `count == 0`。语义后果：表里只要有任何**任何来源**的笔记（非迁移、T17 写入、手工 SQL、测试残余），就永久跳过 identity 迁移。
- 用户感知：*如果将来 T17 在 T3a 之前于某设备的首次启动中写过任何注*（例如一条初始化便笺），那么该设备的 identity.md onboarding 段落永远不会进 `self_cognition`。这是 onboarding 的可见文本，静默丢失。
- 关键的修法（不在 T3a 范围内、应入 T7/之后）：
  - 加独立 `migration_marker(key TEXT PRIMARY KEY, ran_at INTEGER)` 表，迁移成功时 `INSERT`，迁移失败时**不写**。
  - 或在 `self_cognition` 上加一列 `migrated_from TEXT`，迁移记录标 `migrated-from-identity-md`，查询迁移记录用专属列而不是表总数。
- 现在的实现仍然**符合** brief 约束。所以这是 Important，而不是 Critical。

### (iv) 并发初始化

代码每段都 acquire 然后 release `MutexGuard<Connection>`：

```text
count_self_cognition  →  lock  →  COUNT(*)  →  unlock
…  时间窗 …
append_self_cognition →  lock  →  INSERT   →  unlock
```

窗口里另一个线程/进程完全可能 `COUNT` 同样读出 0 再 `INSERT`，产生第二条。没有 `BEGIN IMMEDIATE TRANSACTION` 把两端包成一个原子单元。

- 现存测试 `migration_runs_at_most_once_across_multiple_inits` 是**串行**的，没覆盖并发。
- 当前没有生产并发调用（`init_self_cognition_store` 没有 production caller，brief §3.5 自述）。所以**今天不丢数据**。
- 但是 brief 关心的是“成败可重试”，未要求并发安全；这同 D9 边界的“最小 seam”取舍一样属结构性问题，建议并入 T7 整改项。

**结论：** 找到的临界路径不致生产事故，但仍要登记为 Important 留待 T7 合并修复（推荐：要么把 `count` + `INSERT` 包进一个 `BEGIN IMMEDIATE` 事务，要么采用上面 (iii) 的“迁移标记表”方案让 two-callers 同时尝试时第二次 INSERT 触发 UNIQUE 冲突并被 `OrAbort` 吸收）。

---

## 专项二 — D9 权限面是否被悄悄扩大

`memory_db.rs:50-56` 新增：

```rust
pub(crate) fn conn_locked(&self) -> NortHingResult<std::sync::MutexGuard<'_, Connection>> {
    self.conn.lock()
        .map_err(|e| NortHingError::service(format!("MemoryDb lock poisoned: {}", e)))
}
```

**改动前后能力对照（核心 crate 中持有 `&MemoryDb` 的模块：dream, judge_memory, auto_memory, turn_persist_facts, 及新 self_cognition）**：

- **改动前**：`MemoryDb.conn: Mutex<Connection>` 是**私有字段**（`memory_db.rs:9` 无 `pub(crate)` 标记）。Crate 内任意模块虽然有 `&MemoryDb`，但**只能**通过 `pub(crate)` 的表级方法（`get_judge_mom_value`、`insert_fact`、`get_stale_facts` 等）操作 DB。对 `self_cognition` 表**根本没有 DML 方法**，因为表根本不存在。
- **改动后**：`conn_locked()` 暴露 `MutexGuard<Connection>`。**任何 crate 模块都能用 `db.conn_locked().unwrap().execute("SELECT/INSERT/UPDATE/DELETE FROM self_cognition", ...)`**，绕过 `self_cognition.rs` 的 access module（绕过 append-only 不变量，绕过 D9）。

具体的“事实可读能力”对照：

| 路径 | 改动前能读 self_cognition | 改动后能读 self_cognition |
| --- | --- | --- |
| judge-mom (`judge_memory.rs`) | 不能（无方法、无表） | **能**（raw conn） |
| dream (`dream.rs`) | 不能 | **能** |
| review path（包含 `auto_memory.rs` 用的 reviewer 打分相关路径，以及通过 `MemoryDb::open` 的任何模块） | 不能 | **能** |

**裁决：Important（非 Critical）**。理由：
1. 当前没有 production caller 调用 `conn_locked()`：`grep -r "conn_locked"` 在 `dream/`, `judge_memory/`, `auto_memory/`, `turn_persist_facts/` 中无任何引用（仅 `self_cognition.rs` 和 `memory_db.rs` 自家调用）。
2. D9 的*意图级*不变量——“judge-mom / dream / review 路径不引用 self_cognition”——目前未被违反。
3. 但 D9 的*结构级*执行被削弱：原先靠“私有字段 + 表级方法”构成了不可逾越的边界；现在任何人只要在 crate 里就有一条 escape hatch。这是一处“API 闸门”被打开。
4. Brief §3.2 明确说“put all SQLite access for this table in a **new file** not in `memory_db.rs`… keep any such addition minimal”。“minimal”接受 10 行 seam，但 brief 并未排除“结构性收紧”的可能。

### 推荐修法（按推荐度）

**(a) 推荐**：把 `conn_locked` 私有化，仅做“friend-limited”接口。例如：

```rust
impl MemoryDb {
    pub(crate) fn with_conn<F, R>(&self, f: F) -> NortHingResult<R>
    where F: FnOnce(&Connection) -> NortHingResult<R> {
        let conn = self.conn.lock()
            .map_err(|e| NortHingError::service(format!("MemoryDb lock poisoned: {}", e)))?;
        f(&conn)
    }
}
```

这一版仍允许 self_cognition 模块读 raw conn（用于 SQL 表达），但去掉了“持有 `MutexGuard` 跨越闭包”的能力，调用者只在闭包内有连接句柄可访问，闭包返回后句柄被 drop。这把“连 SQL escape hatch”变成“闭包内地 escape hatch”，结构上仍不完美，但更窄。

**(b) 更彻底**：把 `conn_locked` 完全 private，让 `self_cognition` 模块用 `pub(crate) use super::memory_db::sealed::conn_locked;` 暴露 sealed-reexport。caller 端依旧可见狭缝但更难泛化。代价是 rust 模块 layout。

**(c) 保守**：保持 `pub(crate) fn conn_locked`，但由 T7 在 `scripts/core-boundaries/rules/source/forbidden-rules.mjs` 加规则：
```js
{
  path: 'src/crates/assembly/core/src/service/agent_memory/dream.rs',
  patterns: [{ regex: /\bconn_locked\b/, message: 'dream must not touch raw Connection' }],
}
```
以及 `judge_memory.rs`、`auto_memory.rs`、`turn_persist_facts.rs` 同款。这种做法把 D9 边界转成 CI 检查，避免下次 PR 复发。

### 归 T3a 还是 T7

**建议归 T7，不阻塞 T3a**。理由：

- T3a 范围内 current state 没有违反 D9（无 caller）。
- Brief §3.2 给 `memory_db.rs` 的接口开了“minimal seam”的口子，10 行 `conn_locked` 在不读意图的情况下是合理的最小实现。
- Brief §3.5 已经为“zero production call sites”赢得了 T3a 的范围；把 D9 结构收紧放到 T7（permission matrix）一并处理更连贯。
- 但 T3a 的报告（`task-t3a-report.md` §3.5）应当显式写出这条 D9 风险及责任转交：当前报告 §3.5 只说“没有生产调用点”，没明示这个 seam。**这条报告补充也归 T3a 完成**。

---

## 专项三 — 迁移文本保真

`read_identity_content` 实现 (`self_cognition.rs:109-118`)：

```rust
fn read_identity_content() -> Option<String> {
    let path = resolve_identity_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}
```

逐项核对 brief §3.4 + 一票 edge case：

| 情形 | 处理 | 测试覆盖 | 评估 |
| --- | --- | --- | --- |
| 多行内容 | trim 仅去首尾；内部 `\n` / `\r\n` 保留 | `migration_imports_identity_md_when_table_empty` 用单条字符串断言 | OK |
| CRLF（Windows）| trim 去首尾 `\r\n`；内部保留 | **无显式 CRLF 测试** | OK（但建议补 1 个） |
| BOM (UTF-8 `\xEF\xBB\xBF`) | **未去除** —— `read_to_string` 保留 BOM 到 `text` 字段 | 无 BOM 测试 | **Minor**：与 brief “verbatim” 解释边界模糊；若某 onboarding 路径写入带 BOM 文件则 BOM 会随内容写入。建议在 `read_identity_content` 加 `.strip_prefix('\u{FEFF}')` 或在 write 端剔除 |
| 末尾换行 | trim 去掉 | 测试中 `content = "...直接。"` 用句号结尾覆盖到此 | OK |
| 内容含 SQL 引号 / `%` / `_` | INSERT 用 `params!` 预编译，与 LIKE 无关 | 用 `port_adapter_append_and_load_round_trips` 文本是 ASCII，但未覆盖引号 | OK（参数化已免疫） |
| 内容纯空白 | `trim().is_empty()` ⇒ `None` ⇒ 跳过 | `migration_skipped_when_identity_md_empty_after_trim` | OK |
| 内容极长 | `TEXT NOT NULL` 无长度限制；INSERT 参数化 | 无长度边界测试 | OK（短句预期 50-80 字，brief §2） |
| `created_at`：mtime 可得 | `m.duration_since(UNIX_EPOCH).as_millis() as u64`，正常 | 测试断言 `created_at > 0` | OK |
| `created_at`：mtime 不可得（如文件被另一进程锁）| `self::wall_now_ms()` 兜底 | 无单独测试，但 `_imports_identity_md_` 测试在真实文件上隐式覆盖 | OK |
| `created_at`：pre-epoch mtime（不可能） | `duration_since(UNIX_EPOCH)` 走 Err 分支，`unwrap_or_else(wall_now_ms)` | 无 | OK（`u64` 截断到 `~u64::MAX` 年，几亿年） |

**结论：保真达标，只有 BOM 一项为 Minor 改进建议。**

---

## 专项四 — schema 对既有用户是否安全

`MemoryDb::open` 始终调用 `db.create_tables()?;`（`memory_db.rs:78`），无版本门控。新 `self_cognition` 语句位于既有 `execute_batch`(`memory_db.rs:88-146`)。`execute_batch` 在 SQLite 中按 `;` 切分、autocommit；不是单一事务。

| 问题 | 答 |
| --- | --- |
| ① 批处理是否每次 open 执行？ | 是。`MemoryDb::open` 直接调 `create_tables`，无版本判断、无 feature flag。 |
| ② 旧 DB 改 DB 文件下次 open 是否拿到新表？ | 是。测试 `existing_db_gains_self_cognition_table_on_reopen` 通过：人为 DROP 后 reopen 触发 `CREATE TABLE IF NOT EXISTS` 重建。 |
| ③ 批处理中途某语句失败时的中间态？ | 全部语句都是 `IF NOT EXISTS`（其中 `self_cognition` 是 `CREATE TABLE IF NOT EXISTS`），重 open 自动补齐。**实际无中间态问题**——唯一可失败情况是该批里 SQLite 本身宕了，那时已有 `WAL` 恢复路径。 |

**结论：schema 安全，PASS。**

---

## Findings

### Critical

无。

### Important

**I-1**: `self_cognition.rs` migration guard 选 `count(*)=0` 而非“迁移标记”，导致 (iii) “已有非迁移笔记 ⇒ 永久跳过 identity 导入” 是静默 onboarding 丢失路径。Brief §3.4 显式认可两种 guard，所以不阻塞；但 T7 应引入 `migration_marker(key TEXT PRIMARY KEY, ran_at INTEGER NOT NULL)` 表（或 `self_cognition` 加 `migrated_from` 列），把“迁移已发生”从“表非空”解耦。
- `file:line`: `src/crates/assembly/core/src/service/agent_memory/self_cognition.rs:178-184`
- `fix`: 加迁移标记表并把 I/O 包进 `BEGIN IMMEDIATE` 事务解决 (iv) 并发竞争。

**I-2**: 并发初始化有 TOCTOU race：`count=0` 与 `INSERT` 之间无事务保护。两次同时调用 `init_self_cognition_store` 可产生两条迁移记录（对比测试 ② “三连 init” 是串行，所以测试不覆盖）。
- `file:line`: `src/crates/assembly/core/src/service/agent_memory/self_cognition.rs:171-200`
- `fix`: 在 `count_self_cognition` 与 `append_self_cognition` 之间用 `BEGIN IMMEDIATE`，让 race 窗口内的第二个 caller 因 `WRITELOCK` 等待并重读到 count>0 后返回。建议并入 I-1 的修复中，因为迁移标记表的 `INSERT OR IGNORE` 也能吸收并发。

**I-3**: `pub(crate) fn conn_locked()` 让所有 crate 模块（dream / judge_memory / auto_memory / turn_persist_facts 等）拿到 `&Connection`，可绕过 access module 对 `self_cognition` 做读写——D9 的结构性执行被削弱。
- `file:line`: `src/crates/assembly/core/src/service/agent_memory/memory_db.rs:50-56`
- `fix`: 见专项二推荐 (a)/(b)/(c)。最低限度，T7 在 `scripts/core-boundaries/rules/source/forbidden-rules.mjs` 对 `dream.rs` / `judge_memory.rs` / `auto_memory.rs` / `turn_persist_facts.rs` 加 `\bconn_locked\b` 禁用规则。

**I-4**: 实现者报告 §“Intentionally-unused new production symbols”未提及 `conn_locked` 暴露面给 D9 边界带来的副作用。`task-t3a-report.md` §3.5 只说“no production call sites”，没有写明“any crated caller could now reach raw `Connection`”。责任应转交 T7 并被报告显式承接，否则下一个 reviewer 看不出扣。
- `file:line`: `E:\agent-project\northing\.superpowers\sdd\task-t3a-report.md`（报告路径）
- `fix`: 在报告 “Call sites added (§3.5 - D9 agent-exclusive)” 段尾加一条：“Side note: `pub(crate) fn conn_locked` opens a raw-SQL escape hatch for any crated caller with a `&MemoryDb`. Current crate callers (dream/judge_memory/auto_memory/turn_persist_facts) do not use it; T7 owns the forbidden-rules.mjs entry that will enforce this.”

### Minor

**M-1**: `read_identity_content` 不剥 UTF-8 BOM。文件写入侧若引入 BOM（例如某些编辑器“UTF-8 with BOM”设置）会随内容落到 `text` 字段，对 prompt 注入有可见副作用。
- `file:line`: `src/crates/assembly/core/src/service/agent_memory/self_cognition.rs:111-117`
- `fix`: 在 `std::fs::read_to_string(&path).ok()?` 后 `let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content); let trimmed = content.trim();`
- `test`: 加 `migration_strips_utf8_bom_when_importing_identity` 覆盖。

**M-2**: `self_cognition.rs:130,141` 使用 `unwrap_or_else(...)` / `unwrap_or(...)`。Brief §4 写“No `unwrap` / `expect` / `panic!` in non-test code”。这两处为受控 fallback（不会 panic），是项目既有惯用法（`memory_db.rs` 多处同款），不阻塞；但严格按字面规则算边缘案例。已知合规补丁：把 fallback 抽成显式 `match`。
- `fix`: 抽 `wall_now_ms_or(0)` 为 `match SystemTime::now().duration_since(UNIX_EPOCH) { ... }`。

**M-3**: `self_cognition_tests.rs` 在每个测试内部手工 `fs::remove_file(&db_path)` 而不是用 `MemoryDbPathGuard` RAII。已有 brief §5 提到 RAII helpers，但未强约束；不阻塞。
- `file:line`: 散在 `self_cognition_tests.rs:168, 194, 221, 247, 270, 305, 373, 394`
- `fix`: 用 `with_test_memory_db_path` + `MemoryDbPathGuard` 替换；或保留现写法并加注释解释。

**M-4**: 报告未声明 CRLF 兼容性与 BOM 处理的测试覆盖或选择。
- `file:line`: `E:\agent-project\northing\.superpowers\sdd\task-t3a-report.md`（“Verification” section 3 的 test-mapping 段）
- `fix`: 在那里追加两行：「CRLF: trimmed out by outer trim (internal \r\n preserved); BOM: NOT stripped — see review M-1」.

**M-5**: `growth_adapter.rs:184-193` 的 `load_self_cognition` warn-only helper 没有生产调用点（仅测试通过 alias `load_self_cognition_notes` 使用），但实现了并被测试。报告第 7 行把它列为“warn-only loader for future consumer”。本任务这点合规——保留即可，留待 T17 接线。

**M-6**: `growth_adapter.rs` 从 266 → 364（+98）。这一增长完全合理（平行 `JudgeMomStateStore` 模式添加 `SelfCognitionDbStore`），无需挪位（brief §3.3 明确说这里）。但报告未提一句“为什么在这里”作 deflect 防御。可以在报告里加一句“增长归 `growth_adapter.rs` 是 brief §3.3 显式指定的位置（与 `JudgeMomStateStore` 同模式）”。
- `file:line`: `E:\agent-project\northing\.superpowers\sdd\task-t3a-report.md` “Line counts” 段。
- `fix`: 加一行脚注说明。

---

## 无法从 diff 验证的项（编排者需亲自核）

下面这些 orchestration 已亲验但本 review 不能重跑，列出以便你交叉：

1. **`cargo test -p northhing-agentic-growth` 仍是 139**：未重跑；编排者已验证。
2. **`cargo check -p northhing-core --features product-full` 仍是 19 warnings**：本 review 重跑了一次，仅确认 15 个 `self_cognition` 测试通过且没有 self_cognition/growth_adapter/conn_locked 相关的新 warning，但未完整捕获 baseline 列表。
3. **`node scripts/check-core-boundaries.mjs` exit 0**：未本 review 重跑（编排者已验）。需要确认：T7 增加 `\bconn_locked\b` 禁用规则前，此检查不会主动拦截当前实现；T3a 通过。
4. **`cargo test -p northhing-core --features product-full prompt_injection` 4 passed**：未本 review 重跑（编排者已验）。
5. **`git diff --stat -- src/.../prompt_builder/**` 为空**：本 review 跑了一次（在前文 `git diff --stat -- "src\.../prompt_builder\**"`），返回空，符合预期。
6. **行数**：本 review 重计，与报告一致：`memory_db.rs=961`、`growth_adapter.rs=364`、`self_cognition.rs=257`、`self_cognition_tests.rs=395`、`mod.rs=22`。
7. **`self_cognition.rs` 内无 UPDATE/DELETE SQL**：grep 内只匹配到 doc 注释里的字面值。
8. **`prompt_builder/**` 零改动**：grep 已验证。
9. **`mod.rs:19` re-export 列表正确**：`pub(crate) use self_cognition::{append_self_cognition, count_self_cognition, load_self_cognition, migrate_identity_into_self_cognition, SelfCognitionRow}`，5 个符号都真实存在且可见范围匹配 (`pub(crate)`)。
10. **`MemoryDb::open` 调用 `create_tables()` 无版本门控**（`memory_db.rs:78` 直接调用）已确认。

---

## 一句话总结

SPEC 与 QUALITY 均通过：四张表都建在既有 `CREATE TABLE IF NOT EXISTS` 批里、迁移采用 observable-state guard、文本保真（除 BOM）、D9 边界意图级守住。但**留四个 Important 给 T7/后续**：(iii) “表非空” guard 的 onboarding 永久跳过风险、(iv) 并发 TOCTOU 竞态、`conn_locked` 暴露的 D9 结构执行弱化、报告未声明该弱化。第 ④ 条报告补充可在 T3a 闭环，其他三条转 T7 修复合适。

---

# Round 2 复审

- **Base**: 258d2ea
- **HEAD**: 39fadea
- **Diff**: 	ask-t3a-diff-round2.md (393 行)
- **Material reread this turn**: brief, round-1 review, round-2 report §"Round 2", round-2 diff, all three modified source files at HEAD, the renamed test in full, both new BOM/newlines tests in full, mod.rs, growth_adapter.rs (确认 round-2 未动), prompt_builder/ (确认 round-2 未动), scripts/core-boundaries/ (确认 round-2 未动).

## Verdicts

- **SPEC**: PASS
- **QUALITY**: PASS WITH FINDINGS

Orchestrator 已在 Round 1 报告中划线裁定 I-1/I-2 的"迁移标记表 + BEGIN IMMEDIATE"方案不入本任务，改用"确定性主键 + INSERT OR IGNORE"。本轮即在该方案下判定闭环。

---

## Round 1 findings 闭环状态

### Important

**I-1（onboarding 永久跳过）**: **CLOSED** — Round 2 改用 migration-row identity。判定依据见下方 §"I-1/I-2 闭环判定 ① ③ ④"。
- 实现位置：src\crates\assembly\core\src\service\agent_memory\self_cognition.rs:50（MIGRATION_ROW_ID = "migration:identity-md"），:111-124（insert_migration_row 用 INSERT OR IGNORE），:258-276（migrate_identity_into_self_cognition 调 insert_migration_row，根据 Ok(true)/Ok(false)/Err 三路分支）。
- 测试覆盖：migration_runs_even_when_table_has_non_migration_note (self_cognition_tests.rs:257-295) 显式断言：pre-seed 1 note + init ⇒ ows.len() == 2，migration 行 id == "migration:identity-md"、	ext == content、	rigger == "migrated-from-identity-md"，pre-existing 行原样保留。该测试**真的断言了新语义**（不是测试名漂亮、断言反向；已逐条核对断言表达式）。

**I-2（并发 TOCTOU）**: **CLOSED**（结构正确，无并发测试但当前无生产并发路径）。判定依据见下方 §"I-1/I-2 闭环判定 ②"。
- 关键事实：INSERT OR IGNORE 在 SQLite 下对 PRIMARY KEY 冲突的行为——SQLite 在执行单条 INSERT 时持有隐式事务下的写锁；PRIMARY KEY 冲突时 OR IGNORE 子句把错误转换成"该行不写入"，execute() 返回 0 rows affected（self_cognition.rs:117-123 用 Ok(inserted > 0) 区分）。两个并发 initializer：第一个拿锁 → INSERT → 提交 → 释放；第二个排队拿锁 → INSERT → PK 冲突 → OR IGNORE → no-op → 提交（空） → 释放。**两 migration 行永远不可能出现**。
- WAL：DB PRAGMA journal_mode=WAL 已设（memory_db.rs:88）。WAL 不改变 PK 约束语义，只让读者不阻塞写者；两个写者仍然串行化。
- busy timeout：DB 未设 PRAGMA busy_timeout。若并发 INIT 的第二个 writer 在拿锁时遇到锁等待而 busy timeout 默认未设，可能返回 SQLITE_BUSY。该错误经 insert_migration_row 的 .map_err(...) 转换为 NortHingError::service(...) 并向上抛；migrate_identity_into_self_cognition:270-275 接住 Err(e) 路径，warn log + return，**不留行**。下一个 init 调用同一路径可重试。这符合 brief §3.4 "Migration failure must be warn-only and must never block store creation; a failed migration must be retryable"。
- 未补并发测试：与 round 1 同——migration_runs_at_most_once_across_multiple_inits 是串行三连 init。但当前**没有生产调用点**（init_self_cognition_store 仅在 growth_adapter.rs:176 定义与 :177 由 migrate_identity_into_self_cognition(db) 调用，无生产调用方），因此今天不存在真实并发路径。Brief 未要求并发安全；新方案的 PK 约束为 T3b 接入并发 caller 时天然提供了正确性。
- 是否需要 BEGIN IMMEDIATE：不需要。单条 INSERT OR IGNORE 在 implicit transaction 下已是原子单元；BEGIN IMMEDIATE 仅在把"读 + 写"包成一个事务时才相关，而本设计**完全不读就判断**——INSERT OR IGNORE 本身就是判断。

**I-3（conn_locked D9 结构弱化）**: **NOT ADDRESSED IN THIS TASK**（orchestrator 裁定归 T7）。
- 实现：未改（仍 pub(crate) fn conn_locked at memory_db.rs:69）。grep 确认 dream/judge_memory/auto_memory/turn_persist_facts 五个兄弟模块均**未调用** conn_locked（grep -rn conn_locked src\crates\assembly\core\src\service\agent_memory 只匹配 memory_db.rs 三处 + self_cognition.rs 四处）。
- 仅确认 I-4 已闭环（见下）。

**I-4（报告未声明 conn_locked 副作用）**: **CLOSED**。
- 代码内 doc：memory_db.rs:47-68 共 19 行 doc comment，明示四点：
  1. "改动前读不到 self_cognition"：memory_db.rs:55-58（"Before this helper existed, MemoryDb.conn was a private field ... judge-mom / dream / review paths could not reach self_cognition (the table did not exist)"）。
  2. "改动后读得到 self_cognition"：memory_db.rs:53-54（"any crate-internal caller with a &MemoryDb can issue arbitrary SQL against any table, including self_cognition"）。
  3. "D9 禁止"：memory_db.rs:60-63（"D9 requires that judge-mom, dream/garden, and the review path never read or write self_cognition -- not even a read. This helper weakens that structural enforcement"）。
  4. "T7 强制"：memory_db.rs:65-68（"hard enforcement (a orbidden-rules.mjs entry banning \bconn_locked\b in those files) is owned by T7's permission-matrix work"）。
- 报告内 §3.5 子节：	ask-t3a-report.md:86-106 标题 "D9 side effect: conn_locked escape hatch (I-4, addressed in round 2)"，同样覆盖四点。当前 crate 调用方列举（dream.rs / judge_memory.rs / auto_memory.rs / turn_persist_facts）明示未使用。
- 文档诚实度：四点全部"如实、完整"。本节无遗漏。

### Minor（必修）

**M-1（BOM 剥除）**: **MOSTLY CLOSED**，留一条 Minor edge case 观察（见下方 §"M-1 BOM 闭环判定"）。
- 单 BOM 在 position 0：BOM 测试 migration_strips_utf8_bom_when_importing_identity (self_cognition_tests.rs:372-392) 用 "\u{FEFF}{}\n" 写出，断言 ows[0].text == body（无 BOM，无尾换行）。实现 self_cognition.rs:158-169：先 strip_prefix('\u{FEFF}') 再 	rim()，对单 BOM + 尾空白场景正确。
- BOM 后紧跟空白："\u{FEFF} body" ⇒ strip_prefix 去掉 BOM ⇒ " body" ⇒ 	rim() ⇒ "body"。✓
- 内部换行（多行内容）：migration_preserves_internal_newlines_in_identity_md (self_cognition_tests.rs:395-413) 用三行 body 加首尾 \n 写出，断言内部 \n 保留、首尾空白被 trim。✓ 与 brief "trimmed of surrounding whitespace, otherwise verbatim" 一致。
- UTF-16 风格 BOM 字节：UTF-16 LE (\xFF\xFE) 与 BE (\xFE\xFF) 均不是合法 UTF-8 起始字节。std::fs::read_to_string 会返回 io::Error(InvalidData)；ead_identity_content 的 .ok()? 链路把它转成 None；migrate_identity_into_self_cognition:241-247 接住 None、debug log、return。**不 panic、不写入、行为正确**（skip migration）。

### Minor（留终审 triage，本轮不看）

**M-2 / M-3 / M-5 / M-6**：按 orchestrator 裁定留终审 triage，本轮不评估。M-6 报告补充（growth_adapter 增长归因 brief §3.3）已在 round 2 报告 	ask-t3a-report.md:687-689 加一行脚注，归口关闭。

---

## I-1/I-2 闭环判定（四种 case）

| Case | 实现 | 验证 | 判定 |
| --- | --- | --- | --- |
| ① 串行多次初始化 | INSERT OR IGNORE + PK，第一次 inserted > 0，第二次起 inserted == 0，函数返回 Ok(false) 路径 migrate_identity_into_self_cognition:265-269 仅 debug log | migration_runs_at_most_once_across_multiple_inits (self_cognition_tests.rs:174-195) 三连 init，断言 ows.len() == 1、text/trigger 不变 | **CLOSED** |
| ② 并发两个初始化 | 见上方 I-2 段：INSERT OR IGNORE + PK 串行化；无并发测试，但当前无生产并发路径；最坏情况（SQLITE_BUSY）走 Err 路径不留行，可重试 | 无并发测试；结构正确 | **CLOSED（结构），未并发实测** |
| ③ INSERT 失败可重试不留"已完成"痕迹 | 唯一 mutation 是 INSERT OR IGNORE（self_cognition.rs:117-123）；execute 任何错误（含 SQLITE_BUSY）经 .map_err 转 NortHingError::service(...)，向上抛；migrate_identity_into_self_cognition:270-275 warn log + return；无 marker 表、无 done flag | 无失败重试显式测试；但结构上 migrate 函数体只做"读 identity.md → 计算 created_at → 单条 INSERT OR IGNORE"，任何前置步骤失败也走早返回路径（migrate_identity_into_self_cognition:241-247 identity 缺失 / 空内容早返回），无状态变更；与 brief §3.4 一致 | **CLOSED（结构），未失败注入测试** |
| ④ 迁移行已存在 + identity.md 内容已变 ⇒ 既不覆盖也不追加 | PK 冲突 + OR IGNORE ⇒ Ok(false) 路径只 debug log；迁移行内容保留 | migration_does_not_overwrite_or_append_when_identity_md_changed (self_cognition_tests.rs:335-367) 显式：先 init 写 original，再 s::write(&id_path, "完全不同的自我认知内容。")，再 init，断言 ows.len() == 1、ows[0].text == original、ows[0].trigger == "migrated-from-identity-md" | **CLOSED** |

**Round 1 专项一③ 闭环判定**（预置非迁移笔记 + identity.md 存在 ⇒ 迁移仍执行）：
- 旧测试 migration_skipped_when_table_non_empty 断言 ows.len() == 1、pre-existing 行原样保留 → 这是 round 1 的错误语义。
- 新测试 migration_runs_even_when_table_has_non_migration_note (self_cognition_tests.rs:257-295) 断言 ows.len() == 2、按 trigger 过滤得 1 migration 行 + 1 manual 行、migration 行 id == "migration:identity-md"、	ext == content。逐行核对：第 270-274 行 ssert_eq!(rows.len(), 2, ...)、第 280 行 ssert_eq!(migration_rows.len(), 1, ...)、第 281 行 ssert_eq!(migration_rows[0].text, content)、第 282-285 行 ssert_eq!(migration_rows[0].id, "migration:identity-md", ...)、第 287-292 行 ssert_eq!(manual_rows.len(), 1) 与 ssert_eq!(manual_rows[0].text, "pre-existing note")。**断言与新语义一致，不是测试名漂亮/断言反向**。

---

## M-1 BOM 闭环判定（edge cases）

实现顺序（self_cognition.rs:158-169）：

```rust
let content = std::fs::read_to_string(&path).ok()?;
let stripped = content.strip_prefix('﻿').unwrap_or(&content);
let trimmed = stripped.trim();
```

| 输入 | 行为 | 测试覆盖 | 判定 |
| --- | --- | --- | --- |
| 单 BOM 在 position 0（"\u{FEFF}body\n"） | strip ⇒ "body\n"，trim ⇒ "body" | migration_strips_utf8_bom_when_importing_identity (self_cognition_tests.rs:372-392) | ✓ |
| 多行内容（"\nbody\n"） | trim ⇒ "body"，内部 \n 保留 | migration_preserves_internal_newlines_in_identity_md (self_cognition_tests.rs:395-413) | ✓ |
| BOM 后紧跟空白（"\u{FEFF} body"） | strip ⇒ " body"，trim ⇒ "body" | 无显式测试，结构正确 | OK |
| UTF-16 BOM 字节（\xFF\xFE / \xFE\xFF） | 非合法 UTF-8 ⇒ ead_to_string 返回 InvalidData ⇒ .ok()? ⇒ None ⇒ migration skip + debug log | 无显式测试 | OK（结构保证） |
| **多连续 BOM**（"\u{FEFF}\u{FEFF}body"） | strip_prefix 只去一个 BOM ⇒ "\u{FEFF}body"；	rim 不动 BOM ⇒ 存储 "\u{FEFF}body" | **无测试**，会**残留一个 BOM** | **Minor edge case 未覆盖** |
| **BOM 前有空白**（" \u{FEFF}body"） | strip_prefix 不匹配（首字符是空格）⇒ 原样返回 ⇒ 	rim 去空格 ⇒ "\u{FEFF}body" ⇒ 残留 BOM | **无测试** | **Minor edge case 未覆盖** |
| 仅 BOM（"\u{FEFF}"） | strip ⇒ ""，trim ⇒ ""，is_empty ⇒ None，skip | 无显式测试，但走"empty after trim"早返回路径 | OK |

**Minor 观察**：当前实现对**单前导 BOM + 尾空白**主流场景正确，对**多连续 BOM** 与**BOM 前有空白**这两个低概率 edge case 会残留 BOM。修法是 content.trim_start_matches('\u{FEFF}') 替换 strip_prefix('\u{FEFF}')（再保留后续 	rim()）；但考虑到：

1. 编辑器不常输出多 BOM；BOM 前留空白几乎不会发生；
2. 今天没有生产调用 init_self_cognition_store；
3. BOM 残留在 prompt 注入链路的影响是 1 个不可见 ZWNBSP 字符，肉眼难察。

记为 **Minor**，建议留待 T7 一并处理或在新文件加一个 migration_strips_consecutive_boms_when_importing_identity 测试覆盖。

---

## I-4 闭环判定

四点核对（已在 I-4 段列过行号，此处不重复）：

| 四点要求 | 位置 | 文句要点 |
| --- | --- | --- |
| 改动前读不到 self_cognition | memory_db.rs:55-58 | "Before this helper existed, MemoryDb.conn was a private field ... judge-mom / dream / review paths could not reach self_cognition" |
| 改动后读得到 self_cognition | memory_db.rs:53-54 | "any crate-internal caller with a &MemoryDb can issue arbitrary SQL against any table, including self_cognition" |
| D9 禁止 | memory_db.rs:60-63 | "D9 requires ... never read or write self_cognition -- not even a read" |
| T7 强制 | memory_db.rs:65-68 | "hard enforcement ... orbidden-rules.mjs entry banning \bconn_locked\b ... is owned by T7" |

报告内 	ask-t3a-report.md:86-106 "D9 side effect" 子节亦覆盖同样四点。**CLOSED，如实完整。**

---

## 回退检查

逐条核对 Round 1 已通过项在 Round 2 后状态：

| 项 | Round 1 状态 | Round 2 后状态 | 证据 |
| --- | --- | --- | --- |
| self_cognition 表无 workspace 列 | OK | OK | memory_db.rs:160-165 CREATE TABLE 仍只列 id, text, trigger, created_at 四列，无 workspace_key |
| self_cognition 仅追加（无 UPDATE/DELETE） | OK | OK | self_cognition.rs 内 UPDATE\|DELETE 匹配只在 :9-10（模块 doc）、:80（函数 doc），无 SQL 语句；grep 已确认 |
| load 排序 created_at ASC, id ASC | OK | OK | self_cognition.rs:60 ORDER BY created_at ASC, id ASC 未变 |
| identity.md 非破坏 | OK | OK | self_cognition.rs:158-169 ead_identity_content 仅 ead_to_string + 内存变换，无写入；esolve_identity_path 也只读 |
| prompt 构建文件零改动 | OK | OK | git diff --stat -- src\crates\assembly\core\src\agentic\agents\prompt_builder\** 空输出 |
| 未改 src/agentic/** | OK | OK | git diff fd61f5e..HEAD -- src/agentic/ 空输出 |
| 未改 scripts/core-boundaries/** | OK | OK | git diff fd61f5e..HEAD -- scripts/core-boundaries/ 空输出 |
| mod.rs re-export 列表 | OK | OK | mod.rs:19 仍 pub(crate) use self_cognition::{append_self_cognition, count_self_cognition, load_self_cognition, migrate_identity_into_self_cognition, SelfCognitionRow}，5 个符号都在 self_cognition.rs 中以 pub(crate) 形式存在 |
| MemoryDb::open 无版本门控 | OK | OK | memory_db.rs:88-97 PRAGMA journal_mode=WAL; → create_tables() 顺序未变 |
| crate 139 / core warnings 19 / self_cognition 18 / memory_db 28 / growth_adapter 30 / prompt_injection 4 / boundaries exit 0 | OK | OK | 编排者已亲验，本轮未重跑 |

**无回退。**

---

## count_self_cognition 生产调用者检查

self_cognition.rs:129-135 定义 pub(crate) fn count_self_cognition(db: &MemoryDb) -> NortHingResult<i64>，doc comment 已写明 "Diagnostic only; not used as the migration guard"（:126-128）。mod.rs:19 re-export。

**调用者清单**（grep -rn count_self_cognition src\crates\assembly\core\src）：

- mod.rs:19 —— re-export 定义点
- self_cognition.rs:129 —— 函数定义点
- self_cognition_tests.rs:19, 147, 208, 215, 265 —— 5 处全在测试代码内（其中 :19 是注释，:147/208/215/265 是 4 处 ssert_eq!(count_self_cognition(&db).unwrap(), ...) 断言）

**生产路径零调用**。growth_adapter.rs 不引用；uto_memory.rs / dream.rs / judge_memory.rs / 	urn_persist_facts.rs 不引用。

**评估**：可见性过度。当前 pub(crate) + crate-level re-export 对一个"仅诊断"函数来说范围偏宽。#![allow(dead_code)] 让它不报警告（core/lib.rs:3-4），所以编译器视角看不出问题；但语义上"模块对外暴露的 API"被悄悄多了一项。

**严重度：Minor**（无功能影响；测试用 helper 不收窄可见性在 memory_db.rs 现有惯用法中也有先例，例如 unique_test_memory_db_path 本身已正确 #[cfg(test)]，但 MemoryDbPathGuard 等仍 pub(crate)）。建议处理路径：

- (a) 把 count_self_cognition 改为 #[cfg(test)] pub(crate) fn ...，与 unique_test_memory_db_path 同档（mod.rs:21-22 已有 #[cfg(test)] re-export 的样板）；
- (b) 或保留 pub(crate) 但在 doc comment 已有的 "Diagnostic only" 基础上再注明 "Test-only utility; no production caller as of T3a"，下个用到的生产 caller 来时再改。

两者均不阻塞。本轮仅作记录，留终审 triage。

---

## 新发现

本轮未发现 Critical / Important 级新问题。仅上述 count_self_cognition 可见性 Minor 与 BOM 边界 Minor 两条观察，均已在前文展开。

---

## 一句话总结

SPEC / QUALITY 双通过：**I-1 / I-2 / I-4 闭环**，M-1 主流场景闭环（多 BOM / BOM 前空白 edge case 未覆盖记 Minor），无回退；新方案以"确定性主键 + INSERT OR IGNORE"既解决 race 也顺带消解 onboarding 静默丢失，比原 brief 中"迁移标记表 + BEGIN IMMEDIATE"方案更省一层抽象。