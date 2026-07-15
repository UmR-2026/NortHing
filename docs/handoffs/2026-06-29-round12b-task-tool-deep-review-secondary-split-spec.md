# Round 12b Spec: task_tool_deep_review.rs 1693 → policy + tests + thin facade

> **目标**: 关闭 R12 D1 (deep_review 1693 > 1000 cap 693 行 = 69% over)
> **Pattern**: QClaw R12 review report §10 (1 code file + 1 test file + thin re-export facade)
> **Trigger**: 用户要求解决技术债务,不留下新债务

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review.rs` | wc -l |
| 行数 | **1693** | ReadAllLines.Count |
| 内容 | 37 deep_review_* 生产 fns (~870) + setup_deep_review_for_call + 30+ tests (~820) | grep |
| mod.rs 当前 | 5 pub mod | mod.rs |

### 1.1 fn domain 分布 (deep_review 子域)

| 子域 | fn 数 | 内容 |
|---|---|---|
| launch_batch / cache | 2 | launch_batch_for_task, attach_cache |
| retry_guidance | 4 | max_retries, should_emit, ensure_coverage, prompt_with_scope |
| auto_retry | 3 | suppression_reason, ensure_auto_retry_allowed, is_deep_review_auto_retry (在 input 也有) |
| capacity_decision | 2 | capacity_decision_for_provider_error, capacity_skip_result_for_provider_queue_outcome |
| provider_capacity_retry | 4 | wait_for_provider_capacity_retry, record_*, emit_queue_state |
| reviewer_admission | 2 | try_begin_reviewer_admission, wait_for_reviewer_admission |
| local_capacity_skip | 1 | local_capacity_skip_tool_result |
| cancelled_reviewer | 1 | cancelled_reviewer_tool_result |
| setup helper | 1 | setup_deep_review_for_call (call_impl Phase 2) |
| **TOTAL 生产 fns** | **20** | + 17 deep_review_policy_* / retry_* / queue_* fns 都在这 |

注意:我之前 R12 spec 写 37 deep_review_* fns,但实际 wc 显示 ~870 行的 fns 数应该是 ~20 个,其余是重复命名或子分类前缀 (deep_review_capacity_*, deep_review_provider_*)。

### 1.2 Round 5-R12 lessons

| 错误类 | Round hit | R12b 防御 |
|---|---|---|
| Cargo.lock drift | R6 | preflight baseline cargo check |
| cargo check stop-at-first-error | R6 | 4 crates parallel check |
| M3 model 慢 | R6 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Worker 漏 test attribute | R9b | preserve `#[test]`/`#[tokio::test]` |
| mod.rs 漏 `pub mod` | R3b | 每个新 sibling 必须在 mod.rs 加 `pub mod` |
| Cross-reference paths 错 | R11b | facade re-export 保 backward compat |
| `is_concurrency_safe` correctness fix | R12 | R12b 无此问题 |
| Pre-existing unwrap | R11b | 区分 pre-existing vs new, 不"修复" pre-existing |
| R12 D-deviation 1693 > 1000 | **R12** | **本 round 目标:消除** |

### 1.3 QClaw R12 review §10 R12b 方案

```
task_tool_deep_review_policy.rs (~870): 37 deep_review_* 生产 fns + setup_deep_review_for_call
task_tool_deep_review_tests.rs (~820):  30+ tests
```

原 task_tool_deep_review.rs 变成 thin re-export facade (保 backward compat)。

---

## §2 拆分方案

### §2.1 目标文件结构

```
src/crates/assembly/core/src/agentic/tools/implementations/task_tool/
├── mod.rs                                22   +2 pub mod (deep_review_policy, deep_review_tests)
├── task_tool.rs                          582   facade (existing)
├── task_tool_agents.rs                   300   (existing)
├── task_tool_input.rs                    402   (existing)
├── task_tool_subagent.rs                 438   (existing)
├── task_tool_deep_review.rs              NEW ~10  thin re-export facade (pub use task_tool_deep_review_policy::*)
├── task_tool_deep_review_policy.rs       NEW ~870  37 deep_review_* 生产 fns + setup_deep_review_for_call
└── task_tool_deep_review_tests.rs        NEW ~820  30+ tests (with #[cfg(test)] mod tests)
```

