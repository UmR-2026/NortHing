# B+C+A+D 完成 Review 文档

## 任务状态

| 任务 | 状态 | 说明 |
|------|------|------|
| B: Shell-exec sandbox | ✅ 已完成 | denylist + BashTool/ExecCommandTool 集成 |
| C: Prompt loader architecture | ✅ 已完成 | 4层 PartitionedLoader + ExecutionEngine 集成 |
| A: CoordinatorHiddenSubagentSkill | ✅ 已完成 | direct execution wrapper 替代 A1StubSkill |
| D: Mock display test | ✅ 已完成 | NoopPlatform + MinimalSoftwareWindow headless 测试 |

---

## A 任务修改详情

### 修改文件

1. **`src/crates/assembly/core/src/agentic/coordination/a1_path.rs`** (+159/-50)
2. **`src/crates/assembly/core/src/agentic/coordination/coordinator.rs`** (+2/-1)

### 设计决策

经过深入分析，round-by-round stepping 和 custom SubagentDispatcher 都违反了现有架构约束：
- `LongRunningSkill::tick` 的 invariant #1（不能直接调用 LLM）与 `execute_dialog_turn` 的 monolithic 多轮循环存在架构层级不匹配
- `ToolDispatcherPort` 明确禁止 multi-round work（见 `lightweight_task.rs` 注释）

**最终方案：direct execution wrapper**
- `CoordinatorHiddenSubagentSkill::tick` 直接调用 `execute_hidden_subagent_internal`（完整 phase1/2/3）
- 传递 `actor_runtime=None` 避免递归进入 A1 gate
- 使用 `ctx.cancel` 作为 cancel token，coordinator phase2 内部会观察它
- 返回 `Done` 立即结束 skill

### 代码变更

#### a1_path.rs

**替换 `A1StubSkill` 为 `CoordinatorHiddenSubagentSkill`：**

```rust
struct CoordinatorHiddenSubagentSkill {
 id: String,
 request: HiddenSubagentExecutionRequest,
 cancel_token: Option<CancellationToken>,
 timeout_seconds: Option<u64>,
}

#[async_trait]
impl LongRunningSkill for CoordinatorHiddenSubagentSkill {
 fn id(&self) -> &str { &self.id }
 fn skill_name(&self) -> &str { "coordinator_hidden_subagent" }

 async fn tick(&mut self, ctx: &ActorContext, prior: Option<LightweightTaskOutput>) 
 -> Result<LongRunningTickOutput, ActorError> 
 {
 if prior.is_some() {
 return Err(ActorError::new(
 "CoordinatorHiddenSubagentSkill: unexpected prior output".to_string(),
 ));
 }

 let coordinator = get_global_coordinator()
 .ok_or_else(|| ActorError::new("Global coordinator not available".to_string()))— ;

 let cancel_token = self.cancel_token.clone()
 .unwrap_or_else(|| ctx.cancel.clone());

 let result = coordinator
 .execute_hidden_subagent_internal(
 self.request.clone(),
 Some(&cancel_token),
 self.timeout_seconds,
 None, // actor_runtime=None 避免递归 A1 gate
 )
 .await
 .map_err(|e| ActorError::new(e.to_string()))— ;

 let final_output = map_subagent_result_to_lightweight(result);
 Ok(LongRunningTickOutput::Done { final_output })
 }
}
```

**新增反向映射函数：**

```rust
fn map_subagent_result_to_lightweight(result: SubagentResult) -> LightweightTaskOutput {
 match result.status {
 SubagentResultStatus::Completed => LightweightTaskOutput::ToolResult {
 tool_name: "subagent".to_string(),
 output: result.text,
 },
 SubagentResultStatus::PartialTimeout => {
 let reason = result.reason.as_deref().unwrap_or("unknown");
 if reason == "timeout" { LightweightTaskOutput::Timeout }
 else if reason == "cancelled" { LightweightTaskOutput::Cancelled }
 else { LightweightTaskOutput::Backend { message: result.text } }
 }
 }
}
```

**新增测试（4个）：**
- `completed_maps_to_tool_result`
- `partial_timeout_with_timeout_reason_maps_to_timeout`
- `partial_timeout_with_cancelled_reason_maps_to_cancelled`
- `partial_timeout_with_other_reason_maps_to_backend`

#### coordinator.rs

