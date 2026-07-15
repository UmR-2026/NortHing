# Round 12 Spec: task_tool.rs 3085 → facade + 4 sub-handlers (deep_review 大头)

> **目标**: `agentic/tools/implementations/task_tool.rs` 3085 行 → `task_tool/` subdir + mod.rs facade + 4 sub-handler files
> **Pattern**: Round 11 sub-domain split (free fns + struct impl, R11a/b 经验强化)
> **Trigger**: critical #2 of post-R10 god object list

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/assembly/core/src/agentic/tools/implementations/task_tool.rs` | wc -l |
| 行数 | **3085** | ReadAllLines.Count |
| impl 块 | 3 (Default for TaskTool + TaskTool + Tool for TaskTool) | grep |
| 总 fn 数 | **98** | loose regex |
| `deep_review_*` fn | **37** (大头 38%) | prefix count |
| `subagent_*` fn | 10 | misc |
| `agent_*` fn | 6 | misc |
| `input_validation` fn | 5 | misc |
| `tool_core` fn (含 call_impl) | 5 | facade |

### 1.1 fn domain 分布 (按 prefix)

| Domain | fn count | 候选 sibling |
|---|---|---|
| deep_review | 37 | `task_tool_deep_review.rs` |
| subagent | 10 | `task_tool_subagent.rs` |
| agent | 6 | `task_tool_agents.rs` |
| input_validation | 5 | `task_tool_input.rs` |
| tool_core (call_impl 等) | 5 | facade |
| misc (含 Tool trait impl) | 35 | facade |
| **TOTAL** | **98** | **5 files** |

### 1.2 关键结构

- `TaskTool` struct (空 struct, default new())
- `impl TaskTool` (含 call_impl + deep_review_launch_batch_for_task)
- `impl Tool for TaskTool` (Tool trait impl: name/description/call_impl/...)
- `impl Default for TaskTool` (default = new)

`call_impl` 是 god method (含 deep_review launch, subagent dispatch, input validation), 估计 200-400 行。

### 1.3 Round 5-R11b lessons

| 错误类 | Round hit | R12 防御 |
|---|---|---|
| Cargo.lock drift (rmcp 1.7→1.8) | R6 | Plan YAML preflight baseline cargo check |
| cargo check stop-at-first-error | R6 (32+ errors) | worker 报"0 NEW errors"必须每个 crate 都跑过 |
| M3 model 慢 (39min silence) | R6 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Worker 漏 test attribute | R9b (2 fns) | worker split 必须保留 `#[test]`/`#[tokio::test]` attribute |
| mod.rs 漏 `pub mod` | R3b | 每个新 sibling 必须在 mod.rs 加 `pub mod` |
| Spec 不列 struct owner → worker 拆错 | R11a | R12 spec §5 显式 struct/fn mapping |
| Worker 没报告行数 → D-deviation | R11a | R12 spec §6 强制 "报告当前 sibling 行数" |
| Cross-reference paths 错 | R11b (6 paths) | R12 spec 列出哪些 type 在 mod.rs vs sibling |
| 测 0 lines 漂移 | R6 audit | 用 `[System.IO.File]::ReadAllLines().Count` (= wc -l) |
| Pre-existing unwrap | R11b (26 unwrap) | 区分 pre-existing vs new, 不"修复" pre-existing |

---

## §2 拆分方案（sub-domain split per fn domain）

### §2.1 目标文件结构

```
src/crates/assembly/core/src/agentic/tools/implementations/
├── (task_tool.rs → 删除, 替换为 task_tool/ subdir)
└── task_tool/
    ├── mod.rs                       NEW ~50-100 (sub-facade: `pub use task_tool::*` + `pub use task_tool_deep_review::*` etc.)
    ├── task_tool.rs                 NEW ~600-800 (TaskTool struct + impl + Tool trait impl + tool_core fns + misc)
    ├── task_tool_deep_review.rs     NEW ~800-1000 (37 deep_review_* fns)
    ├── task_tool_subagent.rs        NEW ~250-350 (10 subagent_* fns)
    ├── task_tool_agents.rs          NEW ~150-200 (6 agent_* fns)
    └── task_tool_input.rs           NEW ~150-200 (5 input validation fns)
```

### §2.2 目标行数