### §2.2 目标行数

| File | 目标 | cap | 备注 |
|---|---|---|---|
| mod.rs | 22 | 200 | +2 pub mod |
| task_tool_deep_review.rs (thin facade) | ~10 | 800 | `pub use task_tool_deep_review_policy::*;` |
| task_tool_deep_review_policy.rs | ~870 | 1000 | 生产 fns |
| task_tool_deep_review_tests.rs | ~820 | 1000 | tests in #[cfg(test)] mod |
| **TOTAL** | ~1700 | — | vs R12 = 1693 (略增 due to mod re-exports) |

### §2.3 task_tool_deep_review.rs (thin facade)

```rust
//! Task tool — DeepReview sibling facade (Round 12b thin re-export)
//!
//! Production code lives in `task_tool_deep_review_policy`.
//! Tests live in `task_tool_deep_review_tests`.
//!
//! This file exists only to preserve the `super::task_tool_deep_review::*`
//! import paths used by facade (`task_tool.rs`) and sibling
//! (`task_tool_subagent.rs`) callers. Without this re-export facade, every
//! caller would need to migrate to `super::task_tool_deep_review_policy::*`.
//!
//! Spec: `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md`
//! Pattern: QClaw R12 review §10 (code + tests + thin facade).

pub use super::task_tool_deep_review_policy::*;
```

### §2.4 task_tool_deep_review_policy.rs 内容

37 deep_review_* 生产 fns + setup_deep_review_for_call helper。**所有 fn 已经在原 task_tool_deep_review.rs 中,直接复制 + 修复 use 路径**。

Imports 需要调整:
- `use super::task_tool_input::CallInputs;` → 保持
- `use super::task_tool_subagent::DeepReviewContext;` → 保持
- 去掉 `#[cfg(test)] mod tests { ... }` 整块

### §2.5 task_tool_deep_review_tests.rs 内容

30+ 测试 fns + `PromptOrderTestAgent` 等 helpers。**所有 test code 在原 mod tests 中,直接复制 + 加 `#[cfg(test)] mod tests { ... }` 外壳**。

Imports 需要:
- `use super::super::task_tool_deep_review_policy::*;` (引用生产 fn)
- `use super::super::task_tool_input::CallInputs;` (测试需要的类型)
- 等等

### §2.6 mod.rs 改动

```rust
//! Task tool implementations (Round 12 + Round 12b split)
//!
//! Round 12 split: TaskTool + impl + 5 sub-handler siblings per fn domain.
//! Round 12b split: task_tool_deep_review further split into policy (production)
//! + tests (#[cfg(test)]) + thin facade (backward compat).
//!
//! - `task_tool` (facade): `TaskTool` struct + Tool trait impl + tool_core fns + call_impl orchestrator
//! - `task_tool_deep_review` (thin facade): pub use re-exports
//! - `task_tool_deep_review_policy` (~870): 37 deep_review_* fns + setup helper
//! - `task_tool_deep_review_tests` (~820): 30+ tests
//! - `task_tool_subagent` (~450): 10 subagent fns + 2 tests + call_impl subagent dispatch/loop
//! - `task_tool_agents` (~300): 6 agent fns + 2 tests + call_impl completion result + PromptOrderTestAgent
//! - `task_tool_input` (~250): 5 input validation fns + 2 tests + call_impl input prep phase

pub mod task_tool;
pub mod task_tool_deep_review;
pub mod task_tool_deep_review_policy;
pub mod task_tool_deep_review_tests;
pub mod task_tool_subagent;
pub mod task_tool_agents;
pub mod task_tool_input;

// Re-export public API (preserves caller compatibility: `crate::...::task_tool::TaskTool`)
pub use task_tool::TaskTool;
```

### §2.7 关键:跨文件 fn 调用 path 修复

