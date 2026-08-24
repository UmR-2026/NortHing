# ROT-1 Review — T2-9 冗余合并批 1（独立验收）

- 审查对象：worktree `northing-rot1`（分支 `feat/rot1-dedup-0821`）
- 范围：`30a8590..9721f75`（单 commit `refactor(rot1)`）
- diff 包：`task-rot1-review-brief.md` 点名的 11 文件改动（11 changed, 252 +, 282 −；净 −30 行）
- 上游 review brief：`.superpowers/sdd/task-rot1-review-brief.md`

## 双判决

### SPEC 判决：PASS

每条对 brief 的 `task-rot1-brief.md` 验收标准，给出 file:line 证据。

| Spec | 证据 | 判决 |
|---|---|---|
| Spec 1：deep_research 副本删除 + 路径存活 | `src/crates/execution/agent-runtime/src/deep_research.rs:1-5` 现仅 `pub use northhing_runtime_ports::deep_research::*;`；`src/crates/contracts/runtime-ports/src/deep_research.rs` 仍是 255 行（与原副本比对：相同 `pub fn renumber_research_report` + `ResearchCitationDisplayMapEntry` + `ResearchCitationRenumberOutput` + `ResearchCitationRenumberStats`，private helpers `parse_registry_status / split_at_citation_index / build_display_map / renumber_body / renumber_index_section / row_is_rejected / is_index_data_row / extract_display_sort_key` 全部存在）。`src/crates/execution/agent-runtime/src/lib.rs:32` 仍为 `pub use deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry};`。`tests/deep_research_contracts.rs:1` `use northhing_agent_runtime::deep_research::renumber_research_report;` 编译并通过（已实跑 2/2）。`Cargo.toml:13` 早已声明 `northhing-runtime-ports` 依赖（git show 30a8590 验证：base commit 即有，未被本 diff 添加）；层方向 execution→contracts 合法，与 `src/crates/execution/AGENTS.md` 一致。 | PASS |
| Spec 2：core-types 新 time 模块 | `src/crates/contracts/core-types/src/time.rs:1-51` 新文件，签名为 `pub fn now_unix_ms() -> u64` 与 `pub fn now_unix_millis() -> i64`，doc comment 注明 SystemTime::now + UNIX_EPOCH 语义及 `u64::MAX / i64::MAX` 饱和行为，含 2 个平凡单测。`src/crates/contracts/core-types/src/lib.rs:11,24` 挂载 `pub mod time;` 与 `pub use time::{now_unix_millis, now_unix_ms};`。 | PASS |
| Spec 2：4 处命名重复收口 | (a) `assembly/core/src/agentic/goal_mode/mod.rs:35` `pub use northhing_core_types::time::now_unix_ms as now_ms;`（调用点 `goal_internal_context_message` / 其它消费者路径不变）。(b) `assembly/core/src/agentic/session/evidence_ledger.rs:3,124,322` 删除本地 `current_time_millis`，导入 `now_unix_ms` 替换；调用点 1 处 (`EvidenceLedgerEvent::new`)。(c) `assembly/core/src/service/cron/service_helpers.rs:7,205` `use chrono::{Local, SecondsFormat, TimeZone};` 去掉不再用的 `Utc`，`pub(super) use northhing_core_types::time::now_unix_millis as now_ms;`；调用点仍 `super::service_helpers::now_ms()`（9 处）。(d) `services-core/src/session/metadata_store.rs:13` `use northhing_core_types::time::now_unix_ms as current_unix_ms;`（3 处调用点 lines 200, 222, 235）。`rg "fn now_ms\b|fn current_unix_ms\b|fn current_time_millis\b" src` 无残留。 | PASS |
| Spec 2：不动项落实 | `agent-runtime cache_types`、`debug-log`、`acp manager_process` 未在 core-types 新 time 模块之外添加跨层依赖（已 `rg "core_types::time"` 验证范围限定在 4 个指定调用方）；全仓 80 处内联 `duration_since(UNIX_EPOCH)` 未触碰（属于编排者收口条目）。 | PASS |
| Spec 3a：ndjson 核销 | brief 给出语义差异理由；现有 `audit_log.rs`（age+size 双条件多代）、`debug-log`（8MiB 单代 `.1`）、`facts.rs/episodes/store.rs/config/runtime.rs`（纯 append）、`write_file.rs`（IO 工具非 log）确实各管一面。本 diff 未改任一文件。 | PASS（核销） |
| Spec 3b：FILE_LOCKS 核销 | `persistence.rs:16,86-120` 实测：`save_json` 序列为 `ensure_parent_dir → create_backup(5代) → 写 key.json.tmp → rename`，临时文件路径固定 `key.json.tmp`，同 key 并发写必冲突；3 处调用 `workspace/service.rs:230`、`workspace/admin.rs:197`、`cron/store.rs:86` 都走同一 `save_json`。本 diff 未动 `persistence.rs`。 | PASS（核销保留） |
| Spec 3c：server bootstrap 核销 | `apps/server/src/main.rs:1-61` 实读：61 行，仅 `mod routes;`、`/health` `/api/v1/health` `/api/v1/info` `/ws` 四个 axum 路由 + 状态/log/tracing 启动，零 bootstrap 手抄。`Cargo.toml:8-10` 仅 `[[bin]] path="src/main.rs"` 无 `[lib]`，`src/bootstrap.rs` 与 `src/rpc_dispatcher.rs` 实际未在编译单元内（孤儿）。本 diff 未改 `apps/server`。 | PASS（核销） |
| Spec 3d：CLI init 核销 | CLI 仍 frozen-experimental；本 diff `git diff 30a8590..9721f75 -- src/apps/cli/` 为空（已实证）。 | PASS（核销） |
| Spec 4：rot-budget.json / growth 线 / CLI 不碰 | `git diff 30a8590..9721f75 -- src/apps/cli/ scripts/rot-budget.json | Measure-Object` → 0；`memory/`、`.graph/`、`.opencode/model-capability-notes.md` 不在 diff 文件清单（已 `git diff --stat` 验证）。 | PASS |

