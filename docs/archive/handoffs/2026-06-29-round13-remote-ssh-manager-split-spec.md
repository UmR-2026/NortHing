# Round 13 Spec: remote_ssh/manager.rs 2810 → 1 facade + 3 sub-handlers

> **目标**: critical #3 god object (`remote_ssh/manager.rs` 2810 lines)
> **Pattern**: R11b/R12/R12b sub-domain split (facade + 3 sibling handlers)
> **Trigger**: 12 轮 god object decomposition 连续进度

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/services/services-integrations/src/remote_ssh/manager.rs` | wc -l |
| 行数 | **2810** | ReadAllLines.Count |
| impl 块 | 11 (含 4 struct + 2 enum + 5 impl + 2 impl trait) | grep |
| 总 fn 数 | **102** | loose regex |
| mod.rs 当前 | 4 pub mod (paths/types/workspace_registry/manager) + 4 cfg-gated (manager/password_vault/remote_exec/remote_fs/remote_terminal) + 5 cfg-gated re-exports | mod.rs |

### 1.1 fn domain 分布 (按 prefix)

| Domain | fn 数 | 内容 |
|---|---|---|
| connection_* | 24 | SSH 连接管理主体 (largest) |
| get_* | 7 | various getters |
| exec_* | 6 | command execution |
| port_forward_* | 5 | port forwarding |
| with_* | 4 | builder helpers |
| config_* | 4 | config loading |
| new | 3 | constructors |
| load | 3 | lazy loaders |
| sftp_mkdir | 3 | SFTP mkdir helpers |
| ssh_cfg | 2 | ssh_config helpers |
| save | 2 | save fns |
| remove | 2 | remove fns |
| set | 2 | setters |
| resolve | 2 | resolvers |
| sftp_read | 2 | SFTP read |
| mkdir | 2 | mkdir |
| (other) | 44 | single fns, handler callbacks, etc. |

### 1.2 struct / impl mapping (precise owner per spec §R11a lesson)

| Struct / Enum / Impl | Line | Owner sibling | Notes |
|---|---|---|---|
| `KnownHostEntry` | 64 | facade (manager.rs) | 公共 API, used by callers |
| `ActiveConnection` | 73 | facade (private) | used by SSHConnectionManager |
| `SSHHandler` | 91 | **manager_handler.rs** | Russh callback handler (private) |
| `impl SSHHandler` | 113 | manager_handler.rs | |
| `HandlerError` | 180 | **manager_handler.rs** | error type for SSHHandler |
| `impl std::fmt::Display for HandlerError` | 182 | manager_handler.rs | |
| `impl std::error::Error for HandlerError` | 188 | manager_handler.rs | |
| `impl From<russh::Error> for HandlerError` | 190 | manager_handler.rs | |
| `impl From<String> for HandlerError` | 196 | manager_handler.rs | |
| `impl Handler for SSHHandler` | 203 | manager_handler.rs | Russh trait impl |
| `SSHConnectionManager` | 329 | **facade** (manager.rs) | god object |
| `impl SSHConnectionManager` | 342 | facade | 24+ connection_* + 30 misc fns |
| `PTYSession` | 2356 | **manager_session.rs** | |
| `impl PTYSession` (Display) | 2361 | manager_session.rs | |
| `impl PTYSession` (Drop) | 2373 | manager_session.rs | |
| `PortForward` | 2443 | **manager_port_forward.rs** | |
| `PortForwardDirection` (enum) | 2452 | manager_port_forward.rs | |
| `PortForwardManager` | 2459 | manager_port_forward.rs | |
| `impl PortForwardManager` | 2464 | manager_port_forward.rs | |
| `impl Default for PortForwardManager` | 2616 | manager_port_forward.rs | |

### 1.3 helper fns (no struct owner)

| fn | Line | Owner | Notes |
|---|---|---|---|
| `truncate_at_char_boundary` | 33 | facade | used by truncate logic |
| `ssh_cfg_get` (cfg ssh_config) | 41 | facade | ssh_config helper |
| `ssh_cfg_has` (cfg ssh_config) | 52 | facade | ssh_config helper |
| `sftp_mkdir_all_prefixes` | — | facade | SFTP helper used in connection |

### 1.4 tests

| Block | Lines | 内容 |
|---|---|---|
| `mod tests` | 2643-2810 | 167 lines of tests |

Tests stay in facade (167 lines fits comfortably).

### 1.5 Round 5-R12b lessons

| 错误类 | Round hit | R13 防御 |
|---|---|---|
| Cargo.lock drift | R6 | Plan YAML preflight baseline cargo check |
| cargo check stop-at-first-error | R6 | 4 crates parallel check (services-integrations + 3 downstream) |
| M3 model 慢 | R6 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Worker 漏 test attribute | R9b | preserve `#[test]`/`#[tokio::test]` |
| mod.rs 漏 `pub mod` | R3b | 每个新 sibling 必须在 mod.rs 加 `pub mod` + `pub use` |
| Cross-reference paths 错 | R11b/R12b | sibling 用 `use super::sibling::*` 模式 |
| `is_concurrency_safe` correctness fix | R12 | R13 无类似问题 |
| Pre-existing unwrap | R11b/R12b | 区分 pre-existing vs new, 不"修复" pre-existing |
| R11b D1+D2: 二次拆 needed | R11b | R13 一次性 split (4 文件足够) |
| R12 D-deviation: tests 1114 行超 cap | R12 | R13 提前估算 tests 行数 (~167, fits in facade) |
| R12b Set-Content 编码陷阱 | R12b | Worker 用 Write tool (UTF-8 native) |