| File | 目标 | spec cap | 备注 |
|---|---|---|---|
| mod.rs (facade) | ~50-100 | 200 | `pub mod` 5 + `pub use task_tool::*` re-export |
| task_tool.rs (facade) | ~600-800 | 800 | TaskTool struct + impl + Tool trait impl + tool_core + 35 misc fns |
| task_tool_deep_review.rs | ~800-1000 | 1000 | 37 deep_review fns + 2-3 helpers |
| task_tool_subagent.rs | ~250-350 | 800 | 10 subagent fns |
| task_tool_agents.rs | ~150-200 | 800 | 6 agent fns |
| task_tool_input.rs | ~150-200 | 800 | 5 input fns |
| **TOTAL** | ~2000-2650 | — | 3085 + ~150 (import 重复) |
| **delta vs original** | -435 | — | 略小（extract 后能 fold 重复 import） |

**D-deviation**: `task_tool_deep_review.rs` 目标 800-1000, QClaw tolerance 810。超 1000 需 R12b 二次拆。

### §2.3 mod.rs (sub-facade) 设计

```rust
//! Task tool implementations (Round 12 split)
//!
//! Round 12 split: TaskTool + impl + 4 sub-handler siblings per fn domain.
//! - task_tool (this file): TaskTool struct + Tool trait impl + tool_core fns
//! - task_tool_deep_review (~1000): 37 deep_review fns
//! - task_tool_subagent (~350): 10 subagent fns
//! - task_tool_agents (~200): 6 agent fns
//! - task_tool_input (~200): 5 input validation fns

pub mod task_tool;
pub mod task_tool_deep_review;
pub mod task_tool_subagent;
pub mod task_tool_agents;
pub mod task_tool_input;

// Re-export public API
pub use task_tool::TaskTool;
```

**Caller 兼容**: 原 caller 用 `crate::agentic::tools::implementations::task_tool::TaskTool`. 拆后:
- `task_tool::TaskTool` (old path) 失效, 因为 `task_tool` 现在是 subdir
- 需要 caller 改用 `task_tool::task_tool::TaskTool` (新 path) 或保持 `task_tool::TaskTool` 如果 mod.rs 有 `pub use task_tool::TaskTool`

按 R11 pattern (services-integrations 拆 remote_connect/), 旧路径会被破坏, 需要 cross-crate caller update. Mavis take-over 时 fix 任何 caller.

### §2.4 task_tool.rs (facade) 内容

```rust
//! TaskTool struct + Tool trait impl (Round 12 split facade)
//!
//! Owns TaskTool struct + impl TaskTool + impl Tool for TaskTool + tool_core fns.
//! Sub-domain split: deep_review/subagent/agents/input validation moved to siblings.

use crate::agentic::tools::framework::{Tool, ...};
// ... imports (精确, 不复制 manager use block per R10a lesson)

pub struct TaskTool;

const LARGE_TASK_PROMPT_SOFT_LINE_LIMIT: usize = 180;
const LARGE_TASK_PROMPT_SOFT_BYTE_LIMIT: usize = 16 * 1024;

impl Default for TaskTool {
    fn default() -> Self { Self::new() }
}

impl TaskTool {
    pub fn new() -> Self { Self }

    // 5 tool_core fns (call_impl 是 god method, ~200-400 行)
    pub fn load_configured_tool_execution_timeout(...) -> ... { ... }
    pub fn render_tool_use_message(...) -> ... { ... }
    pub async fn call_impl(...) -> Result<Vec<ToolResult>> { ... }  // GOD METHOD
    pub fn default_tools(...) -> ... { ... }
    pub fn task_tool_default_exposure_is_collapsed(...) -> ... { ... }
}

// 35 misc fns (Tool trait impl + helpers)
// as_any / id / name / description / prompt_template_name / ...
```

`call_impl` 是核心 god method, 估计 200-400 行。 Spec 要求 worker 把它 split 为 sub-fns:
```rust
pub async fn call_impl(...) -> Result<Vec<ToolResult>> {
    validate_input(...)?;
    if is_deep_review_task(...) {
        return deep_review::launch_batch_for_task(...).await;
    }
    if is_subagent_task(...) {
        return subagent::dispatch(...).await;
    }
    // ... actual logic
}
```

### §2.5 task_tool_deep_review.rs 内容

37 deep_review_* fns (清单见 §1.1).