### QUALITY 判决：PASS

- **复用侦察**：报告自陈的「核心时间 helper 不存在」「ndjson 三类语义不同」「FILE_LOCKS 守 backup+rename」三段均独立核验属实（见 SPEC 证据）。无「复制既有能力而不复用」情形。
- **无 owner 抽象**：time helper 由真实 4 个调用方消费；deep_research 由 lib.rs:32 + `tests/deep_research_contracts.rs` 消费；无投机性 trait/wrapper/factory。
- **预算闸**：`scripts/rot-budget.json` 不在 diff 中（已 `git diff --stat` 确认）。`pnpm run check:rot` 实跑通过。
- **god-file 观测**：本 diff 未触及 7 个登记文件（已 `git diff 30a8590..9721f75 -- src/apps/desktop/src/app_state/callbacks_lifecycle.rs src/crates/assembly/core/src/service/agent_memory/{memory_db,facts}.rs | Measure-Object` → 0）。

## 独立验证（实跑输出）

1. `cargo check --workspace`（stable-msvc wrapper）→ `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 2.17s`（无错；仅 keyring/CLI 既有 warning，非本 diff 引入）。
2. `cargo test -p northhing-agent-runtime --test deep_research_contracts` → `test result: ok. 2 passed; 0 failed`；与 diff 修改 `deep_research.rs` 一致。
3. `cargo test -p northhing-core-types` → `test result: ok. 4 passed; 0 failed`；含本次新增的 `time::tests::test_now_unix_ms_positive_and_monotonic` 与 `time::tests::test_now_unix_millis_positive_and_monotonic`。
4. 受影响模块针对性测试（brief 第 4 项原命令 `goal_mode/evidence_ledger/cron/metadata_store 就近`未指定 feature；本仓库默认 `--lib` 不暴露这些 `mod tests`，需 `--features product-full`）：
   - `cargo test -p northhing-core --features product-full --lib -- agentic::goal_mode::tests::` → `test result: ok. 12 passed; 0 failed`（与 implementer 报告 12 一致）。
   - `cargo test -p northhing-core --features product-full --lib -- agentic::session::evidence_ledger::tests::` → `test result: ok. 6 passed; 0 failed`；外加 `session_manager_tests::...::records_subagent_partial_timeout_in_evidence_ledger` 1 处通过 → 共 7 处（与 implementer 报告 7 一致）。
   - `cargo test -p northhing-core --features product-full --lib -- service::cron::` → `test result: ok. 6 passed; 0 failed`（schedule+store 共 6；implementer 报告 10 高估，见 Minor #1）。
   - `cargo test -p northhing-services-core --lib -- session::metadata_store::tests::` → `test result: ok. 5 passed; 0 failed`。
