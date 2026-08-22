# Task PHASE-1B Report — facts jsonl 收口 SQLite

## Spec 落实情况

- **Spec 1（迁移抽取）**：在 `facts.rs` 中实现了 `migrate_facts_jsonl_once(db, memory_dir, ws_key)`，并在 `mod.rs` 导出。迁移时通过 `judge_mom` 表持久化记录 `facts_jsonl_migrated_v1:<ws_key>` 标记，读取 `facts.jsonl`，自动跳过损坏行（`warn!` 日志），按 text 级别去重后批量插入 SQLite（`db.insert_fact`），保留 `INSERT OR IGNORE` 双重保障。在 `turn_persist.rs` 的 `append_facts_entry` finalize 钩子中懒触发。
- **Spec 2（删 jsonl 写路径）**：从 `facts.rs` 删除了 `append_facts` 与 `append_facts_dedup` 及其测试用例；在 `mod.rs` 中删除了对应导出；在 `turn_persist.rs:594-604` 中彻底删除了写 jsonl 的调用。
- **Spec 3（留读兼容）**：保留了 `facts.rs` 中的 `read_facts` 以及 `auto_memory.rs` 中的两处 fallback 读取点，均添加了 `// compat: facts.jsonl read fallback, remove after one release cycle` 注释；未重命名或删除存量 `facts.jsonl` 文件。
- **Spec 4（测试）**：
  - 在 `facts.rs` 中增加了 `migrate_facts_jsonl_once_idempotency_and_marker`（验证迁移计数、ID保留、坏行跳过、文本去重、持久标记防重灌、模拟重启后不重灌）和 `migrate_facts_jsonl_once_missing_file_sets_marker` 测试。
  - 将 `auto_memory.rs` 中的种子测试改为通过 `tokio::fs::write` 直写 `facts.jsonl`，保留 fallback 降级测试。
- **Spec 5（边界保持）**：未触碰 `agentic/episodes/store.rs`、distill/dream/评审记账钩子及 growth 线文件。

## 复用侦察

1. **`turn_persist.rs:425-607` (`append_facts_entry`)**：原先使用 OnceLock 内存守卫进行内联迁移并在文末通过 `append_facts_dedup` 双写 jsonl。重构后直接在已有 `db` 连接上调用 `migrate_facts_jsonl_once`，移除 jsonl 追加，保留候选事实 SQLite 入库、权重衰减与 dream sweep。
2. **`memory_db.rs` 的 `judge_mom` 表**：表结构为 `(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL)`，直接复用已有的 `db.get_judge_mom_value(key)` 与 `db.set_judge_mom_value(key, value, at_ms)` 方法，未新建表或修改数据库 schema。
3. **`with_test_memory_db_path` 隔离缝 (`memory_db.rs:838`)**：在新增迁移测试中完整使用 thread-local `with_test_memory_db_path(unique_test_memory_db_path())` 隔离临时 SQLite 文件，保证测试密闭性。
4. **`auto_memory.rs:240-280` fallback 段**：保留了 DB 为空或打不开时的 `read_facts(&memory_dir).await` 降级逻辑，并已加明确的 release 兼容标记。

## ws_key 同源比对证据（最高危点核实）

- **读侧**（`auto_memory.rs:245` 及 `system_prompt.rs:76`）：
  ```rust
  // system_prompt.rs:
  let workspace = Path::new(&self.context.workspace_path);
  build_workspace_agent_memory_prompt(workspace).await;

  // auto_memory.rs:
  let workspace_key = workspace_root.to_string_lossy().to_string();
  db.get_facts(Some(&workspace_key));
  ```
- **写侧与迁移侧**（`turn_persist.rs:429,548` 及 `facts.rs:67`）：
  ```rust
  // turn_persist.rs:
  migrate_facts_jsonl_once(db, &memory_dir, workspace_path).await;
  db.insert_fact(fact, Some(workspace_path));

  // facts.rs:
  let marker_key = format!("facts_jsonl_migrated_v1:{}", ws_key);
  db.insert_fact(&fact, Some(ws_key));
  ```
- **比对结论**：
  两端均直接使用会话上下文中的 workspace 原始路径字符串（`self.context.workspace_path`），`Path::new(&path).to_string_lossy()` 在标准路径下与 `workspace_path` 逐字节完全同源，无大小写/斜杠格式转换，保证迁移写入的 `workspace_key` 与读侧检索完全一致。

## growth 冲突预警声明

`feat/growth-core-0804` 未合并分支同样修改 `append_facts_entry`。本次重构严格局限于迁移逻辑抽取与 jsonl 写路径移除，未改动函数签名、上下文蒸馏、评审记账及 dream 流程，将后续合并冲突面降至最低。

## 健康度与行数（God-file 观测数据）

- `facts.rs`：905 行 → **744 行**（净减 161 行，成功跌破 800 行软线，退出 god-file 观察名单）。
- `turn_persist.rs`：683 行 → **636 行**（净减 47 行）。

## Rust 错误与修复分层

- `E0583 (module not found for generated_locale_contract)`：机制层，执行 `pnpm run i18n:generate` 生成 contract 模块。
- `Rot budget let_underscore exceeding ceiling`：机制层，将迁移代码中的 `let _ = db.set_judge_mom_value(...)` 改为显式 `if let Err(e) = ...` 处理并打日志，未放宽 ceiling。

## 验证命令与输出证据

### 1. `cargo test -p northhing-core --features product-full --lib agent_memory`
```text
running 67 tests
test service::agent_memory::facts::tests::migrate_facts_jsonl_once_missing_file_sets_marker ... ok
test service::agent_memory::facts::tests::migrate_facts_jsonl_once_idempotency_and_marker ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
...
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 979 filtered out; finished in 0.23s
```

### 2. `cargo test -p northhing-core --features product-full --lib turn`
```text
running 118 tests
...
test result: ok. 118 passed; 0 failed; 0 ignored; 0 measured; 928 filtered out; finished in 1.54s
```

### 3. `cargo check --workspace`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) - 0 errors
```

### 4. `cargo check -p northhing`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 05s - 0 errors (House Rule 6 satisfied)
```

### 5. `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```

### 6. `pnpm run check:rot`
```text
✔ actual workspace rot budget passes with current manifest (390.5486ms)
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

### 7. `pnpm run fmt:rs`
```text
[format-changed-rust] Formatting 4 Rust file(s).
```

## 偏离声明

无偏离。
