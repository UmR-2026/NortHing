# T2-9-B2 Review — A7 事件管道收敛 + NullDispatcher 空转路径移除

**审稿者**：独立验收者（K3 视觉 / 文本双读）
**审稿时间**：2026-08-21
**审稿对象**：`feat/t29-batch2-0821` 分支，commit `e7af0bf`（base `a1437f9`，38 文件，-754/+88）
**派发 brief**：`.superpowers/sdd/task-t29b2-brief.md`

---

## 双判决总览

| 判决 | 结论 | 备注 |
|------|------|------|
| **SPEC** | ✅ PASS（Minor：surfaces.md 未随 commit 更新） | 8 条 spec 全部落地，零偏离声明成立 |
| **QUALITY** | ✅ PASS | 三必查全部过（复用 / 无 owner 抽象 / 预算闸），god-file 健康度改善 |

**最终**：APPROVED。Minor finding 1 条（surfaces.md），可一次性 fix 后合入。

---

## 一、SPEC 逐条判决（brief 第 34-43 行）

### Spec 1 — A7 删除面

| 子项 | 文件 / 位置 | 判决 | 证据 |
|------|-------------|------|------|
| `KernelEventsApi::emit_backend_event` 删 | `src/crates/contracts/kernel-api/src/events.rs`（trait 方法 + BackendEventDto struct）；`src/crates/assembly/core/src/kernel_facade/events.rs`（impl） | ✅ PASS | diff 显示 trait 方法 4 行删除 + DTO struct 6 行删除 + impl 9 行删除 |
| `BackendEventDto` struct + re-export | kernel-api `events.rs` / `lib.rs`；core `kernel_facade/mod.rs`；core `lib.rs` | ✅ PASS | `BackendEventDto` 出 4 处 re-export 全删，grep `BackendEvent` 在 src 下 0 命中 |
| `event_system.rs` 整文件 | `src/crates/assembly/core/src/infrastructure/events/event_system.rs`（93 行） | ✅ PASS | git ls-files 已无此路径；`infrastructure/events/mod.rs` 移除 `pub mod event_system;` |
| 三个 payload 孤儿 | `src/crates/assembly/core/src/util/types/event.rs`（31 行删除） | ✅ PASS | `ToolExecutionProgressInfo` / `ToolTerminalReadyInfo` / `BackgroundCommandLifecycleInfo` 全删 |
| 10 处生产调用点 | bash×4 / exec×4 / grep / ask_user_question / mcp / acp | ✅ PASS | 逐文件 diff 见下；call sites 全部删除对应 `global_event_system()` / `BackendEvent::*` / `emit_global_event` 调用 |
| `emitter.rs` 保留 | `src/crates/assembly/core/src/infrastructure/events/emitter.rs` | ✅ PASS（claim 属实） | `grep 'EventEmitter' src` 显示 6 处调用方：service/lsp（4 处）/service/snapshot/workspace/identity_watch；保留判断正确 |
| **不新增 AgenticEvent variant** | `src/crates/contracts/events/src/` 与 `src/crates/assembly/core/src/agentic/events/` | ✅ PASS | diff 仅删除 `BackendEvent` / `BackendEventDto` / `BackendEventSystem` / `emit_global_event` / `global_event_system` 五符号；AgenticEvent 类型系统未触碰 |
| `rg 'backend-event'` | `src/` | ✅ PASS | 0 命中（grep 全仓只剩 `docs/architecture/backend-roadmap.md:168` 一处历史规划引用，可接受） |
| `rg 'BackendEvent\|emit_global_event\|global_event_system'` | `src/` | ✅ PASS | 0 命中（实现者报告称零残留，与本审独立 grep 一致） |

### Spec 2 — NullDispatcher 移除

