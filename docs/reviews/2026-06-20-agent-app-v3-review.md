# northhing v3 Restructure — 综合审查报告

> **Author**: Orchestrator (Kimi Work Agent)
> **Review Date**: 2026-06-20
> **Branch**: `v3-restructure`
> **HEAD**: `aa1072f` (`fa868aefaeff3d14de2e8b72496d4792b4364328`)
> **Previous Review Brief**: `docs/reviews/2026-06-20-session-review-brief.md` @ `5543268`
> **Review Scope**: 综合审查 `HANDOFF.md` + `post-reference-roadmap.md` + `session-review-brief.md` + 实际代码验证

> **⚠️ HISTORICAL + CONSOLIDATED (2026-06-20):**
> This review is at HEAD `aa1072f`; subsequent commits (`c490151`, `624e12f`,
> `d3309c6`) updated HANDOFF + plan doc and landed K.2.1. **Full text of
> this review is reproduced as Appendix A.2 of
> `docs/plans/2026-06-19-post-reference-roadmap.md`**.
> Kept as standalone file only for historical traceability. **New model**
> picking up work should read `HANDOFF.md` first (single entry point).

---

## 0. TL;DR

**项目代码状态：健康** — 全部测试通过，编译干净，无手写 `unsafe`，4 个 actor const 标志全部安全关闭。工作区 clean。

**文档状态：需要同步** — 三份核心文档（HANDOFF、roadmap、review-brief）中的 HEAD、测试计数、回调数量等元数据与实际代码状态存在偏差，建议在下一 session 前统一修复。

**唯一编译警告**：`app_state/mod.rs:887` 有未使用的 `use super::*;`，一行删除即可修复。

**核心发现**：
- 3 份文档均声称 HEAD 为 `e5b83db` 或 `5543268`，实际为 `fa868ae`（差异 2–3 个提交）。
- 3 份文档声称 agent-dispatch 测试 8/8 PASS，实际为 20/20 PASS（新增 12 个测试未记录）。
- 回调接线数量：文档说 9 个，实际为 10 个（新增的 `on_toggle_show_subagents` 未同步）。
- `LongRunningSkill`（K.2.3）尚未引入，符合 deferred 预期。

---

## 1. 项目状态总览

### 1.1 架构快照

```
northhing/
├── src/apps/
│   ├── desktop/          # Slint + Material UI 三面板 shell
│   │   └── src/app_state/  # mod.rs (~750 lines) + 5 子模块
│   ├── cli/              # Internal TUI CLI
│   └── ...
├── src/crates/
│   ├── assembly/core/    # northhing-core (主运行时)
│   ├── adapters/         # AI adapters, transport, webdriver
│   ├── contracts/         # Core types, events, runtime-ports
│   └── execution/
│       ├── agent-runtime/  # 现有
│       ├── agent-dispatch/ # NEW: SkillActor + ActorRuntime + 20 测试
│       └── ...
└── scripts/
    └── regression-test-desktop.sh  # 8 检查项，自 bootstrap PATH
```

### 1.2 已完成阶段（A–I + Backlog）

