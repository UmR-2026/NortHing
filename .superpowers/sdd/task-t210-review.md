# Task T2-10 Review — 连续性自检测试（seed-restore-diff 形态）

- **Worktree**: `E:\agent-project\.worktrees\northing-t210`
- **Branch**: `feat/t210-continuity-0821`
- **Commit**: `74e25d9` (single commit on `c3fb72e`)
- **Diff**: `.superpowers/sdd/review-package-t210.diff`
- **角色**: 独立验收者（被期望找茬）

## 双判决摘要

| 判决 | 结果 |
|---|---|
| SPEC | **PASS** — 三条 spec 全部满足 |
| QUALITY | **PASS**（带 1 项 Minor） |

---

## 1. SPEC 逐条判决

### Spec 1：identity 测试隔离缝 — **PASS**

**实现位置**：`src/crates/assembly/core/src/agentic/identity.rs:14-23, 31-70`

**逐行复核**：
- L15 `#[cfg(test)]` 守卫 `if let Some(path) = test_identity_path_override() { return path; }` —— 在生产构建中此 if 块整段消失，`identity_path()` 退回原 `dirs::config_dir()...` 行为。✅
- L31-34 `thread_local! { static TEST_IDENTITY_PATH: RefCell<Option<PathBuf>> = ... }` 全部 `#[cfg(test)]` —— 生产代码 0 字节开销。✅
- L36-39 `test_identity_path_override()` 函数体及 thread-local 读取均 `#[cfg(test)]`。✅
- L43-53 `IdentityPathGuard` struct 与 `with_test_identity_path` 构造函数：照抄 `MemoryDbPathGuard` 模式（`prev`/`path` 双 Option + RAII），`pub(crate)` 仅 crate 内可见，**全部 `#[cfg(test)]`**。✅
- L55-59 `unique_test_identity_path()` 复用 `std::env::temp_dir().join(format!("northhing-test-identity-{}.md", uuid::Uuid::new_v4()))`，与 `MemoryDb::unique_test_memory_db_path` 命名/格式一致。✅
- L62-70 `Drop` 守卫：先恢复前一个 thread-local 值，再尽力清理临时文件（`.ok()` 吞错误，与 `MemoryDbPathGuard` 对齐）。✅

**生产路径零行为变化复核**：除去 L86-89 的纯风格化变更（详见 Minor-1），`identity_path()` / `identity_exists()` / `load_identity()` / `save_identity()` / `build_identity_prompt()` 的生产代码路径与本 commit 前完全一致。

### Spec 2：连续性自检测试 — **PASS**

**实现位置**：`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/continuity_selfcheck.rs:1-321`
**挂载点**：`session_manager_lifecycle_tests/mod.rs:15-16` 已新增 `mod continuity_selfcheck;`

**天花板注释复核**（L1-8）：
- L2-8 文件头注释显式声明"kill"范围 = SessionManager / PersistenceManager / MemoryDb / Identity 层 drop + 重建；进程级杀 + 守护进程恢复归 T5。✅ 与 brief §2 第 2 条完全一致。

**流程复核**（对照 brief §2 step 1-8）：
- Step 1（隔离环境，L96-100）：`TestWorkspace::new()` + `with_test_memory_db_path(unique_test_memory_db_path())` + `with_test_identity_path(unique_test_identity_path())` + `ensure_global_config_for_tests().await` —— 复用 `restore_dialog` 先例的 TestWorkspace + `MemoryDbPathGuard` 范式，并叠加新增的 `IdentityPathGuard`。✅
- Step 2（建 manager + 固定 session id，L102-117）：`create_session_with_id(Some("session-continuity-seed-001"), ...)` 调用链命中 `Session::new_with_id`（session.rs:163），session_id 为确定性字符串非 UUID。✅
- Step 3（seed 2 turn + 显式 turn_id，L119-184）：
  - turn_1 (`turn_index=0`) 与 turn_2 (`turn_index=1`) 显式 turn_id = "turn-001" / "turn-002"。✅
  - 每 turn 设置 `kind = UserDialog`, `status = Completed`，model_rounds 推一个 `create_model_round_with_text` 产物。
  - `save_dialog_turn` 持久化两个 turn 文件（路径为 `turn-0.json` / `turn-1.json`，由 `turn_path(workspace, session_id, turn_index)` 决定）。
  - 更新 `updated_session.dialog_turn_ids = vec![turn_1_id, turn_2_id]` 并 `save_session`，确保 session 元数据反映两 turn。✅
  - `save_turn_context_snapshot(..., 1, &seeded_messages)` 写入 `context-1.json`（最新快照索引 = 1）。✅
  - `replace_context_messages(&session_id, seeded_messages.clone())` 同步 in-memory cache（也会触发 best-effort 快照持久化覆盖 context-1.json，幂等）。✅
