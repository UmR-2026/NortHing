# Task ROT-1 Report — T2-9 冗余合并批 1

## 概述

按 `.superpowers/sdd/task-rot1-brief.md` 要求完成 2 个真做项（deep_research re-export、时间 helper 统一收口）与 4 个核销项的证据复核，无行为变化，全工作区通过编译、单测、core boundary 检查及 rot 预算校验。

---

## 复用侦察

1. **`core-types` 时间工具**：查验 `src/crates/contracts/core-types` 确认之前无统一时间 helper。现新建 `src/crates/contracts/core-types/src/time.rs`，挂载至 `lib.rs`。
2. **`ndjson_log` 轮转语义**：
   - `src/crates/assembly/core/src/service/audit_log.rs`：采用 10MB 或 7 天 age-based 多代轮转策略（安全审计日志）。
   - `src/crates/services/debug-log/src/lib.rs`：采用 8MiB 单代 size-based 轮转策略（`.1` 单备份，调试日志）。
   - `facts.rs`/`episodes`/`store.rs`/`config`/`runtime.rs`：纯文件 append，不具备轮转语义。
   - `write_file.rs`：工具层文件写入原子封装，非日志追加。
   结论：各日志面语义及生命周期完全不同，强行合并会破坏领域正交性。
3. **`PersistenceService::FILE_LOCKS` 调用面**：
   - 全仓调用方共 3 处：`workspace/service.rs:230`（`"workspace_data"`）、`workspace/admin.rs:197`（`"workspace_data"`）、`cron/store.rs:86`（`"jobs"`）。
   - `save_json` 内部持有 `FILE_LOCKS` 保护 `create_backup`（多代备份轮转）与静态临时文件 `.json.tmp` 写入及重命名序列。去锁会导致同 key 并发写入时临时文件覆盖与备份破坏。
   结论：文件锁对多步骤备份+原子重命名序列协调必不可少，核销保留。

---

## Spec 实施细节

### 1. `deep_research` 去重与 re-export
- 文件：`src/crates/execution/agent-runtime/src/deep_research.rs`
- 改动：删除 255 行副本实现，替换为 `pub use northhing_runtime_ports::deep_research::*;`。
- 依赖校验：`northhing-agent-runtime` 的 `Cargo.toml` 已有 `northhing-runtime-ports` 依赖，层级方向 `execution -> contracts` 合法。
- 保持接口兼容：`northhing_agent_runtime::deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry}` 路径与 `tests/deep_research_contracts.rs` 保持不变。
- 边界规则同步：`scripts/core-boundaries/rules/source/required-rules.mjs` 与 `scripts/core-boundaries/self-test.mjs` 中的 canonical 路径规则同步指向 canonical 拥有方 `src/crates/contracts/runtime-ports/src/deep_research.rs`。

### 2. 时间 helper 统一收口
- 新增：`src/crates/contracts/core-types/src/time.rs`，实现 `now_unix_ms() -> u64` 与 `now_unix_millis() -> i64`，附带 `SystemTime::now` + `UNIX_EPOCH` 语义及溢出兜底说明与单调性测试。
- 挂载：`src/crates/contracts/core-types/src/lib.rs` 导出 `time` 模块及 `now_unix_ms`, `now_unix_millis`。
- 4 处命名重复收口：
  1. `src/crates/assembly/core/src/agentic/goal_mode/mod.rs:35`：`now_ms() -> u64` 改为 `pub use northhing_core_types::time::now_unix_ms as now_ms;`。
  2. `src/crates/assembly/core/src/agentic/session/evidence_ledger.rs:324`：删除本地 `current_time_millis() -> u64`，调用点直接切换为 `northhing_core_types::time::now_unix_ms()`。
  3. `src/crates/assembly/core/src/service/cron/service_helpers.rs:205`：`now_ms() -> i64` 改为 `pub(super) use northhing_core_types::time::now_unix_millis as now_ms;`。
  4. `src/crates/services/services-core/src/session/metadata_store.rs:387`：删除本地 `current_unix_ms() -> u64`，导入 `northhing_core_types::time::now_unix_ms as current_unix_ms;`。