由于 thin facade 模式,所有跨文件 `super::task_tool_deep_review::*` 调用**继续工作**(因为 facade re-export)。

- `task_tool.rs:442` `super::task_tool_deep_review::prompt_with_deep_review_retry_scope(...)` ✅
- `task_tool.rs:445` `super::task_tool_deep_review::should_emit_deep_review_retry_guidance(...)` ✅
- `task_tool_subagent.rs:217+` `super::task_tool_deep_review::*` ✅
- `task_tool_deep_review_tests.rs` `use super::super::task_tool_deep_review_policy::*;` ✅

**零 caller migration 成本**,thin facade 承担路径兼容。

---

## §3 验证策略

### §3.1 编译验证

```bash
cd E:\agent-project\northing
git fetch origin
git worktree add ../northing-impl-round12b -b impl/round12b-task-tool-deep-review-secondary-split main

# preflight baseline (R12 已知 899/0/1)
git checkout origin/main
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object baseline-main-cargo-check.log
cargo test -p northhing-core --features product-full --lib 2>&1 | Tee-Object baseline-main-cargo-test.log

$baselineErrors = (Select-String -Path baseline-main-cargo-check.log -Pattern "error\[" | Measure-Object).Count
$baselineTestResult = (Select-String -Path baseline-main-cargo-test.log -Pattern "test result:" | Select-Object -First 1).ToString()
Write-Host "BASELINE_ERRORS=$baselineErrors"  # expect 0
Write-Host "BASELINE_TESTS=$baselineTestResult"  # expect 899/0/1

git checkout impl/round12b-task-tool-deep-review-secondary-split
```

### §3.2 测试验证

```bash
cargo test -p northhing-core --features product-full --lib
# expect: 899/0/1 (与 R12 baseline 一致)
```

### §3.3 line count 验证

```bash
# 4 sibling files line counts
for sibling in task_tool_deep_review task_tool_deep_review_policy task_tool_deep_review_tests task_tool_subagent task_tool_agents task_tool_input task_tool; do
  py -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing-impl-round12b\src\crates\assembly\core\src\agentic\tools\implementations\task_tool/${sibling}.rs', encoding='utf-8')))"
done

# expected:
# task_tool_deep_review.rs:  ~10 (thin facade)
# task_tool_deep_review_policy.rs:  ~870 (production, ≤ 1000)
# task_tool_deep_review_tests.rs:   ~820 (tests, ≤ 1000)
# task_tool_subagent.rs:            438 (unchanged)
# task_tool_agents.rs:              300 (unchanged)
# task_tool_input.rs:               402 (unchanged)
# task_tool.rs:                     582 (unchanged)
```

---

## §4 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| task_tool_deep_review_policy.rs 1000 cap | ≤ 1000 | ~870 | R12 数据估算 |
| task_tool_deep_review_tests.rs 1000 cap | ≤ 1000 | ~820 | R12 数据估算 |
| task_tool_deep_review.rs thin facade | ≤ 50 | ~10 | re-export only |

如果任一文件超 1000,需要 R12c 三级拆。

---

## §5 实施步骤 (R12b take-over: Mavis direct, mechanical split)

1. **task_tool_deep_review_policy.rs** (~870): 复制 37 fns + setup helper from 原 task_tool_deep_review.rs, 修复 imports (去掉 tests mod) + cargo check + 报告行数
2. **task_tool_deep_review_tests.rs** (~820): 复制 mod tests 整块 from 原 task_tool_deep_review.rs, 加 `#[cfg(test)] mod tests { ... }` 外壳 + `use super::super::task_tool_deep_review_policy::*;` + cargo check + 报告行数
3. **task_tool_deep_review.rs** (thin facade): 删除原 1693 行内容, 替换为 ~10 行 `pub use super::task_tool_deep_review_policy::*;` + cargo check
4. **mod.rs**: +2 pub mod 声明 + cargo check
5. **全 crate 测试**: cargo test -p northhing-core --features product-full --lib → expect 899/0/1
6. **fmt**: cargo fmt -- src/.../task_tool/
7. **commit + merge + handoff**

