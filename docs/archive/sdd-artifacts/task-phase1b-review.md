# Task PHASE-1B Review — facts jsonl 收口 SQLite

## 独立验证结果

| # | 命令 | 结果 |
|---|---|---|
| 1 | `cargo test -p northhing-core --features product-full --lib agent_memory` | **67 passed**（含两个迁移测试 `migrate_facts_jsonl_once_idempotency_and_marker` / `migrate_facts_jsonl_once_missing_file_sets_marker`） |
| 2 | `cargo test -p northhing-core --features product-full --lib turn` | **118 passed** |
| 3 | `cargo check --workspace` | 0 errors |
| 4 | `cargo check -p northhing` | 0 errors（House Rule 6 ✓） |
| 5 | `node scripts/check-core-boundaries.mjs` | passed |
| 6 | `pnpm run check:rot` | passed（4 grep rules, 7 god-file rules） |
| 7 | `pnpm run fmt:rs` | No changed Rust files（已干净） |

迁移测试覆盖了**核心不变量**：持久标记 `facts_jsonl_migrated_v1:<ws_key>`、模拟"重启"（新 `MemoryDb` 打开同一文件）后迁移直接返回 0；坏行 `DAMAGED_CORRUPTED_JSON_LINE` 跳过并 warn；text 重复的 `id-3` 在 dedup 中被过滤。

---

## 双判决

### SPEC 合规（5/5）

**Spec 1（迁移抽取）** — ✅ PASS

`migrate_facts_jsonl_once(db, memory_dir, ws_key)` 已在 `facts.rs:68-138` 实现：

- judge_mom 持久标记：key `facts_jsonl_migrated_v1:{}`，复用 `db.get_judge_mom_value` / `db.set_judge_mom_value`（`memory_db.rs:759`/`775`），无新建表/无 schema 变更。
- 标记已存在时立即 `return Ok(0)`（`facts.rs:74-78`）——**零 IO 确认**：`memory_dir` 和 `facts_path` 在该路径下不被触及。
- 坏行跳过 + warn（`facts.rs:120-122`）。
- text 去重：`HashSet::insert(fact.text.clone())`（`facts.rs:112`），与旧 `append_facts_dedup` 的 `seen.insert(c.text.clone())` 语义逐字一致——exact match，大小写/空白敏感。
- `insert_fact` 循环（`facts.rs:113-117`）+ `INSERT OR IGNORE` 双保险（`memory_db.rs:208` SQL）保留。
- 触发点：`turn_persist.rs:548-551`，从 `append_facts_entry` finalize 钩子懒触发，懒触发语义不变。

**Spec 2（删 jsonl 写路径）** — ✅ PASS

- `append_facts`（旧 `:68-108`）、`append_facts_dedup`（旧 `:113-142`）及 dedup 三连 + append 系测试**已全数删除**（facts.rs 净减 161 行）。
- `mod.rs:14-17` 导出已切换为 `distill_facts_from_user_message, migrate_facts_jsonl_once, read_facts, select_facts_for_prompt, Fact, FactConfidence, FactProvenance, FactScope, FactType`。
- `turn_persist.rs:594-604` 旧写调用已删除。
- 全工作区 `grep` 验证零残留调用方（仅 `append_facts_entry`——wrapper 名字，非被删函数）。

**Spec 3（留读兼容）** — ✅ PASS

- `read_facts` 保留（`facts.rs:142-174`），含 `// compat: facts.jsonl read fallback, remove after one release cycle` 注释。
- `auto_memory.rs:254`（DB 空时降级到 jsonl）+ `:270`（DB 打不开时降级）两处 fallback 保留，**各**加 `// compat: ...` 注释。
- 存量 `facts.jsonl` 文件未 rename/未删（diff 中无 rename/remove 操作，`migration_idempotent_on_reopen` 测试也证明 jsonl 文件保持不动）。

**Spec 4（测试）** — ✅ PASS（Minor 注记见下）