- Step 4（插 2 条 fixed facts，L186-218）：
  - `fact-continuity-001`：scope=Workspace, fact_type=Feedback, provenance=(session_id, turn_1_id) —— 通过 `MemoryDb::open(&default_memory_db_path())` 在测试隔离路径上插。✅
  - `fact-continuity-002`：scope=Global, fact_type=Project, provenance=(session_id, turn_2_id)，无 workspace_key（global scope）。✅
- Step 5（identity 固定文本，L220-222）：`save_identity("I am northhing assistant, focused on reliable orchestration.")` 写入受 `IdentityPathGuard` 重定向的临时路径。✅
- Step 6（drop 一切，L224-227）：`drop(manager) / drop(persistence_manager) / drop(memory_db)` 顺序 drop，强引用清零。✅
- Step 7（同路径重建 + restore，L229-235）：`PersistenceManager::new(workspace.path_manager())` 与 `test_manager` 重建，再 `restore_session_with_turns(workspace.path(), &session_id)`。✅
- Step 8（三组等价断言，L237-316）：
  - **Session (8a, L240-285)**：
    - `restored_turns.len() == 2` ✅
    - `restored_session.dialog_turn_ids == [turn_1_id, turn_2_id]` ✅
    - `restored_turns[0].turn_id == turn_1_id` / `[1].turn_id == turn_2_id` ✅
    - `matches!(restored_session.state, SessionState::Idle)` ✅
    - `restored_messages.len() == 4` ✅
    - 四条消息逐一断言 `extract_role_and_text`（role + text）+ `metadata.turn_id` ✅
  - **Memory (8b, L287-312)**：
    - `restored_facts.len() == 2` ✅
    - 两条 fact 各断言 `text / scope / confidence / provenance.session_id / provenance.turn_id / fact_type` —— 与 brief §预检（count+text+scope+confidence+session_id+turn_id，排除 *_at）一致，并额外覆盖 `fact_type`（不冲突）。✅
  - **Identity (8c, L314-316)**：
    - `load_identity().ok_or("restored identity should exist")` 全字相等。✅

**断言强度脑内反例演练**：
- 若 `save_dialog_turn` 漏写 turn-1.json：`restored_turns.len() == 2` 即红。✅ 抓得住
- 若 `save_turn_context_snapshot` 改路径写错：`load_latest_turn_context_snapshot` 找不到快照，回退 `build_messages_from_turns`（yield 0 条由 turns 派生 messages，与 4 条断言不一致）。✅ 抓得住
- 若 `insert_fact` 漏写 scope 列：`f1.scope == fact_1.scope` 红。✅ 抓得住
- 若 `save_identity` 不写文件：`load_identity().ok_or` 返回 Err（`restored identity should exist`）。✅ 抓得住
- 若 session state 持久化丢失：`state == Idle` 红。✅ 抓得住

断言都钉真行为，不是钉错行为（ling 事故防御）。✅

### Spec 3：不顺手碰 fake AI backend / turn_persist / identity 生产语义 — **PASS**

- **fake AI backend**：未触碰（编排者裁定已授权绕过，本 commit 仅做 seed-restore-diff 形态）。✅
- **turn_persist**：diff 中无 `turn_persist` 相关文件改动。✅
- **identity 生产语义**：除 `clear_identity` L88 一处纯风格化变更（见 Minor-1）外，生产路径无任何行为变化。✅

---

## 2. QUALITY 判决

### 复用核查（mandatory）

| 已有模式/先例 | 本任务复用方式 | 验证 |
|---|---|---|
| `MemoryDbPathGuard` / `with_test_memory_db_path` / `unique_test_memory_db_path` (memory_db.rs:822-873) | 直接 use，新 IdentityPathGuard 照抄模式 | ✅ |
| `restore_dialog` 先例 (session_manager_lifecycle_tests_restore_dialog.rs:19-137) TestWorkspace + PersistenceManager::new + test_manager + save_dialog_turn + save_turn_context_snapshot | 直接 use | ✅ |
| `ensure_global_config_for_tests` 模式（先例 subagent_ports/mod.rs:147） | 仿写（OnceLock + initialize + eprintln on error） | ✅ |
| `session_manager_tests.rs` 的 TestWorkspace + test_manager + 继承 helpers | 直接 `use super::super::{test_manager, TestWorkspace}` | ✅ |

无重复造轮子。✅

### 无 owner 抽象 / 预算闸

- 未引入新 owner / trait / interface；新代码仅是 RAII guard + 测试 fn。✅
- 无任何 budget 闸被绕过（无预算文件改动）。✅

### 顺手改动核查

- diff 严格 6 个文件：brief / report / identity.rs / continuity_selfcheck.rs / mod.rs / agent_memory/mod.rs。✅
- 无 format / lint 顺手清理（formatter 报告 "No changed Rust files"，因 commit 已包含 rustfmt 结果）。✅
- 无 commit-message 含 housekeeping 标记的额外改动。✅

### agent_memory/mod.rs re-export 必要性判决

`FactConfidence` / `FactProvenance` / `FactScope` 是构造测试 Fact 实例必需的类型：