### 1.6 外部 caller

| Caller path | 调用内容 |
|---|---|
| `src/crates/assembly/core/src/service/remote_ssh/manager.rs` | re-export facade (3 lines) |
| `src/crates/assembly/core/src/service/remote_ssh/remote_terminal.rs` | uses PTYSession |
| 25+ files in `assembly/core/src/agentic/...` | use `crate::remote_ssh::manager::*` (re-exported items) |

mod.rs 当前 re-exports (5 cfg-gated items):
- `KnownHostEntry`, `PTYSession`, `PortForward`, `PortForwardDirection`, `PortForwardManager`, `SSHConnectionManager`

R13 spec 必须 preserve 这些 public API 路径。

---

## §2 拆分方案

### §2.1 目标文件结构

```
src/crates/services/services-integrations/src/remote_ssh/
├── mod.rs 60 +3 pub mod + 3 pub use re-exports
├── manager.rs 700 facade: SSHConnectionManager + KnownHostEntry + ActiveConnection + helpers + tests
├── manager_handler.rs 400 SSHHandler + HandlerError + 4 impls
├── manager_session.rs 250 PTYSession + Display + Drop
├── manager_port_forward.rs 300 PortForward + Direction + PortForwardManager + Default
├── password_vault.rs 241 (unchanged)
├── paths.rs 203 (unchanged)
├── remote_exec.rs 1195 (D-DEV: > 1000 cap, separate round R13b— )
├── remote_fs.rs 415 (unchanged)
├── remote_terminal.rs 372 (unchanged)
├── types.rs 330 (unchanged)
├── workspace_registry.rs 210 (unchanged)
└── workspace_search/ <dir>
```

### §2.2 目标行数

| File | 目标 | cap | 备注 |
|---|---|---|---|
| mod.rs | 60 | 200 | +3 pub mod + 3 pub use |
| manager.rs (facade) | 700 | 800 | god methods split (24 connection_* + helpers + tests) |
| manager_handler.rs | 400 | 800 | SSHHandler + 4 impls |
| manager_session.rs | 250 | 800 | PTYSession + 2 impls |
| manager_port_forward.rs | 300 | 800 | PortForward + 2 types + impl |
| **TOTAL** | 1710 | — | vs original 2810 = -1100 lines |

### §2.3 mod.rs (sub-facade) 设计