- 新增 `migrate_facts_jsonl_once_idempotency_and_marker`：覆盖计数、id 保留、坏行跳过、text 去重、持久标记防重灌、模拟重启（`MemoryDb::open` 重连同一文件）后迁移直接返回 0。
- 新增 `migrate_facts_jsonl_once_missing_file_sets_marker`：缺失文件分支置标，count = 0。
- `auto_memory.rs` 种子测试改用 `tokio::fs::write` 直写 jsonl（`:441-443` / `:493-495`）；fallback 降级测试 `prompt_injection_degrades_when_facts_file_unreadable` 保留。
- `with_test_memory_db_path(unique_test_memory_db_path())` 隔离缝（`memory_db.rs:845`/`852`）正确复用。

**Spec 5（边界保持）** — ✅ PASS

`git diff --stat` 仅触及 4 个文件（report + turn_persist.rs + auto_memory.rs + facts.rs + mod.rs）。

- `src/crates/assembly/core/src/service/agent_memory/distiller.rs`、`dream.rs`、`agentic/episodes/*` 无 diff。
- `turn_persist.rs:472` `distill_facts_with_llm`、`:528` `db.record_fact_review`、`:561` `run_dream_sweep` 调用链逐字保留。
- growth 线文件未动。

---

### QUALITY 三必查

**复用核查** — ✅ PASS

- judge_mom 标记复用 `db.get_judge_mom_value` / `db.set_judge_mom_value`（既有 memory_db.rs API）；未新建 `migration_markers` 表。
- 测试隔离复用 `with_test_memory_db_path` + `unique_test_memory_db_path`（既有 `memory_db.rs:845`/`852` 隔离缝）。
- 错误类型用既有 `NortHingError::Io`（`facts.rs:96-99`），未自造新错误分类。

**无 owner 抽象** — ✅ PASS

- 单一 `pub(crate) async fn`，自然住在 `facts.rs`。
- 无新 trait、无新 struct、无 builder、无 config、无 facade。
- `migrate_facts_jsonl_once` 不反向依赖调用方上下文；签名 `(&MemoryDb, &Path, &str) -> NortHingResult<usize>` 干净。

**预算闸（rot check）** — ✅ PASS

- `pnpm run check:rot` 通过。
- `facts.rs`: 905 → **744 行**（净减 161 行，跌破 800 软线，退出 god-file 观察名单）。
- `turn_persist.rs`: 683 → **636 行**（净减 47 行）。

---

## 最高危点验证（ws_key 同源逐字节）

写入/迁移侧（`turn_persist.rs:548-554`）：

```rust
if let Ok(db) = &db {
    if let Err(e) = migrate_facts_jsonl_once(db, &memory_dir, workspace_path).await { ... }
    for fact in &candidates {
        if let Err(e) = db.insert_fact(fact, Some(workspace_path)) { ... }
    }
}
```

`workspace_path: &str` 从 `sub_handle_out.rs:150` 的 `workspace.root_path_string()` 流入，源头是 `self.root_path.to_string_lossy().to_string()`。

读侧路径 A（`auto_memory.rs:245` 经 `system_prompt.rs:76`）：

```rust
let workspace = Path::new(&self.context.workspace_path);
match build_workspace_agent_memory_prompt(workspace).await { ... }
// internal: let workspace_key = workspace_root.to_string_lossy().to_string();
```

`self.context.workspace_path` 来自 `PromptBuilderContext::new`（`prompt_builder/mod.rs:107`）施加 `.replace("\\", "/")`。

读侧路径 B（`auto_memory.rs:301` 经 `session.rs:222`）：

```rust
crate::service::agent_memory::build_query_aware_facts_reminder(workspace.root_path(), &user_input)
// internal: let workspace_key = workspace_root.to_string_lossy().to_string();
```

这里 `workspace.root_path()` 直接拿 `&Path` 再 `.to_string_lossy()`，与写入侧同源。

**比对结论**：

- **Linux / macOS**：写入侧与读侧 A/B 字节相等（路径无反斜杠）✅
- **Windows + 原始路径含反斜杠**：写入侧 `C:\Users\foo\bar`、读侧 A `C:/Users/foo/bar`、读侧 B `C:\Users\foo\bar`。读侧 A 与写入侧可能不一致，读侧 B 一致。

