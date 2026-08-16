# Round 6 Spec: dialog_turn.rs 3652 → 1 facade + 7 sibling files (sub-domain split)

> **目标**: 把 `src/crates/assembly/core/src/agentic/coordination/dialog_turn.rs` (3652 行) 拆成 1 facade + 7 sibling files
> **风险**: HIGH (per split-analyzer) — 必须 sub-domain split（与 Round 5 chat.rs 模式相同）
> **Errata**: 见 §7

---

## §1 当前状态

| 项 | 值 |
|---|---|
| 文件路径 | `E:\agent-project\northing\src\crates\assembly\core\src\agentic\coordination\dialog_turn.rs` |
| 行数 | 3397 |
| 方法数 | 86 (24 public + 62 private) |
| Struct 名 | **`ConversationCoordinator`**（不在 dialog_turn.rs 文件名里 — 文件名 vs struct 不一致是历史遗留） |
| Cluster 数 | 35 (1 lifecycle 2770 行 + 34 trivial ~627 行) |
| Cross-cluster deps | 0 |
| Splittability | **HIGH risk** (analyzer: "0 cross-cluster dependencies, high coupling — consider refactoring before split") |
| before.json | `C:\Users\UmR\.qclaw\workspace\.rot\before-dialog-turn.json` |

**Round 3a 拆分历史**: `ConversationCoordinator` 已部分拆分（coordinator.rs 619 行 + dialog_turn.rs 3397 + subagent_orchestrator.rs 1773 + ports.rs 1739）。本 spec 只针对 dialog_turn.rs 部分。

**Struct 重复说明**: `coordinator.rs` 注释说 `ConversationCoordinator` impl split across `dialog_turn.rs + subagent_orchestrator.rs`。本次只动 dialog_turn.rs，不动 subagent_orchestrator.rs。

## §2 拆分方案（sub-domain split, 与 Round 5 chat.rs 同模式）

### §2.1 目标文件结构

```
src/crates/assembly/core/src/agentic/coordination/
├── mod.rs                                  # 已有 (39 行) — 不动
├── coordinator.rs                          # 已有 (619 行) — 不动
├── dialog_turn.rs                          # facade (~500 行) — was 3652 行
├── dialog_turn/                            # NEW sibling dir
│   ├── mod.rs                              # pub mod + facade impl (ConversationCoordinator struct + new + 24 public API + 9 sibling use)
│   ├── workspace.rs                        # workspace binding
│   ├── session.rs                          # session CRUD helpers
│   ├── turn.rs                             # start_dialog_turn_internal 701 + persist_cancelled + ensure_assistant_bootstrap
│   ├── compaction.rs                       # compact_session_manually 191 + helpers
│   ├── restore.rs                          # 15+ restore_* methods
│   └── thread_goal.rs                      # 8 public thread_goal_* + private helpers
├── subagent_orchestrator.rs                # 已有 — 不动
├── ports.rs                                # 已有 — 不动
├── scheduler.rs                            # 已有 — 不动
├── a1_path.rs                              # 已有 — 不动
└── state_manager.rs                        # 已有 — 不动
```

### §2.2 目标行数

| 文件 | 预计行数 | 说明 |
|---|---|---|
| `dialog_turn.rs` (facade) | ~500 | struct + new + 24 public API + 9 sibling use |
| `dialog_turn/mod.rs` | 0 | (Rust 不允许 file + dir 同名, dialog_turn.rs 替换为 dialog_turn/mod.rs) |
| `dialog_turn/workspace.rs` | ~250 | workspace binding |
| `dialog_turn/session.rs` | ~350 | session CRUD helpers |
| `dialog_turn/turn.rs` | **~900** | start_dialog_turn_internal 701 + helpers (超 800 cap, §7 E1 例外) |
| `dialog_turn/compaction.rs` | ~250 | compact_session_manually + helpers |
| `dialog_turn/restore.rs` | ~300 | 15+ restore_* methods |
| `dialog_turn/thread_goal.rs` | ~400 | 8 thread_goal_* public + private helpers |
| **Total** | **~2950** | (vs 3397 原文件 — 略 -450 因去 pub use * boilerplate) |