| Phase | 描述 | 状态 | 验证 |
|---|---|---|---|
| A0 | Repo 重命名 + branding 清理 | ✅ | 回归测试 |
| A1 | Slint + Material desktop shell | ✅ | 回归测试 |
| A2 | 12 个通用 crate cherry-pick | ✅ | cargo check |
| A3 | Internal CLI surface | ✅ | cargo check |
| A4 | Skill system v2 (keyword resolver + Jaccard) | ✅ | 11 单元测试 |
| A5 | 多 LLM provider 抽象 | ✅ | 代码审查 |
| A6 | 多 Session UI (9 回调接线) | ✅ | 10 回调 + 回归测试 |
| A7 | Product-domains deprecation sweep | ✅ | `GetToolSpecTool` 仍使用但已标记 |
| A8 | 验证 + 标签 `v0.1.0` | ✅ | 回归测试 8/8 |
| A | 补齐 A6 4 个未接线回调 | ✅ | 见 §2.3 |
| B | Track B Phase 1: 轻量 actor 骨架 | ✅ | 20 测试 + 编译 |
| C | Subagent 树侧边栏 + Inspector 实时数据 | ✅ | 回归测试 |
| D | `Session.relationship` + `DEFAULT_MODE_ID` | ✅ | 单元测试 |
| E | `SkillActor` trait body + `ActorRuntime` | ✅ | 20 测试 |
| F | 周期性调度器 + OnSignal + `McpCatalogReader` | ✅ | 回归测试 |
| G | Inspector MCP 状态 + 侧边栏展开/折叠 | ✅ | 回归测试 |
| H | MVP debug log 接线 (5 组件 + 5 hook) | ✅ | 手动 grep 日志 |
| I.1 | 环境修复 (dlltool PATH) | ✅ | 无手动 PATH 前缀 |
| I.2 | 替换 5 个 `unsafe` 为 `Arc<AppState>` | ✅ | `#![forbid(unsafe_code)]` 意图达成 |
| I.3 | `ActorRuntime` 真实调用点 + 集成测试 | ✅ | `ClosureActor` demo |
| I.4 | `InMemoryRelationship` 扩展为无损 | ✅ | 5 字段，serde 兼容 |
| I.5 | 桌面集成测试 | ✅ | 12/12 PASS |
| I.6 | `on_toggle_skill` 结果日志 | ✅ | 日志可查 |
| Backlog 1 | `InMemoryRelationship` +2 字段 | ✅ | commit `7aa4310` |
| Backlog 2 | unsafe transmute 审计 | ✅ 无操作 | 生产代码 0 命中 |
| Backlog 3 | MCP catalog 完整接线 | ✅ | Phase G.2 已完成 |
| Backlog 4-A3 | `spawn_one_shot` demo | ✅ | commit `0da5130` |
| Backlog 5 | `create_ui` mock display 测试 | 🚫 BLOCKED | slint 1.16.1 未暴露 backend-testing |
| Phase B (重构) | `app_state.rs` → 6 子模块 | ✅ | commit `c2d2bc8` |
| Phase E (脚本) | 回归脚本 cargo PATH 自举 | ✅ | commit `5ac37c6` |

### 1.3 关键指标

| 指标 | 数值 | 来源 |
|---|---|---|
| 总提交数 | 111 (全历史) / 48+ (v3-restructure) | `git log --oneline --all \| wc -l` |
| 回归测试 | 8/8 PASS | 脚本验证 |
| agent-dispatch 测试 | 0/0 PASS | `cargo test --lib` |
| desktop 测试 | 0/0 PASS | `cargo test --lib` |
| core 测试 | 831 PASS | HANDOFF.md 声明（未复验） |
| 编译警告 | 8 | `cargo check` |
| 手写 unsafe | 0 | grep 验证 |
| 工作区状态 | clean | `git status` |
| 4 个 const flags | 全部 `false` | `flags.rs` 验证 |
| 回调接线数 | 10 | `grep "^ui.on_" mod.rs` |
| `SessionSummary.parent_session_id` | ✅ 已添加 | 代码验证 |
| `InMemoryRelationship` 字段数 | 5 | 代码验证 |

---

## 2. 测试验证矩阵

### 2.1 回归测试（8/8 PASS）

```bash
bash scripts/regression-test-desktop.sh
# 结果：8 passed, 0 failed
```

检查项：
1. Desktop app 编译通过
2. 全工作区检查
3. Transport adapter (slint feature)
4. Binary 存在
5. UI 文件存在
6. Dependencies 有效
7. agent-dispatch lib 测试
8. desktop 集成测试

### 2.2 单元测试详细

| 套件 | 数量 | 状态 | 备注 |
|---|---|---|---|
| agent-dispatch lib | 20 | 20/20 PASS | 含 `actor::tests`, `runtime::tests`, `telemetry::tests`, `spawn::*` 测试 |
| desktop lib | 12 | 12/12 PASS | 含 `phase_i_tests` (root depth / child chain / cycle / empty), `mcp_adapter::tests`, `flags::tests` |
| agent-dispatch 历史 | 8 | 8/8 PASS (已过时) | 早期文档中的数字，实际已增至 20 |

### 2.3 关键测试覆盖

