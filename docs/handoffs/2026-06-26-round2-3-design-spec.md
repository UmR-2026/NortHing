# Round 2 + 3 设计 Spec — 代码腐化重构

> **目标**: 把 Round 1 没做的腐化（78 高风险 unwrap + 两个超大文件）在一个 review cycle 内重构完。
> **方法**: 写完整 design spec → 按 spec 实施（每批内部 self-review 闭环）→ 单次 external review pass。
> **基础**: Round 1 4 个 commit 已在 main (e10fe79 / 1a0e86a / 3881bf0 / 3ef44e4)
> **结构腐化核心病灶**: `coordinator.rs` 7215 行 + `session_manager.rs` 6505 行

---

## 1. 总目标与边界

### 1.1 范围内

- **Round 2**: 78 处高风险 `.unwrap()` / `.expect()` 修复
- **Round 3a**: `coordinator.rs` 拆分为 4 个文件
- **Round 3b**: `session_manager.rs` 拆分为 3 个文件

### 1.2 不在范围（明确排除）

- **CLI chat 模式测试覆盖 (3665 行 0 tests)**: 这是个独立大工程，需要单独 spec。Round 3a/b 完成后再单独立项。
- **测试覆盖率 27→50%**: 同上，独立 scope。
- **`review_platform/mod.rs` (4867 行)**: 第二个超大文件但 audit 风险评级中，可后续 Round 4。
- **`persistence/manager.rs` (3640 行)**: 同上。
- **`config/types.rs`**: 中等文件，scope 内不改。
- **CLI / desktop / relay 的 `Mutex.lock()` 替换**: Round 1 只改了 app_state，剩余 .unwrap 在 cargo build + test 时同步处理。

### 1.3 验收原则

- **0 回归**: `cargo test -p northhing --lib` 保持 40/40，`cargo test -p northhing-relay-server --lib` 保持 3/3。
- **workspace build**: `cargo check --workspace` 0 error。
- **diff 集中度**: 每个 split commit 只动 1 个大文件（coordinator 或 session_manager），不混淆。
- **commit 颗粒度**: Round 2 是 1 个 commit，Round 3a 是 1 个 commit，Round 3b 是 1 个 commit。总计 3 commits。
- **行为不变**: 仅结构调整，不改业务逻辑。已有测试覆盖即可证明。

---

## 2. Round 2 — 78 高风险 unwrap 修复

### 2.1 风险分级标准

| 类别 | 风险 | 修法 |
|---|---|---|
| **CLI 启动 init** | 高（用户首启动失败） | `.expect("Failed to initialize X")` → `Result` 返回 + 上层 `process::exit(1)` 友好输出 |
| **Tokio runtime build** | 高（runtime 是 async 基础） | `expect("...")` → `Result` 返回 + 启动 fail-fast |
| **IO 操作（fs/zip/net）** | 中（panics 在用户操作时） | `.unwrap()` → `.context()— ` 或 `.unwrap_or_default()` + tracing warn |
| **字符串边界 / parse** | 中 | `.unwrap()` → `match` 优雅降级 |
| **测试内 `.unwrap()`** | 低 | 保留（测试 panic 是 fail 信号） |
| **tokio Mutex / RwLock lock** | 中（poison 风险但低频） | 保留（已用 parking_lot 大幅减少） |
| **channel recv `.unwrap()`** | 低 | 保留（SendError 不太发生） |
| **Arc::clone 后的强转** | 低 | 保留 |

### 2.2 修法约束

- **不删 panic**，panic 是 fail 信号；改用 `Result + — ` 上传到 main，main 决定怎么处理
- **不引入新 unwrap**
- **测试内 .unwrap() 不动**（panic 是 fail signal）
- 每个修改加注释 `// reason: <why this was unsafe before>`

### 2.3 Commit message 模板

```
fix(core): reduce 78 high-risk .unwrap()/.expect() to Result propagation

[逐项列出按文件分类的修复]

Test results:
- cargo check --workspace: pass
- cargo test --workspace: pass (40+3 unchanged)

Risk classification:
- 6 init unwraps → Result + process::exit
- 23 IO unwraps → — propagation + tracing warn
- 31 string/parse unwraps → match with default
- (其他保留)
```

---

## 3. Round 3a — coordinator.rs 拆分

### 3.1 当前结构 (7215 行)

```
coordinator.rs:
├── Lines 1-617: Subagent types (CancelTokenGuard, SubagentExecutionScope,
│ SubagentConcurrencyPermitGuard, SubagentTimeoutHandle,
│ SubagentResult) + helper fns
├── Lines 618-1214: ConversationCoordinator struct + constructor + accessors
├── Lines 1214-2860: Session CRUD + dialog turn start + finalize + persist
│ + thread goal management + manual compaction
├── Lines 2861-3568: start_dialog_turn_internal (700 行 monolith — audit 标记)
├── Lines 3569-4100: Subagent orchestration (wait/cancel/stop + subscribe)
├── Lines 4100-5701: 更多 method (subscribe_internal, get_subagent_concurrency_limiter, ...)
├── Lines 5702-5973: 5 trait impls (AgentSubmissionPort / AgentSessionManagementPort
│ / AgentTurnCancellationPort / RemoteControlStatePort / SessionTranscriptReader)
└── Lines 5973-7215: 剩余 trait impl + 测试代码入口
```

