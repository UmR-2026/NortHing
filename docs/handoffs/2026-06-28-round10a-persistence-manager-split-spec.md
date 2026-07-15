# Round 10a Spec: persistence/manager.rs 3650 行拆分（sub-domain split）

> **目标**: `persistence/manager.rs` 3650 行 1 个 `impl PersistenceManager` 拆成 facade + 6 sibling
> **Pattern**: sub-domain split (与 Round 5 chat.rs / Round 6 dialog_turn.rs / Round 9 session_manager 一致)
> **Draft**: Mavis 2026-06-28 21:30 (Round 9b 刚合 main 后立即衔接)

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/assembly/core/src/agentic/persistence/manager.rs` | wc -l |
| 行数 | **3650** | ReadAllLines.Count |
| impl 块数 | 1 | grep |
| fn 定义数 | 120 (方法 + helper) | regex `fn \w+\(` |
| `mod.rs` 行数 | 12 (已存在) | wc -l |
| `session_branch.rs` 行数 | 471 (已存在 — Round 3 部分拆) | wc -l |
| `mod.rs` 公共 API | `pub mod manager; pub mod session_branch;` + `pub use manager::PersistenceManager;` + 3 个 re-exports | head -15 |

### 1.1 fn 分布 (按 doc-comment 关键词归类)

| Domain | fn 数 | pub | async | 候选 sibling |
|---|---|---|---|---|
| transcript | 27 | 1 | 2 | `transcript_subhandlers.rs` |
| turn | 25 | 12 | 22 | `turn_subhandlers.rs` |
| misc (paths/utilities) | 21 | 6 | 5 | `paths_utilities.rs` |
| session | 16 | 6 | 12 | `session_subhandlers.rs` |
| skill_snapshot | 15 | 9 | 11 | `skill_snapshot_subhandlers.rs` |
| metadata | 13 | 5 | 10 | `metadata_subhandlers.rs` |
| message | 3 | 0 | 0 | (合并到 paths_utilities) |
| **TOTAL** | **120** | **39** | **62** | **6 sibling** |

注：test fns (line 2700+) 也统计在内；拆分后 test fns 仍跟 sibling 走。

### 1.2 与 Round 3b `session_branch.rs` 的关系

- `session_branch.rs` 471 行已经是 partial split (Round 3b 产物)
- 但 `mod.rs` 已有 `pub mod session_branch;` 声明 + 实际在用
- **不**重新拆 `session_branch.rs`，保留不动
- 10a 只拆 `manager.rs` 3650 → facade + 5 个新 sibling

### 1.3 Round 5/6/7/8/9 经验（避免重复）

| 错误类 | Round 命中 | Round 10a 防御 |
|---|---|---|
| Cargo.lock drift (rmcp 1.7→1.8) | R6 (32+ errors masked) | Plan YAML preflight: baseline commit 重现 `cargo check` |
| cargo check stop-at-first-error | R6 (32+ errors 被 2 E0308 掩盖) | worker 报"0 NEW errors"必须 **每个 crate 都跑过** + 在 baseline 重现 |
| M3 model 太慢 (39min silence) | R6 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Plan engine abort fail | R6 (50001) | 用 `mavis team plan cancel` 不依赖 `mavis session abort` |
| Sibling method private | R6 (16 fns) | bulk `pub(crate)` via Python script |
| Struct field private | R6 (WrappedUserInputPayload) | 跨 sub-handler 共享 struct 字段 `pub(crate)` |
| Import 路径 super::super vs super | R6 (4 处) | 新 sibling 默认 `use super::super::*` |
| 漏 test attribute | R9b (2 fns) | worker split 必须保留 `#[test]`/`#[tokio::test]` attribute |
| mod.rs 漏 `pub mod` | R3b (orphan files) | 每个新 sibling 必须在 mod.rs 加 `pub mod` |
| 测 0 lines 漂移 (Measure-Object vs wc -l) | R6 audit | 用 `[System.IO.File]::ReadAllLines().Count` (= wc -l) |

---

## §2 拆分方案（sub-domain split）

### §2.1 目标文件结构

```
agentic/persistence/
├── mod.rs                       # 现有 12 行（加 5 个新 sibling declaration）
├── manager.rs                   # facade: 3650 → ~150-200 行
├── session_branch.rs            # 保留 471 行（Round 3b 产物，不动）
├── session_subhandlers.rs       # 新增: 16 fns (session IO)
├── turn_subhandlers.rs          # 新增: 25 fns (turn IO)
├── transcript_subhandlers.rs    # 新增: 27 fns (transcript 渲染/解析)
├── metadata_subhandlers.rs      # 新增: 13 fns (metadata list/save/load)
├── skill_snapshot_subhandlers.rs# 新增: 15 fns (skill snapshot)
└── paths_utilities.rs           # 新增: 24 fns (path helpers + message sanitize)
```

### §2.2 目标行数

| 文件 | 目标行数 | spec cap | 备注 |
|---|---|---|---|
| manager.rs (facade) | **~150-200** | 200 (per R9 D6) | struct + 公共方法 dispatch + 几个 public method 透传 |
| session_subhandlers.rs | ~450-500 | 800 | 16 fns + impl block |
| turn_subhandlers.rs | ~700-750 | 800 | 25 fns (最大 sub-domain) |
| transcript_subhandlers.rs | ~600-650 | 800 | 27 fns |
| metadata_subhandlers.rs | ~350-400 | 800 | 13 fns |
| skill_snapshot_subhandlers.rs | ~400-450 | 800 | 15 fns |
| paths_utilities.rs | ~400-450 | 800 | 24 fns (path + message) |
| session_branch.rs (不动) | 471 | — | Round 3b 产物 |
| **TOTAL** | ~3650 + 50 (import/use 重复) | — | 略 +50 可接受 |

### §2.3 facade 实现策略 (Rust multi-impl pattern)

**关键洞察**: Rust 允许同一类型在多个 file 中各自有 `impl` 块。所以**不需要 facade wrapper** — 每个 sibling file 直接定义 `impl PersistenceManager { pub async fn xxx(...) }`，调用方无感（call site 用 `PersistenceManager::save_session()` 还是 `manager.save_session()` 都行）。

参考 `session_branch.rs:8-12` 已经采用此 pattern:
```rust
impl PersistenceManager {
    pub async fn branch_session(...) -> ... { ... }
}
```

**manager.rs (facade) 只保留**:
```rust
//! Persistence Manager
//!
//! Round 10a split: 6 sub-domain siblings (session/turn/transcript/
//! metadata/skill_snapshot/paths_utilities) own the impl blocks.
//! This file keeps the struct + constructors + cross-cutting helpers.

#[derive(Debug, Clone)]
pub struct PersistenceManager { ... }

impl PersistenceManager {
    pub fn new(...) -> Self { ... }
    pub fn path_manager(&self) -> &PathManager { ... }
    pub fn runtime_service(&self) -> ... { ... }
}
```

**每个 sibling file**:
```rust
//! persistence/{domain} sub-handlers (Round 10a)

use super::manager::PersistenceManager;
// 各自需要的 import

impl PersistenceManager {
    pub async fn save_session(...) -> NortHingResult<()> { ... }
    pub async fn load_session(...) -> NortHingResult<Session> { ... }
    // ... 其他 fns, 直接 pub
}
```

**所有 pub 方法 visibility 保持原样** — 不需要改 `pub` → `pub(crate)`，因为 `impl` 块分散在不同 file 不影响方法可见性。

### §2.4 sibling file 内部细节

每个 sibling file 模板:

```rust
//! persistence/{domain} sub-handlers
//!
//! Round 10a split: {domain} methods moved from manager.rs (1:1 sub-domain).
//! Uses Rust multi-impl pattern: this file owns PersistenceManager's
//! {domain}-related methods. No facade wrapper needed.

use super::manager::PersistenceManager;
use crate::util::errors::{NortHingError, NortHingResult};
// 各自需要的 import (per §1.1 domain 划分)

impl PersistenceManager {
    // {domain} public methods (visibility 保持原样)
    pub async fn save_session(...) -> ... { ... }
    pub async fn load_session(...) -> ... { ... }
    // ... 其他 fns

    // {domain} private helpers (file-local, 各自 file 内可见)
    fn helper_xxx(...) -> ... { ... }
}
```

注意:
- `impl PersistenceManager` 在 sibling file 中完全合法
- pub 方法不需要 wrapper — Rust 自动 link 同一 type 的所有 impl block
- private helper fns 保持 `fn` (无 pub) — 各自 file 内可见，跨 file 不可见
- 如某 helper 跨 sibling 需要共享，提到 manager.rs 顶层 OR 改 `pub(super)`

### §2.5 公共 API 列表 (按 sub-domain)

```rust
// session_subhandlers.rs (16 fns, 6 pub)
pub async fn save_session, load_session, save_session_state, delete_session, list_sessions, touch_session
+ 10 个 private helpers (ensure_session_dir, build_session_from_persisted_parts, ...)

// turn_subhandlers.rs (25 fns, 12 pub)
pub async fn load_session_with_turns, load_session_with_turns_timed,
    load_session_with_tail_turns, load_session_with_tail_turns_timed,
    save_dialog_turn, load_dialog_turn, load_session_turns, load_session_tail_turns,
    delete_dialog_turns_from, load_recent_turns, delete_turns_after, delete_turns_from
+ 13 个 private helpers + test fns

// transcript_subhandlers.rs (27 fns, 1 pub)
pub async fn export_session_transcript
+ 24 个 private helpers (transcript_path, transcript_preview, ...) + 3 test fns

// metadata_subhandlers.rs (13 fns, 5 pub)
pub async fn list_session_metadata, list_session_metadata_page,
    list_session_metadata_including_internal, save_session_metadata, load_session_metadata
+ 5 个 private helpers + 3 test fns

// skill_snapshot_subhandlers.rs (15 fns, 9 pub)
pub async fn save_turn_context_snapshot, load_turn_context_snapshot,
    load_latest_turn_context_snapshot, save_turn_skill_agent_snapshot,
    load_turn_skill_agent_snapshot, delete_turn_skill_agent_snapshots_from,
    save_skill_agent_baseline_override_snapshot, load_skill_agent_baseline_override_snapshot,
    delete_turn_context_snapshots_from
+ 4 个 private helpers + 2 test fns

// paths_utilities.rs (24 fns, 6 pub)
pub fn new, path_manager, runtime_service, load_prompt_cache, save_prompt_cache, delete_prompt_cache
+ 14 个 private helpers (turns_dir, state_path, project_sessions_dir, ...) + 4 test fns
```

(注: §1.1 "misc 21 fns" + "message 3 fns" 合并到 paths_utilities.rs = 24 fns)

### §2.5 mod.rs 改动

```rust
// 当前 12 行
pub mod manager;
pub mod session_branch;

pub use manager::PersistenceManager;
pub use norththing_runtime_ports::SessionTurnLoadTiming;
pub use norththing_services_core::session::{
    SessionBranchRequest, SessionBranchResult, SessionMetadataPage,
};

// 加 5 行:
pub mod session_subhandlers;
pub mod turn_subhandlers;
pub mod transcript_subhandlers;
pub mod metadata_subhandlers;
pub mod skill_snapshot_subhandlers;
pub mod paths_utilities;
```

### §2.6 为什么不用 facade wrapper

**备选方案 (rejected)**: 每个 sibling 方法 `pub(crate)` + facade 留 `pub` wrapper 调 `self.suffix_method()`。

**reject 理由**:
1. **重复代码 30+ 个 wrapper** (每行 3-5 行 × 30 方法 = 90-150 行 boilerplate)
2. **call site 完全无感** — Rust 多 impl 块模式下，调用方 `manager.save_session()` 等价，无论 save_session 定义在哪个 file
3. **session_branch.rs 已经验证** — Round 3b 用 `impl PersistenceManager { pub async fn branch_session }` 模式，2 年来无任何 call site 改动需求
4. **visibility 风险低** — sibling 方法是 `pub`，不暴露内部状态；`PersistenceManager` 本身已经 `pub`

---

## §3 验证策略

### §3.1 编译验证

```bash
# 1. baseline 重现 (preflight)
cd E:\agent-project\northing
git log -1 --oneline  # 确认 5e30916 (Round 9b merge)
cargo check -p norththing-core --features product-full  # 期望 0 errors (current main HEAD)

# 2. 改完后
cargo check -p norththing-core --features product-full  # 期望 0 errors, 0 NEW warnings
cargo build --tests -p norththing-core --features product-full  # 期望 0 errors
```

### §3.2 测试验证

```bash
cargo test -p norththing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (与 main HEAD baseline 一致)

# 拆分不丢测试: 120 fn 中 27 个是 test fns，全部保留
```

### §3.3 自定义 verifier

复用 R5 sub-domain-verifier.py pattern:
- 遍历 `manager.rs + 5 新 sibling + session_branch.rs`
- 断言：所有原 120 fn 名都在新结构中
- 断言：`mod.rs` 有 5 个新 `pub mod` 声明
- 断言：manager.rs 总行数 ≤ 200
- 断言：每个 sibling ≤ 800 行
- 断言：120 fn 中 27 个 test fn 都带 `#[test]` 或 `#[tokio::test]` attribute

---

## §4 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| turn_subhandlers.rs 800 cap | 上限 810 | ~700-750 | 25 fns 中 22 async，估计偏高但 < 800 |
| transcript_subhandlers.rs 800 cap | 上限 810 | ~600-650 | 27 fns 大部分是 helper，估计 < 700 |
| facade 200 cap | 上限 210 | ~150-200 | 公共方法 ~30 个 wrapper，每个 1-3 行 |
| 6 个新 sibling 同时引入 | QClaw review 接受 | 5 个新 + 1 个保留 | 与 R6/R9 同 pattern |

**如果某个 sibling 超 800**，需 R10b 二次拆（类比 R9b 二次拆 lifecycle 930 / metadata 1010）。

---

## §5 实施步骤 (Mavis take-over ready)

### Phase 1: 准备
1. 创建 worktree `northing-impl-round10a`
2. 跑 baseline `cargo check` 记录干净状态
3. 写 helper Python 脚本（`promote-sibling-visibility.py`, `extract-manager-fns.py`）

### Phase 2: 拆文件 (atomic per sibling)
1. 拆 session_subhandlers.rs (16 fns)
2. cargo check → 0 errors
3. 拆 turn_subhandlers.rs (25 fns)
4. cargo check → 0 errors
5. 拆 transcript_subhandlers.rs (27 fns)
6. cargo check → 0 errors
7. 拆 metadata_subhandlers.rs (13 fns)
8. cargo check → 0 errors
9. 拆 skill_snapshot_subhandlers.rs (15 fns)
10. cargo check → 0 errors
11. 拆 paths_utilities.rs (24 fns)
12. cargo check → 0 errors, cargo test 899/0/1

### Phase 3: 验证
1. cargo fmt --check clean
2. cargo clippy -- -D warnings 0 errors
3. subdomain-verifier.py PASS
4. cargo test --lib 899/0/1
5. diff stat: ~3650 → facade 200 + 6 sibling ~3500 + 50 (import 重复)

### Phase 4: commit + merge
1. 1 atomic commit per Round 5/6/7/8 D6 precedent
2. 写 handoff doc `docs/handoffs/2026-06-28-round10a-persistence-manager-split-impl.md`
3. merge to main
4. (可选) Mavis take-over 自己 review-fix-cleanup

---

## §6 spec review check-list

请 reviewer (QClaw) 重点检查:

1. **5 个新 sibling 划分合理**: session/turn/transcript/metadata/skill_snapshot/paths_utilities — 是否需要更细分或合并
2. **facade 200 cap 是否合理**: 30 个公共 method × 1-3 行 wrapper ≈ 60-100 行，加 struct + import ≈ 150 行
3. **session_branch.rs 471 行是否需要二次拆**: Round 3b 产物，超 800 cap 否但接近 500
4. **D-deviation 4 项**是否可接受
5. **测试保留**: 27 test fns 全部带 attribute 保留

---

## §7 Errata

- §2.2 目标行数是 estimate，实际由 cargo fmt 后可能 ±50 行
- §3.1 preflight 步骤依赖 main HEAD = 5e30916 (Round 9b merge)
- §5 Phase 2 拆文件顺序按 fn 数从小到大 (降低风险)
- §4 D-deviation 与 R9b lifecycle 930 / metadata 1010 同 reviewer-tolerance
