# Task P0c Brief — F3-UI approval 卡接线（含事件方向缺口补齐）

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F3-UI + 本 brief §0 的新发现。
> Base commit: `4b6a012`（P0b 已落）。

## §0 派发后新发现（编排者预检抓出，任务范围因此扩展）

**Approval 请求根本到不了 UI**：facade 事件映射 `kernel_facade/events.rs:278` 的 `_ => vec![]` 把 `ToolEventData::ConfirmationNeeded`（state_manager.rs:191）静默丢弃；frozen `ToolCallPhase`（contracts events.rs:39-42）只有 `Started/Completed` 两变体。

故本任务含**第二个契约扩展**（与 P0a 的 `respond_to_tool_confirmation` 同类，属用户 2026-08-25 方案 A「approval 走 facade」裁决的自然延伸）：`ToolCallPhase` 追加 `AwaitingConfirmation` 变体（additive，snake_case tag）。

## 范围

### 1. `src/crates/contracts/kernel-api/src/events.rs`

```rust
pub enum ToolCallPhase {
    Started,
    Completed,
    AwaitingConfirmation,   // 新增
}
```

### 2. `src/crates/assembly/core/src/kernel_facade/events.rs`

在 ToolEventData match 的 `_ => vec![]` 之前加 arm：

```rust
crate::agentic::events::ToolEventData::ConfirmationNeeded { tool_id, tool_name, params } => {
    let params_str = params.to_string();
    vec![KernelEventDto::ToolCall(super::ToolCallDto {
        session_id: session_id.clone(),
        turn_id: turn_id.clone(),
        call_id: tool_id.clone(),
        name: tool_name.clone(),
        phase: super::ToolCallPhase::AwaitingConfirmation,
        summary: crate::kernel_facade::helpers::extract_summary_from_params(params),
        detail: Some(crate::kernel_facade::helpers::truncate_4000(&params_str)),
        result_count: None,
    })]
    // 不发 TurnPhase——turn 仍在 ToolUse 语境，不因 awaiting 改 phase
}
```

**测试**（contracts/facade 变更带测试）：`kernel_facade/tests.rs` 加 1 个单测——构造 ConfirmationNeeded 的 AgenticEvent → `agentic_event_to_dtos` → 断言产出 `ToolCall` 且 `phase == AwaitingConfirmation`、`call_id` 正确。

### 3. `src/apps/desktop/src/ui_dioxus/session_mock.rs`

`MockEntry::Approval` 加字段 `call_id: String`（seed 数据填占位如 `"mock-call-1"`）。

### 4. `src/apps/desktop/src/ui_dioxus/app.rs`

**事件消费**（P0b 的 use_future match 里加 arm）：
```rust
KernelEventDto::ToolCall(tc) if tc.phase == ToolCallPhase::AwaitingConfirmation
    && sid.read().as_ref().map(|s| s == &tc.session_id).unwrap_or(true) => {
    entries.write().push(MockEntry::Approval {
        call_id: tc.call_id,
        head: tc.name,
        main: tc.summary,
        risk: tc.detail.unwrap_or_default(),
        resolved: false,
        state_text: None,
    });
}
```
同一 ToolCall call_id 已有未决卡时**不重复追加**（防重连重放）。

**按钮接线**（render_entry 的 Approval 分支，resolved == false）：
- approve onclick → `spawn` 调 `api::respond_to_tool_confirmation(&call_id, true)` → 成功后本地把该卡 `resolved = true`（乐观更新，按 call_id 在 entries 里定位）
- reject onclick → 同，`false`
- render_entry 当前签名 `(&MockEntry, &LocalePack)` 是纯渲染——需要把 `entries: Signal<Vec<MockEntry>>` 传进去（或渲染处 inline 展开）。选最小 diff 方案。
- resolved == true 分支维持不绑事件。

## 禁区

- 不动 `ToolEventData` / `pipeline_pre.rs` / 协调器内部
- 不加 reject 文本输入框
- 不动 `KernelEventDto` 枚举本体（只动 ToolCallPhase）
- 不动 P0b 的 send/streaming 路径

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check --workspace
cargo check -p northhing --features ui-dioxus
cargo test -p northhing-core --features product-full kernel_facade
cargo test -p northhing --features ui-dioxus --lib ui_dioxus
```

报告：`.superpowers/sdd/reports/task-p0c-approval-report.md`（status + files + 验证输出原文 + 偏离声明）。