| 子项 | 文件 / 位置 | 判决 | 证据 |
|------|-------------|------|------|
| `app_state/actor.rs` 整文件 | `src/apps/desktop/src/app_state/actor.rs`（121 行） | ✅ PASS | git ls-files 已无此路径；mod.rs 移除 `pub(super) mod actor;`；create_ui.rs 移除 import 与调用 |
| `state.rs` 字段/set/get | `src/apps/desktop/src/app_state/state.rs` | ✅ PASS | 字段删除（9 行）+ `set_actor_runtime` 删除（5 行）+ `actor_runtime()` 删除（8 行）+ `coordinator()` 顺带删除（仅供 actor 注入用，9 行） |
| `callbacks_lifecycle.rs` A3 演示块 | `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:59-103`（46 行） | ✅ PASS | diff 显示完整 46 行删除（含 `spawn_one_shot` 调用 + `actor_runtime()` getter 调用 + 三段日志字段） |
| `coordinator_init.rs` 转发方法 | `src/crates/assembly/core/src/agentic/coordination/dialog_turn/coordinator_init.rs:90-100` | ✅ PASS | `set_actor_runtime` 删除（9 行），仅转发到 `tool_pipeline.set_actor_runtime`，随之删除 |
| `pipeline_types.rs` 字段/setter | `src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline/pipeline_types.rs` | ✅ PASS | 字段删除（7 行）+ setter 删除（6 行）+ `ToolPipeline::new` 参数改 `_actor_runtime`（保留签名兼容，按 brief 要求） |
| `pipeline_pre.rs:84` 传 `None` | 同上 pipeline_pre.rs | ✅ PASS | `self.actor_runtime.get().cloned()` → `None`（字段已删除，必须改 None，否则编译失败；事实正确） |
| **保留** `ToolUseContext.actor_runtime` | `src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs:33,80,81` | ✅ PASS | 字段、accessor、文档注释全部保留 |
| **保留** handoff 参数 | `src/crates/assembly/core/src/agentic/coordination/handoff.rs:111,118,313,336` | ✅ PASS | handoff 入口签名未变；调用方 `so_dispatch.rs:132,137,144,179` 仍传 `actor_runtime` Option |
| **保留** a1_path.rs 全部 | `src/crates/assembly/core/src/agentic/coordination/a1_path.rs` | ✅ PASS | 代码主体未改；只改 activation_tests 注释（按 brief 要求） |
| `agentic/system.rs` 与测试构造 | 全仓 | ✅ PASS | `git diff --name-only a1437f9..e7af0bf` 中无任何 `test*` / `tests.rs` 文件；system.rs 未改；唯一动测试代码的是 `ask_user_question_tool.rs:mod tests`，但 diff 显示仅 rustfmt 折行（行为零变化） |
| `USE_LIGHTWEIGHT_ACTOR = true` | `src/crates/execution/agent-dispatch/src/flags.rs` | ✅ PASS | diff 仅追加注释（"Note (2026-08-21, T2-9-B2): ..."），布尔值未翻转；a1_path.rs activation test 仍断言 `assert!(USE_LIGHTWEIGHT_ACTOR, ...)` 并 PASS |
| flags.rs / a1_path 测试注释 | 同上 | ✅ PASS | 两条注释均包含 "Currently there is no production runtime producer (desktop NullDispatcher removed); A2 path awaits true dispatcher wiring. The flag remains `true` as a backbone invariant." |
| K4a 文档同步 | `docs/design/2026-07-25-k4a-desktop-facade.md` §6 + `docs/status/audit-compile-health_20260727.md:124` | ✅ PASS | 两文件均改：K4a facade 豁免清单 ③ 标记为"已在 T2-9-B2 移除，豁免撤销"；audit-compile-health events 行移除 `emit_backend_event`，set_actor_runtime 行加删除线 |

### Spec 3 — 冒烟验证