### 3.2 拆分目标 (4 个文件)

```
src/crates/assembly/core/src/agentic/coordination/
├── mod.rs (facade + pub use 重导出，~50 行)
├── coordinator.rs (ConversationCoordinator struct + subagent types
│ + accessors + 简单 helpers, ~1500 行)
├── dialog_turn.rs (dialog turn 生命周期: start/finalize/persist
│ + thread goal + manual compaction
│ + start_dialog_turn_internal 的 monolith, ~3500 行)
├── subagent_orchestrator.rs (subagent 类型 + 子代理编排
│ + wait/cancel/stop + subscribe, ~1400 行)
└── ports.rs (5 trait impls, ~800 行)
```

### 3.3 拆分约束

- **不动公共 API**：`ConversationCoordinator` 的所有 pub 方法签名不变；`get_global_coordinator()` 返回的 `Arc<ConversationCoordinator>` 不变
- **不改 trait impl 内容**：ports.rs 里 5 个 trait impl 直接搬迁，不改逻辑
- **monolith 不拆**：`start_dialog_turn_internal` 700 行是 audit 的目标，但本轮不拆这个函数（拆它需要先写 spec，太大），只搬迁到 dialog_turn.rs
- **pub use 重导出**：`mod.rs` 用 `pub use coordinator::*; pub use dialog_turn::*; ...` 让外部引用路径不变
- **测试代码单独 cfg(test) 模块**：保持原位置不变
- **每个文件 < 3500 行**（audit 红线）

### 3.4 Commit message 模板

```
refactor(coordinator): split 7215-line god object into 4 modules

Splits src/crates/assembly/core/src/agentic/coordination/coordinator.rs
into 4 files by responsibility region. Public API is unchanged:
all callers continue to use `use northhing_core::agentic::coordination::*`
and `get_global_coordinator()` still returns
`Arc<ConversationCoordinator>`.

Module layout:
- mod.rs (facade, ~50 lines)
- coordinator.rs (struct + accessors + helpers + subagent types, ~1500 lines)
- dialog_turn.rs (dialog turn lifecycle + thread goal + compaction, ~3500 lines)
- subagent_orchestrator.rs (subagent orchestration, ~1400 lines)
- ports.rs (5 trait impls, ~800 lines)

The 700-line `start_dialog_turn_internal` monolith is moved as-is to
dialog_turn.rs (the audit flagged it but a function-level split
needs its own spec and is deferred).

Test results:
- cargo check --workspace: pass
- cargo test --workspace: pass (40+3 unchanged)

Co-located tests in #[cfg(test)] modules are kept in their original
files; no test files are added or removed.
```

---

## 4. Round 3b — session_manager.rs 拆分

### 4.1 当前结构 (6505 行)

```
session_manager.rs:
├── Lines 1-200: SessionManagerConfig + Default + SessionTitleMethod + ResolvedSessionTitle
├── Lines 200-430: Workspace resolution + session context window helpers
├── Lines 517-680: Message building + context snapshot
├── Lines 687-805: Prompt cache loading/persistence
├── Lines 805-1000: Evidence ledger + compression contract
├── Lines 1000-1190: Model reconciliation + invalidation
├── Lines 1190-1637: Main SessionManager struct + get_session + 大量 CRUD 方法
└── Lines 1637-6505: 更多 helper + rebuild + cleanup
```

### 4.2 拆分目标 (3 个文件)

```
src/crates/assembly/core/src/agentic/session/
├── mod.rs (facade + pub use 重导出，~30 行)
├── session_manager.rs (SessionManager struct + Config + 主要 CRUD
│ + workspace resolution, ~2500 行)
├── evidence.rs (evidence ledger + compression contract + reconciliation,
│ ~2000 行)
└── compaction.rs (message building + context snapshot + prompt cache,
│ ~1500 行)
```

### 4.3 拆分约束

- **不动公共 API**：`SessionManager` 的所有 pub 方法签名不变
- **SessionManagerConfig / SessionTitleMethod / ResolvedSessionTitle** 留在 session_manager.rs（核心配置）
- **evidenceledger / compression contract methods** 移到 evidence.rs（语义相关）
- **message / context / prompt cache helpers** 移到 compaction.rs
- **pub use 重导出**：保持 `northhing_core::agentic::session::*` 路径
- **每个文件 < 2500 行**

### 4.4 Commit message 模板