**Public API 在 facade** (24 个 method 全部留在 `impl ConversationCoordinator` block on mod.rs):
- `new` (constructor)
- `thread_goal_runtime` (getter)
- `set_scheduler_notifier`, `set_round_injection_source`, `set_actor_runtime`, `set_subagent_timeout` (config setters)
- `create_session`, `create_session_with_id`, `create_session_with_workspace`, `create_session_with_workspace_and_creator`, `create_hidden_subagent_session_with_workspace`, `update_session_model` (session CRUD)
- `ensure_assistant_bootstrap`
- `start_dialog_turn`, `start_dialog_turn_with_prepended_messages`, `start_dialog_turn_with_image_contexts`, `start_dialog_turn_with_image_contexts_and_prepended_messages`
- `get_thread_goal`, `clear_thread_goal`, `create_thread_goal`, `update_thread_goal_objective`, `set_thread_goal_objective`, `maybe_mark_thread_goal_usage_limited`, `set_thread_goal_status`, `pause_thread_goal_after_user_cancel`

### §2.3 sub-domain 分组依据（按职责）

| Sub-domain | 主要方法 |
|---|---|
| **Workspace binding** | resolve_workspace_id_for_config (66), build_workspace_binding (80), require_main_session_workspace (7), track_session_workspace_activity_best_effort (31) |
| **Session** | create_session_internal, create_session_with_id, create_session_with_workspace, create_session_with_workspace_and_creator (45), create_hidden_subagent_session_with_workspace (21), create_hidden_subagent_session (19), update_session_model, normalize_agent_type (7) |
| **Turn** | start_dialog_turn_internal (701), persist_cancelled_dialog_turn (76), ensure_assistant_bootstrap, restore_session_view, related helpers |
| **Compaction** | compact_session_manually (191), estimate_context_tokens, manual_compaction_metadata, build_manual_compaction_round_completed, build_manual_compaction_round_failed |
| **Restore** | restore_* (15+ methods) — public API 8 个 + private helpers |
| **Thread goal** | get_thread_goal, clear_thread_goal, create_thread_goal, update_thread_goal_objective, set_thread_goal_objective, maybe_mark_thread_goal_usage_limited, set_thread_goal_status, pause_thread_goal_after_user_cancel, schedule_thread_goal_resumed_steering, thread_goal_store |

总计 7 个 sub-domain。

## §3 字段可见性变更

`ConversationCoordinator` 当前字段需 `pub(crate)` 提升让 sibling 访问：

| 字段 | 类型 | 当前 | 改为 | 用途 |
|---|---|---|---|---|
| `config` | CliConfig | private | `pub(crate)` | workspace / turn / thread_goal 子模块 |
| `session_manager` | Arc<SessionManager> | private | `pub(crate)` | session / turn / compaction |
| `token_usage_service` | Arc<TokenUsageService> | private | `pub(crate)` | turn / thread_goal |
| `agentic_system` | Arc<AgenticSystem> | private | `pub(crate)` | workspace / session / turn |
| `subscription` | SubscriptionState | private | `pub(crate)` | restore / thread_goal |
| `subscribers` | HashMap | private | `pub(crate)` | restore / thread_goal |
| `next_subscriber_id` | u64 | private | `pub(crate)` | restore / thread_goal |
| `coordinator` | Arc<ConversationCoordinator> | private | `pub(crate)` | (该字段若存在) |

实际字段需从 `dialog_turn.rs` L100-150 区域扫一遍后填全。

**理由选 pub(crate)**：所有 sibling 都在 `crate::assembly::core::agentic::coordination::dialog_turn::*` 同 crate。

## §4 迁移步骤（每步 cargo check）