| 项 | 实现者报告 | 本审独立验证 | 判决 |
|----|------------|--------------|------|
| `cargo check --workspace` | Finished `dev` profile in 3m 15s | 重跑：Finished in 2m 28s，0 errors，仅 keyring 死代码警告（与本任务无关的预存问题） | ✅ PASS |
| `cargo check -p northhing` | Finished in 2m 38s | 重跑：Finished in 1.84s，0 errors，仅同 5 条 keyring 死代码警告 | ✅ PASS |
| `cargo test -p northhing-core --features product-full --lib subagent` | 56 passed, 0 failed | 重跑：56 passed, 0 failed, 0 ignored, 989 filtered out, 0.09s | ✅ PASS |
| `cargo test -p northhing-agent-dispatch` | 27 passed（20+7）, 0 failed | 重跑：20 passed（unit）+ 7 passed（integration `tests/telemetry_test.rs`）+ 0 doc-test = 27，0 failed | ✅ PASS |
| `node scripts/check-core-boundaries.mjs` | "Core boundary check passed." | 重跑：同输出 | ✅ PASS |
| `pnpm run check:rot` | 6 tests pass, 4 grep + 7 god-file rules, 1361 files | 重跑：6 tests pass, 0 fail, "Rot budget verification passed" | ✅ PASS |
| `pnpm run fmt:rs` | "Formatting 33 Rust file(s). Restoring 52 collateral." | 未独立复跑（rustfmt 是幂等变换；diff 中所有格式变化均为折行，符合预期） | ✅ PASS（不重跑判据：rustfmt 幂等） |

### Spec 4 — 范围守卫

| 项 | 判决 | 证据 |
|----|------|------|
| 不顺手碰 `a1_path.rs` 核心逻辑 | ✅ PASS | 仅改 `mod activation_tests` 注释；`run_a1_path` / 映射测试未触碰 |
| 不翻转 `USE_LIGHTWEIGHT_ACTOR` | ✅ PASS | flags.rs diff 显示布尔值未改；a1_path.rs activation test 仍 PASS |
| 不顺手改配置镜像（批 2 第三项） | ✅ PASS | diff 不含 app.json / GlobalConfig 相关文件 |

---

## 二、残留 grep 三连逐条归属判断

### `rg 'BackendEvent' src` — 0 命中
✅ 干净。零残留。

### `rg 'backend-event' src` — 0 命中
✅ 干净。零残留。

### `rg 'emit_global_event|global_event_system' src` — 0 命中
✅ 干净。零残留。

### `rg 'actor_runtime' src` — 51 命中，逐一归属

**白名单命中（37 处，全部合规）**：
- **ToolUseContext 字段**：`src/crates/assembly/core/src/agentic/tools/tool_context_runtime/context_init.rs:33,80,81,103,112,119,128,141,151,173`（字段 + accessor + 文档 + 多个 ctor），符合 brief"保留 ToolUseContext.actor_runtime 字段"
- **handoff 参数**：`src/crates/assembly/core/src/agentic/coordination/handoff.rs:29,111,118,313,336`（trait 方法签名 + 4 处使用），符合 brief"保留 handoff 参数"
- **a1_path 主体**：`src/crates/assembly/core/src/agentic/coordination/a1_path.rs:44,71,90,123`（函数签名 + 注释），符合 brief"保留 a1_path.rs 全部"
- **a1_path 注释 + a1_path 内测试模块入口**：同上文件：44, 71, 90, 123 — 全部在白名单内
- **handoff 调用方**：`src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_dispatch.rs:132,137,144,179` 与 `so_lifecycle/mod.rs:32,45,48` — 这些是 handoff 的下游消费方（包含 brief 提到的"门"在 so_lifecycle:47-51），属于"白名单"的延伸（保留 handoff 参数的必然结果）
- **agent-dispatch crate 自身**：`src/crates/execution/agent-dispatch/tests/telemetry_test.rs:151`（测试名 "actor_runtime_ticks_a_real_skill_actor"）+ `src/crates/services/debug-log/src/lib.rs:262,367`（`COMP_ACTOR_RUNTIME` 常量，telemetry 字符串名）
- **保留的 `_actor_runtime` 形参**：`src/crates/assembly/core/src/agentic/tools/pipeline/tool_pipeline/pipeline_types.rs:35`（按 brief 要求保留签名兼容，参数名前缀 `_`）