```
refactor(session): split 6505-line session_manager.rs into 3 modules

Splits src/crates/assembly/core/src/agentic/session/session_manager.rs
into 3 files by responsibility region. Public API is unchanged.

Module layout:
- mod.rs (facade, ~30 lines)
- session_manager.rs (struct + config + main CRUD + workspace, ~2500 lines)
- evidence.rs (evidence ledger + compression + reconciliation, ~2000 lines)
- compaction.rs (message building + context snapshot + prompt cache, ~1500 lines)

Test results:
- cargo check --workspace: pass
- cargo test --workspace: pass (40+3 unchanged)

Co-located tests in #[cfg(test)] modules are kept in their original
files. session_manager_tests (1336 lines) stays with session_manager.rs.
```

---

## 5. 实施顺序与内部自审

### 5.1 执行顺序（避免— 突、便于自审）

```
Step 1: Round 2 unwrap sweep (1 commit)
 - 跨 25-crate workspace 改动
 - 主要是不动业务逻辑的 .unwrap() → Result 转换
 - 内部自审: cargo check --workspace + cargo test --workspace
 - 如果通过，commit 进 main

Step 2: Round 3a coordinator.rs split (1 commit)
 - 仅动 src/crates/assembly/core/src/agentic/coordination/
 - 内部自审: cargo check -p northhing-core + grep 公共 API 调用
 - 通过后 commit

Step 3: Round 3b session_manager.rs split (1 commit)
 - 仅动 src/crates/assembly/core/src/agentic/session/
 - 内部自审: cargo check -p northhing-core + grep 公共 API 调用
 - 通过后 commit
```

### 5.2 内部自审清单（每个 commit 前必跑）

```
□ cargo check --workspace (0 error)
□ cargo test -p northhing --lib (40/40 pass)
□ cargo test -p northhing-relay-server --lib (3/3 pass)
□ grep -rn "ConversationCoordinator::new\|get_global_coordinator" | wc -l
 (确认调用点签名不变)
□ cargo doc -p northhing-core --no-deps (公开 API doc 仍生成)
```

### 5.3 single-external-review 设计

外部 reviewer 看到：
- 3 个 commit，每个独立可读
- 每个 commit message 清晰列出做了什么
- 综合 review handoff (`docs/handoffs/2026-06-26-debt-r2r3-REVIEW.md`) 涵盖所有改动
- 内部自审 checklist 已经在 commit message 里说明

External reviewer 只需要一次 review pass：
- 如果全部 OK → accept
- 如果某 commit 有问题 → manual_retry 单个 commit
- 不需要反复的 review-fix-review cycle

---

## 6. 风险评估

### 6.1 Round 2 unwrap sweep 风险

**中**：每个 .unwrap() 改成 — + Result 上传可能破坏 error 类型契约。需要逐个 grep 调用点确保错误处理路径仍然有效。

**缓解**：
- 优先改不会破坏 error path 的（init 阶段）
- 中间阶段的 .unwrap() 改 .context() 加 tracing warn 保留 panic-on-fail 行为
- 测试覆盖了 happy path；如果改坏了 happy path，test 会失败

### 6.2 Round 3a coordinator.rs split 风险

**中**：5 个 trait impl 在多个 impl block 中间插入了 5000+ 行的方法，搬迁时容易漏掉方法或破坏 pub/private 边界。

**缓解**：
- 先 grep 所有 pub 方法名（保证搬迁完整）
- 搬迁后用 cargo check 验证编译通过
- 用 cargo doc 验证公开 API 完整

### 6.3 Round 3b session_manager.rs split 风险

**中**：6505 行 split 容易拆错位置（一个方法横跨两个 module boundary）。

**缓解**：
- 按职责 region 严格切分（不交叉）
- 先 grep 所有 method name 保证完整搬迁
- cargo check + test 验证

---

## 7. 不在范围 + 未来规划

### 7.1 已排除（明确范围）

- CLI chat 3665 行测试覆盖 → 独立 Round 4
- 测试覆盖率 27→50% → 独立 Round 5
- review_platform/mod.rs 4867 行 → Round 6
- persistence/manager.rs 3640 行 → Round 6
- config/types.rs 拆分 → Round 7

### 7.2 Spec 之外的 follow-up

- Round 3a 之后，coordinator/dialog_turn.rs 仍有 3500 行（包含 700 行 start_dialog_turn_internal）。下一步拆这个函数需要单独的 spec。
- Round 3b 之后，session_manager 仍有 2500 行。再拆需要更细粒度 spec。

---

## 8. 状态

- [x] Spec written (this file)
- [ ] Round 2 unwrap sweep — pending
- [ ] Round 3a coordinator split — pending
- [ ] Round 3b session_manager split — pending
- [ ] Internal self-review for each batch — pending
- [ ] Single external review handoff — pending

**Owner**: Mavis (orchestrator)
**External reviewer**: TBD (user 安排)
**Target**: 3 commits + 1 review handoff doc, single review pass