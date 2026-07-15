# R22 Spec: terminal/exec.rs 2488 → facade + 4 sibling parallel split

> **目标**: 把 `src/crates/services/terminal/src/exec.rs` (2488 行, services-layer god-object) 拆为 1 facade + 4 sibling files, 走 R21+ parallel flow
> **风险**: MEDIUM (god-object 一次拆分, R21 dialog_turn 已建立 parallel flow pattern)
> **新流程**: 4 sub-rounds **并行** 跑 + producer self-report + Mavis 3-axis verify + sequential merge → user review → cleanup
> **预计时长**: ~2-2.5h（spec 10min + 4 producer 并行 90min + Mavis r22e 30min + verify 20min + squash 5min）

---

## §0 前置状态（实测 baseline, 2026-07-02）

| 项 | 值 |
|---|---|
| `exec.rs` 行数 | **2488** (canonical wc -l) |
| `terminal` crate 其他 sibling | api 610, events 219, lib 108, config 199, pty 4 files 1364, session 7 files 2783, shell 5 files 1663 |
| `lib.rs` 出口 | `pub mod exec;` + `pub use exec::{get_global_exec_process_manager, ExecCommandRequest, ExecCommandResponse, ExecControlAction, ExecControlOrigin, ExecControlRequest, ExecProcessLifecycleEvent, ExecProcessLifecycleStatus, ExecProcessManager, ExecSessionCompletion, ExecSessionCompletionSource, ExecSessionCompletionStatus, SendStdinRequest, WriteStdinRequest};` — 13 个 type facade re-export |
| Cross-crate 调用 | rg "terminal::exec" — 0 hits (全部 caller 走 lib.rs `pub use`) |
| R21 pattern 复用 | 4 producer 并行 + `_impl` 后缀 sibling method + facade mod.rs |

**exec.rs 结构（实测）**：
- L1-45: use + 7 const + GLOBAL_EXEC_MANAGER (OnceLock)
- L40-44: `pub fn get_global_exec_process_manager() -> Arc<ExecProcessManager>` (free fn)
- L46-140: 9 pub struct/enum (ExecCommandRequest / ExecCommandResponse / WriteStdinRequest / SendStdinRequest / ExecControlRequest / ExecControlAction / ExecControlOrigin / ExecSessionCompletion / ExecSessionCompletionStatus / ExecSessionCompletionSource / ExecProcessLifecycleStatus / ExecProcessLifecycleEvent) — 12 个 pub type
- L142-247: pub struct `ExecProcessManager` + 8 内部 struct/enum (`ExecSessionEntry`, `CompletedExecSession`, `ExecProcess`, `Terminator`, `PtyKeepAlive`, `WindowsPipeJobHandle`, `WindowsPipeJob`, `LocalPipeControlState`, `OutputState`, `OutputInner`, `OutputCursor`, `HeadTailText`)
- L249-640: `impl ExecProcessManager` (god-block, 6 pub method: exec_command / exec_command_streaming / write_stdin / write_stdin_streaming / send_stdin / control_session)
- L641-645: `impl Drop for ExecProcess`
- L647-724: `impl ExecProcess` (~78 行)
- L725-853: `impl OutputState` (~128 行)
- L854-863: `fn emit_lifecycle`
- L865-887: `completion_status_for_control_action` + `completion_for_closed_process`
- L888-898: `lifecycle_status_for_completion`
- L899-921: `spawn_lifecycle_exit_watcher`
- L922-927: `struct CollectedOutput`
- L928-1294: `impl HeadTailText` (god-impl, 366 行)
- L1295-1617: 顶层 free fn (PTY Windows/Unix + encoding + utility, ~322 行)

---

## §1 R22 拆分方案（4 sub-rounds 并行 + Mavis r22e 后处理）

### §1.1 sub-round 总览

| ID | 名称 | exec.rs 改 line 段 | 目标 sibling | 预计行数变化 |
|---|---|---|---|---|
| **r22a** | types-and-structs | L46-247 | `exec/types.rs` (新) | exec.rs -202 / types.rs +202 |
| **r22b** | manager-and-process | L249-724 | `exec/manager.rs` (新) | exec.rs -476 / manager.rs +476 |
| **r22c** | output-and-headtail | L725-1294 | `exec/output.rs` (新) | exec.rs -570 / output.rs +570 |
| **r22d** | platform-and-util | L1300-1617 | `exec/platform.rs` (新) | exec.rs -318 / platform.rs +318 |
| **r22e** | exec-cleanup（Mavis） | L1-45 + facade | `exec/mod.rs` (新, 替换 exec.rs) | exec.rs → mod.rs ~50 + 4 sibling |