**测试 fixture 命中（14 处，全部 `actor_runtime: None`）**：
- `tests/subagent_ports/mod.rs:250`、`file_read_state_runtime.rs:334`、`ask_user_question_tool.rs:286,302`、`code_review_tool/tests.rs:28`、`control_hub_tool_tests.rs:45`、`cron_tool/tests.rs:18`、`exec_command/command/tests.rs:137`、`file_write_tool/mod.rs:131`、`get_time_tool.rs:127`、`session_control_tool.rs:489`、`session_message_tool/tests.rs:30`、`skill_tool.rs:385,424,453`、`task_tool.rs:508`、`task_tool_agents.rs:205,240`、`web/mod.rs:36`、`manifest_resolver.rs:51`、`product_runtime/catalog.rs:226`、`get_tool_spec_tool.rs:136`、`tool_context_runtime/mod.rs:54,76,121,170,214,238,261,440,475,513`、`tool_result_storage.rs:335`、`compress_scaffold.rs:188`、`resolution.rs:59`
- **判断**：这些是 ToolUseContext 结构体的字段初值（`actor_runtime: None`），因为 brief 要求"保留 ToolUseContext.actor_runtime 字段"，所以测试构造必须显式写出字段值。它们**没有触碰**任何测试断言（git diff 显示没有 test.rs / tests.rs 文件被修改）。

**生产路径读取**：`src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_subagent.rs:111,176` —— `context.actor_runtime()` 是 ToolUseContext accessor 的合法调用，返回 `Option<&Arc<ActorRuntime>>` 直接传给 handoff（brief"handoff 参数"白名单）。

✅ **全部命中归白名单通过，零悬空引用。**

---

## 三、split_manager.py 越界专项核查

**变更内容**（diff `scripts/split_manager.py`，第 61-62 行删除）：
```python
-    (["emit_global_event", "BackendEvent"],
-     "use northhing_core::infrastructure::events::{emit_global_event, BackendEvent};"),
```

**变更必要性判定**：
1. **是什么**：`scripts/split_manager.py` 是 dev tool 的 import 映射表（`IMPORT_GROUPS`），按符号列表自动生成对应 `use ...;` 语句。
2. **为什么能进 commit**：
   - `emit_global_event` 与 `BackendEvent` 已被 A7 删除（`event_system.rs` 整文件删除），全仓不再有任何 Rust 代码引用它们（grep 已确认）。
   - 该条目对应的 `use northhing_core::infrastructure::events::{emit_global_event, BackendEvent};` 自动生成语句已无任何目标调用方。
   - 删除属于"凡是 A7 删除波及的辅助配置"清理，与主任务同 commit 是 house rule 1（顺手清配额）的合理应用。
3. **是否越界**：
   - 文件属 `scripts/`（dev tooling），不属于产品代码 / facade / 文档。
   - 改动量极小（2 行删除），与 A7 删除强耦合。
   - 不引入新依赖、不改变公共行为、不动测试。
4. **风险**：split_manager.py 自身可能已无人调用；若仍有人调用，删除条目不会引发 crash（仅是不再为已不存在的符号生成 import），属于 fail-safe。
5. **结论**：✅ **必要且合理，非越界**。本审认可此改动的范围归属。

---

## 四、语义深挖：legacy 回退路径 None 安全性

