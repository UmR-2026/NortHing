# Round 11b Spec — `remote_command_handlers` + `remote_session_tracker` 二次拆

> **目标**: 拆 R11 D-deviation 2 个超 cap sibling 为 5 个新 sub-handler files, 全部 ≤ 800 cap
> **Trigger**: QClaw R11 review COND APPROVE 6.8/10 + R11b REQUIRED (D1+D2 项目历史第 2/3 严重度)
> **Spec source**: QClaw review report §R11b Specification (5-file 方案)

---

## §1 当前状态 (R11 后, R11b 前)

| File | Lines | Cap | Over |
|---|---|---|---|
| mod.rs | 81 | 200 | OK ✅ |
| remote_request_builders.rs | 638 | 800 | OK ✅ |
| **remote_session_tracker.rs** | **1272** | 800 | **+472 ❌ D2** |
| **remote_command_handlers.rs** | **1301** | 800 | **+501 ❌ D1** |
| remote_file_io.rs | 176 | 800 | OK ✅ |
| remote_workspace_resolver.rs | 102 | 800 | OK ✅ |
| 5 preserved | preserved | — | OK ✅ |

Test baseline: 899/0/1 (R11 maintained)
Pre-existing iron rules: 26 unwrap + 9 let _ = (all moved from original, Δ=0 vs R11a)

## §2 拆分方案 (QClaw 5-file 钦定)

### `remote_command_handlers.rs` 1301 → 3 sub-handlers

| New file | Target | Scope |
|---|---|---|
| `remote_dialog_handlers.rs` | ~500 | `handle_remote_dialog_*` + `RemoteDialog*` 5 types (QueuePriority/SubmissionPolicy/SubmissionRequest/ResolvedSubmission/SubmitOutcome/SchedulerOutcomeFact/RuntimeHost) + `remote_dialog_*` 2 fns |
| `remote_session_handlers.rs` | ~400 | `handle_remote_session_command` 等 session command handlers |
| `remote_cancel_handlers.rs` | ~400 | `RemoteCancel*` 3 types (Decision/TaskRequest/RuntimeHost) + `cancel_remote_task` |

### `remote_session_tracker.rs` 1272 → 2 sub-handlers

| New file | Target | Scope |
|---|---|---|
| `remote_session_state.rs` | ~700 | `RemoteSessionTracker` struct + impl + RwLock state 管理 + 30+ accessor methods (含 pre-existing 24 unwrap) |
| `remote_session_response_builders.rs` | ~500 | `remote_session_info/list/created/deleted/model_updated` response DTO builders |

### 最终 R11b 文件结构

```
remote_connect/
├── mod.rs                                81   facade (unchanged)
├── device.rs                             74   preserved
├── encryption.rs                         189   preserved
├── pairing.rs                            282   preserved
├── qr_generator.rs                       82   preserved
├── relay_client.rs                       511   preserved
├── remote_request_builders.rs            638   preserved (R11a)
├── remote_file_io.rs                     176   preserved (R11a)
├── remote_workspace_resolver.rs          102   preserved (R11a)
├── remote_session_state.rs               NEW ~700 (R11b)
├── remote_session_response_builders.rs   NEW ~500 (R11b)
├── remote_dialog_handlers.rs             NEW ~500 (R11b)
├── remote_session_handlers.rs            NEW ~400 (R11b)
└── remote_cancel_handlers.rs             NEW ~400 (R11b)
```

Total: 14 files (was 11), all ≤ 800 cap ✅

## §3 R11b 关键约束

- **0 fns dropped**: 59 → 59 preserved
- **每个新 sibling ≤ 800 行** (QClaw tolerance 810)
- **Public API 不变**: mod.rs `pub use` 重导出覆盖所有新 sibling (跟 R11 一致)
- **Pre-existing iron rules 不变**: 26 unwrap + 9 let _ = all moved to `remote_session_state.rs` (struct 内部 state management), 不新增
- **cargo test 899/0/1 maintained**
- **cargo fmt clean**
- **0 unwrap/panic/unreachable 新增** (QClaw 验证: Δ = 0)

## §4 multi-impl pattern (跟 R10a 一致)

`RemoteSessionTracker` struct 在 `remote_session_state.rs` 定义, impl 块可以跨 file (Rust multi-impl). 但因为没有 god struct (跟 R10a PersistenceManager 不同), impl 块分散到各 sibling 即可.

每个 new sibling 用同样 pattern:
```rust
//! remote-connect {domain} sub-handler (Round 11b split)

use super::RemoteConnectSubmissionSource;  // 共享 from mod.rs
// ... 各自需要的 imports

// fn / struct / enum / trait / impl blocks
```

## §5 关键: struct owner mapping (R11a worker 犯的错)

R11a worker 按 fn prefix 拆但忽略 struct owner, 导致 11 types 都塞到 command_handlers。R11b 必须显式按 **struct owner + fn domain** 拆:

| Struct/Enum/Trait | Owner sibling |
|---|---|
| RemoteDialogQueuePriority | remote_dialog_handlers |
| RemoteDialogSubmissionPolicy | remote_dialog_handlers |
| RemoteDialogSubmissionRequest | remote_dialog_handlers |
| RemoteDialogResolvedSubmission | remote_dialog_handlers |
| RemoteDialogSubmitOutcome | remote_dialog_handlers |
| RemoteDialogSchedulerOutcomeFact | remote_dialog_handlers |
| RemoteDialogRuntimeHost | remote_dialog_handlers |
| RemoteCancelDecision | remote_cancel_handlers |
| RemoteCancelTaskRequest | remote_cancel_handlers |
| RemoteCancelRuntimeHost | remote_cancel_handlers |
| RemoteTerminalPrewarmRequest | remote_dialog_handlers (related to dialog submit) |
| RemoteSessionTracker | remote_session_state |
| RemoteSessionInfo/...等 DTOs | remote_session_response_builders |

## §6 实施步骤 (R11a 经验: 按 fn 数从小到大 + 每步报告行数)

1. **remote_cancel_handlers.rs** (~400, smallest + struct 集中) → cargo check, **报告行数**
2. **remote_session_handlers.rs** (~400) → cargo check, **报告行数**
3. **remote_dialog_handlers.rs** (~500) → cargo check, **报告行数**
4. **remote_session_response_builders.rs** (~500) → cargo check, **报告行数**
5. **remote_session_state.rs** (~700, biggest struct impl) → cargo check + cargo test, **报告行数**
6. **删除原 remote_session_tracker.rs** 和 **remote_command_handlers.rs** → cargo check + cargo test

**每步必须**: cargo check 0 errors + **报告当前 sibling 行数** (超 800 立即调整)

## §7 Verification (跟 R11 一致 + 行数报告)

```bash
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (R11 maintained)

cargo fmt --check -p northhing-services-integrations

# 5 个新 sibling 行数检查
for sibling in remote_cancel_handlers remote_session_handlers remote_dialog_handlers remote_session_response_builders remote_session_state; do
  wc=$(python -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing-impl-round11b\src\crates\services\services-integrations\src\remote_connect/${sibling}.rs', encoding='utf-8')))")
  echo "$sibling: $wc lines"
done
# 每个 must ≤ 800

# 0 fns dropped (59 → 59)
python -c "
import re, subprocess
from pathlib import Path
wt = Path(r'E:\agent-project\northing-impl-round11b\src\crates\services\services-integrations\src\remote_connect')
fns = set()
for f in wt.glob('*.rs'):
    fns.update(re.findall(r'^(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'fn count: {len(fns)}')
print('expected: 59')
"

# Iron rules — 0 NEW violations (26 unwrap + 9 let _ = all moved to remote_session_state.rs)
git diff origin/main..HEAD -- src/crates/services/services-integrations/src/remote_connect/ | grep '^+.*unwrap()\|^+.*panic!\|^+.*unreachable!'
# 期望: 0
```

## §8 D-deviation 风险

| Item | 计划 | 实际预期 | 备注 |
|---|---|---|---|
| remote_session_state.rs 700 cap | ≤ 800 | ~700 | RemoteSessionTracker struct + 30 accessors 偏大但 < 800 |
| 其他 4 个 sibling | ≤ 800 | ~400-500 | 充足 |

如果 `remote_session_state.rs` 超 800，需 R11c 三次拆 (按需)。

## §9 11-class sub-domain errors (R5/6/7/8/9b/10a/10b 经验强化)

1. **Import paths**: 新 sibling 默认 `use super::RemoteConnectSubmissionSource;` (mod.rs 共享类型)
2. **Struct field visibility**: 跨 sibling 共享的 struct 字段 `pub(crate)` (但 R11 没有跨 sibling struct 共享, 主要在 RemoteSessionTracker 内)
3. **Cargo.lock drift**: Plan YAML preflight baseline cargo check
4. **mod.rs `pub mod`**: 5 new siblings MUST be declared
5. **Test attribute 丢失**: preserve `#[test]` / `#[tokio::test]`
6. **cargo check stop-at-first-error**: 必须每个上游 crate 都跑过
7. **跨 sibling 共享 enum/trait**: 共享类型 (RemoteConnectSubmissionSource) 留在 mod.rs
8. **RwLock unwrap (24 pre-existing)**: 都搬到 `remote_session_state.rs`, 不"修复" (pre-existing debt)
9. **R10a 1130 unused imports 教训**: 每个 sibling use 精确 use 块
10. **spec 应列出 struct owner → sibling mapping**: §5 已显式 mapping
11. **worker 应在每步 cargo check 后报告行数**: §6 已加

## §10 spec review check-list

QClaw review (81b520e) R11b 草案为本 spec source. 本 spec 与 QClaw 草案一致. 无新方案.

QClaw 重点检查:
1. 5 个新 sibling 划分合理 (按 struct owner + fn domain, 不是只看 fn prefix)
2. 0 fns dropped (59 → 59)
3. Pre-existing 26 unwrap + 9 let _ = all in `remote_session_state.rs`, 不"修复"
4. cargo test 899/0/1 maintained
5. cargo fmt clean
6. mod.rs `pub use` 覆盖所有 5 个新 sibling