```rust
//! Remote SSH service contracts (Round 13 split)
//!
//! `manager.rs` (2810 lines) split into facade + 3 sub-handlers per domain.
//! - manager (facade): SSHConnectionManager + helpers + tests
//! - manager_handler: SSHHandler + HandlerError (Russh callback)
//! - manager_session: PTYSession (Display + Drop)
//! - manager_port_forward: PortForward + Direction + PortForwardManager

pub mod paths;
pub mod types;
pub mod workspace_registry;
#[cfg(feature = "workspace-search")]
pub mod workspace_search;

#[cfg(feature = "remote-ssh-concrete")]
pub mod manager;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_handler;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_session;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_port_forward;
#[cfg(feature = "remote-ssh-concrete")]
mod password_vault;
#[cfg(feature = "remote-ssh-concrete")]
mod remote_exec;
#[cfg(feature = "remote-ssh-concrete")]
pub mod remote_fs;
#[cfg(feature = "remote-ssh-concrete")]
pub mod remote_terminal;

// Public API re-exports (unchanged - preserves caller paths)
pub use paths::*;
pub use types::*;
pub use workspace_registry::*;

#[cfg(feature = "remote-ssh-concrete")]
pub use manager::{KnownHostEntry, SSHConnectionManager};
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_handler::{HandlerError, SSHHandler};
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_session::PTYSession;
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_port_forward::{PortForward, PortForwardDirection, PortForwardManager};

#[cfg(feature = "remote-ssh-concrete")]
pub use remote_exec::{
 get_global_remote_exec_process_manager, RemoteExecCommandRequest, RemoteExecCommandResponse,
 RemoteExecControlAction, RemoteExecControlOrigin, RemoteExecControlRequest, RemoteExecError,
 RemoteExecProcessLifecycleEvent, RemoteExecProcessLifecycleStatus, RemoteExecProcessManager,
 RemoteExecResult, RemoteExecSessionCompletion, RemoteExecSessionCompletionSource,
 RemoteExecSessionCompletionStatus, RemoteSendStdinRequest, RemoteWriteStdinRequest,
};
#[cfg(feature = "remote-ssh-concrete")]
pub use remote_fs::RemoteFileService;
#[cfg(feature = "remote-ssh-concrete")]
pub use remote_terminal::{RemoteTerminalManager, RemoteTerminalSession, SessionStatus};
```

### §2.4 跨 sibling 共享类型 (避免循环依赖)

- `SSHConnectionManager` 在 facade
- `SSHHandler` 在 manager_handler,但需要被 `SSHConnectionManager` 创建 (作为泛型参数 `Handle<SSHHandler>`)
- 解决: `SSHHandler` 必须 `pub(crate)` 可见 (不 pub 到 crate 外部,只在 services-integrations crate 内可见)
- `manager.rs` 用 `use super::manager_handler::SSHHandler;`

### §2.5 handler 中需要跨 sibling 的引用

`SSHHandler` 内部持有 `Vec<u8>` (Russh 数据缓— ),不引用其他 sibling 的类型。但 `SSHConnectionManager::new()` 创建 `SSHHandler`,所以 facade 需要 `use super::manager_handler::SSHHandler`。

### §2.6 PTYSession / PortForward 引用 SSHConnectionManager

`PTYSession` 和 `PortForwardManager` 可能引用 `SSHConnectionManager` (通过弱引用或 callback)。需要检查实际依赖关系:
- 如果 PTYSession 只持有 `Arc<Handle<SSHHandler>>`,则不需要 import SSHConnectionManager
- 如果持有 `Weak<SSHConnectionManager>`,则需要 `use super::SSHConnectionManager` (cyclic 可能,需重构)

---

## §3 验证策略

### §3.1 编译验证

```bash
cd E:\agent-project\northing
git fetch origin
git worktree add ../northing-impl-round13 -b impl/round13-remote-ssh-manager-split main

# preflight baseline (R12b 已知 899/0/1)
git checkout origin/main
cargo check -p services-integrations --features remote-ssh-concrete --lib --message-format=short 2>&1 | Tee-Object baseline-main-cargo-check.log
cargo test -p northhing-core --features product-full --lib 2>&1 | Tee-Object baseline-main-cargo-test.log

$baselineErrors = (Select-String -Path baseline-main-cargo-check.log -Pattern "error\[" | Measure-Object).Count
$baselineTestResult = (Select-String -Path baseline-main-cargo-test.log -Pattern "test result:" | Select-Object -First 1).ToString()
Write-Host "BASELINE_ERRORS=$baselineErrors"
Write-Host "BASELINE_TESTS=$baselineTestResult"

git checkout impl/round13-remote-ssh-manager-split
```