### §1.2 exec.rs 改后预期

```
src/crates/services/terminal/src/
├── exec/                              # NEW dir
│   ├── mod.rs                         # facade (~50 行): use + mod declarations + pub use re-export
│   ├── types.rs                       # ~200 行: 12 pub struct/enum + ExecProcessManager struct + 8 internal struct/enum
│   ├── manager.rs                     # ~480 行: impl ExecProcessManager + Drop + impl ExecProcess
│   ├── output.rs                      # ~570 行: impl OutputState + CollectedOutput + impl HeadTailText
│   └── platform.rs                    # ~320 行: 30+ free fn (PTY Windows/Unix + encoding + utility)
└── (exec.rs DELETED, replaced by exec/mod.rs)
```

**Total**: 2488 → 1620 行 (-35%, 5 文件分散)

### §1.3 lib.rs 改动

`lib.rs` L20 `pub mod exec;` + L33-42 `pub use exec::{...}` 不变（Rust 自动从 `exec/mod.rs` re-export）。exec/ 是 dir, exec/mod.rs 是 facade。

---

## §2 4 sub-rounds 详细 spec

### §2.1 r22a types-and-structs

**目标**: 拆出所有数据类型和内部 struct/enum 到 `exec/types.rs`。

**exec.rs 改 line 段**: L46-247（严格, 不越界）

**目标 sibling**: `exec/types.rs` (~200 行)

**迁入内容**:
- 9 pub struct/enum 数据类型 (L46-140):
  - `ExecCommandRequest` (L47-56)
  - `WriteStdinRequest` (L59-65)
  - `SendStdinRequest` (L68-72)
  - `ExecControlAction` enum (L74-78)
  - `ExecControlOrigin` enum (L80-84)
  - `ExecControlRequest` (L86-93)
  - `ExecSessionCompletionStatus` enum (L95-101)
  - `ExecSessionCompletionSource` enum (L103-107)
  - `ExecSessionCompletion` struct (L109-113)
  - `ExecCommandResponse` (L115-124)
  - `ExecProcessLifecycleStatus` enum (L126-133)
  - `ExecProcessLifecycleEvent` (L135-140)
- pub struct `ExecProcessManager` (L142-145)
- `impl Default for ExecProcessManager` (L147-154)
- 内部 struct/enum (L156-247):
  - `ExecSessionEntry` (L156-162)
  - `CompletedExecSession` (L164-171)
  - `ExecProcess` (L173-182)
  - `Terminator` enum (L184-187)
  - `PtyKeepAlive` (L189-192)
  - `WindowsPipeJobHandle` type (L194-195)
  - `WindowsPipeJob` struct (L197-201)
  - `LocalPipeControlState` enum (L203-207) + impl (L209-216)
  - `OutputState` (L218-222)
  - `OutputInner` (L224-230)
  - `OutputCursor` (L232-235)
  - `HeadTailText` (L237-247)

**实施**:
- 新建 `exec/types.rs`
- 12 pub type + ExecProcessManager struct + 11 内部 struct/enum 全部迁入
- 加 `use super::*;` 或精确 `use` 引用其他 sibling（manager/output 用的类型）
- exec.rs L46-247 段删除

**R21 spec §3.3 教训**: sibling method 不能和 facade 同名。facade 是单文件 exec.rs 残留 (`get_global_exec_process_manager`), 不冲突.

**pub(super) vs pub 选择**:
- 12 pub type 保留 `pub` (cross-crate re-export via lib.rs)
- 内部 struct/enum 保留 private (无 `pub`)
- `ExecProcessManager` struct 保留 `pub`

**producer self-report**:
- exec.rs canonical wc-l: before 2488, after XXX (delta -XXX)
- types.rs canonical wc-l: 0 → XXX
- 12 pub type + 11 internal type 全部迁入
- 0 NEW unwrap/panic
- 0 NEW CRLF/BOM
- Long lines added: N (≤5 R18 tolerance)
- `cargo check -p terminal-core` 0 errors