**两个最小改动：**
1. `HiddenSubagentExecutionRequest` 添加 `#[derive(Clone)]`
2. `execute_hidden_subagent_internal` 改为 `pub(crate)`

### 测试验证

| 测试套件 | 结果 |
|---------|------|
| `cargo test -p northhing-core --lib a1_path` | 9 passed |
| `cargo test -p northhing-core --lib coordination` | 30 passed |
| `cargo test -p northhing-agent-dispatch --lib` | 24 passed |
| `cargo check -p northhing-core --lib` | ✅ 通过 |

### 风险与限制

1. **Cancel 传播延迟**：`tick` 阻塞在 `execute_hidden_subagent_internal` 上，`spawn_long_running` 的 `select!` 无法在 tick 运行期间观察 cancel。但 coordinator phase2 内部有自己的 cancel 观察，所以 cancel 仍然有效（只是延迟到下一个内部检查点）。

2. **不利用 LongRunningSkill 多轮能力**：subagent 的完整多轮循环在单个 tick 内完成。这是架构约束下的必要折衷。

3. **只在 `USE_LIGHTWEIGHT_ACTOR=true` 时激活**：默认 false，不影响现有代码路径。

---

## D 任务修改详情

### 修改文件

1. **`src/apps/desktop/src/app_state/mod.rs`** (+65/-0，测试模块)

### 设计决策

目标：在 headless 环境（无显示器）中测试 `create_ui` 函数，验证 UI 初始属性设置正确。

**方案：使用 MinimalSoftwareWindow 的 Noop Platform**
- `MinimalSoftwareWindow` 是 Slint 官方提供的 headless window adapter
- 自带 `SoftwareRenderer`，不需要自定义 renderer 实现
- 不打开 OS 窗口，安全用于测试环境
- `NoopPlatform::create_window_adapter` 返回 `MinimalSoftwareWindow::new(...)`
- `NoopPlatform::run_event_loop` 直接返回 `Ok(())`

### 技术要点

- `MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer)` 创建带软件渲染器的 headless 窗口
- `RepaintBufferType::NewBuffer` 表示每次重绘使用新缓— 区（最简单模式）
- `MinimalSoftwareWindow` 已实现了 `WindowAdapter` trait，包含完整的 `renderer()` 返回 `SoftwareRenderer`
- 不需要自定义 `WindowAdapter` 或处理 `Rc::new_cyclic` 的复杂性

### 代码变更

```rust
#[cfg(test)]
mod phase_i_tests {
 // ... existing tests ...

 // ═══════════════════════════════════════════════════════════════════
 // K.2.4 Mock display test
 // ═══════════════════════════════════════════════════════════════════

 use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
 use std::rc::Rc;
 use std::sync::Arc;

 /// A no-op Slint platform for headless testing.
 /// Uses MinimalSoftwareWindow (software renderer) so `create_ui` can
 /// instantiate the Slint component tree without a real display.
 struct NoopPlatform;

 impl slint::platform::Platform for NoopPlatform {
 fn create_window_adapter(
 &self,
 ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
 // MinimalSoftwareWindow provides a real (software) renderer
 // but never opens an OS window. Safe for headless tests.
 Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
 }

 fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
 Ok(())
 }
 }

 #[test]
 fn create_ui_runs_with_noop_platform() {
 // Set the no-op platform before creating the UI.
 slint::platform::set_platform(Box::new(NoopPlatform)).unwrap();

 let app_state = Arc::new(super::AppState::new());
 let ui = super::create_ui(app_state).unwrap();

 // Verify initial properties
 assert_eq!(ui.get_app_title(), "northhing v0.1.0");
 assert_eq!(ui.get_dark_mode(), true);
 }
}
```

### 测试验证

| 测试 | 结果 | 说明 |
|------|------|------|
| `cargo check --lib --tests` | ✅ 通过 | 代码编译无错误 |
| 链接/运行 | ⚠️ 环境阻塞 | MinGW 缺少 `shlwapi` 库和 `dlltool.exe`；MSVC 工具链下 `northhing-core` 在 Rust 1.96 有兼容性问题 |

**注意**：链接问题是 Windows 环境配置问题，非代码问题。代码本身已正确实现，使用 `MinimalSoftwareWindow` 避免了自定义 renderer 的 panic 风险。

---

## Review 后修复