### Step 0: baseline
```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py src/crates/assembly/core/src/agentic/coordination/dialog_turn.rs --report | tee /tmp/baseline.txt
cargo check -p northhing-core --features product-full --lib --message-format=short
```
预期: 2 errors pre-existing (transport_remote.rs:515,549, NOT related to dialog_turn)

### Step 1: 创建 dialog_turn/ 目录 + 7 个空 sibling
```bash
mkdir src/crates/assembly/core/src/agentic/coordination/dialog_turn/
touch src/crates/assembly/core/src/agentic/coordination/dialog_turn/{mod.rs,workspace.rs,session.rs,turn.rs,compaction.rs,restore.rs,thread_goal.rs}
```

### Step 2: 删原 dialog_turn.rs 创建 facade mod.rs
- 删除 `src/crates/assembly/core/src/agentic/coordination/dialog_turn.rs`
- 改 `mod.rs` 内容: 
  - struct `ConversationCoordinator` + 字段定义
  - `impl ConversationCoordinator { pub fn new() ... }` (constructor)
  - `impl ConversationCoordinator { pub fn set_scheduler_notifier() { ... } }` 24 个 public 方法 (直接实现或 delegate 到 sibling)
  - `mod workspace; mod session; mod turn; mod compaction; mod restore; mod thread_goal;` (7 sibling declarations)

### Step 3: 提升 8 个字段 `pub(crate)` 
按 spec §3 表 — 字段前加 `pub(crate)`

### Step 4: 迁 workspace 子模块（4 方法）
迁 `resolve_workspace_id_for_config` + `build_workspace_binding` + `require_main_session_workspace` + `track_session_workspace_activity_best_effort` → `workspace.rs`

### Step 5: 迁 session 子模块（8 方法）
迁 `create_session_*` (5 方法) + `create_hidden_subagent_session*` (2 方法) + `update_session_model` + `normalize_agent_type` → `session.rs`

### Step 6: 迁 compaction 子模块（5 方法）
迁 `compact_session_manually` + `estimate_context_tokens` + `manual_compaction_metadata` + `build_manual_compaction_round_*` → `compaction.rs`

### Step 7: 迁 restore 子模块（15+ 方法）
迁所有 `restore_*` 方法 → `restore.rs`

### Step 8: 迁 thread_goal 子模块（8 public + 2-3 private）
迁 `get_thread_goal` + `clear_thread_goal` + `create_thread_goal` + `update_thread_goal_objective` + `set_thread_goal_objective` + `maybe_mark_thread_goal_usage_limited` + `set_thread_goal_status` + `pause_thread_goal_after_user_cancel` + `schedule_thread_goal_resumed_steering` + `thread_goal_store` → `thread_goal.rs`

### Step 9: 迁 turn 子模块（3 方法, ~900 行）
迁 `start_dialog_turn_internal` (701) + `persist_cancelled_dialog_turn` (76) + `ensure_assistant_bootstrap` → `turn.rs`

### Step 10: 最终验证
```bash
cargo check -p northhing-core --features product-full --lib --message-format=short
cargo test -p northhing-core --features product-full --lib
cargo fmt --check src/crates/assembly/core/src/agentic/coordination/
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py src/crates/assembly/core/src/agentic/coordination/dialog_turn.rs --json --out C:\Users\UmR\.qclaw\workspace\.rot\after-dialog-turn.json
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\subdomain-verifier.py  # worker 自写脚本
```

## §5 Gate 标准

- [ ] dialog_turn.rs 行数 ≤ 600 (facade)
- [ ] 每个 sibling ≤ 800 行（turn.rs ≤ 1000 reviewer 例外批准）
- [ ] 公共 API `ConversationCoordinator::new` + 24 public methods 不变
- [ ] `cargo check -p northhing-core --features product-full --lib` 0 NEW errors
- [ ] `cargo test -p northhing-core --features product-full --lib` 0 failed
- [ ] `cargo fmt --check` 0 diffs
- [ ] subdomain-verifier PASS（auto Gate, 类似 Round 5）