This file 是大头 (37 fns). Spec 要求 worker 按 sub-domain 拆, 不强求本 round 内拆分, 但 spec 标 D-deviation risk:
- 37 fns 平均 ~25-30 行 each = ~925-1100 lines
- 超 1000 line cap → 可能需要 R12b 二次拆

如果 worker 报告 > 1000 lines, 立即考虑把 deep_review 拆为 2 sub:
- `task_tool_deep_review_policy.rs` (deep_review_policy_*, deep_review_retry_*, deep_review_auto_retry_*, deep_review_capacity_*, deep_review_provider_capacity_*)
- `task_tool_deep_review_queue.rs` (deep_review_capacity_queue_*, deep_review_concurrency_policy_*, deep_review_local_capacity_*, deep_review_cancelled_reviewer_*)

### §2.6 task_tool_subagent.rs 内容

10 subagent_* fns (清单见 §1.1). 估计 ~300 行.

### §2.7 task_tool_agents.rs 内容

6 agent_* fns. 估计 ~200 行.

### §2.8 task_tool_input.rs 内容

5 input_validation fns. 估计 ~200 行.

### §2.9 lib.rs / tools.rs / mod.rs 改动

检查 `src/crates/assembly/core/src/agentic/tools/mod.rs` + `implementations/mod.rs`:
- `pub mod task_tool;` 已经存在 (指向 file)
- 拆后需要 `pub mod task_tool;` 自动指向 subdir + mod.rs
- 或加 `pub use task_tool::TaskTool;` 在 implementations/mod.rs 顶层 (保留 old path)

---

## §3 验证策略

### §3.1 编译验证

```bash
# 1. baseline 重现 (preflight)
cd E:\agent-project\northing
git log -1 --oneline  # 确认 aba2a98 (R11b handoff)
cargo check -p northhing-core --features product-full --lib  # 期望 0 errors

# 2. 改完后
cargo check -p northhing-core --features product-full --lib  # 期望 0 errors
cargo build --tests -p northhing-core --features product-full  # 期望 0 errors
cargo check --workspace  # 期望 0 errors (cross-crate caller 不受影响)
```

### §3.2 测试验证

```bash
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (与 main HEAD baseline 一致)
```

### §3.3 cross-crate caller check

```bash
# 确保 caller 仍能 import TaskTool
git grep -l 'task_tool::TaskTool\|task_tool::.*::TaskTool' | wc -l
# baseline + after 必须相等

# find any new callers broken
cargo build --workspace --message-format=short 2>&1 | grep "could not find\|unresolved import"
```

---

## §4 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| task_tool_deep_review.rs 1000 cap | ≤ 1000 | ~800-1100 | 37 fns 估计偏高, 超 1000 需 R12b 二次拆 |
| 其他 4 sibling | ≤ 800 | OK | 充足 |
| task_tool.rs facade 800 cap | ≤ 800 | ~700 | 5 tool_core + 35 misc + 3 impl blocks |

如果 `task_tool_deep_review.rs` 超 1000, R12b 必做 (2 sub split).

---

## §5 实施步骤 (R12 经验: 按 fn 数从小到大 + 每步报告行数)

1. **task_tool_input.rs** (~200, smallest) → cargo check + **报告行数**
2. **task_tool_agents.rs** (~200) → cargo check + **报告行数**
3. **task_tool_subagent.rs** (~350) → cargo check + **报告行数**
4. **task_tool_deep_review.rs** (~1000, biggest) → cargo check + cargo test + **报告行数**
5. **task_tool.rs** (facade, ~700) → cargo check + cargo test
6. **mod.rs** (sub-facade) → cargo check
7. **删除原 task_tool.rs** → cargo check + cargo test (final verification)

**每步必须**: cargo check 0 errors + **报告当前 sibling 行数** (超 1000 cap 立即考虑二次拆)

### Critical: cargo check stop-at-first-error prevention

```bash
cargo check -p services-integrations --features product-full --lib --message-format=short 2>&1 | Tee-Object upstream-check.log
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object -Append upstream-check.log
cargo check -p northhing-tools-execution --features product-full --lib --message-format=short 2>&1 | Tee-Object -Append upstream-check.log
cargo check -p northhing-tool-provider-groups --features product-full --lib --message-format=short 2>&1 | Tee-Object -Append upstream-check.log
```

### Critical: Cargo.lock drift check

```bash
git show origin/main:Cargo.lock | Select-String "name = ""rmcp"""
Get-Content Cargo.lock | Select-String "name = ""rmcp"""
```