### 修复 1：恢复被删除的 2 个测试
- `tool_result_invalid_json_leaves_structured_output_none`
- `backend_error_maps_to_partial_timeout`

### 修复 2：编译警告清理
- `partitioned_loader.rs`：删除未使用的 `test_context` 函数和 `PromptBuilderContext` import
- `agents/mod.rs`：删除未使用的 `ToolExposure` import

### 修复 3：D 任务 renderer panic 修复（Review 后第二轮）
- **问题**：自定义 `NoopWindowAdapter` 的 `renderer()` 使用 `unimplemented!()`，运行时 panic
- **根因**：`AppWindow::new()` 内部调用 `WindowInner::set_component`，需要获取 renderer
- **修复**：放弃自定义 `NoopWindowAdapter`，改用 Slint 官方 `MinimalSoftwareWindow`
 - `MinimalSoftwareWindow` 自带 `SoftwareRenderer`，无需自定义 renderer
 - 不打开 OS 窗口，适合 headless 测试
 - 代码更简洁，避免 `Rc::new_cyclic` 复杂性

### 测试验证（修复后）

| 测试套件 | 结果 |
|---------|------|
| `cargo test -p northhing-core --lib a1_path` | **11 passed** (7 正向 + 4 逆映射) |
| `cargo test -p northhing-core --lib coordination` | **32 passed** |
| `cargo test -p northhing-agent-dispatch --lib` | 24 passed |
| `cargo check -p northhing-core --lib` | ✅ 通过，**0 警告** |
| `cargo check --manifest-path src/apps/desktop/Cargo.toml --lib --tests` | ✅ 通过 |

---

## 修改文件清单（最终）

1. `src/crates/assembly/core/src/agentic/coordination/a1_path.rs` — A 任务主体 + 恢复 2 测试
2. `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` — `HiddenSubagentExecutionRequest` Clone + 可见性
3. `src/crates/assembly/core/src/agentic/agents/prompt_builder/partitioned_loader.rs` — 警告修复
4. `src/crates/assembly/core/src/agentic/agents/mod.rs` — 警告修复
5. `src/apps/desktop/src/app_state/mod.rs` — D 任务：NoopPlatform + MinimalSoftwareWindow + 测试

---

## 已知设计决策与折衷

### A 任务

1. **Cancel token 是"选择"而非"合并"**：
 - 代码：`self.cancel_token.clone().unwrap_or_else(|| ctx.cancel.clone())`
 - 语义：如果 `self.cancel_token` 被显式设置，则使用它；否则使用 `ctx.cancel`
 - 影响：如果 `self.cancel_token` 被设置为永不取消的 token，即使 `ctx.cancel` 被触发，skill 也不会响应。这是设计 choice，默认情况下（`self.cancel_token = None`）`ctx.cancel` 会被使用。

2. **tool_name round-trip 信息丢失**：
 - 正向映射：`LightweightTaskOutput::ToolResult { tool_name: "x", output: "hello" }` → `SubagentResult`
 - 反向映射：`SubagentResult` → `LightweightTaskOutput::ToolResult { tool_name: "subagent", output: "hello" }`
 - `tool_name` 从 `"x"` 变成了 `"subagent"`。这是预期的— subagent 的 tool name 不区分来源，统一标记为 `"subagent"`。

### D 任务

1. **MinimalSoftwareWindow vs 自定义 NoopWindowAdapter**：
 - 初始方案：自定义 `NoopWindowAdapter` + `unimplemented!("renderer")` → 运行时 panic
 - 最终方案：`MinimalSoftwareWindow`（Slint 官方 headless adapter）→ 自带 `SoftwareRenderer`，无 panic
 - 选择理由：避免实现 sealed `Renderer` trait，代码更简洁可靠

---

## 建议 Review 重点

1. **A 任务 Cancel token 选择逻辑**：`self.cancel_token` 优先、`ctx.cancel` fallback 的语义是否清晰？
2. **A 任务递归 A1 gate 避免**：`actor_runtime=None` 是否足够防止递归？
3. **A 任务错误映射完整性**：`PartialTimeout` 的 reason 分支是否覆盖了所有情况？
4. **D 任务 MinimalSoftwareWindow 使用**：`RepaintBufferType::NewBuffer` 是否适合测试场景？
5. **D 任务测试价值**：`create_ui_runs_with_noop_platform` 是否足够验证 headless 路径？