- `mod facts;`（mod.rs:4）是私有模块，未对外暴露。
- 若无 re-export，测试需 `use crate::service::agent_memory::facts::{...}` —— 必须先提升 `mod facts` 至 `pub(crate) mod facts`，会扩大 facts 模块可见面。
- re-export `pub(crate)` 仅对同 crate 可见（测试同在 assembly/core），是窄门。

**结论：re-export 必要、非顺手放宽**。✅

### 测试断言与实现一致性

- `restored_turns[i].turn_id == turn_X_id`：实现侧 `list_indexed_turn_paths` 按 turn_index 升序排序（services-core/src/session/layout.rs:135），seed 时 turn_1=index 0、turn_2=index 1，故 restored_turns[0]=turn_1, [1]=turn_2。✅
- `restored_messages.len() == 4`：snapshot turn_index=1（因 dialog_turn_ids.len()=2 → persist 用 len-1=1）；restore 时 `load_latest_turn_context_snapshot` 取 max(context-N.json) = 1，delta_start = 2 == persisted_turns.len()，不追加派生 messages。messages 来自 snapshot 的 4 条。✅
- `restored_session.state == SessionState::Idle`：seed 时 `Session::new_with_id` 默认 Idle；`save_session` 内 `sanitize_runtime_state` 仅 Processing → Idle，Idle 保持；restore 时 `previous_state_was_not_idle = false` 不二次重置。✅
- `restored_facts[i].id == "fact-continuity-XXX"`：`get_facts(Some(&workspace_key))` 返回 workspace+global rows 各一条，测试用 `iter().find(|f| f.id == ...)` 跨位置取值，规避 `ORDER BY created_at` 顺序依赖。✅
- `extract_role_and_text`：seed 用 `Message::user(...)` / `Message::assistant(...)`，均生成 `MessageContent::Text(...)`；helper 第一个分支命中。✅

所有断言钉真行为。✅

### 文件大小（god-file 防御）

- `continuity_selfcheck.rs` 321 行（< 800 ✅）
- `identity.rs` 121 行（< 800 ✅）
- 无文件触发 god-file 警告。

---

## 3. 独立验证（实跑）

### 3.1 新测试 3 连跑确定性

```
=== Run 1 === test agentic::session::session_manager_tests::session_manager_lifecycle_tests::continuity_selfcheck::continuity_selfcheck_seed_restore_diff ... ok  finished in 0.17s
=== Run 2 === ... ok  finished in 0.14s
=== Run 3 === ... ok  finished in 0.16s
```

3/3 全过，时长 0.14-0.17s 区间内（与报告 0.17/0.15/0.13 微差，属机器抖动）。✅

### 3.2 session 就近回归

```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib session
→ test result: ok. 153 passed; 0 failed; 1 ignored; 0 measured; 897 filtered out; finished in 0.41s
```

与报告 153 passed 完全一致。✅

### 3.3 编译 / 家规 6

```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
→ Finished `dev` profile [...] target(s) in 1.93s  ✅

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
→ Finished `dev` profile [...] target(s) in 1.73s  ✅
```

### 3.4 边界 / Rot 预算 / fmt

```
node scripts/check-core-boundaries.mjs
→ Core boundary check passed.  ✅

pnpm run check:rot
→ Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).  ✅

pnpm run fmt:rs
→ [format-changed-rust] No changed Rust files found in workspace or index.  ✅
```

(fmt:rs "No changed files" 含义：working tree 干净、commit 已含 rustfmt 结果。这与 report 提到的 "Formatting 4 Rust file(s)" 时间点是 commit 前的本地工作树，commit 后再跑 fmt:rs 自然无文件可改。)

---

## 4. findings 汇总

### Critical
无。

### Important
无。

### Minor

**Minor-1**：`clear_identity()` 生产代码风格化变更（identity.rs:88）

- 旧：`let _ = std::fs::remove_file(path);`
- 新：`std::fs::remove_file(path).ok();`

两种写法语义完全等价（都丢弃 `Result`）。**严格按 brief §Global Constraints "identity override 必须是 #[cfg(test)]、生产代码路径零变化"，行为零变化（不是 byte-zero），但形式上对生产代码做了一处编辑**。

不阻塞放行。fixer 无需修复；若追求最严，可还原为 `let _ = ...`。

### Plan-mandated findings
无。

---

## 5. 结论

**APPROVED**

理由：
1. 三条 spec 全部满足，断言强度足以钉住实现真行为。
2. 复用 `MemoryDbPathGuard` / `restore_dialog` / `ensure_global_config_for_tests` 先例，未造轮子。
3. `IdentityPathGuard` 严守 `#[cfg(test)]` 门控，生产 `identity_path()` 路径零变化（仅一处无意义风格化改动，详见 Minor-1）。
4. agent_memory re-export 是必要的窄门，非顺手放宽。
5. 3 连跑确定性实测通过（0.14-0.17s 全绿），session regression 153 passed，workspace / desktop / boundary / rot / fmt 全过。

唯一 Minor 不阻塞。Continuity 自检测试落地完整，可作为 T5 进程级恢复的前置门。