### §2.2 r22b manager-and-process

**目标**: 拆出 ExecProcessManager 主 impl block + Drop for ExecProcess + impl ExecProcess 到 `exec/manager.rs`。

**exec.rs 改 line 段**: L249-724（严格）

**目标 sibling**: `exec/manager.rs` (~480 行)

**迁入内容**:
- `impl ExecProcessManager` (L249-640, ~392 行, 含 6 pub method: exec_command / exec_command_streaming / write_stdin / write_stdin_streaming / send_stdin / control_session)
- `impl Drop for ExecProcess` (L641-645)
- `impl ExecProcess` (L647-724, ~78 行)

**实施**:
- 新建 `exec/manager.rs`
- 加 `use super::types::*;`（ExecProcessManager / ExecProcess / ExecSessionEntry / CompletedExecSession / ExecSessionCompletion / ExecControlAction / ExecControlOrigin / ExecProcessLifecycleEvent / ExecProcessLifecycleStatus 等）
- 3 个 impl block 全部迁入
- exec.rs L249-724 段删除

**pub(super) vs pub**:
- 6 pub method on `ExecProcessManager` 保留 `pub` (cross-crate 调用 via `terminal::exec::ExecProcessManager::*`)
- 内部 helper fn 保持 private (无 `pub`)
- Drop + impl ExecProcess method (private) 保持 private

**producer self-report**: 同 r22a + manager.rs 行数 + 6 pub method preserved verbatim

### §2.3 r22c output-and-headtail

**目标**: 拆出 OutputState impl + HeadTailText impl + CollectedOutput struct 到 `exec/output.rs`。

**exec.rs 改 line 段**: L725-1294（严格）

**目标 sibling**: `exec/output.rs` (~570 行)

**迁入内容**:
- `impl OutputState` (L725-853, ~128 行)
- `fn emit_lifecycle` (L854-863) — output 触发事件
- `completion_status_for_control_action` (L865-870) — completion 状态映射
- `completion_for_closed_process` (L872-887) — completion 构造
- `lifecycle_status_for_completion` (L888-898) — lifecycle 状态映射
- `spawn_lifecycle_exit_watcher` (L899-921) — spawn lifecycle watcher task
- `struct CollectedOutput` (L922-927)
- `impl HeadTailText` (L928-1294, ~366 行 god-impl)

**实施**:
- 新建 `exec/output.rs`
- 加 `use super::types::*;`（OutputState / OutputInner / OutputCursor / HeadTailText / CollectedOutput / ExecProcessLifecycleEvent / ExecSessionCompletion / ExecSessionCompletionStatus 等）
- 迁入 1 struct + 1 impl + 5 free fn
- exec.rs L725-1294 段删除

**pub(super) vs pub**:
- `impl OutputState` method 保持 private (struct 是 internal)
- `impl HeadTailText` method 保持 private (struct 是 internal)
- 5 free fn 保持 private

**producer self-report**: 同 r22a + output.rs 行数 + HeadTailText 366 行 god-impl preserved verbatim

### §2.4 r22d platform-and-util

**目标**: 拆出所有 顶层 free fn（PTY Windows/Unix + encoding + utility helper）到 `exec/platform.rs`。

**exec.rs 改 line 段**: L1300-1617（严格）

**目标 sibling**: `exec/platform.rs` (~320 行)

**迁入内容** (30+ free fn):
- `configure_pipe_process_group` (L1295-1308, unix)
- `configure_pipe_process_group` stub (L1310-1311, non-unix)
- `configure_pipe_window_visibility` (L1313-1316, non-windows)
- `configure_pipe_window_visibility` stub (L1318-1319, windows)
- `create_windows_pipe_job` (L1387-1412, windows)
- `close_windows_pipe_job_handle` (L1414-1425, windows)
- `process_group_id` (L1427-1430, unix)
- `request_unix_pipe_control` (L1432-1450, unix)
- `signal_pipe_process_group_id` (L1452-1469, unix)
- `spawn_pipe_reader` (L1471-1487, generic)
- `spawn_pipe_reader_with_done` (L1489-1511, generic)
- `apply_sanitized_environment_to_pty` (L1513-1521)
- `sanitized_environment` (L1523-1534)
- `is_tauri_host_env` (L1536-1541)
- `deadline_from_now` (L1543-1546)
- `new_session_id` (L1548-1565)
- `new_chunk_id` (L1567-1569)
- `input_bytes_for_write` (L1571-1580)
- `bytes_to_string_smart` (L1582-1592)
- `detect_encoding` (L1594-1604)
- `decode_bytes` (L1606-1615)
- `looks_like_windows_1252_punctuation` (L1617-...)

