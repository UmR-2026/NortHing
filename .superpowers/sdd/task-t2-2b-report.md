DONE

# Task T2-2b Report: judge_gate 适配层删除（协议层保留为 TH-5 词汇）

## 1. 逐项执行状态

- **D1. 删适配层目录（1,690 rs 行）**: 完成
  - 删除目录 `src/crates/assembly/core/src/agentic/judge_gate/`（4 个 .rs 文件：`audit.rs` 405L, `mod.rs` 931L, `receipt_store.rs` 101L, `runner.rs` 253L，共 1,690 rs 行）
  - `src/crates/assembly/core/src/agentic/mod.rs` 移除 `// Judge gate adapter module` 注释与 `pub(crate) mod judge_gate;` 声明（-3 行）
  - 删后归零复核：`rg -n -i "judge_gate" src/crates/assembly/core --glob "*.rs"` 命中数为 0（`registry_store.rs:333` 的 `"judge-gate"` doc 注释保留未动）
- **D2. boundary 规则同步（`scripts/core-boundaries/rules/source/forbidden-rules.mjs`）**: 完成
  - 删除了 adapter 规则块（`path: 'src/crates/assembly/core/src/agentic/judge_gate'` 及其 3 条断言，-22 行）
  - 保留 protocol 规则块（`path: 'src/crates/execution/agent-runtime/src/judge_gate'`）与全部 `judge_memory` 规则
  - 验证：`node scripts/check-core-boundaries.mjs` 输出 `Core boundary check passed.`
- **D3. 协议层防再误删注解（`src/crates/execution/agent-runtime/AGENTS.md`）**: 完成
  - 在 `src/crates/execution/agent-runtime/AGENTS.md` 的 `## Guardrails` 中追加说明：`judge_gate` 协议层（`types`, `verdict`, `redlines`, `evidence`, `brief` 纯类型）为 TH-5 身份演化（T3-8）保留词汇（T-08/G15 拍板），零接线债属有意保留，后续清理轮勿标记为死代码。
- **D4. 台账 P2-11 注解（`docs/status/tech-debt-ledger.md`）**: 完成
  - P2-11 保持 `Status: resolved`，在末尾追加 `Note (2026-08-18 T2-2b)` 注解，记录适配层整体已删（含 `receipt_store.rs` 的 append-only JSONL + LazyLock 重放实现，`47b6202`），教训移交 TH-5（T3-8）：consume-once 凭证必须 append-only 持久化 + 初始化重放，否则重启可重放已消费凭证。

---

## 2. 验证原始输出

### 1. `cargo check --workspace` (MSVC)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 45.16s
```

### 2. `cargo check -p northhing` (MSVC)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.52s
```

