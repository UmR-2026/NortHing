# Task T2-9-B2 Brief — A7 事件管道收敛 + NullDispatcher 空转路径移除

## 来源与验收标准（逐字）

来源：`docs/architecture/backend-roadmap.md` T2-9 行批 2（其中两项）：

> **事件管道收敛 A7**（BackendEvent 死管道并入 EventQueue 或删除）、**desktop NullDispatcher 空转路径移除**（agent-dispatch B2，回退直连直至 dispatcher 真接线）

**验收**：Spec 1（A7）与 Spec 2（NullDispatcher）全部落地 + 验证命令输出进 report。

## 编排者预检结论（explore 侦察 2026-08-21，直接采信；证据锚点全在其中，有疑问先复核再动手）

### A7：BackendEvent 死管道（钉死）

- `BackendEvent` 定义于 `assembly/core/src/infrastructure/events/event_system.rs:13-26`；`BackendEventSystem::set_emitter`（:39）**全仓零调用** → emitter 恒 None → `emit()` 空转；facade `emit_backend_event`（kernel_facade/events.rs:68-76）**零 caller**。
- 生产端约 10 处无效调用：bash_tool×4、exec_command×3、grep_tool、ask_user_question_tool、mcp interaction、acp manager_permission（+ import）。
- 活管道 = AgenticEvent/EventQueue：desktop 经 `agentic/system.rs:87-103` 泵 → EventRouter → kernel_facade `KernelEventSubscriber` → `subscribe_events` → `app_state/event_bridge.rs:315-352` → Slint；CLI/server 直接 dequeue。**无消费方需要迁移，纯删**。
- kernel-api 侧死半截：`BackendEventDto`（events.rs:97）+ `KernelEventsApi::emit_backend_event`（:117）+ re-export（kernel-api lib.rs:34、kernel_facade/mod.rs:17）。注意 contracts 层注释区 Source #83/#84 引用需同步。
- payload 孤儿：`util::types::event` 的 `ToolExecutionProgressInfo`/`ToolTerminalReadyInfo`/`BackgroundCommandLifecycleInfo`。
- 最后防线：删前 `rg 'backend-event'` 确认 desktop 无字符串通道依赖（预检已确认无，你再核一遍）。

### NullDispatcher 空转路径

- 构造点 `app_state/actor.rs:22-120`（唯一生产点），调用点 `create_ui.rs:173`；消费链：state.rs OnceLock → coordinator_init.rs:98-100 转发 → pipeline_types.rs 字段/setter → pipeline_pre.rs:84 塞 ToolUseContext → TaskTool（`task_tool_subagent.rs:111,176`）。
- **A2 门**：`so_lifecycle/mod.rs:47-51` `USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some()` → run_a1_path；删构造点后恒 None → 走 legacy phase1/2/3（= 计划要求的"回退直连"，也是 CLI/server 现行路径）。
- **后台路径从来不用 runtime**（so_dispatch.rs:140-182 显式传 None）。
- ⚠️ **flag 处置（裁定，不许偏离）**：`USE_LIGHTWEIGHT_ACTOR` 保持 `true` **不翻转**（骨干不变量的变更需 flag flip + 集成测试，超出本任务授权）；改为在 `agent-dispatch/src/flags.rs` 注释 + `a1_path.rs` 激活测试注释中如实写明"当前无生产 runtime 生产者，A2 路径待 dispatcher 真接线"。
- K4a 豁免清单牵连（家规 2 文档同步）：`docs/design/2026-07-25-k4a-desktop-facade.md` §6 豁免③ 与 `docs/status/audit-compile-health_20260727.md:124` 同 commit 更新。

## 复用侦察（强制）

先核：legacy phase1/2/3 路径现状（so_lifecycle/ 下文件）确认"回退直连"目标存活；`emitter.rs` 的 EventEmitter re-export 是否有他用（A7 删除面判断）；`backend-event` 字符串全仓 grep。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **A7 删除（建议两个 commit 分段，见下）**：
   - ① 删 facade/契约半截：`KernelEventsApi::emit_backend_event` + `BackendEventDto` + kernel_facade/events.rs:68-76 实现 + 两处 re-export + 注释区 Source #83/#84 引用同步。
   - ② 删 core 实现半截：`event_system.rs` 整文件、`infrastructure/events/mod.rs` 与 `lib.rs`/`infrastructure/mod.rs` 的 re-export、~10 处生产调用点（删 import + 删 emit 调用；**不新增任何 AgenticEvent variant**——预检裁定 YAGNI）、三个 payload 孤儿结构体、`emitter.rs`（确认无他用后）。
   - 同 commit 更新 `docs/status/surfaces.md`（crate/文件结构变动，家规 2）。
2. **NullDispatcher 移除（方案 (a)+，预检钉死）**：删 `app_state/actor.rs` 整文件 + `app_state/mod.rs` 模块声明 + `create_ui.rs` import/调用 + `state.rs` 字段与 set/get + `callbacks_lifecycle.rs:59-103` A3 演示块 + `coordinator_init.rs:98-100` 转发 + `pipeline_types.rs` setter/字段（ctor 改内部自建空 OnceLock，不动 `agentic/system.rs` 与测试构造）+ `pipeline_pre.rs:84` 改传 `None`。**保留**：`ToolUseContext.actor_runtime` 字段、handoff 参数、a1_path.rs 全部（留作真接线挂点）。flags.rs/a1_path 测试注释按上方裁定更新；K4a 两文档同步。
3. **冒烟**：desktop Task 工具路径回退后，跑一次 Task 工具冒烟测试（找最近的 subagent/task_tool 相关 focused 测试跑通；若无现成，`cargo test -p northhing-core --features product-full --lib subagent`）。
4. 不顺手碰：配置镜像（批 2 第三项，另一单）、a1_path.rs 内容、USE_LIGHTWEIGHT_ACTOR 值。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 纯删除任务：任何"顺手改行为"的冲动都压住；发现预检与现状不符 STOP 报 BLOCKED。
- 历史事故禁令：删除后逐符号 rg 核实无悬空引用（crate 级 allow(unused_imports) 会掩盖，S-1 教训）；callbacks_lifecycle.rs 是 god-file 观测对象，report 附一句其健康度变化（删 A3 块后更清晰/持平/更纠结）。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. 每段 commit 后 `cargo check --workspace`
2. `cargo check -p northhing`（家规 6）
3. `cargo test -p northhing-core --features product-full --lib subagent`（或就近等价）
4. `cargo test -p northhing-agent-dispatch`
5. `node scripts/check-core-boundaries.mjs`
6. `pnpm run check:rot`
7. `pnpm run fmt:rs`

## 报告

`.superpowers/sdd/task-t29b2-report.md`：Spec 逐条、复用侦察节、callbacks_lifecycle 健康度一句、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `a1437f9`；worktree `E:\agent-project\.worktrees\northing-t29b2`（分支 `feat/t29-batch2-0821`）
- commit message 后缀 `(T2-9-B2)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