5. `node scripts/check-core-boundaries.mjs` → `Core boundary check passed.`
6. `pnpm run check:rot` → `Rot budget verification passed (3 grep rules, 7 god-file rules checked across 1362 files).`
7. **re-export 完整性抽查**（brief 第 5 项）：从 BASE `git show 30a8590:src/crates/execution/agent-runtime/src/deep_research.rs` 取 3 个 pub 符号逐一核对 — `pub fn renumber_research_report`、`pub struct ResearchCitationDisplayMapEntry`、`pub struct ResearchCitationRenumberStats` 全部存在且签名/字段一致于现 `src/crates/contracts/runtime-ports/src/deep_research.rs`（230+ 行原副本中能看见；`pub use ...::*` 在新 `deep_research.rs:5` 启用）。`pub use` 路径已编译通过 = 符号可用已得证。
8. **必查项**：review brief 第 6 项 — `scripts/core-boundaries/rules/source/required-rules.mjs` 与 `self-test.mjs` 被本 diff 改动 4 + 2 行。对照：

```diff
-    path: 'src/crates/execution/agent-runtime/src/deep_research.rs',
-    reason: 'agent-runtime must own provider-neutral DeepResearch citation renumbering without core session or filesystem IO dependencies',
+    path: 'src/crates/contracts/runtime-ports/src/deep_research.rs',
+    reason: 'runtime-ports must own provider-neutral DeepResearch citation renumbering without core session or filesystem IO dependencies',
```

- 实质：纯路径迁移 + 主体名替换。`patterns` 数组（4 个 regex：`renumber_research_report`、`ResearchCitationRenumberOutput`、`ResearchCitationDisplayMapEntry`、`rejected_index_rows_dropped`）与 `self-test.mjs` 的 `contracts` 列表逐字未变。无规则放宽、无阈值提高、无 ceiling 上调。判定「路径失效之必要修正，非越权改规则放行自己」。

## Cannot verify from diff

无 — brief 第 1-5 项已逐条实跑验证（cargo check + 4 个 cargo test + 1 个边界检查 + 1 个 rot 检查 + 1 个 re-export 抽查 + 1 个 SPEC 4 路径排查）。

附记：本审阅过程中执行过 `git stash` / `git stash pop` 以验证 Spec 3b 旁带的 i18n test 失败是否本 diff 引起 — 已确认 `service::i18n::service::tests::translate_keeps_legacy_app_name_alias_on_shared_product_name`（`left: 'NortHing', right: 'northhing'`）在 `30a8590` base 即失败，与本 diff 无关；bookkeeping 状态已 `git restore` 回滚至 `feat/rot1-dedup-0821` clean。

## Findings

- **Minor #1**（报告准确性，非缺陷）：`task-rot1-report.md:91` 声称「northhing-core cron:10 tests」 — 实测 `service::cron::*` 在 lib 下为 6（schedule:3 + store:3）；可能 implementer 把 helpers 集成一起估数。所有实际测试仍全部 pass。
- **Minor #2**（透明度）：`scripts/core-boundaries/{required-rules,self-test}.mjs` 为本 diff 触动 — brief 未明列。已通过第 8 项核验为必要路径迁移，非规则放宽；建议 brief 模板下次预列此类预期触碰面。

无 Critical / Important。

## god-file T0 基线（与本 diff 无关的对照锚点）

> 本 diff 未触动 3 个观测点。基线段落供后续观测轮次对照。