**实施**:
- 新建 `exec/platform.rs`
- 加 `use super::types::*;`（需要 ExecProcess / ExecSessionEntry 等）
- 22+ free fn 全部迁入 (含 `#[cfg(unix)]` 和 `#[cfg(windows)]` 条件)
- exec.rs L1300-1617 段删除

**pub(super) vs pub**:
- 22+ free fn 保持 private (internal helpers)

**producer self-report**: 同 r22a + platform.rs 行数 + cfg(unix)/cfg(windows) 条件保留验证

### §2.5 r22e exec-cleanup (Mavis 后处理)

**目标**: 收尾 exec.rs L1-45 顶层段 + 创建 exec/mod.rs facade。

**Mavis 范围**:
- L1-24: use 全部迁到需要的 sibling (types.rs / manager.rs / output.rs / platform.rs)
- L26-36: 7 const (DEFAULT_YIELD_TIME_MS / MAX_RETAINED_OUTPUT_BYTES / MAX_EXEC_SESSIONS / MAX_COMPLETED_EXEC_SESSIONS / PIPE_INTERRUPT_GRACE_TIMEOUT_MS / PTY_EXIT_DRAIN_TIMEOUT_MS / CREATE_NO_WINDOW / PIPE_JOB_CLOSE_WAIT_MS) 迁到对应 sibling
- L38-44: `static GLOBAL_EXEC_MANAGER: OnceLock<...>` + `pub fn get_global_exec_process_manager()` — 留在 `exec/mod.rs` (cross-crate 入口)
- 创建 `exec/mod.rs`:
  ```rust
  //! Model-facing command execution runtime.
  //!
  //! Facade: re-exports types/manager/output/platform sub-modules.
  //!
  //! This runtime is intentionally separate from terminal sessions. Each
  //! `exec_command` starts a fresh local process; a session id is only retained
  //! while that process is still running so later calls can poll or write stdin.
  
  pub mod manager;
  pub mod output;
  pub mod platform;
  pub mod types;
  
  // Re-export main types for cross-crate API stability (R22 §2.5).
  pub use types::{
      ExecCommandRequest, ExecCommandResponse, ExecControlAction, ExecControlOrigin,
      ExecControlRequest, ExecProcessLifecycleEvent, ExecProcessLifecycleStatus,
      ExecProcessManager, ExecSessionCompletion, ExecSessionCompletionSource,
      ExecSessionCompletionStatus, SendStdinRequest, WriteStdinRequest,
  };
  
  use std::sync::{Arc, OnceLock};
  use types::ExecProcessManager as TypesExecProcessManager;
  
  // Re-export the type under its original name for facade compatibility.
  pub use types::ExecProcessManager;
  
  static GLOBAL_EXEC_MANAGER: OnceLock<Arc<TypesExecProcessManager>> = OnceLock::new();
  
  pub fn get_global_exec_process_manager() -> Arc<TypesExecProcessManager> {
      GLOBAL_EXEC_MANAGER
          .get_or_init(|| Arc::new(TypesExecProcessManager::default()))
          .clone()
  }
  ```

**Mavis 时机**: 4 producer commit + Mavis 3-axis verify PASS 后, 单人做 r22e + delete `exec.rs`。

---

## §3 visibility 与 import 规则

### §3.1 类型 visibility

- 12 pub struct/enum (Request/Response/Status/Event): 保留 `pub`
- `ExecProcessManager` struct: 保留 `pub`
- 11 内部 struct/enum (ExecSessionEntry / ExecProcess / OutputState / HeadTailText 等): 保留 private (无 `pub`)

### §3.2 方法 visibility