### Critical: 12-class sub-domain errors (R5/6/7/8/9b/10a/10b/11/11b lessons)

1. **Import paths**: 新 sibling 默认 `use super::TaskTool;` (facade 的 TaskTool struct) + 各自 imports
2. **Sibling method visibility**: methods stay `pub` (free functions + impl methods)
3. **Struct field visibility**: TaskTool struct 是空 struct, 无字段. 但 helper struct (如果有) 跨 sibling 共享字段 `pub(crate)`
4. **Cargo.lock drift**: see above
5. **mod.rs `pub mod`**: 5 new siblings MUST be declared
6. **Test attribute 丢失**: preserve `#[test]` / `#[tokio::test]`
7. **cargo check stop-at-first-error**: see above
8. **跨 sibling 共享 enum/trait**: TaskTool struct 留在 facade. Tool trait impl 留 facade. 无共享 enum/trait 需要 mod.rs
9. **R10a 1130 unused imports 教训**: 每个 sibling use 精确 use 块
10. **R11a struct owner mapping**: spec §1.1-§1.3 已显式 fn 归属
11. **R11 worker 每步报告行数**: spec §5 已强制
12. **R11b cross-reference paths**: 共享 type (TaskTool) 留 facade, sibling 用 `use super::TaskTool;` 而不是 `use super::task_tool::TaskTool;`

---

## §6 Verification

```bash
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (R11b baseline 一致)

cargo fmt --check -p northhing-core

# 5 个新 sibling 行数检查 (each ≤ 800; deep_review ≤ 1000)
for sibling in task_tool_deep_review task_tool_subagent task_tool_agents task_tool_input task_tool; do
  py -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing-impl-round12\src\crates\assembly\core\src\agentic\tools\implementations\task_tool/${sibling}.rs', encoding='utf-8')))"
done

# 0 fns dropped (98 → 98)
py -c "
import re
from pathlib import Path
wt_dir = Path(r'E:\agent-project\northing-impl-round12\src\crates\assembly\core\src\agentic\tools\implementations\task_tool')
fns = set()
for f in wt_dir.glob('*.rs'):
    fns.update(re.findall(r'^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'fn count: {len(fns)}')
print('expected: 98')
"

# Iron rules — 0 NEW (pre-existing moved, not new)
git diff origin/main..HEAD -- src/crates/assembly/core/src/agentic/tools/implementations/task_tool/ | grep '^+.*unwrap()|^+.*panic!|^+.*unreachable!'
# 期望: 0
```

---

## §7 11-class sub-domain errors (12-class reinforced from R11b)

每个新 sibling 用统一 pattern:

```rust
//! task_tool_{domain} (Round 12 split)
//!
//! Owns {domain} fns extracted from original task_tool.rs.
//! Sub-domain split per spec §1.1 / §5.

use super::TaskTool;  // 共享 struct from facade
use crate::agentic::tools::framework::{Tool, ToolExposure, ToolResult, ...};
// ... 各自需要的 imports (精确, 不复制 use block per R10a lesson)

// fn / struct / enum / trait / impl blocks
```

---

## §8 spec review check-list

QClaw 重点检查:
1. 5 文件拆分结构 (按 fn domain, 跟 R11 sub-domain split 一致)
2. `call_impl` god method split (worker 必须把它拆为 sub-fns, 类似 R7 turn_internal 4 sub-handlers pattern)
3. `task_tool_deep_review.rs` 行数 (37 fns, 可能超 1000)
4. cross-crate caller 不受影响 (`task_tool::TaskTool` 仍可访问 via mod.rs pub use)
5. 0 fns dropped (98 → 98)
6. pre-existing unwrap preserved (R11b pattern: 不"修复" pre-existing)
7. worker 报告行数 (R11 lesson: 每步 cargo check 后报告)

---

## §9 Errata

- §2.5 `task_tool_deep_review.rs` 1000 cap 上限是 reviewer-tolerance, 不是硬 cap. QClaw 100% 接受的话不需要 R12b 二次拆.
- §5 拆文件顺序按 fn 数从小到大, 降低风险
- §2.4 `call_impl` god method 必须 split (类似 R7 turn_internal pattern), 不接受整段搬运
- R12 是 critical #2 (R11 是 critical #1), 跟 R11 同样 pattern + R11a/b lessons 应用