### 3. `node scripts/check-core-boundaries.mjs`
```bash
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 4. `cargo test -p northhing-agent-runtime` (MSVC)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-agent-runtime
```
输出（摘要）：
```text
running 153 tests
...
test judge_gate::brief::tests::brief_contains_verdict_markers ... ok
test judge_gate::brief::tests::brief_contains_evidence_ids ... ok
test judge_gate::brief::tests::brief_contains_subject_digest ... ok
test judge_gate::evidence::tests::both_traces_and_fs_diffs_empty_rejected ... ok
test judge_gate::brief::tests::brief_contains_four_redline_ids_and_text ... ok
test judge_gate::brief::tests::brief_contains_weight_zero_instruction ... ok
test judge_gate::evidence::tests::evidence_ids_correct_format ... ok
test judge_gate::evidence::tests::evidence_ids_human_absent_no_h_ids ... ok
test judge_gate::evidence::tests::evidence_ids_with_fs_diffs_and_human_feedback ... ok
test judge_gate::evidence::tests::excerpt_401_chars_rejected ... ok
test judge_gate::evidence::tests::path_with_northhing_backslash_episodes_rejected ... ok
test judge_gate::evidence::tests::path_with_northhing_episodes_rejected ... ok
test judge_gate::evidence::tests::total_budget_over_12k_rejected ... ok
test judge_gate::evidence::tests::traces_over_16_rejected ... ok
test judge_gate::evidence::tests::turn_id_whitespace_rejected ... ok
test judge_gate::evidence::tests::valid_pack_passes_validation ... ok
test judge_gate::redlines::tests::redline_ids_ordered_correctly ... ok
test judge_gate::redlines::tests::redline_statements_all_non_empty ... ok
test judge_gate::redlines::tests::redline_table_length_is_four ... ok
test judge_gate::types::tests::action_kind_serialize_only_promote_skill_candidate ... ok
test judge_gate::types::tests::subject_digest_fixed_value ... ok
test judge_gate::types::tests::subject_digest_format ... ok
test judge_gate::verdict::tests::parse_approve_all_pass_ok ... ok
test judge_gate::verdict::tests::parse_approve_but_not_all_pass_semantic_check ... ok
test judge_gate::verdict::tests::parse_block_not_json ... ok
test judge_gate::verdict::tests::parse_duplicate_rule ... ok
test judge_gate::verdict::tests::parse_evidence_assessment_empty ... ok
test judge_gate::verdict::tests::parse_evidence_assessment_nonexistent_reference ... ok
test judge_gate::verdict::tests::parse_extra_rule ... ok
test judge_gate::verdict::tests::parse_missing_rule ... ok
test judge_gate::verdict::tests::parse_rationale_empty ... ok
test judge_gate::verdict::tests::parse_reject_with_all_pass ... ok
test judge_gate::verdict::tests::parse_status_invalid_value ... ok
test judge_gate::verdict::tests::parse_status_not_evaluated ... ok
test judge_gate::verdict::tests::parse_two_begin_markers_rejected ... ok
test judge_gate::verdict::tests::parse_two_blocks ... ok
test judge_gate::verdict::tests::parse_two_end_markers_rejected ... ok
test judge_gate::verdict::tests::parse_unknown_rule ... ok
test judge_gate::verdict::tests::parse_verdict_invalid_value ... ok
test judge_gate::verdict::tests::parse_zero_blocks ... ok
...
test result: ok. 153 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
...
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
...
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
...
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5. D1/D2 删后归零 grep 输出

命令 1：
```bash
rg -n -i "judge_gate" src/crates/assembly/core --glob "*.rs"
```
输出：
```text
(0 matches)
```

命令 2：
```bash
rg -n "judge_gate" scripts/core-boundaries/
```
输出：
```text
scripts/core-boundaries/rules\source\forbidden-rules.mjs:2347:    path: 'src/crates/execution/agent-runtime/src/judge_gate',
scripts/core-boundaries/rules\source\forbidden-rules.mjs:2349:      'judge_gate protocol layer must not depend on northhing-core or episodes storage (zero dependency edge, C4 Phase 0 §5.1)',
scripts/core-boundaries/rules\source\forbidden-rules.mjs:2354:          'judge_gate protocol must not import northhing_core (zero dependency edge requirement)',
scripts/core-boundaries/rules\source\forbidden-rules.mjs:2359:          'judge_gate protocol must not import northhing-core (zero dependency edge requirement)',
scripts/core-boundaries/rules\source\forbidden-rules.mjs:2364:          'judge_gate protocol must not import episodes (zero dependency edge requirement)',
```

### 6. `git diff --stat` 摘要
```bash
git diff --stat -- docs/status/tech-debt-ledger.md scripts/core-boundaries/rules/source/forbidden-rules.mjs src/crates/assembly/core/src/agentic/ src/crates/execution/agent-runtime/AGENTS.md
```
输出：
```text
 docs/status/tech-debt-ledger.md                    |   1 +
 .../rules/source/forbidden-rules.mjs               |  22 -
 .../assembly/core/src/agentic/judge_gate/audit.rs  | 405 ---------
 .../assembly/core/src/agentic/judge_gate/mod.rs    | 931 ---------------------
 .../core/src/agentic/judge_gate/receipt_store.rs   | 101 ---
 .../assembly/core/src/agentic/judge_gate/runner.rs | 253 ------
 src/crates/assembly/core/src/agentic/mod.rs        |   3 -
 src/crates/execution/agent-runtime/AGENTS.md       |   1 +
 8 files changed, 2 insertions(+), 1715 deletions(-)
```

---

## 3. 行数对账

- **适配层删除**：
  - `src/crates/assembly/core/src/agentic/judge_gate/audit.rs`: -405 lines
  - `src/crates/assembly/core/src/agentic/judge_gate/mod.rs`: -931 lines
  - `src/crates/assembly/core/src/agentic/judge_gate/receipt_store.rs`: -101 lines
  - `src/crates/assembly/core/src/agentic/judge_gate/runner.rs`: -253 lines
  - `src/crates/assembly/core/src/agentic/mod.rs`: -3 lines (注释 + 模块导出)
  - **总计删除 Rust 代码行数**: 1,693 行（预期 ≈ 1,690 行）
- **规则与文档调整**：
  - `scripts/core-boundaries/rules/source/forbidden-rules.mjs`: -22 lines (移除 adapter 规则块，保留 protocol 块)
  - `src/crates/execution/agent-runtime/AGENTS.md`: +1 line (协议层保留注解)
  - `docs/status/tech-debt-ledger.md`: +1 line (P2-11 教训交接注解)
- **协议层保留**：
  - `src/crates/execution/agent-runtime/src/judge_gate/` 6 个文件（1,473 行纯类型）0 改动。

---

## 4. 遗留疑虑

无（Zero concerns）。未触碰任何排除项与并行 session 资产，所有工作区改动保持未提交状态。
