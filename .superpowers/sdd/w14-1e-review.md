# W14-1e Review — 修复测试污染开发机真实记忆库

- **判决**：**APPROVE**
- **C/I/M 计数**：**0 / 0 / 1**
- **BASE**：`8c00962` → **HEAD**：`eee1552`（恰好 1 commit，与 brief Spec 一致）
- **改动范围**：`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`，`+6 / -1`（`git show --stat` 已核），与 report 声明完全一致；未触碰任何禁区文件（`memory_db.rs` / `dream.rs` / `turn_persist.rs` / `auto_memory.rs:246/302` / `facts.rs` / `continuity_selfcheck.rs` / `kernel_facade/memory.rs`），由 `git diff 8c00962..eee1552 --name-only` 单文件清单佐证。

---

## SPEC 双判决

| Spec | 判定 | 证据 |
|---|---|---|
| 1. 给 `:573`（现 `:579`）测试补守卫作为函数体第一行 | **PASS** | `auto_memory.rs:580` 紧接 `async fn ... {` 后即为 `let _db_guard = with_test_memory_db_path(unique_test_memory_db_path());`；同 `:587` `let db_path = default_memory_db_path();` 仍按 brief 要求保留原样（守卫生效后该行已被重定向）。 |
| 2. 全仓核对 5 处（facts.rs×2、continuity_selfcheck.rs×2、kernel_facade/memory.rs×2） | **PASS** | 抽查：`facts.rs:660–664` `:722–727` import 守卫并在函数首行 `let _guard = ...` 持有，`:668/:704/:731` 的 `default_memory_db_path()` 在持有期内 ✓；`continuity_selfcheck.rs:98` `let memory_guard = with_test_memory_db_path(unique_test_memory_db_path());`，`:318` 显式 `drop(memory_guard)`，`:187/:288` 调用在持有期内 ✓；`kernel_facade/memory.rs:62/101` 是 `KernelFacade::list_facts/search_facts` trait 方法体（非测试）✓。 |
| 3. 不许动生产路径（`:246`、`:302`、`dream.rs:38`、`turn_persist.rs:457`） | **PASS** | `git diff --name-only` 仅 1 文件；`:246/302` 在 commit 前/后字节不变（仅 `:529–608` 区域被修改，落在 `query_aware_tests` 子模块内）。 |
| 4. 不许改 `default_memory_db_path` / `with_test_memory_db_path` / `unique_test_memory_db_path` 实现 | **PASS** | `memory_db.rs` 不在 `git diff --name-only` 清单。 |
| 5. 新发现未受保护测试点一并补 | **PASS** | 新发现 `:565`（非空查询 → 触发 `MemoryDb::open` → 原属缺陷）与 `:552`（空查询）两处；report 已点名补法，符合 brief "一并补上并在 report 点名" 要求。 |
| 验证 — 真实库 mtime/size 前后一致 | **PASS** | 独立 `Get-Item`：测试后 `FullName=C:\Users\UmR\AppData\Roaming\northhing\memory\memory.db`、`LastWriteTime=2026/8/29 17:55:58`、`Length=94208`，与 report 一致。 |
| 验证 — `cargo test ... agent_memory` / `memory` | **PASS** | report 粘贴 69/76 passed；与 guard 修复后预期一致。 |
| 验证 — `node scripts/verify-rot-budget.mjs` | **PASS** | report 粘贴通过（5 grep + 3 dir + 6 god-file）。 |

---

## QUALITY 判决

- **最小改动**：仅 1 文件 `+6/-1`，与 brief "无新文件、不动生产、不上调 ceiling" 全对齐 ✓
- **夹带**：无（无格式化顺手改、无注释无关变更）
- **注释/日志**：新增 `use crate::service::agent_memory::{..., unique_test_memory_db_path, with_test_memory_db_path};` 与 3 处 `let _db_guard = ...;` 皆为纯代码、无注释无日志 → 不违反 "logs english no emoji / comments english" 硬规则
- **测试数**：69 → 69（agent_memory）、76 → 76（memory），未下降
- **commit 数**：1 ✓；`git log -1 eee1552` 与 BASE 线性关系明确

---

## 5 条重点核查

### 1. 守卫位置是否正确（每测试函数体第一行）

- `:554` `build_query_aware_facts_reminder_returns_none_for_empty_query` —— 第一行 `let _db_guard = ...`，后续 `tokio::fs::create_dir_all` 才执行 ✓
- `:566` `build_query_aware_facts_reminder_returns_none_when_no_match` —— 第一行守卫生效，后续 `MemoryDb::open` 经重定向 ✓
- `:580` `build_query_aware_facts_reminder_returns_some_with_matching_fact` —— 第一行守卫生效，`:587` `let db_path = default_memory_db_path();` 在持有期内（已重定向到隔离路径）✓