- 6 pub method on `ExecProcessManager` (exec_command / write_stdin / send_stdin / control_session): 保留 `pub`
- `impl OutputState` / `impl HeadTailText` method: 保持 private
- 22+ free fn: 保持 private

### §3.3 use 导入

- sibling 内访问其他 sibling type: `use super::types::*;` 或精确 `use super::types::{ExecProcessManager, ExecProcess, ...};`
- `super::super::*` 禁止（按 R5/R6 教训）

### §3.4 lib.rs re-export

`lib.rs` L33-42 `pub use exec::{...}` 不变（Rust 自动从 `exec/mod.rs` re-export 13 个 type + `get_global_exec_process_manager`）。

---

## §4 producer 并行约束

### §4.1 file ownership（互不重叠）

| Producer | 写 | 读 |
|---|---|---|
| r22a | `exec/types.rs` (新, 全权) | `exec.rs` L46-247 段（其他段只读） |
| r22b | `exec/manager.rs` (新, 全权) | `exec.rs` L249-724 段 |
| r22c | `exec/output.rs` (新, 全权) | `exec.rs` L725-1294 段 |
| r22d | `exec/platform.rs` (新, 全权) | `exec.rs` L1300-1617 段 |

**exec.rs 不同 line 段同时被 4 producer 改, 但段不重叠**:
- r22a: L46-247
- r22b: L249-724
- r22c: L725-1294
- r22d: L1300-1617
- exec.rs 其他段: 4 producer 都只读

### §4.2 worktree 隔离

每个 producer 在独立 git worktree:
- `impl/r22a-types-and-structs`
- `impl/r22b-manager-and-process`
- `impl/r22c-output-and-headtail`
- `impl/r22d-platform-and-util`

### §4.3 Cargo.lock

- producer 不要 `cargo update`
- 只跑 `cargo check -p terminal-core` (不改 lock)
- 4 worktree 后由 Mavis 在 main HEAD 一次性 `cargo check --workspace` 锁 Cargo.lock

### §4.4 timeout

- 每 producer `timeout_ms: 5400000` (90 min), engine cap 30 min, Mavis 监控 + extend-timeout 如需要

---

## §5 Mavis 3-axis verify (替代 10-axis)

| Axis | 命令 | PASS 标准 |
|---|---|---|
| 1. 编译过 | `cargo check --workspace --message-format=short` | 0 errors |
| 2. 跨 crate 测过 | `cargo check -p northhing-cli` + `cargo check -p northhing-desktop` + `cargo check -p northhing-server` | 0 errors (R19 教训: workspace check 漏跨 crate) |
| 3. 不退化 | `cargo test -p northhing-core --features product-full --lib` | 0 failed (baseline 899/0/1 preserved) + `cargo test -p terminal-core --lib` 0 failed |

注: R22 加 `cargo test -p terminal-core --lib` 因为 target crate 是 terminal-core, 必须测其内部测试不退化.

**其他 7 axis (line cap / long line / BOM / visibility / pub(super) / cross-ref / spec drift) 由 producer self-report, Mavis 不再独立跑**。

---

## §6 squash-merge + stage-summary

### §6.1 squash 顺序

1. 4 producer commit + push worktree branch
2. Mavis 4 个 worktree sequential merge to main (保持 4 个独立 commit, R20 mode)
3. r22e (Mavis cleanup commit) 删 `exec.rs` + 创建 `exec/mod.rs`
4. 用户找 QClaw + Kimi review
5. review 通过后 Mavis 写 stage-summary (无 squash, 保留 5+ commit 历史便于回溯)

### §6.2 stage-summary 必填

- sub-round 列表 + commit hash
- 各 sub-round self-report 关键数字（exec.rs 2488 → ~50 facade + 4 sibling）
- QClaw verdict (如有)
- Kimi verdict (如有)
- Mavis 3-axis verify 结果
- 合并 commit hash

---

## §7 Errata

### E1: exec.rs 单文件无现有 sibling 框架

**事实**: R21 dialog_turn/mod.rs 已有 7 个 sibling (R6/R7 拆过), R22 exec.rs 是单文件直接拆。

**Mitigation**: 4 producer 创建 4 新 sibling + r22e 创建 facade mod.rs. 不同于 R21 模式 (R21 复用已有 sibling + 改 mod.rs), R22 是从零创建 sibling + 改 lib.rs.