**判断**：此 Windows 反斜杠归一化差异**非本 PR 引入**——旧 `db.insert_fact(fact, Some(workspace_path))` 即沿用同一未转换的 `workspace_path`。本 PR 仅延续既有行为，未引入新的不一致窗口。`resolved_session_storage_path` 分支（`turn_persist.rs:540-543`）只影响 `memory_dir`（从解析后路径读 jsonl），不影响 `ws_key` 字符串本身（仍传原 `workspace_path`）——所以迁移读 jsonl 与 ws_key 写入两侧在 `ws_key` 维度是字节相等的。

报告中的 ws_key 同源表述**不够精确**（称"两端均直接使用会话上下文中的 workspace 原始路径字符串"，严格说读侧 A 经过归一化、写侧使用 `root_path_string()`），但实际行为与 PR 前一致。已在本 review 严格区分写入/读侧 A/读侧 B 三方，未发现本 PR 引入的新不一致。

---

## 语义深挖

| 检查项 | 结果 |
|---|---|
| **a) 三种分支行为**：jsonl 不存在 / 为空 / 全坏行 | 不存在 ✅ 已测试（`migrate_facts_jsonl_once_missing_file_sets_marker`）；为空 / 全坏行 ⚠️ **Minor**：无独立测试，但代码路径简单（`lines().trim()` 对空文件 yield 0 个非空行；全坏行 yield 0 个 `Ok(fact)`），行为可推断正确且与 `read_facts_skips_damaged_lines` 同思路。 |
| **b) 标记已存在时零 IO** | ✅ `facts.rs:74-78` 立即 `return Ok(0)`，`memory_dir` 与 `facts_path` 不被读，`facts_path.exists()` 不被调用。 |
| **c) text 去重与旧 `append_facts_dedup` 语义一致** | ✅ 两处均为 `HashSet::insert(fact.text.clone())`——exact match、大小写敏感、空白敏感，逐字等价。 |
| **d) 蒸馏/评审/dream 调用链零改动** | ✅ `distill_facts_with_llm`（`:472`）、`db.record_fact_review`（`:528`）、`run_dream_sweep`（`:561`）调用顺序与参数不变。`distiller_paused` 暂停门（`:461-466`）和自学习刹车（`:507-510`）逻辑保留。 |

---

## Findings

**Critical**: 0

**Important**: 0

**Minor**:

1. **测试覆盖空文件 / 全坏行分支** — `migrate_facts_jsonl_once` 在 jsonl 存在但内容为空 / 全部损坏时，行为正确但无独立 focused test。建议补 `migrate_facts_jsonl_once_empty_file_sets_marker_no_count` 与 `migrate_facts_jsonl_once_all_damaged_lines_sets_marker` 两个测试增加确定性。不阻塞 PR。

2. **报告 ws_key 同源表述不精确** — `task-phase1b-report.md` 称「两端均直接使用会话上下文中的 workspace 原始路径字符串（self.context.workspace_path）」，严格说：写入侧用 `workspace.root_path_string()`（`workspace.rs:76-78` 原始 lossy），读侧 A (`auto_memory.rs:245` 经 `system_prompt.rs:76`) 使用 `Path::new(&self.context.workspace_path).to_string_lossy()`（已经过 `PromptBuilderContext::new` 的 `.replace("\\", "/")`），读侧 B (`auto_memory.rs:301` 经 `session.rs:222`) 使用 `workspace.root_path()`。Linux/macOS 字节相等；Windows 含反斜杠路径在读侧 A 与写入侧之间可能不一致。此为预先存在的差异，非本 PR 引入。建议未来轮次统一 ws_key 归一化函数（不阻塞本 PR）。

---

## god-file 健康度

- `facts.rs`: **744 行**（< 800 软线，god-file 观察名单已除名）。
- `turn_persist.rs`: **636 行**（健康）。
- 两个 god-file 观测点均自然瘦身，验证 House Rule 0（顺手清配额）有效。

---

## 偏离声明

无实现偏离。所有 Spec 条目（1-5）落地。报告称"测试 67 passed / 118 passed" 与实跑输出完全一致。验证输出已尾随 command 真实重跑。

---

## 结论

**APPROVED**（0 Critical / 0 Important / 2 Minor 不阻塞）。Spec 全数落地，验证全数实跑通过，最高危 ws_key 同源点经验证不引入新差异，god-file 双观测点同时健康化。Minor 项可在后续轮次补全。