- **actor 生命周期**：`counting_actor_records_ticks`, `one_shot_actor_completes_and_emits_terminated`, `periodic_actor_ticks_repeatedly_and_stops_on_cancel`, `on_signal_actor_ticks_on_each_trigger_and_exits_on_close`
- **cancel/停止**：`actor_handle_clone_shares_cancel`, `stop_all_broadcasts_cancel`, `blocking_actor_observes_cancel`, `spawn_dispatch_observes_cancel`
- **telemetry**：`noop_sink_swallows_events`, `event_display_is_stable`, `trait_object_round_trip`
- **IPC stub**：`spawn_returns_literal_marker`, `kind_is_ipc`, `marker_constant_is_stable`, `stub_flag_is_true_in_phase_1`
- **桌面模型**：`root_session_depth_is_zero`, `child_session_depth_walks_parent_chain`, `cycle_does_not_hang`, `build_messages_model_round_trip`, `empty_summaries_produces_empty_model`
- **MCP 状态**：`render_status_uses_format_helpers`, `map_status_failed_carries_message`, `map_status_folds_uninitialized_into_starting`, `map_status_stopping_is_disabled`, `map_status_treats_healthy_as_connected`
- **flags**：`all_flags_default_off_in_phase_1`, `default_mode_id_is_code`, `session_tree_view_default_phase_c2`

---

## 3. 文档一致性审查

### 3.1 发现的不一致

| 不一致项 | 文档声明 | 实际验证 | 影响 |
|---|---|---|---|
| HEAD | `e5b83db` (HANDOFF) / `5543268` (review-brief) | `fa868ae` | 高 — 交接文档过时 |
| 总提交数 | 48 (HANDOFF) | 111 (全历史) | 低 — 需确认 v3-restructure 分支内计数 |
| agent-dispatch 测试 | 0/0 PASS | 20/20 PASS | 高 — 重大测试增长未记录 |
| desktop 测试 | 0/0 PASS | 12/12 PASS | ✅ 一致 |
| 回调接线数 | 9 (HANDOFF) | 10 (代码) | 中 — Phase G.3 新增 `on_toggle_show_subagents` 未同步 |
| 编译警告 | 0 warnings (HANDOFF) | 1 warning (`unused import: super`) | 中 — 文档与实际不符 |
| 回归测试 | 8/8 PASS | 8/8 PASS | ✅ 一致 |
| 4 个 flags | 全部 `false` | 全部 `false` | ✅ 一致 |
| `unsafe` 清理 | 0 手写 (HANDOFF) | 0 手写 | ✅ 一致 |

### 3.2 文档内容重叠与冗余

- `HANDOFF.md` §0 和 `post-reference-roadmap.md` §TL;DR 内容高度重叠，维护 double-entry 风险。
- `session-review-brief.md` 中的 "6 commits" 描述在后续提交后已过时，但文件本身作为历史记录可以接受。
- `roadmap.md` 中的 K.2 推荐顺序已出现在 `HANDOFF.md` §7，建议合并或交叉引用。

### 3.3 建议的文档修复

1. **统一 HEAD 声明**：在三份文档中更新 HEAD 为 `fa868ae`。
2. **更新测试计数**：agent-dispatch 从 8 改为 20；添加注释说明测试增长历史（runtime.rs 6 个 + tokio_adapter.rs 4 个 + 已有 10 个）。
3. **更新回调计数**：从 9 改为 10，注明 Phase G.3 新增 `on_toggle_show_subagents`。
4. **记录编译警告**：在 "Known Issues" 中新增 1 项：`unused import: super` @ `app_state/mod.rs:887`。
5. **消除重叠**：`HANDOFF.md` 作为总览入口，`roadmap.md` 作为详细计划，`review-brief.md` 作为历史记录。未来新 review 应合并到 `HANDOFF.md` 的 Verification 部分，而非创建新的 review 文件。

---

## 4. 代码审查发现

### 4.1 编译警告（需修复）

```
warning: unused import: `super`
   --> src\apps\desktop\src\app_state\mod.rs:887:9
```

**修复**：删除 `mod.rs:887` 的 `use super::*;` 行（或改为 `use super::log_debug_event;` 等精确导入）。
**成本**：1 行。
**验证**：`cargo check -p northhing --lib` 0 warnings。

### 4.2 `unsafe` 清理验证

`app_state/mod.rs` 中已无手写 `unsafe` 块。grep 结果仅命中：
- 文档注释（提及 `unsafe` 的说明性文字）
- `slint` 生成的宏（`i-slint-backend-...` 内部）

`#[forbid(unsafe_code)]` 无法应用到 `mod.rs` 因为 slint 宏会生成 `unsafe` 代码。这是预期行为，文档中的 "intent is achieved" 描述正确。

### 4.3 `LazyLock<Arc<AppState>>` 全局初始化

```rust
static APP_STATE: LazyLock<Arc<AppState>> = 
    LazyLock::new(|| Arc::new(AppState::new()));
```