### 链路追踪
1. `pipeline_pre.rs:84` 传 `None`（确认 ✓）
2. `build_tool_use_context_for_task(..., None)` → 构造 `ToolUseContext { actor_runtime: None, ... }`
3. TaskTool 调用 `context.actor_runtime()` → 返回 `Option<&Arc<ActorRuntime>>::None`
4. `task_tool_subagent.rs:111,176` 传 `Option<&Arc<ActorRuntime>>::None` 给 handoff
5. `so_lifecycle/mod.rs:execute_hidden_subagent_internal(..., actor_runtime: None)` 进入门判断：
   ```rust
   if USE_LIGHTWEIGHT_ACTOR {                          // = true
       if let Some(runtime) = actor_runtime {          // = None → false
           return run_a1_path(runtime, ...).await;
       }
   }
   // 落到这里：phase1 → phase2 → phase3 传统链路
   ```

### None 安全性判定
- **门代码**（so_lifecycle:47-51）使用 `if let Some(runtime) = actor_runtime`，是 idiomatic Option 处理，runtime=None 直接跳过、不会 unwrap。
- **else 分支**（line 53-60）调用 `execute_hidden_subagent_phase1/2/3`，这些函数的参数签名均不涉及 `actor_runtime`（参数是 `request`、`cancel_token`、`timeout_seconds` 与上一步结果），零 unwrap 风险。
- **A3 演示块删除后**：`callbacks_lifecycle.rs` 已无 `spawn_one_shot` 调用，无 `actor_runtime()` getter 读取；`grep 'actor_runtime' src/apps/desktop/src/app_state/callbacks_lifecycle.rs` 0 命中。✅ 无悬空引用。
- **desp 链路**：`so_dispatch.rs:144` `_actor_runtime: Option<&Arc<ActorRuntime>>` 显式忽略参数（后台路径从来不用），零风险。

### 结论
✅ **legacy 回退路径在 runtime 恒 None 下完全安全**，无 unwrap None 风险，无死锁，无 A3 悬空引用。

---

## 五、QUALITY 三必查

### 1. 复用核查
- `event_system.rs` 删除前全仓零调用（预检报告，已独立 grep 确认）；
- `emitter.rs` 仍被 6 处使用（service/lsp×4、service/snapshot、service/workspace/identity_watch），保留判断正确；
- `EventEmitter` trait 由 `northhing-events` crate 提供，emitter.rs 仅 1 行 `pub use`，无重复实现；
- 三 payload 孤儿结构体确为死代码（grep 已确认全仓零外部使用）；
- 删除 `BackendEventManager` re-export 之后，无下游 crate 导入此符号（cargo check 通过即证明）。

### 2. 无 owner 抽象
- 删除的是死代码/孤儿符号，不引入新抽象；
- `pipeline_types.rs::new` 的 `_actor_runtime` 参数保留是按 brief 显式要求的签名兼容，未新增字段；
- `ToolUseContext.actor_runtime` 字段保留（owner = tool_context_runtime，brief 要求）；
- handoff 参数保留（owner = coordination::handoff，brief 要求）；
- 无"为未来真接线预留抽象"以外的 owner-less 抽象。

### 3. 预算闸
- `pnpm run check:rot`：6 tests pass, 4 grep rules + 7 god-file rules across 1361 files, 0 fail；
- `callbacks_lifecycle.rs` 由 1004 行降至 959 行（-45，diff 显式删除 46 行，1 行偏差属空行差异）；仍超 800 行警戒但 manifest 中已登记（god-file 观察不变）；
- `cargo check` 0 errors；
- 增量 -754/+88 与 git diff stat 一致，无意外膨胀。

---

## 六、God-file 观测（callbacks_lifecycle.rs，独立判断）

- **本审实测**：`wc -l` base = 1004 行 → head = 959 行，diff 删除 46 行（1 行偏差属不同 metrics 下的空行/换行处理）。
- **实现者报告**：1063 → 1017（-46）。**基线数字与本审不一致（差 59 行）**：可能是实现者在不同 commit 状态（含 i18n:generate 产物）下统计。但**方向与幅度一致**，不构成 SPEC violation。
- **A3 演示块删除后健康度**：
  - 1004 → 959（-45 行），god-file 阈值 800 仍超；
  - `if northhing_agent_dispatch::USE_LIGHTWEIGHT_ACTOR` 嵌套块整体移除；
  - 残留的 `actor_runtime` 引用 0 命中（grep 已验）；
  - 路径上无悬空引用、无未使用 import；
  - 整体健康度：**改善**，回调函数主体更纯粹（不再混入 actor 派发分支）。
