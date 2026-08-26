# Task P2a — Room 全页数据流：启动时真会话消息覆盖 mock seed

来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F1（P2a）。
依赖：P0a 已完成（`ui_dioxus/api.rs` 桥已存在）。

## 目标

诊室主窗（`ui_dioxus/app.rs::room_app_root`）当前启动渲染 `seed_session()` 五条假数据。本任务把启动流接上内核：拿到 room session 后拉取持久化消息，转换后覆盖 `entries`；seed 仅保留为空会话回退。

## 已解决歧义（编排者裁定，实现按此执行）

处方原文写「`get_session` 覆盖 seed_session」。经查证 `SessionDto` 只有 `{id, state, kind}`，**不含消息体**——真正的消息接口是：

```rust
// kernel-api/src/session.rs:265
async fn get_messages(&self, session_id: &SessionId) -> Result<Vec<MessageDto>, KernelError>;
```

本任务用 **`ensure_room_session()` + `get_messages(sid)`** 实现处方的数据流意图。审查者注意：这是对计划字面 API 名的勘误，不是范围变更。api.rs 需新增一个薄包装 `get_messages(session_id: &SessionId)`（与现有 `get_session` 并排，同风格）。

## 数据形状（逐字，kernel-api/src/session.rs）

```rust
pub struct MessageDto {
    pub id: String,
    pub role: MessageRoleDto,        // User | Assistant | Tool | System (snake_case serde)
    pub content: MessageContentDto,
    pub metadata: Option<MessageMetadataDto>,
    pub timestamp: i64,
}

pub enum MessageContentDto {
    Text(String),
    Multimodal { text: String, images: Vec<String> },
    ToolResult { tool_id: String, tool_name: String, result: serde_json::Value,
                 result_for_assistant: Option<String>, is_error: bool },
    Mixed { reasoning_content: Option<String>, text: String, tool_calls: Vec<ToolCallStub> },
}
pub struct ToolCallStub { pub tool_name: String, pub arguments: Option<serde_json::Value>, pub is_error: bool }
```

目标类型（ui_dioxus/session_mock.rs）：`MockEntry::{Entity{who,body,children}, Witness{who,body}, Approval{...}}`，`MockChild::{ToolLog{label}, ArtifactChip{label}}`。

## 改动点

### ① 启动数据流（app.rs）

在 `room_app_root` 中加一个 `use_future`（放在事件订阅 future 附近）：

1. `api::ensure_room_session().await`：
   - Ok(sid) → `session_id_signal.set(Some(sid.clone()))`（顺带修正 L135 等处的 sid 过滤：现在启动即有 sid，事件不再全放行）；
   - Err(e) → `tracing::warn!` 一行，保留 seed，不设 error UI。
2. `api::get_messages(&sid).await`：
   - Ok(msgs) → 转换（见下表）；结果**非空才** `entries.set(converted)`，空 vec 保留 seed；
   - Err(e) → `tracing::warn!` 一行，保留 seed。

### ② 转换函数（session_mock.rs）

纯函数 `pub fn messages_to_entries(msgs: Vec<MessageDto>) -> Vec<MockEntry>`，映射表（房间既定角色约定：用户=见证者右对齐，agent=它左对齐）：

| 输入 | 输出 |
|---|---|
| `role=User`, `Text(t)` 或 `Multimodal{text,..}` | `Witness { who: "见证者", body: t }` |
| `role=Assistant`, `Mixed{text, reasoning_content, tool_calls}` | `Entity { who: "它", body: text 非空取 text，否则 reasoning_content.unwrap_or_default(), children: tool_calls.map(\|tc\| ToolLog{label: tc.tool_name}) }` |
| `role=Assistant`, `Text(t)` 或 `Multimodal{text,..}` | `Entity { who: "它", body: t, children: vec![] }` |
| `role=Tool`（`ToolResult` 内容）或任何 ToolResult 变体 | **跳过**（事件路径今日也不渲染历史工具输出） |
| `role=System` | **跳过** |

时间戳、images、arguments 本轮一律丢弃（MockEntry 无对应字段，不扩类型）。

### ③ 单元测试（session_mock.rs tests mod）

至少 4 个纯函数测试（无需 facade）：
1. User/Text → Witness，body 透传；
2. Assistant/Mixed 带 2 个 tool_calls → Entity + 2 个 ToolLog children，text 优先于 reasoning_content；
3. System 与 Tool 角色被跳过；
4. 空 vec 返回空 vec（app.rs 侧据此保留 seed）。

### ④ TODO 标记（不动逻辑）

- `ui_dioxus/pages_archive.rs` STRATA 区块顶部：`// TODO(data): wire to session/archive query`
- `ui_dioxus/pages_space.rs` DOORS 区块顶部：同上标记

## 禁区

- 不动 `send_action` / `stop_action` / streaming / TurnState 事件分支的既有行为（send_action 的惰性 ensure 留作兜底，不改）。
- 不动 contracts / core / GlobalConfig。
- 不改 MockEntry / MockChild 类型定义，不加字段。
- 不做 i18n 键变更（who 字面量与现网 send_action 用法一致："见证者" / "它"）。
- 不动 seed_session() 的内容。

## 复用侦察（必填进 report）

实现前先确认并写入 report：`get_messages` 包装是否已存在于 api.rs（应无）；仓库内是否已有 MessageDto→展示条目的转换器可复用（提示：core `kernel_facade/dto.rs` 有 message_to_dto 反向转换及 `message_to_item*` 测试，可参考其 Mixed 处理但那是 core 侧，不可跨层引用）。声称"无既有实现"的点需给出检索方式。

## 验证（report 必贴命令+尾部输出）

```
cargo check -p northhing          # 必须绿
cargo check -p northhing --tests  # 测试代码编译绿
```

（GNU 工具链跑不了测试二进制属已知环境问题，测试实测由编排者在 MSVC 侧统一取证，你不必跑 `cargo test`。）

## Report

写 `.superpowers/sdd/reports/task-p2a-room-dataflow-report.md`：改动清单（file:line）、复用侦察结论、验证输出尾部、任何偏离 brief 的决定及理由。