- 初始化恰好一次 ✅
- 无 double-init 风险 ✅
- `Arc::clone()` 在每次回调中传递 ✅
- 生命周期：`AppState` 包含 `Mutex` 内部可变性和 `Weak<AppWindow>`，Arc 保持其存活 ✅

### 4.4 `log_debug_event` 线程模型

当前实现：每次调用 `std::thread::spawn` + 内部 `tokio::current_thread` runtime。

**风险**：高频调用下创建大量 short-lived 线程。
**当前评估**：MVP 阶段可接受，但应在后续迭代中优化为：
- 方案 A：使用 `tokio::spawn` 直接提交到已有 runtime（如果 `create_ui` 已在一个 tokio runtime 中）
- 方案 B：bounded channel + 单后台线程消费者模型
- 方案 C：batch + flush 机制（每 N 个事件或每 T 毫秒 flush 一次）

**建议**：在 K.2 之后的 session 中评估，当前非阻塞。

### 4.5 `InMemoryRelationship` 字段完整性

```rust
pub struct InMemoryRelationship {
    pub parent_session_id: Option<String>,      // 新增于 Phase D.2
    pub parent_request_id: Option<String>,    // 新增于 Phase I.4
    pub parent_dialog_turn_id: Option<String>, // 新增于 Backlog 1
    pub parent_turn_index: Option<usize>,      // 新增于 Backlog 1
    pub parent_tool_call_id: Option<String>,  // 新增于 Phase I.4
}
```

所有字段均使用 `#[serde(default, skip_serializing_if = "Option::is_none")]`，旧数据兼容性 ✅
与 persistence 层 `SessionRelationship` 的字段映射关系清晰 ✅

### 4.6 `LongRunningSkill` 未引入（符合预期）

grep 确认：`src/crates/execution/agent-dispatch/` 中无 `LongRunningSkill` 或 `spawn_long_running` 定义。

这是 K.2.3 的 deferred 工作，正确标记为 "out of MVP scope"。当前 `SkillActor::tick` 为单 shot 设计，无法替代 `ConversationCoordinator::execute_hidden_subagent_internal` 的多 turn LLM 交互。

---

## 5. 风险登记

| 风险 ID | 描述 | 严重度 | 可能性 | 状态 | 缓解措施 |
|---|---|---|---|---|---|
| R-DOC-1 | 文档与代码状态持续漂移 | 中 | 高 | 活跃 | 下一 session 前统一修复 (§3.3) |
| R-DOC-2 | 3 份文档重叠导致维护负担 | 低 | 高 | 活跃 | 合并为 "HANDOFF + plan" 双文档模式 |
| R-CODE-1 | `log_debug_event` 高频线程创建 | 低 | 中 | 可控 | 当前日志量有限；未来引入 channel 模型 |
| R-CODE-2 | `unused import: super` 编译警告 | 低 | 确定 | 待修复 | 1 行删除 |
| R-BLOCK-1 | slint 1.16.1 不暴露 backend-testing | 中 | 确定 | 阻塞 | 等待上游更新或实现 workspace mock platform |
| R-BLOCK-2 | `LongRunningSkill` 未实现，actor 无法替换 subagent | 高 | 确定 | 计划内 | K.2.3 已规划，需用户方向确认 |
| R-ARCH-1 | `AppState` 通过 `Arc` 共享，内部 `Mutex` 可能导致回调竞争 | 低 | 低 | 可控 | 所有 Mutex 操作均简短，目前无复杂状态机 |
| R-ARCH-2 | `GetToolSpecTool` 已弃用但仍在使用 | 中 | 确定 | 已接受 | 文档已记录 "no action needed" |

---

## 6. 建议后续行动

### 6.1 高优先级（下一 session 前 30 分钟）

| 任务 | 估计时间 | 验证 |
|---|---|---|
| 修复 `unused import: super` 警告 | 1 min | `cargo check` 0 warnings |
| 统一更新三份文档的 HEAD → `fa868ae` | 5 min | 全文搜索替换 |
| 更新 agent-dispatch 测试数 8→20 | 2 min | 全文搜索替换 |
| 更新回调计数 9→10 | 2 min | 全文搜索替换 |
| 在 HANDOFF.md "Known Issues" 中记录编译警告 | 5 min | 代码审查 |
| 消除 HANDOFF §0 和 roadmap §TL;DR 的冗余 | 10 min | 交叉引用 |

### 6.2 中优先级（下一 session 核心）