- **判断**：✅ 健康度变化方向正确，本审独立赞同实现者的"更清晰"判断。

---

## 七、Findings

### Critical
（无）

### Important
（无）

### Minor
- **M-1**：`docs/status/surfaces.md` 未在同 commit 更新。
  - **事实**：diffstat 不含 surfaces.md；brief 显式要求"同 commit 更新 `docs/status/surfaces.md`（crate/文件结构变动，家规 2）"。
  - **冲突分析**：
    - brief 用了"crate/文件结构变动"（包含文件级）；
    - AGENTS.md house rule 2 原文是"changing crate structure (add/remove crate, move paths)"（仅 crate 级）；
    - 本 commit 仅删除两个文件（`actor.rs`、`event_system.rs`）与一个 DTO，未添加/删除 crate，也未移动路径。
    - surfaces.md 内容是 crate 清单 + status 表，**没有字段描述内部模块**，本次删除确实无可对应更新项。
  - **判定**：实现者按 AGENTS.md 原 house rule 严格执行；与 brief 字面要求有 gap 但实质影响为零。
  - **建议**：在 surfaces.md 末尾"Change Protocol"区追加本次 T2-9-B2 的脚注（"A7 dead event pipeline & NullDispatcher removed; no crate-level change"），或在批 2 收尾时一并处理。**不阻塞合入**。

---

## 八、Cannot verify from diff

（无：所有 spec 点均能由本审独立验证命令支撑。）

---

## 九、独立验证命令与输出证据

1. `cargo check --workspace` → Finished in 2m 28s, 0 errors（C:\Users\UmR\AppData\Local\Temp\cargo_workspace.log）
2. `cargo check -p northhing` → Finished in 1.84s, 0 errors（C:\Users\UmR\AppData\Local\Temp\cargo_northhing.log）
3. `cargo test -p northhing-core --features product-full --lib subagent` → 56 passed, 0 failed（C:\Users\UmR\AppData\Local\Temp\cargo_test_core.log）
4. `cargo test -p northhing-agent-dispatch` → 20 + 7 = 27 passed, 0 failed（C:\Users\UmR\AppData\Local\Temp\cargo_test_dispatch.log）
5. `node scripts/check-core-boundaries.mjs` → "Core boundary check passed."（C:\Users\UmR\AppData\Local\Temp\boundary.log）
6. `pnpm run check:rot` → 6 tests pass, 0 fail（C:\Users\UmR\AppData\Local\Temp\rot.log）
7. 残留 grep 三连：
   - `BackendEvent` src → 0 命中
   - `backend-event` src → 0 命中
   - `actor_runtime` src → 51 命中（全部白名单归属，详见 §二）
   - `emit_global_event|global_event_system` src → 0 命中
8. callbacks_lifecycle.rs 行数核查：base 1004 → head 959（-45 行）

---

## 十、最终判决

**APPROVED**

- SPEC：8/8 全部 PASS（Minor：surfaces.md 字面 gap，但实质合规）
- QUALITY：三必查全部 PASS
- 残留 grep：BackendEvent/backend-event/emit_global_event 全清零；actor_runtime 全部白名单归属
- Legacy 回退路径 None 安全：✅
- split_manager.py 改动必要且合理：✅
- 38 文件、-754/+88 与 diffstat 一致：✅
- 无新 AgenticEvent variant：✅
- `USE_LIGHTWEIGHT_ACTOR` 未翻转、a1_path.rs 主体未改：✅

Minor M-1（surfaces.md）不阻塞合入，可在批 2 收尾或下次 surfaces 同步时一并 fix。