### E2: lib.rs re-export 跨 crate API 稳定

**事实**: `lib.rs` L33-42 `pub use exec::{...}` 13 个 type re-export. 拆分后 `exec/mod.rs` 必须 re-export 同 13 个 type 才能保持 cross-crate API.

**Mitigation**: r22e (Mavis) 在 exec/mod.rs 加 `pub use types::{...};` 13 个 type re-export, 保持 lib.rs facade 不变.

### E3: HeadTailText 366 行 god-impl

**事实**: `impl HeadTailText` (L928-1294) 是 exec.rs 第 2 大 god-impl, 仅次于 `impl ExecProcessManager`.

**Mitigation**: r22c 完整保留 verbatim, 不在 R22 拆 method 内部 (类似 R21 turn_subhandlers.rs 806 行 deferred R22 候选).

### E4: cfg(unix) / cfg(windows) 平台特定代码

**事实**: exec.rs 含 ~190 行 cfg(unix) + cfg(windows) 平台特定代码 (PTY process group / Windows pipe job).

**Mitigation**: r22d 完整保留 cfg 属性, 不修改平台行为. R22 scope 是文件拆分, 不是跨平台重构.

### E5: GLOBAL_EXEC_MANAGER 入口点

**事实**: L38-44 `static GLOBAL_EXEC_MANAGER` + `pub fn get_global_exec_process_manager()` 是 cross-crate 入口, 不能拆.

**Mitigation**: r22e (Mavis) 保留在 `exec/mod.rs`, sibling types.rs 仅定义 struct + Default impl, manager.rs 仅 impl block.

### E6: 4 producer 并行改 exec.rs 不同 line 段 vs merge 冲突

**风险**: 4 producer 在 4 worktree 都改 exec.rs, merge 时可能冲突。

**Mitigation**:
- spec §4.1 严格 line 段 ownership
- merge 顺序: 先 r22a (L46-247), 再 r22b (L249-724), 再 r22c (L725-1294), 最后 r22d (L1300-1617)
- 4 段不重叠, exec.rs 中 use 段 / const 段 / GLOBAL_EXEC_MANAGER 不动 (r22e 改)
- 如 merge 真冲突, Mavis 手工解（按 line 段所有权判, 不需 producer 重做）

### E7: producer commit 后用户 review 周期

**事实**: R20/R21 模式是 user-driven review (QClaw + Kimi verbal/commits).

**Mitigation**: R22 producer commit 后, Mavis 通知用户启动 review cycle. Review 通过前不 final 决策.

---

## §8 不在范围

- 不拆 `impl HeadTailText` 366 行 method 内部 (R22 候选 deferred)
- 不拆 `impl ExecProcessManager` 6 method 内部 (R22 候选 deferred)
- 不动 `lib.rs` `pub use exec::{...}` (R22e 通过 `exec/mod.rs` 间接 re-export)
- 不动 `ExecProcessManager` 6 pub method 签名
- 不动 12 pub struct/enum 字段定义
- 不动 cfg(unix) / cfg(windows) 平台行为
- 不做 cargo fmt 大范围扫尾 (pre-existing 17 行未提交 cargo fmt 改动是项目历史, R22 不碰)

---

## §9 时间预算（per-sub-round 90 min, 4 并行）

```
[0-10 min]   Mavis 写 spec → commit
[10 min]     Mavis 派 4 producer 并行
[10-100 min] 4 producer 同时跑各自 worktree
[100-120 min] producer commit + push worktree branch
[120-150 min] Mavis merge 4 worktree → main (sequential 4 commit, exec.rs 4 段)
[150-180 min] Mavis r22e exec.rs cleanup (use + const + GLOBAL_EXEC_MANAGER + exec/mod.rs)
[180-210 min] Mavis 3-axis verify
[210-240 min] Mavis 写 stage-summary + 等用户 review 信号
```

---

## §10 Owner

- **Owner**: Mavis (orchestrator)
- **Producer**: 4 个 sub-agent, `minimax/MiniMax-M2.7` (非 highspeed), 4500 calls / 5h 预算
- **Reviewer**: QClaw (user-driven, external) + Kimi (user-driven, external)
- **Final arbitration**: Mavis (after QClaw + Kimi verdicts)