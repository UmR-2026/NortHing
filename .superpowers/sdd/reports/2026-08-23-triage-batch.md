# Triage Batch Report (2026-08-23)

## 修复项结果

### T1. so_handlers.rs:137 未注解 skip 豁免
- **Status**: DONE
- **Evidence**:
  - `git show 7127f9f` 确认了既有 probe-1 豁免注解风格（A2 hidden-subagent, manual compaction, hidden-subagent phase2）。
  - `start_hidden_btw_turn` 为 `/btw` side-question 创建并在后台运行临时子会话（`SessionKind::EphemeralChild`），无法且不应在后台子线程阻塞主会话交互确认。
  - 在 `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_handlers.rs:137` 添加了英文意图注释，行为零改动：
    ```rust
    // Intentional exemption (/btw side question): the side-question turn runs as an
    // ephemeral child session alongside the main conversation without an interactive
    // confirmation prompt attached to the background sub-thread.
    DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi).with_skip_tool_confirmation(true),
    ```

### T2. cli main.rs 800 行临界
- **Status**: DONE
- **Evidence**:
  - 在 `src/apps/cli/src/main.rs:384-394` 的 `initialize_core_services` 中，将冗余局部绑定 `let original = ...;` 合并为直接返回 `ai_config.skip_tool_confirmation`，净减 1 行。
  - 运行 `pnpm run fmt:rs` 格式化后行数为 799 行（<= 799）。
  - `node scripts/verify-rot-budget.mjs` 绿灯（6 god-file rules checked across 1364 files passed）。

### T3. CLI edit 表单明文预填 key
- **Status**: DONE
- **Evidence**:
  - 在 `src/apps/cli/src/ui/startup/selectors.rs:315` 的 `edit_model` 路径中，将表单初始值从 `api_key: model.api_key` 改为 `api_key: String::new()`，编辑时不再向表单预填明文 key，由 F4 的 `resolve_effective_model_key` 保证留空继承 keyring key；add 路径保持不动。
  - `cargo test -p northhing-cli` 36 项测试全部通过（含 keyring_keys 相关测试）。

### T4. sync.rs 注释措辞
- **Status**: DONE
- **Evidence**:
  - `src/apps/desktop/src/app_state/settings/sync.rs:25-28` 中的注释从 "Reads the model-id list from core facade" 修正为 "Reads model configs from core facade (keyless contract shape), resolves each model's key from the OS keyring, and pushes it into core memory via the explicit `api_key` parameter on `upsert_model_config`."，消除了将模型配置结构体误称为 model-id 列表的歧义。

---

## 显式 skip 清单

- **staged-review M3 (push 路径 N 次磁盘写可批量)**: SKIPPED — 仅在启动路径执行单次推送，当前负载下批量写属 YAGNI。
- **staged-review M4 (cache 锁中毒 warn 不传播)**: SKIPPED — 属于 pre-existing 行为模式，与既有 `invalidate_cache` 处理方式一致，保持现状。
- **staged-review M5 (sync_lock 旁路假设)**: SKIPPED — 为未来若引入旁路时的注意事项，当前系统并无旁路。
- **staged-review M6 (push 时序空操作)**: SKIPPED — 当前语义正确且与 desktop 行为一致性优先。
- **p2-review m1 (handoff doc 中英混排)**: SKIPPED — handoff 属于中文工作文档设计，非系统日志范畴。
- **p3a-review M1 (session.rs fmt 触线)**: SKIPPED — 属于已提交代码的 fmt 正常排版，语义完全等价，无 actionable 项。

---

## 验证证据

1. `pnpm run fmt:rs`（幂等）:
   ```text
   > node scripts/format-changed-rust.mjs
   [format-changed-rust] Formatting 6 Rust file(s).
   [format-changed-rust] Restoring 1 collateral Rust file(s) touched through module expansion.
   ```

2. `node scripts/verify-rot-budget.mjs`:
   ```text
   Rot budget verification passed (5 grep rules [unwrap_production=502/502, expect_production=1089/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=109/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=136/400], 6 god-file rules checked across 1364 files).
   ```

3. `cargo check -p northhing-cli`:
   ```text
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 57s
   ```

4. `cargo check -p northhing-core --features product-full`:
   ```text
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.22s
   ```

5. `cargo test -p northhing-cli`:
   ```text
   test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   ```

---

DONE