- 不动项：`agent-runtime` cache_types、`debug-log`、`acp` manager_process 保持现状（无 core-types 依赖，避免跨层引入不必要依赖）；全仓 80 处内联点保持现状。

---

## 核销四项证据（Spec 3）

1. **Spec 3a — ndjson_log 统一**：
   - 证据：`audit_log.rs`（295L）实现基于大小（10MB）与时间（7天）的双条件多代轮转；`debug-log` 实现 8MiB 单代（`.1`）轮转；`facts.rs`/`episodes` 等纯 append 无轮转。重复主体随 T2-2 已清理，现存差异为领域所需语义不同，核销不合。
2. **Spec 3b — FILE_LOCKS 删除**：
   - 证据：`save_json` 序列为 `ensure_parent_dir -> create_backup(backup_count=5) -> write(key.json.tmp) -> rename(key.json.tmp, key.json)`。临时文件路径为固定扩展名 `.json.tmp`，若无 `FILE_LOCKS` 互斥，同 key 并发写入会导致临时文件碰撞及备份文件乱序损坏。核销保留。
3. **Spec 3c — server bootstrap 手抄**：
   - 证据：`src/apps/server/src/main.rs` 全文仅 61 行，只包含基础 Axum 路由（`/health`, `/api/v1/health`, `/api/v1/info`, `/ws`），无任何手抄 bootstrap 逻辑；孤儿文件 `src/apps/server/src/bootstrap.rs` 不在编译单元内。核销无需改动。
4. **Spec 3d — CLI init 样板收口**：
   - 证据：CLI 属于 frozen-experimental 界面，4 处调用点（`src/apps/cli/src/agent/agentic_system.rs:8`, `src/apps/cli/src/management.rs:198`, `src/apps/cli/src/main.rs:400`, `src/apps/cli/src/root_handlers.rs:100/331`）运行稳定，无用户收益，按 frozen 面守则核销。

---

## 偏离声明与编译错误说明

- **偏离声明**：无任何行为偏离。
- **编译错误排查记录**：
  - 错误：新 worktree 未生成 i18n 契约导致 `E0583: file not found for module generated_locale_contract`。
  - 修复层级：机制层（构建产物），执行 `node scripts/generate-i18n-contract.mjs` 生成契约文件。

---

## 验证结果

1. **`cargo check --workspace`**：
   ```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 50.66s
   ```
2. **`cargo test -p northhing-agent-runtime --test deep_research_contracts`**：
   ```
   running 2 tests
   test deep_research_citation_renumber_owner_is_idempotent_without_citations ... ok
   test deep_research_citation_renumber_owner_preserves_report_and_display_map_contracts ... ok
   test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   ```
3. **`cargo test -p northhing-core-types`**：
   ```
   running 4 tests
   test time::tests::test_now_unix_millis_positive_and_monotonic ... ok
   test errors::tests::builds_ai_error_detail_from_provider_metadata ... ok
   test errors::tests::classifies_quota_and_provider_unavailable_errors ... ok
   test time::tests::test_now_unix_ms_positive_and_monotonic ... ok
   test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   ```
4. **受影响模块针对性测试**：
   - `northhing-services-core session`：48 unit tests + 12 contract tests 全部 pass。
   - `northhing-core cron`：10 tests 全部 pass。
   - `northhing-core evidence_ledger`：7 tests 全部 pass。
   - `northhing-core goal_mode`：12 tests 全部 pass。
5. **`node scripts/check-core-boundaries.mjs`**：
   ```
   Core boundary check passed.
   ```
6. **`pnpm run check:rot`**：
   ```
   ✔ compliant fixture exits 0 and reports success
   ✔ grep count exceeding ceiling fails and exits 1 with guidance message
   ✔ unregistered file exceeding 800 lines fails and exits 1
   ✔ registered god-file exceeding ceiling fails
   ✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry
   ✔ actual workspace rot budget passes with current manifest
   Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1362 files).
   ```
7. **`pnpm run fmt:rs`**：
   ```
   [format-changed-rust] Formatting 7 Rust file(s).
   ```