### §3.2 测试验证

```bash
cargo test -p northhing-core --features product-full --lib # expect 899/0/1
cargo test -p services-integrations --features remote-ssh-concrete --lib # expect all pass
```

### §3.3 line count 验证

```bash
# 4 sibling files line counts
for sibling in manager manager_handler manager_session manager_port_forward; do
 py -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing-impl-round13\src\crates\services\services-integrations\src\remote_ssh/${sibling}.rs', encoding='utf-8')))"
done
# expected: manager ~700, manager_handler ~400, manager_session ~250, manager_port_forward ~300
```

---

## §4 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| manager.rs facade 800 cap | ≤ 800 | ~700 | 24 connection_* + 30 misc + tests |
| manager_handler.rs 800 cap | ≤ 800 | ~400 | small |
| manager_session.rs 800 cap | ≤ 800 | ~250 | small |
| manager_port_forward.rs 800 cap | ≤ 800 | ~300 | small |

如果任一文件超 800,需 R13c 二次拆。

### 已知 D-deviation (NOT R13 scope)

- `remote_exec.rs` 1195 行 (R11 已拆过) — D1: > 1000 cap by 195 (19.5% over)
- 这是 R11 之后的遗留,**R13 不修这个**,单独 R13b 处理。

---

## §5 实施步骤 (autonomous, sequential)

1. **manager_handler.rs** (~400): SSHHandler + HandlerError + 4 impls from facade + cargo check + 报告行数
2. **manager_session.rs** (~250): PTYSession + Display + Drop from facade + cargo check + 报告行数
3. **manager_port_forward.rs** (~300): PortForward + Direction + PortForwardManager + Default from facade + cargo check + 报告行数
4. **manager.rs** (facade, ~700): delete moved content, replace with god method split + cargo check + cargo test + 报告行数
5. **mod.rs**: +3 pub mod + +3 pub use re-export + cargo check
6. **cross-crate caller check**: cargo build --workspace + grep `use.*remote_ssh::manager::` paths
7. **final verification**: cargo test -p services-integrations + cargo test -p northhing-core
8. **commit + merge + handoff**

**每步必须**: cargo check 0 errors + **报告当前 sibling 行数**

### Critical: cargo check stop-at-first-error prevention (R6 教训)

```bash
cargo check -p services-integrations --features remote-ssh-concrete --lib --message-format=short 2>&1 | Tee-Object upstream-check.log
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object -Append upstream-check.log
cargo check -p northhing-tools-execution --features product-full --lib --message-format=short 2>&1 | Tee-Object -Append upstream-check.log
```

### Critical: Cargo.lock drift check (R6 教训)

```bash
git show origin/main:Cargo.lock | Select-String 'name = "russh"'
Get-Content Cargo.lock | Select-String 'name = "russh"'
```

### Critical: 12-class sub-domain errors (R11b/R12b lessons reinforced)

1. **Import paths**: facade 用 `use super::manager_handler::SSHHandler;`,handler 用 `use super::super::types::*;` (回到 root crate)
2. **Sibling method visibility**: SSHHandler 用 `pub(crate)` 而非 `pub` (不在 crate 外部暴露)
3. **Struct field visibility**: ActiveConnection 是 facade-private (无跨 sibling 共享)
4. **Cargo.lock drift**: see above
5. **mod.rs `pub mod`**: 3 new siblings MUST be declared with feature gate
6. **Test attribute preservation**: preserve `#[test]` / `#[tokio::test]`
7. **cargo check stop-at-first-error**: see above
8. **Cross-sibling shared enum/trait**: PTYSession + PortForward 不共享 struct (独立)
9. **R10a 1130 unused imports**: 精确 use blocks per sibling
10. **R11a struct owner mapping**: spec §1.2 显式 mapping
11. **Worker 每步报告行数**: cargo check 后 wc -l 当前 sibling
12. **R11b cross-reference paths**: handler 用 `pub(crate)` SSHHandler 供 facade 引用