**每步必须**: cargo check 0 errors + **报告当前 sibling 行数**(超 1000 立即考虑 R12c 二次拆)

### Critical: cargo check stop-at-first-error prevention (R6 教训)

```bash
cargo check -p northhing-tools-execution --features product-full --lib --message-format=short
cargo check -p northhing-core --features product-full --lib --message-format=short
cargo check -p northhing-tool-provider-groups --features product-full --lib --message-format=short
```

### Critical: Cargo.lock drift check (R6 教训)

```bash
git show origin/main:Cargo.lock | Select-String 'name = "rmcp"'
Get-Content Cargo.lock | Select-String 'name = "rmcp"'
```

### Critical: 12-class sub-domain errors (R11b/R12 lessons reinforced)

1. **Import paths**: deep_review_tests 用 `use super::super::task_tool_deep_review_policy::*;` 引用生产 fn
2. **Sibling method visibility**: pub(super) for cross-sibling; pub(crate) for external
3. **Struct field visibility**: `CallInputs`, `DeepReviewContext`, `ExecuteOutcome` use pub(super) fields
4. **Cargo.lock drift**: see above
5. **mod.rs `pub mod`**: 必须加 deep_review_policy + deep_review_tests
6. **Test attribute preservation**: preserve `#[test]` / `#[tokio::test]`
7. **cargo check stop-at-first-error**: see above
8. **Cross-sibling shared enum/trait**: DeepReviewContext 在 subagent sibling,DeepReviewQueueWaitOutcome 在 task_adapter (无变化)
9. **R10a unused imports**: 精确 use blocks
10. **R11a struct owner mapping**: DeepReviewContext stays in subagent sibling (call_impl 跨 phase 需要)
11. **Worker 每步报告行数**: cargo check 后 wc -l 当前 sibling,超 cap 调整
12. **R11b cross-reference paths**: thin facade 承担 backward compat, caller 无需修改

---

## §6 Verification

```bash
# 0 NEW unwrap/panic/unreachable
git diff origin/main..HEAD -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ \
  | Select-String '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# expect 0

# 0 fns dropped (R12 baseline 98 fns preserved)
py -c "
import re
from pathlib import Path
wt_dir = Path(r'E:\agent-project\northing-impl-round12b\src\crates\assembly\core\src\agentic\tools\implementations\task_tool')
fns = set()
for f in wt_dir.glob('*.rs'):
    fns.update(re.findall(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'worktree fns: {len(fns)}')
print('expected: 98')
"

# Cargo test baseline
cargo test -p northhing-core --features product-full --lib
# expect 899/0/1

# Cargo fmt
cargo fmt --check src/crates/assembly/core/src/agentic/tools/implementations/task_tool/
```

---

## §7 spec review check-list

QClaw 重点检查:
1. 4 sibling 文件拆分结构 (跟 R11b/R12 pattern 一致)
2. thin facade re-export 保 backward compat (zero caller migration)
3. cross-file fn calls 继续工作 (不需要改 task_tool.rs / task_tool_subagent.rs)
4. line counts ≤ 1000 (task_tool_deep_review_policy 870, tests 820)
5. 0 fns dropped (98 → 98)
6. pre-existing unwrap preserved (R11b pattern: 不"修复" pre-existing)
7. cargo test 899/0/1 preserved
8. mod.rs 加 2 pub mod 声明

---

## §8 Errata

- §2.1 task_tool_deep_review.rs thin facade ~10 行,实际可能略多 (含 doc comments ~15 行)
- §2.4 task_tool_deep_review_policy.rs ~870 是估算,实际可能 850-900 之间
- §5 实施步骤: Mavis take-over (R12 worker error 后我们已确认 take-over 流程比 dispatch 快)
- §2.6 mod.rs 改动 minimal,只 +2 pub mod
- thin facade 模式是 R11b 已用过的 (sub_facade + cross-reference paths preserve)