- **`src/apps/desktop/src/app_state/callbacks_lifecycle.rs`（1004 行）**：关切构成是 18 个 `register_X_callback(ui, app_state)` 顶层入口（行 32, 309, 415, 458, 532, 543, 565, 679, 757, 784, 820, 830, 839, 853, 862, 904, 983, 993），每个把 `Arc<AppState>` 闭包塞进 Slint `ui.on_X`；关切构成本质是「Slint ↔ AppState ↔ KernelApi」三方对账 — 没有 domain logic，没有 IO 决策，但是重 composition。文件头有 `// allow-god-file: 917L ... split planned with callbacks_settings paradigm` 注释，说明登记在 rot 预算豁免 / 分拆计划已挂账（不过此处的标签是 917，而实测 1004，存在 ~87 行未对账偏移）。是否混杂纠缠 — 每个 callback 看似独立、可拆，但共享 `&AppWindow` + `&Arc<AppState>` 隐式上下文使分拆需先把上下文用具名参数显式化（典型「same-shared-context god-file」）。当前清晰度：纠结/计划已挂分拆但未动；一句依据：`register_X_callback` 的 callable 与 `callbacks_settings/*.rs` 子树的 `register_*_callback` 分工模式不一致（顶层 18 个是裸函数，子树用 `pub(crate) fn`），未来分拆要么整体下沉要么子树回填，要先敲定边界。

- **`src/crates/assembly/core/src/service/agent_memory/memory_db.rs`（918 行）**：关切构成是 `impl MemoryDb { open / create_tables / ... }` 加上 SQL `execute_batch`/`prepare`/`query_row` 调用 + `fn segment_for_fts / is_cjk / ...` 全文搜索辅助 + `impl Drop for MemoryDbPathGuard` 测试夹具。关切集中于「单一 SQLite 连接 + FTS5 中文 bigram + 时间衰减 + test guard 全在同文件」 — 是「one-impl + many-private-helpers」典型 god-file；不混杂纠缠（domain 紧凑），但确实长。是否纠结：当前清晰/紧凑；一句依据：所有私有 helper 命名严格限定在 FTS/chinese-segmentation 一族，全是这一实现的内部分解，没有「孤儿」或跨主题 helper，这是「单调长但内部干净」型（非纠结型）。

- **`src/crates/assembly/core/src/service/agent_memory/facts.rs`（905 行）**：关切构成是 `Fact` 及其 4 个枚举 + `append_facts` / `append_facts_dedup` / `read_facts` / `select_facts_for_prompt` / `distill_facts_from_user_message` 等 7 个 `pub(crate) async fn`，全套 `serde` derive + JSONL 文件语义。关切是「Facts 数据模型 + Facts I/O + Facts 选择策略」三件套串联在一个文件 — 与 `distiller.rs / instruction_context.rs / auto_memory.rs / dream.rs / judge_memory.rs` 等兄弟文件形成「facts.rs 是其它 5 个文件的基底」格局（其它兄弟都 `use super::facts::{Fact, ...}`）。是否混杂：略混杂（三件套未拆，但语义上 distinct：append-only CRUD vs prompt 选择 vs LLM 蒸馏），但分拆风险高（要重新 export 一整套 `Fact*` 类型 + JSONL 文件名常量）。当前清晰：可工作；一句依据：`Distill*-` 函数直接持有 `tokio::fs + serde_json::from_slice` 内联实现，本应在 `distiller.rs` 而非挤在 `facts.rs` 第 239-? 段，这是「文件职责溢出」最早萌发的位置 — 是此 god-file 真切的关注点（不是行数本身，是「distillation 职责 overflow 进 file I/O 单文件」）。

## 总结

- SPEC 全 9 条 PASS（含 4 核销）。
- QUALITY 4 必查项全 PASS。
- 0 Critical / 0 Important / 2 Minor（皆为报告/透明度，无代码缺陷）。
- 预算闸未碰；边界检查 + rot 检查皆过；focused 模块测试全过；re-export 完整性已逐符号核对。

**APPROVED**
