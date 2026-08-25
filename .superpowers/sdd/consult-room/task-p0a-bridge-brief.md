# Task P0a Brief — facade respond_to_tool_confirmation + ui_dioxus/api.rs

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §P0a（B1 + F3 契约变更）。
> Base commit: `9bba819`（main tip）。本批是唯一的 contracts 变更批。

## 范围（两个文件改 + 一个文件新建）

### 1. `src/crates/contracts/kernel-api/src/tools.rs`

`KernelToolsApi` trait 末尾追加一个方法：

```rust
/// Respond to a pending tool confirmation (approve or reject).
///
/// Routes to the coordinator's confirmation channel
/// (`coordinator_session.rs` confirm_tool / reject_tool). Approval cards in
/// host UIs (Dioxus consult-room shell) call this when the user clicks
/// approve/reject on an unresolved ToolCall awaiting confirmation.
async fn respond_to_tool_confirmation(
    &self,
    tool_id: &str,
    approved: bool,
    reason: Option<String>,
) -> Result<(), KernelError>;
```

### 2. `src/crates/assembly/core/src/kernel_facade/tools.rs`

实现该方法（~20 行）：

```rust
async fn respond_to_tool_confirmation(
    &self,
    tool_id: &str,
    approved: bool,
    reason: Option<String>,
) -> Result<(), KernelError> {
    let coordinator = self.coordinator()?;
    if approved {
        coordinator.confirm_tool(tool_id, None).await
    } else {
        coordinator
            .reject_tool(tool_id, reason.unwrap_or_default())
            .await
    }
    .map_err(|e| KernelError::Runtime(format!("respond_to_tool_confirmation failed: {e}")))
}
```

已核实锚点：
- `ConversationCoordinator::confirm_tool(&self, tool_id: &str, updated_input: Option<Value>)` pub，在 `agentic/coordination/dialog_turn/coordinator_session.rs:219`
- `reject_tool(&self, tool_id: &str, reason: String)` 同文件 :224
- `self.coordinator()` 返回 `Result<&Arc<ConversationCoordinator>, KernelError>`（kernel_facade/mod.rs:51）
- **错误体例对齐**：未初始化时用 `KernelError::Runtime`（与 events.rs:49 一致），**不是** Internal

**测试**（同 commit，家规：contracts 变更带测试）：在 `kernel_facade/tools.rs` 或既有 facade 测试模块加 1 个单测——facade 未初始化（coordinator 为 None）时调用返回 `Err(KernelError::Runtime)`。

### 3. 新建 `src/apps/desktop/src/ui_dioxus/api.rs`（~180 行）

薄封装 `kernel_facade()`，供 Dioxus 各页面调用。**不建子模块、不建 AppEvent enum、不建 event_bus.rs。**

已核实签名（全部来自源码，勿再改）：
- `kernel_facade()` → `Arc<KernelFacade>`（kernel_facade/mod.rs:36）
- `submit_turn(TurnInputDto) -> Result<DialogSubmitOutcomeDto, KernelError>`（contracts turn.rs:80；DTO 字段 turn.rs:12-20：session_id/text/mode/policy/source/workspace_path）
- `stop_turn(&TurnId) -> Result<(), KernelError>`（turn.rs:84；`pub type TurnId = String`）
- `list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError>`（session.rs:241）
- `get_session(&SessionId) -> Result<SessionDto, KernelError>`（session.rs:253）
- `subscribe_events(Box<dyn Fn(KernelEventDto) + Send + 'static>) -> Result<SubscriptionId, KernelError>`（kernel_facade/events.rs:41——**callback 模型，非 Stream**）

结构：
```rust
// 上行薄封装：
pub async fn submit_turn(session_id: &str, text: String) -> Result<TurnId, KernelError>
  // TurnInputDto { session_id: session_id.to_string(), text,
  //   mode: "agentic".into(),
  //   policy: SubmissionPolicyDto { allow_subagent: true, max_turns: None },
  //   source: TriggerSourceDto::User, workspace_path: None }
  // outcome.accepted == false → Err(KernelError::Runtime(outcome.error))；否则 Ok(outcome.turn_id)
pub async fn stop_turn(turn_id: &TurnId) -> Result<(), KernelError>
pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError>
pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError>
pub async fn respond_to_tool_confirmation(tool_id: &str, approved: bool) -> Result<(), KernelError>

// 下行事件通道（callback → mpsc 包装）：
pub fn event_channel() -> tokio::sync::mpsc::Receiver<KernelEventDto>
  // let (tx, rx) = mpsc::channel(256);
  // std::thread 或 tokio task 一次性调 facade.subscribe_events(Box::new(move |dto| {
  //     let _ = tx.blocking_send(dto);  // 满则丢——非 Critical 事件可接受
  // }));
  // rx
```

`ui_dioxus/mod.rs` 加 `mod api;`（不 re-export，页面直接 `use super::api`）。

## 禁区

- 不动 `event_bridge.rs` / Slint 侧任何文件
- 不动 `session_mock.rs`
- 不在 api.rs 引入 dioxus 依赖（纯 async 函数 + tokio，保持可测试）
- 不改 `StreamEventSink` trait
- 不写文档文件（除本任务代码注释）

## 验证（implementer 必须实跑并贴输出）

环境陷阱：GNU toolchain 在 `TEMP=C:\WINDOWS\TEMP` 下 linker 必崩（ld response file bug）。先设：
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
```

```powershell
cd E:\agent-project\northing
cargo check --workspace                                    # contracts 变更面广
cargo check -p northhing --features ui-dioxus              # 桌面+Dioxus 门（家规 6）
cargo test -p northhing-core --features product-full kernel_facade   # facade 单测
```

三条全绿才可报 DONE。报告写：`.superpowers/sdd/reports/task-p0a-bridge-report.md`，含命令输出原文。