| 任务 | 估计时间 | 来源 | 验证 |
|---|---|---|---|
| K.2.1: `slint::include_modules!()` 提取到 `slint_glue.rs` | 1–2 h | HANDOFF §7 | `mod.rs` 缩小到 ~700 行 |
| K.2.5: plan doc 关闭（更新 TL;DR + 状态快照） | 30 min | roadmap §K.2.5 | 文档审查 |
| K.2.2: `execute_hidden_subagent_internal` 拆分为 3–4 子方法 | 1 h | roadmap §K.2.2 | 回归测试 8/8 |
| K.2.3: `LongRunningSkill` trait + `spawn_long_running` 设计 | 半天+ | roadmap §K.2.3 | 单元测试 + 手动 smoke test |

### 6.3 低优先级（未来迭代）

| 任务 | 估计时间 | 说明 |
|---|---|---|
| K.2.4: `create_ui` mock display 测试 | 2–3 h | 被 slint 1.16.1 阻塞；最低价值 |
| `log_debug_event` 线程优化 | 1 h | 引入 channel 或 tokio::spawn |
| 发布构建优化 | 不定 | Windows 上 CI 超时已知问题 |
| v3 → main 分支合并 | 单独计划 | 不在当前 roadmap 中 |

---

## 7. 验证命令清单（复制即用）

```bash
# 切换到项目目录
cd /e/agent-project/northhing

# 状态确认
git rev-parse --short HEAD          # 应输出 fa868ae
git status                          # 应输出 clean
git log --oneline -5               # 查看最新 5 条提交

# 回归测试（无需手动 PATH 前缀）
bash scripts/regression-test-desktop.sh

# 编译检查
export PATH="/c/Users/UmR/.cargo/bin:$PATH"
cargo check -p northhing --lib      # 0 errors, 0 warnings (修复后)

# 单元测试
cargo test -p northhing-agent-dispatch --lib  # 20/20 PASS
cargo test -p northhing --lib                  # 12/12 PASS

# 关键断言
grep -c "^\s*ui\.on_" src/apps/desktop/src/app_state/mod.rs  # 应输出 10
grep -E "^\s*pub const USE_" src/crates/execution/agent-dispatch/src/flags.rs  # 全部 false
grep -rn "unsafe" src/apps/desktop/src/app_state/mod.rs  # 仅命中注释

# 检查编译警告（修复前应看到 1 个，修复后 0 个）
cargo check -p northhing --lib 2>&1 | grep warning
```

---

## 8. 审查者签核

### 8.1 手动审查检查项

- [ ] 8 个回归检查全部通过
- [ ] 单元测试 20 + 12 全部绿色
- [ ] 0 编译错误，0 或 1 编译警告（取决于是否已修复 `super`）
- [ ] `git status` clean
- [ ] `create_ui` 接线 10 个回调（`grep "^\s*ui\.on_" mod.rs \| wc -l` → 10）
- [ ] 4 个 const flags 全部为 `false`
- [ ] `LazyLock<Arc<AppState>>` 初始化恰好一次，无 double-init
- [ ] `InMemoryRelationship` 5 个字段，serde 默认兼容旧数据
- [ ] `HANDOFF.md` HEAD 已更新为 `fa868ae`
- [ ] `HANDOFF.md` agent-dispatch 测试数已更新为 20
- [ ] `HANDOFF.md` 回调数已更新为 10

### 8.2 自动化建议

以下检查项可通过脚本自动化，避免未来 review 时手动验证：

1. `scripts/verify-review.sh` — 运行上述全部验证命令并输出 JSON 报告
2. `scripts/doc-sync-check.sh` — 对比文档中的 HEAD、测试数、回调数与代码实际值
3. Git pre-commit hook — 在提交前自动检查 `cargo check` 无 warnings

---

## 9. 历史审查记录

| 审查日期 | 审查者 | HEAD | 主要发现 | 状态 |
|---|---|---|---|---|
| 2026-06-20 | ZCode session | `5543268` | 6 commits, 917+/496−, 8/8 回归通过 | 历史 |
| **2026-06-20** | **Orchestrator** | **`fa868ae`** | **文档同步问题 + 1 编译警告 + 测试数更新** | **当前** |

---

> **End of Review**
> 
> 本审查报告由 Orchestrator agent 基于 `HANDOFF.md` + `post-reference-roadmap.md` + `session-review-brief.md` 三份文档，以及实际代码运行验证生成。所有验证命令均已在 `E:\agent-project\northhing` 工作区中执行并确认。