---

## §6 Verification

```bash
# 0 NEW unwrap/panic/unreachable in production
git diff origin/main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
 | Select-String '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# expect 0

# 0 fns dropped (102 → 102)
py -c "
import re
from pathlib import Path
wt_dir = Path(r'E:\agent-project\northing-impl-round13\src\crates\services\services-integrations\src\remote_ssh')
fns = set()
for f in wt_dir.glob('*.rs'):
 fns.update(re.findall(r'^\s*(— :pub(— :\([^)]+\))— \s+)— (— :async\s+)— fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'worktree fns: {len(fns)}')
print('expected: 102')
"

# Cargo test baseline
cargo test -p northhing-core --features product-full --lib
# expect 899/0/1

# Cross-crate public API unchanged
git grep -l 'use.*remote_ssh::manager::\(KnownHostEntry\|PTYSession\|PortForward\|PortForwardDirection\|PortForwardManager\|SSHConnectionManager\)' -- 'src/'
# expect: count unchanged from baseline

# Iron rules
git diff origin/main..HEAD -- src/crates/services/services-integrations/src/remote_ssh/ \
 | Select-String '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# expect 0 NEW
```

---

## §7 spec review check-list

QClaw 重点检查:
1. 4 sibling 文件拆分结构 (per §1.2 struct owner mapping)
2. SSHHandler visibility `pub(crate)` (避免 crate 外部暴露,但允许 facade 创建)
3. mod.rs +3 pub mod + +3 pub use re-exports
4. cross-crate caller unchanged (25+ files use `use crate::remote_ssh::manager::*`)
5. line counts ≤ 800 (facade 700, others 250-400)
6. 0 fns dropped (102 → 102)
7. pre-existing unwrap preserved (R11b/R12b pattern: 不"修复" pre-existing)
8. cargo test 899/0/1 + services-integrations tests pass
9. 0 NEW iron rules violations

---

## §8 Errata

- §1.5 R12b Set-Content 教训: Worker 必须用 `Write` tool (UTF-8 native) 写 .rs 文件,不用 PowerShell `Set-Content`
- §1.5 R12 教训: tests 单独文件避免超 cap (R13 tests 167 行直接留在 facade OK)
- §2.4 SSHHandler 在 manager_handler.rs 用 `pub(crate)`,facade 用 `use super::manager_handler::SSHHandler` 引用
- §4 R13 不修 remote_exec.rs D-deviation (R13b 单独处理)
- §5 step 1-3 顺序按从最小到最大 (handler 最小先做,facade 最后改)

---

## §9 Pre-existing vs NEW violations distinction (R11b/R12b lesson)

R13 split 必须区分:
- **Pre-existing unwrap/panic in production**: 在原 manager.rs 已存在的,move 到新文件,**不"修复"**
- **NEW violations**: split 时新增的,**禁止**

典型 pre-existing in manager.rs:
- 多处 `unwrap()` 在 SFTP / Russh handler callback 中
- 多个 `expect("...")` 在已知 invariant 点

Worker 必须:
1. 记录 pre-existing unwrap 数量 (e.g. "moved 12 unwrap from facade to handler")
2. 报告 0 NEW (git diff 检查)
3. 不主动改写为 `— ` 或 `ok_or()`

---

## §10 跨轮次参考

- R11b split 模式 (sub_facade + re-export)
- R12 call_impl god method split 5-phase pattern
- R12b thin facade re-export pattern (preserves caller paths without modification)

本轮 (R13) 主要应用 R11b + R12b patterns:
- sub_facade mod.rs + pub use re-export
- 每个 sibling `use super::super::*` 访问跨模块类型
- tests 留在最大文件 (facade),不单独 split (避免 R12 tests 超 cap 教训)

---

*Spec committed at e4261ff (R12b predecessor); R13 spec written by Mavis following
R5-R12b pattern.*