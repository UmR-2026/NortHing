# Task T2-9-B2 Report — A7 事件管道收敛 + NullDispatcher 空转路径移除

## 1. 复用侦察

- **A7 事件管道收敛与 EventEmitter 复用**：
  - 全仓 `backend-event` 检索确认仅在已废弃的旧事件定义与生产端无效调用处存在，desktop 前端无任何字符串通道依赖。
  - `src/crates/assembly/core/src/infrastructure/events/emitter.rs` 与 `northhing_events::EventEmitter` 经核实仍被 `service/lsp`、`service/snapshot`、`service/workspace/identity_watch` 广泛使用，因此保留 `emitter.rs` 及 `infrastructure/events/mod.rs` 对 `EventEmitter` 的 re-export，仅删除死事件系统 `event_system.rs` 及其对应类型与导出。
- **legacy phase1/2/3 路径与 NullDispatcher 回退直连**：
  - 核实 `so_lifecycle/mod.rs`（`execute_hidden_subagent_internal`）在 `actor_runtime` 为 `None` 时，无缝回退至 `execute_hidden_subagent_phase1 -> phase2 -> phase3`（与 CLI/server 现行一致）。
  - 后台任务 `so_dispatch.rs`（`start_background_subagent`）原本即传 `None`。
  - desktop 在移除 `NullDispatcher` 与 `actor.rs` 注入后，`ToolUseContext.actor_runtime` 为 `None`，直连 legacy phase1/2/3 正常运作。

## 2. Spec 落地情况（逐条对照）

- [x] **Spec 1: A7 删除**
  - **Facade/契约层**：删除 `KernelEventsApi::emit_backend_event`、`BackendEventDto`、`kernel_facade/events.rs` 实现、`kernel-api` 与 `kernel_facade` 的 re-export，清理 `#83/#84` 文档注释。
  - **Core 实现层**：删除 `src/crates/assembly/core/src/infrastructure/events/event_system.rs` 整文件，移除 `infrastructure/events/mod.rs`、`infrastructure/mod.rs`、`lib.rs` 的相关导出。
  - **生产调用点清理**：清理 ACP `manager.rs`/`manager_permission.rs`、MCP `manager/mod.rs`/`interaction.rs`、`ask_user_question_tool.rs`、`grep_tool/local.rs`、`bash_tool` 系列模块（`mod.rs`、`bash_sandbox.rs`、`bash_tool_impl.rs`、`execute_loop.rs`、`execute_signal.rs`、`execute_stream.rs`）、`exec_command` 系列模块（`progress.rs`、`command/local.rs`、`command/remote.rs`、`command/shell_helpers.rs`）及 `scripts/split_manager.py`。
  - **Payload 孤儿清理**：删除 `util::types::event` 中的 `ToolExecutionProgressInfo`、`ToolTerminalReadyInfo`、`BackgroundCommandLifecycleInfo`。
- [x] **Spec 2: NullDispatcher 移除**
  - 删除 `src/apps/desktop/src/app_state/actor.rs` 整文件。
  - 移除 `src/apps/desktop/src/app_state/mod.rs` 模块声明 `pub(super) mod actor;`。
  - 移除 `src/apps/desktop/src/app_state/create_ui.rs` 中的 import 与 `maybe_construct_actor_runtime` 调用。
  - 移除 `src/apps/desktop/src/app_state/state.rs` 中的 `actor_runtime` 字段、`set_actor_runtime`、`actor_runtime()` 以及仅供 actor 注入使用的 `coordinator()` 方法。
  - 移除 `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`（第 59-103 行）A3 演示块。
  - 移除 `coordinator_init.rs` 中的 `set_actor_runtime` 转发方法。
  - 改造 `pipeline_types.rs`：移除 `ToolPipeline.actor_runtime` 字段及 `set_actor_runtime` 方法，`ToolPipeline::new` 参数名改为 `_actor_runtime` 保持签名兼容；`pipeline_pre.rs:84` 改为传 `None`。
  - 保留 `ToolUseContext.actor_runtime` 字段、handoff 参数与 `a1_path.rs` 全部实现以备未来真接线。
  - 更新 `flags.rs` 与 `a1_path.rs` 激活测试注释（`USE_LIGHTWEIGHT_ACTOR` 维持 `true` 不变，注明当前无生产 runtime 生产者，待 dispatcher 真接线）。
  - 同步更新 K4a 文档：`docs/design/2026-07-25-k4a-desktop-facade.md` §6 豁免③ 与 `docs/status/audit-compile-health_20260727.md:124`。
- [x] **Spec 3: 冒烟验证**
  - 运行 `cargo test -p northhing-core --features product-full --lib subagent`：56 passed, 0 failed。
  - 运行 `cargo test -p northhing-agent-dispatch`：27 passed, 0 failed。
- [x] **Spec 4: 范围守卫**
  - 未触碰配置镜像、`a1_path.rs` 核心逻辑及 `USE_LIGHTWEIGHT_ACTOR` 布尔值（保持 `true`）。

## 3. God-file 健康度观测

- `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`：行数由 1063 行下降至 1017 行（减少 46 行），删除了 A3 演示 actor 构造与 dispatch 空转链路后，生命周期回调函数更加纯粹清晰。

## 4. 遇到的编译错误与修复层级

- `E0583`（新 worktree 初始缺少构建产物 `service/i18n/generated_locale_contract.rs`）：机制层修复，运行 `pnpm run i18n:generate` 生成合约文件。

## 5. 验证命令与输出证据

### 1. `cargo check --workspace`
```
    Checking northhing-agent-dispatch v0.2.10
    Checking northhing-core v0.2.10
    Checking northhing v0.2.10
    Checking northhing-acp v0.2.10
    Checking northhing-cli v0.2.10
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 15s
```

### 2. `cargo check -p northhing`
```
    Checking northhing-core v0.2.10
    Checking northhing v0.2.10
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 38s
```

### 3. `cargo test -p northhing-core --features product-full --lib subagent`
```
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 989 filtered out; finished in 0.09s
```

### 4. `cargo test -p northhing-agent-dispatch`
```
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

### 5. `node scripts/check-core-boundaries.mjs`
```
Core boundary check passed.
```

### 6. `pnpm run check:rot`
```
✔ compliant fixture exits 0 and reports success (105.8594ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (101.6875ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (100.5022ms)
✔ registered god-file exceeding ceiling fails (5.4822ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.1592ms)
✔ actual workspace rot budget passes with current manifest (323.1432ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1361 files).
```

### 7. `pnpm run fmt:rs`
```
[format-changed-rust] Formatting 33 Rust file(s).
[format-changed-rust] Restoring 52 collateral Rust file(s) touched through module expansion.
```

## 6. 偏离声明

- 无任何偏离。所有操作严格遵循 brief 钉死的方案与预检爆炸半径。