同文件其他测试无遗漏：`auto_memory.rs:430/462/482/505` 4 个 `prompt_injection_*` 测试首行早就有 `_db_guard`（旧 commit 遗留，未被本次改动破坏）；`memory_db_tests.rs` 全套未用 `default_memory_db_path`（grep 验证）；`facts.rs`/`dream.rs` 单元测试也未使用。

### 2. 有没有改不该改的

`git diff 8c00962..eee1552 --stat` 仅 `auto_memory.rs`；禁区的 `memory_db.rs` / `dream.rs` / `turn_persist.rs` / `facts.rs` / `continuity_selfcheck.rs` / `kernel_facade/memory.rs` 均无变更 ✓。改动的 hunk 局限在 `mod query_aware_tests` 子模块（`:529–608`），完全在测试代码区。

### 3. 核心证据可信度判定

**判定：证据充分**。理由：
- mtime/size 严格不变（独立 `Get-Item` 复核一致）：`2026/8/29 17:55:58` / `94208` 字节；
- 真实证据链不是孤立的 mtime/size，而是「测试**确实跑到了**会污染的代码路径 + mtime/size 不变」二者的合取：
  - `build_query_aware_facts_reminder_returns_some_with_matching_fact` 在测试输出日志第 127 行明确 `ok`（见 report §4 命令 1）；该测试体 `:587–590` 显式调用 `MemoryDb::open(&db_path).expect(...)` 与 `db.insert_fact(&fact, ...)` —— 无守卫时这必写真实库；
  - `build_query_aware_facts_reminder_returns_none_when_no_match` 同样在日志第 115 行 `ok`；其非空查询路径会进入 `:302` `MemoryDb::open(&default_memory_db_path())`，即便只读打开 SQLite 也会更新 file header/WAL，**不守卫会观察到 mtime 变化**。
- 因此「mtime 不变」足以反推「守卫生效」（前提是测试确实执行了对应路径，二者皆满足）。
- 轻微 caveat：`mtime/size 完全一致`本身**只能证伪污染**，不能证明"测试真的写到了该路径"；此处靠测试日志 + 代码路径静态阅读补足，合取已构成完整证据。无缺口。

### 4. 守卫是否掩盖真实缺陷

逐条断言核查（每条都看「守卫是否使断言恒真」）：

- `:552`（empty query，`"   "`）→ `build_query_aware_facts_reminder:298` `if query.trim().is_empty() { return Ok(None); }` 在打开 DB 之前就返回。**守卫在此测试里是 no-op**——DB 根本不会被打开，无恒真风险，断言验证的是 fast-path 行为。守卫属「模块风格密闭性」的额外保护，非必要但无害（Minor 记入）。
- `:565`（no match，`"zzzz_no_match_zzzz"`）→ 进入非空路径打开隔离 DB，搜索返回空 → `result.is_none()`。**断言仍验证「非空查询 + 隔离 DB 空 → 返回 None」**，guard 不让该断言恒真（隔离 DB 默认空恰好是 product 语义，符合 brief `select_facts_respects_scope_global_first` 的设计意图）。
- `:579`（matching）→ 打开隔离 DB、插入 fact、查 "pnpm"、断言 `result.is_some()` 与文本包含。**断言验证的是「写入再召回」**，与守卫无关；若插入失败仍会 panic（`expect("insert fact")`），所以 guard 不掩盖"插入失败"等真实缺陷。

**结论：3 个守卫均不掩盖测试本要验证的行为。**

### 5. 全仓核对结论抽查

- `facts.rs:668` 调用 `MemoryDb::open(&default_memory_db_path())` —— 函数体 `:659` 起 `:664` `let _guard = with_test_memory_db_path(unique_test_memory_db_path());` ✓（与 `:727` 同模式）
- `kernel_facade/memory.rs:63` `let db = MemoryDb::open(&default_memory_db_path())` —— 上下文是 `impl MemoryFacade for KernelFacade { async fn list_facts(&self, workspace_slug: Option<&str>) ... }`，trait 方法、生产路径 ✓

两抽查结论与 report §3 一致。

---

## Findings（无 C/I）

### Minor

- **M1**：`auto_memory.rs:552`（empty query 测试）的 `_db_guard` 在该测试语义上是 no-op——`build_query_aware_facts_reminder` 的 `query.trim().is_empty()` fast-path 在打开 DB 前就返回 `Ok(None)`。**不影响正确性、不违反 spec**，且与同模块 `:431/464/483/506` 4 处已有守卫保持风格一致，属合理防御。建议不必修；若要最小化可后续删除该守卫，不阻塞本单。

---

## 结论

- **Spec**：5/5 PASS
- **Quality**：PASS（最小改动、零夹带、测试数不降、commit 数=1）
- **证据**：独立复核 `Get-Item` 与测试日志合取，证据充分
- **守卫掩码风险**：3 处守卫均不掩盖原始断言的验证目标
- **全仓核对抽查**：与 report 一致