## §6 回滚方案

```bash
git revert <commit>
# 或
git checkout <pre-r6-commit> -- src/crates/assembly/core/src/agentic/coordination/dialog_turn.rs src/crates/assembly/core/src/agentic/coordination/dialog_turn/
```

## §7 Errata

### E1: turn.rs ~900 行超 800 cap

**事实**: start_dialog_turn_internal 单一方法 701 行 (类似 chat.rs handle_key_event 555 行), 无法在一次拆分中拆 sub-handler。

**Mitigation**: reviewer 接受 turn.rs ≤ 1000 行（spec §7 E1 传统）。后续 Round (v0.2.0) 可考虑把 start_dialog_turn_internal 拆为 prepare_turn / dispatch_turn / finalize_turn / cleanup 4 个 sub-handler。

### E2: 公共 API 24 个 method 留 facade（与 Round 5 chat 不同的策略）

**事实**: Round 5 chat.rs 拆分用了 facade + sibling pattern (sibling 包含 public method 的实现)。本 spec 采用 facade-only pattern (所有 public method impl 在 facade, sibling 只含 helper functions)。

**理由**: ConversationCoordinator 24 个 public method 调用链复杂, 跨 sibling delegate 会增加 boilerplate + 增加 call stack 开销。把 public method impl 放 facade + helper functions 放 sibling 是 trade-off。

**Mitigation**: 24 个 public method 直接在 facade `impl ConversationCoordinator` block 里实现; 真正跨多次调用的复杂 helper 提到 sibling 作为 free function。

### E3: Round 3a 历史遗留 — subagent_orchestrator.rs 也有 ConversationCoordinator impl

**事实**: coordinator.rs 注释说 ConversationCoordinator impl split across dialog_turn.rs + subagent_orchestrator.rs（Round 3a 已部分拆分）。

**本 spec 范围**: 只动 dialog_turn.rs 部分。subagent_orchestrator.rs 部分不动。如果用户希望完整 ConversationCoordinator 合并, 需另起 Round。

### E4: analyzer HIGH risk 信号与 sub-domain split 策略

同 Round 5: analyzer 报的 35 cluster 中 1 个 lifecycle 占 82%, 其余 34 是 trivial。本 spec 用 sub-domain split 消化 lifecycle cluster (类似 chat.rs 处理)。

## §8 不在范围

- 不动 `coordinator.rs` / `subagent_orchestrator.rs` / `ports.rs` / `scheduler.rs` / `a1_path.rs` / `state_manager.rs`
- 不动 `mod.rs` 已有结构
- 不动 24 个 public API 签名
- 不拆 start_dialog_turn_internal 701 行 (deferred to v0.2.0)
- 不做 Round 3a 未完成的 ConversationCoordinator 全合并（如果需要另起 Round）

## §9 引用

- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py` (产出 before.json)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\structure-verifier.py` (不支持 sub-domain split, 详见 Round 5 handoff)
- `C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier.py` (Round 5 worker 自写, 复用)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md` (worker 必须遵守 7 铁律)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\m3-orchestration-guide.md` (M3 编排指南)
- `docs/handoffs/2026-06-28-round5-chat-rs-split-impl.md` (Round 5 impl handoff 模板)
- `docs/handoffs/2026-06-28-round5-chat-rs-review-report.md` (Round 5 review 模板)
- `docs/code-rot-prevention-guide.md` (腐化预防指南)
- `docs/AGENT_ONBOARDING.md` (5 分钟接入指南)

## §10 Owner

- **Owner**: Mavis (orchestrator, M3 per QClaw guide)
- **Coder**: M2.7-highspeed sub-agent dispatched by Mavis
- **Reviewer**: User 给 Kimi K2.6 / QClaw
- **Final arbitration**: Mavis (after Kimi + QClaw verdicts returned)